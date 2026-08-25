//! The `Core` signature registry: what the compiler resolves a
//! `Core\Class::member(...)` call against.
//!
//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md) is
//! authoritative for every signature; this is that file in the one form a
//! compiler can read. A row here is a *promise* — `mwl-types` seeds its own
//! signature table from [`CLASSES`], so a `Core` call goes through exactly the
//! arity check, the assignability check and the `ResolvedCall` recording a
//! user-declared static call already does, with no second code path.
//!
//! # Why a small type enum rather than a type spelling
//!
//! [`CoreTy`] is a closed enum, not a `&'static str` the compiler re-parses.
//! Two reasons, both structural: a spelling would put a second (partial)
//! parser for MWL's type grammar in the build, and a typo in it would be a
//! *runtime* surprise in `mwl check` rather than a compile error here. The
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
//! `mwl_types::enums` seeds them into the *same* table a declared `enum` goes
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
//! * **A bag is its own type, not an ADR 0036 shape.** `mwl_types::ty::Ty`
//!   has an `Options` variant beside `Shape`, spellable only from here the
//!   way `TypeVar` already is. Reusing `Shape` would need an `optional` flag
//!   on its fields *and* a `?` in the surface type grammar, and would leave an
//!   unknown option accepted — ADR 0036 § 3's width subtyping allows an extra
//!   field on purpose, while a mistyped option name must be an error.
//! * **A bag is always last and always optional**, because every option is.
//!   Its `MethodSig::defaults` entry is a `ConstArg::Options(...)` carrying
//!   each option's own default, synthesized by `mwl_types::core_lib` from the
//!   type itself — so a row never states the bag twice, `MethodSig::required()`
//!   already excludes it, and the arity check needed no change at all.
//! * **A bag flattens at the ABI.** `mwl_ir::lower::lower_call_args` expands
//!   it into one argument per declared option, in the order [`CoreOption`]s
//!   are written here — the literal's value where written, the option's
//!   default where not — so `mwl_core_arr_range` is an ordinary `args: [3]`
//!   helper and no runtime representation of a shape exists. The rejected
//!   alternative was building an `array<mixed>` per call: it allocates on the
//!   common path, and needs a `null`/empty spelling the IR does not have.
//! * **The cost is one restriction:** an options argument must be written as a
//!   shape literal at the call site, or omitted — a diagnostic, never silence.
//!   That is exactly the set of programs that can run today, since
//!   `ExprKind::ObjectLiteral` has no lowering of its own at all.

/// One type in a `Core` member's signature.
///
/// Deliberately smaller than `mwl_types::ty::Ty`: this describes what the
/// *spec* wrote, not what the checker interns. `mwl-types` lowers each of
/// these into its own interner, which is where qualifiers, unions and class
/// identity live.
// No `PartialEq`/`Eq`: [`Self::Options`] carries [`CoreOption`]s, which carry
// [`Const`]s, which carry an `f64` — and there is nothing here to compare
// anyway, since a registry row is matched structurally and interned into
// `mwl_types::ty::Ty` before any consumer asks whether two types are equal.
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
    /// `mwl_runtime::Value::as_decimal` and returns one with
    /// `Value::decimal` — it is a whole `Value` carrying `Tag::Decimal`, so
    /// nothing about the ABI changes for it (`mwl_runtime::decimal`).
    Decimal,
    /// `string`
    Str,
    /// `bytes` — ADR 0009.
    Bytes,
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
    /// at the call by `mwl_runtime::call_closure`, not by this type.
    Callable,
    /// `callable`, plus the name of the type variable its **result** binds —
    /// `U` in `map(array<T> $a, callable $fn): array<U>`.
    ///
    /// The same opaque `callable` at the call site: it constrains nothing a
    /// [`Self::Callable`] parameter does not, and a closure value satisfies it
    /// by ADR 0027 § 2 exactly as before. What it adds is a *binding site* for
    /// a variable that appears at no argument position at all — `U` is the type
    /// of a value the callback produces, which the argument's own type
    /// (`callable`, and opaque) cannot say. `mwl_types::generics` binds it from
    /// the closure literal's recorded return type and owns the one case that
    /// still binds nothing: an argument that is not a written `fn` literal.
    ///
    /// **Parameter position only, and never nested.** It is a declaration of
    /// where a variable comes from, so it means nothing inside an
    /// [`Self::Array`], a [`Self::Union`], a [`CoreOption`] or a return type —
    /// `a_callback_result_type_is_only_ever_a_whole_parameter` holds that.
    CallableTo(&'static str),
    /// A type *variable*, named — `T` in `count(array<T> $a): uint`.
    ///
    /// The spec's `Core\Arr` section states the rule this exists for: "`T` is
    /// a type variable — the stdlib is parametric where user code is not." A
    /// variable is bound by unifying the declared parameter types against the
    /// call's actual argument types and then substituted through the whole
    /// signature; `mwl_types` owns both halves. Nothing user-written can
    /// declare one, which is `docs/agent/loop-goal.md`'s standing decision that
    /// type variables stay compiler-owned.
    Var(&'static str),
    /// A type variable bound from the type argument **written at the call
    /// site**, named — `T` in `decodeAs<T>(string $json): T`.
    ///
    /// The same `mwl_types::ty::Ty::TypeVar` as [`Self::Var`] once lowered,
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
    /// `mwl_runtime::Value` whose tag `mwl-codegen` writes from the argument's
    /// own representation, so a union parameter needs no IR type of its own
    /// and the body decodes by tag; a union *return* lands in the same 16-byte
    /// value, read back as `mwl_ir::ty::Ty::Tagged` — that variant's own doc
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
    /// enum-case expression against it — `mwl_types::expr::literals`'
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
    /// it through `resolve_method`, and `mwl-ir` lowers a value of it to
    /// `Ty::Object`.
    ///
    /// What makes it *`Core`*-owned is the two things [`CoreClass::slots`] and
    /// [`CoreClass::instance`] state: the instance's field slots are
    /// `mwl-stdlib`'s to lay out rather than a program's to declare, and every
    /// method on it is a native helper reached with the receiver in argument
    /// slot 0. So there is no constructor, no property and no subclass — a
    /// program can only receive one from a member that returns it.
    Instance(&'static str),
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
    /// `args: [N]` is a fixed arity, so `mwl_ir::lower::lower_call_args`
    /// collects every argument from this position onward into a fresh
    /// `array<T>` — keys `"0"`, `"1"`, … — and passes that single value. So
    /// `format` is an ordinary `args: [2]` helper whose second slot is a
    /// `Tag::Array`, and the body iterates it the way
    /// [`crate::str`]'s `join` iterates its subject. The rejected alternative
    /// was a second calling convention carrying a count: it would put a
    /// variable-arity path into `mwl-codegen`'s helper emission for one member
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
    /// call site by `mwl_ir::lower::lower_call_args`, exactly as an omitted
    /// positional parameter's default is.
    pub default: Const,
}

/// One optional parameter's default value.
///
/// The registry's counterpart of `mwl_types::defaults::ConstArg`, kept
/// separate for the reason [`CoreTy`] is kept separate from
/// `mwl_types::ty::Ty`: this states what the *spec* wrote, and `mwl-types`
/// translates it into the one representation the checker and `mwl-ir` share.
/// Only the shapes that enum can already emit are expressible — a member whose
/// spec signature defaults to `null` cannot be registered until
/// `mwl_types::defaults` grows that variant, which is exactly the friction
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
    /// `mwl_types::core_lib` resolves it there; `every_enum_case_default_names_a_real_case`
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
    /// holds. `mwl-ir` lowers it to exactly the `InstKind::CoreCall` a written
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
    /// [`crate::symbols`] hands the JIT and what `mwl-ir` records in the
    /// instruction it lowers a call to. Prefixed `mwl_core_` so a `Core`
    /// member is never mistakable for a `mwl_runtime` primitive in a
    /// disassembly.
    pub symbol: &'static str,
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
        CoreTy::Array(inner) | CoreTy::Nullable(inner) | CoreTy::Variadic(inner) => {
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
    /// present in argument slot 0 at the ABI, exactly the way a compiled MWL
    /// method's is. Empty for every namespace class, which is most of them.
    pub instance: &'static [CoreMethod],
    /// One name per field slot an instance of this class holds, in slot order
    /// — the layout `mwl-stdlib` builds an instance against and the helper
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
    crate::math::CLASS,
    crate::regex::CLASS,
    crate::regex::MATCH,
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
    crate::random::CLASS,
    crate::uuid::CLASS,
    crate::hash::CLASS,
    crate::uri::CLASS,
    crate::csv::CLASS,
    crate::validate::CLASS,
];

/// Every `Core` class a program may write `new` on, with the symbol that
/// builds one — `docs/spec/01-core-library.md` § 9's collections and nothing
/// else.
///
/// A roster rather than a synthetic `constructor` row on [`CoreClass`], for
/// the reason [`crate::instance`]'s module docs give: a `Core` class has no
/// member a program resolves here, and `mwl-ir` reads this to lower `new` on
/// one to an ordinary helper call. A name here **must** be in [`CLASSES`] —
/// unlike [`GENERIC_CLASSES`], whose arity is a property of the spec's table
/// rather than of anything on disk — because the helper builds an instance
/// against that class's declared [`CoreClass::slots`].
pub const CONSTRUCTORS: &[(&str, &str)] = &[
    (crate::objmap::NAME, crate::objmap::NEW_SYMBOL),
    (crate::objset::NAME, crate::objset::NEW_SYMBOL),
];

/// The symbol that builds a `class` instance, or `None` when `new` on it is
/// not a thing a program may write — which is every other name.
#[must_use]
pub fn constructor_symbol(class: &str) -> Option<&'static str> {
    CONSTRUCTORS
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, symbol)| *symbol)
}

/// One `Core`-owned enum — [ADR 0010](../../../../docs/adr/0010-enums-are-a-value-type.md)'s
/// closed, named integer type, declared here rather than in MWL source.
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
/// puts every *callable* on a class, and an enum has none. `mwl_types::enums`
/// seeds its own table from this, so `Core\Order::Desc` resolves to an integer
/// constant through exactly the machinery a user-declared `enum` already goes
/// through — the same "seed a table rather than special-case `Core`" rule
/// `mwl_types::core_lib` states for members.
///
/// The spec's § 2 names `SetOn { Values, Keys, Both }` and its § 4 names
/// `Month { January … December }`. Both are deliberately absent: no member
/// takes or answers with either yet, and an entry here is reachable from
/// source the moment it exists — a case a program can write and pass nowhere
/// is surface with no meaning behind it.
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
];

/// Looks a class up by its fully-qualified name.
#[must_use]
pub fn class(name: &str) -> Option<&'static CoreClass> {
    CLASSES.iter().find(|class| class.name == name)
}

/// The closed roster of members whose helper is handed the **class written at
/// the call site**, as an extra leading argument.
///
/// [`CoreTy::Written`] tells the *checker* what a `<...>` list binds; it says
/// nothing to the runtime, because a type argument is erased like every other
/// one ([ADR 0007](../../../../docs/adr/0007-explicit-type-system.md)). A
/// member like `Core\Json::decodeAs<User>` needs more than the erasure: it has
/// to build a `User`, which means reaching that class's
/// `mwl_runtime::ClassDesc` from native Rust.
///
/// A roster rather than a field on [`CoreMethod`] because it is one entry
/// today against two hundred member rows, and a field would be `false` on
/// every one of them. `mwl-ir` reads this to decide whether to emit an
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
/// table. [`mwl_hir::interfaces::type_params`] is the same roster for the
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
];

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
        for (name, symbol) in CONSTRUCTORS {
            let found = class(name).unwrap_or_else(|| panic!("`{name}` is not a registered class"));
            assert!(
                !found.slots.is_empty(),
                "`{name}` is constructible but declares no slots"
            );
            assert!(
                found.members().all(|member| member.symbol != *symbol),
                "`{name}`'s constructor symbol `{symbol}` is also a member's"
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
    /// parameter, and `mwl_types::core_lib` synthesizes exactly one
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
    /// reads only the last parameter, and `mwl_ir::lower::lower_call_args`
    /// collects exactly one trailing array from it.
    #[test]
    fn a_variadic_tail_is_last_and_alone() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Variadic(_) => true,
                CoreTy::Array(elem) | CoreTy::Nullable(elem) => nests_one(elem),
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
            symbol: "mwl_core_sample",
        };
        assert_eq!(METHOD.written(), vec!["K", "V", "R"]);
    }

    /// One variable, one binding site. A name declared both
    /// [`CoreTy::Written`] and [`CoreTy::Var`] in the same member would have
    /// the call site and the arguments each claiming it, and
    /// `mwl_types::generics`' "first binding wins" rule would settle that by
    /// accident rather than by decision.
    #[test]
    fn a_variable_is_written_or_inferred_but_never_both() {
        fn inferred(ty: &CoreTy, found: &mut Vec<&'static str>) {
            match ty {
                CoreTy::Var(name) | CoreTy::CallableTo(name) => found.push(name),
                CoreTy::Array(inner) | CoreTy::Nullable(inner) | CoreTy::Variadic(inner) => {
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
                CoreTy::Array(elem) | CoreTy::Variadic(elem) => check(elem, what),
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

    /// [`CoreTy::CallableTo`] is a *binding site*, so it only means anything
    /// as a whole parameter: nested in an array, a union or an option it would
    /// name a variable nothing ever binds, and in return position it would name
    /// one at the moment it is meant to be read.
    #[test]
    fn a_callback_result_type_is_only_ever_a_whole_parameter() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::CallableTo(_) => true,
                CoreTy::Array(elem) | CoreTy::Nullable(elem) | CoreTy::Variadic(elem) => {
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
                    "{}::{} returns a callback result type",
                    class.name,
                    method.name
                );
                for param in method.params {
                    if matches!(param, CoreTy::CallableTo(_)) {
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

    /// The variable a [`CoreTy::CallableTo`] binds is one the member actually
    /// reads back — a row naming `U` in the callback and `V` in the result
    /// would type-check every call to `mixed` with nothing to say why.
    #[test]
    fn a_callback_result_variable_is_mentioned_by_the_return_type() {
        fn mentions(ty: &CoreTy, name: &str) -> bool {
            match ty {
                CoreTy::Var(var) => *var == name,
                CoreTy::Array(elem) | CoreTy::Nullable(elem) | CoreTy::Variadic(elem) => {
                    mentions(elem, name)
                }
                CoreTy::Union(members) => members.iter().any(|member| mentions(member, name)),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.members() {
                for param in method.params {
                    let CoreTy::CallableTo(name) = param else {
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
    /// `examples/core.mwl`.
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
    /// its case from drifting apart, since `mwl_types::core_lib` resolves one
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
                CoreTy::Array(inner) | CoreTy::Nullable(inner) | CoreTy::Variadic(inner) => {
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
    /// `mwl_ir::lower::Lowering::lower_options_arg` flattens into, so a
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
    /// than only at `examples/numbers.mwl`.
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
                CoreTy::Array(elem) | CoreTy::Nullable(elem) | CoreTy::Variadic(elem) => {
                    check(elem, what)
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

    /// A class's state is reachable and its members have something to read.
    /// Slots nothing can read are dead bytes on every instance, and an instance
    /// member on a class with no slots would be a method with no receiver state
    /// — either is a half-written class rather than a design.
    #[test]
    fn a_class_with_slots_has_instance_members_and_the_reverse() {
        for class in CLASSES {
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
