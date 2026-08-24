//! Seeding the checker's signature table with `Core`.
//!
//! `mwl_stdlib::registry` is the one home for what a `Core` member's signature
//! *is*; this is the one place that turns it into the same
//! [`ClassSignature`](crate::signatures::ClassSignature) a user-declared class
//! produces. Everything after that point is unchanged machinery:
//! `resolve_method` finds `Core\Arr::count` the way it finds `Animal::name`,
//! the arity and assignability checks are the same ones, and the recorded
//! `ResolvedCall` has the same shape — which is the whole reason to seed a
//! table rather than special-case `Core` at each call site.
//!
//! Two properties fall out of that and are worth naming, because a
//! special-cased design would have had to state them:
//!
//! * A `Core` member is `static` and never `private` — [`seed`] sets both, so
//!   `Core\Arr::count($a)` is the only reachable spelling and
//!   `$a->count()` resolves to nothing, as ADR 0063 R20 ("no operation is
//!   reachable two ways") requires.
//! * A name `mwl_stdlib` does not register does not exist. Before this
//!   existed, `mwl_hir::QName::is_core` made every `Core\…` reference trusted
//!   and unchecked; a registered class is now checked like any other, while
//!   an unregistered one stays trusted so the rest of the spec's §§ 1–12 can
//!   still be *written* in a fixture before it is implemented. That trust is
//!   the thing to remove once the registry is complete.

use mwl_hir::QName;
use mwl_stdlib::registry::{CLASSES, Const, CoreTy};
use rustc_hash::FxHashMap;

use crate::defaults::ConstArg;
use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{TypeId, TypeInterner};

/// Adds every `mwl_stdlib::registry::CLASSES` entry to `table`.
///
/// Called once, at the head of
/// [`build_signatures`](crate::signatures::build_signatures), so a `Core`
/// signature is in place before the first user declaration is collected and
/// long before any body is checked.
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    for class in CLASSES {
        let qname = QName::parse(class.name);
        let mut methods = FxHashMap::default();
        for method in class.methods {
            methods.insert(
                method.name.to_owned(),
                MethodSig {
                    params: method
                        .params
                        .iter()
                        .map(|param| lower(param, interner))
                        .collect(),
                    // ADR 0063 R7: nothing in `Core` mutates its subject, so
                    // no `Core` parameter is ever by-reference. Not a gap in
                    // the registry — a property of the convention.
                    by_ref: vec![false; method.params.len()],
                    variadic: false,
                    defaults: defaults_of(method),
                    return_ty: lower(&method.return_ty, interner),
                    is_static: true,
                    interface_private: false,
                    // Native Rust behind a helper symbol, not a compiled MWL
                    // function — but it is code, so a call never needs to go
                    // looking for an override.
                    has_body: true,
                },
            );
        }
        table.seed_class(qname, FxHashMap::default(), methods);
    }
}

/// The symbol a resolved `Core` call is reachable at, or `None` if `qname`
/// names no registered class or `method` no registered member.
///
/// `mwl-ir` reads this to lower a resolved static call whose target is a
/// `Core` member into the helper-shaped instruction that reaches it — the one
/// piece of a `Core` call that is genuinely not the same as a user-declared
/// one, since there is no compiled MWL function to name.
#[must_use]
pub fn symbol_of(qname: &QName, method: &str) -> Option<&'static str> {
    let class = mwl_stdlib::registry::class(&qname.to_string())?;
    class
        .methods
        .iter()
        .find(|candidate| candidate.name == method)
        .map(|found| found.symbol)
}

/// A `Core` class constant's declared type and its value, or `None` if
/// `qname` names no registered class or `name` no constant on it.
///
/// The counterpart of [`symbol_of`] for the one member kind that is not a
/// call: a constant is [ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)'s
/// "every constant is a class constant", and ADR 0010 § 3's inlining rule for
/// an enum case is the one it follows — so what a consumer gets back is the
/// *value*, materialized at the use site, with no storage anywhere.
///
/// Not seeded into [`SignatureTable`] the way a method is, because there is
/// nothing there to seed it into: the table holds properties and methods, and
/// a user-declared class constant's own type is unmodeled either way (see
/// [`crate::expr`]'s known gaps). This is read directly by that module's
/// `ClassConstAccess` arm, which is the one place a constant is resolved at
/// all.
#[must_use]
pub(crate) fn constant(
    qname: &QName,
    name: &str,
    interner: &mut TypeInterner,
) -> Option<(TypeId, ConstArg)> {
    let found = mwl_stdlib::registry::class(&qname.to_string())?.constant(name)?;
    Some((lower(&found.ty, interner), lower_const(&found.value)))
}

/// Whether `qname` names a class this crate seeded — the question
/// [`crate::expr`] asks before reporting an unknown *constant*, so an
/// unregistered `Core` class stays trusted exactly as [`seed`]'s own docs
/// describe.
#[must_use]
pub(crate) fn is_registered(qname: &QName) -> bool {
    mwl_stdlib::registry::class(&qname.to_string()).is_some()
}

/// A registry row's end-aligned `CoreMethod::defaults` as the per-parameter
/// [`MethodSig::defaults`] the checker and `mwl-ir` read — a run of `None` for
/// the required parameters, then one entry per declared default.
///
/// The two spellings differ on purpose: the registry states the shorter one
/// because a `Core` row is written by hand, and the signature table states the
/// positional one because every consumer indexes it by parameter. This is the
/// one place that has to agree with `CoreMethod::defaults`' own alignment
/// rule, which is why the slice is taken from the end rather than the start.
///
/// A trailing options bag gets its entry **synthesized** here from the bag's
/// own [`CoreOption::default`](mwl_stdlib::registry::CoreOption::default)s
/// rather than read from `defaults`, which is what makes the bag optional
/// without a registry row ever saying so twice — see
/// `mwl_stdlib::registry::CoreTy::Options`.
fn defaults_of(method: &mwl_stdlib::registry::CoreMethod) -> Vec<Option<ConstArg>> {
    let positional = method.positional().len();
    let required = positional - method.defaults.len();
    (0..method.params.len())
        .map(|index| {
            if index == positional {
                let options = method
                    .options()
                    .expect("an index past the positional parameters is the options bag");
                return Some(ConstArg::Options(
                    options
                        .iter()
                        .map(|option| (option.name.to_owned(), lower_const(&option.default)))
                        .collect(),
                ));
            }
            index
                .checked_sub(required)
                .map(|offset| lower_const(&method.defaults[offset]))
        })
        .collect()
}

/// One registry default into the checker's own [`ConstArg`] — the same
/// translation [`lower`] performs for a type, and for the same reason.
fn lower_const(value: &Const) -> ConstArg {
    match *value {
        Const::Null => ConstArg::Null,
        Const::Bool(b) => ConstArg::Bool(b),
        Const::Int(v) => ConstArg::Int(v),
        Const::Uint(v) => ConstArg::Uint(v),
        Const::Float(v) => ConstArg::Float(v),
        Const::Str(s) => ConstArg::Str(s.to_owned()),
        // ADR 0010 § 3: the case *is* its integer constant, so what a call
        // site materializes is that constant — the same value the enum table
        // hands `mwl-ir` for a written `Core\Order::Asc`. Resolved here rather
        // than written into the row so the two cannot disagree.
        Const::EnumCase(name, case) => ConstArg::Int(
            mwl_stdlib::registry::core_enum(name)
                .and_then(|found| {
                    found
                        .cases
                        .iter()
                        .find(|(candidate, _)| *candidate == case)
                })
                .map(|(_, value)| *value)
                .unwrap_or_else(|| {
                    panic!("mwl-stdlib defaults an option to `{name}::{case}`, which it does not register")
                }),
        ),
        // `Const` is `#[non_exhaustive]`: a variant this arm has not learned
        // yet has no safe `ConstArg` to become, so it fails loudly here rather
        // than silently defaulting a parameter to the wrong value. Both tables
        // are in this workspace, so reaching it is a build-time oversight.
        ref other => panic!("mwl-types has no ConstArg for the registry default {other:?}"),
    }
}

/// One registry type into an interned one. The registry's enum is
/// deliberately smaller than [`crate::ty::Ty`] — see its own docs — so this
/// is a total match with no failure case.
fn lower(ty: &CoreTy, interner: &mut TypeInterner) -> TypeId {
    match ty {
        CoreTy::Bool => interner.bool_ty(),
        CoreTy::Int => interner.int(),
        CoreTy::Uint => interner.uint(),
        CoreTy::Float => interner.float(),
        CoreTy::Decimal => interner.decimal(),
        CoreTy::Str => interner.string(),
        CoreTy::Bytes => interner.bytes(),
        CoreTy::Void => interner.void(),
        CoreTy::Array(elem) => {
            let elem = lower(elem, interner);
            interner.array(elem)
        }
        CoreTy::Var(name) => interner.type_var(*name),
        CoreTy::Callable => interner.callable(),
        // A `Core`-owned enum is interned exactly as a declared one is —
        // `crate::enums` has already seeded the same name into its own table,
        // so the backing type asked for here is the one every other consumer
        // reads back.
        CoreTy::Enum(name) => {
            let qname = QName::parse(name);
            interner.enum_(qname, crate::enums::EnumBacking::Int)
        }
        CoreTy::CallableTo(name) => interner.callable_to(*name),
        // Canonicalized by the interner, unlike an options bag: a union has no
        // ABI order to preserve, so `int|string` and `string|int` are one type
        // here exactly as they are when written in source.
        CoreTy::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|member| lower(member, interner))
                .collect();
            interner.make_union(members)
        }
        // `?T` is `null|T` and nothing else — the checker has no separate
        // nullable type, so a registry row's `?T` and a source-written `?T`
        // are the *same* interned id, and everything downstream (assignability,
        // `??`, `mwl_ir::lower_checked_ty`'s `Ty::Tagged`) meets one shape.
        CoreTy::Nullable(inner) => {
            let inner = lower(inner, interner);
            let null = interner.null();
            interner.make_union([null, inner])
        }
        // The registry's order is kept, not sorted: it is the order the bag
        // flattens into ABI arguments. `Ty::Options` owns why.
        CoreTy::Options(options) => {
            let options = options
                .iter()
                .map(|option| (option.name.to_owned(), lower(&option.ty, interner)))
                .collect();
            interner.options(options)
        }
        // `Mixed` and anything a later registry variant adds: `mixed` is the
        // registry's own "unchecked position" spelling, and is the only safe
        // answer for a variant this arm has not learned yet, since `Ty` and
        // `CoreTy` are both `#[non_exhaustive]`.
        _ => interner.mixed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::resolve_method;
    use mwl_hir::ClassGraph;

    #[test]
    fn a_registered_member_resolves_with_its_declared_shape() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (owner, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "count",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Arr::count is registered");
        assert_eq!(owner.to_string(), r"Core\Arr");
        assert!(sig.is_static);
        assert!(!sig.variadic);
        assert_eq!(sig.params.len(), 1);
        assert_eq!(interner.describe(sig.params[0]), "array<T>");
        assert_eq!(interner.describe(sig.return_ty), "uint");
    }

    /// The bag's two halves, both derived from one registry row: the
    /// parameter's type carries the option names and types, and its
    /// [`MethodSig::defaults`] entry carries each option's default. Nothing in
    /// `mwl_stdlib::registry` states either twice, so this is the one place
    /// they could disagree.
    #[test]
    fn an_options_bag_lowers_to_one_parameter_and_one_synthesized_default() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "range",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Arr::range is registered");
        assert_eq!(sig.params.len(), 3);
        assert_eq!(interner.describe(sig.params[2]), "{step?: int}");
        // The bag is optional by construction: two arguments are required, so
        // `Core\Arr::range(1, 10)` is a legal call with no registry row saying
        // the bag has a default.
        assert_eq!(sig.required(), 2);
        assert_eq!(
            sig.defaults[2],
            Some(ConstArg::Options(vec![(
                "step".to_owned(),
                ConstArg::Int(1)
            )]))
        );
    }

    /// `CoreTy::CallableTo` lowers to a parameter that *describes* as
    /// `callable` — the variable name it carries is a registry fact, and
    /// [`crate::ty::Ty::CallableTo`] owns why no diagnostic ever quotes it.
    #[test]
    fn a_callback_result_parameter_lowers_to_something_that_reads_as_callable() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "map",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Arr::map is registered");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(interner.describe(sig.params[0]), "array<T>");
        assert_eq!(interner.describe(sig.params[1]), "callable");
        assert_eq!(interner.describe(sig.return_ty), "array<U>");
        // Distinct from a plain `callable` all the same, or it would have
        // nowhere to carry the name it binds.
        let plain = interner.callable();
        assert_ne!(sig.params[1], plain);
    }

    /// `CoreTy::Nullable` is `null|T` and *is* the union the checker already
    /// had — the one property the whole `?T` half of ADR 0066 rests on, since
    /// a registry row's `?string` and a source-written `?string` have to be
    /// one interned id for `??` and assignability to meet a single shape.
    #[test]
    fn a_nullable_return_is_the_same_union_a_source_written_one_interns_to() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "firstKey",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Arr::firstKey is registered");
        let null = interner.null();
        let string = interner.string();
        let expected = interner.make_union([null, string]);
        assert_eq!(sig.return_ty, expected);
        assert_eq!(interner.describe(sig.return_ty), "string|null");
    }

    /// A nullable *element* type substitutes like any other: `first`'s `?T`
    /// carries the variable, so a call over `array<int>` answers `null|int`
    /// rather than the `mixed` an unbound variable would give.
    #[test]
    fn a_nullable_return_carrying_a_variable_still_substitutes() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "first",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Arr::first is registered");
        assert_eq!(interner.describe(sig.return_ty), "T|null");
    }

    #[test]
    fn an_unregistered_member_resolves_to_nothing() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        assert!(
            resolve_method(
                &QName::parse(r"Core\Arr"),
                "nope",
                &table,
                &ClassGraph::default()
            )
            .is_none()
        );
    }

    #[test]
    fn a_registered_member_names_the_symbol_its_implementation_lives_at() {
        assert_eq!(
            symbol_of(&QName::parse(r"Core\Arr"), "count"),
            Some("mwl_core_arr_count")
        );
        assert_eq!(symbol_of(&QName::parse(r"Core\Arr"), "nope"), None);
        assert_eq!(symbol_of(&QName::parse("Animal"), "count"), None);
    }
}
