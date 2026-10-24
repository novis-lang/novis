//! *Extract to variable* and *Extract to method*, offered over a selection
//! (`rule:ide/no-refactoring-introduces-an-alias`).
//!
//! Extract to variable writes a typed local declaration above the statement
//! the selected expression is in, and replaces the selection with the
//! variable. Extract to method writes a `private` method after the one the
//! selection is in, whose parameters are the locals the selection reads, and
//! replaces the selection with a call to it. A selection is one expression or
//! a run of whole statements of one list; a run extracts to a method that
//! returns `void`.
//!
//! # Decision: every type is one the checker recorded, and every edit is checked
//!
//! A parameter's type is the type its local was declared with
//! (`nvs_types::ExprTypeTable::bodies_at`). An expression's type is read by
//! writing `var $name = <selection>;` above its statement and analysing that
//! text: the checker fixes the binding's type, which is the type
//! `rule:types/var-inference` gives it. Before either action is offered, the
//! text it writes is analysed too, and an action whose text reports an error is
//! not offered. A type the checker narrowed at the selection, a name written
//! where it does not resolve, or a declaration the rest of the body still reads
//! is therefore never offered wrong. That is
//! `rule:ide/a-code-action-writes-only-what-is-already-determined`: a document
//! that already reports an error is offered neither action, since its types
//! are the checker's stand-ins.
//!
//! # Decision: what is refused without asking the checker
//!
//! An analysis says whether a text compiles, not whether it does what the old
//! one did. So these are refused by shape:
//!
//! - **Extract to variable**, where moving the expression above its statement
//!   changes when it runs: in a loop's condition, an `elseif`, the right side
//!   of `&&`, `||` or `??`, a branch of `?:` or `match`, after `?->`, and in an
//!   anonymous function's body.
//! - **Extract to method**, for a selection with an early exit or a write that
//!   escapes it: a `return`, `yield` or `goto`, a `break` or `continue` with no
//!   loop around it inside the selection, an assignment to a local declared
//!   before the selection, and an `inout` or `&` argument. A selection inside an
//!   anonymous function, and one holding one, is refused too, because its
//!   variables and `$this` are the function's.
//! - **Both**, for a selection that is written to (an assignment's target, an
//!   increment, `isset`, `empty`, `unset`) and for a bare variable.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. Per `textDocument/codeAction`
//! over a selection that is exactly one expression, up to three analyses of the
//! entry's graph — the probe, the variable and the method — and one for a run of
//! statements. An empty selection, a selection on no expression, and a document
//! that reports an error cost one walk of the entry's tree and no analysis.
//! Everything is dropped with the answer.

use nvs_diagnostics::{BytePos, PositionEncoding, Span};
use nvs_syntax::walk::{self, Field, Node};
use rustc_hash::FxHashMap;

use crate::document::{Analysed, Documents, analyse_rewritten};
use crate::position::range_at;
use crate::render::{Action, Edit};

/// The kind both actions are filed under.
pub const KIND: &str = "refactor.extract";

/// The title of the action that writes a local declaration.
pub const VARIABLE: &str = "Extract to variable";

/// The title of the action that writes a method.
pub const METHOD: &str = "Extract to method";

/// One level of indentation, the way `nvs fmt` writes it.
const INDENT: &str = "    ";

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

/// The productions whose statements are a list a declaration can be written
/// into. A file's own statements are one too.
const OWNERS: &[&str] = &["Block", "Case", "Fn", "Method", "NamespaceDecl", "Try"];

/// The expressions either action extracts.
const EXTRACTABLE: &[&str] = &[
    "AnonObject",
    "ArrayLiteral",
    "Await",
    "Binary",
    "Bool",
    "Call",
    "ClassConstAccess",
    "ClassNameConst",
    "Clone",
    "ConstFetch",
    "Conversion",
    "Duration",
    "Float",
    "HtmlTemplate",
    "Index",
    "Int",
    "Interpolated",
    "Match",
    "MethodCall",
    "New",
    "Null",
    "Paren",
    "PropertyAccess",
    "StaticCall",
    "StaticPropertyAccess",
    "Str",
    "Ternary",
    "TypeTest",
    "Unary",
];

/// The productions `break` and `continue` leave.
const LOOPS: &[&str] = &["DoWhile", "For", "Foreach", "Switch", "While"];

/// The productions extract to method refuses anywhere in the selection.
const EXITS: &[&str] = &[
    "Destructure",
    "Fn",
    "Global",
    "Goto",
    "InlineHtml",
    "Return",
    "StaticLocal",
    "Unset",
    "Yield",
    "YieldFrom",
];

/// Both extractions offered over `[start, end)` of the entry document.
///
/// Empty for an empty range, which is a cursor and selects nothing.
#[must_use]
pub fn at(
    analysed: &Analysed,
    documents: &Documents,
    start: BytePos,
    end: BytePos,
    encoding: PositionEncoding,
) -> Vec<Action> {
    let file = analysed.map.file(analysed.entry);
    let source = file.text();
    let Some(selection) =
        trimmed(source, start, end).map(|(start, end)| Span::new(analysed.entry, start, end))
    else {
        return Vec::new();
    };
    if analysed.diags.error_count() != 0 {
        return Vec::new();
    }
    let Some(stmts) = crate::imports::entry_stmts(analysed) else {
        return Vec::new();
    };
    let roots = walk::of_stmts(stmts);
    let Some(selected) = find(&roots, selection) else {
        return Vec::new();
    };
    let context = Context {
        analysed,
        documents,
        source,
        selection,
    };
    let mut offered = Vec::new();
    match selected {
        Selected::Expr { path, node } => {
            let probe = context.probe(&path, node);
            if let Some(probe) = &probe
                && probe.moves_safely
            {
                offered.extend(context.variable(probe));
            }
            if let Some(probe) = &probe {
                offered.extend(context.method(&path, std::slice::from_ref(node), Some(&probe.ty)));
            }
        }
        Selected::Stmts { path, run } => offered.extend(context.method(&path, run, None)),
    }
    offered
        .into_iter()
        .map(|(title, (span, replacement), also)| Action {
            title: title.to_owned(),
            kind: KIND.to_owned(),
            range: range_at(file, span, encoding),
            replacement,
            also: also
                .into_iter()
                .map(|(span, replacement)| Edit {
                    range: range_at(file, span, encoding),
                    replacement,
                })
                .collect(),
        })
        .collect()
}

/// What a selection is.
enum Selected<'a> {
    /// One expression, and every node it is inside, outermost first.
    Expr { path: Vec<&'a Node>, node: &'a Node },
    /// A run of statements of one list, and every node the list is inside,
    /// outermost first. The last is the list's owner, and none is a file's own
    /// statements.
    Stmts {
        path: Vec<&'a Node>,
        run: &'a [Node],
    },
}

/// What the selection over `selection` is, if either action extracts it.
fn find(roots: &[Node], selection: Span) -> Option<Selected<'_>> {
    let mut path: Vec<&Node> = Vec::new();
    let mut level = roots;
    loop {
        if let Some(run) = run_of(level, selection) {
            let listed = path.last().is_none_or(|owner| OWNERS.contains(&owner.kind));
            return listed.then_some(Selected::Stmts { path, run });
        }
        let child = level
            .iter()
            .find(|node| node.span.start <= selection.start && selection.end <= node.span.end)?;
        if child.span.start == selection.start
            && child.span.end == selection.end
            && !STATEMENTS.contains(&child.kind)
        {
            return EXTRACTABLE
                .contains(&child.kind)
                .then_some(Selected::Expr { path, node: child });
        }
        path.push(child);
        level = &child.children;
    }
}

/// The statements of `level` that start where `selection` starts and end where
/// it ends, if they are all statements.
fn run_of(level: &[Node], selection: Span) -> Option<&[Node]> {
    let first = level
        .iter()
        .position(|node| node.span.start == selection.start)?;
    let last = level
        .iter()
        .position(|node| node.span.end == selection.end)?;
    let run = level.get(first..=last)?;
    run.iter()
        .all(|node| STATEMENTS.contains(&node.kind))
        .then_some(run)
}

/// An edit: what it replaces, and with what.
type Change = (Span, String);

/// An action before it is placed: its title, its main edit and the others.
type Offer = (&'static str, Change, Vec<Change>);

/// The type of the selected expression, read off the probe, and where its
/// statement starts.
struct Probe {
    /// The type, written the way the entry document writes it.
    ty: String,
    /// Where the line of the statement starts.
    line_start: usize,
    /// The whitespace before the statement on that line.
    indent: String,
    /// The variable's name.
    name: String,
    /// Whether the expression runs exactly once, right before its statement,
    /// so it may move above the statement.
    moves_safely: bool,
}

/// What every step reads.
struct Context<'a> {
    /// The analysis the request was answered from.
    analysed: &'a Analysed,
    /// The open buffers, which an analysis of rewritten text reads too.
    documents: &'a Documents,
    /// The entry document's text.
    source: &'a str,
    /// The selection, its whitespace trimmed.
    selection: Span,
}

impl Context<'_> {
    /// The selected expression's type, read by analysing the document with
    /// `var $name = <selection>;` written above its statement.
    ///
    /// `None` where no statement is around the expression, where that
    /// statement shares its line with other code, and where the probe does not
    /// compile.
    fn probe(&self, path: &[&Node], node: &Node) -> Option<Probe> {
        let source = self.source;
        let mut moves_safely = true;
        let mut child = node;
        let mut statement = None;
        for (depth, ancestor) in path.iter().enumerate().rev() {
            if STATEMENTS.contains(&ancestor.kind) {
                let parent = depth.checked_sub(1).map(|above| path[above]);
                if parent.is_some_and(|parent| !OWNERS.contains(&parent.kind)) {
                    return None;
                }
                let first = ancestor
                    .children
                    .first()
                    .is_some_and(|first| first.span == child.span);
                moves_safely &= match ancestor.kind {
                    "Expr" | "Return" | "Echo" | "LocalDecl" => true,
                    "If" | "Switch" | "Foreach" => first,
                    _ => false,
                };
                statement = Some(*ancestor);
                break;
            }
            if ancestor.kind == "Fn" {
                return None;
            }
            if written_to(ancestor, child) {
                return None;
            }
            moves_safely &= runs_first(ancestor, child, source);
            child = ancestor;
        }
        let statement = statement?;
        let start = statement.span.start as usize;
        let line_start = source.get(..start)?.rfind('\n').map_or(0, |at| at + 1);
        let indent = source.get(line_start..start)?;
        if !indent.chars().all(|ch| ch == ' ' || ch == '\t') {
            return None;
        }
        let name = variable_name(node, source);
        let probe = self.declared(line_start, indent, "var", &name)?;
        let rewritten = analyse_rewritten(self.documents, self.analysed, probe)?;
        if rewritten.diags.error_count() != 0 {
            return None;
        }
        let at = BytePos::try_from(start).ok()?;
        let ty = rewritten
            .exprs
            .bodies_at(rewritten.entry, at)
            .into_iter()
            .find_map(|locals| locals.iter().find(|local| local.name == name))?
            .ty;
        Some(Probe {
            ty: self.written(&rewritten.interner.describe(ty), at),
            line_start,
            indent: indent.to_owned(),
            name,
            moves_safely,
        })
    }

    /// The document with `<ty> $<name> = <selection>;` on its own line above
    /// the statement, and the selection replaced by the variable.
    fn declared(&self, line_start: usize, indent: &str, ty: &str, name: &str) -> Option<String> {
        let source = self.source;
        let selection = self.selection.range();
        let expression = source.get(selection.clone())?;
        Some(format!(
            "{}{indent}{ty} ${name} = {expression};\n{}${name}{}",
            source.get(..line_start)?,
            source.get(line_start..selection.start)?,
            source.get(selection.end..)?,
        ))
    }

    /// *Extract to variable*, once the typed declaration compiles.
    fn variable(&self, probe: &Probe) -> Option<Offer> {
        let text = self.declared(probe.line_start, &probe.indent, &probe.ty, &probe.name)?;
        let checked = analyse_rewritten(self.documents, self.analysed, text)?;
        if checked.diags.error_count() != 0 {
            return None;
        }
        let selection = self.selection.range();
        let replacement = format!(
            "{}{} ${} = {};\n{}${}",
            probe.indent,
            probe.ty,
            probe.name,
            self.source.get(selection.clone())?,
            self.source.get(probe.line_start..selection.start)?,
            probe.name,
        );
        let span = Span::new(
            self.analysed.entry,
            BytePos::try_from(probe.line_start).ok()?,
            self.selection.end,
        );
        Some((VARIABLE, (span, replacement), Vec::new()))
    }

    /// *Extract to method* for `region`, the one expression or the run of
    /// statements selected, once the text it writes compiles.
    ///
    /// `returns` is the expression's type, and `None` for a run of statements,
    /// which returns `void`.
    fn method(&self, path: &[&Node], region: &[Node], returns: Option<&str>) -> Option<Offer> {
        let source = self.source;
        let method = path.iter().rposition(|node| node.kind == "Method")?;
        if path[method..].iter().any(|node| node.kind == "Fn") {
            return None;
        }
        let method = path[method];
        let selected = source.get(self.selection.range())?;
        if selected.contains("inout") || selected.contains("&$") {
            return None;
        }
        let mut uses = Uses::default();
        for node in region {
            uses.walk(node, 0, source)?;
        }
        let mut params = Vec::new();
        let mut args = Vec::new();
        for name in &uses.read {
            if uses.declared.contains(name) {
                continue;
            }
            let local = name.strip_prefix('$')?;
            let ty = self
                .analysed
                .exprs
                .bodies_at(self.analysed.entry, self.selection.start)
                .into_iter()
                .find_map(|locals| {
                    locals.iter().find(|binding| {
                        binding.name == local && binding.declared.start < self.selection.start
                    })
                })?
                .ty;
            let ty = self.written(&self.analysed.interner.describe(ty), self.selection.start);
            params.push(format!("{ty} {name}"));
            args.push(name.clone());
        }
        if uses
            .written
            .iter()
            .any(|name| !uses.declared.contains(name))
        {
            return None;
        }

        let name_at = method.name?;
        let is_static = source
            .get(method.span.start as usize..name_at.start as usize)?
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|word| word == "static");
        let start = method.span.start as usize;
        let line_start = source.get(..start)?.rfind('\n').map_or(0, |at| at + 1);
        let outer = source.get(line_start..start)?;
        if !outer.chars().all(|ch| ch == ' ' || ch == '\t') {
            return None;
        }
        let inner = format!("{outer}{INDENT}");
        let selection = self.selection.range();
        let line = source
            .get(..selection.start)?
            .rfind('\n')
            .map_or(0, |at| at + 1);
        let from: String = source
            .get(line..selection.start)?
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .collect();

        let called = unique(source, "extracted", false);
        let args = args.join(", ");
        let call = if is_static {
            format!("self::{called}({args})")
        } else {
            format!("$this->{called}({args})")
        };
        let (body, call, ret) = match returns {
            Some(ty) => (format!("return {selected};"), call, ty),
            None => (selected.to_owned(), format!("{call};"), "void"),
        };
        let body = reindented(&body, &from, &inner);
        let modifiers = if is_static {
            "private static"
        } else {
            "private"
        };
        let written = format!(
            "\n\n{outer}{modifiers} function {called}({}): {ret} {{\n{inner}{body}\n{outer}}}",
            params.join(", "),
        );
        let after = method.span.end as usize;
        let text = format!(
            "{}{call}{}{written}{}",
            source.get(..selection.start)?,
            source.get(selection.end..after)?,
            source.get(after..)?,
        );
        let checked = analyse_rewritten(self.documents, self.analysed, text)?;
        if checked.diags.error_count() != 0 {
            return None;
        }
        let end = Span::new(self.analysed.entry, method.span.end, method.span.end);
        Some((METHOD, (self.selection, call), vec![(end, written)]))
    }

    /// `described`, a type as the checker prints it, with each qualified name
    /// written the way the entry document writes it at `offset`.
    fn written(&self, described: &str, offset: BytePos) -> String {
        let short: FxHashMap<String, String> = crate::definition::imports_of(self.analysed)
            .into_iter()
            .map(|(name, target)| (target.to_string(), name))
            .collect();
        let here = crate::definition::namespace_at(self.analysed, offset);
        let mut out = String::new();
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String| {
            if word.contains('\\') {
                out.push_str(&crate::completion::written_as(word, &here, &short));
            } else {
                out.push_str(word);
            }
            word.clear();
        };
        for ch in described.chars() {
            if ch.is_alphanumeric() || ch == '_' || ch == '\\' {
                word.push(ch);
            } else {
                flush(&mut word, &mut out);
                out.push(ch);
            }
        }
        flush(&mut word, &mut out);
        out
    }
}

/// The locals a selection reads, writes and declares, each written with its
/// `$`, in the order the selection first writes each.
#[derive(Default)]
struct Uses {
    /// Every local read or written, `$this` left out.
    read: Vec<String>,
    /// Every local assigned to or incremented.
    written: Vec<String>,
    /// Every local the selection declares.
    declared: Vec<String>,
}

impl Uses {
    /// Records `node` and everything under it. `loops` counts the loops around
    /// it inside the selection. `None` is an early exit, which refuses the
    /// whole selection.
    fn walk(&mut self, node: &Node, loops: usize, source: &str) -> Option<()> {
        if EXITS.contains(&node.kind) {
            return None;
        }
        if matches!(node.kind, "Break" | "Continue") && (loops == 0 || !node.children.is_empty()) {
            return None;
        }
        let named = node.name.and_then(|span| source.get(span.range()));
        match node.kind {
            "Variable" => {
                if let Some(name) = named.filter(|name| *name != "$this") {
                    push_once(&mut self.read, name);
                }
            }
            "LocalDecl" => push_once(&mut self.declared, named?),
            "Assign" | "PreIncDec" | "PostIncDec" => {
                if let Some(name) = node
                    .children
                    .first()
                    .and_then(|target| root(target, source))
                {
                    push_once(&mut self.written, name);
                }
            }
            _ => {}
        }
        let inner = loops + usize::from(LOOPS.contains(&node.kind));
        for child in &node.children {
            self.walk(child, inner, source)?;
        }
        Some(())
    }
}

/// Adds `name` to `names` unless it is there already.
fn push_once(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|known| known == name) {
        names.push(name.to_owned());
    }
}

/// The local an assignment to `target` changes: the variable itself, or the
/// variable an offset is written into. An array is a value, so writing one of
/// its offsets changes the local.
fn root<'a>(target: &Node, source: &'a str) -> Option<&'a str> {
    match target.kind {
        "Variable" => target.name.and_then(|span| source.get(span.range())),
        "Index" => root(target.children.first()?, source),
        _ => None,
    }
}

/// Whether `child`, inside `ancestor`, is a place the program writes to or
/// only tests, which a variable or a call cannot replace.
fn written_to(ancestor: &Node, child: &Node) -> bool {
    let first = ancestor
        .children
        .first()
        .is_some_and(|first| first.span == child.span);
    match ancestor.kind {
        "Assign" | "PreIncDec" | "PostIncDec" => first,
        "Isset" | "Empty" | "IsInit" | "Unset" => true,
        _ => false,
    }
}

/// Whether `child` runs every time `ancestor` does, and before the rest of it.
fn runs_first(ancestor: &Node, child: &Node, source: &str) -> bool {
    let first = ancestor
        .children
        .first()
        .is_some_and(|first| first.span == child.span);
    let op = ancestor.fields.iter().find_map(|(key, field)| match field {
        Field::Word(word) if *key == "op" => Some(*word),
        _ => None,
    });
    match ancestor.kind {
        "Binary" if matches!(op, Some("And" | "Or" | "Coalesce")) => first,
        "Assign" if op.is_some_and(|op| op.starts_with("Coalesce")) => first,
        "Ternary" | "Match" => first,
        "Catch" => false,
        "MethodCall" | "PropertyAccess" => source
            .get(ancestor.span.start as usize..child.span.start as usize)
            .is_some_and(|before| !before.contains("?->")),
        _ => true,
    }
}

/// The variable's name: the member a call or a property access names, without
/// a `get` in front, or `value`. A number is added until the name is not in the
/// document.
fn variable_name(node: &Node, source: &str) -> String {
    let member = match node.kind {
        "MethodCall" | "StaticCall" | "PropertyAccess" | "StaticPropertyAccess" => node
            .name
            .and_then(|span| source.get(span.range()))
            .map(|name| name.trim_start_matches('$')),
        _ => None,
    };
    let base = member
        .filter(|name| {
            name.chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic())
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
        .map(|name| match name.strip_prefix("get") {
            Some(rest) if rest.starts_with(|ch: char| ch.is_ascii_uppercase()) => {
                let mut chars = rest.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_lowercase().to_string() + chars.as_str()
                })
            }
            _ => name.to_owned(),
        })
        .unwrap_or_else(|| "value".to_owned());
    unique(source, &base, true)
}

/// `base`, or `base` with the first number from 2 up that makes it a name the
/// document does not write. `variable` says whether it is written with `$`.
fn unique(source: &str, base: &str, variable: bool) -> String {
    let mut candidate = base.to_owned();
    let mut number = 2;
    while writes(source, &candidate, variable) {
        candidate = format!("{base}{number}");
        number += 1;
    }
    candidate
}

/// Whether `source` writes `name` as a whole word, with `$` when `variable`.
fn writes(source: &str, name: &str, variable: bool) -> bool {
    let word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
    let needle = if variable {
        format!("${name}")
    } else {
        name.to_owned()
    };
    source.match_indices(&needle).any(|(at, _)| {
        let before = source.get(..at).and_then(|text| text.chars().next_back());
        let after = source
            .get(at + needle.len()..)
            .and_then(|text| text.chars().next());
        !after.is_some_and(word) && (variable || !before.is_some_and(|ch| word(ch) || ch == '$'))
    })
}

/// `text` with every line after the first moved from the indentation `from`
/// to `to`. A blank line stays blank.
fn reindented(text: &str, from: &str, to: &str) -> String {
    let mut lines = text.split('\n');
    let mut out = lines.next().unwrap_or_default().to_owned();
    for line in lines {
        out.push('\n');
        if line.trim().is_empty() {
            continue;
        }
        out.push_str(to);
        out.push_str(line.strip_prefix(from).unwrap_or_else(|| line.trim_start()));
    }
    out
}

/// `[start, end)` without the whitespace at either end, or `None` when nothing
/// is left.
fn trimmed(source: &str, start: BytePos, end: BytePos) -> Option<(BytePos, BytePos)> {
    let text = source.get(start as usize..end as usize)?;
    let lead = BytePos::try_from(text.len() - text.trim_start().len()).ok()?;
    let trail = BytePos::try_from(text.len() - text.trim_end().len()).ok()?;
    let (start, end) = (start + lead, end - trail);
    (start < end).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::{reindented, trimmed, unique};

    #[test]
    fn a_name_the_document_writes_gets_a_number() {
        let source = "$total = 1; $total2 = 2; $totals = 3;";
        assert_eq!(unique(source, "total", true), "total3");
        assert_eq!(unique(source, "totals2", true), "totals2");
        assert_eq!(
            unique("function extracted() {}", "extracted", false),
            "extracted2"
        );
        assert_eq!(unique("$extracted = 1;", "extracted", false), "extracted");
    }

    #[test]
    fn a_selection_loses_its_whitespace_and_an_empty_one_is_nothing() {
        assert_eq!(trimmed("  $a + $b \n", 0, 11), Some((2, 9)));
        assert_eq!(trimmed("   ", 0, 3), None);
        assert_eq!(trimmed("$a", 1, 1), None);
    }

    #[test]
    fn a_body_moves_to_the_new_indentation() {
        let text = "foo(\n        $a,\n\n    );";
        assert_eq!(
            reindented(text, "    ", "        "),
            "foo(\n            $a,\n\n        );"
        );
    }
}
