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
//!   `$a->count()` resolves to nothing, as `rule:core-api/shape-rules` R20 ("no operation is
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
use nvs_hir::interfaces::{COMPARABLE, ITERABLE, ITERATOR, PARSES};
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
        // `rule:classes/comparable`'s `Comparable`, which a `Core` class satisfies by carrying
        // the member rather than by naming the interface —
        // `nvs_stdlib::registry::implements_comparable` owns why, and seeding
        // it here is what lets `$a < $b` reach the same
        // `crate::expr::operators::object_comparison_result` a user class's
        // `implements Comparable` reaches. No arguments: the interface takes
        // none (`nvs_hir::interfaces::RESERVED`).
        if nvs_stdlib::registry::implements_comparable(class.name) {
            table.seed_implements(qname.clone(), QName::parse(COMPARABLE), Vec::new());
        }
        // `rule:expressions/try-parse`'s pair, read as `Parses` on exactly the
        // same terms: `nvs_stdlib::registry::implements_parses` asks the rows
        // because a `Core` class has no `implements` clause to write, and this
        // is the one place the edge a binding site names the interface at gets
        // written. No arguments — `Parses` takes none
        // (`nvs_hir::interfaces::RESERVED`).
        if nvs_stdlib::registry::implements_parses(class.name) {
            table.seed_implements(qname.clone(), QName::parse(PARSES), Vec::new());
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
        // `rule:core-api/shape-rules` R2: every `Core` parameter is callable by the spec's
        // `$name`. The row carries one name per positional slot and none
        // for a trailing bag, so `param_names` re-aligns them to `params`
        // — see its own docs for why the bag's entry is made there.
        param_names: param_names(method),
        // `rule:security/unclassified-parameter-refuses-tainted`'s classification, carried rather than dropped.
        // `lower` interns a `Text`/`Blob` parameter at its plain type
        // on purpose — what a member *does* with a qualifier is not
        // what the argument *is* — so this is the one place the row's
        // judgement reaches the checker. See `MethodSig::param_quals`.
        param_quals: method.params.iter().map(qual_of).collect(),
        // `rule:core-api/shape-rules` R7: nothing in `Core` mutates its subject, so
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
        // A `Core` member is reachable exactly one way (`rule:core-api/shape-rules`
        // R20): a static one through its class name, an instance
        // one through a value. The registry states which by which
        // roster the row is written in — see
        // `nvs_stdlib::registry::CoreClass::instance`.
        is_static,
        interface_private: false,
        // Every registered row is part of `Core`'s surface — the
        // registry has no way to write an internal one, so there
        // is nothing here for `rule:core-api/written-visibility`'s levels to say.
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
/// call: a constant is `rule:classes/no-free-functions-or-constants`'s
/// "every constant is a class constant", and `rule:enums/no-class-machinery`'s inlining rule for
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
///
/// A [`CoreTy::Shape`](nvs_stdlib::registry::CoreTy::Shape) parameter gets its
/// entry synthesized the same way, from [`shape_fills`], and this is the one
/// place `rule:core-api/shape-flattens-at-the-abi`'s fills are recorded. Which *variant* carries them is
/// the parameter's own optionality and nothing else: a shape a call must write
/// takes [`ConstArg::RequiredShape`], which
/// [`MethodSig::required`](crate::signatures::MethodSig::required) skips, and
/// an optional one — no registry row declares one yet — takes
/// [`ConstArg::Options`], because for an omittable parameter the fills *are*
/// what an omitting call passes. Either way the row's own `defaults` entry for
/// that slot is not read: a shape has no single constant to be defaulted to.
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
            if let CoreTy::Shape(arms) = &method.params[index] {
                let fills = shape_fills(arms);
                return Some(match index >= required {
                    true => ConstArg::Options(fills),
                    false => ConstArg::RequiredShape(fills),
                });
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
/// `rule:core-api/shape-rules` R2 says a name never reaches one, and
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
        Const::NeverWritten => ConstArg::NeverWritten,
        Const::Bool(b) => ConstArg::Bool(b),
        Const::Int(v) => ConstArg::Int(v),
        Const::Uint(v) => ConstArg::Uint(v),
        Const::Float(v) => ConstArg::Float(v),
        Const::Str(s) => ConstArg::Str(s.to_owned()),
        Const::Bytes(b) => ConstArg::Bytes(b.to_vec()),
        Const::EmptyArray => ConstArg::EmptyArray,
        // `rule:enums/no-class-machinery`: the case *is* its integer constant, so what a call
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
        CoreTy::Variadic(elem) => qual_of(elem),
        // Which spellings carry one is the registry's own question and this is
        // not a second answer to it: every other walk here reads a row, and a
        // leaf list restated on this side is how the two drift. What this adds
        // is the variadic rule above, which the registry has no reason to know
        // — an `array<text>` element and an options bag's members are
        // unclassified for the reason `MethodSig::param_quals` records, and a
        // `None` refuses a qualified argument exactly as `Sink` does.
        _ => ty.classification(),
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
        // It **is** read back at the call: `crate::expr::args`' `check_arg`
        // asks `MethodSig::qual_at` for the parameter's mark and
        // `crate::expr::quals`' `admits_tainted_argument`/
        // `admits_secret_argument` decide from it — `Sink` refuses a qualified
        // argument, `Neutral`, `Launder` and `Reveal` admit one, and
        // `Contagious` admits one exactly where the result can carry the
        // qualifier back out.
        //
        // **What has no mark is a nested spelling.** `qual_of` answers `None`
        // for a `CoreTy::Union`, for an `array<text>` element and for an
        // options bag's members, and `None` refuses a qualified argument
        // exactly as `Sink` does. That is safe and over-strict — it is how
        // `Core\Regex`'s seven `Pattern|string` parameters refuse a tainted
        // pattern without a row saying so — and it is the reason a union can
        // never be given an *accepting* mark without widening this function.
        CoreTy::Str | CoreTy::Text(_) => interner.string(),
        // `rule:security/isolate-shares-nothing`'s entry operand accepts two written shapes and is a
        // *syntactic* rule over them, so there is no declared type that states
        // it: `string` would report the accepted `Chat::run(...)` as a
        // mismatch, and `string|callable` would name a variable holding a
        // callable as accepted. It interns as the type that reports nothing
        // and `super::expr::isolate`'s `check_entry` states the whole rule,
        // exactly as `Core\Debug::dump`'s `mixed` leaves its own refusal to
        // the call site. `nvs_stdlib::registry::CoreTy::Entry` owns why.
        CoreTy::Entry => interner.mixed(),
        CoreTy::Bytes | CoreTy::Blob(_) => interner.bytes(),
        // The one pair that carries a *qualifier* rather than a
        // classification, so unlike every other spelling above it interns as a
        // qualified atom — `CoreTy::SecretBytes`'s own docs are the home of why
        // that is a different thing from a `Qual` and what it buys in return
        // position. Nothing about the admission rules changes: a plain `bytes`
        // argument still reaches this parameter, because `super::assign` widens
        // onto a qualifier bit and never off one.
        CoreTy::SecretBytes | CoreTy::SecretBlob(_) => interner.secret_bytes(),
        // The same arm on the text base, and the row that needs it is
        // `Core\Crypto::deriveKey`: a password is `secret` where a key is, and
        // a parameter spelled `string` would refuse one outright, because
        // `super::assign` narrows through no qualifier bit and the marks that
        // admit a `secret` argument are `rule:core-classes/secret-reveal`'s two
        // classes wide.
        CoreTy::SecretText(_) => interner.secret_string(),
        // The same pair of questions as the arm above, on the other axis: this
        // is a qualifier and not a classification, so it interns qualified.
        // `rule:security/verification-does-not-launder` is what needs it — a verified signature does not
        // launder, and the only way to say that of a *result* is to type the
        // result. A `Qual::Contagious` parameter would have made the claims
        // tainted exactly when the token already was, which is the property
        // that ADR reads like an oversight for not having.
        CoreTy::TaintedStr => interner.tainted_string(),
        // The same arm on the octet axis, and it interns qualified for the same
        // reason: `Core\Request::bodyStream` hands back the bytes of the body
        // `body()` already types `tainted string`, one chunk at a time.
        CoreTy::TaintedBytes => interner.tainted_bytes(),
        // Both axes at once, for the arms above's reasons taken together: ADR
        // 0086 § 4's prompt answers a password that is confidential and came
        // from outside, and only a qualified result type can say both.
        CoreTy::SecretTaintedStr => interner.secret_tainted_string(),
        // The same arm on the octet axis. Both of these are written in an
        // options bag as well as in return position, and there they are the
        // *only* way to admit a qualifier: `qual_of` above answers `None` for an
        // option, so `Core\Http\Client`'s body keys say in the type what a
        // parameter would have said with a [`Qual`].
        CoreTy::SecretTaintedBytes => interner.secret_tainted_bytes(),
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
        // `rule:types/enum-case-type`'s narrowed case type, interned exactly as a source-
        // written `Core\Digest::Sha256` in type position is — `crate::lower`
        // reaches the same `enum_case` for that spelling, so the union a
        // registry row builds out of these and one a program could write are
        // the same interned id.
        CoreTy::EnumCase(name, case) => {
            let qname = QName::parse(name);
            interner.enum_case(qname, crate::enums::EnumBacking::Int, *case)
        }
        CoreTy::ShapeOfCallables(name) => interner.shape_of_callables(*name),
        // Lowered field-wise, so a variable a registry row wrote inside a
        // callback's signature is the same `Ty::TypeVar` the member's other
        // parameters intern to — which is what lets `crate::generics`
        // substitute the call's own bindings through it and hand
        // `crate::expr::calls`' `check_fn_literal` an expected type naming
        // `User` rather than `T`. `Ty::CallableSig` owns why this is a type
        // rather than a binding site like the two arms above it.
        CoreTy::CallableSig(params, ret) => {
            let params: Vec<TypeId> = params.iter().map(|param| lower(param, interner)).collect();
            let ret = lower(ret, interner);
            interner.callable_sig(params, ret)
        }
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
        // The same class type, at the arguments the row wrote rather than at
        // the class's own variables — `nvs_stdlib::registry::CoreTy::InstanceAt`
        // owns why a member that *produces* a generic instance needs its own
        // spelling. Nothing is checked here: the arity against the roster is
        // `every_instance_type_names_a_registered_class`'s, in the crate that
        // owns the roster, and by the time a row reaches this it holds.
        CoreTy::InstanceAt(name, args) => {
            let qname = QName::parse(name);
            let args: Vec<TypeId> = args.iter().map(|arg| lower(arg, interner)).collect();
            interner.generic_class(qname, args)
        }
        // `rule:iteration/foreach-subjects`'s three iterable shapes, interned as the union of all
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
        // `rule:types/literal-types`'s integer atom, which the interner already has: the
        // registry variant exists only so a row can *write* one, and there is
        // nothing to translate beyond the value itself.
        CoreTy::IntLiteral(value) => interner.int_literal(*value),
        // The registry's order is kept, not sorted: it is the order the bag
        // flattens into ABI arguments. `Ty::CoreShape` owns why.
        CoreTy::Options(options) => {
            let options = options
                .iter()
                .map(|option| {
                    (
                        option.name.to_owned(),
                        lower(&option.ty, interner),
                        qual_of(&option.ty),
                    )
                })
                .collect();
            interner.options(options)
        }
        // `rule:core-api/shape-flattens-at-the-abi`'s ABI, built here and nowhere else: the arms in
        // declaration order, each arm's fields in declaration order, a name
        // already emitted skipped. See [`merge_shape_arms`].
        CoreTy::Shape(arms) => {
            let shape = crate::ty::CoreShape {
                fields: merge_shape_arms(arms, interner),
                arms: arms.iter().map(|arm| lower_arm(arm, interner)).collect(),
            };
            interner.core_shape(shape)
        }
        // The top of the instance half of the lattice, and the one spelling a
        // registry row has for "structured, whatever its declaration says":
        // `crate::expr::is_assignable` satisfies it with a class instance and
        // with a shape, so `Core\Jwt::signObject`'s claims are checked at the
        // call rather than in the helper.
        CoreTy::Object => interner.object(),
        // `Mixed` and anything a later registry variant adds: `mixed` is the
        // registry's own "unchecked position" spelling, and is the only safe
        // answer for a variant this arm has not learned yet, since `Ty` and
        // `CoreTy` are both `#[non_exhaustive]`.
        _ => interner.mixed(),
    }
}

/// `rule:core-api/shape-flattens-at-the-abi`'s merged field list: the arms in declaration order, each arm's
/// fields in declaration order, a name a previous arm already emitted skipped.
///
/// A name more than one arm declares occupies **one** slot whose type is the
/// union of what those arms declare for it — that is what lets `Db\Settings`'s
/// `driver` arrive as one `Driver` value the helper switches on rather than as
/// one slot per arm.
///
/// A key is required of the merged parameter only where **every** arm requires
/// it. A key an arm does not declare at all is one that arm's call site
/// legitimately omits, so treating "required in the arm that has it" as
/// required of the parameter would refuse every other arm's literal.
fn merge_shape_arms(
    arms: &[&'static [nvs_stdlib::registry::CoreField]],
    interner: &mut TypeInterner,
) -> Vec<crate::ty::CoreShapeField> {
    let mut merged: Vec<crate::ty::CoreShapeField> = Vec::new();
    for (name, declared) in merged_arm_fields(arms) {
        let mut ty = lower(&declared[0].ty, interner);
        for field in &declared[1..] {
            let next = lower(&field.ty, interner);
            ty = interner.make_union([ty, next]);
        }
        let required = arms.iter().all(|arm| {
            arm.iter()
                .any(|other| other.name == name && other.default.is_none())
        });
        merged.push(crate::ty::CoreShapeField {
            name: name.to_owned(),
            ty,
            required,
            // The first arm that declares the name, which is the slot's own
            // declaration order: § 3 merges the *type* of a name two arms
            // share and nothing merges a classification, because a name
            // classified two ways would be one ABI slot with two rules. No
            // registry row writes one, and `SETTINGS`' shared `driver`,
            // `timeZone` and `timeout` classify nothing at all.
            qual: qual_of(&declared[0].ty),
        });
    }
    merged
}

/// One arm's own fields, lowered — `rule:core-api/shape-arms-are-disjoint`'s half of
/// [`crate::ty::CoreShape`], where [`merge_shape_arms`] builds § 3's.
///
/// A field is required of its arm exactly where it declares no default, which
/// is `CoreField::default` read the way [`merge_shape_arms`] reads it. The
/// merge can only say that where *every* arm agrees, and saying it per arm is
/// the whole of what arm selection needs that the merged list cannot give it.
fn lower_arm(
    arm: &'static [nvs_stdlib::registry::CoreField],
    interner: &mut TypeInterner,
) -> Vec<crate::ty::CoreShapeField> {
    arm.iter()
        .map(|field| crate::ty::CoreShapeField {
            name: field.name.to_owned(),
            ty: lower(&field.ty, interner),
            required: field.default.is_none(),
            qual: qual_of(&field.ty),
        })
        .collect()
}

/// One slot of `rule:core-api/shape-flattens-at-the-abi`'s merged list per entry, in the order the list
/// flattens, carrying every arm declaration behind that slot: the arms in
/// declaration order, each arm's fields in declaration order, a name a
/// previous arm already emitted folded into the slot it already has.
///
/// The order is stated here once because two things describe the same
/// flattening — [`merge_shape_arms`] builds the checked type's fields and
/// [`shape_fills`] the constant each slot takes — and a parameter whose type
/// and whose fills disagreed about slot order would pass every argument one
/// position out.
fn merged_arm_fields(
    arms: &[&'static [nvs_stdlib::registry::CoreField]],
) -> Vec<(&'static str, Vec<&'static nvs_stdlib::registry::CoreField>)> {
    let mut merged: Vec<(&'static str, Vec<&'static nvs_stdlib::registry::CoreField>)> = Vec::new();
    for arm in arms {
        for field in *arm {
            match merged.iter_mut().find(|(name, _)| *name == field.name) {
                Some((_, declared)) => declared.push(field),
                None => merged.push((field.name, vec![field])),
            }
        }
    }
    merged
}

/// `rule:core-api/shape-flattens-at-the-abi`'s fill list: one constant per slot of [`merged_arm_fields`]'s
/// order — the first declaring arm's own default where it has one, and `null`
/// for a field belonging to an arm the caller did not write.
///
/// Every slot gets an entry, including the required ones, because the list is
/// read per *call site*: a key required by the arm the caller wrote is filled
/// by the literal and never reaches here, and the same key is a key some other
/// arm's call site does not write at all. So "no default" is `null` rather than
/// an absence — which is also why a shape field is never nullable
/// (`rule:core-api/shape-flattens-at-the-abi`), so that `null` cannot be mistaken for a written one.
fn shape_fills(arms: &[&'static [nvs_stdlib::registry::CoreField]]) -> Vec<(String, ConstArg)> {
    merged_arm_fields(arms)
        .into_iter()
        .map(|(name, declared)| {
            let fill = declared
                .iter()
                .find_map(|field| field.default.as_ref())
                .map_or(ConstArg::Null, lower_const);
            (name.to_owned(), fill)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::resolve_method;
    use crate::ty::Ty;
    use nvs_hir::ClassGraph;
    use nvs_stdlib::registry::CoreField;

    /// `rule:core-api/shape-flattens-at-the-abi`'s ABI, held over a two-arm shape because no registry row
    /// declares one yet: the arms in declaration order, a name a previous arm
    /// already emitted taking **one** slot whose type is the union of what the
    /// arms declare for it, and a key required only where every arm requires
    /// it. Written here rather than over `Core\Db::open` so it still holds when
    /// that row's arms change.
    #[test]
    fn a_shapes_arms_merge_in_declaration_order_and_deduplicate_by_name() {
        const SERVER: &[CoreField] = &[
            CoreField {
                name: "driver",
                ty: CoreTy::Str,
                default: None,
            },
            CoreField {
                name: "host",
                ty: CoreTy::Str,
                default: None,
            },
            CoreField {
                name: "port",
                ty: CoreTy::Int,
                default: Some(Const::Int(5432)),
            },
        ];
        const FILE: &[CoreField] = &[
            CoreField {
                name: "driver",
                ty: CoreTy::Int,
                default: None,
            },
            CoreField {
                name: "path",
                ty: CoreTy::Str,
                default: None,
            },
        ];

        let mut interner = TypeInterner::new();
        let id = lower(&CoreTy::Shape(&[SERVER, FILE]), &mut interner);
        let Ty::CoreShape(shape) = interner.get(id) else {
            panic!("a `CoreTy::Shape` lowers to a `Ty::CoreShape`");
        };
        let fields = &shape.fields;
        // `rule:core-api/shape-arms-are-disjoint`'s half, which the merge deliberately cannot state: each
        // arm keeps its own keys, its own types and its own required flags, so
        // `host` is required *of the server arm* where the merged list below
        // has to call it optional.
        assert_eq!(
            shape
                .arms
                .iter()
                .map(|arm| arm
                    .iter()
                    .map(|f| (f.name.as_str(), f.required))
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            [
                vec![("driver", true), ("host", true), ("port", false)],
                vec![("driver", true), ("path", true)],
            ],
        );
        assert_eq!(
            fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["driver", "host", "port", "path"],
            "the arms in declaration order, `driver` deduplicated to one slot",
        );
        // `driver` is declared and required by both arms; `host` and `path` are
        // required by one arm each, and the *other* arm's call site legitimately
        // omits them, so neither is required of the parameter.
        assert_eq!(
            fields.iter().map(|f| f.required).collect::<Vec<_>>(),
            [true, false, false, false],
        );
        assert_eq!(
            interner.describe(fields[0].ty),
            "string|int",
            "the shared key's slot is the union of the arms' declarations",
        );
    }

    /// `rule:core-api/shape-flattens-at-the-abi`'s fill list, and the reason it is a variant of its own: a
    /// shape parameter records what its unwritten slots pass *and* stays
    /// required, which a `ConstArg::Options` entry could not do — `required`
    /// reads optionality off this vector, so a bag-shaped entry here would let
    /// a call omit the whole parameter. Over the same two-arm fixture the
    /// merge test uses, because no registry row declares a shape yet.
    #[test]
    fn a_required_shape_records_its_fills_without_becoming_optional() {
        const SERVER: &[CoreField] = &[
            CoreField {
                name: "driver",
                ty: CoreTy::Str,
                default: None,
            },
            CoreField {
                name: "port",
                ty: CoreTy::Int,
                default: Some(Const::Int(5432)),
            },
        ];
        const FILE: &[CoreField] = &[
            CoreField {
                name: "driver",
                ty: CoreTy::Int,
                default: None,
            },
            CoreField {
                name: "path",
                ty: CoreTy::Str,
                default: None,
            },
        ];
        const OPEN: nvs_stdlib::registry::CoreMethod = nvs_stdlib::registry::CoreMethod {
            name: "open",
            names: &["settings"],
            params: &[CoreTy::Shape(&[SERVER, FILE])],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_shape_fixture_open",
            doc: None,
        };

        let mut interner = TypeInterner::new();
        let sig = method_sig(&OPEN, true, &mut interner);
        assert_eq!(
            sig.required(),
            1,
            "a shape parameter carrying its fills is still written at every call site",
        );
        let Some(ConstArg::RequiredShape(fills)) = &sig.defaults[0] else {
            panic!("a required shape's fills are a `RequiredShape` entry, not a bag's");
        };
        // The merged list's own order (`merged_arm_fields`), one entry per
        // slot: `port`'s own default, and `null` for a key belonging to an arm
        // the caller did not write.
        assert_eq!(
            fills,
            &vec![
                ("driver".to_owned(), ConstArg::Null),
                ("port".to_owned(), ConstArg::Int(5432)),
                ("path".to_owned(), ConstArg::Null),
            ],
        );
        // The type and the fills describe one flattening, so a slot is the
        // same slot in both — the invariant `merged_arm_fields` exists for.
        let Ty::CoreShape(shape) = interner.get(sig.params[0]) else {
            panic!("a `CoreTy::Shape` parameter lowers to a `Ty::CoreShape`");
        };
        assert_eq!(
            shape
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            fills
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
        );
    }

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

    /// `rule:security/unclassified-parameter-refuses-tainted`'s classification is the row's own judgement, so the
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

    /// The same sweep, two marks further. `rule:security/unclassified-parameter-refuses-tainted` has **four**
    /// classifications and the test above proves only three of them arrive, so
    /// a [`Qual::Launder`] that stopped being lowered would leave every
    /// laundering row refusing the argument it exists to accept — and the only
    /// symptom would be a `Core\Regex::quote` call that no longer compiles.
    /// `rule:core-classes/secret-reveal`'s [`Qual::Reveal`] is the fifth and answers on the other
    /// axis, and it is counted here now that `Core\Secret`'s rows write it: a
    /// mark that stopped being lowered would leave the one escape hatch every
    /// `secret` refusal's help text names refusing its own argument.
    ///
    /// The second half is `rule:core-api/shape-rules` R11's four grammars, asked of the registry
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
        //
        // The `secret` axis's own roster is asked separately, in
        // `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`.
        for grammar in [r"Core\Regex", r"Core\Str", r"Core\Bytes", r"Core\Time"] {
            assert!(
                sinks_by_class.iter().any(|name| name.starts_with(grammar)),
                "`rule:core-api/shape-rules` R11's {grammar} grammar carries a sink somewhere in its rows, \
                 among {sinks_by_class:?}"
            );
        }
    }

    /// `rule:core-classes/secret-reveal`, asked of the whole registry rather than of one row: which
    /// parameters may be handed a `secret` at all?
    ///
    /// [`Qual::Reveal`] is the only mark that admits one, and the roster of
    /// members that write it is **four rows across two classes** —
    /// `Core\Secret`, whose two members are the escape hatch itself, and
    /// `Core\Password`, whose `hash` and `verify` are the one operation that
    /// takes a password and answers something deliberately not a password
    /// (`nvs_stdlib::registry`'s `Qual` doc comment is the home of why the
    /// roster is closed at two).
    ///
    /// Asserted as a **set**, because a third class quietly gaining the mark is
    /// invisible from any one row: every individual `Reveal` looks exactly like
    /// the four legitimate ones, and the whole confidentiality claim is that
    /// there are no others. A member added here on purpose fails this test and
    /// is meant to — the edit that adds it is the edit that decides the roster
    /// grew.
    #[test]
    fn reveal_and_the_password_helpers_are_the_only_launderers_of_secret() {
        use crate::expr::quals::admits_secret_argument;
        use std::collections::BTreeSet;

        let mut interner = TypeInterner::new();
        let mut launderers: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
        for class in CLASSES {
            for method in class.members() {
                let sig = method_sig(method, true, &mut interner);
                if sig
                    .param_quals
                    .iter()
                    .flatten()
                    .any(|qual| matches!(qual, Qual::Reveal))
                {
                    launderers.insert((class.name, method.name));
                }
            }
        }
        assert_eq!(
            launderers,
            BTreeSet::from([
                (r"Core\Password", "hash"),
                (r"Core\Password", "verify"),
                (r"Core\Secret", "reveal"),
                (r"Core\Secret", "revealBytes"),
            ]),
            "the roster of members that may be handed a `secret` is closed"
        );

        // The other half of the claim, and the reason the set above is the
        // whole answer: no other mark admits a `secret`, so a row that wanted
        // to launder one quietly would have to write `Reveal` and land in it.
        for qual in [Qual::Contagious, Qual::Sink, Qual::Neutral, Qual::Launder] {
            assert!(
                !admits_secret_argument(Some(qual)),
                "{qual:?} refuses a `secret` argument"
            );
        }
        assert!(
            !admits_secret_argument(None),
            "an unclassified parameter refuses a `secret` argument"
        );
        assert!(admits_secret_argument(Some(Qual::Reveal)));
    }

    /// `rule:security/launderers-are-sink-named`'s escape hatch, asked the way the test above asks `rule:core-classes/secret-reveal`'s: as a **closed set** rather than of the one row.
    ///
    /// [`Qual::Launder`] obliges a doc comment naming the sink the row launders
    /// for, and `Core\Taint::assertTrusted` is the one row that names all of
    /// them instead. `nvs_stdlib::registry`'s `Qual` doc comment argues why,
    /// and holds that exception **on the roster rather than on the mark** —
    /// exactly as `Qual::Reveal`'s two-class roster is held rather than spelled
    /// into a variant. This is the `tainted` twin of that roster, and there is
    /// nowhere else it could live: a second general-purpose hatch is invisible
    /// from any one row, since every individual `Launder` looks exactly like
    /// the sink-named ones.
    ///
    /// A doc comment cannot be read from here, so the property asserted is the
    /// structural one a general-purpose hatch has and a sink-named launderer
    /// does not: **its class is nothing else, and its answer is a plain
    /// `string`.** `Core\Html`, `Core\Uri` and `Core\Regex` each launder for
    /// the sink their other members are about, so a `Launder` row there is a
    /// member of a domain. `Core\Cli\Text` is the near miss the second half
    /// exists for — its whole roster is `Launder` rows, and they answer the
    /// terminal's *carrier*, which makes them constructors for that sink rather
    /// than an escape from every sink (`nvs_stdlib::html`'s
    /// `every_launderer_for_an_auto_escaping_sink_answers_a_carrier` is where
    /// that reading is argued, against `rule:security/launderer-answers-a-carrier`'s table).
    #[test]
    fn the_launderer_that_names_no_sink_is_one_class_and_one_row() {
        use std::collections::BTreeSet;

        let mut interner = TypeInterner::new();
        let mut hatches: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
        for class in CLASSES {
            let mut whole_class: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
            let mut every_row_is_one = false;
            for method in class.members() {
                let sig = method_sig(method, true, &mut interner);
                let launders = sig
                    .param_quals
                    .iter()
                    .flatten()
                    .any(|qual| matches!(qual, Qual::Launder));
                if !launders || interner.describe(sig.return_ty) != "string" {
                    every_row_is_one = false;
                    break;
                }
                every_row_is_one = true;
                whole_class.insert((class.name, method.name));
            }
            if every_row_is_one {
                hatches.extend(whole_class);
            }
        }

        assert_eq!(
            hatches,
            BTreeSet::from([(r"Core\Taint", "assertTrusted")]),
            "the roster of launderers that answer for every sink is closed at one — `rule:security/launderers-are-sink-named` \
             makes laundering sink-named and names a single exception, and a second class here is \
             the generic `sanitize()` that section exists to refuse"
        );
    }

    /// `rule:security/verification-does-not-launder`, which reads like an oversight and is a decision: a
    /// verified signature proves origin, not safety for any sink, so
    /// `Core\Jwt::verify`'s claims come back **`tainted`** — and the one
    /// signature check in the language that does the opposite is the cookie,
    /// which laundered a value the application itself sealed.
    ///
    /// Asked as a **pair of sets** rather than of the two rows, for
    /// `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`'s
    /// reason. A member that promises `tainted` is invisible from any row but
    /// its own, and the whole content of § 5 is that a *third* signature
    /// verifier cannot quietly join the laundering side: a row that wanted to
    /// would have to write `Qual::Launder`, and one that wanted to drop the
    /// promise would have to stop writing `CoreTy::TaintedStr`. Both edits
    /// land here.
    ///
    /// **The promising side is six, and only one of them is a signature.**
    /// `Core\Http\Response::text` answers `tainted string` because a reply is
    /// bytes another host chose, and pinning an address settles which host they
    /// came from rather than what is in them — `rule:security/tainted-qualifier`'s roster, which
    /// `nvs_stdlib::http`'s module doc argues from. `Core\Env::get` and
    /// `Core\Env::all` are the other two, and the plainest reading of that same
    /// roster: the environment is outside the program's own text, so a value
    /// out of it is untrusted however the operator wrote it, and a variable
    /// holding a URL still has to reach `Core\Http::allowUrl`. `Core\Cli::ask`
    /// and `Core\Cli::secret` are the last two and the same reading again —
    /// `rule:tooling/a-prompt-is-a-core-member`'s
    /// prompts answer what a person typed at a terminal, which is outside the
    /// program exactly as a request body is, and `secret`'s answer carries the
    /// other axis as well because a password is confidential *and* untrusted.
    /// `Core\Cli::arguments` is the seventh and the same reading a third time:
    /// spec § 15's raw word list came off a command line somebody else wrote,
    /// so a path or a URL built out of one passes its own launderer first, and
    /// the qualifier sits on the *element* because the array itself is the
    /// program's own. `Core\IO::stdin` is the eighth, and the one member of its
    /// class whose answer is marked: every other read in `Core\IO` names a path
    /// the program chose, while standard input is a descriptor somebody else
    /// attached, so what comes back is as much the invoker's as a command-line
    /// word is. `Core\Request::path` is the ninth and the least surprising of
    /// them — spec § 15's opening sentence marks every value these classes
    /// answer that came from outside the process, and a request path is the
    /// canonical one. `header`, `headers` and `cookie` are the tenth, eleventh
    /// and twelfth, and the same sentence read three more times: a field line
    /// and a cookie are what a client sent. `headers` puts the qualifier two
    /// levels down, a repeated field name being a list of values under one key
    /// and neither array able to carry it. `query` is the one member of that
    /// class deliberately *absent*: it answers `mixed`, because § 9's bracket
    /// convention makes a value a `string` or a nested array and there is no
    /// tainted array to hold it, which `nvs_stdlib::request`'s module doc names
    /// as the hole it is. `body` is the thirteenth and the widest of them: the
    /// request's own bytes, undecoded, which is the least trusted thing any of
    /// these classes hands back and the one a program is most likely to want to
    /// parse — so the mark is what stands between it and a grammar sink. The
    /// fourteenth, fifteenth and sixteenth are `Core\Request\Part`'s three
    /// readers, and they are the same sentence read once more over a
    /// `multipart/form-data` body:
    /// `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`
    /// marks `filename` and `contentType`, and `name` is marked with them
    /// because a peer chooses the field name it sends back as freely as it
    /// chooses the other two — `nvs_stdlib::request`'s `PART` doc owns why the
    /// spec's two marks became three. `filename` is the one of the sixteen with
    /// a named launderer standing in front of it: it is a claim about a file on
    /// someone else's machine, and `Core\IO::within` is what turns one into a
    /// path here.
    /// The eighteenth and nineteenth are `Core\Router\Match`'s two capture
    /// readers, and they are the only rows here whose answer is a *union*: a
    /// capture is the segment text where the route declared `string` and the
    /// number the match already converted where it declared `int` or `uint`
    /// (`rule:routing/a-capture-narrows-to-a-closed-set`
    /// ), so the mark sits on the one arm that can carry an injection.
    /// `name()` is deliberately not among them, which is the same test read the
    /// other way — a route's declared name is the unit's own literal. The
    /// twentieth is `Core\Request\Mount::captures`, `rule:routing/a-request-reads-its-mount`'s glob
    /// captures of the mount serving the request, and its sibling `prefix()`
    /// is the same pairing read the other way again: every mount row was
    /// expanded against the disk at boot, so the prefix is the operator's text
    /// while which row a request selects is the peer's choice. The
    /// twenty-first and twenty-second are `Core\Socket\Message`'s two payload
    /// readers, and they are spec § 15's sentence read once more one layer
    /// out: a WebSocket frame is untrusted input off a network exactly as a
    /// request body is, which
    /// `rule:concurrency/a-connection-is-a-loop`
    /// states outright. They are the only rows here whose answer is
    /// *nullable*, because a message carries one payload kind and answers
    /// `null` for the other; the mark is on the arm that can carry one. Their
    /// siblings `topic()` and `value()` are deliberately absent, and that is
    /// the same test read the other way twice — a topic name is the one this
    /// connection subscribed under, and a delivery's value crossed an isolate
    /// boundary carrying whatever qualifiers it already had, which this row
    /// may not add to.
    /// The twenty-third, twenty-fourth and twenty-fifth are `Core\Request`'s
    /// three peer facts — `clientIp`, `scheme` and `host` — and they are the
    /// roster read from the far end of the wire: what a hop asserted about
    /// where a request came from is peer input, and passing
    /// `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s
    /// trust check makes it believable rather than this process's own fact.
    /// Two of the three answer a union for a reason none of the rows above
    /// share: a request can arrive with no address to report and can name no
    /// authority, so the mark sits on the arm that carries a value and the
    /// `null` says the fact is missing rather than empty.
    /// The twenty-sixth is `Core\Net\Stream::read`, and it is the first row here
    /// the program opened the door for itself: every other entry is something
    /// the runtime handed it. That changes nothing — a peer on the far end of a
    /// socket this program dialled is exactly as much somebody else as one that
    /// dialled in, and `Core\Request::body` is the same octets one layer up.
    /// It answers `bytes` rather than `string` because a socket carries octets
    /// and `rule:types/bytes` makes `string` valid UTF-8 by construction.
    /// After it comes `Core\Net\Datagram\Message::payload`, which is that row
    /// over the transport that has no connection: octets off a datagram socket
    /// are if anything less accountable, since nothing established who the
    /// sender was before they arrived. Its two siblings `host()` and `port()`
    /// are deliberately absent, and that absence is a decision rather than an
    /// omission — `Core\Net\Datagram::send`'s host is a sink and there is no
    /// launderer for an address, so a qualified answer there would be a
    /// datagram socket that could not reply. What guards a reply instead is the
    /// grant `send` asks of every address it is handed
    /// (`rule:security/net-address-policy`), and `nvs_stdlib::net`'s module doc
    /// is the home of that reading.
    /// The newest two are `Core\Zip`'s, and they are the roster read over a
    /// *container* rather than over a wire: an archive's entry names and an
    /// entry's octets were written by whoever built the archive, so both are
    /// qualified whatever the archive's own type was — which is the reading
    /// `Core\Jwt::verify`'s claims get, since a literal archive in a test is no
    /// safer than a downloaded one. `Core\Mime::detect` is deliberately absent
    /// beside them, and that is the same test read the other way: it answers a
    /// case of a closed enum, and a case carries no octet of its subject.
    /// The newest four are `Core\Xml\Node`'s — `name`, `text`, `attributes` and
    /// `source` — and they are the roster read over a *document*: what a
    /// parse answers is untrusted whatever the text handed to `Core\Xml::parse`
    /// was, so the mark is unconditional rather than contagious, and that is
    /// why the parameter is `Qual::Neutral` and not `Qual::Contagious`.
    /// `source()` is the same reading over a whole subtree: it is built out of
    /// the other three and writing them back out as one document launders none
    /// of them. Their siblings `kind()` and `children()` are deliberately
    /// absent, and that is the same test read the other way twice: a case of a
    /// closed enum carries no character of its document, and a child is another
    /// node whose own members are already here.
    /// `Core\Jwe::decrypt` is the roster read over a payload only the holder of
    /// a key can see: decrypting proves who wrote it and never that it is safe
    /// for a sink, which is `Core\Jwt::verify`'s reading over confidentiality
    /// rather than over a signature. `Core\SignedCookie::open` is the one
    /// deliberate absence from that pair and is a launderer instead, because a
    /// payload this program sealed itself was already plain when it went in.
    /// All of them belong in this set for the reason the claims do: a member that
    /// promises `tainted` is invisible from every row but its own, so this is
    /// where a new arrival has to be looked at rather than waved through.
    ///
    /// **One tainted answer is deliberately not here, and it is the shape this
    /// roster cannot see.** `Core\Request::bodyStream` answers an instance, and
    /// the mark is on what a `foreach` over it *binds* — written in
    /// `nvs_stdlib::registry::ITERABLES` rather than in a return type, exactly
    /// as `Core\Jwt::verify`'s is written on an array's element. So a member
    /// answering an `Iterable` whose element came from outside has two places to
    /// be looked at, and this one only covers the first.
    #[test]
    fn a_verified_signature_does_not_launder_its_claims() {
        use std::collections::BTreeSet;

        let mut interner = TypeInterner::new();
        let mut promises: BTreeSet<(&'static str, &'static str, String)> = BTreeSet::new();
        let mut launderers: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
        for class in CLASSES {
            for method in class.members() {
                let sig = method_sig(method, true, &mut interner);
                let answer = interner.describe(sig.return_ty);
                if answer.contains("tainted") {
                    promises.insert((class.name, method.name, answer));
                }
                if sig
                    .param_quals
                    .iter()
                    .flatten()
                    .any(|qual| matches!(qual, Qual::Launder))
                {
                    launderers.insert((class.name, method.name));
                }
            }
        }

        assert_eq!(
            promises,
            BTreeSet::from([
                (r"Core\Cli", "arguments", "array<tainted string>".to_owned(),),
                (r"Core\Cli", "ask", "tainted string".to_owned()),
                (r"Core\Cli", "secret", "secret tainted string".to_owned(),),
                (r"Core\Env", "all", "array<tainted string>".to_owned()),
                (r"Core\Env", "get", "null|tainted string".to_owned()),
                (r"Core\Http\Response", "text", "tainted string".to_owned(),),
                (r"Core\IO", "stdin", "tainted string".to_owned()),
                (r"Core\Jwe", "decrypt", "tainted string".to_owned()),
                (r"Core\Jwt", "verify", "array<tainted string>".to_owned()),
                (
                    r"Core\Net\Datagram\Message",
                    "payload",
                    "tainted bytes".to_owned(),
                ),
                (r"Core\Net\Stream", "read", "tainted bytes".to_owned()),
                (r"Core\Request", "body", "tainted string".to_owned()),
                (
                    r"Core\Request",
                    "clientIp",
                    "null|tainted string".to_owned(),
                ),
                (r"Core\Request", "cookie", "null|tainted string".to_owned(),),
                (r"Core\Request", "header", "null|tainted string".to_owned(),),
                (
                    r"Core\Request",
                    "headers",
                    "array<array<tainted string>>".to_owned(),
                ),
                (r"Core\Request", "host", "null|tainted string".to_owned(),),
                (r"Core\Request", "path", "tainted string".to_owned()),
                (r"Core\Request", "scheme", "tainted string".to_owned()),
                (
                    r"Core\Request\Mount",
                    "captures",
                    "array<tainted string>".to_owned(),
                ),
                (
                    r"Core\Request\Part",
                    "contentType",
                    "tainted string".to_owned(),
                ),
                (
                    r"Core\Request\Part",
                    "filename",
                    "tainted string".to_owned(),
                ),
                (r"Core\Request\Part", "name", "tainted string".to_owned()),
                (r"Core\Request\Part", "readAll", "tainted bytes".to_owned(),),
                (
                    r"Core\Router\Match",
                    "param",
                    "uint|int|null|decimal|tainted string|Core\\Uuid|Parses".to_owned(),
                ),
                (
                    r"Core\Router\Match",
                    "params",
                    "array<uint|int|decimal|tainted string|Core\\Uuid|Parses>".to_owned(),
                ),
                (
                    r"Core\Signature",
                    "verify",
                    "array<tainted string>".to_owned(),
                ),
                (
                    r"Core\Socket\Message",
                    "bytes",
                    "null|tainted bytes".to_owned(),
                ),
                (
                    r"Core\Socket\Message",
                    "text",
                    "null|tainted string".to_owned(),
                ),
                (
                    r"Core\Xml\Node",
                    "attributes",
                    "array<tainted string>".to_owned(),
                ),
                (r"Core\Xml\Node", "name", "tainted string".to_owned()),
                (r"Core\Xml\Node", "source", "tainted string".to_owned()),
                (r"Core\Xml\Node", "text", "tainted string".to_owned()),
                (r"Core\Zip", "entries", "array<tainted string>".to_owned(),),
                (r"Core\Zip", "read", "tainted bytes".to_owned()),
            ]),
            "the roster of members whose *answer* is qualified `tainted` is closed — a \
             verified claim, a verified signature's payload, a decrypted payload, an \
             outbound reply's body, \
             the two environment \
             reads, the two prompts that answer what a person typed, the words the program \
             was started with, everything attached to its standard input, the five reads of \
             the request being answered, the three facts it arrived on, the three \
             declarations one of its uploaded parts made, the bytes of that part held \
             whole, the two readers of the captures the matched route filled, the captures \
             of the mount serving the request, the two payloads a connection's peer \
             sent, what a socket the program opened itself read back, what one \
             datagram carried, the names and octets read out of an archive, and \
             the four strings a node of a parsed document answers with. \
             `content()` is not one of them and is not a gap: \
             its answer is a walk, and the `tainted bytes` is on the element `Iterable<T>` \
             yields. Where the answer is a \
             collection the \
             element type is what carries it, since `nvs_types` has no tainted array and a member \
             answering `array<mixed>` would have laundered every entry silently"
        );

        assert!(
            !launderers.contains(&(r"Core\Jwt", "verify")),
            "a JWT verification removes no qualifier from anything: the token's issuer is not \
             this application, and even a self-issued token routinely carries user input"
        );
        assert!(
            !launderers.contains(&(r"Core\Signature", "verify")),
            "a detached signature removes no qualifier either, and for a sharper reason than the \
             token above: its payload is meant to be read by whoever holds it, so the value a \
             program gets back is the value someone else put in"
        );
        assert!(
            launderers.contains(&(r"Core\SignedCookie", "open")),
            "the cookie is § 5's one admitted exception — the value round-trips through our own \
             AEAD unchanged, and it was already plain when it went in"
        );
    }

    /// `rule:security/tainted-qualifier` asked as the **converse** of
    /// `a_verified_signature_does_not_launder_its_claims`, over the one class
    /// tree every request reads itself through. That test pins which members
    /// across the whole registry *promise* `tainted`, and a `Core\Request` row
    /// passes it by not being in the set at all. This one asks what is left
    /// over: every member of `Core\Request` and of the classes under it,
    /// partitioned by **how** its answer carries the peer's bytes, with the
    /// partition asserted rather than the marks.
    ///
    /// Three buckets carry the marked rows and the fourth is the one that
    /// does the work. A row answers qualified; or it answers a walk whose
    /// element is qualified, which is exactly the place the sibling test's own
    /// doc says it cannot see, since an `Iterable<T>`'s `T` is written in
    /// `nvs_stdlib::registry::ITERABLES` and not in the return type; or it
    /// answers a walk over another class in this same sweep, which hands the
    /// question on to that class's own rows rather than answering it. What is
    /// left is `plain`, and `plain` is what the exact-set assertion is for: a
    /// member added tomorrow that answers a bare `string` off the wire lands
    /// there and nowhere else, while a test naming the *marked* rows would
    /// never look at it.
    ///
    /// The sweep is taken by **prefix** rather than from a list of four class
    /// names, so a new class under the request — a trailer bag, a second body
    /// shape — joins it without anyone remembering to add it.
    ///
    /// **`query`, `post` and `json` are the plain rows that answer outside
    /// data with nowhere to put the mark**, and they are the hole this
    /// repository has already
    /// written down rather than an oversight: spec § 9's bracket convention
    /// makes a value a `string` or a nested `array`, a decoded JSON document is
    /// that same array, `nvs_types` has no tainted array to hold either, so all
    /// three rows answer `mixed` and the mark has nowhere to sit —
    /// `nvs_stdlib::request`'s module doc owns why. They are one hole and not
    /// three: `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename` gives a submitted form the parse a query string gets,
    /// so a tainted array would close all of them in the same edit. Naming them
    /// here is what makes closing it an edit to this assertion instead of a
    /// test that stays green across the fix. The other nine plain rows are not
    /// outside data at all: `method` has narrowed to a closed enum before
    /// anything can hold a payload, `isHead` is one bit derived from it,
    /// `saveTo` answers `void` because its bytes went to a file rather than to
    /// the caller, and `route` answers an instance of a class *outside* this
    /// prefix — `Core\Router\Match`, whose two capture readers are marked and
    /// are checked by the sibling test above. That last one is the third shape
    /// this sweep cannot see on its own, beside the two the buckets handle: a
    /// row handing the question to a class the prefix does not reach. `mount`
    /// is the same shape read the *inside* way: it answers
    /// `Core\Request\Mount`, which the prefix does reach, so that class's own
    /// two rows are swept here — the marked `captures()` and the sixth plain
    /// row, `prefix()`, which is plain because a mount row was expanded against
    /// the disk at boot rather than derived from the URL
    /// (`rule:http-server/a-path-is-never-derived-from-a-url`
    /// ).
    ///
    /// **`jsonAs`, `queryAs` and `postAs` are the fourth shape, and the only
    /// rows whose type is not `Core`'s at all.** Each answers the `T` its call
    /// site wrote, so there is no return type for the mark to sit on and no row
    /// in any sweep to check: `rule:security/derived-codec-qualifiers` puts the
    /// question on that type's own declared fields, at the call site that
    /// decodes. What they hydrate is outside data like `json()`'s document, and
    /// the three are plain here for a different reason — not that the mark has
    /// nowhere to go, but that where it goes is a declaration this compiler
    /// never wrote.
    #[test]
    fn every_request_member_returning_outside_data_returns_it_tainted() {
        use nvs_stdlib::registry::iterable_element;
        use std::collections::BTreeSet;

        let mut interner = TypeInterner::new();
        let mut marked: BTreeSet<(&'static str, &'static str, String)> = BTreeSet::new();
        let mut walks_marked_elements: BTreeSet<(&'static str, &'static str, String)> =
            BTreeSet::new();
        let mut hands_on: BTreeSet<(&'static str, &'static str, String)> = BTreeSet::new();
        let mut plain: BTreeSet<(&'static str, &'static str, String)> = BTreeSet::new();
        for class in CLASSES.iter().filter(|class| {
            class.name == r"Core\Request" || class.name.starts_with(r"Core\Request\")
        }) {
            for method in class.members() {
                let sig = method_sig(method, true, &mut interner);
                let answer = interner.describe(sig.return_ty);
                if answer.contains("tainted") {
                    marked.insert((class.name, method.name, answer));
                    continue;
                }
                let element = match &method.return_ty {
                    CoreTy::Instance(name) => iterable_element(name),
                    _ => None,
                };
                if let Some(element) = element {
                    let described = {
                        let id = lower(element, &mut interner);
                        interner.describe(id)
                    };
                    if described.contains("tainted") {
                        walks_marked_elements.insert((class.name, method.name, described));
                    } else {
                        hands_on.insert((class.name, method.name, described));
                    }
                    continue;
                }
                plain.insert((class.name, method.name, answer));
            }
        }

        assert_eq!(
            plain,
            BTreeSet::from([
                (r"Core\Request", "isHead", "bool".to_owned()),
                (r"Core\Request", "method", r"Core\Http\Method".to_owned()),
                (r"Core\Request", "json", "mixed".to_owned()),
                (r"Core\Request", "jsonAs", "T".to_owned()),
                (r"Core\Request", "post", "mixed".to_owned()),
                (r"Core\Request", "postAs", "T".to_owned()),
                (r"Core\Request", "query", "mixed".to_owned()),
                (r"Core\Request", "queryAs", "T".to_owned()),
                (
                    r"Core\Request",
                    "route",
                    r"null|Core\Router\Match".to_owned(),
                ),
                (r"Core\Request", "mount", r"Core\Request\Mount".to_owned(),),
                (r"Core\Request\Mount", "prefix", "string".to_owned()),
                (r"Core\Request\Part", "saveTo", "void".to_owned()),
            ]),
            "the request tree's rows that answer an unqualified value are closed at twelve, and \
             nine of them have the mark somewhere other than the return type: a closed method \
             enum, the bit derived from it, the `void` of bytes that went to a file, the matched \
             route, whose own class carries the mark on the captures it hands back, the mount \
             serving the request, whose class does the same, and that class's own `prefix()` — a \
             mount row is expanded against the disk at boot (`rule:http-server/a-path-is-never-derived-from-a-url`), so a prefix is the \
             operator's text and not the peer's — and `jsonAs`, `queryAs` and `postAs`, whose `T` \
             is a class or shape the call site wrote, so \
             `rule:security/derived-codec-qualifiers` asks for the mark on that type's \
             own declared fields. `query`, `post` and `json` are the \
             other three and are one known hole — § 9's brackets make a value a `string` or a \
             nested array, a decoded document is the same array, and there is no tainted array, so \
             a member added here answering a bare `string` off the wire joins this set and is the \
             thing it exists to catch"
        );
        assert_eq!(
            walks_marked_elements,
            BTreeSet::from([
                (r"Core\Request", "bodyStream", "tainted bytes".to_owned()),
                (r"Core\Request\Part", "content", "tainted bytes".to_owned()),
            ]),
            "the two members whose mark is on what a `foreach` binds rather than on the return \
             type — the second place a qualifier can live, which the roster over return types \
             alone cannot see"
        );
        assert_eq!(
            hands_on,
            BTreeSet::from([(r"Core\Request", "files", r"Core\Request\Part".to_owned())]),
            "`files()` answers neither bytes nor a marked element: it walks a class this same \
             sweep covers, so `rule:http-server/an-upload-is-received-only-through-files`'s one way in is checked by the `Core\\Request\\Part` \
             rows above rather than by its own return type"
        );
        assert!(
            marked.len() >= 9,
            "the marked rows are the sibling test's closed set and are only counted here, to \
             keep this partition total: {marked:?}"
        );
    }

    /// `rule:security/outbound-url-is-a-sink`, asked as a **closed set** rather than of one row: across
    /// the whole registry, the only parameter that admits a `tainted` URL is
    /// the launderer's.
    ///
    /// § 1 makes the URL parameter of every outbound member a sink, so a
    /// `tainted` operand is a diagnostic there exactly as at `Core\Db`'s query
    /// text. Asserted over every `$url` in `CLASSES` because that is the shape
    /// the rule actually has: a *second* member accepting an outbound URL is
    /// the thing this exists to catch, and it is invisible from any one row.
    /// A client member added tomorrow with `CoreTy::Text(Qual::Contagious)` at
    /// its URL — the spelling that reads most natural, since the URL is what
    /// the request is made of — fails here rather than at a review.
    ///
    /// The `names` column is what identifies the parameter, and `rule:core-api/shape-rules` R2
    /// makes that the spec's own signature column rather than a convention
    /// this test invented.
    #[test]
    fn an_outbound_url_parameter_refuses_a_tainted_operand() {
        use std::collections::BTreeSet;

        let mut interner = TypeInterner::new();
        let mut admitting: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
        let mut seen: BTreeSet<(&'static str, &'static str)> = BTreeSet::new();
        for class in CLASSES {
            for method in class.members() {
                let Some(at) = method.names.iter().position(|name| *name == "url") else {
                    continue;
                };
                seen.insert((class.name, method.name));
                let sig = method_sig(method, true, &mut interner);
                if crate::expr::quals::admits_tainted_argument(
                    sig.qual_at(at),
                    sig.return_ty,
                    &mut interner,
                ) {
                    admitting.insert((class.name, method.name));
                }
            }
        }

        assert!(
            seen.contains(&(r"Core\Http", "allowUrl")),
            "the roster is read off the `names` column, so a member spelling its URL parameter \
             something else would empty this test rather than fail it"
        );
        assert_eq!(
            admitting,
            BTreeSet::from([(r"Core\Http", "allowUrl")]),
            "an outbound URL is a sink (`rule:security/outbound-url-is-a-sink`) and `Core\\Http::allowUrl` is the one door \
             through it (§ 2) — a second member admitting a tainted URL is a second door, and the \
             policy is only worth what the narrowest of them enforces"
        );
    }

    /// `rule:http-server/allow-url-pins-the-address`'s load-bearing half: the launderer's answer is a **value**,
    /// not a laundered `string`.
    ///
    /// A `Qual::Launder` that returned `CoreTy::Str` would satisfy `rule:security/launderers-are-sink-named`
    /// completely and still be wrong here, because what it hands back is as
    /// resolvable as what went in — the check happens at one moment and the
    /// connection at another, and a second DNS answer in between is the
    /// rebinding attack. So the two halves are asserted together: the
    /// qualifier comes off, *and* what comes back is `Core\Http\Target`.
    ///
    /// The target's own emptiness is the third half. It has no members at all,
    /// so a program cannot read the approved address back out and rebuild a
    /// request around a different one; `nvs_stdlib::http`'s module doc is the
    /// home of why that is deliberate rather than unfinished.
    #[test]
    fn allow_url_pins_what_it_launders() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (owner, sig) = resolve_method(
            &QName::parse(r"Core\Http"),
            "allowUrl",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Http::allowUrl is registered");
        assert_eq!(owner.to_string(), r"Core\Http");
        assert_eq!(sig.params.len(), 1);

        assert_eq!(
            sig.qual_at(0),
            Some(Qual::Launder),
            "`rule:security/launderers-are-sink-named`'s narrow, sink-named launderer — this one for the outbound sink"
        );
        assert!(
            crate::expr::quals::admits_tainted_argument(
                sig.qual_at(0),
                sig.return_ty,
                &mut interner
            ),
            "and it is the one outbound parameter that takes a tainted operand, since a URL that \
             needed no laundering would not be at this door"
        );

        let answer = interner.describe(sig.return_ty);
        assert_eq!(
            answer, r"Core\Http\Target",
            "the answer pins: it carries the address that was approved, so the connection is made \
             to that address and no second resolution can answer differently"
        );
        assert!(
            !answer.contains("string"),
            "a `string` answer would have removed the qualifier and left the rebinding gap open, \
             which is the failure `rule:http-server/allow-url-pins-the-address` exists to close"
        );

        let target = CLASSES
            .iter()
            .find(|class| class.name == r"Core\Http\Target")
            .expect("the launderer's answer is a registered class");
        assert!(
            target.members().next().is_none(),
            "and it exposes nothing: a member answering the pinned address would let a program \
             rebuild the request around a different one"
        );
        assert_eq!(target.slots, &["url", "address"]);
    }

    /// `rule:http-server/no-spelling-for-an-unbounded-wait`, asked of the lowered signature rather than of the row:
    /// **no member of `Core\Http\Client` can be told to wait forever.**
    ///
    /// The guarantee comes from the absence of a spelling, so what this asserts
    /// is that nothing has grown one. Every time bound lowers to exactly
    /// `Core\Time\Duration` — a `?Duration` would describe as
    /// `null|Core\Time\Duration` and fail here, and it is the one edit that
    /// would make `{deadline: null}` compile — and every bound is left out by
    /// omitting it, which § 5 makes inherit `[http.client]`'s own figure rather
    /// than remove the bound. A default of `0` or `-1` would be the unbounded
    /// spelling arriving as a *default*, which no call site would show.
    ///
    /// Asked over every member and every option rather than of `deadline`
    /// alone, because the rule is about the class: a sixth row added tomorrow
    /// with a bound of its own is exactly what this exists to catch, and it is
    /// invisible from any one row.
    #[test]
    fn no_client_member_accepts_an_unbounded_timeout() {
        use nvs_stdlib::registry::Const;

        const BOUNDS: &[&str] = &["deadline", "connectTimeout", "retryBackoff"];

        let mut interner = TypeInterner::new();
        let client = CLASSES
            .iter()
            .find(|class| class.name == r"Core\Http\Client")
            .expect("the client is a registered class");
        assert!(
            client.members().next().is_some(),
            "a class with no members would pass every assertion below vacuously"
        );

        let mut seen: Vec<&'static str> = Vec::new();
        for method in client.members() {
            let Some(CoreTy::Options(options)) = method.params.last() else {
                panic!("{}::{} has no options bag", client.name, method.name);
            };
            for option in *options {
                assert!(
                    !matches!(option.ty, CoreTy::Nullable(_)),
                    "{}::{}'s `{}` is nullable, and `null` at a bound is the spelling § 5 has \
                     none of",
                    client.name,
                    method.name,
                    option.name
                );
                if !BOUNDS.contains(&option.name) {
                    continue;
                }
                seen.push(option.name);
                let bound = lower(&option.ty, &mut interner);
                assert_eq!(
                    interner.describe(bound),
                    r"Core\Time\Duration",
                    "{}::{}'s `{}` is not a plain `Duration`",
                    client.name,
                    method.name,
                    option.name
                );
                assert!(
                    matches!(option.default, Const::Null),
                    "{}::{}'s `{}` defaults to something other than *not given*, so a call \
                     that omits it would carry a bound this test cannot see",
                    client.name,
                    method.name,
                    option.name
                );
            }
        }

        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen,
            {
                let mut want = BOUNDS.to_vec();
                want.sort_unstable();
                want
            },
            "every bound § 5 names is one this test read; a renamed one would empty the check \
             rather than fail it"
        );
    }

    /// `rule:core-classes/process-run`, asked of the resolved signature the way a call site asks
    /// it: **neither half** of what `Core\Process::run` is handed can carry a
    /// tainted value into the child, and the two halves are refused by
    /// different mechanisms. `$path` is a [`Qual::Sink`], so a
    /// `tainted string` is refused at the parameter. `$argv`'s elements carry
    /// no mark of their own and need none — [`qual_of`] reads no nested
    /// classification — because `array<tainted string>` is simply a different
    /// type from the `array<string>` the row declares, which the ordinary
    /// argument check refuses without a qualifier rule at all.
    ///
    /// The second half is the one worth a test: a reader who knows only that
    /// the elements are unclassified would conclude the argv is the hole, and
    /// what closes it is the interner rather than [`Qual`].
    #[test]
    fn a_tainted_path_or_argv_element_is_a_compile_time_diagnostic() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (owner, sig) = resolve_method(
            &QName::parse(r"Core\Process"),
            "run",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Process::run is registered");
        assert_eq!(owner.to_string(), r"Core\Process");
        assert_eq!(sig.params.len(), 2);

        assert_eq!(
            sig.qual_at(0),
            Some(Qual::Sink),
            "a program's name is an instruction, so the path is a sink — `rule:core-classes/process-run`"
        );
        assert!(
            !crate::expr::quals::admits_tainted_argument(
                sig.qual_at(0),
                sig.return_ty,
                &mut interner
            ),
            "and a sink admits no tainted argument, which is the diagnostic"
        );

        assert_eq!(
            interner.describe(sig.params[1]),
            "array<string>",
            "the argv is an array of plain strings"
        );
        assert_eq!(
            sig.qual_at(1),
            None,
            "an array parameter has no cell to classify, and needs none here"
        );
        let tainted = interner.tainted_string();
        let tainted_argv = interner.array(tainted);
        assert_ne!(
            sig.params[1], tainted_argv,
            "`array<tainted string>` is not the declared `array<string>`, so an argv built out of \
             request data is refused by the argument check rather than by a qualifier rule"
        );
    }

    /// `rule:security/process-exec-capability`, held at the row rather than at the door: starting a
    /// program is a capability, it is the one `nvs.toml` spells `process.exec`,
    /// a host that configured nothing has granted it to nobody, and a request
    /// cannot widen it because the whole `capabilities` block is
    /// [`nvs_config::Class::RuntimeTighten`].
    ///
    /// Enforcement is `nvs_runtime::capability::require`, inside the door — the
    /// table this reads is audit data and grants nothing, which
    /// `nvs_stdlib::registry::CAPABILITIES`' own docs state. What is asserted
    /// here is that the member is *on* it under the right capability, since a
    /// member reaching the operating system with no row is the shape that would
    /// go unreported.
    #[test]
    fn process_exec_is_deny_by_default() {
        let (_, _, cap) = *nvs_stdlib::registry::CAPABILITIES
            .iter()
            .find(|(class, member, _)| *class == r"Core\Process" && *member == "run")
            .expect("Core\\Process::run starts a program, so it carries a capability row");
        // `Some`, not just any row: § 7's table now also carries `None` for a
        // member of a door-bearing class that reaches nothing, so "is on the
        // table" stopped being the same claim as "is declared as an effect".
        let cap = cap.expect("starting a program is an effect, so its row names a capability");
        assert_eq!(cap, nvs_config::Cap::ProcessExec);
        assert_eq!(
            nvs_config::Cap::parse("process.exec"),
            Some(nvs_config::Cap::ProcessExec),
            "and that is the spelling an operator writes in `nvs.toml`"
        );
        assert!(
            cap.grant(&nvs_config::tree::Capabilities::default())
                .is_none(),
            "a host that configured no capabilities has granted this one to nobody: deny by \
             default is the absence of a setting, not a setting that says no"
        );

        let block = nvs_config::DIRECTIVES
            .iter()
            .find(|directive| directive.key == "capabilities")
            .expect("the capability block is a directive, so a reload knows what it takes");
        assert_eq!(
            block.class,
            nvs_config::Class::RuntimeTighten,
            "and a request may narrow the grant it was started under, never widen it"
        );
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

    /// `rule:expressions/try-parse`'s pair seeded as `Parses`, on the terms
    /// `Comparable` set: a `Core` class writes no `implements` clause, so
    /// `nvs_stdlib::registry::implements_parses` reads the rows and the edge is
    /// written here. What a binding site asks for is this edge and nothing
    /// else, so a class the predicate stops answering for stops being nameable
    /// there — which is why both halves are asserted.
    #[test]
    fn a_core_class_carrying_the_whole_parse_pair_implements_parses() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = ClassGraph::default();

        assert_eq!(
            crate::signatures::resolve_interface_args(
                &QName::parse(r"Core\Uuid"),
                &QName::parse(PARSES),
                &table,
                &graph
            ),
            Some(Vec::new()),
            "`Core\\Uuid` carries the pair, and `Parses` takes no type parameters"
        );
        assert!(
            crate::signatures::resolve_interface_args(
                &QName::parse(r"Core\Uri"),
                &QName::parse(PARSES),
                &table,
                &graph
            )
            .is_none(),
            "`Core\\Uri` carries a `parse`/`tryParse` pair that refuses a `tainted` argument, so \
             it is not an implementor — `nvs_stdlib::registry::implements_parses` owns why"
        );
    }

    /// [`CoreTy::Iterated`] is exactly `rule:iteration/foreach-subjects`'s three shapes, and the
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

    /// `Core\Task::map`'s callback is a written signature like every other
    /// one, so what the member hands it — the value and the key — reads back
    /// off the seeded row rather than out of a variable name beside an opaque
    /// `callable`.
    #[test]
    fn a_callback_parameter_lowers_to_the_signature_the_row_writes() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (_, sig) = resolve_method(
            &QName::parse(r"Core\Task"),
            "map",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Task::map is registered");
        assert_eq!(sig.params.len(), 3);
        assert_eq!(interner.describe(sig.params[0]), "array<T>");
        assert_eq!(
            interner.describe(sig.params[1]),
            "callable(T, string): U",
            "`rule:types/callable-signature`: the row says what the callback \
             receives, so the checker can place a literal against it"
        );
        assert_eq!(interner.describe(sig.return_ty), "array<U>");
        // Distinct from a bare `callable` all the same: that one is the top of
        // the lattice and constrains nothing.
        let plain = interner.callable();
        assert_ne!(sig.params[1], plain);
    }

    /// A written callback signature lowers to the interned type it spells, at
    /// the member's own variables — and substitutes field-wise rather than
    /// collapsing, which is the whole of what
    /// `rule:types/callable-literal-inference` needs from a `Core` row: the
    /// expected type an unannotated `fn` parameter reads is this one with the
    /// call's bindings already applied.
    #[test]
    fn a_written_callback_signature_lowers_to_the_signature_it_spells() {
        const SIG: CoreTy =
            CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Var("U"));

        let mut interner = TypeInterner::new();
        let lowered = lower(&SIG, &mut interner);
        assert_eq!(interner.describe(lowered), "callable(T, string): U");

        // The id a source-written signature interns to, not a registry-only
        // shape beside it — the same property `?T` has above.
        let var_t = interner.type_var("T");
        let string = interner.string();
        let var_u = interner.type_var("U");
        let written = interner.callable_sig(vec![var_t, string], var_u);
        assert_eq!(lowered, written);

        // `T` becomes what the call bound it to and the unbound `U` becomes
        // `mixed`, exactly as they would anywhere else in the signature.
        let user = interner.class(QName::parse(r"App\User"));
        let mut bindings = crate::generics::Bindings::default();
        bindings.insert("T".to_owned(), user);
        let substituted = crate::generics::substitute(lowered, &bindings, &mut interner);
        let mixed = interner.mixed();
        let expected = interner.callable_sig(vec![user, string], mixed);
        assert_eq!(substituted, expected);
    }

    /// `CoreTy::Nullable` is `null|T` and *is* the union the checker already
    /// had — the one property the whole `?T` half of `rule:expressions/nullable-conversion` rests on, since
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
        let expected = interner.options(vec![("version".to_owned(), version, None)]);
        assert_eq!(sig.params[1], expected);
        // A literal is its own type, not the `int` it erases to — the whole
        // point of `rule:types/literal-types` at this position.
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
