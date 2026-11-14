//! Where a local variable is declared and every place its body writes it.
//!
//! A local is no workspace symbol: the reference index holds declarations and
//! uses of names a file exports, and a local is scoped to the body that
//! declares it. So a local's answer is read off the type phase's record of
//! each body's bindings (`nvs_types::ExprTable::local_scopes`) and the open
//! document's tokens, and never off the index. Three requests read it:
//! `textDocument/definition` answers the binding's declaration,
//! `textDocument/documentHighlight` and `textDocument/references` every site,
//! and rename ([`crate::rename`]) every site with its own refusals on top.
//!
//! **The variable token wins over the expression around it.** A cursor on the
//! `$u` of `$u->name`, or on a `$markup` passed as an argument, is on the
//! local. The member or the call is answered only where the cursor is on its
//! own name, which is what every request that walks outward from the
//! innermost node would otherwise get wrong.
//!
//! **A site is a variable the body itself writes.** A named function, a
//! method and a property hook inside the body are scopes of their own and are
//! skipped. An anonymous function reads the body's locals by value, so a read
//! inside one is the same local, unless that function binds the name again:
//! then its reads are its own variable and are not sites of this one. A cursor
//! inside such a function answers nothing, because which of the two it names
//! is not recorded.
//!
//! **A parameter's `@param` name is one of its sites.** The `$name` after an
//! `@param` in the method's `///` run is written for the parameter, so
//! references, highlight, definition and rename treat it as one more place the
//! parameter is written, and a cursor on it finds the parameter
//! (`rule:tooling/doc-comment-tags-are-closed-and-checked`).
//!
//! - **Memory:** nothing held. One request tokenizes the open document and
//!   walks its tree once.

use std::collections::BTreeSet;

use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, Span};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, DocComment, DocTagKind, MethodMember, Param, Stmt, StmtKind,
};
use nvs_syntax::{Token, TokenKind, tokenize, walk};
use nvs_types::LocalBinding;

use crate::definition::{covers, text_of};
use crate::document::Analysed;

/// One local of one body, found from a variable token in the entry document.
#[derive(Debug)]
pub(crate) struct Local<'a> {
    /// The body that declares it.
    pub body: Span,
    /// The binding the body recorded for it.
    pub binding: &'a LocalBinding,
    /// Every name the body binds.
    pub bindings: &'a [LocalBinding],
    /// Every name an anonymous function inside the body binds.
    pub nested: BTreeSet<String>,
    /// Whether a `global` statement in the body writes the name.
    pub global: bool,
    /// Every place the body writes the local, the declaration included, in
    /// source order. A parameter's `@param` name is one of them.
    pub sites: Vec<Span>,
    /// The method that declares the local as a parameter, and that parameter.
    /// `None` for every other local, an anonymous function's parameter
    /// included.
    pub parameter: Option<(&'a MethodMember, &'a Param)>,
}

impl Local<'_> {
    /// Whether the local is a parameter of the function or method whose body
    /// declares it.
    pub(crate) const fn is_parameter(&self) -> bool {
        self.binding.declared.start < self.body.start
    }
}

/// The local the variable at `offset` in the entry document names, for the
/// three read-only requests: `None` where the cursor is on no variable, on
/// `$this`, on a variable no body binds, on a variable that is no local's
/// site, and inside an anonymous function that binds the name again. A
/// parameter, a name an anonymous function binds again and a `global` are
/// answered here, because nothing is changed.
pub(crate) fn at(analysed: &Analysed, offset: BytePos) -> Option<Local<'_>> {
    let file = analysed.map.file(analysed.entry);
    let tokens = tokenize(file, &mut Diagnostics::new());
    if let Some((target, param)) = param_tag_at(analysed, offset) {
        let written = text_of(file, param.name);
        let local = of(analysed, file, &tokens, param.name, written.get(1..)?)?;
        return local.sites.contains(&target).then_some(local);
    }
    let token = variable_at(&tokens, offset)?;
    let written = text_of(file, token.span);
    if written == "$this" {
        return None;
    }
    let local = of(analysed, file, &tokens, token.span, &written[1..])?;
    local.sites.contains(&token.span).then_some(local)
}

/// The variable token at `offset`, preferring one the offset is strictly
/// inside over one it ends.
pub(crate) fn variable_at(tokens: &[Token], offset: BytePos) -> Option<Token> {
    tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Variable)
        .filter(|token| covers(token.span, offset))
        .min_by_key(|token| token.span.end == offset)
        .copied()
}

/// The local `$name` written at `token`, or `None` where no body covering
/// it binds that name.
///
/// The sites are filled even where rename refuses — a parameter, a name an
/// anonymous function binds again, a `global` — and the refusal is the
/// caller's to make off [`Local::is_parameter`], [`Local::nested`] and
/// [`Local::global`]. `token` itself need not be among the sites: a variable
/// some other node writes, such as the `$count` of `User::$count`, is not.
pub(crate) fn of<'a>(
    analysed: &'a Analysed,
    file: &SourceFile,
    tokens: &[Token],
    token: Span,
    name: &str,
) -> Option<Local<'a>> {
    let scopes: Vec<(Span, &[LocalBinding])> = analysed
        .exprs
        .local_scopes()
        .filter(|(body, _)| body.file == analysed.entry)
        .collect();
    let offset = token.start;
    // A body covers the token, or a parameter of it is the token itself.
    let mut covering: Vec<&(Span, &[LocalBinding])> = scopes
        .iter()
        .filter(|(body, bindings)| {
            (body.start <= offset && offset < body.end)
                || bindings
                    .iter()
                    .any(|binding| binding.name == name && binding.declared == token)
        })
        .collect();
    covering.sort_by_key(|(body, _)| body.end - body.start);
    let &&(body, bindings) = covering
        .iter()
        .find(|(_, bindings)| bindings.iter().any(|binding| binding.name == name))?;
    let binding = bindings.iter().find(|binding| binding.name == name)?;

    let stmts = entry_stmts(analysed);
    let roots = walk::of_stmts(stmts);
    let nodes: Vec<&walk::Node> = roots
        .iter()
        .flat_map(|root| std::iter::once(root).chain(root.descendants()))
        .collect();

    // A named function, a method and a property hook inside the body are
    // scopes of their own, so nothing written in one is this local.
    let own: Vec<Span> = nodes
        .iter()
        .filter(|node| matches!(node.kind, "Function" | "Method" | "PropertyHook"))
        .filter(|node| inside(body, node.span) && node.span != body)
        .map(|node| node.span)
        .collect();
    let in_scope = |span: Span| inside(body, span) && !own.iter().any(|skip| inside(*skip, span));

    // An anonymous function reads this body's locals by value. One that binds
    // the same name has a variable of its own under it.
    let mut nested: BTreeSet<String> = BTreeSet::new();
    let mut shadowing: Vec<Span> = Vec::new();
    for function in nodes
        .iter()
        .filter(|node| node.kind == "Fn" && in_scope(node.span))
    {
        let bound = bound_in(file, tokens, &scopes, function);
        if bound.contains(name) {
            shadowing.push(function.span);
        }
        nested.extend(bound);
    }

    let spelled = format!("${name}");
    let mut sites = Vec::new();
    let mut global = false;
    for candidate in tokens.iter().filter(|candidate| {
        candidate.kind == TokenKind::Variable
            && in_scope(candidate.span)
            && !shadowing.iter().any(|skip| inside(*skip, candidate.span))
            && text_of(file, candidate.span) == spelled
    }) {
        let Some(innermost) = innermost(&nodes, candidate.span) else {
            continue;
        };
        match innermost.kind {
            "Variable" | "LocalDecl" | "Foreach" | "Try" | "Destructure" | "StaticLocal" => {
                sites.push(candidate.span);
            }
            "Global" => global = true,
            _ => {}
        }
    }
    if text_of(file, binding.declared) == spelled {
        sites.push(binding.declared);
    }
    let parameter = methods(stmts).into_iter().find_map(|(method, doc)| {
        let param = method
            .params
            .iter()
            .find(|param| param.name == binding.declared)?;
        sites.extend(param_tags(doc, file, &spelled));
        Some((method, param))
    });
    sites.sort_by_key(|span| (span.start, span.end));
    sites.dedup();
    Some(Local {
        body,
        binding,
        bindings,
        nested,
        global,
        sites,
        parameter,
    })
}

/// The `$name` of each `@param` line in `doc` that names `spelled`.
fn param_tags(doc: Option<&DocComment>, file: &SourceFile, spelled: &str) -> Vec<Span> {
    doc.into_iter()
        .flat_map(|doc| &doc.tags)
        .filter(|tag| tag.kind == DocTagKind::Param)
        .filter_map(|tag| tag.target)
        .filter(|target| text_of(file, *target) == spelled)
        .collect()
}

/// The `$name` of an `@param` line under the cursor at `offset` in the entry
/// document, and the parameter of the method below it that it names.
pub(crate) fn param_tag_at(analysed: &Analysed, offset: BytePos) -> Option<(Span, &Param)> {
    let file = analysed.map.file(analysed.entry);
    let stmts = entry_stmts(analysed);
    methods(stmts).into_iter().find_map(|(method, doc)| {
        let target = doc?
            .tags
            .iter()
            .filter(|tag| tag.kind == DocTagKind::Param)
            .filter_map(|tag| tag.target)
            .find(|target| covers(*target, offset))?;
        let written = text_of(file, target);
        let param = method
            .params
            .iter()
            .find(|param| text_of(file, param.name) == written)?;
        Some((target, param))
    })
}

/// The entry document's statements.
fn entry_stmts(analysed: &Analysed) -> &[Stmt] {
    analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
        .map(|loaded| loaded.stmts.as_slice())
        .unwrap_or_default()
}

/// Every method a class, an interface or an enum in `stmts` declares, a
/// bracketed namespace's included, each with its `///` run.
fn methods(stmts: &[Stmt]) -> Vec<(&MethodMember, Option<&DocComment>)> {
    stmts
        .iter()
        .flat_map(|stmt| match &stmt.kind {
            StmtKind::NamespaceDecl(decl) => decl
                .body
                .as_ref()
                .map(|body| methods(&body.stmts))
                .unwrap_or_default(),
            StmtKind::ClassDecl(decl) => methods_of(&decl.members),
            StmtKind::InterfaceDecl(decl) => methods_of(&decl.members),
            StmtKind::EnumDecl(decl) => methods_of(&decl.members),
            _ => Vec::new(),
        })
        .collect()
}

/// The methods among `members`, each with its `///` run.
fn methods_of(members: &[ClassMember]) -> Vec<(&MethodMember, Option<&DocComment>)> {
    members
        .iter()
        .filter_map(|member| match &member.kind {
            ClassMemberKind::Method(method) => Some((method, member.doc.as_ref())),
            _ => None,
        })
        .collect()
}

/// Every name the anonymous function `function` binds: its block body's own
/// bindings, and each parameter, which no child node covers.
fn bound_in(
    file: &SourceFile,
    tokens: &[Token],
    scopes: &[(Span, &[LocalBinding])],
    function: &walk::Node,
) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = scopes
        .iter()
        .filter(|(body, _)| inside(function.span, *body))
        .flat_map(|(_, bindings)| bindings.iter().map(|binding| binding.name.clone()))
        .collect();
    for token in tokens.iter().filter(|token| {
        token.kind == TokenKind::Variable
            && inside(function.span, token.span)
            && !function
                .children
                .iter()
                .any(|child| inside(child.span, token.span))
    }) {
        if let Some(name) = text_of(file, token.span).strip_prefix('$') {
            found.insert(name.to_owned());
        }
    }
    found
}

/// Whether `outer` holds every byte of `inner`.
pub(crate) const fn inside(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// The smallest node that holds `span`, a variable or a declaration before
/// any other node of the same size.
fn innermost<'a>(nodes: &[&'a walk::Node], span: Span) -> Option<&'a walk::Node> {
    nodes
        .iter()
        .filter(|node| inside(node.span, span))
        .min_by_key(|node| {
            (
                node.span.end - node.span.start,
                !matches!(node.kind, "Variable" | "LocalDecl"),
            )
        })
        .copied()
}
