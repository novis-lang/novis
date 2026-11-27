//! *Implement the missing methods*, *Override a method*, *Declare the method
//! you called* and *Declare the attribute*, the four generators
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
//! Declare the attribute is a quick fix on the `E0303` of a `#[Name]` that
//! nothing declares, and writes `attribute Name on <site> { … }` at file scope.
//!
//! # Decision: what declare the attribute writes
//!
//! One declaration, after the `namespace` and `use` lines in force where the
//! attribute is used, so its name resolves to the one the use writes. Its site
//! is the declaration the use is on: `class`, `method`, `property` or
//! `parameter`, and a use anywhere else gets nothing, since no retrieval reads
//! another site. Each field is a key the use writes, typed from its value as
//! written: `bool`, `int`, `float`, `string`, and `array<T>` for a list whose
//! values all have type `T`. Any other value is typed `string`. The edit is
//! checked like the others, so a field typed wrongly that way leaves the
//! action out. A qualified name and the member form `Owner::Name` get
//! nothing, and so does a file with the shape an autoloaded file has
//! (`nvs_hir::autoload::sole_declaration`), where a second declaration is
//! `E0317`.
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
//! ([`crate::refactor::adds_no_error_in_entry`],
//! `rule:ide/a-refactoring-checks-its-edit-before-it-writes`), and an action
//! whose text adds an error is not offered. An error the document already had
//! does not hide it. That covers what this module does not compute: a generic interface,
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
//! The same list is a completion row where a member of the class is being
//! written ([`overridable_at`], read by `crate::completion`). There the
//! document is half-written, so the text is checked for an error inside what
//! it writes ([`writes_cleanly`]) and not for an error anywhere.
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
use nvs_syntax::ast::{
    Attribute, AttributeGroup, AttributeSite, ClassDecl, ClassMember, ClassMemberKind, Expr,
    ExprKind, MethodMember, Modifier, Stmt, StmtKind,
};
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

/// The title of the quick fix that declares an attribute nothing declares.
pub const DECLARE_ATTRIBUTE: &str = "Declare the attribute";

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
        .chain(declare_attribute(analysed, documents, start, end))
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
    let edits = implemented(analysed, documents, diagnostic.primary_span()?.start).ok()?;
    Some(Offer {
        title: IMPLEMENT.to_owned(),
        kind: QUICK_FIX,
        edits,
    })
}

/// The edits that write every method the class declared at `offset` owes, or
/// why there are none to write.
fn implemented(
    analysed: &Analysed,
    documents: &Documents,
    offset: BytePos,
) -> Result<Vec<Change>, &'static str> {
    let (class, decl) = class_named(analysed, offset).ok_or("is not a class")?;
    let methods = missing(analysed, &class);
    if methods.is_empty() {
        return Err("already has every method that its interfaces and parent classes need");
    }
    let entry = analysed.map.file(analysed.entry);
    let unplaced = "is written in a layout where the methods have no place to go";
    let place = Place::of(entry.text(), decl).ok_or(unplaced)?;
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
    let edits = with_not_written(analysed, &place, place.joined(&written)).ok_or(unplaced)?;
    compiles(analysed, documents, &edits)
        .ok_or("needs methods that cannot be written without an error")?;
    Ok(edits)
}

/// Every method the class `symbol` owes, written as *Implement the missing
/// methods* writes it, and checked by [`crate::refactor::checked`] before it
/// is returned. This is `nvs code implement`.
///
/// # Errors
///
/// Returns why nothing is written: the name is no class of the project, the
/// class owes no method, or the edit adds an error.
pub fn implement_class(
    documents: &Documents,
    index: &crate::SymbolIndex,
    symbol: &str,
) -> Result<Vec<crate::rename::Rewrite>, String> {
    let declared = index
        .declaration(symbol)
        .ok_or_else(|| format!("`{symbol}` is not declared in this project."))?;
    let path = declared.site.path.clone();
    let analysed = crate::document::analyse_file(documents, &path, 0)
        .ok_or_else(|| format!("The file of `{symbol}` cannot be read."))?;
    let edits = implemented(&analysed, documents, declared.site.start)
        .map_err(|why| format!("`{symbol}` {why}, so nothing was changed."))?;
    let rewrites = edits
        .into_iter()
        .map(|(span, text)| crate::rename::Rewrite {
            site: crate::Site {
                path: path.clone(),
                start: span.start,
                end: span.end,
            },
            text,
        })
        .collect();
    crate::refactor::checked(documents, index, rewrites, &[])
        .map(|checked| checked.rewrites)
        .map_err(|broken| broken.refusal())
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

/// *Declare the attribute*, where the `E0303` of an attribute's name meets the
/// range. The module doc says what it writes.
fn declare_attribute(
    analysed: &Analysed,
    documents: &Documents,
    start: BytePos,
    end: BytePos,
) -> Option<Offer> {
    let entry = analysed.map.file(analysed.entry);
    let stmts = crate::imports::entry_stmts(analysed)?;
    if nvs_hir::autoload::sole_declaration(stmts, entry).is_some() {
        return None;
    }
    let (attribute, site) = phase_gated(&analysed.diags)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == Some(code::E_UNDEFINED_CLASS))
        .filter_map(|diagnostic| diagnostic.primary_span())
        .filter(|span| span.file == analysed.entry && span.start <= end && start <= span.end)
        .find_map(|span| used_at(stmts, span))?;
    let name = text_of(entry, attribute.name.as_ref()?.span);
    if name.contains('\\') {
        return None;
    }
    let fields: Vec<String> = attribute
        .fields
        .iter()
        .map(|field| {
            let ty = value_type(&field.value).unwrap_or_else(|| "string".to_owned());
            format!("{}: {ty}", text_of(entry, field.name))
        })
        .collect();
    let body = if fields.is_empty() {
        "{}".to_owned()
    } else {
        format!("{{ {} }}", fields.join(", "))
    };
    let site_word = site.word();
    let place = nvs_hir::import_site(stmts, entry, attribute.span.start)?;
    let after = entry.text().get(place.at as usize..)?;
    let rest = after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'))
        .unwrap_or(after);
    let tail = if rest
        .lines()
        .next()
        .is_none_or(|line| line.trim().is_empty())
    {
        ""
    } else {
        "\n"
    };
    let edits = vec![(
        place.span(analysed.entry),
        format!("\n\nattribute {name} on {site_word} {body}{tail}"),
    )];
    compiles(analysed, documents, &edits)?;
    Some(Offer {
        title: DECLARE_ATTRIBUTE.to_owned(),
        kind: QUICK_FIX,
        edits,
    })
}

/// The attribute among `stmts` whose plain name is written at `name`, and the
/// site it is used on, where that site is one a program's declaration may
/// list.
fn used_at(stmts: &[Stmt], name: Span) -> Option<(&Attribute, AttributeSite)> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::NamespaceDecl(namespace) => used_at(&namespace.body.as_ref()?.stmts, name),
        StmtKind::ClassDecl(decl) => named(&decl.attributes, name)
            .map(|attribute| (attribute, AttributeSite::Class))
            .or_else(|| used_in_members(&decl.members, name)),
        StmtKind::InterfaceDecl(decl) => used_in_members(&decl.members, name),
        StmtKind::EnumDecl(decl) => used_in_members(&decl.members, name),
        _ => None,
    })
}

/// [`used_at`] over the members of a class, interface or enum body.
fn used_in_members(members: &[ClassMember], name: Span) -> Option<(&Attribute, AttributeSite)> {
    members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Property(property) => {
            named(&property.attributes, name).map(|attribute| (attribute, AttributeSite::Property))
        }
        ClassMemberKind::Method(method) => named(&method.attributes, name)
            .map(|attribute| (attribute, AttributeSite::Method))
            .or_else(|| {
                method.params.iter().find_map(|param| {
                    named(&param.attributes, name)
                        .map(|attribute| (attribute, AttributeSite::Parameter))
                })
            }),
        ClassMemberKind::Enum(decl) => used_in_members(&decl.members, name),
        _ => None,
    })
}

/// The attribute in `groups` whose plain name is written at `name`.
fn named(groups: &[AttributeGroup], name: Span) -> Option<&Attribute> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attribute| {
            attribute.member.is_none()
                && attribute
                    .name
                    .as_ref()
                    .is_some_and(|written| written.span == name)
        })
}

/// The type a payload value has as written, or `None` for a value whose type
/// is not read off its syntax alone.
fn value_type(value: &Expr) -> Option<String> {
    Some(match &value.kind {
        ExprKind::Bool(_) => "bool".to_owned(),
        ExprKind::Int(_) => "int".to_owned(),
        ExprKind::Float(_) => "float".to_owned(),
        ExprKind::Str(_) | ExprKind::Interpolated(_) => "string".to_owned(),
        ExprKind::ArrayLiteral(items) => {
            let mut types = items.iter().map(|item| {
                if item.key.is_some() || item.spread {
                    return None;
                }
                value_type(&item.value)
            });
            let first = types.next()??;
            if !types.all(|ty| ty.as_deref() == Some(first.as_str())) {
                return None;
            }
            format!("array<{first}>")
        }
        _ => return None,
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
    let offered: Vec<(String, String)> = overridable(analysed, &class, decl, place.at, None)
        .into_iter()
        .map(|method| {
            let from = method.parent.segments().last().map_or("", String::as_str);
            (
                format!("Override `{}` from `{from}`", method.name),
                place.method(&method.signature(), &method.body),
            )
        })
        .collect();
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

/// One method a class may override, with its signature written the way the
/// class's own file names each type.
pub(crate) struct Override {
    /// The method's name.
    pub(crate) name: String,
    /// The class above that declares the method.
    pub(crate) parent: QName,
    /// `public` or `protected`, as the parent declares it.
    visibility: &'static str,
    /// Each parameter as its declaration writes it.
    pub(crate) params: Vec<String>,
    /// The return type, where the parent declares one.
    pub(crate) returns: Option<String>,
    /// The body's one statement: the call to `parent::`, returned unless the
    /// return type is `void` or `never`.
    body: String,
}

impl Override {
    /// `public function name(params): type`.
    fn signature(&self) -> String {
        let returns = self
            .returns
            .as_ref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();
        format!(
            "{} function {}({}){returns}",
            self.visibility,
            self.name,
            self.params.join(", ")
        )
    }

    /// The whole method, its first line not indented and every other line
    /// at `indent`, which is where the first one already starts.
    pub(crate) fn text(&self, indent: &str) -> String {
        format!(
            "{}\n{indent}{{\n{indent}{INDENT}{}\n{indent}}}",
            self.signature(),
            self.body
        )
    }
}

/// The methods `decl`, the class `class` of the entry document, may override
/// and does not declare, nearest first and each name once, with every type
/// written the way the entry document writes it at `at`.
///
/// A method with a body that a class above declares, and that is not
/// `private`, `final`, `static`, the constructor, or one with an `inout`
/// parameter, whose argument the call would have to choose a form for. A
/// method of `decl` whose name lies inside `writing` is not counted as
/// declared: it is the one a completion is about to replace.
pub(crate) fn overridable(
    analysed: &Analysed,
    class: &QName,
    decl: &ClassDecl,
    at: BytePos,
    writing: Option<(BytePos, BytePos)>,
) -> Vec<Override> {
    let entry = analysed.map.file(analysed.entry);
    let mut taken: FxHashSet<&str> = methods_of(&decl.members)
        .filter(|method| {
            writing.is_none_or(|(start, end)| method.name.start < start || method.name.end > end)
        })
        .map(|method| text_of(entry, method.name))
        .collect();
    let mut found = Vec::new();
    for (parent, stmt, file) in parents(analysed, class) {
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
            let (visibility, params, returns) = parts(analysed, file, method, at);
            found.push(Override {
                name: name.to_owned(),
                parent: parent.clone(),
                visibility,
                params,
                returns,
                body,
            });
        }
    }
    found
}

/// The class the entry document declares around `offset`, with the methods it
/// may override ([`overridable`]), for a completion that replaces `writing`.
pub(crate) fn overridable_at(
    analysed: &Analysed,
    offset: BytePos,
    writing: (BytePos, BytePos),
) -> Vec<Override> {
    let Some((class, decl)) = class_named(analysed, offset) else {
        return Vec::new();
    };
    overridable(analysed, &class, decl, writing.0, Some(writing))
}

/// Whether the entry document with `[start, end)` replaced by `text` analyses
/// with no error inside the text written.
///
/// An error elsewhere is the document's own, such as a half-written line
/// further down, and does not say anything about `text`. This is the one
/// check that does not count errors before and after
/// (`crate::refactor`): in a half-written document the parser recovers at a
/// different place once `text` is in it, and an error it already had moves.
pub(crate) fn writes_cleanly(
    analysed: &Analysed,
    documents: &Documents,
    (start, end): (BytePos, BytePos),
    text: &str,
) -> bool {
    let mut rewritten = analysed.map.file(analysed.entry).text().to_owned();
    if rewritten.get(start as usize..end as usize).is_none() {
        return false;
    }
    rewritten.replace_range(start as usize..end as usize, text);
    let Some(checked) = analyse_rewritten(documents, analysed, rewritten) else {
        return false;
    };
    let Ok(len) = BytePos::try_from(text.len()) else {
        return false;
    };
    let written = start..start + len;
    !checked.diags.iter().any(|diagnostic| {
        diagnostic.is_error()
            && diagnostic
                .primary_span()
                .is_some_and(|span| span.file == checked.entry && written.contains(&span.start))
    })
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
    let (visibility, params, returns) = parts(analysed, file, method, offset);
    let returns = returns.map(|ty| format!(": {ty}")).unwrap_or_default();
    format!(
        "{visibility} function {}({}){returns}",
        text_of(file, method.name),
        params.join(", "),
    )
}

/// [`signature`]'s three parts: the visibility, each parameter, and the
/// return type where one is declared.
fn parts(
    analysed: &Analysed,
    file: &SourceFile,
    method: &MethodMember,
    offset: BytePos,
) -> (&'static str, Vec<String>, Option<String>) {
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
    let returns = method.return_type.as_ref().map(|ty| typed(ty.span));
    (visibility, params, returns)
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

/// `Some` when the entry document with `edits` applied analyses with no error
/// it did not already have ([`crate::refactor::adds_no_error_in_entry`]).
fn compiles(analysed: &Analysed, documents: &Documents, edits: &[Change]) -> Option<()> {
    let edits: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|(span, text)| (span.range().start, span.range().end, text.as_str()))
        .collect();
    crate::refactor::adds_no_error_in_entry(documents, analysed, &edits).then_some(())
}
