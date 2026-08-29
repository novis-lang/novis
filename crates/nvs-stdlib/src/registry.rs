//! The `Core` signature registry: what the compiler resolves a
//! `Core\Class::member(...)` call against.
//!
//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md) is
//! authoritative for every signature; this is that file in the one form a
//! compiler can read. A row here is a *promise* — `nvs-types` seeds its own
//! signature table from [`CLASSES`], so a `Core` call goes through exactly the
//! arity check, the assignability check and the `ResolvedCall` recording a
//! user-declared static call already does, with no second code path.
//!
//! # Why a small type enum rather than a type spelling
//!
//! [`CoreTy`] is a closed enum, not a `&'static str` the compiler re-parses.
//! Two reasons, both structural: a spelling would put a second (partial)
//! parser for Novis's type grammar in the build, and a typo in it would be a
//! *runtime* surprise in `nvs check` rather than a compile error here. The
//! cost is that widening the registry to a type this enum cannot express —
//! a union, a nullable, a shape — is a change to this file rather than a
//! string edit, which is the correct amount of friction for something the
//! whole language resolves against.
//!
//! # Known gap
//!
//! The enum covers exactly what the members registered so far need, and
//! [`CoreClass`] covers exactly the *kind* of member they are — [`crate`]'s
//! own gap 3 owns what is left, and it is now a matter of *named* arguments
//! rather than shapes. A **variadic** parameter is no longer on it: it is
//! [`CoreTy::Variadic`], read through [`CoreMethod::variadic`]. A class
//! **constant** is no longer on it — it is
//! [`CoreConst`], a roster on [`CoreClass`] rather than a [`CoreTy`] variant,
//! since a constant has a value and no signature. Nor is a `Core`-owned
//! **instance**: it is [`CoreTy::Instance`] plus [`CoreClass::instance`] and
//! [`CoreClass::slots`], and [`crate::instance`] is the value behind it.
//!
//! # A `Core` enum is declared here too
//!
//! The spec's own tables name enums as well as members — `Core\Order` at
//! § 2's *Ordering* is the first — so [`ENUMS`] is a second roster beside
//! [`CLASSES`], and [`CoreTy::Enum`] refers to one by name. Its own doc
//! comment owns why the two are separate; what belongs here is that
//! `nvs_types::enums` seeds them into the *same* table a declared `enum` goes
//! into, so nothing downstream of that point can tell the two apart.
//!
//! # The options bag
//!
//! [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R2 makes a
//! trailing options shape (`{step?: int}`) the form of *every* optioned
//! member. It is [`CoreTy::Options`], and the four properties below are what
//! it costs and what it buys — recorded here because the fork is the
//! expensive part, not the code:
//!
//! * **A bag is its own type, not an ADR 0036 shape.** `nvs_types::ty::Ty`
//!   has an `Options` variant beside `Shape`, spellable only from here the
//!   way `TypeVar` already is. Reusing `Shape` would need an `optional` flag
//!   on its fields *and* a `?` in the surface type grammar, and would leave an
//!   unknown option accepted — ADR 0036 § 3's width subtyping allows an extra
//!   field on purpose, while a mistyped option name must be an error.
//! * **A bag is always last and always optional**, because every option is.
//!   Its `MethodSig::defaults` entry is a `ConstArg::Options(...)` carrying
//!   each option's own default, synthesized by `nvs_types::core_lib` from the
//!   type itself — so a row never states the bag twice, `MethodSig::required()`
//!   already excludes it, and the arity check needed no change at all.
//! * **A bag flattens at the ABI.** `nvs_ir::lower::lower_call_args` expands
//!   it into one argument per declared option, in the order [`CoreOption`]s
//!   are written here — the literal's value where written, the option's
//!   default where not — so `nvs_core_arr_range` is an ordinary `args: [3]`
//!   helper and no runtime representation of a shape exists. The rejected
//!   alternative was building an `array<mixed>` per call: it allocates on the
//!   common path, and needs a `null`/empty spelling the IR does not have.
//! * **The cost is one restriction:** an options argument must be written as a
//!   shape literal at the call site, or omitted — a diagnostic, never silence.
//!   That is exactly the set of programs that can run today, since
//!   `ExprKind::ObjectLiteral` has no lowering of its own at all.

/// [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// § 2's qualifier classification, declared **per parameter** on the
/// `string`/`bytes` parameter it describes.
///
/// Not to be confused with the qualifier a *value* carries: `tainted` and
/// `secret` live on `nvs_types::ty::Ty` and describe an argument. This
/// describes what a member does with one, and there are exactly four answers
/// ([the spec's *How to read an entry*](../../../../docs/spec/01-core-library.md)
/// renders them as the Q column).
///
/// **There is no default.** A parameter with no classification is spelled
/// [`CoreTy::Str`]/[`CoreTy::Bytes`] and *refuses* a qualified argument, which
/// is why [`Self::Contagious`] is a thing an author writes rather than a thing
/// an author gets by forgetting. `every_member_parameter_carries_a_qualifier_classification`
/// is what stops a member shipping unclassified at all.
///
/// # The rule a class is classified by
///
/// Written down once here rather than re-derived per class, because the
/// judgement is the same one every time and what it costs to get wrong is a
/// member that launders by accident:
///
/// * A member whose answer carries no byte of any argument — a `bool`, a
///   count, an ordering — is [`Self::Neutral`] in **every** parameter.
/// * Otherwise every `string`/`bytes` parameter is [`Self::Contagious`],
///   including one whose own bytes never appear in the answer:
///   `Core\Str::before`'s separator decides *which* slice comes back, and
///   laundering by influence is not something a member may do silently.
/// * ADR 0063 R11's four grammars — a regex pattern, a `printf` template, a
///   CLDR date pattern and a `Core\Bytes::pack` format — are [`Self::Sink`]
///   wherever they are declared, by ADR 0088 § 1's own corollary.
/// * [`Self::Launder`] is the default of nothing. A member claims it, and its
///   doc comment names the sink it launders for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qual {
    /// A qualified argument yields a qualified result — the overwhelming
    /// majority, and the blank cell in the spec's Q column.
    Contagious,
    /// This parameter's content becomes an instruction something executes, so
    /// it **refuses** a qualified argument. ADR 0088 § 1 is the predicate.
    Sink,
    /// The result never carries this argument's qualifier: a `bool`, a count,
    /// a hash of a secret.
    Neutral,
    /// This member removes the qualifier, and its doc comment names the sink
    /// it launders for — `Core\Regex::quote` launders for the pattern sink.
    Launder,
}

/// One type in a `Core` member's signature.
///
/// Deliberately smaller than `nvs_types::ty::Ty`: this describes what the
/// *spec* wrote, not what the checker interns. `nvs-types` lowers each of
/// these into its own interner, which is where qualifiers, unions and class
/// identity live.
// No `PartialEq`/`Eq`: [`Self::Options`] carries [`CoreOption`]s, which carry
// [`Const`]s, which carry an `f64` — and there is nothing here to compare
// anyway, since a registry row is matched structurally and interned into
// `nvs_types::ty::Ty` before any consumer asks whether two types are equal.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum CoreTy {
    /// `bool`
    Bool,
    /// `int`
    Int,
    /// `uint` — ADR 0007 § 4.
    Uint,
    /// `float`
    Float,
    /// `decimal` — [ADR 0054](../../../../docs/adr/0054-decimal-scalar-type.md)'s
    /// scalar, which the spec writes wherever a member is exact over money:
    /// the `int|float|decimal` unions of `Core\Math` and the subject of
    /// `Core\Arr::sum`/`product`/`average`.
    ///
    /// A helper reads one out of its argument slot with
    /// `nvs_runtime::Value::as_decimal` and returns one with
    /// `Value::decimal` — it is a whole `Value` carrying `Tag::Decimal`, so
    /// nothing about the ABI changes for it (`nvs_runtime::decimal`).
    Decimal,
    /// `string`, **unclassified** — which in parameter position is not a
    /// default but a state: ADR 0088 § 2 makes it refuse a `tainted` argument.
    /// A classified `string` parameter is [`Self::Text`]. In return position
    /// this is the only spelling, because a classification describes what a
    /// member does with an argument.
    Str,
    /// `bytes` — ADR 0009. Unclassified, exactly as [`Self::Str`] is;
    /// [`Self::Blob`] is the classified spelling.
    Bytes,
    /// A `string` parameter carrying [`Qual`], ADR 0088 § 2's classification.
    ///
    /// A leaf variant rather than a wrapper around [`Self::Str`] on purpose:
    /// every walk over a [`CoreTy`] in this crate ends its `match` with a
    /// wildcard arm documented as "a variant that carries no nested type
    /// carries no variable either", and a wrapper would have quietly falsified
    /// that in seven places at once.
    Text(Qual),
    /// A `bytes` parameter carrying its classification — [`Self::Text`]'s twin.
    Blob(Qual),
    /// `void`, return position only.
    Void,
    /// `mixed` — ADR 0007 § 3's one unchecked position.
    Mixed,
    /// `array<T>`, whose element type is the wrapped one.
    Array(&'static CoreTy),
    /// `callable` — [ADR 0031](../../../../docs/adr/0031-callable-is-the-only-closure-type.md)
    /// § 4's one closure type, and opaque: it says nothing about the
    /// parameters or the result of the closure that satisfies it. What a
    /// `Core` member actually hands a callback is stated by
    /// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
    /// § 2 — `($value, $key)`, with fewer parameters allowed — and enforced
    /// at the call by `nvs_runtime::call_closure`, not by this type.
    Callable,
    /// `callable`, plus the name of the type variable its **result** binds —
    /// `U` in `map(array<T> $a, callable $fn): array<U>`.
    ///
    /// The same opaque `callable` at the call site: it constrains nothing a
    /// [`Self::Callable`] parameter does not, and a closure value satisfies it
    /// by ADR 0027 § 2 exactly as before. What it adds is a *binding site* for
    /// a variable that appears at no argument position at all — `U` is the type
    /// of a value the callback produces, which the argument's own type
    /// (`callable`, and opaque) cannot say. `nvs_types::generics` binds it from
    /// the closure literal's recorded return type and owns the one case that
    /// still binds nothing: an argument that is not a written `fn` literal.
    ///
    /// **Parameter position only, and never nested.** It is a declaration of
    /// where a variable comes from, so it means nothing inside an
    /// [`Self::Array`], a [`Self::Union`], a [`CoreOption`] or a return type —
    /// `a_callback_result_type_is_only_ever_a_whole_parameter` holds that.
    CallableTo(&'static str),
    /// A **shape literal of zero-argument `fn` literals**, plus the name of
    /// the type variable the shape of their *results* binds — `S` in
    /// `Core\Task::all({...}): S`.
    ///
    /// [`Self::CallableTo`] one level up, and it exists for exactly that
    /// variant's reason at a wider position.
    /// [ADR 0072](../../../../docs/adr/0072-core-task-structured-concurrency.md)
    /// § 1 makes `Task::all`'s answer a shape with the argument's own field
    /// names, each field typed as *that field's* closure returns — which is the
    /// whole reason the member is worth having, since the uniform alternative
    /// answers `array<mixed>` and every call site then pays a cast. No type at
    /// this position can say that: the argument's own type is a shape of
    /// `callable`s, and ADR 0027 § 2 keeps a `callable` opaque.
    ///
    /// **The argument has to be *written* at the call site**, and each field's
    /// value has to be a written `fn` literal — `E0773` and `E0774` are the two
    /// diagnostics, and `nvs_types::expr::args` is the one place that reads a
    /// field. That is ADR 0072 § 1's own restriction rather than an
    /// implementation limit: `callable` carries no signature, so a variable has
    /// nothing to bind from, and ADR 0007 § 3's deferred typed-`callable`
    /// signatures are what would lift it.
    ///
    /// **Parameter position only, and never nested**, exactly as
    /// [`Self::CallableTo`] is, and held by the same
    /// `a_callback_result_type_is_only_ever_a_whole_parameter`.
    CallableShapeTo(&'static str),
    /// A type *variable*, named — `T` in `count(array<T> $a): uint`.
    ///
    /// The spec's `Core\Arr` section states the rule this exists for: "`T` is
    /// a type variable — the stdlib is parametric where user code is not." A
    /// variable is bound by unifying the declared parameter types against the
    /// call's actual argument types and then substituted through the whole
    /// signature; `nvs_types` owns both halves. Nothing user-written can
    /// declare one, which is `docs/agent/loop-goal.md`'s standing decision that
    /// type variables stay compiler-owned.
    Var(&'static str),
    /// A type variable bound from the type argument **written at the call
    /// site**, named — `T` in `decodeAs<T>(string $json): T`.
    ///
    /// The same `nvs_types::ty::Ty::TypeVar` as [`Self::Var`] once lowered,
    /// and the same substitution afterwards; what differs is where the binding
    /// comes from. [`Self::Var`] is *inferred* from an argument's type, which
    /// only works where some parameter position holds the answer —
    /// `Core\Json::decodeAs`'s `T` appears in no parameter at all, so the call
    /// site has to say it. `docs/spec/01-core-library.md` § 6 writes exactly
    /// that, and `docs/agent/loop-goal.md`'s standing decision names the
    /// explicit call-site type argument as one of the two things user code
    /// gets from the compiler-owned `<T>` machinery.
    ///
    /// **A member's written parameters are these variants, in first-appearance
    /// order over `params` then `return_ty`** — [`CoreMethod::written`] is the
    /// one place that traversal happens, so the order a call site's arguments
    /// bind in cannot drift from the order the row declares them. A member
    /// mixing this with [`Self::Var`] is legal and each half binds from its own
    /// side; a member with none refuses a written type-argument list outright
    /// (`E_TYPE_ARGS_NOT_GENERIC`), which is what keeps an *inferred* variable
    /// from gaining a second, unchecked spelling.
    Written(&'static str),
    /// `A|B|...` — ADR 0007 § 3's union, at least two members.
    ///
    /// Legal in **either** direction. A helper's argument slot is a whole
    /// `nvs_runtime::Value` whose tag `nvs-codegen` writes from the argument's
    /// own representation, so a union parameter needs no IR type of its own
    /// and the body decodes by tag; a union *return* lands in the same 16-byte
    /// value, read back as `nvs_ir::ty::Ty::Tagged` — that variant's own doc
    /// comment owns the representation and what it spends.
    ///
    /// **An option's type only where `null` is not one of the values it
    /// admits**, which is the one restriction left. `CoreTy::Options` flattens
    /// a bag into one ABI argument per option and an omitted option passes a
    /// [`Const`], which has no union-shaped spelling — so what a union option
    /// defaults to is [`Const::Null`], read back by the helper as "not given"
    /// exactly as it already is for the `{by?: callable}` an option cannot
    /// otherwise spell. That works for as long as no *written* value can
    /// arrive as a `Tag::Null` too, which is why a [`Self::Nullable`] member
    /// (or a bare nullable option) is refused: there, "omitted" and
    /// `{a: null}` would be one argument.
    /// `Core\Validate::isIp`'s `{version?: 4|6}` was the row that wanted a
    /// union here first — [`crate::validate`]'s docs say why the alternative
    /// was worse — and `Core\Arr::column`'s `{indexBy?: int|string}` is the
    /// one that showed the restriction was about `null` rather than about
    /// literals. `a_union_option_excludes_null` holds it.
    Union(&'static [CoreTy]),
    /// **One `int` literal** — [ADR 0047](../../../../docs/adr/0047-literal-and-enum-case-types.md)
    /// § 1's integer atom, whose only use is inside a [`Self::Union`] that
    /// spells out a closed set of numbers.
    ///
    /// The same relationship to [`Self::Int`] that [`Self::EnumCase`] has to
    /// [`Self::Enum`], and it exists for the same reason at a different place:
    /// spec § 12 writes `isIp(string $s, {version?: 4|6})`, and the point of
    /// that spelling is that `{version: 5}` does not compile. `int` would not
    /// say that, and a `Core\IpVersion` enum would say it by adding a name to
    /// the surface — which is precisely what § 12 removed `isIpV4`/`isIpV6` to
    /// avoid.
    ///
    /// Never a whole parameter or a bare option type, for [`Self::EnumCase`]'s
    /// reason: a position admitting exactly one number admits no choice, and
    /// would be an argument the caller writes and the member could assume.
    /// `a_literal_type_only_appears_inside_a_union` holds that.
    IntLiteral(i64),
    /// `?T` — [ADR 0066](../../../../docs/adr/0066-nullable-conversion-operator.md)'s
    /// nullable, which the spec's own tables write at every member that
    /// answers "absent" (`Core\Arr::first`, `Str::indexOf`, `Path::extension`
    /// — ADR 0063 R5 makes it the *only* absence spelling).
    ///
    /// A variant of its own rather than a [`Self::Union`] with a `Null`
    /// member, because that is what the spec writes and because there is no
    /// other position a bare `null` type would be legal in. It interns as
    /// exactly `Union([Null, T])` all the same — the checker has no separate
    /// nullable type — so it inherits everything the union arm above says,
    /// including the `Ty::Tagged` representation the value comes back in.
    ///
    /// **Never wraps a nullable or a `void`**: `??T` is `?T` and the interner
    /// would silently collapse it, while `?void` is not a type at all.
    /// `a_nullable_wraps_something_that_can_be_null` holds both.
    Nullable(&'static CoreTy),
    /// A `Core`-owned enum, named by its fully-qualified name — `Core\Order`
    /// in `sort(array<T> $a, {order?: Order, ...})`.
    ///
    /// The name is resolved against [`ENUMS`], not against the compiled
    /// program: an enum the spec's own tables name is part of `Core`'s
    /// surface exactly as a member is, so it is declared here and seeded into
    /// the checker's enum table alongside the user's own — see
    /// [`ENUMS`] for why that is one table rather than two.
    ///
    /// Carries no backing type. ADR 0010 § 2 makes `int` the default and
    /// there is no reason for a `Core` enum to be anything else: nothing
    /// stores one, so the only thing a `uint` backing could buy is a case
    /// past `i64::MAX`.
    Enum(&'static str),
    /// **One case** of a `Core`-owned enum, named by that enum and the case —
    /// [ADR 0047](../../../../docs/adr/0047-literal-and-enum-case-types.md)
    /// § 3's narrowed type, whose only use is inside a [`Self::Union`] that
    /// spells out a closed subset.
    ///
    /// `Core\Hash::hmac`'s third parameter is the reason it exists.
    /// [`docs/spec/01-core-library.md`](../../../../docs/spec/01-core-library.md)
    /// § 11 writes that parameter as `StrongDigest`, "the closed subset that
    /// the HMAC and signature members declare," so that
    /// `Hash::hmac($m, $k, Digest::Md5)` is a compile error naming the reason.
    /// A *second enum* would not say that: `Core\StrongDigest::Sha256` would
    /// be a different type from `Core\Digest::Sha256`, and no value could be
    /// passed to both `of` and `hmac`. A union of case types is the shape ADR
    /// 0047 already gives that idea, and the checker already places an
    /// enum-case expression against it — `nvs_types::expr::literals`'
    /// `placed_literal` looks inside a union, so `Digest::Sha256` narrows to
    /// its case type and `Digest::Md5` stays the whole enum and fails to
    /// assign.
    ///
    /// Never a whole parameter on its own: a position that admits exactly one
    /// case admits no choice at all, and would be an argument the caller has
    /// to write and the member could have assumed.
    /// `an_enum_case_type_only_appears_inside_a_union` holds that.
    EnumCase(&'static str, &'static str),
    /// An **instance** of a `Core`-owned class, named by its fully-qualified
    /// name — `Core\Regex\Match` in `match(string $s, string $p): ?Match`.
    ///
    /// The name is resolved against [`CLASSES`] exactly as [`Self::Enum`]'s is
    /// against [`ENUMS`]: the class is part of `Core`'s surface, so it is
    /// declared here and seeded into the checker's signature table, where it
    /// becomes an ordinary class type. Nothing downstream of that point can
    /// tell it from a user-declared class — the checker resolves a method on
    /// it through `resolve_method`, and `nvs-ir` lowers a value of it to
    /// `Ty::Object`.
    ///
    /// What makes it *`Core`*-owned is the two things [`CoreClass::slots`] and
    /// [`CoreClass::instance`] state: the instance's field slots are
    /// `nvs-stdlib`'s to lay out rather than a program's to declare, and every
    /// method on it is a native helper reached with the receiver in argument
    /// slot 0. So there is no constructor, no property and no subclass — a
    /// program can only receive one from a member that returns it.
    Instance(&'static str),
    /// **Whatever `foreach` accepts**, over the element type wrapped:
    /// [ADR 0053](../../../../docs/adr/0053-iteration-and-generators.md) § 3's
    /// three shapes at once, interned as the union
    /// `array<T>|Iterable<T>|Iterator<T>`. `Core\Arr::from`'s
    /// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
    /// § 2 row is the first to write one, and § 9's collections are the next.
    ///
    /// **A plain `array<T>` satisfies it, and the spec row is written that
    /// way.** ADR 0053 § 3 fixes the set of things that can be iterated at
    /// exactly three, so a member asking for "a sequence" that accepted only
    /// two of them would refuse `Core\Arr::from($array)` — the commonest
    /// argument, and the one PHP's own `iterator_to_array` has accepted since
    /// 8.2. The alternative was a second, narrower notion of iterability
    /// living in the registry, which is the drift AGENTS.md's one-home rule
    /// exists to prevent: this variant *is* § 3's list, and a fourth shape
    /// added there would be added here.
    ///
    /// **A helper reads one by tag, and never as a pre-drained array.** The
    /// interned union gives the parameter `nvs_ir::ty::Ty::Tagged`, so the
    /// argument slot holds a whole 16-byte `nvs_runtime::Value`: `Tag::Array`
    /// is an `NvsArray` the helper walks directly, and `Tag::Obj` is a cursor
    /// it *drives* — `iterate()` first when the value reaches `Iterable<T>`,
    /// then `advance()`/`current()` — through the class descriptor's own
    /// method table, exactly as `nvs_runtime::call_closure` already reaches a
    /// closure's `invoke`. Every one of those members is bodiless
    /// (`nvs_types::iter_lib`), so that name lookup *is* the dispatch a
    /// `foreach` over the same value performs. Materialising the sequence into
    /// an array in the IR before the call was the rejected alternative: it
    /// allocates a second copy of every array argument, and it would drain an
    /// unbounded generator before `Core\Arr::from`'s `{limit: n}` — the spec's
    /// only guard against exactly that — could stop it.
    ///
    /// **Parameter position only.** A member *returning* a sequence would be
    /// answering with a union whose object half a program can only consume by
    /// `foreach`, and no spec row does: every row that produces a collection
    /// produces an `array<T>` or a named [`Self::Instance`].
    Iterated(&'static CoreTy),
    /// `...$rest` — a **variadic** tail, wrapping the type *each* trailing
    /// argument is checked against (`mixed` in
    /// `format(string $template, mixed ...$arguments)`).
    ///
    /// Only ever the **last** entry of [`CoreMethod::params`], never beside a
    /// [`Self::Options`] bag, and never nested: `a_variadic_tail_is_last_and_alone`
    /// holds all three. The bag exclusion is not a limitation of the ABI but
    /// of the *call site* — a trailing `{…}` written after a variadic tail is
    /// ambiguous between "one more argument" and "the bag", and ADR 0063 R20
    /// leaves no room for a rule that guesses.
    ///
    /// **One ABI argument, not one per written argument.** A helper's
    /// `args: [N]` is a fixed arity, so `nvs_ir::lower::lower_call_args`
    /// collects every argument from this position onward into a fresh
    /// `array<T>` — keys `"0"`, `"1"`, … — and passes that single value. So
    /// `format` is an ordinary `args: [2]` helper whose second slot is a
    /// `Tag::Array`, and the body iterates it the way
    /// [`crate::str`]'s `join` iterates its subject. The rejected alternative
    /// was a second calling convention carrying a count: it would put a
    /// variable-arity path into `nvs-codegen`'s helper emission for one member
    /// shape, and buy only the allocation this spends.
    ///
    /// It carries no default and never appears in [`CoreMethod::defaults`]:
    /// zero trailing arguments is already what an empty array means, so
    /// `MethodSig::required()` stops one short of the parameter list for a
    /// variadic signature.
    Variadic(&'static CoreTy),
    /// ADR 0063 R2's trailing options shape — `{step?: int}`, one
    /// [`CoreOption`] per declared option, in the order the ABI passes them.
    /// See this module's own docs for why it is its own type rather than a
    /// [`CoreTy`] wrapping an ADR 0036 shape.
    ///
    /// Only ever the **last** entry of [`CoreMethod::params`], and never
    /// listed in [`CoreMethod::defaults`]: every option has a default of its
    /// own, so the bag itself is optional by construction rather than by
    /// declaration. `an_options_bag_is_last_and_never_empty` holds both.
    Options(&'static [CoreOption]),
}

/// One option inside a [`CoreTy::Options`] bag: its name, its type, and the
/// value a call that leaves it out passes.
///
/// The default is stated here rather than in [`CoreMethod::defaults`] because
/// an option is named, not positional — there is no end-alignment rule that
/// could relate a run of defaults to a set of names, and stating it beside the
/// name is the only arrangement in which the two cannot drift apart.
#[derive(Clone, Copy, Debug)]
pub struct CoreOption {
    /// The option's own name, `camelCase` per ADR 0029 — what a call site
    /// writes on the left of the `:` in `{step: 2}`.
    pub name: &'static str,
    /// Its declared type. Never itself a [`CoreTy::Options`]: a bag flattens
    /// to one ABI argument per option, and a nested one would have nothing to
    /// flatten into.
    pub ty: CoreTy,
    /// The constant a call that omits this option passes — materialized at the
    /// call site by `nvs_ir::lower::lower_call_args`, exactly as an omitted
    /// positional parameter's default is.
    pub default: Const,
}

/// One optional parameter's default value.
///
/// The registry's counterpart of `nvs_types::defaults::ConstArg`, kept
/// separate for the reason [`CoreTy`] is kept separate from
/// `nvs_types::ty::Ty`: this states what the *spec* wrote, and `nvs-types`
/// translates it into the one representation the checker and `nvs-ir` share.
/// Only the shapes that enum can already emit are expressible — a member whose
/// spec signature defaults to `null` cannot be registered until
/// `nvs_types::defaults` grows that variant, which is exactly the friction
/// this crate wants around a signature the whole language resolves against.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Const {
    /// `null` — an option that was **not given**.
    ///
    /// ADR 0063 R2 makes every option optional, but the spec writes several
    /// whose type has no "absent" value in it: `Core\Arr::sort`'s
    /// `{by?: callable, comparator?: callable}` are the first two — a
    /// `callable` cannot be a "no callback" callable, and inventing a
    /// do-nothing one would silently change the answer. So the *declared*
    /// type stays what a call site may write, and the default an omitting
    /// call site passes is this: the helper reads `Tag::Null` and takes its
    /// own not-given path.
    ///
    /// This is the only default whose value is not of its option's declared
    /// type, and deliberately: `?callable` would be a union, and
    /// `a_union_is_only_ever_a_parameter` records why an option cannot be
    /// one.
    Null,
    /// A `bool` default.
    Bool(bool),
    /// An `int` default.
    Int(i64),
    /// A `uint` default.
    Uint(u64),
    /// A `float` default.
    Float(f64),
    /// A `string` default, already cooked — a registry row writes the bytes it
    /// means, so there is no escape grammar here at all.
    Str(&'static str),
    /// A `bytes` default, as the octets themselves.
    ///
    /// Separate from [`Self::Str`] rather than reusing it, because the two
    /// differ in exactly the way [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)
    /// § 1 says they do: a `bytes` default carries no UTF-8 promise, so it is
    /// written as a byte string (`b"…"`) and materialized under `Tag::Bytes`.
    /// Passing a `Str` default into a `bytes` parameter would be a type lie
    /// the helper would have to `FATAL` on — `Core\Bytes::join`'s
    /// `$separator = ""` is the row that wanted this variant, and the module
    /// doc there records what it was blocked on.
    Bytes(&'static [u8]),
    /// The **empty array**, `[]` — the only array a default is ever written
    /// as.
    ///
    /// A variant of its own rather than a general array constant, because the
    /// spec never writes a populated one: `Core\Arr::replaceRange`'s
    /// `$replacement = []` is the row that wanted this, and "nothing to
    /// splice in" is the only array default a member has a use for. A
    /// populated one would also have to state its keys, which is a second
    /// literal grammar this table has no reason to grow.
    ///
    /// It costs one allocation per *use site*, exactly as [`Self::Str`] does
    /// and for the same reason — an array is refcounted, so a call that omits
    /// the argument materializes a fresh empty one rather than sharing a
    /// static. That is a byte per call the caller was going to spend anyway
    /// on the `[]` it would otherwise have written.
    EmptyArray,
    /// A [`CoreTy::Enum`] case, by enum name and case name — the default for
    /// an option whose type is a `Core` enum.
    ///
    /// Named rather than written as the integer it is so that the default and
    /// the case cannot drift apart: ADR 0010 § 3 makes a case an integer
    /// constant, and [`ENUMS`] is the one place that constant is stated.
    /// `nvs_types::core_lib` resolves it there; `every_enum_case_default_names_a_real_case`
    /// holds that it resolves at all.
    EnumCase(&'static str, &'static str),
    /// A `Core`-owned **instance**, named by the symbol that builds it and the
    /// constant arguments it takes — `Core\Time\Zone::UTC` is
    /// `Zone::of("UTC")`.
    ///
    /// The one variant with no scalar under it, and the only shape a
    /// [`CoreTy::Instance`]-typed [`CoreConst`] can have: an instance has a
    /// heap layout that nothing outside [`crate::instance`] lays out, so what
    /// is stated here is the *call* that produces one rather than the bytes it
    /// holds. `nvs-ir` lowers it to exactly the `InstKind::CoreCall` a written
    /// `Zone::of("UTC")` lowers to — so a constant is still ADR 0010 § 3's
    /// "inlined at every use site" and still has no storage, no descriptor and
    /// no address; what it has instead is one allocation per use site, which
    /// is what an instance costs however it is reached.
    ///
    /// **A constant's value only, never an option or a parameter default.**
    /// A default is materialized inside an argument list whose ownership rule
    /// is "borrowed", and a fresh instance there would have no owner to
    /// release it; `a_built_constant_is_never_a_default` holds that.
    Built {
        /// The `Core` symbol that builds the value — a member of the class the
        /// constant is declared on, so the two cannot drift apart.
        symbol: &'static str,
        /// Its arguments, positional, each a constant in its own right.
        args: &'static [Const],
    },
}

/// One member's reference documentation — the card, not the essay.
///
/// [ADR 0117](../../../docs/adr/0117-an-implemented-core-member-documents-itself-in-the-registry.md)
/// § 1's five fields, as plain static data next to the row they describe,
/// so the one artifact that provably matches the shipped behaviour is also
/// the one that documents it. Every field is inline markdown, one or two
/// sentences; long-form prose stays in the website's pages by the same
/// ADR's *Alternatives rejected*. **An empty string or an empty slice means
/// "not written yet"**, never "there is nothing to say": `nvs meta --json`
/// omits such a field, and a consumer falls back to the spec for it
/// (§ 3's field-wise precedence).
///
/// Spends memory per process, never per request — static strings in
/// `.rodata`, a few hundred bytes per documented member, which the ADR's
/// *Consequences* prices for the whole registry.
#[derive(Clone, Copy, Debug)]
pub struct MethodDoc {
    /// What the member does, in one or two sentences.
    pub short: &'static str,
    /// One entry per parameter, positional, in [`CoreMethod::params`]'s
    /// order — the trailing options bag included, documented as one
    /// parameter whose [`ParamDoc::shape`] lists its options.
    pub params: &'static [ParamDoc],
    /// What the member answers with, beyond the type the row already states.
    pub ret: &'static str,
    /// Every error the member throws, and when.
    pub errors: &'static [ErrorDoc],
}

/// One parameter's name and description.
///
/// The first place a parameter's **name** exists in the registry at all —
/// [`crate`]'s gap 3 records that a row carries types and never names, and
/// this is the field that will carry them when a `Core` call becomes
/// callable by name. Until then it is documentation, and nothing in
/// `nvs-types` reads it.
#[derive(Clone, Copy, Debug)]
pub struct ParamDoc {
    /// The name as the spec writes it, without the `$` — `s`, `pattern`,
    /// `options`.
    pub name: &'static str,
    /// What the parameter is, in one sentence.
    pub desc: &'static str,
    /// For a shape-typed parameter — an options bag — one entry per key;
    /// empty for a parameter that is not a shape.
    pub shape: &'static [ShapeKeyDoc],
}

/// One key of a shape-typed parameter.
#[derive(Clone, Copy, Debug)]
pub struct ShapeKeyDoc {
    /// The key's own name — what a call site writes on the left of the
    /// `:` in `{pretty: true}`.
    pub key: &'static str,
    /// Its type, spelled as the spec spells it — `bool`, `int`, `?string`.
    pub ty: &'static str,
    /// What the key does, in one sentence.
    pub desc: &'static str,
}

/// One error a member throws.
#[derive(Clone, Copy, Debug)]
pub struct ErrorDoc {
    /// The thrown class's name as a `catch` writes it —
    /// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
    /// § 10's tree, so `RuntimeError`, `ParseError`, and not a namespaced
    /// spelling the language has no such class under.
    pub error: &'static str,
    /// When it is thrown, in one sentence.
    pub desc: &'static str,
}

/// One `Core` member.
#[derive(Clone, Copy, Debug)]
pub struct CoreMethod {
    /// The member's own name, `camelCase` per ADR 0029.
    pub name: &'static str,
    /// Each parameter's declared type, positional. ADR 0063 R1 puts the
    /// subject first, and R2 puts an options bag ([`CoreTy::Options`]) last if
    /// the member has one.
    pub params: &'static [CoreTy],
    /// Defaults for the *trailing* optional positional parameters, aligned to
    /// the end of [`Self::positional`] — so `positional().len() -
    /// defaults.len()` is how many arguments a call must supply, and an empty
    /// slice means every positional parameter is required.
    ///
    /// A trailing options bag is excluded on both sides of that subtraction:
    /// it is optional by construction and carries its own per-option defaults,
    /// so it never appears here. See [`CoreTy::Options`].
    ///
    /// Aligned to the end rather than carrying one entry per parameter because
    /// that is the only arrangement the language allows: a required parameter
    /// can never follow an optional one (`E_PARAM_DEFAULT_ORDER`), so a
    /// per-parameter list would be a run of `None` followed by a run of `Some`
    /// and every row would spell out the `None`s.
    pub defaults: &'static [Const],
    /// The declared return type.
    pub return_ty: CoreTy,
    /// The linker symbol its implementation is reachable at — what
    /// [`crate::symbols`] hands the JIT and what `nvs-ir` records in the
    /// instruction it lowers a call to. Prefixed `nvs_core_` so a `Core`
    /// member is never mistakable for a `nvs_runtime` primitive in a
    /// disassembly.
    pub symbol: &'static str,
    /// The member's reference documentation, or `None` for a row not yet
    /// documented —
    /// [ADR 0117](../../../docs/adr/0117-an-implemented-core-member-documents-itself-in-the-registry.md)'s
    /// seam. Read by `nvs meta --json` and by nothing on the request path;
    /// the runtime dispatches on [`Self::symbol`] and never looks here.
    pub doc: Option<&'static MethodDoc>,
}

impl CoreTy {
    /// This type's ADR 0088 § 2 classification, or `None` for a type that
    /// carries none — every type that is not a `string`/`bytes` *parameter*,
    /// and the unclassified [`Self::Str`]/[`Self::Bytes`] spellings, whose
    /// `None` is the refusal rather than an omission.
    #[must_use]
    pub const fn classification(&self) -> Option<Qual> {
        match self {
            Self::Text(qual) | Self::Blob(qual) => Some(*qual),
            _ => None,
        }
    }

    /// Whether this type is a `string` or `bytes`, classified or not — the one
    /// place a consumer asks the question without caring which spelling it is
    /// looking at.
    #[must_use]
    pub const fn is_text_like(&self) -> bool {
        matches!(
            self,
            Self::Str | Self::Bytes | Self::Text(_) | Self::Blob(_)
        )
    }
}

impl CoreMethod {
    /// This member's trailing options bag, or `None` for a member with none —
    /// the one place the "always last" rule of [`CoreTy::Options`] is read,
    /// so no consumer scans [`Self::params`] for it a second time.
    #[must_use]
    pub fn options(&self) -> Option<&'static [CoreOption]> {
        match self.params.last() {
            Some(CoreTy::Options(options)) => Some(options),
            _ => None,
        }
    }

    /// The type each argument past this member's fixed parameters is checked
    /// against, or `None` for a member with no variadic tail — the one place
    /// the "always last" rule of [`CoreTy::Variadic`] is read.
    #[must_use]
    pub fn variadic(&self) -> Option<&'static CoreTy> {
        match self.params.last() {
            Some(CoreTy::Variadic(elem)) => Some(elem),
            _ => None,
        }
    }

    /// This member's **written** type parameters, in the order a call site's
    /// `<...>` list binds them — [`CoreTy::Written`]'s first-appearance order
    /// over [`Self::params`] and then [`Self::return_ty`], with a name that
    /// appears twice counted once.
    ///
    /// Empty for all but a handful of members, and the cheap test a call site
    /// runs before doing any of this work at all.
    #[must_use]
    pub fn written(&self) -> Vec<&'static str> {
        let mut found = Vec::new();
        for param in self.params {
            collect_written(param, &mut found);
        }
        collect_written(&self.return_ty, &mut found);
        found
    }

    /// The positional parameters — [`Self::params`] without a trailing options
    /// bag. What [`Self::defaults`] aligns to the end of.
    #[must_use]
    pub fn positional(&self) -> &'static [CoreTy] {
        match self.options() {
            Some(_) => &self.params[..self.params.len() - 1],
            None => self.params,
        }
    }
}

/// [`CoreMethod::written`]'s walk: every [`CoreTy::Written`] name reachable
/// from `ty`, appended to `found` in first-appearance order and never twice.
///
/// The wildcard arm is deliberate — [`CoreTy`] is `#[non_exhaustive]`, and a
/// variant that carries no nested type carries no variable either.
fn collect_written(ty: &CoreTy, found: &mut Vec<&'static str>) {
    match ty {
        CoreTy::Written(name) => {
            if !found.contains(name) {
                found.push(name);
            }
        }
        CoreTy::Array(inner)
        | CoreTy::Nullable(inner)
        | CoreTy::Variadic(inner)
        | CoreTy::Iterated(inner) => {
            collect_written(inner, found);
        }
        CoreTy::Union(members) => {
            for member in *members {
                collect_written(member, found);
            }
        }
        CoreTy::Options(options) => {
            for option in *options {
                collect_written(&option.ty, found);
            }
        }
        _ => {}
    }
}

/// One `Core` class constant — [ADR 0011](../../../../docs/adr/0011-functions-and-constants-are-class-members.md)'s
/// "every constant is a class constant", which is what `Core\Math::PI`
/// replaces PHP's global `M_PI` with.
///
/// A constant is not a member with an arity, so it is a roster of its own on
/// [`CoreClass`] rather than a [`CoreTy`] variant: it has a *value* and no
/// signature, and nothing about it is resolved through the method table.
/// The value reuses [`Const`] — the same enum an omitted option's default is
/// written in — because the two want exactly the same thing, a literal the
/// compiler can materialize at the use site, and ADR 0010 § 3's "inlined at
/// every use site" rule for an enum case is the one this follows too: a
/// `Core` constant has no storage, no descriptor and no address.
#[derive(Clone, Copy, Debug)]
pub struct CoreConst {
    /// The constant's own name, `SCREAMING_SNAKE_CASE` per ADR 0029.
    pub name: &'static str,
    /// Its declared type — what a `var $x = Core\Math::PI;` binding infers.
    /// Always a scalar, since [`Const`] can express nothing else.
    pub ty: CoreTy,
    /// Its value, inlined wherever the constant is written.
    pub value: Const,
}

/// One `Core` domain class — ADR 0011's "every callable is a class member,"
/// with `Core` as the reserved namespace.
///
/// Most are pure **namespaces**: a roster of static members, no state, and
/// nothing a program can hold a value of. A class that declares
/// [`Self::slots`] and [`Self::instance`] is the other kind — see
/// [`CoreTy::Instance`], which owns what a `Core`-owned instance is and what it
/// is not.
#[derive(Clone, Copy, Debug)]
pub struct CoreClass {
    /// The fully-qualified name, backslash-separated exactly as written in
    /// source (`Core\Arr`).
    pub name: &'static str,
    /// Its **static** members, in the spec's own order — `Core\Arr::count`,
    /// reached through the class name and nothing else.
    pub methods: &'static [CoreMethod],
    /// Its **instance** members, in the spec's own order — `$match->text()`,
    /// reached through a value and nothing else.
    ///
    /// A separate roster rather than a flag on [`CoreMethod`] because the two
    /// differ in *shape*, not only in reachability: an instance member's
    /// receiver is implicit, so it is absent from [`CoreMethod::params`] and
    /// present in argument slot 0 at the ABI, exactly the way a compiled Novis
    /// method's is. Empty for every namespace class, which is most of them.
    pub instance: &'static [CoreMethod],
    /// One name per field slot an instance of this class holds, in slot order
    /// — the layout `nvs-stdlib` builds an instance against and the helper
    /// bodies read back by index.
    ///
    /// Named rather than merely counted so the one file that writes a slot and
    /// the one that reads it agree on more than a number. Empty for a namespace
    /// class; nothing outside this crate reads it, since a `Core` instance has
    /// no property a program can reach ([`CoreTy::Instance`]).
    pub slots: &'static [&'static str],
    /// Its constants, in the spec's own order — empty for a class the spec
    /// gives none, which is most of them.
    pub constants: &'static [CoreConst],
}

impl CoreClass {
    /// Looks one of this class's constants up by name.
    #[must_use]
    pub fn constant(&self, name: &str) -> Option<&'static CoreConst> {
        self.constants.iter().find(|found| found.name == name)
    }

    /// Every member this class declares, static then instance — what a
    /// consumer that only cares about "the code behind a name" iterates, so
    /// neither roster can be forgotten at one of them.
    pub fn members(&self) -> impl Iterator<Item = &'static CoreMethod> {
        self.methods.iter().chain(self.instance)
    }

    /// The index of the slot named `slot`, for a helper body reading an
    /// instance's own state back.
    ///
    /// # Panics
    ///
    /// Panics naming the slot if this class declares none by that name — both
    /// halves are in this crate, so that is a build-time oversight rather than
    /// anything a program can cause.
    #[must_use]
    pub fn slot(&self, slot: &str) -> usize {
        self.slots
            .iter()
            .position(|found| *found == slot)
            .unwrap_or_else(|| panic!("{} declares no `{slot}` slot", self.name))
    }
}

/// Every `Core` class the compiler knows.
///
/// The single home for "does `Core\X::y` exist" — see [`crate`]'s own known
/// gap 1 for how much of the spec is here so far.
///
/// **One line per class, never one per member.** Each domain module declares
/// its own `CLASS` const beside the implementations it registers, so adding a
/// member touches exactly that module, and adding a *class* adds one line here
/// plus one in [`crate::symbols`]. That is what lets two sessions add two
/// different domains without touching the same lines; the flat table this
/// replaced made every such pair conflict. Order is the spec's own § order.
pub const CLASSES: &[CoreClass] = &[
    crate::str::CLASS,
    crate::arr::CLASS,
    // ADR 0046 §§ 4-5's structural retrieval. Registered like any other class
    // and implemented by nothing — see [`crate::attributes`].
    crate::attributes::CLASS,
    crate::math::CLASS,
    crate::regex::CLASS,
    crate::regex::MATCH,
    crate::regex::PATTERN,
    crate::json::CLASS,
    crate::encoding::CLASS,
    crate::bytes::CLASS,
    crate::path::CLASS,
    crate::time::TIME,
    crate::time::INSTANT,
    crate::time::DATETIME,
    crate::time::DATE,
    crate::time::TIME_OF_DAY,
    crate::time::DURATION,
    crate::time::ZONE,
    crate::objmap::CLASS,
    crate::objset::CLASS,
    crate::heap::CLASS,
    crate::random::CLASS,
    crate::uuid::CLASS,
    crate::hash::CLASS,
    crate::hash::STREAM,
    crate::uri::CLASS,
    // § 15's link half only — ADR 0077 § 4's `url`/`urlAbsolute`. `match` and
    // `methodsFor` answer a request and land with the server; [`crate::router`]
    // owns why, and owns the enum that section's `method` parameter takes.
    crate::router::CLASS,
    crate::csv::CLASS,
    // ADR 0023 § 2's externalizing carrier, and no spec § of its own: the walk
    // it reaches is `nvs_runtime::graph`'s, shared with the `spawn` boundary.
    crate::serialize::CLASS,
    crate::validate::CLASS,
    crate::out::CLASS,
    // § 16. [ADR 0092](../../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
    // § 4's `dump` and `render` only — the coverage, trace and profile members
    // that section also lists are ADR 0018's and land at M10.
    crate::debug::CLASS,
    // [ADR 0079](../../../../docs/adr/0079-testing-is-a-language-feature.md)
    // § 4's assertion surface rather than a spec § of its own: testing is a
    // language feature, and `Core\Test` is the same `QName` `#[Test]` names.
    crate::test::CLASS,
    // ADR 0072 § 1's `Core\Task`, and no spec § of its own either: structured
    // concurrency is a language surface over the `nvs-host` scheduler. The
    // signature is here and the body is a placeholder — [`crate::task`] owns
    // why, and it is the one row of the three that is not compile-time folded.
    crate::task::CLASS,
    // Goal 2's item 11, and no spec § of its own either — ADR 0072's scope
    // line leaves this type's spelling undecided, so [`crate::channel`]'s
    // module doc is the one home for the surface and for why the queue lives
    // in the instance's own slots rather than in the host.
    crate::channel::CLASS,
    // ADR 0006's isolate handle, and no spec § of its own either — the
    // concurrency *language* surface belongs to `docs/spec/00-overview.md` § 2,
    // which is where `spawn script`'s grammar already is. The one class here
    // with no members at all; [`crate::script`] owns why that is the point.
    crate::script::HANDLE,
    // § 13, and the second row after `Core\Attributes` whose members never
    // run: ADR 0061 § 3 expands `implementing<T>()` while checking, so
    // [`crate::program`] registers a signature and an aborting body.
    crate::program::CLASS,
    // § 13, and here only because § 12's `Core\Out::capture` answers with it —
    // ADR 0088 § 5. [`crate::cli`]'s module docs own why the rest of
    // `Core\Cli` is not here.
    crate::cli::TEXT,
];

/// [ADR 0066](../../../../docs/adr/0066-nullable-conversion-operator.md)
/// The `Core` classes that declare a `tryParse` beside their `parse` —
/// [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md) § 3a's
/// closed exception to [ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
/// R5's `try…` ban.
///
/// A **list, not a roster**: unlike [`CONSTRUCTORS`] this drives nothing at
/// run time, because `tryParse` is an ordinary member with an ordinary symbol
/// that an ordinary `Core\Uri::tryParse($s)` call site resolves. It exists so
/// `a_try_parse_is_its_own_class_parse_made_nullable` can hold § 3a's three
/// conditions mechanically, and so a class added here has to be added
/// deliberately rather than by writing a member with a suggestive name.
///
/// This replaces the withdrawn `PARSE_ROSTER`, which carried one **non**-member
/// symbol per class for `$s as ?Core\Uri` to lower to. ADR 0066 § 3 withdrew
/// that form: `as` never targets a class now, with no exceptions, so nothing
/// here is chained into [`crate::symbols`] and `nvs-ir` has no roster to read.
pub const TRY_PARSE_CLASSES: &[&str] = &[crate::uri::NAME, crate::uuid::NAME];

/// Every `Core` class a program may write `new` on, with the constructor that
/// builds one — `docs/spec/01-core-library.md` § 9's collections and nothing
/// else.
///
/// A roster rather than a synthetic `constructor` row on [`CoreClass`], for
/// the reason [`crate::instance`]'s module docs give: a `Core` class has no
/// member a program resolves here, and `nvs-ir` reads this to lower `new` on
/// one to an ordinary helper call. A name here **must** be in [`CLASSES`] —
/// unlike [`GENERIC_CLASSES`], whose arity is a property of the spec's table
/// rather than of anything on disk — because the helper builds an instance
/// against that class's declared [`CoreClass::slots`].
///
/// The second half is a whole [`CoreMethod`] rather than a bare symbol so that
/// **a constructor can take arguments**: § 9's `Core\Heap` is ordered by a
/// comparator given at construction, and one written `params`/`defaults` pair
/// is what lets `nvs_types::core_lib` seed it as an ordinary `constructor`
/// signature — the same shape every other `Core` row is checked through,
/// rather than a second arity rule reachable only from `new`. Its `name` is
/// `constructor` and its `return_ty` the class itself, so it reads the same
/// way in a signature table as a user-declared one.
pub const CONSTRUCTORS: &[(&str, &CoreMethod)] = &[
    (crate::objmap::NAME, &crate::objmap::NEW),
    (crate::objset::NAME, &crate::objset::NEW),
    (crate::heap::NAME, &crate::heap::NEW),
    (crate::channel::NAME, &crate::channel::NEW),
];

/// The constructor that builds a `class` instance, or `None` when `new` on it
/// is not a thing a program may write — which is every other name.
#[must_use]
pub fn constructor_of(class: &str) -> Option<&'static CoreMethod> {
    CONSTRUCTORS
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, method)| *method)
}

/// The symbol that builds a `class` instance, or `None` when `new` on it is
/// not a thing a program may write — which is every other name.
#[must_use]
pub fn constructor_symbol(class: &str) -> Option<&'static str> {
    constructor_of(class).map(|method| method.symbol)
}

/// One `Core`-owned enum — [ADR 0010](../../../../docs/adr/0010-enums-are-a-value-type.md)'s
/// closed, named integer type, declared here rather than in Novis source.
#[derive(Clone, Copy, Debug)]
pub struct CoreEnum {
    /// The fully-qualified name, backslash-separated exactly as written in
    /// source (`Core\Order`).
    pub name: &'static str,
    /// Its cases, in declaration order. The value is each case's own
    /// constant, written out rather than auto-incremented: ADR 0010 § 1's
    /// auto-increment is a *source* convenience, and a table read by the
    /// compiler has nothing to gain from re-deriving what it could state.
    pub cases: &'static [(&'static str, i64)],
}

/// Every enum `Core` owns.
///
/// A second roster beside [`CLASSES`] rather than a member of it, because an
/// enum is not a class: [ADR 0011](../../../../docs/adr/0011-functions-and-constants-are-class-members.md)
/// puts every *callable* on a class, and an enum has none. `nvs_types::enums`
/// seeds its own table from this, so `Core\Order::Desc` resolves to an integer
/// constant through exactly the machinery a user-declared `enum` already goes
/// through — the same "seed a table rather than special-case `Core`" rule
/// `nvs_types::core_lib` states for members.
///
/// The spec's § 4 names `Month { January … December }` and it is deliberately
/// absent: no member takes or answers with it yet, and an entry here is
/// reachable from source the moment it exists — a case a program can write and
/// pass nowhere is surface with no meaning behind it. That is the test every
/// row below has passed, and it is the one [`crate::router::METHOD`] passes
/// through an *attribute* rather than through a member: ADR 0077 § 1's
/// `#[Route(method: …)]` is where a program writes a case of it, and
/// [`crate::router::AUDIENCE`] passes it the same way, through ADR 0096
/// § 1a's `#[Access(allow: …)]`.
///
/// One line per enum, declared beside the member that takes it — the same
/// rule [`CLASSES`] follows, for the same reason.
pub const ENUMS: &[CoreEnum] = &[
    crate::arr::ORDER,
    crate::arr::SET_ON,
    crate::str::NORMAL_FORM,
    crate::math::ROUND_MODE,
    crate::encoding::CHARSET,
    crate::time::UNIT,
    crate::time::WEEKDAY,
    crate::hash::DIGEST,
    crate::router::METHOD,
    crate::router::AUDIENCE,
];

/// Looks a class up by its fully-qualified name.
#[must_use]
pub fn class(name: &str) -> Option<&'static CoreClass> {
    CLASSES.iter().find(|class| class.name == name)
}

/// Whether a `Core`-owned class renders as text —
/// [ADR 0028](../../../../docs/adr/0028-closing-the-remaining-magic-methods.md)
/// § 1's question, asked here because a `Core` class has no other place to
/// answer it: it declares no interfaces, so there is no `Stringable` for
/// `nvs_types` to prove against, and its members are the rows above rather
/// than entries in a class graph.
///
/// **Two rows answer yes, and they are two different rules.** A class the spec
/// gives a `toString` renders through that member, which is ADR 0028 § 1
/// exactly. A **sink carrier** renders through
/// [ADR 0088](../../../../docs/adr/0088-output-sinks-and-escaping.md) § 5
/// instead and has no such member: it is the sink's own value type, holding
/// bytes that have *already* been through the sink, so `nvs_runtime` renders
/// it as precisely those bytes and asks for no member at all. That roster is
/// `nvs_runtime::is_carrier`'s and stays there — this reads it rather than
/// copying it, so the two cannot disagree about `Core\Cli\Text`.
///
/// A name this registry does not know is not a `Core` class at all, and
/// answers `false` — the caller's own diagnostic is the right answer for it
/// too, since nothing here will resolve a member on it either.
#[must_use]
pub fn class_renders(name: &str) -> bool {
    nvs_runtime::is_carrier(name) || render_symbol(name).is_some()
}

/// The symbol of `name`'s `toString` — the member half of [`class_renders`],
/// and `None` for both sink carriers, which have no member to name.
///
/// Factored out rather than searched twice: `crate::instance` puts this
/// address on the class's own descriptor, so a `Core` object reached through
/// an erased operand (`mixed $m = Core\Uri::parse(…); echo $m;`) renders
/// through the very implementation this check told the *checker* it would. A
/// second search here would be free to see a member that one did not.
#[must_use]
pub(crate) fn render_symbol(name: &str) -> Option<&'static str> {
    class(name)
        .and_then(|class| class.members().find(|member| member.name == "toString"))
        .map(|member| member.symbol)
}

/// The closed roster of members whose helper is handed the **class written at
/// the call site**, as an extra leading argument.
///
/// [`CoreTy::Written`] tells the *checker* what a `<...>` list binds; it says
/// nothing to the runtime, because a type argument is erased like every other
/// one ([ADR 0007](../../../../docs/adr/0007-explicit-type-system.md)). A
/// member like `Core\Json::decodeAs<User>` needs more than the erasure: it has
/// to build a `User`, which means reaching that class's
/// `nvs_runtime::ClassDesc` from native Rust.
///
/// A roster rather than a field on [`CoreMethod`] because it is one entry
/// today against two hundred member rows, and a field would be `false` on
/// every one of them. `nvs-ir` reads this to decide whether to emit an
/// `InstKind::ClassDescConst` ahead of the call's own arguments; the helper's
/// `args: [N]` therefore counts one more than [`CoreMethod::params`] does, and
/// the descriptor is always **argument 0** — the same slot an instance
/// member's receiver takes, so nothing else about the ABI moves.
pub const WRITTEN_CLASS_MEMBERS: &[(&str, &str)] = &[(r"Core\Json", "decodeAs")];

/// Whether `class::method` is one of [`WRITTEN_CLASS_MEMBERS`].
#[must_use]
pub fn takes_written_class(class: &str, method: &str) -> bool {
    WRITTEN_CLASS_MEMBERS
        .iter()
        .any(|(owner, name)| *owner == class && *name == method)
}

/// The closed roster of `Core`-owned **generic** classes, each with the type
/// parameters it declares, in order — spec § 9's three collections.
///
/// This is the other half of `docs/agent/loop-goal.md`'s standing decision on
/// type variables: user code gets an explicit type argument only where the
/// compiler owns the declaration, and for a `new` target that means this
/// table. `nvs_hir::interfaces::type_params` is the same roster for the
/// reserved *interfaces*, and answers the same shape for the same reason —
/// two tables rather than one because a `Core` class and a global interface
/// are resolved by different rules, not because the question differs.
///
/// **The names are positional and load-bearing.** `ObjectMap<K, V>` against
/// `ObjectSet<T>` is what the checker reads to tell a wrong count
/// (`E_TYPE_ARG_COUNT`) from a target that is not generic at all
/// (`E_TYPE_ARGS_NOT_GENERIC`), and the order is the order a call site's
/// arguments bind in — `ObjectMap<Tag, int>` keys on `Tag`.
///
/// A roster rather than a field on [`CoreClass`], for the reason
/// [`WRITTEN_CLASS_MEMBERS`] gives: it is three entries against every class
/// in [`CLASSES`], and a field would be empty on all the rest. An entry here
/// **need not be registered yet** — the arity is a property of the spec's
/// table, so the grammar and the checker can agree on it before the class's
/// own members land.
pub const GENERIC_CLASSES: &[(&str, &[&str])] = &[
    (r"Core\ObjectMap", &["K", "V"]),
    (r"Core\ObjectSet", &["T"]),
    (r"Core\Heap", &["T"]),
    (r"Core\Task\Channel", &["T"]),
];

/// Every `Core` class a `foreach` can walk, and the element its
/// `Iterable<T>` is fixed at — `nvs_types::core_lib` seeds one
/// `ClassSignature::implements` entry per row.
///
/// A roster rather than a field on [`CoreClass`] for the reason
/// [`GENERIC_CLASSES`] is one: three rows out of forty-odd classes, and a
/// field would be `&[]` on every other line of a table already long enough to
/// read badly. The element may be one of that class's own type variables — a
/// map iterates its keys, so `Core\ObjectMap<K, V>` is `Iterable<K>` — and
/// `nvs_types::expr::iteration` substitutes the receiver's own arguments in
/// before a binding is checked against it, exactly as a member's return type
/// already is.
///
/// The *runtime* half is `nvs_stdlib::instance`'s dispatch roster: this table
/// says the checker will let a `foreach` compile, that one says what the
/// receiver answers `iterate()` with. A row added here without one there is a
/// program that type-checks and faults, so the two are kept in step by
/// `an_iterable_class_answers_the_iteration_protocol`.
pub const ITERABLES: &[(&str, &CoreTy)] = &[
    (r"Core\ObjectMap", &CoreTy::Var("K")),
    (r"Core\ObjectSet", &CoreTy::Var("T")),
    (r"Core\Heap", &CoreTy::Var("T")),
    (r"Core\Task\Channel", &CoreTy::Var("T")),
];

/// The element type `class`'s `Iterable<T>` is fixed at, or `None` when it is
/// not one of [`ITERABLES`].
#[must_use]
pub fn iterable_element(class: &str) -> Option<&'static CoreTy> {
    ITERABLES
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, elem)| *elem)
}

/// The type parameters `class` declares, in order — `None` when it is not one
/// of [`GENERIC_CLASSES`], which is every other name in the program.
#[must_use]
pub fn class_type_params(class: &str) -> Option<&'static [&'static str]> {
    GENERIC_CLASSES
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, params)| *params)
}

/// Looks a `Core`-owned enum up by its fully-qualified name.
#[must_use]
pub fn core_enum(name: &str) -> Option<&'static CoreEnum> {
    ENUMS.iter().find(|found| found.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0066 § 3b: a class with a `tryParse` declares no `isValid`,
    /// because `Core\Uri::isValid($s)` and `Core\Uri::tryParse($s) != null`
    /// are one predicate and R17 keeps one spelling of it. Named for
    /// `Core\Uri` because that is the one where the deletion cost something:
    /// its `isValid` asked the *narrower* "is this an absolute URI", which is
    /// now `->scheme() != null` on the parsed value — one reader call rather
    /// than a second implementation of the grammar, which is the drift
    /// ADR 0066 cites CVE-2024-5458 for.
    #[test]
    fn core_uri_declares_no_is_valid_member() {
        for name in TRY_PARSE_CLASSES {
            let class = class(name).expect("a `tryParse` class is registered");
            assert!(
                class.members().all(|method| method.name != "isValid"),
                "{name} still declares `isValid`"
            );
        }
    }

    /// ADR 0066 § 3a's three conditions, held mechanically: the class has a
    /// `parse` taking exactly one `string` and answering with one of itself,
    /// its `tryParse` takes the same one `string` and answers the **nullable**
    /// of that, and the spelling is `tryParse` — the only `try…` R5 admits.
    ///
    /// What this cannot check is condition 2, that `tryParse` *is* `parse`
    /// plus a caught throw rather than a second implementation. That one is
    /// held by each helper being three lines over the other's own reader, and
    /// by the conformance cases asserting the two agree on the same inputs.
    #[test]
    fn a_try_parse_is_its_own_class_parse_made_nullable() {
        for name in TRY_PARSE_CLASSES {
            let class = class(name).expect("a `tryParse` class is registered");
            let member = |wanted: &str| {
                class
                    .members()
                    .find(|method| method.name == wanted)
                    .unwrap_or_else(|| panic!("{name} declares no `{wanted}`"))
            };
            let (parse, try_parse) = (member("parse"), member("tryParse"));
            for (method, what) in [(parse, "parse"), (try_parse, "tryParse")] {
                assert!(
                    matches!(method.params, [one] if one.is_text_like()),
                    "{name}::{what} takes something other than one `string`"
                );
            }
            assert!(
                matches!(parse.return_ty, CoreTy::Instance(answered) if answered == *name),
                "{name}::parse does not answer with one"
            );
            assert!(
                matches!(
                    try_parse.return_ty,
                    CoreTy::Nullable(CoreTy::Instance(answered)) if answered == name
                ),
                "{name}::tryParse does not answer `?{name}`"
            );
        }
    }

    /// R5's ban is otherwise total: `tryParse` is the one `try…` in the whole
    /// registry, and `…OrNull`, `…Safe` and `…Ex` have no members at all. A
    /// name is worth this much because the ban is what stops the exception
    /// growing back into PHP's `from`/`tryFrom` habit one member at a time.
    #[test]
    fn no_member_spells_a_banned_non_throwing_variant() {
        for class in CLASSES {
            for method in class.members() {
                let name = method.name;
                assert!(
                    !(name.starts_with("try") && name != "tryParse"),
                    "{}::{name} spells a `try…` ADR 0063 R5 bans",
                    class.name
                );
                assert!(
                    !name.ends_with("OrNull") && !name.ends_with("Safe") && !name.ends_with("Ex"),
                    "{}::{name} spells a non-throwing variant ADR 0063 R5 bans",
                    class.name
                );
            }
        }
    }

    /// Every registered name is one the spec's own naming rules allow: a
    /// class under `Core`, a `camelCase` member (ADR 0029), and no leading
    /// underscore anywhere (ADR 0030). Cheap, and it catches a paste error in
    /// a table that will grow to several hundred rows.
    #[test]
    fn every_registered_name_follows_the_casing_rules() {
        for class in CLASSES {
            assert!(
                class.name.starts_with(r"Core\"),
                "{} is not under Core",
                class.name
            );
            for segment in class.name.split('\\') {
                assert!(
                    segment.starts_with(|c: char| c.is_ascii_uppercase()),
                    "{segment} is not PascalCase"
                );
            }
            for method in class.members() {
                assert!(
                    method.name.starts_with(|c: char| c.is_ascii_lowercase()),
                    "{}::{} is not camelCase",
                    class.name,
                    method.name
                );
            }
        }
    }

    /// A [`GENERIC_CLASSES`] entry declares at least one parameter, and no
    /// two of a class's parameters share a name. An empty list would make
    /// `Foo<>` the only accepted spelling, and a repeated name would make the
    /// positional binding ambiguous — both are paste errors rather than
    /// designs, and this is the only place either can be caught.
    #[test]
    fn a_generic_class_declares_distinct_named_parameters() {
        for (name, params) in GENERIC_CLASSES {
            assert!(!params.is_empty(), "{name} declares no type parameter");
            for (i, param) in params.iter().enumerate() {
                assert!(
                    !params[..i].contains(param),
                    "{name} declares `{param}` twice"
                );
            }
        }
    }

    /// Every [`CONSTRUCTORS`] entry names a class this crate registers, and
    /// one that declares slots. The helper builds an instance against that
    /// class's own layout ([`crate::instance::build`]), so a name that is not
    /// in [`CLASSES`] would panic at the first `new`, and a slotless one has
    /// no descriptor at all.
    #[test]
    fn every_constructible_class_is_registered_with_slots() {
        for (name, new) in CONSTRUCTORS {
            let found = class(name).unwrap_or_else(|| panic!("`{name}` is not a registered class"));
            assert!(
                !found.slots.is_empty(),
                "`{name}` is constructible but declares no slots"
            );
            let symbol = new.symbol;
            assert!(
                found.members().all(|member| member.symbol != symbol),
                "`{name}`'s constructor symbol `{symbol}` is also a member's"
            );
            // The seeded signature is looked up under this name, and a `new`
            // is the only thing that reaches it — a row spelled anything else
            // would be a member no call site can write.
            assert_eq!(
                new.name, "constructor",
                "`{name}`'s constructor row is spelled `{}`",
                new.name
            );
        }
    }

    /// No class is registered twice — [`class`] returns the first match, so a
    /// duplicate would silently hide every member on the second entry.
    #[test]
    fn no_class_is_registered_twice() {
        let mut names: Vec<&str> = CLASSES.iter().map(|class| class.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    /// No member declares more defaults than it has positional parameters —
    /// [`CoreMethod::defaults`] is aligned to the end of
    /// [`CoreMethod::positional`], so a longer slice has nowhere to align to
    /// and would make the required count underflow.
    #[test]
    fn no_member_declares_more_defaults_than_parameters() {
        for class in CLASSES {
            for method in class.members() {
                assert!(
                    method.defaults.len() <= method.positional().len(),
                    "{}::{} declares {} defaults for {} positional parameters",
                    class.name,
                    method.name,
                    method.defaults.len(),
                    method.positional().len()
                );
            }
        }
    }

    /// ADR 0063 R2, mechanically: at most one options bag per member, always
    /// last, never empty, and never nested inside another type. Every one of
    /// those is load-bearing — [`CoreMethod::options`] reads only the last
    /// parameter, and `nvs_types::core_lib` synthesizes exactly one
    /// `ConstArg::Options` entry from it.
    #[test]
    fn an_options_bag_is_last_and_never_empty() {
        for class in CLASSES {
            for method in class.members() {
                for (index, param) in method.params.iter().enumerate() {
                    let CoreTy::Options(options) = param else {
                        continue;
                    };
                    assert_eq!(
                        index,
                        method.params.len() - 1,
                        "{}::{} puts its options bag at {index} of {} parameters",
                        class.name,
                        method.name,
                        method.params.len()
                    );
                    assert!(
                        !options.is_empty(),
                        "{}::{} declares an empty options bag",
                        class.name,
                        method.name
                    );
                    for option in *options {
                        assert!(
                            option.name.starts_with(|c: char| c.is_ascii_lowercase()),
                            "{}::{}'s option `{}` is not camelCase",
                            class.name,
                            method.name,
                            option.name
                        );
                        assert!(
                            !matches!(option.ty, CoreTy::Options(_)),
                            "{}::{}'s option `{}` nests a second bag",
                            class.name,
                            method.name,
                            option.name
                        );
                    }
                }
            }
        }
    }

    /// [`CoreTy::Variadic`], mechanically: at most one per member, always the
    /// last parameter, never beside an options bag, and never nested inside
    /// another type. Every one of those is load-bearing — [`CoreMethod::variadic`]
    /// reads only the last parameter, and `nvs_ir::lower::lower_call_args`
    /// collects exactly one trailing array from it.
    #[test]
    fn a_variadic_tail_is_last_and_alone() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Variadic(_) => true,
                CoreTy::Array(elem) | CoreTy::Nullable(elem) | CoreTy::Iterated(elem) => {
                    nests_one(elem)
                }
                CoreTy::Union(members) => members.iter().any(nests_one),
                CoreTy::Options(options) => options.iter().any(|option| nests_one(&option.ty)),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.members() {
                assert!(
                    !nests_one(&method.return_ty),
                    "{}::{} returns a variadic, which is a parameter shape",
                    class.name,
                    method.name
                );
                for (index, param) in method.params.iter().enumerate() {
                    if let CoreTy::Variadic(elem) = param {
                        assert_eq!(
                            index,
                            method.params.len() - 1,
                            "{}::{} puts its variadic tail at {index} of {} parameters",
                            class.name,
                            method.name,
                            method.params.len()
                        );
                        assert!(
                            !nests_one(elem),
                            "{}::{} nests a variadic inside its variadic",
                            class.name,
                            method.name
                        );
                        continue;
                    }
                    assert!(
                        !nests_one(param),
                        "{}::{} nests a variadic inside parameter {index}",
                        class.name,
                        method.name
                    );
                }
                assert!(
                    method.variadic().is_none() || method.options().is_none(),
                    "{}::{} declares both a variadic tail and an options bag, which no call \
                     site could tell apart",
                    class.name,
                    method.name
                );
            }
        }
    }

    /// [`CoreMethod::written`] is the one place a call site's `<...>` order
    /// comes from, so the traversal it does — parameters left to right, then
    /// the return type, a repeat counted once — is held here rather than
    /// re-derived by a reader.
    #[test]
    fn a_written_type_parameter_is_ordered_and_deduplicated() {
        const METHOD: CoreMethod = CoreMethod {
            name: "sample",
            params: &[
                CoreTy::Array(&CoreTy::Written("K")),
                CoreTy::Written("K"),
                CoreTy::Options(&[CoreOption {
                    name: "spare",
                    ty: CoreTy::Nullable(&CoreTy::Written("V")),
                    default: Const::Null,
                }]),
            ],
            defaults: &[],
            return_ty: CoreTy::Written("R"),
            symbol: "nvs_core_sample",
            doc: None,
        };
        assert_eq!(METHOD.written(), vec!["K", "V", "R"]);
    }

    /// One variable, one binding site. A name declared both
    /// [`CoreTy::Written`] and [`CoreTy::Var`] in the same member would have
    /// the call site and the arguments each claiming it, and
    /// `nvs_types::generics`' "first binding wins" rule would settle that by
    /// accident rather than by decision.
    #[test]
    fn a_variable_is_written_or_inferred_but_never_both() {
        fn inferred(ty: &CoreTy, found: &mut Vec<&'static str>) {
            match ty {
                CoreTy::Var(name) | CoreTy::CallableTo(name) | CoreTy::CallableShapeTo(name) => {
                    found.push(name);
                }
                CoreTy::Array(inner)
                | CoreTy::Nullable(inner)
                | CoreTy::Variadic(inner)
                | CoreTy::Iterated(inner) => {
                    inferred(inner, found);
                }
                CoreTy::Union(members) => members.iter().for_each(|m| inferred(m, found)),
                CoreTy::Options(options) => {
                    options.iter().for_each(|o| inferred(&o.ty, found));
                }
                _ => {}
            }
        }
        for class in CLASSES {
            for method in class.members() {
                let mut names = Vec::new();
                method.params.iter().for_each(|p| inferred(p, &mut names));
                inferred(&method.return_ty, &mut names);
                for written in method.written() {
                    assert!(
                        !names.contains(&written),
                        "{}::{} declares `{written}` both written and inferred",
                        class.name,
                        method.name
                    );
                }
            }
        }
    }

    /// A union is legal in either direction, and as an **option's** type only
    /// where `null` is none of the values it admits — see [`CoreTy::Union`],
    /// which owns why: a bag flattens to one ABI argument per option, the
    /// [`Const`] an omitted one passes has no union-shaped spelling, so the
    /// default is [`Const::Null`] and the helper reads `Tag::Null` for "not
    /// given". A member that could itself be `null` would collide with that
    /// sentinel, so [`CoreTy::Nullable`] is refused both as a whole option
    /// type and inside one — and so is [`CoreTy::Mixed`], which admits `null`
    /// without spelling it.
    #[test]
    fn a_union_option_excludes_null() {
        for class in CLASSES {
            for method in class.members() {
                for member in method.options().unwrap_or(&[]) {
                    let closed = match member.ty {
                        CoreTy::Nullable(_) => false,
                        CoreTy::Union(members) => {
                            members
                                .iter()
                                .all(|one| !matches!(one, CoreTy::Nullable(_) | CoreTy::Mixed))
                                && matches!(member.default, Const::Null)
                        }
                        _ => true,
                    };
                    assert!(
                        closed,
                        "{}::{}'s option `{}` is a union that either admits `null` or does \
                         not default to it, so an omitted option and a written one would \
                         reach the helper as the same argument",
                        class.name, method.name, member.name
                    );
                }
            }
        }
    }

    /// The members that still owe ADR 0088 § 2 a classification, frozen at the
    /// size the gate below landed at.
    ///
    /// **Only deletions.** A member reaches this list once, when the gate is
    /// written, and leaves it when its parameters are classified;
    /// `every_member_parameter_carries_a_qualifier_classification` fails on a
    /// member that is here *and* classified, so an entry cannot go stale and a
    /// new member cannot be added to it.
    const UNCLASSIFIED: &[(&str, &str)] = &[
        ("Core\\Uuid", "parse"),
        ("Core\\Uuid", "tryParse"),
        ("Core\\Hash", "of"),
        ("Core\\Hash", "hmac"),
        ("Core\\Hash", "equals"),
        ("Core\\Hash\\Stream", "update"),
        ("Core\\Router", "url"),
        ("Core\\Router", "urlAbsolute"),
        ("Core\\Csv", "parse"),
        ("Core\\Csv", "format"),
    ];

    /// ADR 0088 § 2: every `string`/`bytes` parameter of a `Core` member
    /// carries a qualifier classification, and a member that ships without one
    /// fails this crate's own suite rather than merely a review. The default an
    /// unclassified parameter gets is *refusal*, so a forgotten mark costs a
    /// program a call it should have been able to make — never a security
    /// event, which is the whole of that section.
    ///
    /// **A ratchet, in both directions**, exactly as
    /// `every_core_class_has_a_conformance_floor_of_three` is: an unclassified
    /// member missing from [`UNCLASSIFIED`] fails, and a classified member
    /// still named in it fails too. The list only shrinks.
    ///
    /// **What counts as a `string`/`bytes` parameter**: the parameter's own
    /// type, and the same type under [`CoreTy::Nullable`],
    /// [`CoreTy::Variadic`], [`CoreTy::Union`], or as one option of a
    /// [`CoreTy::Options`] bag — every position where the *argument* the call
    /// writes is itself a string. Under [`CoreTy::Array`] or
    /// [`CoreTy::Iterated`] the argument is a container and the string is its
    /// element type; ADR 0088 § 2 is written about the parameter, and
    /// classifying an element type would be a claim about flow through a
    /// container that no member makes yet.
    #[test]
    fn every_member_parameter_carries_a_qualifier_classification() {
        fn unclassified(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Str | CoreTy::Bytes => true,
                CoreTy::Nullable(inner) | CoreTy::Variadic(inner) => unclassified(inner),
                CoreTy::Union(members) => members.iter().any(unclassified),
                CoreTy::Options(options) => options.iter().any(|option| unclassified(&option.ty)),
                _ => false,
            }
        }

        let mut owing = Vec::new();
        for class in CLASSES {
            for method in class.members() {
                if method.params.iter().any(unclassified) {
                    owing.push((class.name, method.name));
                }
            }
        }

        let listed = |class: &str, member: &str| {
            UNCLASSIFIED
                .iter()
                .any(|(one, other)| *one == class && *other == member)
        };
        let missing = owing
            .iter()
            .filter(|(class, member)| !listed(class, member))
            .map(|(class, member)| format!("        ({class:?}, {member:?}),"))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "{} member(s) have an unclassified `string`/`bytes` parameter and are not named in \
             UNCLASSIFIED. Classify them -- `CoreTy::Text(Qual::…)` / `CoreTy::Blob(Qual::…)` in \
             the row -- or, if this gate is being re-frozen, these are the lines:\n{}",
            missing.len(),
            missing.join("\n")
        );

        for (class, member) in UNCLASSIFIED {
            assert!(
                owing
                    .iter()
                    .any(|(one, other)| one == class && other == member),
                "{class}::{member} carries its classification and is still named in \
                 UNCLASSIFIED; delete that line, because the list only shrinks"
            );
        }
    }

    /// A [`CoreTy::Nullable`] wraps something a `null` can actually widen a
    /// type of: never a second nullable, which the interner would collapse
    /// into the first, and never `void`, which is a return-position marker
    /// rather than a type a value can have.
    #[test]
    fn a_nullable_wraps_something_that_can_be_null() {
        fn check(ty: &CoreTy, what: &str) {
            match ty {
                CoreTy::Nullable(inner) => {
                    assert!(
                        !matches!(**inner, CoreTy::Nullable(_) | CoreTy::Void),
                        "{what} nests {inner:?} inside a nullable"
                    );
                    check(inner, what);
                }
                CoreTy::Array(elem) | CoreTy::Variadic(elem) | CoreTy::Iterated(elem) => {
                    check(elem, what);
                }
                CoreTy::Union(members) => {
                    for member in *members {
                        check(member, what);
                    }
                }
                CoreTy::Options(options) => {
                    for option in *options {
                        check(&option.ty, what);
                    }
                }
                _ => {}
            }
        }
        for class in CLASSES {
            for method in class.members() {
                let what = format!("{}::{}", class.name, method.name);
                check(&method.return_ty, &what);
                for param in method.params {
                    check(param, &what);
                }
            }
        }
    }

    /// [`CoreTy::CallableTo`] and [`CoreTy::CallableShapeTo`] are *binding
    /// sites*, so each only means anything as a whole parameter: nested in an
    /// array, a union or an option either would name a variable nothing ever
    /// binds, and in return position it would name one at the moment it is
    /// meant to be read.
    #[test]
    fn a_callback_result_type_is_only_ever_a_whole_parameter() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::CallableTo(_) | CoreTy::CallableShapeTo(_) => true,
                CoreTy::Array(elem)
                | CoreTy::Nullable(elem)
                | CoreTy::Variadic(elem)
                | CoreTy::Iterated(elem) => nests_one(elem),
                CoreTy::Union(members) => members.iter().any(nests_one),
                CoreTy::Options(options) => options.iter().any(|option| nests_one(&option.ty)),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.members() {
                assert!(
                    !nests_one(&method.return_ty),
                    "{}::{} returns a callback result type",
                    class.name,
                    method.name
                );
                for param in method.params {
                    if matches!(param, CoreTy::CallableTo(_) | CoreTy::CallableShapeTo(_)) {
                        continue;
                    }
                    assert!(
                        !nests_one(param),
                        "{}::{} nests a callback result type inside a parameter",
                        class.name,
                        method.name
                    );
                }
            }
        }
    }

    /// The variable a [`CoreTy::CallableTo`] or a [`CoreTy::CallableShapeTo`]
    /// binds is one the member actually reads back — a row naming `U` in the
    /// callback and `V` in the result would type-check every call to `mixed`
    /// with nothing to say why.
    #[test]
    fn a_callback_result_variable_is_mentioned_by_the_return_type() {
        fn mentions(ty: &CoreTy, name: &str) -> bool {
            match ty {
                CoreTy::Var(var) => *var == name,
                CoreTy::Array(elem)
                | CoreTy::Nullable(elem)
                | CoreTy::Variadic(elem)
                | CoreTy::Iterated(elem) => mentions(elem, name),
                CoreTy::Union(members) => members.iter().any(|member| mentions(member, name)),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.members() {
                for param in method.params {
                    let (CoreTy::CallableTo(name) | CoreTy::CallableShapeTo(name)) = param else {
                        continue;
                    };
                    assert!(
                        mentions(&method.return_ty, name),
                        "{}::{} binds `{name}` from its callback but never returns it",
                        class.name,
                        method.name
                    );
                }
            }
        }
    }

    /// A union has at least two members — a one-member union is that member,
    /// and the interner collapses it, so writing one here would be a row that
    /// does not say what it looks like it says.
    #[test]
    fn a_union_has_at_least_two_members() {
        for class in CLASSES {
            for method in class.members() {
                for param in method.params {
                    if let CoreTy::Union(members) = param {
                        assert!(
                            members.len() >= 2,
                            "{}::{} declares a {}-member union",
                            class.name,
                            method.name,
                            members.len()
                        );
                    }
                }
            }
        }
    }

    /// The one member that has a bag today, spelled out — so a paste error
    /// that dropped `step` would fail here rather than only at
    /// `examples/core.nvs`.
    #[test]
    fn range_declares_one_step_option_defaulting_to_one() {
        let range = class(r"Core\Arr")
            .expect(r"Core\Arr is registered")
            .methods
            .iter()
            .find(|method| method.name == "range")
            .expect("range is registered");
        assert_eq!(range.positional().len(), 2);
        let options = range.options().expect("range takes an options bag");
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "step");
        assert!(matches!(options[0].default, Const::Int(1)));
    }

    /// A `Core` enum's name and cases follow ADR 0029's casing rules too —
    /// `PascalCase` for both, since a case is a type-level name (§ 1's
    /// enum-case row), not a member.
    #[test]
    fn every_core_enum_name_and_case_follows_the_casing_rules() {
        for declared in ENUMS {
            assert!(
                declared.name.starts_with(r"Core\"),
                "{} is not under Core",
                declared.name
            );
            assert!(!declared.cases.is_empty(), "{} has no cases", declared.name);
            for (case, _) in declared.cases {
                assert!(
                    case.starts_with(|c: char| c.is_ascii_uppercase()),
                    "{}::{case} is not PascalCase",
                    declared.name
                );
            }
        }
    }

    /// Every [`Const::EnumCase`] default names an enum this crate registers
    /// and a case that enum actually has — the check that keeps a default and
    /// its case from drifting apart, since `nvs_types::core_lib` resolves one
    /// against the other and panics if it cannot.
    #[test]
    fn every_enum_case_default_names_a_real_case() {
        for class in CLASSES {
            for method in class.members() {
                for option in method.options().unwrap_or(&[]) {
                    let Const::EnumCase(name, case) = option.default else {
                        continue;
                    };
                    let declared = core_enum(name).unwrap_or_else(|| {
                        panic!(
                            "{}::{} defaults `{}` to an unregistered enum `{name}`",
                            class.name, method.name, option.name
                        )
                    });
                    assert!(
                        declared.cases.iter().any(|(found, _)| *found == case),
                        "{}::{} defaults `{}` to `{name}::{case}`, which is not a case",
                        class.name,
                        method.name,
                        option.name
                    );
                }
            }
        }
    }

    /// [`Const::Built`] appears as a *constant's* value and nowhere else —
    /// see that variant's own docs for why a fresh instance materialized
    /// inside a borrowed argument list would have no owner.
    #[test]
    fn a_built_constant_is_never_a_default() {
        for class in CLASSES {
            for method in class.members() {
                for option in method.options().unwrap_or(&[]) {
                    assert!(
                        !matches!(option.default, Const::Built { .. }),
                        "{}::{} defaults `{}` to a built instance",
                        class.name,
                        method.name,
                        option.name
                    );
                }
                for default in method.defaults {
                    assert!(
                        !matches!(default, Const::Built { .. }),
                        "{}::{} defaults a parameter to a built instance",
                        class.name,
                        method.name
                    );
                }
            }
        }
    }

    /// Every [`Const::Built`] names a symbol its own class registers, and
    /// every [`CoreTy::Instance`]-typed constant is one — the two halves of
    /// "a constant that is an instance states the call that produces it".
    #[test]
    fn every_built_constant_names_a_member_of_its_own_class() {
        for class in CLASSES {
            for constant in class.constants {
                match constant.value {
                    Const::Built { symbol, .. } => assert!(
                        class.members().any(|method| method.symbol == symbol),
                        "{}::{} is built by `{symbol}`, which that class does not register",
                        class.name,
                        constant.name
                    ),
                    _ => assert!(
                        !matches!(constant.ty, CoreTy::Instance(_)),
                        "{}::{} is typed as an instance but is not built by one of its members",
                        class.name,
                        constant.name
                    ),
                }
            }
        }
    }

    /// An option typed as a `Core` enum names one this crate registers — the
    /// name is resolved rather than declared, so a typo would otherwise intern
    /// a type nothing can ever produce a value of.
    #[test]
    fn every_enum_typed_option_names_a_registered_enum() {
        for class in CLASSES {
            for method in class.members() {
                for option in method.options().unwrap_or(&[]) {
                    if let CoreTy::Enum(name) = option.ty {
                        assert!(
                            core_enum(name).is_some(),
                            "{}::{}'s option `{}` is typed as the unregistered enum `{name}`",
                            class.name,
                            method.name,
                            option.name
                        );
                    }
                }
            }
        }
    }

    /// Every parameter and return type in the registry, flattened — the walk
    /// the two [`CoreTy::EnumCase`] checks below share.
    fn every_type() -> impl Iterator<Item = (String, &'static CoreTy)> {
        CLASSES.iter().flat_map(|class| {
            class.members().flat_map(move |method| {
                let what = format!("{}::{}", class.name, method.name);
                method
                    .params
                    .iter()
                    .chain(std::iter::once(&method.return_ty))
                    .map(move |ty| (what.clone(), ty))
            })
        })
    }

    /// A [`CoreTy::EnumCase`] names an enum this crate registers *and* a case
    /// that enum actually has — [`every_enum_typed_option_names_a_registered_enum`]'s
    /// reason, one level narrower: a typo in the case would intern a type no
    /// expression can ever place against, so every call would report.
    #[test]
    fn every_enum_case_type_names_a_registered_case() {
        fn check(ty: &CoreTy, what: &str) {
            match ty {
                CoreTy::EnumCase(name, case) => {
                    let found = core_enum(name)
                        .unwrap_or_else(|| panic!("{what} names the unregistered enum `{name}`"));
                    assert!(
                        found.cases.iter().any(|(candidate, _)| candidate == case),
                        "{what} names `{name}::{case}`, which is not a case of it"
                    );
                }
                CoreTy::Array(inner)
                | CoreTy::Nullable(inner)
                | CoreTy::Variadic(inner)
                | CoreTy::Iterated(inner) => {
                    check(inner, what);
                }
                CoreTy::Union(members) => {
                    for member in *members {
                        check(member, what);
                    }
                }
                CoreTy::Options(options) => {
                    for option in *options {
                        check(&option.ty, what);
                    }
                }
                _ => {}
            }
        }
        for (what, ty) in every_type() {
            check(ty, &what);
        }
    }

    /// A case type is only ever a member of a union — see [`CoreTy::EnumCase`]
    /// for why a position admitting exactly one case is not a position at all.
    #[test]
    fn an_enum_case_type_only_appears_inside_a_union() {
        for (what, ty) in every_type() {
            assert!(
                !matches!(ty, CoreTy::EnumCase(..)),
                "{what} takes or answers a bare enum-case type"
            );
        }
        for class in CLASSES {
            for method in class.members() {
                for option in method.options().unwrap_or(&[]) {
                    assert!(
                        !matches!(option.ty, CoreTy::EnumCase(..)),
                        "{}::{}'s option `{}` is a bare enum-case type",
                        class.name,
                        method.name,
                        option.name
                    );
                }
            }
        }
    }

    /// A literal type is only ever a member of a union — see
    /// [`CoreTy::IntLiteral`] for why a position admitting exactly one number
    /// is not a position at all.
    #[test]
    fn a_literal_type_only_appears_inside_a_union() {
        for (what, ty) in every_type() {
            assert!(
                !matches!(ty, CoreTy::IntLiteral(_)),
                "{what} takes or answers a bare literal type"
            );
        }
        for class in CLASSES {
            for method in class.members() {
                for option in method.options().unwrap_or(&[]) {
                    assert!(
                        !matches!(option.ty, CoreTy::IntLiteral(_)),
                        "{}::{}'s option `{}` is a bare literal type",
                        class.name,
                        method.name,
                        option.name
                    );
                }
            }
        }
    }

    /// `sort`'s bag spelled out, in ABI order — the one member whose options
    /// are all four kinds at once: two absent-by-default callbacks, an enum
    /// and a `bool`. The order is what
    /// `nvs_ir::lower::Lowering::lower_options_arg` flattens into, so a
    /// reordering here is a silently wrong call rather than a build failure.
    #[test]
    fn sort_declares_its_four_options_in_abi_order() {
        let sort = class(r"Core\Arr")
            .expect(r"Core\Arr is registered")
            .methods
            .iter()
            .find(|method| method.name == "sort")
            .expect("sort is registered");
        assert_eq!(sort.positional().len(), 1);
        let options = sort.options().expect("sort takes an options bag");
        let names: Vec<&str> = options.iter().map(|option| option.name).collect();
        assert_eq!(names, vec!["by", "order", "comparator", "preserveKeys"]);
        assert!(matches!(options[0].default, Const::Null));
        assert!(matches!(
            options[1].default,
            Const::EnumCase(r"Core\Order", "Asc")
        ));
        assert!(matches!(options[2].default, Const::Null));
        assert!(matches!(options[3].default, Const::Bool(false)));
    }

    /// ADR 0029's `SCREAMING_SNAKE_CASE` for every registered constant, and
    /// no name registered twice on one class — [`CoreClass::constant`]
    /// returns the first match, so a duplicate would silently hide the second.
    #[test]
    fn every_registered_constant_follows_the_casing_rules() {
        for class in CLASSES {
            let mut names: Vec<&str> = Vec::new();
            for declared in class.constants {
                assert!(
                    declared
                        .name
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                    "{}::{} is not SCREAMING_SNAKE_CASE",
                    class.name,
                    declared.name
                );
                assert!(
                    declared.name.starts_with(|c: char| c.is_ascii_uppercase()),
                    "{}::{} does not start with a letter",
                    class.name,
                    declared.name
                );
                names.push(declared.name);
            }
            let total = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(
                names.len(),
                total,
                "{} registers a constant twice",
                class.name
            );
        }
    }

    /// A constant's value is of its declared type. The two are written side by
    /// side and nothing else relates them, so this is the only thing standing
    /// between a typo and a `float`-typed `Core\Math::PI` that lowers to an
    /// `int` constant — a wrong program, not a build failure.
    #[test]
    fn every_registered_constant_matches_its_declared_type() {
        for class in CLASSES {
            for declared in class.constants {
                let agrees = matches!(
                    (&declared.ty, &declared.value),
                    (CoreTy::Bool, Const::Bool(_))
                        | (CoreTy::Int, Const::Int(_))
                        | (CoreTy::Uint, Const::Uint(_))
                        | (CoreTy::Float, Const::Float(_))
                        | (CoreTy::Str, Const::Str(_))
                        | (CoreTy::Bytes, Const::Bytes(_))
                        // An instance's own agreement is a different question
                        // — the *symbol* has to be one of this class's
                        // members — and
                        // `every_built_constant_names_a_member_of_its_own_class`
                        // is where it is asked.
                        | (CoreTy::Instance(_), Const::Built { .. })
                );
                assert!(
                    agrees,
                    "{}::{} is typed {:?} and valued {:?}",
                    class.name, declared.name, declared.ty, declared.value
                );
            }
        }
    }

    /// `Core\Math`'s eleven constants, spelled out — spec § 3's own list, and
    /// the only place `PI` being a `float` and `INT_MAX` an `int` is stated
    /// twice on purpose. A row dropped from the registry fails here rather
    /// than only at `examples/numbers.nvs`.
    #[test]
    fn math_registers_the_eleven_constants_the_spec_names() {
        let math = class(r"Core\Math").expect(r"Core\Math is registered");
        let names: Vec<&str> = math.constants.iter().map(|found| found.name).collect();
        assert_eq!(
            names,
            vec![
                "PI",
                "TAU",
                "E",
                "EPSILON",
                "INT_MAX",
                "INT_MIN",
                "UINT_MAX",
                "FLOAT_MAX",
                "FLOAT_MIN",
                "NAN",
                "INFINITY",
            ]
        );
        assert!(matches!(
            math.constant("INT_MAX").map(|found| found.value),
            Some(Const::Int(i64::MAX))
        ));
        assert!(matches!(
            math.constant("UINT_MAX").map(|found| found.value),
            Some(Const::Uint(u64::MAX))
        ));
        assert!(math.constant("Pi").is_none());
    }

    /// A [`CoreTy::Instance`] names a class this crate registers, in every
    /// position a type can appear — the same check
    /// `every_enum_typed_option_names_a_registered_enum` performs for an enum,
    /// and for the same reason: the name is *resolved*, so a typo would intern
    /// a class type nothing can ever produce a value of.
    #[test]
    fn every_instance_type_names_a_registered_class() {
        fn check(ty: &CoreTy, what: &str) {
            match ty {
                CoreTy::Instance(name) => assert!(
                    class(name).is_some(),
                    "{what} names the unregistered class `{name}`"
                ),
                CoreTy::Array(elem)
                | CoreTy::Nullable(elem)
                | CoreTy::Variadic(elem)
                | CoreTy::Iterated(elem) => check(elem, what),
                CoreTy::Union(members) => {
                    for member in *members {
                        check(member, what);
                    }
                }
                CoreTy::Options(options) => {
                    for option in *options {
                        check(&option.ty, what);
                    }
                }
                _ => {}
            }
        }
        for class in CLASSES {
            for method in class.members() {
                let what = format!("{}::{}", class.name, method.name);
                check(&method.return_ty, &what);
                for param in method.params {
                    check(param, &what);
                }
            }
        }
    }

    /// A class's state is reachable and its members have something to read.
    /// Slots nothing can read are dead bytes on every instance, and an instance
    /// member on a class with no slots would be a method with no receiver state
    /// — either is a half-written class rather than a design.
    ///
    /// The one exception is a **handle**: a class whose slots something other
    /// than its own members read, and which the spec writes no member *on*.
    /// Its state is reachable, just not through itself, so it is listed here
    /// by name rather than given a member the spec does not write. Two are:
    /// `Core\Regex\Pattern`, read by `Core\Regex`'s members, and
    /// `Core\Cli\Text`, whose one slot `nvs_runtime::value_to_string` reads
    /// when `echo` writes a captured carrier out ([`crate::cli`]). The third
    /// is `Core\Script\Handle`, whose one slot the lowering of `await` reads
    /// and whose emptiness of members is the whole point of it
    /// ([`crate::script`]).
    #[test]
    fn a_class_with_slots_has_instance_members_and_the_reverse() {
        const HANDLES: &[&str] = &[
            r"Core\Regex\Pattern",
            nvs_runtime::CARRIER_CLI_TEXT,
            crate::script::HANDLE_NAME,
        ];
        for class in CLASSES {
            if HANDLES.contains(&class.name) {
                assert!(
                    !class.slots.is_empty() && class.instance.is_empty(),
                    "{} is listed as a handle but is not one",
                    class.name
                );
                continue;
            }
            assert_eq!(
                class.slots.is_empty(),
                class.instance.is_empty(),
                "{} declares {} slot(s) and {} instance member(s)",
                class.name,
                class.slots.len(),
                class.instance.len()
            );
            let mut names: Vec<&str> = class.slots.to_vec();
            let total = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), total, "{} names a slot twice", class.name);
        }
    }

    /// `Core\Regex\Match`'s four members and two slots, spelled out — the first
    /// `Core`-owned instance, and the one place spec § 5's own list is stated
    /// twice on purpose.
    #[test]
    fn the_first_core_owned_instance_declares_the_four_members_the_spec_names() {
        let found = class(r"Core\Regex\Match").expect(r"Core\Regex\Match is registered");
        assert!(found.methods.is_empty(), "a Match has no static member");
        let names: Vec<&str> = found.instance.iter().map(|found| found.name).collect();
        assert_eq!(names, vec!["group", "groups", "offset", "text"]);
        assert_eq!(found.slots, ["groups", "offset"]);
        // The receiver is implicit, so `text()` declares no parameter at all.
        assert!(found.instance[3].params.is_empty());
    }

    #[test]
    fn a_registered_class_is_found_by_name_and_an_unregistered_one_is_not() {
        assert!(class(r"Core\Arr").is_some());
        assert!(class(r"Core\Nope").is_none());
        assert!(class("Arr").is_none());
    }
}
