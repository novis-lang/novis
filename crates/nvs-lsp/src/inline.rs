//! *Inline*: a local's reads, or a method's calls, replaced with what they are
//! (`rule:tooling/nvs-code-answers-about-the-project`,
//! `rule:ide/no-refactoring-introduces-an-alias`).
//!
//! [`local`] replaces each read of a local with its initializer and deletes
//! the declaration. [`method`] replaces a call of a method with its body, with
//! each parameter replaced by the argument the call passes. Given a name, it
//! inlines every call and deletes the method. Given one call, it inlines that
//! call and keeps the method. `nvs code inline` calls both through
//! `crate::query::inline`, and the editor's `refactor.inline` actions ([`at`])
//! offer the same edits.
//!
//! # Decision: an inline is refused wherever it could change what runs
//!
//! An analysis says whether the text compiles, not whether it does what the
//! old one did. So the shape is judged first:
//!
//! - **A local** is inlined when it is declared once, with a value, and only
//!   read after that. A value with no call, `new`, assignment, increment or
//!   property read replaces every read, while none of the locals it reads is
//!   written between the declaration and the last read (to the end of a loop
//!   the read is in). Any other value replaces one read only, in the next
//!   statement, where nothing before it in that statement runs code and it
//!   runs every time the statement does ([`crate::extract::runs_first`]). A
//!   typed declaration is inlined only where the value has the same type, so
//!   the arithmetic and the calls on it stay the same.
//! - **A method** is inlined when its body is one `return <expression>;`, or
//!   when it returns `void`, its body has no `return` and declares no
//!   variable, and the call is a statement of its own. It is refused when it
//!   has an attribute (the inlined code would run without it), is abstract,
//!   overrides or is overridden, is taken as a callable value,
//!   calls itself, changes a parameter, writes `static::` or `parent::`, has
//!   a call with a diagnostic on it (a `[[restrict]]` entry's included), or
//!   writes `self::` where the call is in another class. Each argument must be
//!   a variable or a literal, since any other argument could run twice or not
//!   at all. A missing argument takes the parameter's default, which must be a
//!   literal too.
//!
//! Each replaced read or call is put in parentheses where the place it lands
//! needs them. Every edit then passes `crate::refactor::checked`, so a body
//! that reads a member the caller cannot see is refused with the checker's
//! error.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. A local costs one walk of
//! its file's tree, and one analysis of the file when the declaration writes
//! a type. A method costs one analysis of its file and of each file that calls
//! it. The check costs what `crate::refactor::checked` says. The editor's
//! actions are computed only on a variable or a method's name, and each one
//! analyses the rewritten document once more before it is offered.

use std::collections::BTreeMap;
use std::path::PathBuf;

use nvs_diagnostics::{BytePos, PositionEncoding, Span, canonical_key};
use nvs_syntax::ast::{ClassMember, ClassMemberKind, MethodMember, Modifier, Stmt, StmtKind};
use nvs_syntax::walk::{self, Node};

use crate::document::{Analysed, Documents, analyse_file, analyse_rewritten};
use crate::extract::{reindented, root, runs_first, written_to};
use crate::index::{DeclKind, Site, SymbolIndex};
use crate::position::range_at;
use crate::rename::Rewrite;
use crate::render::{Action, Edit};

/// The kind the editor's inline actions are filed under.
pub const KIND: &str = "refactor.inline";

/// Why an inline is refused, written for the person who asked.
type Why = String;

/// An edit of one file: what it replaces, and with what.
type Change = (Span, String);

/// The statement productions, in the walk's spelling.
const STATEMENTS: &[&str] = &[
    "Block",
    "Break",
    "Continue",
    "Destructure",
    "DoWhile",
    "Echo",
    "Expr",
    "For",
    "Foreach",
    "Global",
    "Goto",
    "If",
    "InlineHtml",
    "LocalDecl",
    "Return",
    "StaticLocal",
    "Switch",
    "Try",
    "Unset",
    "While",
];

/// The productions that run code, or may run it, when they are evaluated. A
/// property read is one, since a hook or an observer may run.
const RUNS_CODE: &[&str] = &[
    "Assign",
    "Await",
    "Call",
    "Clone",
    "Fn",
    "MethodCall",
    "New",
    "PostIncDec",
    "PreIncDec",
    "PropertyAccess",
    "StaticCall",
    "StaticPropertyAccess",
    "Yield",
    "YieldFrom",
];

/// The expressions that never need parentheses where an expression goes.
const ATOMS: &[&str] = &[
    "AnonObject",
    "ArrayLiteral",
    "Bool",
    "Call",
    "ClassConstAccess",
    "ClassNameConst",
    "ConstFetch",
    "Float",
    "Index",
    "Int",
    "Interpolated",
    "MethodCall",
    "Null",
    "Paren",
    "PropertyAccess",
    "StaticCall",
    "StaticPropertyAccess",
    "Str",
    "Variable",
];

/// The literal productions an argument or a default may be.
const LITERALS: &[&str] = &["Bool", "Float", "Int", "Null", "Str"];

/// The productions a loop is.
const LOOPS: &[&str] = &["DoWhile", "For", "Foreach", "While"];

/// The productions that bind a variable, which a `void` body may not have.
const BINDS: &[&str] = &[
    "Destructure",
    "Foreach",
    "Global",
    "LocalDecl",
    "StaticLocal",
    "Try",
];

/// The edits that inline the local whose variable is at `offset` of the entry
/// document of `analysed`, or why it cannot be inlined. The edits are not
/// checked: the caller passes them to `crate::refactor::checked`.
///
/// # Errors
///
/// Returns why the local is not inlined.
pub fn local(
    analysed: &Analysed,
    documents: &Documents,
    offset: BytePos,
) -> Result<Vec<Change>, Why> {
    if analysed.diags.error_count() != 0 {
        return Err(
            "The file has an error. Fix it first, because the types in it are not known."
                .to_owned(),
        );
    }
    let local = crate::locals::at(analysed, offset)
        .ok_or_else(|| "There is no local variable here.".to_owned())?;
    let name = format!("${}", local.binding.name);
    if local.is_parameter() {
        return Err(format!(
            "`{name}` is a parameter. Only a local variable can be inlined."
        ));
    }
    if local.global || local.nested.contains(&local.binding.name) {
        return Err(format!(
            "`{name}` is also used by a `global` statement or declared again in an anonymous \
             function, so it cannot be inlined."
        ));
    }
    let source = analysed.map.file(analysed.entry).text();
    let stmts = crate::imports::entry_stmts(analysed)
        .ok_or_else(|| "The file cannot be read.".to_owned())?;
    let roots = walk::of_stmts(stmts);
    let declared = local.binding.declared;
    let decl_path = path_to(&roots, &|node| {
        node.kind == "LocalDecl" && node.name == Some(declared)
    })
    .ok_or_else(|| {
        format!(
            "`{name}` is not declared with a type or `var` and a value, so it cannot be inlined."
        )
    })?;
    let decl = *decl_path
        .last()
        .ok_or_else(|| "The file cannot be read.".to_owned())?;
    let value = decl
        .children
        .first()
        .ok_or_else(|| format!("`{name}` is declared without a value."))?;

    let mut reads: Vec<Vec<&Node>> = Vec::new();
    for site in local.sites.iter().filter(|site| **site != declared) {
        let line = line_of(source, site.start);
        let path = path_to(&roots, &|node| {
            node.kind == "Variable" && node.span == *site
        })
        .ok_or_else(|| format!("`{name}` is assigned again on line {line}."))?;
        let changed = path
            .windows(2)
            .last()
            .is_some_and(|pair| written_to(pair[0], pair[1]));
        let before = source
            .get(..site.start as usize)
            .unwrap_or_default()
            .trim_end();
        if changed || before.ends_with('&') || before.ends_with("inout") {
            return Err(format!("`{name}` is changed on line {line}."));
        }
        if site.start < decl.span.end {
            return Err(format!(
                "`{name}` is read before its value is set, on line {line}."
            ));
        }
        if path
            .iter()
            .any(|node| matches!(node.kind, "Interpolated" | "HtmlTemplate"))
        {
            return Err(format!(
                "`{name}` is read inside a string on line {line}. Only a variable can go there."
            ));
        }
        reads.push(path);
    }
    if reads.is_empty() {
        return Err(format!("Nothing reads `{name}`."));
    }
    same_type(analysed, documents, decl, &local.binding.name)?;

    if runs_code(value) {
        only_next(source, &name, &decl_path, &reads)?;
    } else {
        unchanged_between(source, &name, decl, value, &reads)?;
    }

    let text = source
        .get(value.span.range())
        .ok_or_else(|| "The file cannot be read.".to_owned())?;
    let mut edits = vec![(removed_lines(source, decl.span), String::new())];
    for path in &reads {
        let read = path[path.len() - 1];
        let parent = path.len().checked_sub(2).map(|at| path[at]);
        edits.push((read.span, parenthesized(text, value.kind, parent, read)));
    }
    Ok(edits)
}

/// Refuses a typed declaration whose value has another type: the value, put
/// where the variable was, would be read as its own type.
fn same_type(
    analysed: &Analysed,
    documents: &Documents,
    decl: &Node,
    name: &str,
) -> Result<(), Why> {
    let source = analysed.map.file(analysed.entry).text();
    let Some(variable) = decl.name else {
        return Ok(());
    };
    let written = source
        .get(decl.span.start as usize..variable.start as usize)
        .unwrap_or_default()
        .trim();
    if written == "var" {
        return Ok(());
    }
    let differs = || {
        format!(
            "`${name}` is declared as `{written}`, and its value has another type. Inlining it \
             could change what the code does."
        )
    };
    let probe = format!(
        "{}var {}",
        source.get(..decl.span.start as usize).unwrap_or_default(),
        source.get(variable.start as usize..).unwrap_or_default(),
    );
    let shift = written.len().saturating_sub(3);
    let rewritten = analyse_rewritten(documents, analysed, probe).ok_or_else(differs)?;
    if rewritten.diags.error_count() != 0 {
        return Err(differs());
    }
    let before = analysed
        .exprs
        .local_ty(analysed.entry, name, decl.span.end)
        .map(|ty| analysed.interner.describe(ty));
    let at = decl
        .span
        .end
        .saturating_sub(BytePos::try_from(shift).unwrap_or(0));
    let after = rewritten
        .exprs
        .local_ty(rewritten.entry, name, at)
        .map(|ty| rewritten.interner.describe(ty));
    if before.is_none() || before != after {
        return Err(differs());
    }
    Ok(())
}

/// Refuses a value that runs code unless it has one read, in the statement
/// right after the declaration, at a place that runs first.
fn only_next(
    source: &str,
    name: &str,
    decl_path: &[&Node],
    reads: &[Vec<&Node>],
) -> Result<(), Why> {
    let moved = || {
        format!(
            "The value of `{name}` runs code, and it would run at another time at its read. \
             It can only be inlined into one read at the start of the next statement."
        )
    };
    let [path] = reads else {
        return Err(format!(
            "The value of `{name}` runs code, and `{name}` is read {} times. It can only be \
             inlined into one read.",
            reads.len()
        ));
    };
    let decl = decl_path[decl_path.len() - 1];
    let siblings: &[Node] = match decl_path.len().checked_sub(2) {
        Some(at) => &decl_path[at].children,
        None => return Err(moved()),
    };
    let next = siblings
        .iter()
        .find(|node| node.span.start >= decl.span.end && STATEMENTS.contains(&node.kind))
        .ok_or_else(moved)?;
    let at = path
        .iter()
        .position(|node| node.span == next.span && node.kind == next.kind)
        .ok_or_else(moved)?;
    let mut child = path[path.len() - 1];
    for ancestor in path[at..path.len() - 1].iter().rev() {
        let first = ancestor
            .children
            .first()
            .is_some_and(|first| first.span == child.span);
        let fine = if ancestor.span == next.span {
            match ancestor.kind {
                "Expr" | "Return" | "Echo" | "LocalDecl" => true,
                "If" | "Switch" | "Foreach" => first,
                _ => false,
            }
        } else {
            ancestor.kind != "Fn" && runs_first(ancestor, child, source)
        };
        let earlier_runs_code = ancestor
            .children
            .iter()
            .take_while(|sibling| sibling.span != child.span)
            .any(runs_code);
        if !fine || earlier_runs_code {
            return Err(moved());
        }
        child = ancestor;
    }
    Ok(())
}

/// Refuses a value whose locals are written between the declaration and the
/// last read, or the end of a loop a read is in.
fn unchanged_between(
    source: &str,
    name: &str,
    decl: &Node,
    value: &Node,
    reads: &[Vec<&Node>],
) -> Result<(), Why> {
    let mut read_names: Vec<&str> = Vec::new();
    for node in std::iter::once(value).chain(value.descendants()) {
        if node.kind == "Variable"
            && let Some(text) = node.name.and_then(|span| source.get(span.range()))
            && text != "$this"
        {
            read_names.push(text);
        }
    }
    if read_names.is_empty() {
        return Ok(());
    }
    let mut end = decl.span.end;
    for path in reads {
        let read = path[path.len() - 1];
        end = end.max(read.span.end);
        if let Some(outer) = path
            .iter()
            .find(|node| LOOPS.contains(&node.kind) && node.span.start > decl.span.start)
        {
            end = end.max(outer.span.end);
        }
    }
    let Some(top) = reads.first().and_then(|path| path.first()) else {
        return Ok(());
    };
    for node in std::iter::once(*top).chain(top.descendants()) {
        if node.span.start < decl.span.end || node.span.start >= end {
            continue;
        }
        let target = match node.kind {
            "Assign" | "PreIncDec" | "PostIncDec" => node
                .children
                .first()
                .and_then(|target| root(target, source)),
            "LocalDecl" | "Foreach" => node.name.and_then(|span| source.get(span.range())),
            _ => None,
        };
        let by_reference = node.kind == "Variable"
            && node
                .name
                .and_then(|span| source.get(span.range()))
                .is_some_and(|text| {
                    let before = source.get(..node.span.start as usize).unwrap_or_default();
                    read_names.contains(&text)
                        && (before.trim_end().ends_with('&')
                            || before.trim_end().ends_with("inout"))
                });
        if let Some(changed) = target.filter(|target| read_names.contains(target)) {
            return Err(format!(
                "`{changed}` changes on line {} before the last read of `{name}`, so the value \
                 would not be the same.",
                line_of(source, node.span.start)
            ));
        }
        if by_reference {
            return Err(format!(
                "A variable the value of `{name}` reads is passed as `inout` on line {}.",
                line_of(source, node.span.start)
            ));
        }
    }
    Ok(())
}

/// The edits that inline the method `symbol`: every call, and the method's
/// deletion, or only the call whose name starts at `call`. Every edit is
/// checked before it is returned.
///
/// # Errors
///
/// Returns why the method is not inlined.
pub fn method(
    documents: &Documents,
    index: &SymbolIndex,
    symbol: &str,
    call: Option<&Site>,
) -> Result<Vec<Rewrite>, Why> {
    let declared = index
        .declaration(symbol)
        .filter(|declared| declared.kind == DeclKind::Method)
        .ok_or_else(|| format!("`{symbol}` is not a method of this project."))?;
    if declared.attributed {
        return Err(format!(
            "`{symbol}` has an attribute, which would not apply to the inlined code."
        ));
    }
    if !index.overridden(symbol).is_empty() || !index.overriders(symbol).is_empty() {
        return Err(format!(
            "`{symbol}` overrides another method or is overridden, so a call may run another \
             body."
        ));
    }
    if index.unfollowed(symbol) {
        return Err(format!(
            "`{symbol}` is used as a value, or called where its class is not known, so not every \
             call can be inlined."
        ));
    }
    let calls = index.occurrences(symbol);
    if calls
        .iter()
        .any(|used| used.caller.as_deref() == Some(symbol))
    {
        return Err(format!("`{symbol}` calls itself, so it cannot be inlined."));
    }
    let chosen: Vec<_> = match call {
        Some(site) => calls
            .into_iter()
            .filter(|used| used.site == *site)
            .collect(),
        None => calls,
    };
    if chosen.is_empty() {
        return Err(match call {
            Some(_) => format!("There is no call of `{symbol}` here."),
            None => format!("Nothing calls `{symbol}`. Delete it with `nvs code delete`."),
        });
    }
    if call.is_none() && !index.mentions_of(symbol).is_empty() {
        return Err(format!(
            "A doc comment names `{symbol}` with `@see` or `@throws`, so it cannot be deleted."
        ));
    }

    let path = declared.site.path.clone();
    let analysed = analyse_file(documents, &path, 0)
        .ok_or_else(|| format!("`{}` cannot be read.", path.display()))?;
    let body = Body::of(&analysed, declared.site.start, symbol)?;
    let class = symbol.rsplit_once("::").map_or("", |(class, _)| class);

    let mut by_file: BTreeMap<PathBuf, Vec<&Site>> = BTreeMap::new();
    for used in &chosen {
        if body.writes_self {
            let caller_class = used
                .caller
                .as_deref()
                .and_then(|caller| caller.rsplit_once("::"))
                .map(|(class, _)| class);
            if caller_class != Some(class) {
                return Err(format!(
                    "The body of `{symbol}` writes `self::`, and a call is in another class."
                ));
            }
        }
        by_file
            .entry(used.site.path.clone())
            .or_default()
            .push(&used.site);
    }
    let mut rewrites = Vec::new();
    for (file, sites) in by_file {
        let read = if file == path {
            None
        } else {
            Some(
                analyse_file(documents, &file, 0)
                    .ok_or_else(|| format!("`{}` cannot be read.", file.display()))?,
            )
        };
        let calling = read.as_ref().unwrap_or(&analysed);
        let source = calling.map.file(calling.entry).text();
        let stmts = crate::imports::entry_stmts(calling)
            .ok_or_else(|| format!("`{}` cannot be read.", file.display()))?;
        let roots = walk::of_stmts(stmts);
        for site in sites {
            // A call with a diagnostic on it, such as one a `[[restrict]]`
            // entry does not allow there, would lose that diagnostic.
            if calling.diags.iter().any(|diagnostic| {
                diagnostic.primary_span().is_some_and(|span| {
                    span.file == calling.entry && span.start <= site.end && site.start <= span.end
                })
            }) {
                return Err(format!(
                    "The call of `{symbol}` on line {} has a problem the checker reports. Fix it \
                     first.",
                    line_of(source, site.start)
                ));
            }
            let (span, text) = body.inlined(&roots, source, site.start, symbol)?;
            rewrites.push(Rewrite {
                site: Site {
                    path: file.clone(),
                    start: span.start,
                    end: span.end,
                },
                text,
            });
        }
    }
    if call.is_none() {
        let text = analysed.map.file(analysed.entry).text();
        let (from, to) =
            crate::delete::removed(text, body.member.start as usize, body.member.end as usize);
        rewrites.push(Rewrite {
            site: Site {
                path,
                start: byte(from),
                end: byte(to),
            },
            text: String::new(),
        });
    }
    crate::refactor::checked(documents, index, rewrites, &[])
        .map(|checked| checked.rewrites)
        .map_err(|broken| broken.refusal())
}

/// A method's body, read once from its file, ready to be written at a call.
struct Body {
    /// The whole member, its modifiers included.
    member: Span,
    /// Whether the method is `static`.
    is_static: bool,
    /// Each parameter's name with its `$`, and its default's text.
    params: Vec<(String, Option<String>)>,
    /// The body's text: the returned expression, or the statements.
    text: String,

    /// The returned expression's kind, or `None` for a `void` body.
    returns: Option<&'static str>,
    /// The indentation of the body's statements.
    indent: String,
    /// Each variable the body reads, as its place in [`text`](Self::text) and
    /// its name.
    variables: Vec<(usize, usize, String)>,
    /// Whether the body writes `self::`.
    writes_self: bool,
}

impl Body {
    /// The body of the method whose name starts at `name_at` in the entry
    /// document of `analysed`, or why it cannot be inlined.
    fn of(analysed: &Analysed, name_at: BytePos, symbol: &str) -> Result<Self, Why> {
        let source = analysed.map.file(analysed.entry).text();
        let unread = || format!("`{symbol}` cannot be read.");
        let stmts = crate::imports::entry_stmts(analysed).ok_or_else(unread)?;
        let (member, method) = method_member(stmts, name_at).ok_or_else(unread)?;
        if method.modifiers.contains(&Modifier::Abstract) {
            return Err(format!("`{symbol}` is abstract, so it has no body."));
        }
        let block = method
            .body
            .as_ref()
            .ok_or_else(|| format!("`{symbol}` has no body."))?;
        let mut params = Vec::new();
        for param in &method.params {
            let name = source
                .get(param.name.range())
                .ok_or_else(unread)?
                .to_owned();
            if param.variadic || param.inout || param.is_promoted() {
                return Err(format!(
                    "The parameter `{name}` of `{symbol}` is variadic, `inout` or a property."
                ));
            }
            let default = match &param.default {
                None => None,
                Some(default) => {
                    let text = source.get(default.span.range()).ok_or_else(unread)?;
                    let node = walk::of_source("default", &format!("<?nvs\n{text};"))
                        .ok()
                        .and_then(|file| first_expr(&file).map(|node| node.kind));
                    if !node.is_some_and(|kind| LITERALS.contains(&kind)) {
                        return Err(format!(
                            "The default of `{name}` is not a literal, so it may mean something \
                             else at a call."
                        ));
                    }
                    Some(text.to_owned())
                }
            };
            params.push((name, default));
        }
        let returns_void = method
            .return_type
            .as_ref()
            .and_then(|ty| source.get(ty.span.range()))
            .is_some_and(|ty| ty.trim() == "void");

        let roots = walk::of_stmts(stmts);
        let method_node = path_to(&roots, &|node| {
            node.kind == "Method" && node.span.start == member.start
        })
        .and_then(|path| path.last().copied())
        .ok_or_else(unread)?;
        let statements: Vec<&Node> = method_node
            .children
            .iter()
            .filter(|node| node.span.start >= block.span.start && STATEMENTS.contains(&node.kind))
            .collect();
        let shape = || {
            format!(
                "The body of `{symbol}` must be one `return` statement, or it must return `void` \
                 without a `return`."
            )
        };
        let (region, returns) = match statements.as_slice() {
            [only] if only.kind == "Return" => {
                let value = only.children.first().ok_or_else(shape)?;
                (value.span, Some(value.kind))
            }
            all if returns_void => {
                let start = all
                    .first()
                    .map_or(block.span.start, |first| first.span.start);
                let end = all.last().map_or(start, |last| last.span.end);
                (Span::new(analysed.entry, start, end), None)
            }
            _ => return Err(shape()),
        };
        let nodes: Vec<&Node> = statements
            .iter()
            .flat_map(|stmt| std::iter::once(*stmt).chain(stmt.descendants()))
            .collect();
        for node in &nodes {
            if node.kind == "Fn" {
                return Err(format!("The body of `{symbol}` has an anonymous function."));
            }
            if returns.is_none() && (node.kind == "Return" || BINDS.contains(&node.kind)) {
                return Err(format!(
                    "The body of `{symbol}` returns early or declares a variable, so it cannot be \
                     written at a call."
                ));
            }
            if matches!(node.kind, "Assign" | "PreIncDec" | "PostIncDec")
                && let Some(target) = node
                    .children
                    .first()
                    .and_then(|target| root(target, source))
                && params.iter().any(|(name, _)| name == target)
            {
                return Err(format!("`{symbol}` changes its parameter `{target}`."));
            }
        }
        let text = source.get(region.range()).ok_or_else(unread)?.to_owned();
        if text.contains("static::") || text.contains("parent::") {
            return Err(format!(
                "The body of `{symbol}` writes `static::` or `parent::`, which would mean another \
                 class at a call."
            ));
        }
        if text.contains("inout") || text.contains("&$") {
            return Err(format!(
                "The body of `{symbol}` passes an `inout` argument."
            ));
        }
        let variables = nodes
            .iter()
            .filter(|node| node.kind == "Variable" && inside(region, node.span))
            .filter_map(|node| {
                let span = node.name.unwrap_or(node.span);
                let name = source.get(span.range())?;
                Some((
                    (span.start - region.start) as usize,
                    (span.end - region.start) as usize,
                    name.to_owned(),
                ))
            })
            .collect();
        let line = source
            .get(..region.start as usize)
            .unwrap_or_default()
            .rfind('\n')
            .map_or(0, |at| at + 1);
        let indent = source
            .get(line..region.start as usize)
            .unwrap_or_default()
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .collect();
        Ok(Self {
            member,
            is_static: method.modifiers.contains(&Modifier::Static),
            params,
            writes_self: text.contains("self::"),
            text,

            returns,
            indent,
            variables,
        })
    }

    /// The edit that writes this body at the call whose name starts at
    /// `name_at`, in the file of `roots` and `source`.
    fn inlined(
        &self,
        roots: &[Node],
        source: &str,
        name_at: BytePos,
        symbol: &str,
    ) -> Result<Change, Why> {
        let line = line_of(source, name_at);
        let path = path_to(roots, &|node| {
            matches!(node.kind, "MethodCall" | "StaticCall")
                && node.name.is_some_and(|name| name.start == name_at)
        })
        .ok_or_else(|| format!("The call of `{symbol}` on line {line} cannot be read."))?;
        let call = path[path.len() - 1];
        let parent = path.len().checked_sub(2).map(|at| path[at]);
        let opening = call.name.map_or(call.span.start, |name| name.end);
        let between = source
            .get(call.span.start as usize..opening as usize)
            .unwrap_or_default();
        if call.kind == "MethodCall" && between.contains("?->") {
            return Err(format!(
                "The call of `{symbol}` on line {line} uses `?->`, so it may not run."
            ));
        }
        let mut children = call.children.iter();
        let receiver = children.next();
        let receiver_text = match (call.kind, receiver) {
            ("MethodCall", Some(receiver)) if receiver.kind == "Variable" => {
                source.get(receiver.span.range()).map(str::to_owned)
            }
            ("MethodCall", _) => {
                if self.text.contains("$this") {
                    return Err(format!(
                        "The body of `{symbol}` reads `$this`, and the call on line {line} is not \
                         on a variable."
                    ));
                }
                None
            }
            _ => None,
        };
        let mut given: Vec<Option<String>> = vec![None; self.params.len()];
        let mut position = 0;
        let mut previous = opening;
        for arg in children.filter(|node| node.span.start >= opening) {
            let before = source
                .get(previous as usize..arg.span.start as usize)
                .unwrap_or_default();
            previous = arg.span.end;
            let lead =
                before.trim_start_matches(|ch: char| ch == '(' || ch == ',' || ch.is_whitespace());
            if lead.starts_with("...") || lead.starts_with("inout") || lead.starts_with('&') {
                return Err(format!(
                    "The call of `{symbol}` on line {line} spreads an argument or passes one as \
                     `inout`."
                ));
            }
            let text = source.get(arg.span.range()).unwrap_or_default();
            let literal = LITERALS.contains(&arg.kind)
                || (arg.kind == "Unary"
                    && arg
                        .children
                        .iter()
                        .all(|node| LITERALS.contains(&node.kind)));
            if !(arg.kind == "Variable" || literal) {
                return Err(format!(
                    "The argument `{text}` on line {line} is not a variable or a literal. It \
                     could run twice or not at all."
                ));
            }
            let written = if arg.kind == "Unary" {
                format!("({text})")
            } else {
                text.to_owned()
            };
            let named = lead.trim_end().strip_suffix(':').map(str::trim);
            let at = match named {
                Some(label) => self
                    .params
                    .iter()
                    .position(|(param, _)| param.get(1..) == Some(label)),
                None => {
                    position += 1;
                    Some(position - 1)
                }
            };
            let slot = at
                .and_then(|at| given.get_mut(at))
                .ok_or_else(|| format!("The call of `{symbol}` on line {line} cannot be read."))?;
            *slot = Some(written);
        }
        let mut values: BTreeMap<&str, String> = BTreeMap::new();
        for ((param, default), value) in self.params.iter().zip(given) {
            let value = value.or_else(|| default.clone()).ok_or_else(|| {
                format!("The call of `{symbol}` on line {line} gives no value for `{param}`.")
            })?;
            values.insert(param, value);
        }
        if let Some(receiver) = &receiver_text
            && !self.is_static
        {
            values.insert("$this", receiver.clone());
        }
        let mut text = String::new();
        let mut done = 0;
        for (start, end, name) in &self.variables {
            if let Some(value) = values.get(name.as_str()) {
                text.push_str(self.text.get(done..*start).unwrap_or_default());
                text.push_str(value);
                done = *end;
            }
        }
        text.push_str(self.text.get(done..).unwrap_or_default());

        match self.returns {
            Some(kind) => Ok((call.span, parenthesized(&text, kind, parent, call))),
            None => {
                let statement = parent
                    .filter(|parent| parent.kind == "Expr")
                    .ok_or_else(|| {
                        format!(
                            "`{symbol}` returns `void`, and its call on line {line} is not a \
                             statement of its own."
                        )
                    })?;
                let line_start = source
                    .get(..statement.span.start as usize)
                    .unwrap_or_default()
                    .rfind('\n')
                    .map_or(0, |at| at + 1);
                let indent = source
                    .get(line_start..statement.span.start as usize)
                    .unwrap_or_default();
                if !indent.chars().all(|ch| ch == ' ' || ch == '\t') {
                    return Err(format!(
                        "The call of `{symbol}` on line {line} shares its line with other code."
                    ));
                }

                if text.trim().is_empty() {
                    return Ok((removed_lines(source, statement.span), String::new()));
                }
                Ok((statement.span, reindented(&text, &self.indent, indent)))
            }
        }
    }
}

/// The member of a class, an interface or an enum in `stmts` that declares a
/// method whose name starts at `name_at`, with the member's span.
pub(crate) fn method_member(stmts: &[Stmt], name_at: BytePos) -> Option<(Span, &MethodMember)> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(namespace) => namespace
            .body
            .as_ref()
            .and_then(|body| method_member(&body.stmts, name_at)),
        StmtKind::ClassDecl(decl) => in_members(&decl.members, name_at),
        StmtKind::InterfaceDecl(decl) => in_members(&decl.members, name_at),
        StmtKind::EnumDecl(decl) => in_members(&decl.members, name_at),
        _ => None,
    })
}

/// [`method_member`] over the members of one declaration.
fn in_members(members: &[ClassMember], name_at: BytePos) -> Option<(Span, &MethodMember)> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(method) if method.name.start == name_at => {
            Some((member.span, method))
        }
        ClassMemberKind::Enum(decl) => in_members(&decl.members, name_at),
        _ => None,
    })
}

/// The editor's inline actions at `offset` of the entry document: *Inline
/// variable* on a local, *Inline this call* on a call of a project method,
/// and *Inline method* on a method's name when every call is in this
/// document. Each is offered only when its edit is in this document and the
/// document still compiles after it.
#[must_use]
pub fn at(
    analysed: &Analysed,
    documents: &Documents,
    index: Option<&SymbolIndex>,
    offset: BytePos,
    encoding: PositionEncoding,
) -> Vec<Action> {
    let file = analysed.map.file(analysed.entry);
    let mut offered = Vec::new();
    if let Some(local) = crate::locals::at(analysed, offset)
        && let Ok(edits) = self::local(analysed, documents, offset)
        && compiles(analysed, documents, &edits)
    {
        offered.push(action(
            format!("Inline variable `${}`", local.binding.name),
            &edits,
            analysed,
            encoding,
        ));
    }
    let (Some(index), Some(path)) = (index, file.path().map(canonical_key)) else {
        return offered;
    };
    let source = file.text();
    let (title, symbol, site) = if let Some(name) = call_name_at(analysed, offset) {
        let Some(used) = index
            .occurrences_in(&path)
            .iter()
            .find(|used| used.site.start == name.start)
        else {
            return offered;
        };
        let short = source.get(name.range()).unwrap_or_default();
        (
            format!("Inline this call to `{short}`"),
            used.symbol.clone(),
            Some(used.site.clone()),
        )
    } else if let Some((symbol, _)) = crate::index::declared_at(analysed, offset) {
        let short = symbol
            .rsplit_once("::")
            .map_or(symbol.as_str(), |(_, name)| name);
        (format!("Inline method `{short}`"), symbol.clone(), None)
    } else {
        return offered;
    };
    let Ok(rewrites) = method(documents, index, &symbol, site.as_ref()) else {
        return offered;
    };
    if rewrites.iter().any(|rewrite| rewrite.site.path != path) {
        return offered;
    }
    let edits: Vec<Change> = rewrites
        .into_iter()
        .map(|rewrite| {
            (
                Span::new(analysed.entry, rewrite.site.start, rewrite.site.end),
                rewrite.text,
            )
        })
        .collect();
    offered.push(action(title, &edits, analysed, encoding));
    offered
}

/// Whether [`at`] could offer to inline a call at `offset`: the name of a
/// method call is there. The `.lspt` suite builds an index only for such a
/// case, or for a declaration's name.
#[must_use]
pub fn wanted(analysed: &Analysed, offset: BytePos) -> bool {
    call_name_at(analysed, offset).is_some()
}

/// The name of the method call at `offset` of the entry document.
fn call_name_at(analysed: &Analysed, offset: BytePos) -> Option<Span> {
    let stmts = crate::imports::entry_stmts(analysed)?;
    let roots = walk::of_stmts(stmts);
    let path = path_to(&roots, &|node| {
        matches!(node.kind, "MethodCall" | "StaticCall")
            && node
                .name
                .is_some_and(|name| name.start <= offset && offset <= name.end)
    })?;
    path.last().and_then(|call| call.name)
}

/// One action that applies `edits`, the first as its main edit.
fn action(
    title: String,
    edits: &[Change],
    analysed: &Analysed,
    encoding: PositionEncoding,
) -> Action {
    let file = analysed.map.file(analysed.entry);
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|(span, _)| span.start);
    let (first, rest) = sorted.split_first().map_or_else(
        || ((Span::new(analysed.entry, 0, 0), String::new()), &[][..]),
        |(first, rest)| (first.clone(), rest),
    );
    Action {
        title,
        kind: KIND.to_owned(),
        range: range_at(file, first.0, encoding),
        replacement: first.1,
        also: rest
            .iter()
            .map(|(span, replacement)| Edit {
                range: range_at(file, *span, encoding),
                replacement: replacement.clone(),
            })
            .collect(),
    }
}

/// Whether the entry document of `analysed` reports no error once `edits`
/// are applied.
fn compiles(analysed: &Analysed, documents: &Documents, edits: &[Change]) -> bool {
    let mut text = analysed.map.file(analysed.entry).text().to_owned();
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    for (span, replacement) in sorted {
        if text.get(span.range()).is_none() {
            return false;
        }
        text.replace_range(span.range(), &replacement);
    }
    analyse_rewritten(documents, analysed, text).is_some_and(|after| after.diags.error_count() == 0)
}

/// Whether evaluating `node` may run code.
pub(crate) fn runs_code(node: &Node) -> bool {
    std::iter::once(node)
        .chain(node.descendants())
        .any(|node| RUNS_CODE.contains(&node.kind))
}

/// `text`, an expression of `kind`, as it is written where `child` of
/// `parent` was: in parentheses unless it is an atom or the place needs none.
fn parenthesized(text: &str, kind: &str, parent: Option<&Node>, child: &Node) -> String {
    let first = parent
        .and_then(|parent| parent.children.first())
        .is_some_and(|first| first.span == child.span);
    let bare = ATOMS.contains(&kind)
        || parent.is_none_or(|parent| {
            STATEMENTS.contains(&parent.kind)
                || matches!(parent.kind, "Paren" | "ArrayLiteral")
                || (matches!(
                    parent.kind,
                    "Call" | "MethodCall" | "StaticCall" | "New" | "Assign"
                ) && !first)
        });
    if bare {
        text.to_owned()
    } else {
        format!("({text})")
    }
}

/// The nodes from a root of `roots` down to the first node `wanted` accepts,
/// outermost first.
pub(crate) fn path_to<'a>(
    roots: &'a [Node],
    wanted: &dyn Fn(&Node) -> bool,
) -> Option<Vec<&'a Node>> {
    for root in roots {
        if wanted(root) {
            return Some(vec![root]);
        }
        if let Some(mut below) = path_to(&root.children, wanted) {
            below.insert(0, root);
            return Some(below);
        }
    }
    None
}

/// The first expression of a file's tree, under its statement.
fn first_expr(file: &Node) -> Option<&Node> {
    file.descendants()
        .into_iter()
        .find(|node| !STATEMENTS.contains(&node.kind) && node.kind != "Whitespace")
}

/// Whether `inner` is inside `outer`.
const fn inside(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// The bytes a statement at `span` leaves when it is deleted: its whole line
/// when nothing else is on it, and the statement alone otherwise.
fn removed_lines(source: &str, span: Span) -> Span {
    let start = span.start as usize;
    let end = span.end as usize;
    let line_start = source
        .get(..start)
        .and_then(|head| head.rfind('\n'))
        .map_or(0, |at| at + 1);
    let line_end = source
        .get(end..)
        .and_then(|tail| tail.find('\n'))
        .map_or(source.len(), |at| end + at + 1);
    let alone = source
        .get(line_start..start)
        .is_some_and(|text| text.trim().is_empty())
        && source
            .get(end..line_end)
            .is_some_and(|text| text.trim().is_empty());
    if alone {
        Span::new(span.file, byte(line_start), byte(line_end))
    } else {
        span
    }
}

/// The line of `offset` in `source`, counted from 1.
pub(crate) fn line_of(source: &str, offset: BytePos) -> usize {
    source
        .get(..offset as usize)
        .map_or(0, |head| head.matches('\n').count())
        + 1
}

/// A byte offset as a position.
fn byte(offset: usize) -> BytePos {
    BytePos::try_from(offset).unwrap_or(BytePos::MAX)
}
