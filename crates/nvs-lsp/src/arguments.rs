//! A string literal written as an argument whose parameter says what its text
//! names: a file path, or a class's whole name.
//!
//! The mark is the parameter's, never the literal's. A `Core` row states it
//! with `nvs_stdlib::registry::CoreTy::Path` or `CoreTy::ClassName`, a user
//! method states a path with `#[Core\Path]` on the parameter, and a field of a
//! `Core` shape carries it on `nvs_types::CoreShapeField::text`
//! (`rule:programs/path-literals-resolve-from-their-file`). The checker keeps
//! the parameter's mark on the call it resolved
//! (`nvs_types::ResolvedCall::param_text`), so this module reads that and
//! decides nothing of its own: a literal at a parameter that carries no mark is
//! ordinary text, and nothing here answers inside it.
//!
//! **Which parameter a literal fills is the checker's answer too.** The
//! literal is one of the call's argument nodes, counted from the call's own
//! `(` the way `textDocument/signatureHelp` counts them ([`opens_at`]), and
//! `nvs_types::ResolvedCall::arg_slots` maps that written position to the
//! parameter it fills, which is what makes a named argument land where its
//! name says.
//!
//! **A shape field is reached through its parameter's type.** The literal is
//! the value of one field of a `{...}` literal written as an argument. The
//! parameter's type is a `nvs_types::Ty::CoreShape`, and the field is looked
//! up by the key written in front of the value. A key has no node of its own
//! in the index, so it is read off the text between the previous field's end
//! and the value, which is a name and a `:` and nothing else.
//!
//! **A completion file's parameter is named, not marked.** [`named_at`]
//! answers for a literal at any parameter, marked or not, with the declaring
//! class, the method and the parameter that a completion file's attachment is
//! keyed by (`rule:ide/completion-files-offer-values-at-named-parameters`),
//! and the text of every other argument that is one string literal, which a
//! `when` compares.
//!
//! **Where a path literal leads is the checker's join.** A relative literal at
//! a path parameter was resolved while checking, and
//! `nvs_types::ExprTypeTable::path_literal` keeps the absolute path under the
//! literal's span ([`target`]). Joining the text here again would be a second
//! implementation of the rule, and `crate::links`' module doc says why a
//! second one is refused. An absolute literal names itself.

use std::path::{Path, PathBuf};

use nvs_diagnostics::{BytePos, Span};
use nvs_hir::QName;
use nvs_stdlib::registry::ParamText;
use nvs_syntax::walk;
use nvs_types::expr_table::ArgSlot;
use nvs_types::{CoreShapeField, ExprInfo, ResolvedCall, Ty, TypeId};

use crate::definition::text_of;
use crate::document::Analysed;

/// The members whose class-name parameter names an error to expect, which is
/// a `Throwable`: the class and the member, as the registry spells them.
///
/// A class-name parameter says only that its text is a class's whole name.
/// What the member does with that name is in its reference card, so the one
/// member that wants an error class is named here rather than in the mark.
const THROWN: &[(&str, &str)] = &[("Core\\Test", "assertThrows")];

/// The call kinds an argument is written inside, as `nvs_syntax::walk` spells
/// them.
const CALLS: &[&str] = &["Call", "MethodCall", "StaticCall", "New"];

/// One string literal at a marked parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Argument {
    /// The literal, quotes included.
    pub span: Span,
    /// What its parameter's text names: never [`ParamText::Plain`].
    pub text: ParamText,
    /// For a class name, whether the parameter names an error class to
    /// expect, so only a `Throwable` fits it.
    pub thrown: bool,
}

/// A string literal argument named the way a completion file names its
/// parameter: by the class that declares the method the checker resolved the
/// call to, the method, and the parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Named {
    /// The literal, quotes included.
    pub span: Span,
    /// The declaring class's whole name, with no leading `\`.
    pub class: String,
    /// The method's name, `constructor` for the one a `new` runs.
    pub method: String,
    /// The parameter's name, without its `$`.
    pub parameter: String,
    /// Every other argument the call writes for a parameter, by that
    /// parameter's name: the text of a lone string literal, and `None` for any
    /// other expression. A parameter the call leaves out is not here.
    pub others: Vec<(String, Option<String>)>,
}

/// A node that holds the literal, with the spans and kinds of its own
/// children.
struct Holder {
    kind: &'static str,
    span: Span,
    children: Vec<Span>,
    child_kinds: Vec<&'static str>,
}

/// The marked argument literal the cursor at `offset` is inside, if any. The
/// cursor has to be between the quotes.
pub(crate) fn at(analysed: &Analysed, offset: BytePos) -> Option<Argument> {
    let (literal, holders) = holders_at(analysed, offset)?;
    classify(analysed, literal, &holders)
}

/// The string literal argument the cursor at `offset` is inside, named by its
/// parameter, whatever the parameter's mark. The cursor has to be between the
/// quotes, and the literal has to be an argument of a call the checker
/// resolved, written for a parameter it named.
pub(crate) fn named_at(analysed: &Analysed, offset: BytePos) -> Option<Named> {
    let (literal, holders) = holders_at(analysed, offset)?;
    let raw = text_of(analysed.map.file(analysed.entry), literal);
    if !(raw.starts_with('\'') || raw.starts_with('"')) {
        return None;
    }
    let parent = holders
        .first()
        .filter(|parent| CALLS.contains(&parent.kind))?;
    let call = call_at(analysed, parent.span)?;
    let index = parameter(analysed, call, parent, literal)?;
    let open = opens_at(analysed, parent.span, &parent.children)?;
    let file = analysed.map.file(analysed.entry);
    let others = parent
        .children
        .iter()
        .zip(&parent.child_kinds)
        .filter(|(child, _)| child.start >= open)
        .zip(&call.arg_slots)
        .filter_map(|((child, kind), slot)| match slot {
            ArgSlot::Param(other) if *other != index => {
                let text = (*kind == "Str")
                    .then(|| text_of(file, *child))
                    .filter(|raw| raw.starts_with('\'') || raw.starts_with('"'))
                    .map(|_| nvs_syntax::string_lit::cook_string_literal(file, *child));
                Some((call.param_names.get(*other)?.clone(), text))
            }
            _ => None,
        })
        .collect();
    Some(Named {
        span: literal,
        class: call.class.to_string(),
        method: call.method.clone(),
        parameter: call.param_names.get(index)?.clone(),
        others,
    })
}

/// The string literal the cursor at `offset` is inside, and the node that
/// holds it and the one that holds that.
fn holders_at(analysed: &Analysed, offset: BytePos) -> Option<(Span, Vec<Holder>)> {
    let path = analysed.index.at(offset);
    let [string, rest @ ..] = path.nodes() else {
        return None;
    };
    if string.kind != "Str" || !(string.span.start < offset && offset < string.span.end) {
        return None;
    }
    let holders = rest
        .iter()
        .take(2)
        .map(|node| {
            let children = analysed.index.children_of(*node);
            Holder {
                kind: node.kind,
                span: node.span,
                children: children.iter().map(|child| child.span).collect(),
                child_kinds: children.iter().map(|child| child.kind).collect(),
            }
        })
        .collect();
    Some((string.span, holders))
}

/// Every marked argument literal the entry document writes, in the order they
/// were written.
pub(crate) fn in_document(analysed: &Analysed) -> Vec<Argument> {
    let Some(loaded) = analysed
        .loaded
        .iter()
        .find(|loaded| loaded.id == analysed.entry)
    else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for root in &walk::of_stmts(&loaded.stmts) {
        visit(analysed, root, &mut Vec::new(), &mut found);
    }
    found.sort_by_key(|argument| argument.span.start);
    found
}

/// [`in_document`]'s walk: `node` and everything under it, with the nodes
/// that hold it in `above`, innermost last.
fn visit<'n>(
    analysed: &Analysed,
    node: &'n walk::Node,
    above: &mut Vec<&'n walk::Node>,
    found: &mut Vec<Argument>,
) {
    if node.kind == "Str" {
        let holders: Vec<Holder> = above
            .iter()
            .rev()
            .take(2)
            .map(|holder| Holder {
                kind: holder.kind,
                span: holder.span,
                children: holder.children.iter().map(|child| child.span).collect(),
                child_kinds: holder.children.iter().map(|child| child.kind).collect(),
            })
            .collect();
        found.extend(classify(analysed, node.span, &holders));
        return;
    }
    above.push(node);
    for child in &node.children {
        visit(analysed, child, above, found);
    }
    above.pop();
}

/// What the parameter the literal at `literal` fills says about its text,
/// given the node that holds it and the one that holds that.
fn classify(analysed: &Analysed, literal: Span, holders: &[Holder]) -> Option<Argument> {
    let raw = text_of(analysed.map.file(analysed.entry), literal);
    if !(raw.starts_with('\'') || raw.starts_with('"')) {
        return None;
    }
    let parent = holders.first()?;
    let (call, text) = if CALLS.contains(&parent.kind) {
        let call = call_at(analysed, parent.span)?;
        let index = parameter(analysed, call, parent, literal)?;
        (call, call.text_at(index))
    } else if parent.kind == "ObjectLiteral" {
        let holder = holders
            .get(1)
            .filter(|holder| CALLS.contains(&holder.kind))?;
        let call = call_at(analysed, holder.span)?;
        let index = parameter(analysed, call, holder, parent.span)?;
        let key = field_key(analysed, parent, literal)?;
        (
            call,
            shape_field(analysed, *call.param_tys.get(index)?, key)?,
        )
    } else {
        return None;
    };
    if text == ParamText::Plain {
        return None;
    }
    let thrown = text == ParamText::ClassName
        && THROWN
            .iter()
            .any(|(class, method)| call.class == QName::parse(class) && call.method == *method);
    Some(Argument {
        span: literal,
        text,
        thrown,
    })
}

/// The call the checker resolved at `span`: a function, method or static
/// call, or the constructor a `new` runs.
fn call_at(analysed: &Analysed, span: Span) -> Option<&ResolvedCall> {
    match analysed.exprs.lookup(span)? {
        ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => Some(call),
        ExprInfo::New {
            ctor: Some(call), ..
        } => Some(call),
        _ => None,
    }
}

/// The index of the parameter the argument at `argument` fills.
fn parameter(
    analysed: &Analysed,
    call: &ResolvedCall,
    holder: &Holder,
    argument: Span,
) -> Option<usize> {
    let open = opens_at(analysed, holder.span, &holder.children)?;
    let written = holder
        .children
        .iter()
        .filter(|child| child.start >= open)
        .position(|child| *child == argument)?;
    match call.arg_slots.get(written)? {
        ArgSlot::Param(index) => Some(*index),
        ArgSlot::Spread(_) | ArgSlot::Unresolved => None,
    }
}

/// Where the argument list of the call at `call` opens: the first `(` in it
/// that none of its `children` covers.
///
/// Every form that carries a `nvs_types::ResolvedCall` writes what it calls as
/// a name or as an expression the index holds as a child — `$this->of()->take(`,
/// `Str::take(`, `new User(` — so a parenthesis belonging to the callee is
/// inside that child's span and the first one left over is the argument
/// list's. `None` for a call whose parenthesis the parser never saw.
pub(crate) fn opens_at(analysed: &Analysed, call: Span, children: &[Span]) -> Option<BytePos> {
    let text = text_of(analysed.map.file(analysed.entry), call);
    text.char_indices()
        .filter(|(_, ch)| *ch == '(')
        .filter_map(|(at, _)| u32::try_from(at).ok())
        .map(|at| call.start + at)
        .find(|at| !children.iter().any(|child| child.contains(*at)))
}

/// The key written in front of the field value at `value` inside the `{...}`
/// literal `object`.
fn field_key<'a>(analysed: &'a Analysed, object: &Holder, value: Span) -> Option<&'a str> {
    let from = object
        .children
        .iter()
        .filter(|child| child.end <= value.start)
        .map(|child| child.end)
        .max()
        .unwrap_or(object.span.start);
    let text = analysed.map.file(analysed.entry).text();
    let between = text.get(from as usize..value.start as usize)?;
    let key = between
        .trim()
        .trim_start_matches(['{', ','])
        .trim()
        .strip_suffix(':')?
        .trim();
    (!key.is_empty() && key.chars().all(|ch| ch.is_alphanumeric() || ch == '_')).then_some(key)
}

/// What the text of the field `key` names, in the `Core` shape a parameter of
/// type `ty` takes.
///
/// The type may be a union of shapes, as `Db\Settings` is: one arm per
/// driver, told apart by a field. The first arm that declares `key` answers,
/// since a key that two arms share is one field written the same way in
/// both.
fn shape_field(analysed: &Analysed, ty: TypeId, key: &str) -> Option<ParamText> {
    let interner = &analysed.interner;
    let members = match interner.get(ty) {
        Ty::Union(members) => members.clone(),
        _ => vec![ty],
    };
    members
        .into_iter()
        .find_map(|member| match interner.get(member) {
            Ty::CoreShape(shape) => shape
                .fields
                .iter()
                .find(|field: &&CoreShapeField| field.name == key)
                .map(|field| field.text),
            _ => None,
        })
}

/// The absolute path a path argument names: the checker's join for a
/// relative literal, and the literal itself for an absolute one. `None` for a
/// relative literal the checker left alone, which names no file it could find
/// (`nvs_types::paths::resolved`).
pub(crate) fn target(analysed: &Analysed, argument: &Argument) -> Option<PathBuf> {
    if let Some(joined) = analysed.exprs.path_literal(argument.span) {
        return Some(PathBuf::from(joined));
    }
    let text = nvs_syntax::string_lit::cook_string_literal(
        analysed.map.file(analysed.entry),
        argument.span,
    );
    Path::new(&text).is_absolute().then(|| PathBuf::from(text))
}
