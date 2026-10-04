//! Type variables: binding them from a call's arguments, and substituting
//! them back through the signature.
//!
//! `rule:types/declaration` parks
//! user-declared generics and `rule:attributes/call-site-type-argument` keeps type variables
//! **compiler-owned**, so exactly two things in the whole compiler produce a
//! [`Ty::TypeVar`]: [`crate::core_lib`] lowering a `nvs_stdlib::registry`
//! signature, and [`crate::iter_lib`] writing `rule:iteration/two-interfaces`'s two iteration
//! interfaces. The spec's own `Core\Arr` section states why they exist at all
//! — "`T` is a type variable — the stdlib is parametric where user code is
//! not."
//!
//! The two are bound from different places, and that difference is the only
//! subtlety here. A `Core` member's variable is bound from its *arguments*
//! ([`bind`], driven by [`crate::expr`]'s `check_generic_args`). An iteration
//! interface's is bound from its *receiver* — `$cursor->current()` takes `T`
//! from the `Iterator<int>` the receiver is already typed as, not from an
//! argument list that is empty. Both end at the same [`substitute`] call and
//! both keep the same property below.
//!
//! # What this is, and what it deliberately is not
//!
//! It is *not* a solver. There is no constraint set, no occurs check, no
//! unification variable that can be bound to another unification variable —
//! nothing that could fail to terminate or need an error message of its own.
//! It is one structural walk, in one direction:
//!
//! 1. [`bind`] walks a *declared* parameter type alongside the *actual*
//!    argument type. Wherever the declared side is a variable, that variable
//!    is bound to whatever the actual side is at that position. First binding
//!    wins, so a member declaring the same variable twice
//!    (`contains(array<T> $a, T $needle)`) takes it from the subject, which is
//!    `rule:core-api/shape-rules` R1's first parameter and therefore the argument a developer
//!    means to be authoritative.
//! 2. [`substitute`] rewrites the whole signature with those bindings. A
//!    variable no argument bound becomes `mixed` — the honest answer for "this
//!    position's type is unconstrained by this call," and the only one that
//!    keeps a later pass from meeting a variable it has no rule for.
//!
//! The consequence is the property everything downstream relies on:
//! **a type variable never survives a call site.** `crate::expr` substitutes
//! before it checks a single argument or records a `ResolvedCall`, so
//! `nvs-ir`, `nvs-codegen` and every diagnostic only ever see concrete types.
//!
//! # The one variable that is not at a position
//!
//! `map(array<T> $a, callable(T, string): U $fn): array<U>` finds `U` where
//! every other variable is found — at a structural position, inside the
//! parameter's own written signature, which [`bind`] descends into. That is
//! `rule:types/callable-signature`'s point: a callback's result is a type the
//! row states rather than a variable named beside an opaque `callable`.
//!
//! [`Ty::ShapeOfCallables`] is what is left, and the reason this section
//! exists at all. `Core\Task::all({...}): S` has its whole *result* at no one
//! position in its argument's type: `rule:concurrency/all-answers-a-typed-shape`
//! wants the argument's own field names carrying what each field's callable
//! returns, and a field is a position this walk cannot name in advance because
//! the call site chooses it. So [`bind`] rebuilds the shape instead — one pass
//! over the argument's fields, taking each [`Ty::CallableSig`]'s return type,
//! and `mixed` where a field declares no signature to read.
//!
//! It is still one walk over two types, and it is a **constraint as well as a
//! binding site**: what the parameter accepts is an ordinary assignability
//! question — a shape whose every field is a `callable` — which is why this is
//! the one variant [`substitute`] leaves standing. Erasing it would erase the
//! check that the argument is a shape of callables at all.

use nvs_hir::{ClassGraph, QName};
use rustc_hash::FxHashMap;

use crate::signatures::SignatureTable;
use crate::ty::{ShapeField, Ty, TypeId, TypeInterner};

/// Which concrete type each named variable was bound to.
pub(crate) type Bindings = FxHashMap<String, TypeId>;

/// Whether `id` mentions a type variable anywhere inside it — the test that
/// decides whether a call site needs any of this machinery at all. A
/// user-declared signature never does, so the ordinary path is unchanged.
#[must_use]
pub(crate) fn mentions_type_var(id: TypeId, interner: &TypeInterner) -> bool {
    match interner.get(id) {
        // `ShapeOfCallables` names a variable rather than being one, and binds
        // it from its own argument — so a signature carrying one needs this
        // machinery to run even where nothing else in it mentions a variable.
        Ty::TypeVar(_) | Ty::ShapeOfCallables(_) => true,
        Ty::Array(elem) => mentions_type_var(*elem, interner),
        Ty::CallableSig { params, ret } => {
            params
                .iter()
                .any(|param| mentions_type_var(*param, interner))
                || mentions_type_var(*ret, interner)
        }
        Ty::Class(_, args) => args.iter().any(|arg| mentions_type_var(*arg, interner)),
        Ty::Union(members) | Ty::Intersection(members) => members
            .iter()
            .any(|member| mentions_type_var(*member, interner)),
        Ty::Shape(fields) => fields
            .iter()
            .any(|field| mentions_type_var(field.ty, interner)),
        // The merged list alone: every key an arm declares has a slot there,
        // typed as the union of the arms' declarations for it, so a variable
        // mentioned by any arm is mentioned by that slot.
        Ty::CoreShape(shape) => shape
            .fields
            .iter()
            .any(|field| mentions_type_var(field.ty, interner)),
        _ => false,
    }
}

/// A type an `implements` clause wrote in terms of `qname`'s own type
/// variables, with the *receiver's* type arguments — `args` — put in.
///
/// A user class fixes its interface at a concrete type (`rule:iteration/concrete-generic-implements`), so this
/// is the identity for every class a program declares. `Core`'s
/// docs/spec/01-core-library.md § 9 collections are what need it: a
/// `Core\ObjectMap<K, V>` implements `Iterable<K>` for whatever `K` its
/// receiver was constructed at, so [`crate::signatures::resolve_interface_args`]
/// hands the variable back and the receiver is the only thing that can say what
/// it stands for. Anything the receiver leaves unbound becomes `mixed`, which
/// is [`substitute`]'s standing answer and keeps this module's "a type variable
/// never survives a call site" property.
///
/// **Three callers, one rule** — `foreach`'s element type
/// (`crate::expr::iteration`), the nominal assignability check
/// (`crate::expr::assign`'s `class_satisfied`) and [`bind`] below all ask the
/// same question of the same roster, and answering it three ways is how
/// `Core\Arr::from($set)` came to be refused for a set `foreach` walks happily.
pub(crate) fn with_class_args(
    qname: &QName,
    args: &[TypeId],
    id: TypeId,
    interner: &mut TypeInterner,
) -> TypeId {
    if args.is_empty() {
        return id;
    }
    let Some(params) = nvs_stdlib::registry::class_type_params(&qname.to_string()) else {
        return id;
    };
    let bindings: Bindings = params
        .iter()
        .map(|name| (*name).to_owned())
        .zip(args.iter().copied())
        .collect();
    substitute(id, &bindings, interner)
}

/// Binds every variable `declared` mentions against the matching position in
/// `actual`, recording into `out`. See this module's own docs for the
/// first-binding-wins rule and why there is no failure case: a shape the two
/// sides do not share simply binds nothing, and the unbound variable becomes
/// `mixed` in [`substitute`].
///
/// **Every arm answers with the pairs to walk into, and the walk runs after the
/// `match` rather than inside it.** One arm has to substitute, which needs the
/// interner mutably, and a scrutinee's borrow lasts the whole match. The arms
/// are mutually exclusive, so a call has exactly one group of pairs and
/// first-binding-wins is unaffected by where the recursion sits.
pub(crate) fn bind(
    declared: TypeId,
    actual: TypeId,
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
    out: &mut Bindings,
) {
    // The one arm whose pairs cannot be read straight off the two types: what
    // a class fixed for an interface is written in the class's own variables,
    // so the pairs exist only once `with_class_args` has put the receiver's
    // arguments in.
    // The one binding whose answer is at no position in either type: this
    // module's docs own why `rule:concurrency/all-answers-a-typed-shape` needs
    // a shape assembled rather than found. Handled ahead of the pair table
    // because assembling one interns, and the table reads both types at once.
    let shape_of_callables = match interner.get(declared) {
        Ty::ShapeOfCallables(name) => Some(name.clone()),
        _ => None,
    };
    if let Some(name) = shape_of_callables {
        let Ty::Shape(fields) = interner.get(actual).clone() else {
            // Not a shape at all, so there are no field names to answer with.
            // `crate::expr::assign` refuses the argument on its own; binding
            // nothing leaves the variable to substitute to `mixed`.
            return;
        };
        let mut results = Vec::with_capacity(fields.len());
        for field in &fields {
            let declared_ret = match interner.get(field.ty) {
                Ty::CallableSig { ret, .. } => Some(*ret),
                _ => None,
            };
            // A field typed bare `callable` is the lattice top and declares no
            // result — `mixed` is what that field is worth, and the rest of
            // the shape still answers precisely.
            let result = declared_ret.unwrap_or_else(|| interner.mixed());
            results.push(ShapeField::required(field.name.clone(), result));
        }
        let shape = interner.shape(results);
        out.entry(name).or_insert(shape);
        return;
    }

    let mut through_interface: Option<(QName, Vec<TypeId>, QName, Vec<TypeId>)> = None;
    let pairs: Vec<(TypeId, TypeId)> = match (interner.get(declared), interner.get(actual)) {
        (Ty::TypeVar(name), _) => {
            out.entry(name.clone()).or_insert(actual);
            Vec::new()
        }
        (Ty::Array(declared_elem), Ty::Array(actual_elem)) => {
            vec![(*declared_elem, *actual_elem)]
        }
        // Two written signatures, paired field-wise — a declared
        // `callable(T, string): U` against the `callable(User, string): string`
        // a written `fn` literal reports binds both variables out of one
        // argument. This is the binding a `Core` row used to name beside an
        // opaque `callable`, performed structurally instead: the callback's
        // result is a type here rather than a fact about an expression, so
        // nothing outside this arm has to know where it came from.
        //
        // The parameters stop at the shorter list, which is
        // `rule:types/callable-arity`'s prefix match read at the binding pass: an anonymous function
        // declaring fewer parameters than the position hands it binds from the
        // ones it wrote, and a position's surplus parameter binds nothing
        // rather than pairing with something else.
        (
            Ty::CallableSig {
                params: declared_params,
                ret: declared_ret,
            },
            Ty::CallableSig {
                params: actual_params,
                ret: actual_ret,
            },
        ) => declared_params
            .iter()
            .copied()
            .zip(actual_params.iter().copied())
            .chain(std::iter::once((*declared_ret, *actual_ret)))
            .collect(),
        // `Iterator<T>` against `Iterator<int>` — `rule:iteration/concrete-generic-implements`'s generic
        // interfaces, the only class-shaped type that carries arguments at
        // all. Two *different* names bind nothing, deliberately: this walk
        // has no notion of a supertype, so `Iterable<T>` against a `Counter`
        // is a miss rather than a wrong answer.
        (Ty::Class(declared_q, declared_args), Ty::Class(actual_q, actual_args))
            if declared_q == actual_q && declared_args.len() == actual_args.len() =>
        {
            declared_args
                .iter()
                .copied()
                .zip(actual_args.iter().copied())
                .collect()
        }
        // The same pair reached through `implements` rather than by name:
        // `Iterable<T>` against a `class Counter implements Iterable<int>`,
        // whose own class type carries no arguments at all because it declares
        // none. The arm above cannot see this, since what `Counter` fixed for
        // the interface lives in its signature rather than in its type — so
        // this asks `resolve_interface_args` for it, which is the same lookup
        // `expr::assign`'s nominal rule performs to *accept* the argument.
        // `nvs_stdlib::registry::CoreTy::Iterated` is what needs it: without
        // it `Core\Arr::from($counter)` would bind nothing and answer
        // `array<mixed>` for a sequence whose element type is written down.
        // What that lookup answers with is written in *`actual_q`'s* own
        // variables, which is why [`with_class_args`] stands between it and
        // the recursion: a `Core\ObjectMap<Tag, int>` fixes `Iterable<K>`, and
        // binding `T` to a bare `K` is how `Core\Arr::from` came to answer
        // `array<K>`.
        (Ty::Class(declared_q, declared_args), Ty::Class(actual_q, actual_args))
            if !declared_args.is_empty() && declared_q != actual_q =>
        {
            through_interface = Some((
                declared_q.clone(),
                declared_args.clone(),
                actual_q.clone(),
                actual_args.clone(),
            ));
            Vec::new()
        }
        // A declared **union**, which is where a registry row's
        // `array<T>|Iterable<T>|Iterator<T>` and its `?T` both arrive: bind
        // against every member and let first-binding-wins settle it. At most
        // one member can match a given argument structurally — an array is not
        // a class, and `null` binds nothing — so the interner's canonical
        // member order does not decide the answer.
        (Ty::Union(members), _) => members.iter().map(|member| (*member, actual)).collect(),
        // Neither a bag nor `rule:core-api/shape-parameter`'s shape parameter ever appears on the
        // `actual` side — a call site writes an anonymous object, which infers to
        // a `Ty::Shape` — so the two arms below cover every pair that occurs,
        // and binding a declared key's type against a matching written field is
        // the same walk either way.
        (Ty::Shape(declared_fields), Ty::Shape(actual_fields)) => declared_fields
            .iter()
            .filter_map(|declared| {
                actual_fields
                    .iter()
                    .find(|actual| actual.name == declared.name)
                    .map(|actual| (declared.ty, actual.ty))
            })
            .collect(),
        (Ty::CoreShape(declared), Ty::Shape(actual_fields)) => declared
            .fields
            .iter()
            .filter_map(|declared| {
                actual_fields
                    .iter()
                    .find(|actual| actual.name == declared.name)
                    .map(|actual| (declared.ty, actual.ty))
            })
            .collect(),
        _ => Vec::new(),
    };
    if let Some((declared_q, declared_args, actual_q, actual_args)) = through_interface
        && let Some(fixed) =
            crate::signatures::resolve_interface_args(&actual_q, &declared_q, signatures, graph)
        && fixed.len() == declared_args.len()
    {
        for (declared_arg, fixed_arg) in declared_args.into_iter().zip(fixed) {
            let actual_arg = with_class_args(&actual_q, &actual_args, fixed_arg, interner);
            bind(declared_arg, actual_arg, interner, graph, signatures, out);
        }
    }
    for (declared, actual) in pairs {
        bind(declared, actual, interner, graph, signatures, out);
    }
}

/// Rewrites `id` with `bindings` applied, interning whatever that produces.
/// A variable with no binding becomes `mixed`; anything mentioning no
/// variable is returned unchanged, so this allocates nothing on the ordinary
/// path.
pub(crate) fn substitute(id: TypeId, bindings: &Bindings, interner: &mut TypeInterner) -> TypeId {
    if !mentions_type_var(id, interner) {
        return id;
    }
    match interner.get(id).clone() {
        Ty::TypeVar(name) => bindings
            .get(&name)
            .copied()
            .unwrap_or_else(|| interner.mixed()),
        // The one arm that rewrites nothing. It is the parameter the argument
        // is checked against — a shape whose every field is a `callable` — so
        // erasing it would erase the check, and it names a variable rather
        // than being one, so there is nothing inside it to put a binding into.
        // See [`Ty::ShapeOfCallables`].
        Ty::ShapeOfCallables(_) => id,
        Ty::Array(elem) => {
            let elem = substitute(elem, bindings, interner);
            interner.array(elem)
        }
        // The one callable variant that does **not** collapse: a written
        // signature is a type rather than a binding site, so it is rewritten
        // field-wise like an array's element and comes back out a signature —
        // see [`Ty::CallableSig`].
        Ty::CallableSig { params, ret } => {
            let params: Vec<TypeId> = params
                .iter()
                .map(|param| substitute(*param, bindings, interner))
                .collect();
            let ret = substitute(ret, bindings, interner);
            interner.callable_sig(params, ret)
        }
        Ty::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|member| substitute(*member, bindings, interner))
                .collect();
            interner.make_union(members)
        }
        Ty::Intersection(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|member| substitute(*member, bindings, interner))
                .collect();
            interner.make_intersection(members)
        }
        Ty::Shape(fields) => {
            // The required bit rides along untouched for the reason the arm
            // below gives: substitution rewrites a key's type, never whether
            // the key has to be there.
            let fields: Vec<ShapeField> = fields
                .iter()
                .map(|field| ShapeField {
                    name: field.name.clone(),
                    ty: substitute(field.ty, bindings, interner),
                    required: field.required,
                })
                .collect();
            interner.shape(fields)
        }
        // Substituted in place, never through `shape`: a bag and a shape
        // parameter both keep their declared order because that order is their
        // ABI (`Ty::CoreShape`), and the required flag and the classification
        // ride along untouched — substitution rewrites a key's type, never
        // whether it must be written and never what it is a sink for.
        Ty::CoreShape(shape) => {
            let substitute_fields =
                |fields: &[crate::ty::CoreShapeField], interner: &mut TypeInterner| {
                    fields
                        .iter()
                        .map(|field| crate::ty::CoreShapeField {
                            name: field.name.clone(),
                            ty: substitute(field.ty, bindings, interner),
                            required: field.required,
                            qual: field.qual,
                            text: field.text,
                        })
                        .collect::<Vec<_>>()
                };
            // Both halves, because both are checked against: an arm left
            // unsubstituted would hold a written literal to a type variable no
            // value can be assignable to.
            let arms: Vec<Vec<crate::ty::CoreShapeField>> = shape
                .arms
                .iter()
                .map(|arm| substitute_fields(arm, interner))
                .collect();
            let fields = substitute_fields(&shape.fields, interner);
            interner.core_shape(crate::ty::CoreShape { fields, arms })
        }
        Ty::Class(qname, args) => {
            let args: Vec<TypeId> = args
                .iter()
                .map(|arg| substitute(*arg, bindings, interner))
                .collect();
            interner.generic_class(qname, args)
        }
        // Unreachable given the `mentions_type_var` guard above, which is
        // exactly the set of variants handled here — kept total rather than a
        // panic, since `Ty` is `#[non_exhaustive]` and a leaf variant added
        // later genuinely has nothing to substitute.
        _ => id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`super::bind`] with the two tables its `implements` arm consults left
    /// empty — every test below binds structurally, and a class reaching a
    /// generic interface through an `implements` clause is pinned by
    /// `crate::core_lib`'s own tests, where a seeded table exists. A local
    /// item shadows a glob-imported name, so each call site below reads
    /// exactly as it did before that arm.
    fn bind(declared: TypeId, actual: TypeId, interner: &mut TypeInterner, out: &mut Bindings) {
        super::bind(
            declared,
            actual,
            interner,
            &ClassGraph::default(),
            &SignatureTable::new(),
            out,
        );
    }

    #[test]
    fn a_bare_variable_binds_to_whatever_it_is_matched_against() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let int = interner.int();
        let mut bindings = Bindings::default();
        bind(t, int, &mut interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
    }

    /// The shape `Core\Arr::count` needs: `array<T>` against `array<int>`.
    #[test]
    fn a_variable_inside_an_array_binds_to_the_element_type() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let declared = interner.array(t);
        let int = interner.int();
        let actual = interner.array(int);
        let mut bindings = Bindings::default();
        bind(declared, actual, &mut interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
        assert_eq!(substitute(declared, &bindings, &mut interner), actual);
    }

    /// The shape a `Core` row spelling its callback's signature needs: both of
    /// `map`'s variables bound out of the one anonymous function argument, structurally.
    #[test]
    fn a_written_callback_signature_binds_from_the_anon_fns_own_signature() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let u = interner.type_var("U");
        let string = interner.string();
        let declared = interner.callable_sig(vec![t, string], u);
        let int = interner.int();
        let actual = interner.callable_sig(vec![int, string], string);
        let mut bindings = Bindings::default();
        bind(declared, actual, &mut interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
        assert_eq!(bindings.get("U"), Some(&string));
        assert_eq!(substitute(declared, &bindings, &mut interner), actual);
    }

    /// `rule:types/callable-arity`'s prefix match, read at the binding pass: a
    /// anonymous function that declares fewer parameters than the position hands it binds
    /// from the ones it wrote, and the surplus parameter binds nothing rather
    /// than pairing with the return type.
    #[test]
    fn a_shorter_anon_fn_binds_the_parameters_it_wrote_and_no_others() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let k = interner.type_var("K");
        let u = interner.type_var("U");
        let int = interner.int();
        let string = interner.string();
        let declared = interner.callable_sig(vec![t, k], u);
        let actual = interner.callable_sig(vec![int], string);
        let mut bindings = Bindings::default();
        bind(declared, actual, &mut interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
        assert_eq!(bindings.get("K"), None);
        assert_eq!(bindings.get("U"), Some(&string));
    }

    /// A variable a parameter *contains* binds as readily as one that is a
    /// whole parameter: the pairs an arm answers with are walked, so
    /// `callable(array<T>, string): U` reaches `T` through the array the same
    /// way `array<T>` at an argument does. Nothing in the walk is special to
    /// the top level, and a row nesting one is therefore not a shape this has
    /// to grow an arm for.
    #[test]
    fn bind_descends_into_a_callable_types_parameters() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let u = interner.type_var("U");
        let string = interner.string();
        let int = interner.int();
        let var_array = interner.array(t);
        let int_array = interner.array(int);
        let declared = interner.callable_sig(vec![var_array, string], u);
        let actual = interner.callable_sig(vec![int_array, string], string);
        let mut bindings = Bindings::default();
        bind(declared, actual, &mut interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
        assert_eq!(bindings.get("U"), Some(&string));
    }

    #[test]
    fn the_first_binding_of_a_repeated_variable_wins() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let int = interner.int();
        let string = interner.string();
        let mut bindings = Bindings::default();
        bind(t, int, &mut interner, &mut bindings);
        bind(t, string, &mut interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
    }

    #[test]
    fn an_unbound_variable_substitutes_to_mixed() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let declared = interner.array(t);
        let bindings = Bindings::default();
        let mixed = interner.mixed();
        let expected = interner.array(mixed);
        assert_eq!(substitute(declared, &bindings, &mut interner), expected);
    }

    /// A mismatched shape binds nothing rather than failing — the "no error
    /// case" property this module's docs claim.
    #[test]
    fn a_non_matching_actual_type_binds_nothing() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let declared = interner.array(t);
        let int = interner.int();
        let mut bindings = Bindings::default();
        bind(declared, int, &mut interner, &mut bindings);
        assert!(bindings.is_empty());
    }

    /// The variable a callback's signature names is what the *return* type
    /// reads back — `array<U>` becomes `array<string>` for a callback
    /// returning `string`.
    #[test]
    fn the_bound_callback_result_reaches_the_return_type() {
        let mut interner = TypeInterner::new();
        let u = interner.type_var("U");
        let declared = interner.array(u);
        let string = interner.string();
        let expected = interner.array(string);

        let mut bindings = Bindings::default();
        bindings.insert("U".to_owned(), string);
        assert_eq!(substitute(declared, &bindings, &mut interner), expected);
    }

    #[test]
    fn a_type_with_no_variable_is_returned_unchanged() {
        let mut interner = TypeInterner::new();
        let int = interner.int();
        let array = interner.array(int);
        assert!(!mentions_type_var(array, &interner));
        let bindings = Bindings::default();
        assert_eq!(substitute(array, &bindings, &mut interner), array);
    }
}
