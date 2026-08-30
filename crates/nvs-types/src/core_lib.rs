//! Seeding the checker's signature table with `Core`.
//!
//! `nvs_stdlib::registry` is the one home for what a `Core` member's signature
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
//! * A member `nvs_stdlib` does not register does not exist. Before this
//!   existed, `nvs_hir::QName::is_core` made every `Core\…` reference trusted
//!   and unchecked; every class is now checked like any other, and the
//!   registry being the whole roster of `Core` is what makes an *unregistered*
//!   class's member knowably wrong too — `Core\Env::EOL` is `E0405` where it
//!   is written rather than a panic in `nvs-ir`, which is the crate that would
//!   otherwise have to lower a call with no target. What stays trusted is the
//!   bare *name*: `nvs_hir` still resolves any `Core\…` without a declaration,
//!   so a not-yet-implemented class may be named in a type position while no
//!   member of it can be reached.

use nvs_hir::QName;
use nvs_hir::interfaces::{COMPARABLE, ITERABLE, ITERATOR};
use nvs_stdlib::registry::{CLASSES, Const, CoreTy, OPTIONS_NAME, Qual};
use rustc_hash::FxHashMap;

use crate::defaults::ConstArg;
use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{TypeId, TypeInterner};

/// Adds every `nvs_stdlib::registry::CLASSES` entry to `table`.
///
/// Called once, at the head of
/// [`build_signatures`](crate::signatures::build_signatures), so a `Core`
/// signature is in place before the first user declaration is collected and
/// long before any body is checked.
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    for class in CLASSES {
        let qname = QName::parse(class.name);
        let mut methods = FxHashMap::default();
        for (method, is_static) in class
            .methods
            .iter()
            .map(|method| (method, true))
            .chain(class.instance.iter().map(|method| (method, false)))
        {
            methods.insert(
                method.name.to_owned(),
                method_sig(method, is_static, interner),
            );
        }
        // A `Core` class is constructible only where
        // `nvs_stdlib::registry::CONSTRUCTORS` says so, and that roster's row
        // is a `CoreMethod` like any other — so `new Core\Heap<T>($by)` is
        // arity- and type-checked by exactly the machinery every other `Core`
        // call goes through, rather than by a second rule reachable only from
        // `new`. It is an *instance* member, because the receiver a compiled
        // call would pass is the instance being built; nothing resolves it as
        // a static one.
        if let Some(new) = nvs_stdlib::registry::constructor_of(class.name) {
            methods.insert(new.name.to_owned(), method_sig(new, false, interner));
        }
        // No properties, for either kind: a `Core` instance's slots are
        // `nvs-stdlib`'s layout rather than a surface a program reads, so
        // `$match->groups` is an unknown member and `$match->groups()` is the
        // member. `nvs_stdlib::registry::CoreTy::Instance` owns why.
        table.seed_class(qname.clone(), FxHashMap::default(), methods);
        // ADR 0013's `Comparable`, which a `Core` class satisfies by carrying
        // the member rather than by naming the interface —
        // `nvs_stdlib::registry::implements_comparable` owns why, and seeding
        // it here is what lets `$a < $b` reach the same
        // `crate::expr::operators::object_comparison_result` a user class's
        // `implements Comparable` reaches. No arguments: the interface takes
        // none (`nvs_hir::interfaces::RESERVED`).
        if nvs_stdlib::registry::implements_comparable(class.name) {
            table.seed_implements(qname.clone(), QName::parse(COMPARABLE), Vec::new());
        }
        // The one thing a `Core` class says about a hierarchy, and it says it
        // to `foreach`: `nvs_stdlib::registry::ITERABLES` is the roster, and a
        // row's element may be one of the class's own type variables, which
        // `crate::expr::iteration` substitutes the receiver's arguments into.
        if let Some(elem) = nvs_stdlib::registry::iterable_element(class.name) {
            let elem = lower(elem, interner);
            table.seed_implements(qname, QName::parse(ITERABLE), vec![elem]);
        }
    }
}

/// One registry row as the checker's own signature.
fn method_sig(
    method: &nvs_stdlib::registry::CoreMethod,
    is_static: bool,
    interner: &mut TypeInterner,
) -> MethodSig {
    MethodSig {
        params: method
            .params
            .iter()
            .map(|param| lower(param, interner))
            .collect(),
        // No `Core` member returns `static`: the registry has no
        // spelling for it, and a `Core` class is never extended —
        // see `MethodSig::returns_static`.
        returns_static: false,
        // ADR 0063 R2: every `Core` parameter is callable by the spec's
        // `$name`. The row carries one name per positional slot and none
        // for a trailing bag, so `param_names` re-aligns them to `params`
        // — see its own docs for why the bag's entry is made there.
        param_names: param_names(method),
        // ADR 0088 § 2's classification, carried rather than dropped.
        // `lower` interns a `Text`/`Blob` parameter at its plain type
        // on purpose — what a member *does* with a qualifier is not
        // what the argument *is* — so this is the one place the row's
        // judgement reaches the checker. See `MethodSig::param_quals`.
        param_quals: method.params.iter().map(qual_of).collect(),
        // ADR 0063 R7: nothing in `Core` mutates its subject, so
        // no `Core` parameter is ever by-reference. Not a gap in
        // the registry — a property of the convention.
        inout: vec![false; method.params.len()],
        // The last parameter's own shape says it — a
        // `CoreTy::Variadic` there and nowhere else, which that
        // variant's docs hold. `MethodSig::params` keeps the
        // *element* type in that slot, which is what
        // `MethodSig::param_at` hands every argument from that
        // position onward.
        variadic: method.variadic().is_some(),
        defaults: defaults_of(method),
        // The registry's own first-appearance order, never
        // recomputed here — see `CoreMethod::written`.
        type_params: method.written().into_iter().map(str::to_owned).collect(),
        return_ty: lower(&method.return_ty, interner),
        // A `Core` member is reachable exactly one way (ADR 0063
        // R20): a static one through its class name, an instance
        // one through a value. The registry states which by which
        // roster the row is written in — see
        // `nvs_stdlib::registry::CoreClass::instance`.
        is_static,
        interface_private: false,
        // Every registered row is part of `Core`'s surface — the
        // registry has no way to write an internal one, so there
        // is nothing here for ADR 0094's levels to say.
        visibility: nvs_syntax::ast::Visibility::Public,
        // Native Rust behind a helper symbol, not a compiled Novis
        // function — but it is code, so a call never needs to go
        // looking for an override.
        has_body: true,
    }
}

/// The symbol a resolved `Core` call is reachable at, or `None` if `qname`
/// names no registered class or `method` no registered member.
///
/// `nvs-ir` reads this to lower a resolved static call whose target is a
/// `Core` member into the helper-shaped instruction that reaches it — the one
/// piece of a `Core` call that is genuinely not the same as a user-declared
/// one, since there is no compiled Novis function to name.
#[must_use]
pub fn symbol_of(qname: &QName, method: &str) -> Option<&'static str> {
    let class = nvs_stdlib::registry::class(&qname.to_string())?;
    class
        .members()
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
/// Not seeded into [`SignatureTable`] the way a method is, even though a
/// user-declared constant now takes a [`crate::signatures::ConstSig`] row
/// there: seeding would mean interning every `Core` class's constants whether
/// or not a program names one, where this resolves the single row a written
/// `Core\Math::PI` asks for. This is read directly by `crate::expr::members`'
/// `ClassConstAccess` arm, which is the one place a constant is resolved at
/// all.
#[must_use]
pub(crate) fn constant(
    qname: &QName,
    name: &str,
    interner: &mut TypeInterner,
) -> Option<(TypeId, ConstArg)> {
    let found = nvs_stdlib::registry::class(&qname.to_string())?.constant(name)?;
    Some((lower(&found.ty, interner), lower_const(&found.value)))
}

/// Whether `qname` names a class this crate seeded — the question
/// [`crate::expr`] asks about a `new` target and about the implicit
/// constructor's arity, both of which are answered by the registry's own row
/// rather than by the class's members. A *member* reference asks nothing here:
/// the registry is the whole roster of `Core`, so a miss is a miss whether or
/// not the class is registered ([`seed`]'s own docs).
#[must_use]
pub(crate) fn is_registered(qname: &QName) -> bool {
    nvs_stdlib::registry::class(&qname.to_string()).is_some()
}

/// A registry row's end-aligned `CoreMethod::defaults` as the per-parameter
/// [`MethodSig::defaults`] the checker and `nvs-ir` read — a run of `None` for
/// the required parameters, then one entry per declared default.
///
/// The two spellings differ on purpose: the registry states the shorter one
/// because a `Core` row is written by hand, and the signature table states the
/// positional one because every consumer indexes it by parameter. This is the
/// one place that has to agree with `CoreMethod::defaults`' own alignment
/// rule, which is why the slice is taken from the end rather than the start.
///
/// A trailing options bag gets its entry **synthesized** here from the bag's
/// own [`CoreOption::default`](nvs_stdlib::registry::CoreOption::default)s
/// rather than read from `defaults`, which is what makes the bag optional
/// without a registry row ever saying so twice — see
/// `nvs_stdlib::registry::CoreTy::Options`.
fn defaults_of(method: &nvs_stdlib::registry::CoreMethod) -> Vec<Option<ConstArg>> {
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

/// A registry row's `CoreMethod::names` as the per-parameter
/// [`MethodSig::param_names`] every consumer indexes — one entry per
/// [`MethodSig::params`] entry, so an index into either names the same slot.
///
/// The two spellings differ for one reason, and it is the shape
/// [`defaults_of`] already has: `names` is aligned to
/// `CoreMethod::positional`, because a trailing options bag has no per-row
/// name to record — its one name is [`OPTIONS_NAME`] for every member that
/// has one, which is a property of *being* a bag rather than a per-row
/// choice. This is the one place that entry is materialized.
///
/// A variadic tail keeps its entry, so the alignment holds for every row.
/// ADR 0063 R2 says a name never reaches one, and
/// [`MethodSig::param_index`](crate::signatures::MethodSig::param_index) is
/// where that is enforced — for a `Core` row on exactly the terms a
/// user-declared method gets.
fn param_names(method: &nvs_stdlib::registry::CoreMethod) -> Vec<String> {
    // `every_registry_row_names_one_parameter_per_positional_slot` holds this
    // over the whole registry; asserted again here because a name resolves to
    // a slot by *index*, so two lists of different lengths would bind an
    // argument to the wrong parameter rather than refuse it.
    debug_assert_eq!(
        method.names.len(),
        method.positional().len(),
        "`{}` names {} of its {} positional parameter(s)",
        method.name,
        method.names.len(),
        method.positional().len(),
    );
    let mut names: Vec<String> = method.names.iter().map(|name| (*name).to_owned()).collect();
    if method.options().is_some() {
        names.push(OPTIONS_NAME.to_owned());
    }
    names
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
        Const::Bytes(b) => ConstArg::Bytes(b.to_vec()),
        Const::EmptyArray => ConstArg::EmptyArray,
        // ADR 0010 § 3: the case *is* its integer constant, so what a call
        // site materializes is that constant — the same value the enum table
        // hands `nvs-ir` for a written `Core\Order::Asc`. Resolved here rather
        // than written into the row so the two cannot disagree.
        Const::EnumCase(name, case) => ConstArg::Int(
            nvs_stdlib::registry::core_enum(name)
                .and_then(|found| {
                    found
                        .cases
                        .iter()
                        .find(|(candidate, _)| *candidate == case)
                })
                .map(|(_, value)| *value)
                .unwrap_or_else(|| {
                    panic!("nvs-stdlib defaults an option to `{name}::{case}`, which it does not register")
                }),
        ),
        // An instance has no constant form, so what is carried across is the
        // call that builds one — `nvs_stdlib::registry::Const::Built` owns why
        // that is still an inlined constant.
        Const::Built { symbol, args } => ConstArg::Built {
            symbol,
            args: args.iter().map(lower_const).collect(),
        },
        // `Const` is `#[non_exhaustive]`: a variant this arm has not learned
        // yet has no safe `ConstArg` to become, so it fails loudly here rather
        // than silently defaulting a parameter to the wrong value. Both tables
        // are in this workspace, so reaching it is a build-time oversight.
        ref other => panic!("nvs-types has no ConstArg for the registry default {other:?}"),
    }
}

/// The classification a registry parameter carries, or `None` for a parameter
/// whose type has no cell to write one in.
///
/// A variadic tail answers its element's, for [`lower`]'s reason: the tail is
/// an arity rule rather than a type of its own, so every argument from that
/// position onward is checked against — and classified by — the element.
fn qual_of(ty: &CoreTy) -> Option<Qual> {
    match ty {
        CoreTy::Text(qual) | CoreTy::Blob(qual) => Some(*qual),
        CoreTy::Variadic(elem) => qual_of(elem),
        // Every other spelling has no classification of its own, including an
        // `array<text>` element and an options bag's members: no registry row
        // writes one nested, and `MethodSig::param_quals` owns that limit.
        _ => None,
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
        // A classification describes what the member does with the argument,
        // never what the argument is: `nvs_stdlib::registry::Qual` and the
        // interner's own `tainted`/`secret` are different questions, and these
        // two lower to exactly what their unclassified spellings lower to.
        //
        // The classification is not dropped: `method_sig` puts the row's
        // `Qual` on `MethodSig::param_quals`, and `qual_of` next to it is what
        // reads it out. This arm interns the *type*, and the two are separate
        // questions on purpose.
        //
        // **Known gap: nothing reads it back at the call yet.**
        // `crate::expr::calls` does not consult `param_quals`, so a parameter
        // marked `Contagious` or `Neutral` still refuses a qualified argument
        // exactly as an unclassified one does — `Core\Bytes::length($tainted)`
        // is `E0401: expected bytes, found tainted bytes`. That is ADR 0088
        // § 2's *default* applied to rows whose author wrote something else:
        // § 2 gives `Contagious` a qualified result and `Neutral` a plain one,
        // and both are meant to *accept*. Only `Sink`'s refusal is what the
        // tree actually does, and it is right for the wrong reason. Being
        // over-strict is safe — a tainted value cannot launder through a
        // member it cannot reach — so the cost is that `tainted` is unusable
        // with `Core` rather than that it leaks. What is left is the call
        // check admitting a qualified argument on the two accepting marks, and
        // a `Contagious` call's *result* gaining the union of its arguments'
        // qualifiers. `Neutral` dropping `secret` is a laundering decision, so
        // that half is ADR 0088's to answer before it is written.
        CoreTy::Str | CoreTy::Text(_) => interner.string(),
        CoreTy::Bytes | CoreTy::Blob(_) => interner.bytes(),
        CoreTy::Void => interner.void(),
        CoreTy::Array(elem) => {
            let elem = lower(elem, interner);
            interner.array(elem)
        }
        // A variadic tail is not a type of its own: `MethodSig` records the
        // arity rule in its `variadic` flag, and the parameter slot holds the
        // type *each* trailing argument is checked against. So this unwraps
        // rather than interning anything.
        CoreTy::Variadic(elem) => lower(elem, interner),
        // Both variable kinds intern as the same `Ty::TypeVar`: they differ
        // only in where the binding comes from, and `MethodSig::type_params`
        // is where that difference is recorded.
        CoreTy::Var(name) | CoreTy::Written(name) => interner.type_var(*name),
        CoreTy::Callable => interner.callable(),
        // A `Core`-owned enum is interned exactly as a declared one is —
        // `crate::enums` has already seeded the same name into its own table,
        // so the backing type asked for here is the one every other consumer
        // reads back.
        CoreTy::Enum(name) => {
            let qname = QName::parse(name);
            interner.enum_(qname, crate::enums::EnumBacking::Int)
        }
        // ADR 0047 § 3's narrowed case type, interned exactly as a source-
        // written `Core\Digest::Sha256` in type position is — `crate::lower`
        // reaches the same `enum_case` for that spelling, so the union a
        // registry row builds out of these and one a program could write are
        // the same interned id.
        CoreTy::EnumCase(name, case) => {
            let qname = QName::parse(name);
            interner.enum_case(qname, crate::enums::EnumBacking::Int, *case)
        }
        CoreTy::CallableTo(name) => interner.callable_to(*name),
        CoreTy::CallableShapeTo(name) => interner.callable_shape_to(*name),
        // A `Core`-owned instance is an ordinary class type from here on, for
        // the reason the enum arm above is an ordinary enum type: `seed` has
        // already put the class in this very table, so `resolve_method` finds
        // `$match->text()` through the machinery `$animal->name()` goes
        // through, and `nvs-ir` lowers the value to `Ty::Object`.
        //
        // **A generic one is interned at its own type variables**, never bare.
        // `Core\ObjectSet::union` answers `CoreTy::Instance(NAME)`, and what an
        // instance of a generic class *is* is that class at some arguments —
        // so writing none of them is what made `$set->union($other)` answer a
        // bare `Core\ObjectSet` and lose the element the receiver was built at.
        // Every site that reaches one then fixes the variable the way it fixes
        // any other: an instance member through
        // `crate::expr::args::substitute_receiver_args`, a written
        // `new Core\ObjectSet<Tag>()` through the type arguments themselves.
        CoreTy::Instance(name) => {
            let qname = QName::parse(name);
            match nvs_stdlib::registry::class_type_params(name) {
                Some(params) => {
                    let args: Vec<TypeId> = params
                        .iter()
                        .map(|param| interner.type_var(*param))
                        .collect();
                    interner.generic_class(qname, args)
                }
                None => interner.class(qname),
            }
        }
        // ADR 0053 § 3's three iterable shapes, interned as the union of all
        // three — [`CoreTy::Iterated`] owns why an `array<T>` is one of them
        // and how a helper reads the argument back. The two interface members
        // are the same generic class types [`crate::iter_lib`] seeds, so a
        // user class implementing `Iterable<int>` satisfies this parameter
        // through the ordinary `class_satisfied` rule rather than through
        // anything this arm has to know about.
        CoreTy::Iterated(elem) => {
            let elem = lower(elem, interner);
            let array = interner.array(elem);
            let iterable = interner.generic_class(QName::parse(ITERABLE), vec![elem]);
            let iterator = interner.generic_class(QName::parse(ITERATOR), vec![elem]);
            interner.make_union([array, iterable, iterator])
        }
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
        // `??`, `nvs_ir::lower_checked_ty`'s `Ty::Tagged`) meets one shape.
        CoreTy::Nullable(inner) => {
            let inner = lower(inner, interner);
            let null = interner.null();
            interner.make_union([null, inner])
        }
        // ADR 0047 § 1's integer atom, which the interner already has: the
        // registry variant exists only so a row can *write* one, and there is
        // nothing to translate beyond the value itself.
        CoreTy::IntLiteral(value) => interner.int_literal(*value),
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
    use nvs_hir::ClassGraph;

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

    /// ADR 0088 § 2's classification is the row's own judgement, so the
    /// signature the checker resolves has to *carry* it — `lower` interns a
    /// `Text`/`Blob` parameter at its plain type, and dropping the mark there
    /// is what left the whole surface refusing an argument it is meant to
    /// accept. Counted over every registered member rather than read off one
    /// row, so a fill that collapsed every parameter to one answer fails here
    /// while still looking right on any single line.
    #[test]
    fn every_registered_parameter_carries_its_classification_into_its_signature() {
        let mut interner = TypeInterner::new();
        let (mut contagious, mut sink, mut neutral) = (0_usize, 0_usize, 0_usize);
        for class in CLASSES {
            for method in class.members() {
                let sig = method_sig(method, true, &mut interner);
                assert_eq!(
                    sig.param_quals.len(),
                    sig.params.len(),
                    "{}::{} carries one classification slot per parameter",
                    class.name,
                    method.name
                );
                for (slot, param) in method.params.iter().enumerate() {
                    assert_eq!(
                        sig.param_quals[slot],
                        qual_of(param),
                        "{}::{} parameter {slot}",
                        class.name,
                        method.name
                    );
                    match sig.param_quals[slot] {
                        Some(Qual::Contagious) => contagious += 1,
                        Some(Qual::Sink) => sink += 1,
                        Some(Qual::Neutral) => neutral += 1,
                        _ => {}
                    }
                }
            }
        }
        assert!(
            contagious > 0 && sink > 0 && neutral > 0,
            "all three of the marks a row can write reach a signature \
             (contagious {contagious}, sink {sink}, neutral {neutral})"
        );
    }

    /// The same sweep, two marks further. ADR 0088 § 2 has **four**
    /// classifications and the test above proves only three of them arrive, so
    /// a [`Qual::Launder`] that stopped being lowered would leave every
    /// laundering row refusing the argument it exists to accept — and the only
    /// symptom would be a `Core\Regex::quote` call that no longer compiles.
    /// ADR 0033 § 3's [`Qual::Reveal`] is the fifth and answers on the other
    /// axis, and it is counted here now that `Core\Secret`'s rows write it: a
    /// mark that stopped being lowered would leave the one escape hatch every
    /// `secret` refusal's help text names refusing its own argument.
    ///
    /// The second half is ADR 0063 R11's four grammars, asked of the registry
    /// rather than of one row: each of the classes that owns one has to carry a
    /// sink somewhere in its rows, so a class whose pattern parameter lost its
    /// mark fails here while every case over the other three still passes.
    #[test]
    fn every_mark_a_row_can_write_reaches_a_signature() {
        let mut interner = TypeInterner::new();
        let (mut contagious, mut sink, mut neutral, mut launder, mut reveal) =
            (0_usize, 0, 0, 0, 0);
        let mut sinks_by_class: Vec<&'static str> = Vec::new();
        for class in CLASSES {
            for method in class.members() {
                let sig = method_sig(method, true, &mut interner);
                for qual in sig.param_quals.iter().flatten() {
                    match qual {
                        Qual::Contagious => contagious += 1,
                        Qual::Sink => {
                            sink += 1;
                            sinks_by_class.push(class.name);
                        }
                        Qual::Neutral => neutral += 1,
                        Qual::Launder => launder += 1,
                        // The fifth mark is the `secret` axis's, and
                        // `nvs_stdlib::secret`'s two rows are the only ones
                        // that may write it — `nvs_stdlib::registry`'s `Qual`
                        // doc comment is the home of why.
                        Qual::Reveal => reveal += 1,
                    }
                }
            }
        }
        assert!(
            contagious > 0 && sink > 0 && neutral > 0 && launder > 0 && reveal > 0,
            "all five marks reach a signature (contagious {contagious}, sink {sink}, \
             neutral {neutral}, launder {launder}, reveal {reveal})"
        );

        // A `Core\Time` pattern is declared on three classes — `DateTime`,
        // `Date` and `TimeOfDay` — so the grammar is named by its prefix and
        // not by one of them.
        for grammar in [r"Core\Regex", r"Core\Str", r"Core\Bytes", r"Core\Time"] {
            assert!(
                sinks_by_class.iter().any(|name| name.starts_with(grammar)),
                "ADR 0063 R11's {grammar} grammar carries a sink somewhere in its rows, \
                 among {sinks_by_class:?}"
            );
        }
    }

    /// The same classification, read the way a call site reads it: through the
    /// seeded table and [`MethodSig::qual_at`], which is the only accessor a
    /// consumer may use.
    #[test]
    fn a_resolved_member_answers_its_parameters_classification() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Str"),
            "join",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Str::join is registered");
        assert_eq!(
            sig.qual_at(0),
            None,
            "an array parameter has no cell to classify"
        );
        assert_eq!(
            sig.qual_at(1),
            Some(Qual::Contagious),
            "the separator decides which bytes come back, so it is contagious"
        );
    }

    /// § 9's three collections are the only `Core` classes that say anything
    /// about a hierarchy, and what they say is exactly what a `foreach` asks.
    /// A map yields its **keys**, so the element comes back as `K` and stays a
    /// variable until a receiver fixes it — `crate::expr::iteration` is what
    /// substitutes the subject's own arguments in.
    #[test]
    fn a_collection_implements_iterable_at_its_own_element() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = ClassGraph::default();

        let (interface, element) = crate::signatures::resolve_iteration_element(
            &QName::parse(r"Core\ObjectMap"),
            &table,
            &graph,
        )
        .expect("a map is iterable");
        assert_eq!(interface.to_string(), ITERABLE);
        assert_eq!(element, interner.type_var("K"));

        let (_, element) = crate::signatures::resolve_iteration_element(
            &QName::parse(r"Core\Heap"),
            &table,
            &graph,
        )
        .expect("a heap is iterable");
        assert_eq!(element, interner.type_var("T"));

        assert!(
            crate::signatures::resolve_iteration_element(
                &QName::parse(r"Core\Arr"),
                &table,
                &graph
            )
            .is_none(),
            "a namespace class is not a `foreach` subject"
        );
    }

    /// [`CoreTy::Iterated`] is exactly ADR 0053 § 3's three shapes, and the
    /// union it interns to is the one a program could have written out by
    /// hand — the property that keeps `Core\Arr::from($x)` accepting the same
    /// `$x` a `foreach` over it would.
    #[test]
    fn an_iterated_parameter_is_the_three_shapes_foreach_accepts() {
        let mut interner = TypeInterner::new();
        let lowered = lower(&CoreTy::Iterated(&CoreTy::Int), &mut interner);

        let int = interner.int();
        let array = interner.array(int);
        let iterable = interner.generic_class(QName::parse(ITERABLE), vec![int]);
        let iterator = interner.generic_class(QName::parse(ITERATOR), vec![int]);
        let expected = interner.make_union([array, iterable, iterator]);
        assert_eq!(lowered, expected);
    }

    /// The two variable kinds part company here and nowhere else: both lower
    /// to the same `Ty::TypeVar`, and only a `CoreTy::Written` one reaches
    /// `MethodSig::type_params`, which is what `crate::expr` reads to decide
    /// whether a call site may write `<...>` at all.
    #[test]
    fn only_a_written_variable_becomes_a_call_site_type_parameter() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "count",
            &table,
            &ClassGraph::default(),
        )
        .expect("count is registered");
        assert!(sig.is_generic(&interner), "`count` mentions `T`");
        assert!(sig.type_params.is_empty(), "but infers it from the subject");

        let written = lower(&CoreTy::Written("T"), &mut interner);
        let inferred = lower(&CoreTy::Var("T"), &mut interner);
        assert_eq!(written, inferred);
    }

    /// The bag's two halves, both derived from one registry row: the
    /// parameter's type carries the option names and types, and its
    /// [`MethodSig::defaults`] entry carries each option's default. Nothing in
    /// `nvs_stdlib::registry` states either twice, so this is the one place
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

    /// `CoreTy::IntLiteral` inside an option's union interns to exactly the
    /// two literal types a source-written `4|6` would — which is what makes
    /// `Core\Validate::isIp($s, {version: 5})` an `E0401` naming `4|6` rather
    /// than an `int` argument the member has to re-check at run time.
    ///
    /// The union is built here rather than compared against a `describe`
    /// string on purpose: a union orders its members by type id, so seeding a
    /// member anywhere could otherwise flip `4|6` to `6|4`.
    #[test]
    fn a_literal_union_option_interns_to_its_two_literals() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Validate"),
            "isIp",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Validate::isIp is registered");

        let four = interner.int_literal(4);
        let six = interner.int_literal(6);
        let version = interner.make_union([four, six]);
        let expected = interner.options(vec![("version".to_owned(), version)]);
        assert_eq!(sig.params[1], expected);
        // A literal is its own type, not the `int` it erases to — the whole
        // point of ADR 0047 § 1 at this position.
        let int = interner.int();
        assert_ne!(version, int);
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
        // Compared as an interned union rather than as its rendering: a union's
        // members are ordered by type id, so which of the two `describe` writes
        // first is decided by the order the whole registry happened to be
        // seeded in, and a member added to any other class can flip it.
        let null = interner.null();
        let var = interner.type_var("T");
        assert_eq!(sig.return_ty, interner.make_union([null, var]));
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
            Some("nvs_core_arr_count")
        );
        assert_eq!(symbol_of(&QName::parse(r"Core\Arr"), "nope"), None);
        assert_eq!(symbol_of(&QName::parse("Animal"), "count"), None);
    }
}
