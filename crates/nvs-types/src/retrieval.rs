//! [ADR 0046](../../../docs/adr/0046-attributes-shape-literal-metadata.md)
//! §§ 4-5: `Core\Attributes::get<T>` and `::all<T>`, answered here rather than
//! at run time.
//!
//! § 5 is what makes this a checker pass at all. A declaration's attached
//! attributes are fixed by its source, and [`crate::attributes`] has already
//! proved every payload value is a compile-time constant (§ 2), so the whole
//! question — *which attached literals structurally satisfy `T`* — has an
//! answer before the program starts. The call is therefore **replaced** with
//! that answer: a compiled-in `null` where nothing matches, the matched
//! literal itself where exactly one does, and a compile-time diagnostic
//! (`E0728`) where more than one does, naming `::all<T>` as the fix. There is
//! no runtime lookup, no reflection table in the compiled unit, and
//! `nvs_stdlib::attributes`' two symbols name a body that aborts if it is ever
//! reached.
//!
//! # Structural, not nominal
//!
//! § 4's rule is that retrieval matches on *shape*: an attached literal is an
//! answer to `get<T>` exactly when it satisfies `T` under
//! [`crate::expr::is_assignable`] — ADR 0036 § 3's width subtyping, the same
//! test a shape-typed binding goes through — regardless of whether it was
//! written bare or under a name, and regardless of what that name was. An
//! attribute's optional name exists to check the literal where it is
//! *written* ([`crate::attributes`]), and is never part of how a caller asks
//! for it. That is what keeps attributes from growing a second namespace of
//! kind names for unrelated frameworks to collide in.
//!
//! # Which declaration `$target` names
//!
//! § 4 fixes four spellings and this pass inspects them **syntactically**,
//! exactly as ADR 0033 § 4's sinks inspect a literal argument: the reference
//! is never evaluated, so `Foo::bar(...)` here is a written name rather than a
//! closure value. A method is its own first-class-callable reference; a class
//! is its `constructor`'s; a parameter is its method's reference plus
//! `$member`; a property is its class's `constructor` reference plus
//! `$member`.
//!
//! The last two overlap, and deliberately: `constructor` plus a `$member` is
//! both "the property named `$member`" and "the constructor parameter named
//! `$member`", which are the *same declaration* for an ADR 0043 § 4 promoted
//! parameter. So both rosters are consulted and their attributes joined, which
//! is the one arrangement that reads a promoted parameter's attribute once and
//! an unpromoted one's at all.
//!
//! A `$member` that is not a string *literal* names nothing this pass can
//! resolve. § 4's *Consequences* already fixes that case as an empty result
//! rather than a diagnostic, and so it is here: `get` folds to `null` and
//! `all` to the empty array.
//!
//! # What a matched payload has to be
//!
//! § 5 replaces the call with the payload, so every value in a *matched*
//! payload needs a constant form ([`ConstArg`]). Every shape § 2 admits has
//! one but two: a class constant, which `crate::signatures::ConstSig` folds
//! for a *read* but which this walk cannot ask for, since [`fold_value`] runs
//! over the written expression and carries none of the namespace context a
//! class name resolves through; and an enum case, which reaches a program
//! through `ExprInfo::EnumCase` rather than through a constant. Either in a
//! matched payload is `E0731` at the retrieval, naming the value; neither is
//! refused where it is *attached*, because § 2 admits it and an attribute
//! nobody retrieves costs nothing.

use nvs_diagnostics::{Diagnostic, SourceFile, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    ArrayItem, Attribute, AttributeGroup, CallArgs, ClassMember, ClassMemberKind, Expr, ExprKind,
    MemberName, NamespaceDecl, Stmt, StmtKind, UnaryOp,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::defaults::ConstArg;
use crate::expr::{check_object_literal, is_assignable, resolve_class_expr};
use crate::expr_table::ExprInfo;
use crate::locals::LocalScope;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// The class ADR 0046 § 4's two members live on, spelled as
/// [`nvs_hir::QName`] renders it.
const OWNER: &str = "Core\\Attributes";

/// One attached attribute, with the file it was written in — a payload's own
/// field names and string literals are read out of *that* source rather than
/// out of whichever file the retrieval was written in.
#[derive(Clone, Copy)]
struct Site<'a> {
    src: &'a SourceFile,
    attr: &'a Attribute,
}

/// Every attach site in the program, indexed by the declaration ADR 0046 § 4's
/// `$target` spellings name.
///
/// Built once, before any body is checked, for the reason
/// [`crate::consts::build_const_table`] is: a retrieval may be written above
/// the declaration it asks about, in the same file or in another one, so a
/// table filled as the walk descends would answer differently depending on
/// source order.
#[derive(Default)]
pub(crate) struct AttributeTable<'a> {
    classes: FxHashMap<QName, ClassAttrs<'a>>,
}

#[derive(Default)]
struct ClassAttrs<'a> {
    /// The declaration's own `#[...]` groups.
    own: Vec<Site<'a>>,
    /// Each method's own groups, by method name.
    methods: FxHashMap<String, Vec<Site<'a>>>,
    /// Each declared property's groups, by name with no `$` sigil.
    properties: FxHashMap<String, Vec<Site<'a>>>,
    /// Each parameter's groups, keyed by its method's name and its own, with
    /// no `$` sigil.
    params: FxHashMap<(String, String), Vec<Site<'a>>>,
}

/// Collects every attach site in the program. See [`AttributeTable`].
pub(crate) fn build_attribute_table<'a>(files: &[crate::ProgramFile<'a>]) -> AttributeTable<'a> {
    let mut table = AttributeTable::default();
    for file in files {
        collect(file.stmts, file.src, &[], &mut table);
    }
    table
}

fn collect<'a>(
    stmts: &'a [Stmt],
    src: &'a SourceFile,
    namespace: &[String],
    table: &mut AttributeTable<'a>,
) {
    let mut current_ns = namespace.to_vec();
    for stmt in stmts {
        let (name, attributes, members) = match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name.as_ref().map_or_else(Vec::new, |n| {
                    QName::parse(span_text(src, n.span)).segments().to_vec()
                });
                match body {
                    Some(block) => collect(&block.stmts, src, &new_ns, table),
                    None => current_ns = new_ns,
                }
                continue;
            }
            StmtKind::ClassDecl(decl) => (&decl.name, &decl.attributes, &decl.members),
            StmtKind::InterfaceDecl(decl) => (&decl.name, &decl.attributes, &decl.members),
            _ => continue,
        };
        let qname = QName::join(&current_ns, span_text(src, name.span));
        let entry = table.classes.entry(qname).or_default();
        push(&mut entry.own, attributes, src);
        collect_members(entry, members, src);
    }
}

fn collect_members<'a>(
    entry: &mut ClassAttrs<'a>,
    members: &'a [ClassMember],
    src: &'a SourceFile,
) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                let name = crate::strip_sigil(span_text(src, p.name)).to_owned();
                push(
                    entry.properties.entry(name).or_default(),
                    &p.attributes,
                    src,
                );
            }
            ClassMemberKind::Method(m) => {
                let method = span_text(src, m.name).to_owned();
                push(
                    entry.methods.entry(method.clone()).or_default(),
                    &m.attributes,
                    src,
                );
                for param in &m.params {
                    let name = crate::strip_sigil(span_text(src, param.name)).to_owned();
                    push(
                        entry.params.entry((method.clone(), name)).or_default(),
                        &param.attributes,
                        src,
                    );
                }
            }
            _ => {}
        }
    }
}

fn push<'a>(out: &mut Vec<Site<'a>>, groups: &'a [AttributeGroup], src: &'a SourceFile) {
    for group in groups {
        for attr in &group.attributes {
            out.push(Site { src, attr });
        }
    }
}

/// Whether `owner::member` is one of ADR 0046 § 4's two retrievals — the cheap
/// test [`crate::expr::calls`] makes before reaching for anything here.
pub(crate) fn is_retrieval(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && matches!(member, "get" | "all")
}

/// Resolves one retrieval and records its folded answer against the call's own
/// span, as the [`ExprInfo::CoreConst`] `nvs-ir` materializes any other
/// compile-time constant from.
///
/// Records nothing where the call is refused: a diagnostic has been reported,
/// and `nvs-ir` never reaches a unit that failed to check.
pub(crate) fn fold_retrieval(
    call: &Expr,
    member: &str,
    written: &[TypeId],
    args: &CallArgs,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    let Some(want) = written.first().copied() else {
        // The type-argument count is `check_written_type_args`' refusal and
        // has already been made; a second one names the same mistake twice.
        return;
    };
    if !matches!(env.interner.get(want), Ty::Shape(_)) {
        let found = env.interner.describe(want);
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_TYPE_ARG_NOT_A_SHAPE,
                format!("`Core\\Attributes::{member}` retrieves a shape, and `{found}` is not one"),
            )
            .with_primary(call.span, format!("`{found}` written here"))
            .with_help(
                "ADR 0046 § 4: retrieval is structural — an attached literal is an answer \
                 exactly when it satisfies `T` under ADR 0036 § 3's width subtyping, so `T` \
                 is an inline `{...}` or a `type` alias naming one",
            ),
        );
        return;
    }
    let Some(target) = list.first().map(|arg| &arg.value) else {
        // A missing argument is the arity check's refusal, already made.
        return;
    };
    let Some((class, method)) = target_declaration(target, ctx, env) else {
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_TARGET_NOT_A_DECLARATION,
                "a `Core\\Attributes` retrieval names no declaration",
            )
            .with_primary(target.span, "this is not a declaration reference")
            .with_help(
                "ADR 0046 § 4: the target is written as a first-class-callable reference and \
                 inspected where it is written — `Foo::bar(...)` for a method, and \
                 `Foo::constructor(...)` for the class itself, plus a literal member name for \
                 one of its properties or parameters",
            ),
        );
        return;
    };
    let member_name = list.get(1).and_then(|arg| match &arg.value.kind {
        ExprKind::Str(span) => Some(crate::string_lit::cook_string_literal(env.src, *span)),
        _ => None,
    });
    let sites = sites_for(env, &class, &method, member_name.as_deref());
    let matched = matching(&sites, want, ctx, env);
    let value = match (member, matched.len()) {
        ("get", 0) => ConstArg::Null,
        ("get", 1) => match fold_payload(matched[0], env) {
            Some(value) => value,
            None => return,
        },
        ("get", _) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS,
                    format!(
                        "`{class}::{method}` carries {} attached literals satisfying this shape",
                        matched.len()
                    ),
                )
                .with_primary(call.span, "`get` answers at most one")
                .with_help(
                    "ADR 0046 § 5: an attached-attribute list is static, so this is decided \
                     here rather than by a test run — write `Core\\Attributes::all<T>(…)`, \
                     which answers every match",
                ),
            );
            return;
        }
        _ => {
            let mut entries = Vec::with_capacity(matched.len());
            for (index, site) in matched.iter().enumerate() {
                let Some(value) = fold_payload(*site, env) else {
                    return;
                };
                entries.push((index.to_string(), value));
            }
            ConstArg::Array(entries)
        }
    };
    env.exprs.record(call.span, ExprInfo::CoreConst { value });
}

/// ADR 0046 § 4's `$target` spelling, read syntactically: the class it names
/// and the method whose reference was written.
fn target_declaration(target: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<(QName, String)> {
    let ExprKind::StaticCall {
        class,
        method: MemberName::Ident(name),
        args: CallArgs::FirstClassCallable,
        ..
    } = &target.kind
    else {
        return None;
    };
    let qname = resolve_class_expr(class, ctx, env)?;
    Some((qname, span_text(env.src, *name).to_owned()))
}

/// The attach sites `$target` plus `$member` names, joined where § 4's two
/// spellings overlap — see this module's own docs.
fn sites_for<'a>(
    env: &Env<'a>,
    class: &QName,
    method: &str,
    member: Option<&str>,
) -> Vec<Site<'a>> {
    let Some(entry) = env.attributes.classes.get(class) else {
        return Vec::new();
    };
    let Some(member) = member.filter(|name| !name.is_empty()) else {
        return if method == "constructor" {
            // A class with no written `constructor` still names itself this
            // way — ADR 0022 synthesizes one, so § 4 uses the spelling for
            // every class rather than only for the ones that declare it, and
            // a class that *does* write one may carry attributes on both.
            let mut sites = entry.own.clone();
            if let Some(found) = entry.methods.get(method) {
                sites.extend(found.iter().copied());
            }
            sites
        } else {
            entry.methods.get(method).cloned().unwrap_or_default()
        };
    };
    let mut sites = Vec::new();
    if method == "constructor"
        && let Some(found) = entry.properties.get(member)
    {
        sites.extend(found.iter().copied());
    }
    if let Some(found) = entry.params.get(&(method.to_owned(), member.to_owned())) {
        sites.extend(found.iter().copied());
    }
    sites
}

/// The sites whose payload satisfies `want`, in attach order.
///
/// Each payload is inferred under **its own** file's source, because a field
/// name and a string literal are read out of the text they were written in.
fn matching<'a>(
    sites: &[Site<'a>],
    want: TypeId,
    ctx: &Ctx<'_>,
    env: &mut Env<'a>,
) -> Vec<Site<'a>> {
    let outer = env.src;
    let mut matched = Vec::new();
    for site in sites {
        env.src = site.src;
        let mut live = FxHashSet::default();
        let scope = LocalScope::new();
        let actual = check_object_literal(&site.attr.fields, &mut live, &scope, ctx, env);
        if is_assignable(actual, want, env.interner, env.graph, env.signatures) {
            matched.push(*site);
        }
    }
    env.src = outer;
    matched
}

/// One matched payload as the constant § 5 replaces the call with, or `None`
/// after reporting `E0731` for a value with no constant form.
fn fold_payload<'a>(site: Site<'a>, env: &mut Env<'a>) -> Option<ConstArg> {
    let outer = env.src;
    env.src = site.src;
    let mut fields = Vec::with_capacity(site.attr.fields.len());
    let mut ok = true;
    for field in &site.attr.fields {
        let name = span_text(env.src, field.name).to_owned();
        match fold_value(&field.value, env) {
            Some(value) => fields.push((name, value)),
            None => {
                report_unfoldable(&field.value, env);
                ok = false;
            }
        }
    }
    env.src = outer;
    ok.then_some(ConstArg::Shape(fields))
}

/// One payload value as a [`ConstArg`], or `None` for one with no constant
/// form. Reports nothing — [`fold_payload`] names the position.
fn fold_value(expr: &Expr, env: &mut Env<'_>) -> Option<ConstArg> {
    let mut negated = false;
    let mut inner = expr;
    loop {
        match &inner.kind {
            ExprKind::Paren(next) => inner = next,
            ExprKind::Unary {
                op: UnaryOp::Neg,
                expr: next,
            } => {
                negated = !negated;
                inner = next;
            }
            ExprKind::Unary {
                op: UnaryOp::Plus,
                expr: next,
            } => inner = next,
            _ => break,
        }
    }
    match &inner.kind {
        ExprKind::Null if !negated => Some(ConstArg::Null),
        ExprKind::Bool(b) if !negated => Some(ConstArg::Bool(*b)),
        ExprKind::Str(span) if !negated => Some(ConstArg::Str(
            crate::string_lit::cook_string_literal(env.src, *span),
        )),
        ExprKind::Float(span) => crate::defaults::float_value(*span, env.src)
            .map(|f| ConstArg::Float(if negated { -f } else { f })),
        // ADR 0007 § 2's "untyped until placed" has no target here to place
        // it against, so the value's own magnitude decides: an `int` where one
        // holds it, a `uint` above that, which is the same order a written
        // annotation would have narrowed it in.
        ExprKind::Int(span) => {
            let magnitude = crate::defaults::int_magnitude(*span, env)?;
            if negated {
                i64::try_from(magnitude)
                    .ok()
                    .map(|v| ConstArg::Int(-v))
                    .or_else(|| (magnitude == 1 << 63).then_some(ConstArg::Int(i64::MIN)))
            } else {
                Some(i64::try_from(magnitude).map_or(ConstArg::Uint(magnitude), ConstArg::Int))
            }
        }
        ExprKind::ArrayLiteral(items) if !negated => fold_array(items, env),
        ExprKind::ObjectLiteral(fields) if !negated => {
            let mut out = Vec::with_capacity(fields.len());
            for field in fields {
                out.push((
                    span_text(env.src, field.name).to_owned(),
                    fold_value(&field.value, env)?,
                ));
            }
            Some(ConstArg::Shape(out))
        }
        _ => None,
    }
}

/// An array literal's entries under the `string` keys ADR 0007 § 5 gives them,
/// with a keyless run taking its position in that run — every value here is
/// constant, so the auto-index has one answer and this is the last place it is
/// cheap to compute.
fn fold_array(items: &[ArrayItem], env: &mut Env<'_>) -> Option<ConstArg> {
    let mut out = Vec::with_capacity(items.len());
    let mut next = 0_u64;
    for ArrayItem {
        key, value, spread, ..
    } in items
    {
        if *spread {
            return None;
        }
        let key = match key {
            Some(key) => match fold_value(key, env)? {
                ConstArg::Str(s) => s,
                ConstArg::Int(n) => n.to_string(),
                ConstArg::Uint(n) => n.to_string(),
                _ => return None,
            },
            None => {
                let key = next.to_string();
                next += 1;
                key
            }
        };
        out.push((key, fold_value(value, env)?));
    }
    Some(ConstArg::Array(out))
}

/// The `E0731` half of [`fold_payload`], which owns why.
fn report_unfoldable(value: &Expr, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_PAYLOAD_UNFOLDABLE,
            "this attribute satisfies the retrieved shape, but holds a value with no \
             compile-time form",
        )
        .with_primary(value.span, "no constant form")
        .with_help(
            "ADR 0046 § 5 replaces the retrieval with the payload itself, so every value in \
             it has to be materializable — a user-declared class constant's value and an \
             enum case are the two ADR 0046 § 2 admits and this compiler cannot yet inline",
        ),
    );
}
