//! *Implement the missing methods*, *Override a method* and *Declare the
//! method you called*, the three generators
//! `rule:ide/a-code-action-writes-only-what-is-already-determined` names that
//! write a declaration.
//!
//! Implement the missing methods is a quick fix on `E0449`, which the checker
//! reports at a class that does not declare a method one of its interfaces or
//! its `abstract` parents leaves without a body. It writes every such method
//! after the class's last member, each with its signature as declared and a
//! body that throws a `LogicError`. Override a method is offered at a cursor in
//! a class body between two members, once per method a parent class gives a
//! body that the class may override and does not, and writes it with a call to
//! `parent::`. Declare the method you called is a quick fix on the `E0405` of
//! `$obj->name(...)`. The checker records that call's argument types and the
//! type its context expects ([`nvs_types::expr_table::UnresolvedCall`]),
//! keyed by the diagnostic's span. It writes that
//! method after the last member of the receiver's class, when the entry
//! document declares that class, with a body that throws a `LogicError`.
//!
//! # Decision: what declare writes
//!
//! A method, since Novis has no function outside a class. It is `private`
//! when the call is inside the class's own declaration, and `public`
//! otherwise. Each parameter has the argument's type, and `inout` where the
//! call writes it. Its name is the argument's own name at a named argument,
//! the variable at an argument that is one variable, and `$argN` for the Nth
//! argument otherwise or where a name repeats. The return type is the one the
//! context expects. With no expectation, it is `void` where the call starts a
//! statement and `mixed` anywhere else. A static call `C::name()` gets
//! nothing, since its diagnostic carries no call, and so does a call on an
//! interface, an enum or a class another file declares.
//!
//! # Decision: the signature is the declaration's own text, and the edit is checked
//!
//! A parameter is copied as its declaration writes it, so `inout`, `covariant`,
//! `...`, an attribute and a default value stay. Each type in it is written
//! the way the class's own file names that type
//! ([`crate::imports::written_at`]), so a type declared in another namespace
//! still resolves. A `LogicError` written in a namespace gets its `use` line.
//! The text both actions write is analysed before it is offered
//! ([`analyse_rewritten`]), and an action whose text reports an error is not
//! offered. That covers what this module does not compute: a generic interface,
//! whose methods are written in its type parameter, a method of a `Core`
//! interface, which has no declaration to copy, and a default value whose names
//! resolve differently in the class's file.
//!
//! # Decision: what override offers
//!
//! The methods with a body that the classes the class extends declare, nearest
//! first, each name once. A `private`, `final` or `static` method and the
//! constructor are left out, and so is a method with an `inout` parameter,
//! whose argument the call would have to choose a form for. An interface's
//! default body is left out too, since `parent::` does not reach it. A method
//! whose return type is `void` or `never` calls the parent as a statement, and
//! every other one returns what the parent returns. All of a class's overrides
//! are written into one text and analysed once, and none is offered if that
//! text reports an error.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. Per
//! `textDocument/codeAction`, one analysis of the entry's graph where an
//! `E0449` or an `E0405` with its call meets the range, and one where the
//! cursor is between the members of a class that may override something.
//! Everything is dropped with the answer.

use nvs_diagnostics::{BytePos, PositionEncoding, SourceFile, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{ClassDecl, ClassMemberKind, MethodMember, Modifier, Stmt, StmtKind};
use rustc_hash::FxHashSet;

use crate::completion::lineage;
use crate::definition::{body_of, class_at, declared_type, resolved_name, text_of};
use crate::diagnostics::phase_gated;
use crate::document::{Analysed, Documents, analyse_rewritten};
use crate::imports::written_at;
use crate::position::range_at;
use crate::render::{Action, Edit};

/// The title of the quick fix that writes every missing method.
pub const IMPLEMENT: &str = "Implement the missing methods";

/// The parameter name an argument with no name of its own is given, before
/// its position counted from 1.
const UNNAMED: &str = "arg";

/// The kind implement is filed under, LSP's `CodeActionKind::QUICKFIX`.
const QUICK_FIX: &str = "quickfix";

/// The kind override is filed under, which [`crate::split_join`] declares.
pub const OVERRIDE_KIND: &str = crate::split_join::KIND;

/// One level of indentation, the way `nvs fmt` writes it.
const INDENT: &str = "    ";

/// The class a method body that is not written yet throws.
const NOT_WRITTEN: &str = "LogicError";

/// An edit: what it replaces, and with what.
type Change = (Span, String);

/// An action before it is placed: its title, its kind and its edits, the main
/// one first.
struct Offer {
    title: String,
    kind: &'static str,
    edits: Vec<Change>,
}

/// The three generators offered over `[start, end)` of the entry document.
#[must_use]
pub fn at(
    analysed: &Analysed,
    documents: &Documents,
    start: BytePos,
    end: BytePos,
    encoding: PositionEncoding,
) -> Vec<Action> {
    let file = analysed.map.file(analysed.entry);
    let mut offered: Vec<Offer> = implement(analysed, documents, start, end)
        .into_iter()
        .chain(declare(analysed, documents, start, end))
        .collect();
    if analysed.diags.error_count() == 0 {
        offered.extend(overrides(analysed, documents, start));
    }
    offered
        .into_iter()
        .filter_map(|offer| {
            let (first, rest) = offer.edits.split_first()?;
            Some(Action {
                title: offer.title,
                kind: offer.kind.to_owned(),
                range: range_at(file, first.0, encoding),
                replacement: first.1.clone(),
                also: rest
                    .iter()
                    .map(|(span, replacement)| Edit {
                        range: range_at(file, *span, encoding),
                        replacement: replacement.clone(),
                    })
                    .collect(),
            })
        })
        .collect()
}

/// *Implement the missing methods*, where an `E0449` meets the range.
fn implement(
    analysed: &Analysed,
    documents: &Documents,
    start: BytePos,
    end: BytePos,
) -> Option<Offer> {
    let diagnostic = phase_gated(&analysed.diags)
        .into_iter()
        .find(|diagnostic| {
            diagnostic.code == Some(code::E_INTERFACE_METHOD_MISSING)
                && diagnostic.primary_span().is_some_and(|span| {
                    span.file == analysed.entry && span.start <= end && start <= span.end
                })
        })?;
    let (class, decl) = class_named(analysed, diagnostic.primary_span()?.start)?;
    let methods = missing(analysed, &class);
    if methods.is_empty() {
        return None;
    }
    let entry = analysed.map.file(analysed.entry);
    let place = Place::of(entry.text(), decl)?;
    let short = text_of(entry, decl.name.span);
    let written: Vec<String> = methods
        .iter()
        .map(|(file, method)| {
            let name = text_of(file, method.name);
            place.method(
                &signature(analysed, file, method, place.at),
                &format!("throw new {NOT_WRITTEN}(\"{short}::{name} is not implemented\");"),
            )
        })
        .collect();
    let edits = with_not_written(analysed, &place, place.joined(&written))?;
    compiles(analysed, documents, &edits)?;
    Some(Offer {
        title: IMPLEMENT.to_owned(),
        kind: QUICK_FIX,
        edits,
    })
}

/// *Declare the method you called*, where the `E0405` of a call meets the
/// range.
fn declare(
    analysed: &Analysed,
    documents: &Documents,
    start: BytePos,
    end: BytePos,
) -> Option<Offer> {
    let (site, call) = phase_gated(&analysed.diags)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == Some(code::E_UNKNOWN_MEMBER))
        .filter_map(|diagnostic| diagnostic.primary_span())
        .filter(|span| span.file == analysed.entry && span.start <= end && start <= span.end)
        .find_map(|span| Some((span, analysed.exprs.unresolved_call(span)?)))?;
    let (stmt, file) = declared_type(analysed, &QName::parse(&call.class))?;
    let StmtKind::ClassDecl(decl) = &stmt.kind else {
        return None;
    };
    if file.id() != analysed.entry {
        return None;
    }
    let entry = analysed.map.file(analysed.entry);
    let place = Place::of(entry.text(), decl)?;
    let typed = |ty: &str| written_at(analysed, ty, None, place.at);
    let mut taken: FxHashSet<String> = FxHashSet::default();
    let params: Vec<String> = call
        .args
        .iter()
        .enumerate()
        .map(|(index, arg)| {
            let name = arg
                .name
                .clone()
                .filter(|name| taken.insert(name.clone()))
                .unwrap_or_else(|| format!("{UNNAMED}{}", index + 1));
            let inout = if arg.inout { "inout " } else { "" };
            format!("{inout}{} ${name}", typed(&arg.ty))
        })
        .collect();
    let returns = match &call.returns {
        Some(ty) => typed(ty),
        None if starts_statement(entry.text(), site.start) => "void".to_owned(),
        None => "mixed".to_owned(),
    };
    let visibility = if decl.span.start <= site.start && site.end <= decl.span.end {
        "private"
    } else {
        "public"
    };
    let method = &call.method;
    let short = text_of(entry, decl.name.span);
    let text = place.method(
        &format!(
            "{visibility} function {method}({}): {returns}",
            params.join(", ")
        ),
        &format!("throw new {NOT_WRITTEN}(\"{short}::{method} is not implemented\");"),
    );
    let edits = with_not_written(analysed, &place, place.joined(&[text]))?;
    compiles(analysed, documents, &edits)?;
    Some(Offer {
        title: format!("Declare `{method}` in `{short}`"),
        kind: QUICK_FIX,
        edits,
    })
}

/// Whether `offset` is where a statement starts in `source`: the text before
/// it, past whitespace, ends a statement, opens a block or is nothing.
fn starts_statement(source: &str, offset: BytePos) -> bool {
    source
        .get(..offset as usize)
        .and_then(|before| before.trim_end().chars().last())
        .is_none_or(|ch| matches!(ch, ';' | '{' | '}'))
}

/// The edit that writes `methods` at `place`, and the `use` line for
/// `LogicError` where the entry document does not name it already.
fn with_not_written(analysed: &Analysed, place: &Place, methods: String) -> Option<Vec<Change>> {
    let mut edits = vec![(place.span(analysed), methods)];
    if resolved_name(analysed, NOT_WRITTEN, place.at) != Some(QName::parse(NOT_WRITTEN)) {
        let entry = analysed.map.file(analysed.entry);
        let stmts = crate::imports::entry_stmts(analysed)?;
        let site = nvs_hir::import_site(stmts, entry, place.at)?;
        edits.push((
            site.span(analysed.entry),
            site.use_line(&QName::parse(NOT_WRITTEN)),
        ));
    }
    Some(edits)
}

/// *Override a method*, once per method the class may override, where
/// `offset` is in the class's body and in none of its members.
fn overrides(analysed: &Analysed, documents: &Documents, offset: BytePos) -> Vec<Offer> {
    let Some((class, decl)) = class_named(analysed, offset) else {
        return Vec::new();
    };
    let entry = analysed.map.file(analysed.entry);
    let Some(place) = Place::of(entry.text(), decl) else {
        return Vec::new();
    };
    let inside = place.open < offset && offset < decl.span.end;
    if !inside
        || decl
            .members
            .iter()
            .any(|member| member.span.start <= offset && offset < member.span.end)
    {
        return Vec::new();
    }
    let mut taken: FxHashSet<&str> = methods_of(&decl.members)
        .map(|method| text_of(entry, method.name))
        .collect();
    let mut offered: Vec<(String, String)> = Vec::new();
    for (parent, stmt, file) in parents(analysed, &class) {
        let Some((_, members)) = body_of(stmt) else {
            continue;
        };
        for method in methods_of(members) {
            let name = text_of(file, method.name);
            let skipped = method.body.is_none()
                || name == "constructor"
                || method.params.iter().any(|param| param.inout)
                || method.modifiers.iter().any(|modifier| {
                    matches!(
                        modifier,
                        Modifier::Private | Modifier::Final | Modifier::Static
                    )
                });
            if skipped || !taken.insert(name) {
                continue;
            }
            let args: Vec<String> = method
                .params
                .iter()
                .map(|param| {
                    let dots = if param.variadic { "..." } else { "" };
                    format!("{dots}{}", text_of(file, param.name))
                })
                .collect();
            let call = format!("parent::{name}({})", args.join(", "));
            let returns = method
                .return_type
                .as_ref()
                .is_some_and(|ty| !matches!(text_of(file, ty.span), "void" | "never"));
            let body = if returns {
                format!("return {call};")
            } else {
                format!("{call};")
            };
            let from = parent.segments().last().map_or("", String::as_str);
            offered.push((
                format!("Override `{name}` from `{from}`"),
                place.method(&signature(analysed, file, method, place.at), &body),
            ));
        }
    }
    if offered.is_empty() {
        return Vec::new();
    }
    let every: Vec<String> = offered.iter().map(|(_, text)| text.clone()).collect();
    let all = [(place.span(analysed), place.joined(&every))];
    if compiles(analysed, documents, &all).is_none() {
        return Vec::new();
    }
    offered
        .into_iter()
        .map(|(title, text)| Offer {
            title,
            kind: OVERRIDE_KIND,
            edits: vec![(place.span(analysed), place.joined(&[text]))],
        })
        .collect()
}

/// The class declared in the entry document whose declaration `offset` is in.
fn class_named(analysed: &Analysed, offset: BytePos) -> Option<(QName, &ClassDecl)> {
    let class = class_at(analysed, offset)?;
    let (stmt, file) = declared_type(analysed, &class)?;
    if file.id() != analysed.entry {
        return None;
    }
    match &stmt.kind {
        StmtKind::ClassDecl(decl) => Some((class, decl)),
        _ => None,
    }
}

/// The methods among `members`.
fn methods_of(members: &[nvs_syntax::ast::ClassMember]) -> impl Iterator<Item = &MethodMember> {
    members.iter().filter_map(|member| match &member.kind {
        ClassMemberKind::Method(method) => Some(method),
        _ => None,
    })
}

/// The methods `class` owes: each bodiless method of a type above it that no
/// type of its lineage gives a body and that `class` does not declare, nearest
/// first and each name once.
fn missing<'a>(analysed: &'a Analysed, class: &QName) -> Vec<(&'a SourceFile, &'a MethodMember)> {
    let types: Vec<(&Stmt, &SourceFile)> = lineage(analysed, class)
        .iter()
        .filter_map(|ty| declared_type(analysed, ty))
        .collect();
    let mut answered: FxHashSet<&str> = FxHashSet::default();
    for (index, (stmt, file)) in types.iter().enumerate() {
        let Some((_, members)) = body_of(stmt) else {
            continue;
        };
        for method in methods_of(members) {
            if index == 0 || method.body.is_some() {
                answered.insert(text_of(file, method.name));
            }
        }
    }
    let mut found = Vec::new();
    for (stmt, file) in types.iter().skip(1) {
        let Some((_, members)) = body_of(stmt) else {
            continue;
        };
        for method in methods_of(members) {
            if method.body.is_none() && answered.insert(text_of(file, method.name)) {
                found.push((*file, method));
            }
        }
    }
    found
}

/// The classes `class` extends, nearest first, each with its declaration,
/// up to the first one the program does not declare.
fn parents<'a>(analysed: &'a Analysed, class: &QName) -> Vec<(QName, &'a Stmt, &'a SourceFile)> {
    let mut found = Vec::new();
    let mut seen = FxHashSet::from_iter([class.clone()]);
    let mut current = class.clone();
    while let Some(parent) = analysed
        .module
        .graph
        .get(&current)
        .and_then(|links| links.extends.first())
        .cloned()
    {
        if !seen.insert(parent.clone()) {
            break;
        }
        let Some((stmt, file)) = declared_type(analysed, &parent) else {
            break;
        };
        found.push((parent.clone(), stmt, file));
        current = parent;
    }
    found
}

/// `method`'s signature as its declaration writes it, without `abstract`, and
/// with each type written the way the entry document writes it at `offset`.
fn signature(
    analysed: &Analysed,
    file: &SourceFile,
    method: &MethodMember,
    offset: BytePos,
) -> String {
    let typed = |span: Span| {
        written_at(
            analysed,
            text_of(file, span),
            Some((file.id(), span.start)),
            offset,
        )
    };
    let visibility = if method.modifiers.contains(&Modifier::Protected) {
        "protected"
    } else {
        "public"
    };
    let params: Vec<String> = method
        .params
        .iter()
        .map(|param| {
            let whole = text_of(file, param.span);
            let Some(ty) = &param.ty else {
                return whole.to_owned();
            };
            let from = (ty.span.start - param.span.start) as usize;
            let to = (ty.span.end - param.span.start) as usize;
            match (whole.get(..from), whole.get(to..)) {
                (Some(before), Some(after)) => format!("{before}{}{after}", typed(ty.span)),
                _ => whole.to_owned(),
            }
        })
        .collect();
    let returns = method
        .return_type
        .as_ref()
        .map(|ty| format!(": {}", typed(ty.span)))
        .unwrap_or_default();
    format!(
        "{visibility} function {}({}){returns}",
        text_of(file, method.name),
        params.join(", "),
    )
}

/// Where a class's new methods are written, and how they are indented.
struct Place {
    /// The offset of the class body's `{`.
    open: BytePos,
    /// Where the methods are inserted: after the last member, or after the
    /// `{` of an empty body.
    at: BytePos,
    /// What comes before the first method: a blank line after a member, and a
    /// line break after the `{`.
    lead: &'static str,
    /// What comes after the last method: a line break and the class's
    /// indentation where the `}` shares a line with the `{`.
    tail: String,
    /// The class's indentation, plus one level.
    inner: String,
}

impl Place {
    /// Where `decl`'s new methods go in `source`.
    fn of(source: &str, decl: &ClassDecl) -> Option<Self> {
        let name_end = decl.name.span.end as usize;
        let open = name_end + source.get(name_end..decl.span.end as usize)?.find('{')?;
        let start = decl.span.start as usize;
        let line_start = source.get(..start)?.rfind('\n').map_or(0, |at| at + 1);
        let outer = source.get(line_start..start)?;
        if !outer.chars().all(|ch| ch == ' ' || ch == '\t') {
            return None;
        }
        let (at, lead, tail) = match decl.members.last() {
            // A property's or a constant's span ends before its `;`.
            Some(last) => {
                let end = last.span.end as usize;
                let rest = source.get(end..)?;
                let skipped = rest.trim_start();
                let at = match skipped.strip_prefix(';') {
                    Some(_) => end + (rest.len() - skipped.len()) + 1,
                    None => end,
                };
                (BytePos::try_from(at).ok()?, "\n\n", String::new())
            }
            None => {
                let after = source.get(open + 1..)?.trim_start_matches([' ', '\t']);
                let tail = if after.starts_with('}') {
                    format!("\n{outer}")
                } else {
                    String::new()
                };
                (BytePos::try_from(open + 1).ok()?, "\n", tail)
            }
        };
        Some(Self {
            open: BytePos::try_from(open).ok()?,
            at,
            lead,
            tail,
            inner: format!("{outer}{INDENT}"),
        })
    }

    /// A method with `signature` and the one statement `body`.
    fn method(&self, signature: &str, body: &str) -> String {
        let inner = &self.inner;
        format!("{inner}{signature}\n{inner}{{\n{inner}{INDENT}{body}\n{inner}}}")
    }

    /// `methods` as the text inserted at [`Self::at`].
    fn joined(&self, methods: &[String]) -> String {
        format!("{}{}{}", self.lead, methods.join("\n\n"), self.tail)
    }

    /// The empty span at [`Self::at`].
    fn span(&self, analysed: &Analysed) -> Span {
        Span::new(analysed.entry, self.at, self.at)
    }
}

/// `Some` when the entry document with `edits` applied analyses with no error.
fn compiles(analysed: &Analysed, documents: &Documents, edits: &[Change]) -> Option<()> {
    let mut text = analysed.map.file(analysed.entry).text().to_owned();
    let mut ordered: Vec<&Change> = edits.iter().collect();
    ordered.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    for (span, replacement) in ordered {
        text.get(span.range())?;
        text.replace_range(span.range(), replacement);
    }
    let checked = analyse_rewritten(documents, analysed, text)?;
    (checked.diags.error_count() == 0).then_some(())
}
