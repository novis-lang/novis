//! The `Core` signature registry: what the compiler resolves a
//! `Core\Class::member(...)` call against.
//!
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) is
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
//! # What the enum covers
//!
//! Exactly what a spec §§ 1–12 signature writes, and [`CoreClass`] exactly the
//! *kind* of member those rows are — [`crate`] § *Every shape a §§ 1–12
//! signature writes can be stated* is that claim's home. Three things a reader
//! looks for in the enum are elsewhere on purpose: a **variadic** parameter is
//! [`CoreTy::Variadic`], read through [`CoreMethod::variadic`]; a class
//! **constant** is [`CoreConst`], a roster on [`CoreClass`] rather than a
//! [`CoreTy`] variant, since a constant has a value and no signature; and a
//! `Core`-owned **instance** is [`CoreTy::Instance`] plus
//! [`CoreClass::instance`] and [`CoreClass::slots`], with [`crate::instance`]
//! the value behind it. Widening the enum for a §§ 13–20 row is a change to
//! this file, which is the friction the section above buys.
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
//! `rule:core-api/shape-rules` R2 makes a
//! trailing options shape (`{step?: int}`) the form of *every* optioned
//! member. It is [`CoreTy::Options`], and the four properties below are what
//! it costs and what it buys — recorded here because the fork is the
//! expensive part, not the code:
//!
//! * **A bag is its own type, not an `rule:types/object-top` shape.** `nvs_types::ty::Ty`
//!   has an `Options` variant beside `Shape`, spellable only from here the
//!   way `TypeVar` already is. Reusing `Shape` would need an `optional` flag
//!   on its fields *and* a `?` in the surface type grammar, and would leave an
//!   unknown option accepted — `rule:types/shape-type`'s width subtyping allows an extra
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
//!
//! A **required, positional** fixed-key shape parameter — `Core\Db::open`'s
//! `Db\Settings`, the first the spec writes — generalises all four, and is
//! `rule:core-api/shape-parameter`'s
//! `CoreTy::Shape`: a list of *arms*, flattened by the same rule, one arm for
//! the ordinary case and two or more for a discriminated union. The bag keeps
//! its own variant rather than being folded into it, because the two differ in
//! *call-site rules* and not in checking — both intern to one type, so the
//! exact-key check, the flatten and `E0453`/`E0454` are written once.

/// `rule:security/unclassified-parameter-refuses-tainted`
/// 's qualifier classification, declared **per parameter** on the
/// `string`/`bytes` parameter it describes.
///
/// Not to be confused with the qualifier a *value* carries: `tainted` and
/// `secret` live on `nvs_types::ty::Ty` and describe an argument. This
/// describes what a member does with one, and on the `tainted` axis there are
/// exactly four answers
/// ([the spec's *How to read an entry*](/docs/spec/01-core-library.md)
/// renders them as the Q column). [`Self::Reveal`] is a fifth variant and not
/// a fifth Q value: it is the `secret` axis's only mark, and § *How a `secret`
/// parameter is spelled* below is why it lives here rather than in a type.
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
/// * `rule:core-api/shape-rules` R11's four grammars — a regex pattern, a `printf` template, a
///   CLDR date pattern and a `Core\Bytes::pack` format — are [`Self::Sink`]
///   wherever they are declared, by `rule:security/sink-predicate`'s own corollary.
/// * [`Self::Launder`] is the default of nothing. A member claims it, and its
///   doc comment names the sink it launders for.
///
/// # How a `secret` parameter is spelled
///
/// **With a mark, not with a type: [`Self::Reveal`]**, and only
/// `rule:core-classes/secret-reveal`'s `Core\Secret` members write it. That ADR spells the member's signature
/// `reveal(secret string, string $reason): string` and leaves open how a row
/// says so; this is that decision, and it is recorded here rather than in an
/// ADR because what it decides is how a *row* is written.
///
/// * **Not a [`CoreTy`].** That type is documented as what the spec wrote and
///   not what the checker interns — `nvs_types::core_lib` lowers
///   [`CoreTy::Text`] to a plain interned `string` and carries the mark beside
///   it as `MethodSig::param_quals`, so every qualifier question a row asks is
///   already asked through [`Qual`]. A `CoreTy::Secret` would be the first
///   qualifier inside a type description, in exchange for one refusal nobody
///   wants: revealing a value that is not `secret` is the identity, so
///   refusing it costs a diagnostic and prevents no exposure.
/// * **Not a pair hard-coded in `nvs_types::expr::quals`.** The checker could
///   name `Core\Secret::reveal` and admit its argument by that name, and that
///   is the one shape AGENTS.md's ordering rules out: an invariant every
///   future contributor has to remember, held nowhere near the row it is
///   about.
/// * **A fifth mark rather than a second meaning for [`Self::Launder`].** The
///   two remove different qualifiers and must not share a spelling.
///   `Core\Regex::quote` launders `tainted` for the pattern sink; quoting a
///   `secret` value into a pattern exposes it exactly as much as not quoting
///   it did, so a `Launder` row that also admitted `secret` would leak at
///   every one of the sites that mark exists to make safe.
///
/// # How the `tainted` escape hatch is spelled
///
/// **With [`Self::Launder`], and with no sixth variant.**
/// `rule:security/launderers-are-sink-named`
/// 's `Core\Taint::assertTrusted(tainted string, string $reason): string` is
/// the one launderer that names no single sink, and
/// `rule:core-classes/db-capabilities` makes it the only way
/// through `Settings.host`, which has no launderer of its own — so it earns a
/// row, and it earns one before anything else it unblocks, because
/// `nvs_types::expr::args`' shape-field diagnostic already advises the call and
/// the name it advises resolves to nothing. Recorded here for the reason the
/// `secret` paragraph above is: what it decides is how a *row* is written.
///
/// * **Not a sixth variant.** A mark decides which qualifier a parameter
///   accepts and whether the answer carries it, and this member's effect is
///   [`Self::Launder`]'s exactly. A variant checking identically would be
///   `Launder` under a second spelling at every site that matches on one —
///   which is not what separates [`Self::Reveal`] from `Launder`, since those
///   two remove *different* qualifiers and the bullet above is why they may
///   not share one.
/// * **What is exceptional here is the contract, not the type rule**, and the
///   contract already has a home: [`Self::Launder`] obliges a doc comment
///   naming the sink it launders for, and this is the row whose doc comment
///   names all of them and argues once why that is allowed.
/// * **So the exception is held on the roster, not on the mark**, the way
///   [`Self::Reveal`]'s two-class roster is held rather than spelled — the
///   `tainted` twin of that test being that `Core\Taint` is the only class
///   whose `Launder` row names no one sink.
/// * **It answers a plain `string`, not a carrier**, by
///   `rule:security/launderer-answers-a-carrier`
///   's predicate: the transform is the identity, so it is idempotent, and
///   a value the developer has just sworn is trusted re-entering a sink is the
///   case that predicate exists to let pass.
// `Hash` because `nvs_types::ty::CoreShapeField` carries one — `rule:core-api/shape-flattens-at-the-abi`'s
// classification lands on the field, and a `Ty` is interned by hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Qual {
    /// A qualified argument yields a qualified result — the overwhelming
    /// majority, and the blank cell in the spec's Q column.
    Contagious,
    /// This parameter's content becomes an instruction something executes, so
    /// it **refuses** a qualified argument. `rule:security/sink-predicate` is the predicate.
    Sink,
    /// The result never carries this argument's qualifier: a `bool`, a count,
    /// a hash of a secret.
    Neutral,
    /// This member removes the qualifier, and its doc comment names the sink
    /// it launders for — `Core\Regex::quote` launders for the pattern sink.
    Launder,
    /// The one mark on the `secret` axis: this parameter **accepts** a
    /// `secret` argument, and the answer does not carry the qualifier —
    /// `rule:core-classes/secret-reveal`'s named escape hatch, which is why the member alongside it
    /// takes a written `$reason`. [`Self::Launder`]'s twin one axis over, and
    /// **two classes may write it and no third**: `Core\Secret`, whose members
    /// are the escape hatch itself, and `Core\Password`, whose `hash` and
    /// `verify` are the one operation that takes a password and answers
    /// something that is deliberately not a password. Every other mark refuses
    /// `secret`, which is the whole of what makes a reveal greppable — and the
    /// roster being two long rather than one is checked, in `nvs-types`'
    /// `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`.
    Reveal,
}

impl Qual {
    /// Whether two marks are the same one.
    ///
    /// The derived `PartialEq` answers this everywhere else; this exists
    /// because [`CoreTy::classification`] is a `const fn` and asks it while
    /// folding a union's arms together, where `==` is not available.
    const fn same_as(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Contagious, Self::Contagious)
                | (Self::Sink, Self::Sink)
                | (Self::Neutral, Self::Neutral)
                | (Self::Launder, Self::Launder)
                | (Self::Reveal, Self::Reveal)
        )
    }
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
    /// `uint` — `rule:types/arithmetic`.
    Uint,
    /// `float`
    Float,
    /// `decimal` — `rule:types/decimal`'s
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
    /// default but a state: `rule:security/unclassified-parameter-refuses-tainted` makes it refuse a `tainted` argument.
    /// A classified `string` parameter is [`Self::Text`]. In return position
    /// this is the only spelling, because a classification describes what a
    /// member does with an argument.
    Str,
    /// `bytes` — `rule:types/bytes`. Unclassified, exactly as [`Self::Str`] is;
    /// [`Self::Blob`] is the classified spelling.
    Bytes,
    /// A `string` parameter carrying [`Qual`], `rule:security/unclassified-parameter-refuses-tainted`'s classification.
    ///
    /// A leaf variant rather than a wrapper around [`Self::Str`] on purpose:
    /// every walk over a [`CoreTy`] in this crate ends its `match` with a
    /// wildcard arm documented as "a variant that carries no nested type
    /// carries no variable either", and a wrapper would have quietly falsified
    /// that in seven places at once.
    Text(Qual),
    /// A `bytes` parameter carrying its classification — [`Self::Text`]'s twin.
    Blob(Qual),
    /// An **isolate entry** —
    /// [ADR 0006](/docs/decisions/0006.md) § *Decision*'s
    /// operand, written as a parameter.
    ///
    /// The spec's column is `string` and a path is what the member reads. What
    /// this adds to [`Self::Text`] is the *other* accepted spelling: a static
    /// method written `Chat::run(...)`, which is a `callable`-typed expression
    /// and would be an ordinary mismatch at a `string` parameter. The two are
    /// told apart by how the operand is **written** and never by its type — a
    /// variable holding the callable that reference produces is refused — so
    /// the rule cannot be a parameter type at all, and this is a mark rather
    /// than a type. `nvs_types::expr::isolate`'s `check_entry` is its one home,
    /// at `spawn script` and at every row marked here alike, and
    /// [`entry_parameter`] is how it finds the argument.
    ///
    /// It therefore interns as `mixed`, exactly as `Core\Debug::dump`'s and
    /// `Core\Serialize::encode`'s parameters do for their own call-site rules:
    /// a declared type admitting one of the two shapes would report half the
    /// rule as a type mismatch before the rule ran, and a union admitting both
    /// would name `callable` as accepted in every message. The `string`
    /// comparison the checker still makes is `check_entry`'s own, which is why
    /// a `tainted` path is refused here with no cell to write [`Qual::Sink`]
    /// in — [`Self::classification`] answers `Sink` for it regardless, because
    /// every entry is one and a row has no way to say otherwise.
    Entry,
    /// `secret bytes` — `rule:security/secret-qualifier`
    /// 's qualifier written into a row's own signature, unclassified, and
    /// [`Self::SecretBlob`]'s twin exactly as [`Self::Bytes`] is
    /// [`Self::Blob`]'s.
    ///
    /// **The first spelling here that carries a qualifier rather than a
    /// classification, and the difference is the whole of what it buys.** A
    /// [`Qual`] says what a member *does with* an argument; this says what the
    /// value *is*. So it is the one of the two that means anything in return
    /// position: `Core\Crypto::generateKey(): secret bytes` hands back a key
    /// the checker will not let a program put in a plain `bytes`, which is a
    /// property no classification of an argument could have produced.
    ///
    /// In parameter position it is a *demand* rather than an admission.
    /// `nvs_types`' assignment relation widens a value onto either qualifier
    /// bit freely and narrows through neither, so a plain `bytes` argument is
    /// accepted here and a `secret bytes` one is too, and nothing is
    /// laundered on the way in — which is why [`crate::crypto`] writes no
    /// [`Qual::Reveal`] and the roster closed by `nvs_types`'
    /// `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`
    /// stays two classes wide.
    ///
    /// [`crate::hash`]'s module doc recorded the gap this closes, and
    /// `Core\Hash::hmac`'s key is the parameter it named as wanting it first.
    SecretBytes,
    /// A `secret bytes` parameter carrying its `tainted`-axis classification —
    /// [`Self::SecretBytes`]'s twin, as [`Self::Blob`] is [`Self::Bytes`]'s.
    /// The two axes are independent, so a row that declares confidentiality
    /// still owes `rule:security/unclassified-parameter-refuses-tainted`'s separate answer about where the value came
    /// from.
    SecretBlob(Qual),
    /// `secret string`, **unclassified** — [`Self::SecretBytes`] on the text
    /// base, and the spelling a *return* takes.
    ///
    /// A [`Qual`] on a parameter says what the member does with that argument,
    /// so the strongest thing a classified spelling can produce is
    /// [`Qual::Contagious`]'s conditional. A member answering this one promises
    /// confidentiality outright — `Core\Cache\Store::getSecret` is the row that
    /// needs it, and what comes back out of a sealed entry is `secret` whatever
    /// the `string` that went in was typed as. [`Self::TaintedStr`] is the same
    /// promise one axis over, and its docs work the reasoning through.
    SecretStr,
    /// `secret string` — [`Self::SecretBlob`] on the text base, and a *demand*
    /// in parameter position for that variant's reason: `nvs_types`'
    /// assignment relation widens onto a qualifier bit and narrows through
    /// none, so a plain `string` argument reaches this parameter and a `secret
    /// string` one does too, with nothing laundered either way.
    ///
    /// The argument that has nowhere else to go is a **password**, which is
    /// `secret` where a key is: `Core\Crypto::deriveKey` takes one and answers
    /// [`Self::SecretBytes`], carrying no octet of it either way.
    /// [`Qual::Reveal`] is the only other spelling that admits a `secret` text,
    /// and that roster is closed at two classes because the mark *removes* the
    /// qualifier — a derivation removes nothing, so the parameter says what the
    /// value **is** rather than what the member is allowed to do to it.
    SecretText(Qual),
    /// `tainted string` — `rule:security/tainted-qualifier`'s
    /// qualifier written into a row's own signature, and
    /// [`Self::SecretBytes`]'s opposite number on the other axis.
    ///
    /// **Return position, where it is a promise rather than an admission.**
    /// A [`Qual`] on a parameter says what the member does with *that
    /// argument*, so the strongest thing it can produce is
    /// [`Qual::Contagious`]'s conditional — a qualified argument yields a
    /// qualified result, and a plain one yields a plain one. That is not what
    /// `rule:security/verification-does-not-launder`
    /// asks for. Claims out of a verified JWT are `tainted` *whatever the
    /// token's own type was*, because a signature proves origin and not safety
    /// for any sink, and a token written as a literal in a test is no safer
    /// than one off the wire. Only a spelling that says what the value **is**
    /// can state that, which is why this variant exists and why
    /// [`Self::Text`] could not have been stretched to cover it.
    ///
    /// It nests: `Core\Jwt::verify`'s answer is
    /// `CoreTy::Array(&CoreTy::TaintedStr)`, one `tainted string` per claim,
    /// which is the only shape that keeps the qualifier on the value a program
    /// actually reaches — there is no `tainted array<T>` in `nvs_types`,
    /// because the qualifier axes are defined over `string` and `bytes` alone.
    ///
    /// In parameter position it means what [`Self::Str`] means and adds
    /// nothing, for the reason [`Self::SecretBytes`] gives: `nvs_types`'
    /// assignment relation widens onto a qualifier bit and narrows through
    /// none, so a row that wrote it there would be documenting a demand no
    /// caller can fail to meet.
    TaintedStr,
    /// `tainted bytes` — [`Self::TaintedStr`] on the other of `rule:types/bytes`'s two
    /// octet types, and return position only for that variant's reason.
    ///
    /// The row that needs it is `Core\Request::bodyStream`, whose element type
    /// this is: a chunk of a request body is exactly as untrusted as the whole
    /// of it, so a member that answered a plain `bytes` would be a launderer —
    /// `body()` writes [`Self::TaintedStr`] over the same octets, and two
    /// readings of one body that disagree about the mark is the one shape
    /// `rule:security/tainted-qualifier`
    /// cannot survive. Its home in the checker is `nvs_types`' `Ty::TaintedBytes`,
    /// which already existed because the language can write the type.
    TaintedBytes,
    /// `secret tainted string` — both qualifiers at once, and the one row that
    /// needs it is `Core\Cli::secret`.
    ///
    /// [`Self::SecretBytes`] and [`Self::TaintedStr`] are the two axes
    /// separately, and this is neither's generalisation: a password typed at a
    /// prompt is confidential *and* came from outside, so
    /// `rule:security/secret-qualifier`'s
    /// five sinks refuse it and
    /// `rule:security/tainted-qualifier`'s
    /// launderers are still what let it reach one. Dropping either half would
    /// be a claim the prompt cannot make —
    /// `rule:tooling/a-prompt-is-a-core-member`
    /// writes the return type with both words for that reason.
    ///
    /// Return position for `Core\Cli::secret`, and **options-bag position** for
    /// `Core\Http\Client`'s body keys, which is where the two words earn
    /// something an ordinary parameter cannot get from them. At an ordinary
    /// parameter a [`Qual`] already decides what the slot admits, so this
    /// spelling would demand what `nvs_types`' assignment relation grants
    /// anyway ([`Self::SecretBytes`]'s docs work that through). Inside a bag
    /// there is no mark to read — `nvs_types::core_lib`'s `qual_of` answers
    /// `None` for an option, and `None` refuses a qualified argument exactly as
    /// [`Qual::Sink`] does — so the **type** is the whole of what says a request
    /// body may carry a credential or something a user sent
    /// (`rule:http-server/an-outbound-request-carries-one-body`).
    SecretTaintedStr,
    /// `secret tainted bytes` — [`Self::SecretTaintedStr`] on the other of
    /// `rule:types/bytes`'s two octet types, and it exists for the same slot:
    /// `Core\Http\Client`'s `body` takes octets as readily as text, and a key
    /// out of `Core\Crypto` or a payload off a request reaches it unlaundered.
    SecretTaintedBytes,
    /// `void`, return position only.
    Void,
    /// `mixed` — `rule:types/grammar`'s one unchecked position.
    Mixed,
    /// `object` — the top of the instance half of the lattice, which
    /// `nvs_types::expr::is_assignable` satisfies with a class instance and a
    /// shape and with nothing else.
    ///
    /// **Not [`Self::Mixed`] with a check in the body.** A member that takes a
    /// structured value takes one that *is* structured, so `Core\Jwt::signObject`
    /// refuses an `int` where the call is written rather than where the token
    /// would have been assembled. The unchecked position is for a member whose
    /// subject is genuinely any value — `Core\Json::encode`'s — and spelling a
    /// structured parameter that way moves a refusal from the checker to a
    /// throw the caller has to catch.
    ///
    /// It carries no classification for the reason it needs none: the
    /// `tainted`/`secret` axes are written on `string` and `bytes`, and an
    /// object is neither. `rule:security/tainted-qualifier` distributes
    /// `tainted {…}` onto the shape's own text fields while checking, so a
    /// claims shape built out of a request reaches this parameter as the
    /// unqualified shape type it already is — and each field keeps the
    /// qualifier it was given.
    Object,
    /// `array<T>`, whose element type is the wrapped one.
    Array(&'static CoreTy),
    /// `callable` — `rule:types/callable-absorbs-closure`'s one closure type,
    /// and the **top** of `rule:types/callable-signature`'s lattice: it says
    /// nothing about the parameters or the result of the closure that
    /// satisfies it, so a call through one is checked argument by argument at
    /// run time by `nvs_runtime::call_closure`, at the cost
    /// `rule:types/unions-and-mixed` gives `mixed`.
    ///
    /// A row writes this where the value is **not a callback the member
    /// calls**: `Core\Attributes::get`'s `$target` is a reference to a
    /// declaration, and what it names has whatever signature it was declared
    /// with. It also stands in a [`CoreOption`], which has had no pass of its
    /// own yet. Those are the two places, and
    /// `every_callback_parameter_declares_its_signature` names them — every
    /// callback a member *invokes* writes [`Self::CallableSig`] instead, so
    /// what it is handed and what it must answer are checked where the call is
    /// written.
    Callable,
    /// A **method reference** — `Mailer::send` written bare at the call site,
    /// which is `rule:testing/interaction-after-the-fact`'s compile-checked
    /// name of a method rather than any value a program can hold.
    ///
    /// The whole of what it is at run time is the method's own name as a
    /// `string`: `nvs_types::core_lib` lowers it to that, and the checker folds
    /// the reference to the name where it is written, so the helper reading it
    /// is handed a `Tag::Str` and nothing about the ABI changes for it.
    ///
    /// **This variant is the only place the spelling is admitted.** A
    /// `Class::name` that names a method rather than a constant is an undefined
    /// constant everywhere else, and stays one: `nvs_types::expr::members`
    /// admits it at exactly the argument positions a row writes this at, which
    /// is what keeps "a method as a value" out of the language while letting a
    /// test name the method it is asserting about.
    MethodRef,
    /// A **shape whose every field is a callable**, plus the name of the type
    /// variable the shape of those callables' *results* binds — `S` in
    /// `Core\Task::all({...}): S`.
    ///
    /// `rule:concurrency/all-answers-a-typed-shape` makes `Task::all`'s answer
    /// a shape with the argument's own field names, each field typed as *that
    /// field's* callable returns — which is the whole reason the member is
    /// worth having, since the uniform alternative answers `array<mixed>` and
    /// every call site then pays a cast. No writable type says that, because
    /// the field names belong to the call site rather than to this row.
    ///
    /// **An ordinary type, unlike the binding sites this replaced.** What it
    /// accepts is `nvs_types::expr::assign`'s assignability relation and what
    /// it binds is `nvs_types::generics::bind`'s own walk over the argument's
    /// fields, so a mismatch is the mismatch every other parameter reports and
    /// a variable holding the shape is as good as a literal —
    /// `rule:types/callable-signature` is what put the field's result type
    /// within a type's reach. A field declaring bare `callable` binds `mixed`
    /// for that field alone.
    ///
    /// **Parameter position only, and never nested.** The variable it binds is
    /// read back by the return type, so it means nothing inside a
    /// [`Self::Array`], a [`Self::Union`], a [`CoreOption`] or a return type —
    /// `a_shape_of_callables_is_only_ever_a_whole_parameter` holds that.
    ShapeOfCallables(&'static str),
    /// A **written callback signature** — `callable(T, string): U` in
    /// `Core\Arr::map(array<T> $a, callable(T, string): U $fn): array<U>`: the
    /// parameter types the member hands the callback, left to right, and the
    /// return type `rule:types/callable-signature` makes mandatory.
    ///
    /// An ordinary *type*, which is what separates it from the two binding
    /// sites above it. It says what a callback receives, so the checker
    /// answers where the call is written what
    /// [`Self::Callable`] leaves to `nvs_runtime::call_closure`'s per-argument
    /// tag test, and `nvs_types::expr::calls`' `check_fn_literal` fills an
    /// unannotated `fn($u) => …`'s parameter from it —
    /// `rule:types/callable-literal-inference`.
    ///
    /// **It nests, and a variable inside it is the member's own.** `T` here is
    /// bound by whatever argument position writes it — the subject's
    /// `array<T>` for `map` — because `nvs_types::core_lib` lowers this to
    /// `Ty::CallableSig` and `nvs_types::generics` rewrites that field-wise
    /// rather than collapsing it. That is what makes the expected type a
    /// closure literal is checked against the *substituted* one, and `$u` a
    /// `User` rather than a `T`.
    CallableSig(&'static [CoreTy], &'static CoreTy),
    /// A type *variable*, named — `T` in `count(array<T> $a): uint`.
    ///
    /// The spec's `Core\Arr` section states the rule this exists for: "`T` is
    /// a type variable — the stdlib is parametric where user code is not." A
    /// variable is bound by unifying the declared parameter types against the
    /// call's actual argument types and then substituted through the whole
    /// signature; `nvs_types` owns both halves. Nothing user-written can
    /// declare one, which is `rule:attributes/call-site-type-argument`'s rule
    /// that type variables stay compiler-owned.
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
    /// that, and `rule:attributes/call-site-type-argument` names the
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
    /// `A|B|...` — `rule:types/grammar`'s union, at least two members.
    ///
    /// Legal in **either** direction. A helper's argument slot is a whole
    /// `nvs_runtime::Value` whose tag `nvs-codegen` writes from the argument's
    /// own representation, so a union parameter needs no IR type of its own
    /// and the body decodes by tag; a union *return* lands in the same 16-byte
    /// value, read back as `nvs_ir::ty::Ty::Tagged` — that variant's own doc
    /// comment owns the representation and what it spends.
    ///
    /// **An option's type, and a null arm decides which constant its omission
    /// passes.** `CoreTy::Options` flattens a bag into one ABI argument per
    /// option and an omitted option passes a [`Const`], which has no
    /// union-shaped spelling — so a union admitting no `null` defaults to
    /// [`Const::Null`], read back by the helper as "not given" exactly as it
    /// already is for the `{by?: callable}` an option cannot otherwise spell,
    /// and a union with a [`Self::Nullable`] member defaults to
    /// [`Const::NeverWritten`] instead, so "omitted" and `{a: null}` stay two
    /// arguments (`rule:core-api/omission-is-not-a-written-null`).
    /// `Core\Validate::isIp`'s `{version?: 4|6}` was the row that wanted a
    /// union here first — [`crate::validate`]'s docs say why the alternative
    /// was worse — and `Core\Arr::column`'s `{indexBy?: int|string}` is the
    /// one that showed the restriction was about `null` rather than about
    /// literals. `a_nullable_option_omits_as_the_never_written_marker` holds
    /// the pairing.
    ///
    /// **Its classification is the one its arms declare.** A union is the one
    /// parameter spelling with no cell of its own, so [`Self::classification`]
    /// folds its members: the marks the arms carry, where they agree, are the
    /// parameter's — `Core\Regex`'s `Pattern|string` is
    /// `rule:security/regex-pattern-is-a-sink`'s sink because its text arm is
    /// written [`Qual::Sink`], and `Core\Compress`'s `bytes|string` is
    /// [`Qual::Contagious`] because both of its arms are. A union whose arms
    /// disagree, or whose text arm is an unclassified spelling, carries no
    /// classification and so refuses a qualified argument
    /// (`rule:security/unclassified-parameter-refuses-tainted`). A mark
    /// therefore goes on the arm that can hold it, which is also where a union
    /// wanting [`Qual::Launder`] writes one.
    Union(&'static [CoreTy]),
    /// **One `int` literal** — `rule:types/literal-types`'s integer atom, whose only use is inside a [`Self::Union`] that
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
    /// `?T` — `rule:expressions/nullable-conversion`'s
    /// nullable, which the spec's own tables write at every member that
    /// answers "absent" (`Core\Arr::first`, `Str::indexOf`, `Path::extension`
    /// — `rule:core-api/shape-rules` R5 makes it the *only* absence spelling).
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
    /// Carries no backing type. `rule:enums/one-backing-type` makes `int` the default and
    /// there is no reason for a `Core` enum to be anything else: nothing
    /// stores one, so the only thing a `uint` backing could buy is a case
    /// past `i64::MAX`.
    Enum(&'static str),
    /// **One case** of a `Core`-owned enum, named by that enum and the case —
    /// `rule:types/enum-case-type`'s narrowed type, whose only use is inside a [`Self::Union`] that
    /// spells out a closed subset.
    ///
    /// `Core\Hash::hmac`'s third parameter is the reason it exists.
    /// [`docs/spec/01-core-library.md`](/docs/spec/01-core-library.md)
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
    /// An instance of a **generic** `Core`-owned class at the arguments
    /// written here — `Core\Db\Rows<Core\Db\Row>`, which is what
    /// `Core\Db\Connection::query` answers with.
    ///
    /// [`Self::Instance`] cannot spell this, and the reason is in its own
    /// docs: a generic class reached through that variant interns at the
    /// class's *own* type variables. That is right for `Core\ObjectSet::union`,
    /// where the receiver's arguments are substituted back in, and wrong for a
    /// member that produces one out of nothing — `query` has no
    /// `Core\Db\Rows` receiver to take a `T` from, so answering the bare class
    /// would leave `$rows->first()` typed at a variable nothing ever binds.
    ///
    /// The arguments are positional against the class's [`GENERIC_CLASSES`]
    /// row, which `every_instance_type_names_a_registered_class` checks the
    /// count against, and each is an ordinary [`CoreTy`] — including a
    /// [`Self::Written`] one, which is how `queryAs<T>` answers `Rows<T>` at
    /// the type its call site wrote. [`CoreMethod::written`] descends in here
    /// for exactly that reason: a `T` invisible to it would make the call
    /// non-generic and the `<T>` a syntax error.
    InstanceAt(&'static str, &'static [CoreTy]),
    /// **Whatever `foreach` accepts**, over the element type wrapped:
    /// `rule:iteration/foreach-subjects`'s
    /// three shapes at once, interned as the union
    /// `array<T>|Iterable<T>|Iterator<T>`. `Core\Arr::from`'s
    /// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
    /// § 2 row is the first to write one, and § 9's collections are the next.
    ///
    /// **A plain `array<T>` satisfies it, and the spec row is written that
    /// way.** `rule:iteration/foreach-subjects` fixes the set of things that can be iterated at
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
    /// ambiguous between "one more argument" and "the bag", and `rule:core-api/shape-rules` R20
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
    /// `rule:core-api/shape-rules` R2's trailing options shape — `{step?: int}`, one
    /// [`CoreOption`] per declared option, in the order the ABI passes them.
    /// See this module's own docs for why it is its own type rather than a
    /// [`CoreTy`] wrapping an `rule:types/object-top` shape.
    ///
    /// Only ever the **last** entry of [`CoreMethod::params`], and never
    /// listed in [`CoreMethod::defaults`] or in [`CoreMethod::names`]: every
    /// option has a default of its own, so the bag itself is optional by
    /// construction rather than by declaration, and its callable name is
    /// [`OPTIONS_NAME`] for every member that has one.
    /// `an_options_bag_is_last_and_never_empty` holds both.
    Options(&'static [CoreOption]),
    /// `rule:core-api/shape-parameter`'s
    /// fixed-key shape parameter — `Core\Db::open`'s `Db\Settings`, the first
    /// one the spec writes. The outer slice is the **arms** and is never
    /// empty: one arm is a plain fixed-key shape, two or more a discriminated
    /// union whose separator falls out of the arms being pairwise disjoint
    /// (`a_shapes_arms_are_pairwise_disjoint`) rather than out of a field
    /// declared to be the discriminant.
    ///
    /// **The union is a property of this one type rather than a
    /// [`Self::Union`] of two shapes**, because `nvs_types::ty::Ty`'s union is
    /// sorted by member `TypeId` while a shape's field order *is* the ABI, so
    /// the interner would reorder the arms underneath it — and because a union
    /// of a shape and an `int` would be a type with no ABI at all and nothing
    /// to refuse it.
    ///
    /// **A whole parameter, never nested**: never a member of a
    /// [`Self::Union`], never a [`CoreOption`]'s or a [`CoreField`]'s own
    /// type, never inside a [`Self::Nullable`], a [`Self::Array`] or a
    /// [`Self::Variadic`]. `a_shape_is_only_ever_a_whole_parameter` holds it,
    /// the way `a_shape_of_callables_is_only_ever_a_whole_parameter` already
    /// holds [`Self::ShapeOfCallables`]'s.
    ///
    /// Unlike a [`Self::Options`] bag it is an **ordinary parameter in every
    /// other respect**: it sits at its own position in [`CoreMethod::params`]
    /// with a name in [`CoreMethod::names`], and carries a
    /// [`CoreMethod::defaults`] entry only if it is itself optional. The bag
    /// keeps its own variant rather than being folded into this one because
    /// the two differ in call-site rules and not in checking.
    ///
    /// It flattens at the call site into one ABI argument per field of the
    /// **arms merged in order, deduplicated by name** (`rule:core-api/shape-flattens-at-the-abi`) — so no
    /// runtime representation of a shape appears anywhere, and `open`'s helper
    /// is an ordinary `args: [12]`.
    Shape(&'static [&'static [CoreField]]),
}

/// The one name a trailing [`CoreTy::Options`] bag is callable by, for every
/// member that has one — `rule:core-api/shape-rules`
/// R2's "the trailing bag by the one name `options`".
///
/// It lives beside the type rather than on the row because it is a property of
/// *being* a bag: a per-row spelling would be 60 copies of one string and a
/// question at every call site about which one this member chose. That is also
/// why [`CoreMethod::names`] carries no entry for it — there is nothing
/// per-row to record.
pub const OPTIONS_NAME: &str = "options";

/// One option inside a [`CoreTy::Options`] bag: its name, its type, and the
/// value a call that leaves it out passes.
///
/// The default is stated here rather than in [`CoreMethod::defaults`] because
/// an option is named, not positional — there is no end-alignment rule that
/// could relate a run of defaults to a set of names, and stating it beside the
/// name is the only arrangement in which the two cannot drift apart.
#[derive(Clone, Copy, Debug)]
pub struct CoreOption {
    /// The option's own name, `camelCase` per `rule:core-api/identifier-casing` — what a call site
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

/// One field of one arm of a [`CoreTy::Shape`]: its name, its type, and
/// whether a call site may leave it out — `rule:core-api/shape-parameter`.
///
/// Modelled on [`CoreOption`] and differing in exactly one field. A bag's
/// option is optional *by construction*, so its default is a bare [`Const`];
/// a shape's field states its own required-ness, so its default is an
/// [`Option<Const>`] where `None` means required. That single difference is
/// why the two are not one type: folding them would put a "required?" question
/// on every option, where the answer is already known.
///
/// A qualifier classification lands **here** rather than on the parameter —
/// `Db\Settings`'s `host` is a [`CoreTy::Text`] at [`Qual::Sink`] because
/// `rule:core-classes/db-capabilities` makes an address a
/// sink, while the shape as a whole classifies nothing.
#[derive(Clone, Copy, Debug)]
pub struct CoreField {
    /// The field's own name, `camelCase` per `rule:core-api/identifier-casing` — what a call site
    /// writes on the left of the `:` in `{driver: Driver::Sqlite}`.
    pub name: &'static str,
    /// Its declared type. Never itself a [`CoreTy::Shape`] —
    /// `a_shape_is_only_ever_a_whole_parameter` holds that — and nullable
    /// exactly where [`Self::default`] is [`Const::NeverWritten`], which
    /// `a_nullable_shape_field_omits_as_the_never_written_marker` holds. A
    /// field that admits `null` and omitted as one would reach the helper as
    /// the same argument whether it was written or not; the marker is what
    /// keeps the two apart
    /// (`rule:core-api/a-nullable-field-omits-as-the-never-written-marker`).
    pub ty: CoreTy,
    /// `None` — **required**. `Some(c)` — omittable, and `c` is the constant
    /// an omitting call site passes, materialized by
    /// `nvs_ir::lower::lower_call_args` exactly as a [`CoreOption::default`]
    /// is.
    pub default: Option<Const>,
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
    /// `rule:core-api/shape-rules` R2 makes every option optional, but the spec writes several
    /// whose type has no "absent" value in it: `Core\Arr::sort`'s
    /// `{by?: callable, comparator?: callable}` are the first two — a
    /// `callable` cannot be a "no callback" callable, and inventing a
    /// do-nothing one would silently change the answer. So the *declared*
    /// type stays what a call site may write, and the default an omitting
    /// call site passes is this: the helper reads `Tag::Null` and takes its
    /// own not-given path.
    ///
    /// This is the only default whose value is not of its option's declared
    /// type, and deliberately: writing the type as `?callable` instead would
    /// make it admit `null`, and an option that admits `null` states its
    /// omission with [`Self::NeverWritten`] — which is the pairing
    /// `a_nullable_option_omits_as_the_never_written_marker` holds, and
    /// the reason the declared type stays what a call site may write.
    Null,
    /// **Not a value**: the never-written marker, which is what an omitting
    /// call site materializes for a **nullable** option or shape field —
    /// `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`.
    ///
    /// The one default a call site cannot also write, and that is the whole of
    /// what it buys: a field admitting `null` reaches the helper under
    /// `nvs_runtime::Tag::Null` when a program wrote one and under
    /// `Tag::Unset` when it wrote nothing, so a member with a removal to offer
    /// can tell *leave this alone* from *clear this*
    /// (`rule:core-api/omission-is-not-a-written-null`). An in-band sentinel —
    /// `""`, a reserved string — is a value the field's own type admits, so
    /// user data can arrive as one by accident; this is a tag the type system
    /// has no spelling for at all.
    ///
    /// It costs nothing: `Tag::Unset` is one more discriminant on a
    /// representation that already carries one, it is not refcounted, and the
    /// omitting call site emits one constant either way
    /// (`rule:core-api/the-bag-abi-is-unchanged`).
    ///
    /// **Paired with the declared type, and the pairing is checked.** This is
    /// admitted exactly where the field admits `null`, and refused everywhere
    /// else — `a_nullable_option_omits_as_the_never_written_marker` and
    /// `a_nullable_shape_field_omits_as_the_never_written_marker` hold both
    /// halves. A non-nullable field keeps [`Self::Null`] and lowers exactly as
    /// it does today.
    NeverWritten,
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
    /// differ in exactly the way `rule:types/bytes` says they do: a `bytes` default carries no UTF-8 promise, so it is
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
    /// the case cannot drift apart: `rule:enums/no-class-machinery` makes a case an integer
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
    /// `Zone::of("UTC")` lowers to — so a constant is still `rule:enums/no-class-machinery`'s
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
/// `rule:core-api/reference-card`'s five fields, as plain static data next to the row they describe,
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
    /// One entry per positional parameter, in [`CoreMethod::params`]'s
    /// order, and then **one per option** of a trailing bag under the
    /// option's own name — a consumer looks a description up by the name the
    /// spec's signature gives the row, and a bag has no name of its own.
    /// [`ParamDoc::shape`] is for a shape-typed parameter with fixed keys,
    /// which an options bag is not ([`CoreTy::Options`] owns why).
    pub params: &'static [ParamDoc],
    /// What the member answers with, beyond the type the row already states.
    pub ret: &'static str,
    /// Every error the member throws, and when.
    pub errors: &'static [ErrorDoc],
}

/// One parameter's name and description.
///
/// A **key**, not the name's home: [`CoreMethod::names`] is where a
/// parameter's name lives, and this must equal it entry for entry —
/// `a_documented_rows_param_docs_agree_with_its_names` holds the two
/// together. The card hangs a description off a name the row already
/// declares, so a documented member cannot end up with two spellings of one
/// parameter and no rule about which a caller writes.
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
    /// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
    /// § 10's tree, so `RuntimeError`, `ParseError`, and not a namespaced
    /// spelling the language has no such class under.
    pub error: &'static str,
    /// When it is thrown, in one sentence.
    pub desc: &'static str,
}

/// One `Core` member.
#[derive(Clone, Copy, Debug)]
pub struct CoreMethod {
    /// The member's own name, `camelCase` per `rule:core-api/identifier-casing`.
    pub name: &'static str,
    /// The `$name` each positional parameter is callable by — one per entry of
    /// [`Self::positional`], in the same order, and **the spelling
    /// [01-core-library.md](/docs/spec/01-core-library.md)'s signature
    /// column writes**.
    ///
    /// `rule:core-api/shape-rules` R2 is the
    /// rule: every `Core` parameter is callable by name under exactly the
    /// rules a user-declared method has, so a name is compatibility surface
    /// and renaming one is a breaking change to the spec. That is what makes
    /// the spec's column the source rather than a convenience — the guard test
    /// `every_registry_rows_names_are_the_specs_signature_column` parses the
    /// same column and holds the two together.
    ///
    /// A trailing options bag has no entry here: its one name is
    /// [`OPTIONS_NAME`], which is why this is aligned to [`Self::positional`]
    /// rather than to [`Self::params`]. A variadic tail keeps its entry, so
    /// the alignment holds for every row, but R2 also says a name never
    /// reaches one — the entry documents the tail, it does not open it.
    pub names: &'static [&'static str],
    /// Each parameter's declared type, positional. `rule:core-api/shape-rules` R1 puts the
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
    /// `rule:core-api/reference-card`'s
    /// seam. Read by `nvs meta --json` and by nothing on the request path;
    /// the runtime dispatches on [`Self::symbol`] and never looks here.
    pub doc: Option<&'static MethodDoc>,
}

impl CoreTy {
    /// This type's `rule:security/unclassified-parameter-refuses-tainted` classification, or `None` for a type that
    /// carries none — every type that is not a `string`/`bytes` *parameter*,
    /// and the unclassified [`Self::Str`]/[`Self::Bytes`] spellings, whose
    /// `None` is the refusal rather than an omission.
    ///
    /// A [`Self::Union`] answers the mark its arms declare, which is the one
    /// place a parameter's classification is not written beside the parameter
    /// itself — that variant's own docs hold the rule and what a disagreement
    /// answers.
    #[must_use]
    pub const fn classification(&self) -> Option<Qual> {
        match self {
            Self::Text(qual)
            | Self::Blob(qual)
            | Self::SecretBlob(qual)
            | Self::SecretText(qual) => Some(*qual),
            // The one spelling whose classification is fixed by the variant
            // rather than written beside it: an entry's content becomes the
            // instruction "execute this file", so `rule:security/sink-predicate` makes every one
            // of them a sink and there is no cell for a row to say otherwise
            // in. See [`Self::Entry`].
            Self::Entry => Some(Qual::Sink),
            Self::Union(members) => Self::union_classification(members, None),
            _ => None,
        }
    }

    /// Whether this is one of the `string`/`bytes` spellings that has no cell
    /// to write a classification in.
    ///
    /// The distinction the union walk needs and the parameter gate makes:
    /// these four *could* carry a mark and do not, which
    /// `rule:security/unclassified-parameter-refuses-tainted` turns into a
    /// refusal, where an `int` or a class instance was never a qualifier
    /// question at all. The leaf list is here because both readers ask.
    const fn is_unclassified_string(&self) -> bool {
        matches!(
            self,
            Self::Str | Self::Bytes | Self::SecretStr | Self::SecretBytes
        )
    }

    /// The mark a union's arms declare, carrying the one found so far.
    ///
    /// An arm with no cell — an `int`, a class instance, an enum case —
    /// contributes nothing, so `Pattern|string` answers what its text arm
    /// says. An arm that could carry a mark and does not makes the whole union
    /// unclassified, and so do two arms that disagree: both answer `None`,
    /// which refuses a qualified argument exactly as [`Qual::Sink`] does, and
    /// a union that means anything else says so on every arm that can hold it.
    const fn union_classification(members: &[Self], found: Option<Qual>) -> Option<Qual> {
        match members {
            [] => found,
            [head, rest @ ..] => {
                if head.is_unclassified_string() {
                    return None;
                }
                match (head.classification(), found) {
                    (None, carried) => Self::union_classification(rest, carried),
                    (Some(mark), None) => Self::union_classification(rest, Some(mark)),
                    (Some(mark), Some(carried)) if mark.same_as(carried) => {
                        Self::union_classification(rest, Some(carried))
                    }
                    _ => None,
                }
            }
        }
    }

    /// This type in the spelling a program writes it — the spec's column, so
    /// `Text(Sink)` is `string` and `Iterated(T)` is the shapes `foreach`
    /// accepts.
    ///
    /// Here rather than in a consumer because more than one of them asks:
    /// `nvs meta --json` prints it as a row's `type`, and `nvs_lsp::completion`
    /// puts it in the detail column of a member offered off a `Core` receiver.
    /// A type has one spelling, and a second copy of this match is how the
    /// documentation and the editor start disagreeing about what a member
    /// takes. A consumer holding an interned type spells it with
    /// `nvs_types::TypeInterner::describe` instead — that is the checker's own
    /// answer for a type it has already resolved, and this is the registry's
    /// for a row nothing has resolved yet.
    ///
    /// The wildcard arm is the one `#[non_exhaustive]` requires, and it is
    /// where a variant this has not learned to spell would show up.
    #[must_use]
    pub fn spelled(&self) -> String {
        match self {
            Self::Bool => "bool".into(),
            Self::Int => "int".into(),
            Self::Uint => "uint".into(),
            Self::Float => "float".into(),
            Self::Decimal => "decimal".into(),
            // `rule:security/isolate-shares-nothing`'s entry is a `string` in the spec's column: the method form
            // it also accepts is a *written shape* rather than a second type, so
            // spelling it as a union here would document a `callable` variable as
            // accepted where `nvs_types::expr::isolate` refuses one.
            Self::Str | Self::Text(_) | Self::Entry => "string".into(),
            Self::Bytes | Self::Blob(_) => "bytes".into(),
            Self::SecretBytes | Self::SecretBlob(_) => "secret bytes".into(),
            Self::SecretStr | Self::SecretText(_) => "secret string".into(),
            Self::TaintedStr => "tainted string".into(),
            Self::TaintedBytes => "tainted bytes".into(),
            Self::SecretTaintedStr => "secret tainted string".into(),
            Self::SecretTaintedBytes => "secret tainted bytes".into(),
            Self::Void => "void".into(),
            Self::Mixed => "mixed".into(),
            Self::Object => "object".into(),
            Self::Array(elem) => format!("array<{}>", elem.spelled()),
            Self::Callable => "callable".into(),
            // Not `string`, which is what it lowers to: the position takes
            // `Mailer::send` and refuses every `string` a program could write,
            // so spelling it as one would document an accepted argument that is
            // refused. See [`Self::MethodRef`].
            Self::MethodRef => "method".into(),
            Self::ShapeOfCallables(_) => "{name: callable(): T, ...}".into(),
            // Spelled as the grammar writes it, because a program can write this
            // one: the parameters in their own order and the mandatory return
            // type after the colon.
            Self::CallableSig(params, ret) => {
                let params: Vec<String> = params.iter().map(Self::spelled).collect();
                format!("callable({}): {}", params.join(", "), ret.spelled())
            }
            Self::Var(name) | Self::Written(name) => (*name).into(),
            Self::Union(members) => members
                .iter()
                .map(Self::spelled)
                .collect::<Vec<_>>()
                .join("|"),
            Self::IntLiteral(value) => value.to_string(),
            Self::Nullable(inner) => format!("?{}", inner.spelled()),
            Self::Enum(name) | Self::Instance(name) => (*name).into(),
            Self::InstanceAt(name, args) => {
                let args: Vec<String> = args.iter().map(Self::spelled).collect();
                format!("{name}<{}>", args.join(", "))
            }
            Self::EnumCase(owner, case) => format!("{owner}::{case}"),
            Self::Iterated(elem) => {
                let elem = elem.spelled();
                format!("array<{elem}>|Iterable<{elem}>|Iterator<{elem}>")
            }
            Self::Variadic(elem) => format!("{} ...", elem.spelled()),
            Self::Options(options) => {
                let fields: Vec<String> = options
                    .iter()
                    .map(|option| format!("{}?: {}", option.name, option.ty.spelled()))
                    .collect();
                format!("{{{}}}", fields.join(", "))
            }
            // Each arm as the grammar writes a shape, arms joined as a union:
            // a field with a default is one a call site may leave out.
            Self::Shape(arms) => {
                let arms: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let fields: Vec<String> = arm
                            .iter()
                            .map(|field| {
                                let optional = if field.default.is_some() { "?" } else { "" };
                                format!("{}{optional}: {}", field.name, field.ty.spelled())
                            })
                            .collect();
                        format!("{{{}}}", fields.join(", "))
                    })
                    .collect();
                arms.join("|")
            }
        }
    }

    /// Whether this type is a `string` or `bytes`, classified or not — the one
    /// place a consumer asks the question without caring which spelling it is
    /// looking at.
    #[must_use]
    pub const fn is_text_like(&self) -> bool {
        matches!(
            self,
            Self::Str
                | Self::Bytes
                | Self::Text(_)
                | Self::Blob(_)
                | Self::SecretBytes
                | Self::SecretBlob(_)
                | Self::SecretStr
                | Self::SecretText(_)
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
        CoreTy::Union(members) | CoreTy::InstanceAt(_, members) => {
            for member in *members {
                collect_written(member, found);
            }
        }
        CoreTy::CallableSig(params, ret) => {
            for param in *params {
                collect_written(param, found);
            }
            collect_written(ret, found);
        }
        CoreTy::Options(options) => {
            for option in *options {
                collect_written(&option.ty, found);
            }
        }
        CoreTy::Shape(arms) => {
            for arm in *arms {
                for field in *arm {
                    collect_written(&field.ty, found);
                }
            }
        }
        _ => {}
    }
}

/// One `Core` class constant — `rule:classes/no-free-functions-or-constants`'s
/// "every constant is a class constant", which is what `Core\Math::PI`
/// replaces PHP's global `M_PI` with.
///
/// A constant is not a member with an arity, so it is a roster of its own on
/// [`CoreClass`] rather than a [`CoreTy`] variant: it has a *value* and no
/// signature, and nothing about it is resolved through the method table.
/// The value reuses [`Const`] — the same enum an omitted option's default is
/// written in — because the two want exactly the same thing, a literal the
/// compiler can materialize at the use site, and `rule:enums/no-class-machinery`'s "inlined at
/// every use site" rule for an enum case is the one this follows too: a
/// `Core` constant has no storage, no descriptor and no address.
#[derive(Clone, Copy, Debug)]
pub struct CoreConst {
    /// The constant's own name, `SCREAMING_SNAKE_CASE` per `rule:core-api/identifier-casing`.
    pub name: &'static str,
    /// Its declared type — what a `var $x = Core\Math::PI;` binding infers.
    ///
    /// A scalar wherever [`Self::value`] is a literal, and a
    /// [`CoreTy::Instance`] wherever it is a [`Const::Built`]:
    /// `Core\Time\Zone::UTC` and `Core\Cli\Color::RED` are constants that are
    /// *objects*, allocated at the use site by the call that variant names. The
    /// pairing is not free-form — `every_registered_constant_matches_its_declared_type`
    /// holds the two together, so a `Built` value under a scalar type is a test
    /// failure rather than a lowering that emits the wrong instruction.
    pub ty: CoreTy,
    /// Its value, inlined wherever the constant is written.
    pub value: Const,
    /// What the constant is, in one sentence of inline markdown — `rule:core-api/reference-card`'s card for a constant, which is this one field because a constant
    /// has a value and no signature. A plain string rather than an
    /// `Option<&'static …>` as [`CoreEnum::doc`] is, since a one-field card
    /// has nothing for a struct to hold: **empty means "not written yet"**,
    /// the same rule [`MethodDoc`] states, and `nvs meta --json` omits it.
    pub desc: &'static str,
}

/// One `Core` domain class — `rule:classes/no-free-functions-or-constants`'s "every callable is a class member,"
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
    /// Its own card, or `None` for a class that landed before classes carried
    /// one — the same seam [`CoreMethod::doc`] is. A completion list shows
    /// the card's `short` beside the class's name, which is the one place a
    /// reader meets a class before its members.
    pub doc: Option<&'static ClassDoc>,
}

/// One class's reference documentation — `rule:core-api/reference-card`'s
/// card for the class itself, above the cards its members carry.
///
/// One field, because a class has no signature: what it is for, in one or two
/// sentences, written for the reader of a completion list who has not opened
/// the class's page yet. The members say what each of them does; this says
/// why the class exists.
#[derive(Clone, Copy, Debug)]
pub struct ClassDoc {
    /// What the class is for, in one or two sentences.
    pub short: &'static str,
}

/// The hand-written intro pages under `docs/reference/core/`, compiled in by
/// `build.rs` as `(class, body)` rows — the one place the pages are read, so a
/// page is compiled in the day it is written and never copied into Rust by
/// hand. [`CoreClass::intro`] reads it; `bun nv reference` and the website
/// keep reading the `.md` files.
mod intros {
    include!(concat!(env!("OUT_DIR"), "/intros.rs"));
}

impl CoreClass {
    /// Looks one of this class's constants up by name.
    #[must_use]
    pub fn constant(&self, name: &str) -> Option<&'static CoreConst> {
        self.constants.iter().find(|found| found.name == name)
    }

    /// This class's intro page — `docs/reference/core/<Class>.md` below its
    /// front matter — or `None` for a class nobody has written one for.
    ///
    /// Shown under the card's `short` by a class hover and at the head of the
    /// class's stub. It is prose in the repository's own voice, rendered as it
    /// is written: the page is the website's, and rewriting one is the page's
    /// business rather than a renderer's.
    #[must_use]
    pub fn intro(&self) -> Option<&'static str> {
        intros::INTROS
            .iter()
            .find(|(name, _)| *name == self.name)
            .map(|(_, text)| *text)
    }
}

impl CoreClass {
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
    // `rule:attributes/structural-retrieval` and `rule:attributes/retrieval-folds-while-checking`'s structural retrieval. Registered like any other class
    // and implemented by nothing — see [`crate::attributes`].
    crate::attributes::CLASS,
    crate::math::CLASS,
    // Beside `Core\Math` because it is the other half of one question: § 3's
    // rounding is over `float`, and `rule:types/arithmetic`'s two named-rounding members
    // are the same decision made exactly. No spec § of its own — the spec's
    // own roster table points at that ADR for the non-operator members of the
    // `decimal` scalar.
    crate::decimal::CLASS,
    // And the other half of ADR 0054: the exact scalar above is the
    // human-magnitude answer, and this is the arbitrary one. Spec § 13's row
    // names the pair together, and `crate::bigint`'s module doc owns why
    // neither subsumes the other.
    crate::bigint::CLASS,
    crate::regex::CLASS,
    crate::regex::MATCH,
    crate::regex::PATTERN,
    crate::json::CLASS,
    crate::encoding::CLASS,
    crate::bytes::CLASS,
    crate::path::CLASS,
    // No spec § of its own, and here beside `Core\Path` because that is where a
    // reader looks for it: § 8's class splits a path lexically and this one
    // reaches the filesystem behind it. What it needs to do that is
    // [`CAPABILITIES`], and `rule:security/capability-check-at-the-door`'s doors are what make it need one.
    crate::io::CLASS,
    // What `Core\IO::lines` answers with, and the whole of spec § 14's
    // `Iterable<string>` — a name for the walk, with no member on it. Its own
    // docs say why it holds the lines rather than streaming them.
    crate::io::LINES,
    // What `Core\IO::walk` answers with — the same shape as `LINES` above and
    // the other of spec § 14's two `Iterable<string>`s. Its own docs say why a
    // tree is a different question from `list`'s directory rather than a second
    // spelling of it.
    crate::io::WALK,
    // Spec § 14's `File` — R14's "an open file is an object and never a
    // `resource`". Its slot holds a key into the request's own table of open
    // descriptors, which its own docs argue for; `Core\Script\Handle` is the
    // same shape and landed first.
    crate::io::FILE,
    // What `Core\IO::stat` answers with — § 14's whole metadata bullet as one
    // value, so a program asking more than one question about a path pays for
    // one syscall. Its own docs argue why it is an instance rather than an ADR
    // 0036 shape, and why the questions it answers are also members of
    // `Core\IO` without that being `rule:core-api/shape-rules` R17's two spellings.
    crate::io::METADATA,
    // `rule:core-classes/process-is-argv-only`'s one way to run another program, and the result it answers
    // with. Beside `Core\IO` because it is the other class that reaches the
    // operating system through a door of its own; its capability row is in
    // [`CAPABILITIES`] alongside that class's.
    crate::process::CLASS,
    crate::process::RESULT,
    crate::process::HANDLE,
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
    // Spec § 11's reproducible generator, which is a *type* rather than a mode
    // the class above can be put into — `crate::random`'s own docs argue the
    // separation, and [`CONSTRUCTORS`] is what makes `new` on it resolve.
    crate::random::SEEDED,
    crate::uuid::CLASS,
    crate::hash::CLASS,
    crate::hash::STREAM,
    crate::uri::CLASS,
    // § 15's link half only — `rule:routing/link-name-and-params-are-checked`'s `url`/`urlAbsolute`. `match` and
    // `methodsFor` answer a request and land with the server; [`crate::router`]
    // owns why, and owns the enum that section's `method` parameter takes.
    crate::router::CLASS,
    // `rule:routing/matched-once-before-the-handler`'s match, which is a `Core\Router` name and so lives in that
    // module — but is produced by `Core\Request::route()` rather than by
    // anything on the class above, because § 1 puts the match on the *request*.
    crate::router::MATCH,
    crate::csv::CLASS,
    // What `Core\Csv::rows` answers with: § 12's streaming read, as a name a
    // return type can write. Its own docs say why it carries the iteration trio
    // itself where `Core\IO\Lines` hands back a cursor — a record that has not
    // been read yet cannot be in a snapshot.
    crate::csv::ROWS,
    // `rule:classes/graph-copy`'s externalizing carrier, and no spec § of its own: the walk
    // it reaches is `nvs_runtime::graph`'s, shared with the `spawn` boundary.
    crate::serialize::CLASS,
    crate::validate::CLASS,
    crate::out::CLASS,
    // § 16. `rule:errors/debug-dump`'s `dump` and `render` only — the coverage, trace and profile members
    // that section also lists are `rule:testing/debug-probes`'s and land at M10.
    crate::debug::CLASS,
    // `rule:testing/assertions-are-typed`'s assertion surface rather than a spec § of its own: testing is a
    // language feature, and `Core\Test` is the same `QName` `#[Test]` names.
    crate::test::CLASS,
    // § 18's in-process request, as the thing one answers with — a class beside
    // `Core\Test` for the reason `Core\Script\ExitReport` sits beside
    // `Core\Script`, and [`crate::test`]'s own doc on it owns why it is a
    // `Core`-owned instance rather than the shape § 18's example writes.
    crate::test::RESPONSE,
    // `rule:testing/an-outbound-call-is-answered-from-a-table`'s record of one
    // outbound call, beside `Core\Test` for the reason the response above is:
    // what a test asserts on is a `Core`-owned instance, and this one is
    // readonly because the call it describes has already been made.
    crate::test::SENT_REQUEST,
    // `rule:concurrency/all-answers-a-typed-shape`'s `Core\Task`, and no spec § of its own either: structured
    // concurrency is a language surface over the `nvs-host` scheduler. The
    // signature is here and the body is a placeholder — [`crate::task`] owns
    // why, and it is the one row of the three that is not compile-time folded.
    crate::task::CLASS,
    // Goal `concurrency`'s item 11, and no spec § of its own either — `rule:concurrency/one-scheduler`'s scope
    // line leaves this type's spelling undecided, so [`crate::channel`]'s
    // module doc is the one home for the surface and for why the queue lives
    // in the instance's own slots rather than in the host.
    crate::channel::CLASS,
    // `rule:security/isolate-shares-nothing`'s isolate handle, and no spec § of its own either — the
    // concurrency *language* surface belongs to `docs/spec/00-overview.md` § 2,
    // which is where `spawn script`'s grammar already is. The one class here
    // with no members at all; [`crate::script`] owns why that is the point.
    crate::script::HANDLE,
    // `rule:core-classes/script-args`'s `Core\Script::args()`, which replaced `rule:security/isolate-shares-nothing`'s `$_ARGS`
    // — the read half of `spawn script`'s `args:` option, and a class beside
    // the handle for the same reason `Core\Time` sits beside `Core\Time\Instant`.
    crate::script::CLASS,
    crate::script::EXIT_REPORT,
    // § 13, and the second row after `Core\Attributes` whose members never
    // run: `rule:programs/implementing` expands `implementing<T>()` while checking, so
    // [`crate::program`] registers a signature and an aborting body.
    crate::program::CLASS,
    // § 13's terminal profile — `rule:tooling/the-terminal-profile-resolves-once`'s four members, which reach the
    // operating system and need no capability for it: [`crate::cli`]'s module
    // docs own why a question about a stream the process already holds grants
    // nothing, and why the rest of § 13 is not here yet.
    crate::cli::CLASS,
    // § 13, and here only because § 12's `Core\Out::capture` answers with it —
    // `rule:security/capture-answers-the-carrier`.
    crate::cli::TEXT,
    // § 13's styling half — `rule:tooling/styling-is-a-value-not-a-grammar`'s two value types, which exist so that
    // the carrier above has something to wear that is not a grammar.
    crate::cli::COLOR,
    crate::cli::STYLE,
    // § 13's in-place output — `rule:tooling/in-place-output-is-a-scoped-live-region`'s two handles, which exist because
    // a region has to have an end for § 8's restoration to be enforceable.
    crate::cli::LIVE,
    crate::cli::PROGRESS,
    // § 13's other half — `rule:tooling/commands-are-compiled`'s members over the table `#[Command]`
    // built while compiling. Beside `Core\Cli` because it answers with that
    // class's carrier; [`crate::command`] owns the page's layout, and
    // `nvs_runtime::commands` owns why the compiled rows cross into the runtime.
    crate::command::CLASS,
    // `rule:config/ini-set-is-core-config-set`, and no spec § of its own: `ini_get`'s family are free
    // functions in PHP, so what replaces them is decided by the configuration
    // format's ADR rather than by the library spec. [`crate::config`] owns why
    // every member of it is four lines long.
    crate::config::CLASS,
    // § 15's environment half — `rule:statements/no-host-populated-variables`'s replacement for `$_ENV` and
    // `getenv`, and beside `Core\Config` because the two are the same operator's
    // two answers to "what was this process started with". The one class here
    // that reaches the operating system and is deliberately *not* in the
    // capability table below: [`crate::env`]'s module doc owns why a
    // process-wide fact the operator chose is not a door, and why every value
    // it hands back is `tainted` instead.
    crate::env::CLASS,
    // § 15's other class that needs no request, and beside `Core\Env` because
    // the two ask the same operator the same kind of question: what was this
    // process started with, and what was it allowed to do. `rule:security/optional-capability-degrades` is what
    // specifies it — the member exists so that a package which declared a
    // capability *optional* has a branch to take — and [`crate::cap`]'s module
    // doc owns why reporting a grant is not widening one.
    crate::cap::CLASS,
    // § 15's third class that needs no request — so far. What is registered is
    // `rule:http-server/the-server-block-is-boot-class`'s `isDraining`, which asks the same kind of question those
    // two do about the *process* rather than about a request; the rest of
    // `Core\Server` is the request's own environment and waits on a served
    // request carrying it. [`crate::server`]'s module doc owns that gap, and
    // why the bit it reads lives in `nvs-runtime`.
    crate::server::CLASS,
    // The other end of that same process fact: § 16's `Core\Signal`, which is
    // what a program runs when the shutdown `isDraining` reports has begun.
    // `rule:core-api/tier-roster` keeps the class to graceful shutdown alone —
    // no `kill`, no `alarm` and no signal number — so it registers one member,
    // and [`crate::signal`]'s module doc is the home of why it holds no reading
    // of the drain of its own.
    crate::signal::CLASS,
    // § 16's other class that needs no request in hand, and the last of this
    // run of them: `Core\Budget`, which answers what *this* request holds
    // rather than what the process does. `rule:observability/memory-is-three-numbers-on-core-budget` is the roster —
    // three readings, no `$real_usage` boolean — and [`crate::budget`]'s module
    // doc owns why the peak is recorded by the allocator rather than sampled
    // here, and why the process's own memory stays on `Core\Os`.
    crate::budget::CLASS,
    // § 15's second request-facing class, and the one every value that came
    // from outside the process arrives through: `rule:statements/no-host-populated-variables`'s replacement for
    // `$_GET`, `$_POST`, `$_COOKIE` and `$_FILES`, of which the request line —
    // `method`, `path`, `query` — is registered. Beside `Core\Server` for the
    // reason that class sits beside `Core\Response`: the two are one request
    // read from two ends. [`crate::request`]'s module doc owns why a program
    // that is answering no request gets a throw rather than an empty answer,
    // and why a verb becomes a `Core\Http\Method` case here and nowhere else.
    crate::request::CLASS,
    // What `Core\Request::mount` answers with: `rule:routing/a-request-reads-its-mount`'s two facts about
    // which mount is serving this request, as a pair rather than as the shape
    // that section spells — [`crate::request`]'s `MOUNT` docs own why a shape
    // has no return-position spelling and why the captures carry the mark.
    crate::request::MOUNT,
    // What `Core\Request::bodyStream` answers with, and the whole of § 15's
    // `Iterable<bytes>` — a name for the walk, with no member on it, exactly as
    // `Core\IO\Lines` is. It is the one `Iterable` in `Core` that is its own
    // iterator rather than a snapshot, and its own docs say why: the next chunk
    // of a request body does not exist yet when the walk is named.
    crate::request::BODY_STREAM,
    // What `Core\Request::files` answers with, and `rule:http-server/an-upload-is-received-only-through-files`'s whole
    // `Iterable<Part>` — a name for the walk, with every member on the part
    // rather than on it, exactly as the body walk above carries none.
    crate::request::FILES,
    // And what that walk yields: `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`'s file part, which is a part iff
    // it declared a `filename`. Three readers of what one upload said about
    // itself, all of them `tainted`, and no `size` — [`crate::request`]'s `PART`
    // docs own why the spec's two marks became three and why there is no
    // fourth reader.
    crate::request::PART,
    // And what the part's own `content()` answers: `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s walk over one
    // upload's bytes. The body walk above with an identity — `crate::request`'s
    // `PART_CONTENT` docs own why a walk over a *part* needs one where a walk
    // over a body does not.
    crate::request::PART_CONTENT,
    // § 15's fourth request-facing class, and the first one that *writes*: ADR
    // 0088 § 4's five body members, of which `text` is registered. Beside
    // `Core\Server` because the two are the same request's two halves, and
    // [`crate::response`]'s module doc owns what a written body is — the bytes
    // are `echo`'s output and the member adds the `Content-Type`.
    crate::response::CLASS,
    // `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    // request-scoped half, immediately after the class whose `stream` is the
    // only thing that produces one. [`crate::response`] owns why the handle has
    // no slots and why there is no member that closes it.
    crate::response::STREAM,
    // Spec § 15's third half of one request: what arrived, what goes back, and
    // what is remembered between the two. `rule:core-api/session-roster`'s roster, of which
    // `start` is the member that talks to the store — [`crate::session`]'s own
    // module doc owns which of § 2's four operations are on disk, why the local
    // tier is unreachable from there rather than merely unselected, and why a
    // presented identifier this store cannot have issued is *absent* rather
    // than a second question about validity.
    crate::session::CLASS,
    // `rule:concurrency/a-connection-is-a-root-isolate` and `rule:concurrency/an-upgrade-is-spawn-shaped`, and no spec § of its own: the spec's roster has no
    // `Core\Socket` row, so that ADR is the whole specification. Last of the
    // request-facing group because it is where a request stops being one — §
    // 1 makes the connection a *root* isolate, so the request that upgraded it
    // ends normally and shares nothing onward. [`crate::socket`] owns why the
    // member answers `void` where the ADR writes a returned response, and why
    // three of § 2's four options are absent rather than accepted.
    crate::socket::CLASS,
    // `rule:concurrency/a-connection-is-a-loop`'s message, immediately after the class whose `receive`
    // is the only thing that produces one.
    crate::socket::MESSAGE,
    // `rule:core-classes/topic`, and beside the class whose `receive` is the only thing a
    // topic delivery arrives through: the bus is the *second* of § 3's two
    // sources, so it is not a facility of its own but the far end of a member
    // already registered above. [`crate::topic`] owns why the subscriber table
    // is per core and holds a weak reference, and why `publish` — § 4's third
    // row — is not here yet.
    crate::topic::CLASS,
    // `rule:concurrency/two-doors-one-isolate`, and beside `Core\Socket` because the two are one model with
    // two doors: the same root isolate, reached through the cell the hand-over
    // needs. [`crate::sse`] owns why that is a second cell rather than a second
    // use of the first, and why this member is offered to every request a
    // server answers where its sibling is offered only to an upgradable one.
    crate::sse::CLASS,
    // `rule:concurrency/a-connection-is-a-loop`'s message on the door with no
    // peer, immediately after the class whose `receive` is the only thing that
    // produces one — the sibling pair two rows up, read against a hand-over
    // that took no socket. [`crate::sse`]'s own doc owns why it is two readers
    // rather than the four beside it and why it is a class of its own.
    crate::sse::MESSAGE,
    // `rule:errors/on-limit`, and no spec § of its own: the escalation ladder's ADR is
    // where this member is specified, because what it registers is a rung of
    // that ladder rather than a library facility. [`crate::fatal`] owns why the
    // closure is held by the request's context and not by the module.
    crate::fatal::CLASS,
    // `rule:errors/log-write`, and beside `Core\Fatal` because the two are one ladder:
    // the rung above every `catch` and the reporting half every rung of it
    // ends at. [`crate::log`] owns why the level enum's integers are syslog
    // severities and why a record reaches the output stream rather than the
    // diagnostic one.
    crate::log::CLASS,
    // `rule:security/launderers-are-sink-named`, and no spec § of its own, for the reason `Core\Secret` below
    // has none: a member that removes `tainted` is a rung of the qualifier's own
    // mechanism rather than a library facility. Registered immediately before
    // its twin because the two are one shape on two axes — one narrow, named,
    // reasoned call each — and [`Qual`]'s doc comment is the home of why this
    // one writes [`Qual::Launder`] and names no single sink.
    crate::taint::CLASS,
    // `rule:core-classes/secret-reveal`, and no spec § of its own: what this class is for is decided
    // by the qualifier's ADR, because a member that removes `secret` is a rung
    // of that mechanism rather than a library facility. The only class that may
    // write [`Qual::Reveal`] — see that variant's own docs — and
    // [`crate::secret`] owns why the `bytes` half is a second name.
    crate::secret::CLASS,
    // § 16's SMTP row, and `rule:programs/framework-core-half`'s transport half. Registered on its own
    // rather than beside `Core\Http` because it shares nothing with it: the
    // endpoint is an operator-named block and not a program-supplied URL, so
    // `rule:http-server/allow-url-pins-the-address`'s launderer is not in the path at all. [`crate::mail`] owns why
    // `mail()`'s fourth argument has no successor here.
    crate::mail::CLASS,
    // `rule:programs/framework-core-half`'s other half of the same row pair, and registered beside
    // `Core\Mail` because the two are what that section adds to `Core`: an
    // operator names the endpoint in one and the disk in the other, and neither
    // takes a host or a path. It is the one class here that declares **no**
    // capability of its own — [`crate::storage`] owns why `fs.*` answering
    // twice would be the bug.
    crate::storage::CLASS,
    // `rule:programs/framework-core-half`'s last row, and the third of the three this stage adds to
    // `Core`. Beside the two above because it completes them and not because
    // it shares anything else: it is the one entry in that table placed by
    // test 4 — data the language already had to carry — rather than by an
    // outside world it waits on. [`crate::cldr`] owns why the rules are a
    // closed roster and why a language outside it is refused.
    crate::cldr::CLASS,
    // § 16, and beside `Core\Secret` rather than in section order because the
    // two are one mechanism: `rule:core-classes/secret-reveal` has exactly two operations that take
    // a `secret` and answer something that is not one, and these are the rows
    // of the second. [`crate::password`] owns the parameters and why there is
    // no argument for them.
    crate::password::CLASS,
    // `rule:core-api/tier-roster`'s "AEAD only", and beside the two above for their reason:
    // this is the class whose `secret bytes` key crosses the same boundary
    // without any member removing the mark, which is what keeps the roster
    // above closed at two. [`crate::crypto`] owns the construction and why
    // there is no cipher argument.
    crate::crypto::CLASS,
    // The public half of an asymmetric key, as the object `rule:core-api/shape-rules` R14 asks
    // for: a key has a lifetime — it is read once, checked once, and then used
    // — so it is a class and not a `bytes` every later member would have to
    // re-validate. Its own doc owns why its slots are a canonical SPKI and a
    // kind rather than the octets the program was handed.
    crate::crypto::PUBLIC_KEY,
    // The private half, and a class for the same R14 reason with a second one
    // beside it: a pair is the only thing in the language a `secret bytes`
    // leaves rather than enters, and an object is what makes `write` the one
    // door it leaves through. Its own doc owns why its slots are the PKCS#8 and
    // the kind, and why there is no cache of parsed keys behind them.
    crate::crypto::KEY_PAIR,
    // `rule:security/protocol-roster`'s first roster entry, and beside `Core\Crypto` because it
    // *is* `Core\Crypto` — [`crate::signed_cookie`] keys the same construction
    // through the same three helpers, with a key ring over it and a
    // cookie-safe spelling around it, so there is one AEAD in this crate and
    // not two. Its own module doc owns which end of the ring is the newest key
    // and why its `open` is the one verification in the language that removes
    // `tainted`.
    crate::signed_cookie::CLASS,
    // `rule:security/protocol-roster`'s second roster entry, and the third caller of the one
    // construction above — [`crate::csrf`] seals a domain tag and a session
    // identifier where the cookie seals a payload, so this crate still holds
    // one AEAD and not three. Its own module doc owns why the session arrives
    // as an argument, why there is no key ring here, and why a class whose
    // whole point is a missing accessor is not `rule:core-api/shape-rules` R17's "reachable two
    // ways" against the cookie above.
    crate::csrf::CLASS,
    // `rule:security/protocol-roster`'s third roster entry, and the one class in this crate that
    // is *not* on the near side of the AEAD above: a one-time code is HMAC by
    // RFC 6238, and [`crate::totp`]'s own module doc is the home of why the
    // algorithm is SHA-1 when nothing else here is, why the window has no
    // widening argument, and why "no replay" is a counter the caller stores
    // rather than state this class keeps.
    crate::totp::CLASS,
    // `rule:security/protocol-roster`'s fourth roster entry, and the only one
    // of the five whose wire format was designed elsewhere, so [`crate::jwt`]
    // is the one class here that reads a field an attacker wrote. Its own
    // module doc is the home of why `alg` is only ever compared, why the
    // expiry is a positional `Duration` rather than a claim, and why a claim
    // comes back as `tainted string` when the cookie above launders.
    crate::jwt::CLASS,
    // The keys the class above verifies another party's token against, beside
    // it for `crate::jwe::KEY`'s reason: it is the value that member takes and
    // nothing else produces one. Every rule about which keys a document may
    // publish is applied at its one member, so a verification is a lookup by
    // `kid` and a signature check — `rule:security/algorithm-comes-from-the-key`
    // is why that is a lookup and never a try, and [`crate::jwt`]'s own module
    // doc is the home of why a key this roster has no use for is skipped where
    // a document that is wrong is refused whole.
    crate::jwt::KEY_SET,
    // The roster's encryption entry, beside `Core\Jwt` because the two read one
    // wire format from opposite ends: a JWS proves who wrote a payload and this
    // hides one. `rule:security/jwe-compact-subset` is the subset — compact
    // serialization, `A256GCM` and nothing else — and [`crate::jwe`]'s own
    // module doc is the home of why the key arrives as a named constructor,
    // what the protected header is allowed to carry, and why a ring holding a
    // password holds exactly one key.
    crate::jwe::CLASS,
    // That constructor itself: four statics, no accessor and no way back out,
    // which is what makes the key-management algorithm a name the caller wrote
    // rather than an inference off a value's type
    // (`rule:security/algorithm-comes-from-the-key`). Registered beside its own
    // class for `Core\Crypto\PublicKey`'s reason — it is the value the class
    // above takes and nothing else produces one.
    crate::jwe::KEY,
    // The roster's detached entry, and the one that is not `Core\Crypto`: a
    // signature is an HMAC over a canonical
    // document, because its payload is meant to be *read* by whoever holds the
    // token where a seal's is meant to be hidden
    // (`rule:core-api/each-door-takes-a-different-thing`). It shares the key
    // ring with the cookie above and nothing else. [`crate::signature`]'s own
    // module doc is the home of the wire format, of the domain byte that stops
    // one ring being replayed across three doors, and of why a verified
    // payload value is text.
    crate::signature::CLASS,
    // `rule:security/launderers-are-sink-named`'s own worked example of a launderer, and here rather than in
    // spec § order because the class beneath it is the one this crate defers to
    // whenever a `tainted` value has to be written into a document: § 5 makes
    // HTML the sink that escapes by default, and this is what it escapes with.
    // No spec § of its own yet — § 20's roster row names the class, and
    // [`crate::html`] owns why `sanitize` and the WHATWG parser are not here.
    crate::html::CLASS,
    // `rule:core-classes/html-auto-escape`'s carrier for that same sink, memberless: `nvs_runtime`
    // already renders it and `nvs_types` already refuses a `tainted` or
    // computed conversion to it, so what this row adds is the registered
    // layout the slot lives in. [`crate::html`] owns why it has no
    // constructor.
    crate::html::MARKUP,
    // § 17's document class, next after `Core\Html` because the two are one
    // subsystem: `rule:core-classes/html-parsing` makes both parsers produce
    // the node family below, so which door parsed a document does not change
    // what a program can do with it. The tree half is what is registered —
    // [`crate::xml`] owns the split and why the streaming half is not a mode of
    // this one.
    crate::xml::CLASS,
    // The node family itself, memberless of statics and produced only by a
    // parse. Beside its class rather than in spec § order for `Core\Html`'s own
    // reason: this is the value the section's first two rows both answer with.
    crate::xml::NODE,
    // The other door onto that family: § 17's stream half, beside the tree half
    // rather than in spec order because the two are one subsystem in two
    // shapes. `rule:core-classes/xml-tree-and-stream` is why it is a class of
    // its own and not a mode on the row above — the family is shared and no
    // operation is, so which shape a program uses is settled at the door.
    crate::xml::READER,
    // The stream half's other door, beside the reader for the reason the
    // reader sits beside the tree: reading a document a node at a time and
    // building one a node at a time are the two halves of one shape, and
    // `rule:core-classes/xml-tree-and-stream` is what keeps neither of them
    // reachable through the other.
    crate::xml::WRITER,
    // § 17's codec class, beside `Core\Html` because that is the section both
    // are rows of. Tier 0 for a reason that is not the other three's:
    // `rule:core-classes/decompression-bound`'s bound is policy, and a policy
    // a program can decline is not one — see [`crate::compress`].
    crate::compress::CLASS,
    // § 17's incremental pair, beside the class whose two openers answer them.
    // Two classes rather than one because an instance carries no qualifier:
    // `rule:security/tainted-sources` makes the decompressing half's `finish`
    // answer `tainted bytes` and the compressing half's answer `bytes`, which
    // one class would have to collapse — [`crate::compress`] is the argument.
    crate::compress::COMPRESSOR,
    crate::compress::DECOMPRESSOR,
    // § 17's detection class, beside the codec class for the reason both are
    // Tier 0: what a program is allowed to conclude about untrusted octets is
    // policy. Its whole knowledge is a compiled-in table of literal
    // signatures — [`crate::mime`] owns why that is not libmagic's rule
    // language, and why nothing here reads a file name.
    crate::mime::CLASS,
    // § 17's archive class, beside the codec class whose bound it applies and
    // the detection class it sits between in that section. Tier 0 for the
    // reason both of those are: a `../` entry, an absolute-path entry, a
    // symlink entry and a bomb are what an archive is *allowed* to contain,
    // which is policy — and [`crate::zip`] owns why the refusals live in the
    // reader rather than in an extraction a caller can decline to use.
    crate::zip::CLASS,
    // `rule:http-server/allow-url-pins-the-address`'s launderer, which is where every outbound URL in the
    // language has to pass through — and the first `rule:security/tainted-qualifier` launderer whose
    // answer is a value rather than a plain string. [`crate::http`]'s own
    // module doc is the home of why that is the whole design, and of which
    // half of the policy lives in the capability instead.
    crate::http::CLASS,
    // The value that launderer answers with. No members at all: a program
    // names it and hands it to the member that connects, and reading the
    // approved address back out is the one operation that would make pinning
    // decorative.
    crate::http::TARGET,
    // Who *this* end is, where the target above is who the other end must be:
    // the certificate chain a call presents when a server asks for one, over
    // goal `webcrypto`'s key pair. One static and no members, since reading the
    // key back out is not what an identity is for.
    crate::http::IDENTITY,
    // `rule:http-server/no-spelling-for-an-unbounded-wait`'s request members, whose URL parameter is the sink `rule:security/outbound-url-is-a-sink`
    // makes it and whose one trailing shape has no spelling for an
    // unbounded wait. Five rows over one bag: the verb is the member's own
    // name, which is what lets § 7 answer "is this retry idempotent" while
    // compiling.
    crate::http::CLIENT,
    // What those five answer with: the status, one body slot read three ways,
    // the header pair, and the session the reply arrived over.
    crate::http::RESPONSE,
    // Which that last member answers with, and the one `Core\Http` reader whose
    // answer is `null` for an ordinary reply: a plain `http` exchange had no
    // session to report. `rule:http-server/a-reply-reports-its-tls-session` is
    // why `verified` is on it at all — a deployment that relaxed trust at one
    // host has no other way to assert that every other call did not.
    crate::http::TLS_INFO,
    // `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`'s
    // answer, which is the reply that is read as it arrives: the same head
    // members over the same map, and four framings of a body that is taken by
    // the first of them to name it rather than read by all four.
    // `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`'s
    // answer, and the one value on this class that outlives the call that made
    // it: a conversation the program holds until its own task ends. What it
    // answers with is `Core\Socket\Message` above, because one RFC 6455 frame
    // gets one shape whichever end of the wire it arrived at.
    crate::http::socket::SOCKET,
    crate::http::stream::STREAM,
    // One server-sent event, which is what the first of those four frames.
    crate::http::stream::EVENT,
    // The three walks those framings answer with. Three classes rather than one
    // because `CoreTy::Iterated` is parameter position only, so an
    // `Iterable<T>` return is a named class and [`ITERABLES`] is where its `T`
    // is declared — an event, a line and a chunk are three `T`s over one
    // implementation.
    crate::http::stream::EVENTS,
    crate::http::stream::LINES,
    crate::http::stream::CHUNKS,
    // `rule:http-server/an-outbound-request-carries-one-body`'s part, which is
    // the arm of `body` and of a `multipart` field that names a file instead of
    // holding one. Two constructors and no member, for the reason
    // [`crate::http::PART`] states.
    crate::http::PART,
    // § 16's socket half, beside the HTTP client because the two are the same
    // door at two heights: `rule:core-classes/net-one-api-three-transports`
    // replaces `socket_*`, `stream_socket_*` and `fsockopen` with one class over
    // three transports, and `rule:security/net-listen-is-a-separate-grant-from-net-connect`
    // is why reaching outward and binding are two grants rather than one
    // widened. Three of the five entry points so far; [`crate::net`]'s module
    // doc is the home of what each one asks.
    crate::net::CLASS,
    // The connected transport those entry points answer with — a `Read` and a
    // `Write` shaped like every other stream in the language, whose one slot
    // holds a key into the request's own table of open sockets. `Core\IO\File`
    // is the same shape over a descriptor and landed first.
    crate::net::STREAM,
    // The accepting half, and the reason `Core\Net` has five entry points
    // rather than six: taking the next connection off a socket the program
    // already holds is a member on that socket, not another way to open one.
    crate::net::LISTENER,
    // The third transport, which answers neither `Read` nor `Write` because a
    // datagram socket has no stream to read. It is the one handle class here
    // whose members do not all reach what its bind already granted:
    // `rule:security/net-address-policy` asks the outbound grant at the send,
    // because that is where a datagram's address is named at all.
    crate::net::DATAGRAM,
    // And what a receive answers with — the octets and the endpoint they came
    // from, as one object because Novis has neither out-parameters nor tuples
    // and `stream_socket_recvfrom`'s row has to be answered by something.
    crate::net::MESSAGE,
    // § 16's other half: what the host itself is, as five facts about the
    // process asking rather than about anything it can reach. It is the one
    // class in this stretch with no row in [`CAPABILITIES`], and
    // [`crate::os`]'s module doc is the home of why declaring `None` five times
    // would have been the wrong claim rather than a cautious one.
    crate::os::CLASS,
    // `rule:concurrency/cross-request-state-is-explicit`'s two tiers, as the two members that hand back a store — the
    // sanctioned exception to `rule:security/no-cross-request-state`'s closed door on cross-request
    // state, and the one place a value outlives the request that made it.
    crate::cache::CLASS,
    // What those two answer with: one class for both tiers, because a tier is a
    // destination and not a different operation. [`crate::cache`]'s module doc
    // is the home of why an entry is a byte payload rather than a live graph.
    crate::cache::STORE,
    // What a `getSecret` fill answers with, and the one shape a lifetime a
    // fetch learned can reach this store in. [`crate::cache`]'s own class doc
    // is the home of why it answers nothing back.
    crate::cache::SECRET_ENTRY,
    // `rule:core-classes/ratelimit-two-members`'s limiter for what only the application knows — per account, per
    // tenant — with edge and flood limiting left to the proxy that owns them.
    // Its state is the same shared store `Core\Cache::shared` names, because a
    // deployment has one.
    crate::ratelimit::CLASS,
    // § 3's decision: what was decided, what against, and the exact wait when it
    // was refused. Four readers rather than four readonly properties, for the
    // reason [`CoreTy::Instance`] states — a `Core` instance has no property a
    // program can reach.
    crate::ratelimit::DECISION,
    // `rule:observability/metrics-three-members`'s three verbs, beside the
    // limiter because they are the two halves of one question a deployment
    // asks: how much a caller may have, and how much everything is doing. Its
    // registry is `nvs_runtime::metrics`' and not this crate's, for the reason
    // that module's own doc gives.
    crate::metrics::CLASS,
    // `rule:tooling/reflection-and-source-parsing-are-core-features`'s read-only introspection, whose members are the door onto a
    // description and nothing else — a program can reach a member it may not
    // call only through the description, and § 2 makes that reach face the
    // ordinary check.
    crate::reflect::CLASS,
    // What `forObject` answers with: the described class, as the questions that
    // need no argument and the acting members that take the value back. Readers
    // rather than properties, for the reason [`CoreTy::Instance`] states — a
    // `Core` instance has no property a program can reach.
    crate::reflect::CLASS_INFO,
    // One row of that description's method roster. A class of its own rather
    // than a `{name, public}` shape, because a shape has no member to hang the
    // next question off and the roster ADR 0019 § 1 names is a family of
    // descriptions, not of records.
    crate::reflect::METHOD_INFO,
    // The same row one member over, for the description's other roster. Its
    // own class for [`crate::reflect::METHOD_INFO`]'s reason, and the two are
    // separate classes rather than one `MemberInfo` because what a row carries
    // past its name and its bit differs: a parameter count on one side, a
    // declared type on the other.
    crate::reflect::PROPERTY_INFO,
    // One row of a method row's own roster, hanging off `MethodInfo` rather
    // than off the description: a parameter belongs to the member that declares
    // it, and a class-wide list would have nothing to align to.
    crate::reflect::PARAMETER_INFO,
    // The description's third roster, and its own class for the two above's
    // reason: what a row carries past its name and its bit is a fact no other
    // row has — whether the declaration folded to a value at all.
    crate::reflect::CONSTANT_INFO,
    crate::reflect::ATTRIBUTE_INFO,
    // `rule:enums/reflection`'s description of the one type that has no
    // descriptor: an enum case at run time is the integer behind it, so there
    // is no value to describe and no `forObject` twin — the name is the only
    // door, and what it opens onto is the shape the compiler carried down.
    crate::reflect::ENUM_INFO,
    // § 3's other half of the same ADR: the compiler's own parser, reached at
    // run time. One member, because parsing is one question.
    crate::ast::CLASS,
    // The tree `parse` answers with — a kind and the nodes under it, and
    // nothing that reaches back into execution, which is what makes § 3's
    // inertness structural rather than promised.
    crate::ast::NODE,
    // § 18, and `rule:core-classes/db-one-api` for every semantic behind it. `open` is the one entry
    // point still missing, and [`crate::db`]'s known gaps own why: what it
    // needs is a `CoreTy` for a shape parameter, not a body.
    crate::db::CLASS,
    // What `connect` answers with, and `Core\Db\Queryable`'s own home: `rule:classes/no-traits`
    // has `Transaction` delegate the interface to its connection, so the seven
    // members are declared here once. `query`, `execute`, `executeMany` and
    // `transaction` are the four that have landed.
    crate::db::CONNECTION,
    // What `rule:core-classes/db-transactions`'s closure is handed. It carries the same four rows
    // under the same symbols — which is what the delegation above is at
    // runtime — plus `rollBack`, the one member of the pair that is a
    // transaction's alone.
    crate::db::TRANSACTION,
    // What `query` answers with — `rule:core-classes/db-statement-members`'s buffered result set, and all
    // six of § 18's readers over it. Five read the rows it holds and
    // `columns()` reads the description beside them, which is why it is
    // readable before a row is.
    crate::db::ROWS,
    // What `stream` answers with — § 18's `Iterable<Db\Row>`, a name for the
    // walk with no member on it exactly as `Core\IO\Lines` is. It is the second
    // `Iterable` in `Core` that is its own iterator rather than a snapshot, and
    // [`crate::db::stream`] says why: a row of a streamed result set does not
    // exist until the walk asks the server for it.
    crate::db::STREAM,
    // One row of that set: § 18's associative reading plus the eleven typed
    // readers `rule:core-classes/db-column-types` puts in place of PHP's three fetch modes. Four of
    // the eleven answer only their refusal until § 9's structured columns land.
    crate::db::ROW,
    // What `execute` answers with: § 4's two counts and the id a `RETURNING`
    // clause handed back, as the three readers § 18 now writes — for the reason
    // [`CoreTy::Instance`] states, that a `Core` instance has no property a
    // program can reach, which that section's own preamble records.
    crate::db::WRITE,
    // One column of what a statement described — § 18's three readers in place
    // of `getColumnMeta`'s per-driver array, reached through `Rows::columns`
    // and produced nowhere else. Its `type()` is what gives `crate::db`'s
    // `COLUMN_TYPE` a member that answers one.
    crate::db::COLUMN,
    // What `inList` answers with: `rule:core-classes/db-parameters`'s explicit expansion marker,
    // memberless because § 18's own table accepts it nowhere but a bound
    // parameter. [`crate::db`] owns why the expansion itself stays in `nvs-db`.
    crate::db::IN_LIST,
    // `rule:core-classes/schema-is-a-value`'s schema value, beside the connection classes because that is
    // what it is asked about: a schema is compared against a live database and
    // applied to one. It needs no connection to exist, which is why its two
    // members here are the array form alone — § 9's three are the ones that take
    // a `Core\Db\Connection`.
    crate::db::SCHEMA,
    // `rule:core-classes/schema-plan`'s plan and its steps, immediately after the schema whose
    // `planAgainst` answers one: a plan is the value that carries the answer,
    // and neither class is reachable except through that member.
    crate::db::PLAN,
    crate::db::STEP,
    // `rule:concurrency/queue-four-members`'s durable background job, immediately after the database classes
    // because that is what it is made of: a job is a row in one of these connections,
    // which is the whole of why § 3's enqueue can commit with the write that caused
    // it. All four of § 1's members are landed, and [`crate::queue`]'s known gaps say
    // what that surface still owes around them.
    crate::queue::CLASS,
    // What `push` answers with — the row it wrote and the queue it is in, memberless
    // because the two members that take one are asked *about* it rather than through
    // it. [`CoreTy::Instance`]'s own rule about a `Core` instance's properties is why
    // it is a class and not a shape.
    crate::queue::ID,
    // What `stats` answers with, and the same rule read the other way round: counters
    // a program has to reach are members here, precisely because that rule leaves a
    // `Core` instance's properties unreachable. [`crate::queue::STATS`] owns which
    // counters they are and why the two that sum attempts are disjoint.
    crate::queue::STATS,
];

/// Every member of a capability-bearing class, and which capability it needs —
/// `rule:security/capability-declaration-is-one-table`
/// , with `None` for a member that needs none.
///
/// One table rather than a field on 346 rows, because "what can this runtime do
/// to my machine" is a question whose whole answer should be one screen of one
/// file. Spread across 41 class literals in 39 modules it is 39 greps and a
/// judgement about whether you found them all, which is precisely the judgement
/// a security review is trying not to have to make.
///
/// **Nothing reads this at run time.** It is audit data — `nvs meta` renders
/// it, the reference documentation prints it beside a member's card (`rule:core-api/reference-card`),
/// and § 7's closure test reads it. Enforcement is
/// `nvs_runtime::capability::require`, called inside the door that performs the
/// effect, and that function never looks here — so an edit to this table cannot
/// grant a permission, only misreport one.
///
/// Entries are `(class, member, Option<capability>)`, by the spellings
/// [`CoreClass::name`] and [`CoreMethod::name`] use;
/// `every_capability_entry_names_a_member` fails on one naming neither.
///
/// The rows naming a capability are the whole of what this runtime can do to a
/// machine — the filesystem, another program, a socket, and the endpoints an
/// operator configured. Every other `Core` member reaches no spelling that
/// performs an effect, which `nvs_stdlib_reaches_the_os_only_through_the_gate`
/// holds mechanically rather than by this table being kept honest. There is no
/// count here on purpose: a number in a comment beside a list that grows is one
/// that is wrong between every pair of additions, and `nvs meta` renders the
/// table itself.
///
/// **A `None` row is a declaration, not an exemption**, and that is § 7's
/// closure as a *shape* rather than as a promise to keep a list short. A class
/// is a door once any one of its members is declared, and its remaining members
/// then divide into two kinds a review has to tell apart: the ones nobody
/// classified, and the ones classified as reaching nothing. Both used to look
/// alike from here — a member simply absent from the table — so the second kind
/// lived on a frozen allowlist beside the closure test, where growing it by one
/// entry was the move `rule:testing/capability-closure-test` forbids and the only move a sibling like
/// `Core\RateLimit::shed` left. Declaring `None` costs a would-be exemption
/// exactly what a declaration costs, in the same table under the same review,
/// and buys a total claim in place of an "all but a list" one: every member of a
/// capability-bearing class has a row here, and there is no exception list
/// anywhere. Nothing about enforcement moves — a `None` row grants nothing,
/// because no row grants anything.
///
/// The `fs.read`/`fs.write` split is the filesystem's own and not a finer one:
/// asking a file's size is reading it, and removing a file is writing it, so
/// neither gets a capability of its own to be granted separately from the
/// effect it already implies. `within` is `fs.read` by the same reading:
/// resolving a name follows the symlinks and reads the directories above it,
/// which is what a program that could resolve an ungranted path would be
/// enumerating.
pub const CAPABILITIES: &[(&str, &str, Option<nvs_config::Cap>)] = &[
    // `Core\Response::sendFile` is the one member outside `Core\IO` that opens a
    // file, and it is `fs.read` for the reading below rather than for a weaker
    // one of its own: the member resolves a name and opens what is at it, which
    // is what `Core\IO::read` does, and answering with the bytes instead of
    // returning them changes nothing about the authority that was exercised. A
    // program that may not read a path may not send it either.
    (
        crate::response::NAME,
        "sendFile",
        Some(nvs_config::Cap::FsRead),
    ),
    // The rest of this class reaches nothing: a body member writes into the
    // response its own request is answering, and `setStatus`, `setHeader`,
    // `redirect` and `addCookie` declare a word onto the context that request
    // ends on. The bytes go to a peer the server already accepted, so there is
    // no name resolved, no socket opened and no process started — which is why
    // `sendFile` above is the row worth reading twice, being the one that leaves
    // the response and reaches the machine.
    (crate::response::NAME, "html", None),
    (crate::response::NAME, "json", None),
    (crate::response::NAME, "text", None),
    (crate::response::NAME, "bytes", None),
    (crate::response::NAME, "stream", None),
    (crate::response::NAME, "setStatus", None),
    (crate::response::NAME, "setHeader", None),
    (crate::response::NAME, "redirect", None),
    (crate::response::NAME, "addCookie", None),
    (crate::io::NAME, "read", Some(nvs_config::Cap::FsRead)),
    (crate::io::NAME, "write", Some(nvs_config::Cap::FsWrite)),
    // Adding to a file is writing it, and the split above has nothing finer to
    // offer: a grant that let a program extend a file but not replace it would
    // be a promise about the bytes already there that no filesystem keeps.
    (crate::io::NAME, "append", Some(nvs_config::Cap::FsWrite)),
    (
        crate::io::NAME,
        "writeStream",
        Some(nvs_config::Cap::FsWrite),
    ),
    (crate::io::NAME, "exists", Some(nvs_config::Cap::FsRead)),
    (crate::io::NAME, "isFile", Some(nvs_config::Cap::FsRead)),
    (crate::io::NAME, "isDir", Some(nvs_config::Cap::FsRead)),
    (crate::io::NAME, "isReadable", Some(nvs_config::Cap::FsRead)),
    // `fs.write` and not `fs.read`, which is the one place this table's split
    // is decided by what a question is *about* rather than by what its member
    // does: `isWritable` performs no write and asks about nothing else, so a
    // program granted only reads may not ask it. `capability::writable`'s own
    // doc is the home of that reading.
    (
        crate::io::NAME,
        "isWritable",
        Some(nvs_config::Cap::FsWrite),
    ),
    (crate::io::NAME, "size", Some(nvs_config::Cap::FsRead)),
    // The same reading as `size`: a modification time is metadata, and a
    // program that can learn when a path it was never granted last changed can
    // watch a directory it may not open.
    (crate::io::NAME, "modifiedAt", Some(nvs_config::Cap::FsRead)),
    // One row for the member that answers every one of the questions above at
    // once, and the same capability each of them needs — a `stat` is one read
    // whether the caller wanted one field of it or four.
    (crate::io::NAME, "stat", Some(nvs_config::Cap::FsRead)),
    // Enumeration is a read of one path — the directory itself — and never a
    // grant over what the listing names. `capability::read_dir`'s own doc is
    // the home of that reading, and of why it is a door rather than a widening
    // of `exists`.
    (crate::io::NAME, "list", Some(nvs_config::Cap::FsRead)),
    // The same grant for the same door, declared once here and asked for every
    // directory the walk enters: this table names the authority a member
    // exercises, not the number of times it exercises it.
    (crate::io::NAME, "walk", Some(nvs_config::Cap::FsRead)),
    // Resolving a name reads the directories above it, so `fs.read` and not
    // nothing: a program that can resolve a path it was never granted can
    // learn which of that path's components exist.
    (
        crate::io::NAME,
        "canonicalize",
        Some(nvs_config::Cap::FsRead),
    ),
    // `copy` reads one path and writes another, and this table names one
    // capability per member, so it names the stronger — exactly as `open`'s row
    // does for its `ReadWrite` mode and for that row's reason. The door checks
    // both, and it is the door a refusal comes from.
    (crate::io::NAME, "copy", Some(nvs_config::Cap::FsWrite)),
    // `move` is `fs.write` on both ends and not `copy`'s pair: taking the
    // source away is destroying it.
    (crate::io::NAME, "move", Some(nvs_config::Cap::FsWrite)),
    (crate::io::NAME, "makeDir", Some(nvs_config::Cap::FsWrite)),
    (crate::io::NAME, "remove", Some(nvs_config::Cap::FsWrite)),
    (crate::io::NAME, "removeDir", Some(nvs_config::Cap::FsWrite)),
    (
        crate::io::NAME,
        "temporaryDir",
        Some(nvs_config::Cap::FsWrite),
    ),
    (crate::io::NAME, "within", Some(nvs_config::Cap::FsRead)),
    (crate::io::NAME, "readText", Some(nvs_config::Cap::FsRead)),
    (crate::io::NAME, "lines", Some(nvs_config::Cap::FsRead)),
    // The one member whose capability its *argument* decides: `fs.read` for
    // `FileMode::Read`, `fs.write` for `Write` and `Append`, and both for
    // `ReadWrite`. This table has one cap per member and cannot say that, so it
    // names the stronger of the two — a row that understated the authority a
    // member can exercise would be the wrong direction for a table a reviewer
    // reads to find out what this runtime can do to a machine. The enforcement
    // is `nvs_runtime::capability::open`, which asks per mode; this is the
    // declaration, and `rule:security/capability-check-at-the-door` is why the two are separate.
    (crate::io::NAME, "open", Some(nvs_config::Cap::FsWrite)),
    // The one member of this class that opens nothing and resolves no name:
    // standard input is a descriptor the process was started holding, so ADR
    // 0118 § 1 has no door to put a check at — the same reading as the handle
    // members below, one step earlier, since here there was never even a path.
    // A grant would be a boolean over an authority the invoker already
    // exercised by running the program with its input attached.
    (crate::io::NAME, "stdin", None),
    // `Core\IO\File`'s members need no row of their own: the descriptor was
    // checked when `open` produced it, which `capability::open_read`'s own doc
    // states as the reason a door hands back a handle at all. A `None` row here
    // is the declaration that says so, per this table's own docs — and it is
    // one row per member rather than one for the class precisely so that a
    // member added later cannot inherit the answer without being asked.
    // `Core\IO\Metadata`'s four members reach nothing at all: the `stat` that
    // built the value was checked when `Core\IO::stat` asked, and each member
    // here is a slot read over the answer it already holds. That is the same
    // shape as the handle rows below — the check happened where the value was
    // produced — and the reason a snapshot is worth having.
    (crate::io::METADATA_NAME, "size", None),
    (crate::io::METADATA_NAME, "modifiedAt", None),
    (crate::io::METADATA_NAME, "isFile", None),
    (crate::io::METADATA_NAME, "isDir", None),
    (crate::io::FILE_NAME, "read", None),
    (crate::io::FILE_NAME, "readLine", None),
    (crate::io::FILE_NAME, "write", None),
    (crate::io::FILE_NAME, "seek", None),
    (crate::io::FILE_NAME, "tell", None),
    // `truncate` is where that reading is worth stating out loud, because it is
    // the one member of this class that destroys data the handle never wrote:
    // the answer is still the descriptor's, since re-checking the *path* would
    // check a name that may since have been renamed away from the file being
    // resized. A handle opened `Read` has no such proof and the operating system
    // refuses it, which is `nvs_core_io_file_truncate`'s own doc.
    (crate::io::FILE_NAME, "truncate", None),
    (crate::io::FILE_NAME, "flush", None),
    // `lock` reaches no further than the descriptor either, and reaches *less*
    // far than `write`: it changes nothing about the file's contents, which is
    // why a handle opened `Read` may take one.
    (crate::io::FILE_NAME, "lock", None),
    (crate::io::FILE_NAME, "close", None),
    // `rule:security/process-exec-capability`: starting a program is deny-by-default and path-scoped, the
    // same shape `script.spawn` already has. `Core\Process\Result`'s three
    // members need no row — the child has exited by the time one exists, and a
    // slot read performs no effect.
    (
        crate::process::NAME,
        "run",
        Some(nvs_config::Cap::ProcessExec),
    ),
    // `spawn` reaches the same door on the same terms — it differs from `run`
    // only in not waiting, and a program that may not start a child may not
    // start one it intends to stream either. `Core\Process\Handle`'s five
    // members need no row for `Core\Process\Result`'s reason once over: the
    // child was checked when `spawn` produced it, and reading a pipe it already
    // owns reaches nothing a second grant could scope.
    (
        crate::process::NAME,
        "spawn",
        Some(nvs_config::Cap::ProcessExec),
    ),
    // `rule:http-server/allow-url-pins-the-address` and `rule:security/net-address-policy`: approving a URL resolves its host, which is an effect,
    // and the grant is host-scoped. The address policy behind the same door is
    // not a second capability — it is deny-by-default and applies to every
    // grant, which is why it is a table in `nvs_config::capability` rather than
    // a row here.
    (
        crate::http::NAME,
        "allowUrl",
        Some(nvs_config::Cap::NetConnect),
    ),
    // `fs.read` and at the constructor rather than at the send: naming a file as
    // a body is what decides which file leaves the process, and the split above
    // makes measuring one a read of it. The framing asks the same door again
    // when it opens the file, so the grant covers both the name and the octets.
    (
        crate::http::PART_NAME,
        "file",
        Some(nvs_config::Cap::FsRead),
    ),
    // Octets the program is already holding reach nothing, and the declaration
    // is what says so — a class is a door once any one of its members is one.
    (crate::http::PART_NAME, "bytes", None),
    // `rule:security/net-listen-is-a-separate-grant-from-net-connect`: the two
    // rows below are the whole of that rule in this table. Which grant a door
    // asks is decided by what the program is *doing* and never by the transport
    // it does it over, so reaching outward names `net.connect` and binding names
    // `net.listen` — and neither widens the other, which is what makes them two
    // rows rather than one with a wider grant in it. The address policy behind
    // the first is not a third row for `allowUrl`'s reason: it is deny-by-default
    // over every grant, and it deliberately does not apply to the second at all.
    (
        crate::net::NAME,
        "connect",
        Some(nvs_config::Cap::NetConnect),
    ),
    (crate::net::NAME, "listen", Some(nvs_config::Cap::NetListen)),
    // A bind is a bind whatever transport is bound at it: what decides the
    // grant is what the program is doing, not the protocol it does it over.
    (
        crate::net::NAME,
        "bindDatagram",
        Some(nvs_config::Cap::NetListen),
    ),
    // `rule:config/net-local-is-named-and-not-on-the-roster`: a program-supplied
    // socket path is `net.local` at **both** ends, which is why these two rows
    // name one grant between them. It is not `net.connect` widened to admit a
    // path — that grant's character is the address policy it carries, and a path
    // has no address for the policy to read — and it is not `net.listen` for the
    // bind, because the question a path asks is which path, not which endpoint.
    (
        crate::net::NAME,
        "connectLocal",
        Some(nvs_config::Cap::NetLocal),
    ),
    (
        crate::net::NAME,
        "listenLocal",
        Some(nvs_config::Cap::NetLocal),
    ),
    // `Core\Net\Stream`'s and `Core\Net\Listener`'s members need no grant of
    // their own, and these `None` rows are the declaration that says so —
    // `Core\IO\File`'s rows one class over are the same reading. The socket was
    // checked where it was opened, which is what makes a door hand back a
    // handle at all; `accept` is the one worth reading twice, since a connection
    // arriving is not this program reaching anywhere, and asking `net.connect`
    // of a peer's address would make a granted listener unable to serve anyone
    // outside its *outbound* grant.
    (crate::net::STREAM_NAME, "read", None),
    (crate::net::STREAM_NAME, "write", None),
    (crate::net::STREAM_NAME, "close", None),
    (crate::net::LISTENER_NAME, "accept", None),
    (crate::net::LISTENER_NAME, "port", None),
    (crate::net::LISTENER_NAME, "close", None),
    // `Core\Net\Datagram` is the one handle class here with a member declaring a
    // grant of its own, and it is not an exception to the reading above but the
    // same reading applied: a datagram socket is bound and never connected, so
    // the outbound address is named at the send rather than at the opening, and
    // `rule:security/net-address-policy` says that is where it is asked about.
    // The other three reach nothing the bind had not already granted.
    (
        crate::net::DATAGRAM_NAME,
        "send",
        Some(nvs_config::Cap::NetConnect),
    ),
    (crate::net::DATAGRAM_NAME, "receive", None),
    (crate::net::DATAGRAM_NAME, "port", None),
    (crate::net::DATAGRAM_NAME, "close", None),
    // A received message is three values already in hand: nothing it answers
    // reaches anything, and there is no socket behind it to reach with.
    (crate::net::MESSAGE_NAME, "payload", None),
    (crate::net::MESSAGE_NAME, "host", None),
    (crate::net::MESSAGE_NAME, "port", None),
    // `rule:core-api/two-cache-tiers`: the shared tier is a real store over the network, and
    // `shared()` is the door — it dials the store the deployment configured, while
    // `Core\Cache\Store`'s two operations run on what it opened and so declare
    // nothing, exactly as `Core\Http\Client` declares nothing behind
    // `Core\Http::allowUrl`. The grant is `cache.shared` and not `net.connect`
    // for `mail.send`'s reason two classes over and by the same authority
    // (`rule:config/cache-shared-is-the-grant-over-the-configured-store`): the
    // endpoint is one an operator wrote into root-owned configuration, so it is
    // pre-approved, and a grant naming the *store* stays true when a deployment
    // moves it.
    (
        crate::cache::NAME,
        "shared",
        Some(nvs_config::Cap::CacheShared),
    ),
    // And the tiers that stay in the process declare `None`, which is the
    // asymmetry these rows exist to state: a tier that leaves the process has a
    // door, and one that cannot has nothing to put a door on. `rule:core-api/two-cache-tiers` decided this before the
    // member was written — the local tier is a `HashMap` in the calling core's
    // own thread, so nothing leaves the process, no name is resolved and no file
    // is opened, and `rule:security/capability-question-is-grant-and-scope` has no door to check at. What is left to bound
    // is footprint, which `rule:concurrency/cache-memory-is-charged-to-the-core`'s `nvs.toml` cap bounds and a boolean
    // grant would not: a grant would price caching anything as an authority
    // question every deployment then has to answer, and still not bound a byte.
    (crate::cache::NAME, "local", None),
    // The process tier is the same answer one step further out: one map every
    // core of the serving process shares
    // (`rule:concurrency/the-process-tier-is-one-store-per-process`), which is
    // still no door, because the bytes reach no other process and no name is
    // resolved to put them there. What bounds it is `[cache.process] max_size`,
    // for the reason the row above gives and with the same arithmetic — held once
    // per process rather than once per core.
    (crate::cache::NAME, "process", None),
    // `rule:core-classes/ratelimit-two-members` and `rule:core-classes/ratelimit-unreachable-store-throws` write `Core\RateLimit::consume` standing alone, so it
    // is its own door onto the same store rather than something that has to
    // follow a `Core\Cache::shared()`: it reads the same directive, asks for the
    // same `cache.shared` grant and reuses the same per-core socket.
    // `Core\RateLimit\Decision`'s four readers need no row — the store has
    // answered by the time one exists, and a slot read performs no effect,
    // exactly as `Core\Process\Result`'s three members do.
    (
        crate::ratelimit::NAME,
        "consume",
        Some(nvs_config::Cap::CacheShared),
    ),
    // `rule:core-classes/ratelimit-two-members`'s other member, and the same asymmetry one class over:
    // `shed`'s state is the calling core's own memory, so it is `Core\Cache`'s
    // local tier by construction — the row above it is `cache.shared` because a
    // coherent limiter is a store on the network, and this one is `None`
    // because an approximate one is a map in this thread. It is bounded by the
    // same cap for the same reason, since it is the same store.
    (crate::ratelimit::NAME, "shed", None),
    // `rule:errors/log-write`'s writer, and the third `None` here — declared rather than
    // left off because "writing a log" is exactly the kind of effect a reader
    // expects a door on, and the answer has to be somewhere they will look.
    // What it reaches is the running program's own output stream, which every
    // `echo` already reaches and no capability governs: the record goes where
    // the operator pointed the process, not to a name the program chose, which
    // is `crate::env`'s reasoning one class over. The moment `[log] target`
    // grows `rule:errors/engine-floor`'s `file:<path>` this row is where the question is
    // asked again, because a path the operator names is still a file the
    // engine opens rather than one the program picked.
    (crate::log::NAME, "write", None),
    // `rule:security/optional-capability-degrades`'s query, and the fourth `None`: the member whose whole
    // subject is capabilities is the one that needs none. It reads the table a
    // door would read and answers a `bool` — no name is resolved, no file is
    // opened and nothing leaves the process, so `rule:security/capability-question-is-grant-and-scope` has no door to put
    // a check at, exactly as `Core\Cache::local` two rows up. Requiring a grant
    // to ask about grants would also close the shape § 6 opened: a package that
    // declared `fs.write` optional would need a second capability before it
    // could find out whether it had the first.
    (crate::cap::NAME, "has", None),
    // `rule:programs/framework-core-half`'s transport half, granted the way `rule:core-classes/db-capabilities` grants a
    // database: by the *name* of the block, never by the host inside it. That is
    // what makes the row `mail.send` rather than `net.connect` — a `net.connect`
    // grant is a claim about hosts a program may reach, and this member reaches
    // no host a program can name. [`crate::mail`]'s module doc is the home of
    // why the address is not additionally pinned.
    (crate::mail::NAME, "send", Some(nvs_config::Cap::MailSend)),
    // `rule:core-classes/db-capabilities`'s split. `db.connect` names *blocks* and not hosts, for
    // `mail.send`'s reason and by the same authority: the endpoint is one an
    // operator wrote into root-owned configuration. `db.open`'s targets are
    // program-supplied and reach `rule:http-server/allow-url-pins-the-address`'s address policy in full, which is
    // the whole difference between the two grants — and why they are two.
    (crate::db::NAME, "connect", Some(nvs_config::Cap::DbConnect)),
    (crate::db::NAME, "open", Some(nvs_config::Cap::DbOpen)),
    // The two connectionless members reach no effect at all: they are pure
    // functions of their arguments, and the `None` rows are what make this
    // table's claim total rather than "all but a list".
    (crate::db::NAME, "inList", None),
    (crate::db::NAME, "quoteIdentifier", None),
    // `rule:core-classes/schema-apply-capability`'s split, and the reason `Core\Db\Schema` is a
    // capability-bearing class at all: reaching a database is not permission to
    // change its shape. `db.schema` gates whether this program may issue DDL to
    // the named block, which is a different question from `db.connect`'s "may
    // it reach that block" — a deployment grants the first to a migration entry
    // point and the second to everything.
    (
        crate::db::SCHEMA_NAME,
        "applySafe",
        Some(nvs_config::Cap::DbSchema),
    ),
    (
        crate::db::SCHEMA_NAME,
        "applyIncludingRisky",
        Some(nvs_config::Cap::DbSchema),
    ),
    // The other three reach no effect this table has anything to say about.
    // `planAgainst` does reach the database — it issues § 4's two catalog
    // queries — and still declares `None`, because the grant that let it reach
    // that database was asked for at `Core\Db::connect` and § 9's first
    // sentence is that nothing about computing a plan is privileged. The two
    // conversions touch nothing at all.
    (crate::db::SCHEMA_NAME, "planAgainst", None),
    (crate::db::SCHEMA_NAME, "fromArray", None),
    (crate::db::SCHEMA_NAME, "toArray", None),
    // `rule:concurrency/queue-four-members`'s four, which take no grant, beside
    // `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s removal, which
    // takes the only one this class has. The `None` rows are the declaration and
    // not an omission: `push` names no block for a grant to be about, and the
    // other three read or release what the caller already holds a receipt for.
    // Removal is the one act here that destroys the record that work existed, so
    // it is the one asked about — scoped on the queue the receipt carries, which
    // is `rule:core-classes/db-capabilities`' shape and not `net.connect`'s, the
    // database behind it having been granted at `[queue] connection` already.
    (crate::queue::NAME, "push", None),
    (crate::queue::NAME, "status", None),
    (crate::queue::NAME, "cancel", None),
    (crate::queue::NAME, "stats", None),
    (
        crate::queue::NAME,
        "delete",
        Some(nvs_config::Cap::QueuePurge),
    ),
    (
        crate::queue::NAME,
        "purge",
        Some(nvs_config::Cap::QueuePurge),
    ),
    // `Core\Queue\Stats`' counters read slots off a record the statement already
    // answered, and `Core\Queue\Id`'s row is not here because it has no member at
    // all — both are `Core\Process\Result`'s reading one class over.
    (crate::queue::STATS_NAME, "pending", None),
    (crate::queue::STATS_NAME, "claimed", None),
    (crate::queue::STATS_NAME, "attempts", None),
    (crate::queue::STATS_NAME, "deadLettered", None),
    (crate::queue::STATS_NAME, "deadAttempts", None),
    // `rule:programs/framework-core-half`'s storage half, and the rows that make its "over `rule:core-api/tier-placement`'s
    // existing `fs.*` capabilities" true: the same two grants `Core\IO` above
    // declares, asked about the path the disk's root and the object's key
    // resolve to. There is deliberately no `storage.*` capability — a second
    // grant over one door is the shape where a deployment is tightened in one
    // of them and stays open through the other, which [`crate::storage`]'s
    // module doc is the home of.
    (crate::storage::NAME, "put", Some(nvs_config::Cap::FsWrite)),
    (crate::storage::NAME, "get", Some(nvs_config::Cap::FsRead)),
    (
        crate::storage::NAME,
        "delete",
        Some(nvs_config::Cap::FsWrite),
    ),
    // Reading the disk's root as a directory, which is a read of it and not a
    // fifth grant: enumerating is the question `fs.read` already answers about
    // a path, and `nvs_runtime::capability::exists`' own doc is where that
    // reading is argued.
    (crate::storage::NAME, "list", Some(nvs_config::Cap::FsRead)),
    // `Core\Zip` is a door because one of its three members writes files, and
    // the other two then declare what they reach, which is nothing: an archive
    // is octets a program already holds, so listing it and reading one entry
    // out of it touch no path at all. `extract` declares `fs.write` for the
    // effect it has, and shows `fs.read` at the same door for the destination
    // as well, because resolving a name is reading the directories above it —
    // the reading this table's own `within` row already takes.
    (crate::zip::NAME, "entries", None),
    (crate::zip::NAME, "read", None),
    (crate::zip::NAME, "extract", Some(nvs_config::Cap::FsWrite)),
];

/// `rule:expressions/nullable-conversion`
/// The `Core` classes that declare a `tryParse` beside their `parse` —
/// `rule:expressions/try-parse`'s
/// closed exception to `rule:core-api/shape-rules`
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
/// symbol per class for `$s as ?Core\Uri` to lower to. `rule:expressions/nullable-conversion-availability` withdrew
/// that form: `as` never targets a class now, with no exceptions, so nothing
/// here is chained into [`crate::symbols`] and `nvs-ir` has no roster to read.
pub const TRY_PARSE_CLASSES: &[&str] = &[crate::uri::NAME, crate::uuid::NAME];

/// Every `Core` class a program may write `new` on, with the constructor that
/// builds one — `docs/spec/01-core-library.md` § 9's collections and § 11's
/// seeded generator.
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
    (crate::random::SEEDED_NAME, &crate::random::SEEDED_NEW),
];

/// The constructor that builds a `class` instance, or `None` when `new` on it
/// is not a thing a program may write — which is every other name.
#[must_use]
pub fn constructor_of(class: &str) -> Option<&'static CoreMethod> {
    nvs_footprint::class(class);
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

/// One `Core`-owned enum — `rule:enums/closed-integer-type`'s
/// closed, named integer type, declared here rather than in Novis source.
#[derive(Clone, Copy, Debug)]
pub struct CoreEnum {
    /// The fully-qualified name, backslash-separated exactly as written in
    /// source (`Core\Order`).
    pub name: &'static str,
    /// Its cases, in declaration order. The value is each case's own
    /// constant, written out rather than auto-incremented: `rule:enums/declaration`'s
    /// auto-increment is a *source* convenience, and a table read by the
    /// compiler has nothing to gain from re-deriving what it could state.
    pub cases: &'static [(&'static str, i64)],
    /// Its reference card, or `None` for an enum nobody has documented yet —
    /// the same seam [`CoreMethod::doc`] is, on the roster it left out.
    pub doc: Option<&'static EnumDoc>,
}

/// One enum's reference documentation — `rule:core-api/reference-card`'s card for the values a
/// parameter may take, so a `{mode: RoundMode::HalfEven}` is explained where
/// `RoundMode` is declared and not only where `round` is.
///
/// The same rules as [`MethodDoc`]: inline markdown, one or two sentences a
/// field, **an empty string or an empty slice means "not written yet"**, and
/// `nvs meta --json` omits such a field. Spends the same kind of memory —
/// static strings, per process — which the ADR's *Consequences* prices.
#[derive(Clone, Copy, Debug)]
pub struct EnumDoc {
    /// What the enum chooses between, in one or two sentences.
    pub short: &'static str,
    /// One entry per case, in [`CoreEnum::cases`]' declaration order and
    /// matched to a case **by name** — `every_enum_case_doc_names_a_real_case`
    /// holds the two rosters together. Empty for an enum whose cases are a
    /// table rather than a card, such as one case per WHATWG encoding.
    pub cases: &'static [CaseDoc],
}

/// One enum case's name and description.
#[derive(Clone, Copy, Debug)]
pub struct CaseDoc {
    /// The case's name exactly as [`CoreEnum::cases`] spells it — `Asc`,
    /// `HalfEven`.
    pub name: &'static str,
    /// What choosing this case means, in one sentence.
    pub desc: &'static str,
}

/// Every enum `Core` owns.
///
/// A second roster beside [`CLASSES`] rather than a member of it, because an
/// enum is not a class: `rule:classes/no-free-functions-or-constants`
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
/// through an *attribute* rather than through a member: `rule:routing/route-attribute`'s
/// `#[Route(method: …)]` is where a program writes a case of it, and
/// [`crate::router::AUDIENCE`] passes it the same way, through `rule:attributes/access-payload`'s `#[Access(allow: …)]`.
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
    // Beside `Core\Digest` because it is the same decision on a second
    // subsystem: a format chosen at the call site is a closed enum, never a
    // name in a string that a run-time comparison has to validate.
    crate::compress::CODEC,
    // The cipher a `Core\Crypto::seal` or `::open` names, beside `Core\Digest`
    // and `Core\Compress\Codec` because it is their decision a third time: the
    // primitive is a closed case at the call site, never the mode string
    // `openssl_encrypt` reads, and this one carries no default either.
    crate::crypto::CIPHER,
    // Beside the cipher because it is the same class's other closed choice, on
    // the asymmetric half: which key a program means, named at the call rather
    // than sniffed out of the encoding. It is also where RSA's two signature
    // schemes are told apart, which a PKCS#8 or SPKI key cannot do for itself.
    crate::crypto::KEY_KIND,
    // And the encoding that key crosses in, which is the third closed choice of
    // the same class: `exportKey`'s own three formats, named at the call rather
    // than sniffed out of the octets a program was handed.
    crate::crypto::KEY_FORMAT,
    // The answer `Core\Mime::detect` gives, whose zero case is `Unknown`: a
    // detection that cannot say is an ordinary case rather than an error, and
    // a closed one so a caller never compares against a media-type spelling
    // that occurs nowhere.
    crate::mime::TYPE,
    // What `Core\Xml\Node::kind` answers with, and the one roster both parsers
    // build against — closed at five, so a walk over a document is exhaustive.
    // Beside `Core\Mime`'s because both are § 17's, and for the reason that
    // section's classes sit together in [`CLASSES`].
    crate::xml::KIND,
    crate::log::LEVEL,
    crate::env::MODE,
    crate::router::METHOD,
    crate::router::AUDIENCE,
    // The one enum whose case values are not numbered from zero: they are the
    // HTTP status codes themselves, which [`crate::response::REDIRECT`]'s own
    // doc owns. It sits beside the router's two because all three are read off
    // one request.
    crate::response::REDIRECT,
    // The other half of the same response: `Set-Cookie`'s attribute, whose
    // three cases *are* numbered from zero because the wire never numbered
    // them — [`crate::response::SAME_SITE`] owns that contrast with the row
    // above it.
    crate::response::SAME_SITE,
    crate::cli::STREAM,
    crate::cli::COLOR_DEPTH,
    crate::cli::SHELL,
    crate::reflect::TYPE_KIND,
    crate::io::FILE_MODE,
    crate::cldr::PLURAL_CATEGORY,
    crate::script::EXIT_REASON,
    crate::db::DRIVER,
    crate::db::TLS,
    crate::db::ISOLATION,
    crate::db::COLUMN_TYPE,
    crate::db::ERROR_KIND,
    // `rule:core-classes/schema-plan`'s grade, beside the database enums because it is read off a
    // plan the way `ErrorKind` above is read off a failure.
    crate::db::GRADE,
    // `rule:concurrency/claiming-is-one-statement` and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`'s job lifecycle, immediately after the database enums
    // for the reason [`crate::queue::CLASS`] sits after the database classes: a
    // job is a row, and this enum is one of that row's columns as well as what
    // `Core\Queue::status` answers. [`crate::queue`]'s own docs own why there
    // are four cases and no `Failed`.
    crate::queue::STATE,
];

/// Looks a class up by its fully-qualified name.
#[must_use]
pub fn class(name: &str) -> Option<&'static CoreClass> {
    nvs_footprint::class(name);
    CLASSES.iter().find(|class| class.name == name)
}

/// Records, when `NVS_FOOTPRINT_LOG` names a log, that a program used the signatures of the class
/// `name`: the class itself, and every `Core` class and enum a parameter or return type of one of
/// its members names. A program that calls `Core\Process::run` is typed against
/// `Core\Process\Result` without ever looking that class up, so an edit to its row has to reach
/// the program too. The name is compared without regard to ASCII case, as the compiler compares
/// one.
pub fn record_signature(name: &str) {
    if !nvs_footprint::enabled() {
        return;
    }
    nvs_footprint::class(name);
    for named in signature_classes(name) {
        nvs_footprint::class(named);
    }
}

/// Every `Core` class and enum a parameter or return type of a member of the class `name` names,
/// sorted and without the class itself.
#[must_use]
pub fn signature_classes(name: &str) -> Vec<&'static str> {
    fn walk(ty: &CoreTy, into: &mut Vec<&'static str>) {
        match ty {
            CoreTy::Instance(name) | CoreTy::Enum(name) | CoreTy::EnumCase(name, _) => {
                into.push(name);
            }
            CoreTy::InstanceAt(name, args) => {
                into.push(name);
                args.iter().for_each(|arg| walk(arg, into));
            }
            CoreTy::Array(inner) | CoreTy::Nullable(inner) | CoreTy::Iterated(inner) => {
                walk(inner, into);
            }
            CoreTy::Union(members) => members.iter().for_each(|member| walk(member, into)),
            CoreTy::CallableSig(params, ret) => {
                params.iter().for_each(|param| walk(param, into));
                walk(ret, into);
            }
            _ => {}
        }
    }
    let Some(class) = CLASSES
        .iter()
        .find(|class| class.name.eq_ignore_ascii_case(name))
    else {
        return Vec::new();
    };
    let mut named = Vec::new();
    for member in class.members() {
        member
            .params
            .iter()
            .for_each(|param| walk(param, &mut named));
        walk(&member.return_ty, &mut named);
    }
    named.sort_unstable();
    named.dedup();
    named.retain(|found| *found != class.name);
    named
}

/// The `Core` interfaces `rule:core-classes/derive-attribute`'s attributes
/// stand for. A class carrying `#[Json\Derive]` implements the first of them
/// whether or not it writes the clause, and writing the clause is redundant
/// but accepted — so both names are ones a link clause may resolve to, while
/// neither is a [`CoreClass`] row, since a program calls no member on them.
pub const DERIVE_INTERFACES: &[&str] = &[r"Core\Json\Codec", r"Core\Db\Codec"];

/// Every `Core` name a class may write in an `extends`/`implements` clause:
/// the classes above and [`DERIVE_INTERFACES`].
///
/// This is what a front end hands `nvs_hir::CoreRoster::Names`, which is the
/// only authority on the question — `Core` is compiler-owned, so no source
/// file declares a name there and no symbol table can answer for one. **A
/// `Core` interface a future rule lets a program name joins
/// [`DERIVE_INTERFACES`]**, which is the bound: this function reads the
/// tables and holds nothing of its own.
#[must_use]
pub fn link_targets() -> Vec<&'static str> {
    CLASSES
        .iter()
        .map(|class| class.name)
        .chain(DERIVE_INTERFACES.iter().copied())
        .collect()
}

/// Every `Core` type a program may name where a class, an interface or an
/// enum goes: the classes, [`DERIVE_INTERFACES`] and the enums.
///
/// This is what a front end hands `nvs_hir::undeclared_name` as the roster a
/// fix may import from: a bare `Request` that resolved to nothing is offered
/// `use Core\Request;` because this list holds it. It is [`link_targets`]
/// widened by the enums, which no clause may name and any expression may.
#[must_use]
pub fn type_names() -> Vec<&'static str> {
    nvs_footprint::every_class();
    link_targets()
        .into_iter()
        .chain(ENUMS.iter().map(|core| core.name))
        .collect()
}

/// Whether a `Core`-owned class renders as text —
/// `rule:classes/stringable`'s question, asked here because a `Core` class has no other place to
/// answer it: it declares no interfaces, so there is no `Stringable` for
/// `nvs_types` to prove against, and its members are the rows above rather
/// than entries in a class graph.
///
/// **Two rows answer yes, and they are two different rules.** A class the spec
/// gives a `toString` renders through that member, which is `rule:classes/stringable`
/// exactly. A **sink carrier** renders through
/// `rule:security/capture-answers-the-carrier`
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
    nvs_footprint::class(name);
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
/// one (`rule:types/declaration`). A
/// member like `Core\Json::decodeAs<User>` needs more than the erasure: it has
/// to build a `User`, which means reaching that class's
/// `nvs_runtime::ClassDesc` from native Rust. `rule:testing/doubles`'s `double`
/// reaches the same descriptor for the opposite reason: what it builds is a
/// class that **conforms to** the one written, so the descriptor arrives as a
/// parent rather than as a layout to fill.
///
/// A roster rather than a field on [`CoreMethod`] because it names a handful of
/// members against two hundred member rows, and a field would be `false` on
/// nearly every one of them. `nvs-ir` reads this to decide whether to emit the block
/// of constants ahead of the call's own arguments; the helper's `args: [N]`
/// therefore counts **three** more than [`CoreMethod::params`] does.
///
/// **The descriptor is always argument 0, and on an instance member the
/// receiver moves to argument 3.** The block goes ahead of *everything*,
/// receiver included, rather than after it: what a call site wrote as its type
/// argument is a constant of the call and not a value of the receiver, and
/// putting it first means one rule for both kinds of member instead of two
/// positions to remember. So `Core\Db\Connection::queryAs<T>(string, array)` is
/// `args: [6]` — descriptor, list flag, wire contract, receiver, `$sql`,
/// `$params`. `nvs_ir::lower::Lowering::written_type_constants` is the one
/// place all three call paths emit it.
///
/// **Argument 1 is a `bool`: whether the class was written inside an
/// `array<...>`.** `decodeAs<array<User>>` hands over `User`'s descriptor and
/// `true`, because a list decode is the same decode run once per element and
/// there is no descriptor for `array` to send instead. It rides beside the
/// descriptor rather than being recovered from the document's own shape: the
/// checker has already given the call site the type `array<User>` or `User`,
/// and a helper guessing from the JSON would hand back the other one.
///
/// **Argument 2 is the wire contract of an inline shape**, where the type
/// argument named one instead of a class — `nvs_runtime::ShapeCodec`, and the
/// zero word where it named a class, whose own contract is on the descriptor in
/// argument 0. The slot is there either way, so a member reads one ABI rather
/// than branching on what its call site wrote; `nvs-ir`'s module docs, *A
/// shape's wire contract*, own why a shape's cannot ride on its descriptor.
///
/// **Each of `Core\Db\Queryable`'s written members is on it twice**, because
/// `rule:classes/no-traits`'s delegation is two registry rows and this roster is
/// keyed by the *declaring* class — the one `nvs_types::expr::calls` resolves a
/// call to. A member forwarded from `Core\Db\Transaction` to its connection is
/// still written on the transaction, so both spellings have to be here or the
/// same call through a `$tx` would lose its class.
pub const WRITTEN_CLASS_MEMBERS: &[(&str, &str)] = &[
    (r"Core\Arr", "shapeAs"),
    (r"Core\Json", "decodeAs"),
    (r"Core\Jwt", "verifyIssued"),
    (r"Core\Request", "jsonAs"),
    (r"Core\Request", "queryAs"),
    (r"Core\Request", "postAs"),
    (r"Core\Http\Response", "jsonAs"),
    (r"Core\Db\Connection", "queryAs"),
    (r"Core\Db\Transaction", "queryAs"),
    (r"Core\Db\Connection", "streamAs"),
    (r"Core\Db\Transaction", "streamAs"),
    (r"Core\Test", "double"),
    (r"Core\Test", "partial"),
];

/// Which positional parameter of `class::method` is `rule:security/isolate-shares-nothing`'s isolate entry,
/// if any is — the slot `nvs_types::expr::isolate`'s `check_entry` applies its
/// rule at.
///
/// Read off the row's own [`CoreTy::Entry`] rather than from a roster the
/// checker holds, and that is the whole reason the mark is on the type instead
/// of being a `(class, member)` pair like [`WRITTEN_CLASS_MEMBERS`]: a member
/// that opens an isolate gets the rule in the edit that writes its signature,
/// so `Core\Sse`'s row cannot land accepting a `callable` the ADR refuses
/// because a second table was not updated. There is no roster to forget.
///
/// `None` for a member with no entry parameter, which is every member but the
/// two `rule:concurrency/an-upgrade-is-spawn-shaped` and § 5 name.
#[must_use]
pub fn entry_parameter(class: &str, method: &str) -> Option<usize> {
    self::class(class)?
        .members()
        .find(|member| member.name == method)?
        .positional()
        .iter()
        .position(|param| matches!(param, CoreTy::Entry))
}

/// Whether `class::method` is one of [`WRITTEN_CLASS_MEMBERS`].
#[must_use]
pub fn takes_written_class(class: &str, method: &str) -> bool {
    nvs_footprint::class(class);
    WRITTEN_CLASS_MEMBERS
        .iter()
        .any(|(owner, name)| *owner == class && *name == method)
}

/// The closed roster of members whose helper is handed **where it was called**,
/// as an extra leading argument — the file, the one-based line and the
/// enclosing `Class::member` of the call.
///
/// Only the compiler knows where a call is, so the datum arrives as a constant
/// of the call: `nvs_ir::ir::InstKind::SourceConst` carries what
/// `nvs_ir::lower::Lowering::source` derived, `nvs-codegen` bakes
/// `nvs_runtime::source::encode`'s bytes into the unit, and the helper reads
/// them back through `nvs_runtime::source::of_operand`. A member on this roster
/// therefore has `args: [N]` **one more** than [`CoreMethod::params`] counts.
///
/// **Two rules put a member here**, and each wants the datum to send a reader
/// to a line of their own program.
/// `rule:errors/a-record-names-where-it-was-produced` makes a record name where
/// it was produced, which is `Core\Debug::dump` and `Core\Log::write`.
/// `rule:observability/metrics-three-members` fixes a metric name to one kind
/// on first use, so the call that disagrees with it throws naming **both**
/// sites — the one that fixed the kind and the one that did not match it —
/// and the first of those is a site the registry kept from an earlier call.
///
/// **The constant is always argument 0**, ahead of the receiver and of
/// everything the call wrote, which is [`WRITTEN_CLASS_MEMBERS`]' rule for its
/// own block and holds here for one more reason: `Core\Debug::dump` is
/// variadic, so a trailing slot would be one no reader of `args` can point at.
/// No member is on both rosters, and one that joined both would be deciding an
/// order between two blocks that each claim the front — a decision to take
/// then, not a collision to discover at a call site.
///
/// **The zero word is a producer with no call site**, which is the thunk a
/// callable reference synthesizes: that record is produced wherever the
/// callable is later invoked, and the envelope omits the field rather than
/// naming the line that wrote the reference.
///
/// **`Core\Debug::render` is deliberately not here.** It answers a *rendering*
/// — `rule:errors/debug-dump`'s nodes, without an envelope — so a source it was
/// handed would reach no reader, and a slot nothing reads is a slot that goes
/// wrong quietly.
pub const SOURCE_MEMBERS: &[(&str, &str)] = &[
    (crate::debug::NAME, "dump"),
    (crate::log::NAME, "write"),
    (crate::metrics::NAME, "increment"),
    (crate::metrics::NAME, "observe"),
    (crate::metrics::NAME, "gauge"),
];

/// Whether `class::method` is one of [`SOURCE_MEMBERS`].
#[must_use]
pub fn takes_source(class: &str, method: &str) -> bool {
    nvs_footprint::class(class);
    SOURCE_MEMBERS
        .iter()
        .any(|(owner, name)| *owner == class && *name == method)
}

/// The closed roster of members whose helper is handed **the class its call
/// site is inside**, as an extra trailing argument — the same
/// `nvs_ir::ir::InstKind::SourceConst` [`SOURCE_MEMBERS`] takes, read for its
/// enclosing `Class::member` half alone.
///
/// `rule:security/reflection-enforces-visibility` states its rule over the call
/// *site*, and a native member has no view of its caller: every question it can
/// ask of a receiver it was handed is a question about the receiver. So the
/// compiler supplies the answer as a constant of the call, which is what makes
/// it unforgeable — there is no argument position a program can write it in,
/// and no value it can reach that carries one. A member on this roster has
/// `args: [N]` **one more** than [`CoreMethod::params`] counts, receiver
/// included.
///
/// **The constant is always the last argument**, where [`SOURCE_MEMBERS`]' is
/// always argument 0, so the two rosters need no order decided between them: a
/// member on both reads the source at the front and the site at the back. A
/// trailing slot is affordable here and not there because no member on this
/// roster is variadic, which
/// `every_call_site_member_takes_a_fixed_argument_list` holds.
///
/// **The zero word is a call site inside no class at all** — a script frame, or
/// the thunk a callable reference synthesizes, which is invoked wherever it is
/// later passed rather than where it was written. Both read as *outside*, which
/// is the direction that fails closed: a reflective act attributed to no class
/// faces exactly the checks an out-of-class one faces.
///
/// **Three lowerings reach a `Core` member and every one of them emits this**,
/// which is what adding a member here owes: `nvs_ir::lower`'s instance-call and
/// static-call paths, and the callable-reference thunk that emits the zero
/// word. A path that skipped it would leave the helper reading the slot past
/// its own arguments — not a refusal but a slot nothing wrote.
pub const CALL_SITE_MEMBERS: &[(&str, &str)] = &[
    (crate::reflect::CLASS_INFO_NAME, "readableProperties"),
    (crate::reflect::CLASS_INFO_NAME, "get"),
    (crate::reflect::CLASS_INFO_NAME, "set"),
    (crate::reflect::CLASS_INFO_NAME, "constant"),
    (crate::reflect::CLASS_INFO_NAME, "call"),
    (crate::reflect::CLASS_INFO_NAME, "construct"),
];

/// Whether `class::method` is one of [`CALL_SITE_MEMBERS`].
#[must_use]
pub fn takes_call_site(class: &str, method: &str) -> bool {
    nvs_footprint::class(class);
    CALL_SITE_MEMBERS
        .iter()
        .any(|(owner, name)| *owner == class && *name == method)
}

/// The closed roster of members whose helper is handed **what the compiler
/// prepared out of the literal it was written with**, as an extra leading
/// argument — `rule:expressions/intrinsic-list-is-closed`'s roster reaching the
/// runtime, and the one channel it has.
///
/// `nvs_types::intrinsics` reads a written literal against its member's grammar
/// while checking, and preparation is what that read leaves behind:
/// `nvs_stdlib::regex::Tier` for a pattern, a parsed plan for the grammars that
/// do not travel this yet. A member on this roster is handed that answer instead
/// of deriving it again at the first call, which is the *second* effect
/// `rule:expressions/preparation-preserves-behaviour` asks for — the first,
/// refusing a malformed literal while compiling, needs no channel at all.
///
/// **What travels is the durable fact, never the object.** A compiled regex is
/// an `Rc` on one core's thread-local cache and an artifact cannot hold one, so
/// what the constant carries is the word the fact encodes to — the tier, here —
/// and the runtime spends what is left. That bound is why this is a roster of
/// members rather than a table of automata, and `nvs_ir::ir::Prepared` is the
/// closed set of facts a word can be.
///
/// **Argument 0, ahead of everything, and it is there for every call site.** A
/// member on this roster whose pattern the program computed is handed
/// `nvs_stdlib::regex::PREPARED_NONE`, exactly as [`WRITTEN_CLASS_MEMBERS`]'
/// third slot is written either way: a helper reads one ABI rather than
/// branching on what its call site wrote, and a slot nothing wrote is what
/// `nvs_ir::lower::Lowering::prepared_constant` exists to prevent. The helper's
/// `args: [N]` therefore counts **one** more than [`CoreMethod::params`] does.
///
/// [`SOURCE_MEMBERS`] names the same position, and no member is on both rosters:
/// what a producer is handed is where it was *called*, and what a prepared
/// member is handed is what its own argument said. A member that ever wanted
/// each would take this word first, which is the order `nvs-ir` emits them in.
///
/// **One slot, several grammars.** A row's word is its own grammar's, and the
/// words are minted from one space — [`crate::regex::PREPARED_LINEAR`] and
/// [`crate::cldr::PREPARED_PATTERN`] are different integers — so a word that
/// reached the wrong helper decodes as nothing prepared rather than as that
/// helper's own fact. The zero is the only word every row spells alike.
pub const PREPARED_MEMBERS: &[(&str, &str)] = &[
    (crate::regex::NAME, "compile"),
    (crate::time::DATETIME_NAME, "format"),
    (crate::time::TIME_NAME, "parse"),
];

/// Whether `class::method` is one of [`PREPARED_MEMBERS`].
#[must_use]
pub fn takes_prepared(class: &str, method: &str) -> bool {
    nvs_footprint::class(class);
    PREPARED_MEMBERS
        .iter()
        .any(|(owner, name)| *owner == class && *name == method)
}

/// The closed roster of `Core`-owned **generic** classes, each with the type
/// parameters it declares, in order — spec § 9's three collections.
///
/// This is the other half of `rule:attributes/call-site-type-argument`'s rule on
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
    // The first row that is not one of spec § 9's collections: § 18's result
    // set is generic in what a row hydrated into, and `Core\Db\Rows<Row>` — the
    // instance `query` answers with, spelled as a [`CoreTy::InstanceAt`] — is
    // the unhydrated case of the same class rather than a second one.
    (crate::db::ROWS_NAME, &["T"]),
    // § 18's other result, and generic for the same reason at the other end of
    // the memory trade: a walk carries what each row was built into, a
    // `Core\Db\Row` for `stream` and the call site's own class for
    // `streamAs<T>`, so the two members answer one class at two arguments
    // rather than two classes with one iteration protocol each.
    (crate::db::STREAM_NAME, &["T"]),
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
    // The first row whose element is a concrete type rather than one of the
    // receiver's own type variables: `Core\IO::lines` answers a walk over the
    // lines of a file, and a line is a `string` whatever the file was.
    (crate::io::LINES_NAME, &CoreTy::Str),
    // And the second, for the same reason: an entry of a walked tree is a
    // `string` whatever the tree held.
    (crate::io::WALK_NAME, &CoreTy::Str),
    // A streamed reply's three framings, whose elements are `tainted` for the
    // reason every byte of a reply is: pinning says where the octets came from
    // and nothing about what is in them.
    (
        crate::http::stream::EVENTS_NAME,
        &CoreTy::Instance(crate::http::stream::EVENT_NAME),
    ),
    (crate::http::stream::LINES_NAME, &CoreTy::TaintedStr),
    (crate::http::stream::CHUNKS_NAME, &CoreTy::TaintedBytes),
    // `rule:core-classes/db-statement-members`' `Db\Rows` under `foreach ($rows as Row
    // $row)`: a result set walks what it is already holding, which is its own `T` — a `Core\Db\Row` for `query`
    // and the hydrated class for `queryAs<T>`. Back to one of the receiver's
    // own variables, like the three collections above, because § 18's `Rows`
    // and `Rows<T>` are one generic class and not two.
    (crate::db::ROWS_NAME, &CoreTy::Var("T")),
    // § 18's `stream(): Iterable<Db\Row>` and `streamAs<T>(): Iterable<T>`,
    // which are one class at two arguments as the row above is: a walk yields
    // what it was opened to build, so the element is the receiver's own
    // variable and `nvs_types::expr::iteration` substitutes the argument in.
    (crate::db::STREAM_NAME, &CoreTy::Var("T")),
    // Spec § 15's `bodyStream(): Iterable<bytes>`, with the qualifier `body()`
    // puts on the same octets: a chunk of a request body is `tainted` whatever
    // the body held, so this is a concrete element like the two `Core\IO` rows
    // above and never one of a receiver's own variables.
    (crate::request::BODY_STREAM_NAME, &CoreTy::TaintedBytes),
    // `rule:http-server/an-upload-is-received-only-through-files`'s `files(): Iterable<Part>`. A concrete element again, and
    // the first one that is an *instance* rather than a scalar: what a
    // multipart body yields is a `Core\Request\Part` whatever the upload was,
    // so there is no receiver variable for it to be one of.
    (
        crate::request::FILES_NAME,
        &CoreTy::Instance(crate::request::PART_NAME),
    ),
    // § 12's `rows(): Core\Csv\Rows`. A concrete element like the `Core\IO`
    // rows above and for the same reason: a CSV record is an `array<string>`
    // whatever the document held, keyed by the header's names or by column
    // index, and neither keying is a type the receiver could be at.
    (crate::csv::ROWS_NAME, &CoreTy::Array(&CoreTy::Str)),
    // `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s `content(): Iterable<bytes>`, whose element is the body
    // walk's exactly: a chunk of an upload is a chunk of a request body with a
    // delimiter search in front of it, and `tainted` for the same reason.
    (crate::request::PART_CONTENT_NAME, &CoreTy::TaintedBytes),
];

/// The element type `class`'s `Iterable<T>` is fixed at, or `None` when it is
/// not one of [`ITERABLES`].
#[must_use]
pub fn iterable_element(class: &str) -> Option<&'static CoreTy> {
    nvs_footprint::class(class);
    ITERABLES
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, elem)| *elem)
}

/// Whether `class` satisfies `rule:classes/comparable`'s
/// `Comparable`, so `<`/`<=`/`>`/`>=`/`<=>` order two of its instances.
///
/// **Asked of the member roster, never of a list.** A `Core` class writes no
/// `implements` clause and `Comparable`'s whole content is one signature, so
/// the row *is* the declaration: an instance `compareTo` taking one argument
/// of this same class and answering `int` is exactly what the interface
/// requires, and a class carrying that row cannot be un-`Comparable`. A roster
/// beside it would be a second copy of the same fact, wrong the first time a
/// domain module adds the member without it — which is how the time types
/// spent this whole gap (`docs/reference/findings.md` D1).
///
/// The `self`-typed parameter is load-bearing rather than decoration: `rule:classes/ordering-lowers-to-compare-to` refuses a comparison across classes, so a `compareTo` taking anything
/// else answers a different question and is not this interface's member.
#[must_use]
pub fn implements_comparable(class: &str) -> bool {
    nvs_footprint::class(class);
    CLASSES
        .iter()
        .find(|found| found.name == class)
        .is_some_and(|found| {
            found.instance.iter().any(|member| {
                member.name == nvs_runtime::COMPARE_TO
                    && matches!(member.params, [CoreTy::Instance(param)] if *param == class)
                    && matches!(member.return_ty, CoreTy::Int)
            })
        })
}

/// The symbol of `name`'s `compareTo` — the implementation half of
/// [`implements_comparable`], and `None` for every class that declares no such
/// member.
///
/// Factored out for [`render_symbol`]'s reason, over the other of the two
/// members the engine reaches by name: `crate::instance` puts this address on
/// the class's own descriptor, so a `Core` object ordered through an erased
/// operand — the `Core\Heap` holding it, which knows no class — compares
/// through the very implementation this registry told the *checker* it would.
/// Asked of [`implements_comparable`] rather than of the roster directly, so a
/// row that answers the interface and a row that is merely spelled like it
/// cannot part company here.
#[must_use]
pub(crate) fn compare_symbol(name: &str) -> Option<&'static str> {
    if !implements_comparable(name) {
        return None;
    }
    class(name)
        .and_then(|class| {
            class
                .instance
                .iter()
                .find(|member| member.name == nvs_runtime::COMPARE_TO)
        })
        .map(|member| member.symbol)
}

/// Whether `class` implements `Parses` — `rule:expressions/try-parse`'s pair,
/// asked of the member roster for [`implements_comparable`]'s reason and no
/// other: a `Core` class writes no `implements` clause, so the two rows *are*
/// the declaration, and a roster beside them would be a second copy of the same
/// fact.
///
/// **The rule's three conditions are exactly what this encodes.** `parse` takes
/// one `string` and nothing else, so a row carrying a format or an options bag
/// beside the text answers a different question and is not this member.
/// `tryParse` is `parse` with the throw caught, so it takes that same one
/// parameter and answers the nullable of the same class — a nullable over
/// anything else is a different pair. And the spelling is `parse`/`tryParse`,
/// which is what the two name comparisons are.
///
/// **The classification is load-bearing rather than decoration.**
/// `Parses::parse` declares `tainted string $s`, and
/// `nvs_types::expr::quals`' `admits_tainted_argument` lets a tainted argument
/// through a [`Qual::Contagious`] parameter only where the *result* can carry
/// the qualifier back out — which an object never can, since `tainted` is a
/// property of `string` and `bytes` and not of a class. A `Contagious` row
/// would therefore refuse the interface's own argument type, and
/// [`Qual::Neutral`] is the one mark that both admits the text and says what is
/// true of what comes back: the parse checked it, and the object carries none
/// of it.
#[must_use]
pub fn implements_parses(class: &str) -> bool {
    fn reads_one_text(member: &CoreMethod) -> bool {
        matches!(member.params, [CoreTy::Text(Qual::Neutral)]) && member.defaults.is_empty()
    }

    nvs_footprint::class(class);
    CLASSES
        .iter()
        .find(|found| found.name == class)
        .is_some_and(|found| {
            let parses = found.methods.iter().any(|member| {
                member.name == "parse"
                    && reads_one_text(member)
                    && matches!(member.return_ty, CoreTy::Instance(answered) if answered == class)
            });
            let tries = found.methods.iter().any(|member| {
                member.name == "tryParse"
                    && reads_one_text(member)
                    && matches!(
                        member.return_ty,
                        CoreTy::Nullable(CoreTy::Instance(answered)) if *answered == class
                    )
            });
            parses && tries
        })
}

/// The type parameters `class` declares, in order — `None` when it is not one
/// of [`GENERIC_CLASSES`], which is every other name in the program.
#[must_use]
pub fn class_type_params(class: &str) -> Option<&'static [&'static str]> {
    nvs_footprint::class(class);
    GENERIC_CLASSES
        .iter()
        .find(|(name, _)| *name == class)
        .map(|(_, params)| *params)
}

/// Looks a `Core`-owned enum up by its fully-qualified name.
#[must_use]
pub fn core_enum(name: &str) -> Option<&'static CoreEnum> {
    nvs_footprint::class(name);
    ENUMS.iter().find(|found| found.name == name)
}

/// `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`'s
/// obligation, as the two option names it is written over: a member whose verb
/// repeats an *effect* may ask for retries only alongside an idempotency key.
///
/// Two names rather than a `bool` so that the checker reporting it holds no
/// copy of either spelling — the rows and the rule read the same
/// `crate::http` constants, which is the arrangement [`CoreOption`]'s own docs
/// ask for wherever a name is stated twice.
#[derive(Clone, Copy, Debug)]
pub struct IdempotentRetry {
    /// The option that turns retries on. Written at a call site, it is what
    /// makes the key compulsory; **left out it obliges nothing**, because an
    /// omitted count is not a retry — no `[http.client]` figure turns one on
    /// behind the call, so there is no second attempt for a key to identify.
    pub asks: &'static str,
    /// The option that must accompany it, sent as `Idempotency-Key` and
    /// identical across attempts.
    pub key: &'static str,
}

/// Whether `class::member` carries [`IdempotentRetry`]'s obligation — `None`
/// for every member that retries freely, which is every other one in `Core`.
///
/// Asked by name for [`implements_comparable`]'s reason: a `Core` row has no
/// cell for a rule this narrow, and a flag on [`CoreMethod`] would be a column
/// six hundred rows wide to say one thing about one of them. The roster is
/// § 7's: the verbs that repeat an effect rather than restate a question, which
/// is `post` and `patch` and nothing else `Core\Http\Client` offers.
#[must_use]
pub fn idempotent_retry_rule(class: &str, member: &str) -> Option<IdempotentRetry> {
    nvs_footprint::class(class);
    (class == crate::http::CLIENT_NAME && matches!(member, "post" | "patch")).then_some(
        IdempotentRetry {
            asks: crate::http::RETRY_ATTEMPTS_OPTION,
            key: crate::http::RETRY_KEY_OPTION,
        },
    )
}

/// `rule:http-server/an-outbound-request-carries-one-body`'s one-body rule, as
/// the key names it is written over — [`IdempotentRetry`]'s shape, for its
/// reason: the checker that reports the refusal holds no copy of a spelling that
/// `crate::http`'s [`CoreOption`]s declare.
#[derive(Clone, Copy, Debug)]
pub struct RequestBody {
    /// The keys a body may be written under, of which at most one may appear at
    /// a call.
    pub keys: &'static [&'static str],
    /// The one of [`Self::keys`] that carries raw octets, and so the only one
    /// [`Self::content_type`] has anything to say about.
    pub raw: &'static str,
    /// The key that types those octets and says nothing on its own.
    pub content_type: &'static str,
    /// Whether this member's verb carries no body at all, which makes any of
    /// [`Self::keys`] a refusal rather than a choice.
    pub bodyless: bool,
}

/// Which body keys `class::member` accepts — `None` for every member that takes
/// no body option, which is every other one in `Core`.
///
/// Asked by name for [`idempotent_retry_rule`]'s reason, and it answers `Some`
/// for a member that accepts a body as well as for one that refuses every key:
/// `get` and `head` are the verbs `rule:http-server/an-outbound-request-carries-one-body`
/// has no body for, and a rule that answered `None` for them would read as "this
/// member is outside the rule" where what is meant is "this member's answer is
/// no".
///
/// `request` is in the roster and is `bodyless: false`, which is the half of
/// the rule a call site can still be held to when the verb is an argument: two
/// keys written at once, or a `contentType` typing octets that are not there,
/// are wrong under every verb. Whether *that* verb carries a body at all is
/// asked at the call instead, by `crate::http`'s `judge_verb`.
#[must_use]
pub fn request_body_rule(class: &str, member: &str) -> Option<RequestBody> {
    nvs_footprint::class(class);
    (class == crate::http::CLIENT_NAME
        && matches!(
            member,
            "get" | "post" | "put" | "patch" | "delete" | "head" | "request"
        ))
    .then_some(RequestBody {
        keys: crate::http::BODY_OPTIONS,
        raw: crate::http::BODY_OPTION,
        content_type: crate::http::CONTENT_TYPE_OPTION,
        bodyless: matches!(member, "get" | "head"),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A class's signatures name the classes its members take and return, so a program typed
    /// against `Core\Process` is recorded as using `Core\Process\Result` too.
    #[test]
    fn a_signature_use_records_the_classes_its_members_name() {
        let named = signature_classes("core\\process");
        assert!(named.contains(&"Core\\Process\\Result"), "{named:?}");
        assert!(!named.contains(&"Core\\Process"), "{named:?}");
        let ((), lines) = nvs_footprint::capture(|| record_signature("Core\\Process"));
        assert_eq!(
            lines.first().map(String::as_str),
            Some("class\tCore\\Process")
        );
        assert!(
            lines.contains(&"class\tCore\\Process\\Result".to_string()),
            "{lines:?}"
        );
        assert!(signature_classes("App\\Helper").is_empty());
    }

    /// `rule:expressions/try-parse`: a class with a `tryParse` declares no `isValid`,
    /// because `Core\Uri::isValid($s)` and `Core\Uri::tryParse($s) != null`
    /// are one predicate and R17 keeps one spelling of it. Named for
    /// `Core\Uri` because that is the one where the deletion cost something:
    /// its `isValid` asked the *narrower* "is this an absolute URI", which is
    /// now `->scheme() != null` on the parsed value — one reader call rather
    /// than a second implementation of the grammar, which is the drift
    /// `rule:expressions/nullable-conversion` cites CVE-2024-5458 for.
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

    /// `rule:expressions/try-parse`'s three conditions, held mechanically: the class has a
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
                    "{}::{name} spells a `try…` `rule:core-api/shape-rules` R5 bans",
                    class.name
                );
                // R5's `…Safe` ban reads the suffix as a claim about *failure*,
                // and `rule:core-classes/schema-apply-capability`'s `applySafe` is the one member where it is
                // an adjective of the subject instead: it is named for the
                // grade of the steps it will run, throws where its sibling
                // `applyIncludingRisky` does not, and is the more refusing of
                // the two. R5's own cell records the exception, which is why
                // this is one spelling and not a list to grow.
                let adjective = class.name == crate::db::SCHEMA_NAME && name == "applySafe";
                assert!(
                    adjective
                        || (!name.ends_with("OrNull")
                            && !name.ends_with("Safe")
                            && !name.ends_with("Ex")),
                    "{}::{name} spells a non-throwing variant `rule:core-api/shape-rules` R5 bans",
                    class.name
                );
            }
        }
    }

    /// Every registered name is one the spec's own naming rules allow: a
    /// class under `Core`, a `camelCase` member (`rule:core-api/identifier-casing`), and no leading
    /// underscore anywhere (`rule:classes/no-leading-underscore-identifiers`). Cheap, and it catches a paste error in
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

    /// `rule:core-api/shape-rules` R2, mechanically: at most one options bag per member, always
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
        /// The fixture's reference card — `rule:core-api/reference-card`, so that no row anywhere in
        /// the crate is left undocumented.
        const SAMPLE_DOC: MethodDoc = MethodDoc {
            short: "A fixture rather than a member: the row this test reads its `<K, V, R>` \
                    order from.",
            params: &[
                ParamDoc {
                    name: "a",
                    desc: "An array whose element type is the written `K`.",
                    shape: &[],
                },
                ParamDoc {
                    name: "key",
                    desc: "One `K`, written at the call site.",
                    shape: &[],
                },
                ParamDoc {
                    name: "spare",
                    desc: "An optional `?V`, `null` by default.",
                    shape: &[],
                },
            ],
            ret: "The written `R`.",
            errors: &[],
        };
        const METHOD: CoreMethod = CoreMethod {
            name: "sample",
            names: &["a", "key"],
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
            doc: Some(&SAMPLE_DOC),
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
                CoreTy::Var(name) | CoreTy::ShapeOfCallables(name) => {
                    found.push(name);
                }
                CoreTy::Array(inner)
                | CoreTy::Nullable(inner)
                | CoreTy::Variadic(inner)
                | CoreTy::Iterated(inner) => {
                    inferred(inner, found);
                }
                CoreTy::Union(members) | CoreTy::InstanceAt(_, members) => {
                    members.iter().for_each(|m| inferred(m, found));
                }
                CoreTy::CallableSig(params, ret) => {
                    params.iter().for_each(|p| inferred(p, found));
                    inferred(ret, found);
                }
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

    /// `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`'s
    /// pairing, over an options bag — the same rule
    /// [`a_nullable_shape_field_omits_as_the_never_written_marker`] holds one
    /// level down, and `rule:core-api/one-checked-shape-type` is why one rule
    /// covers both spellings. A bag flattens to one ABI argument per option
    /// and the [`Const`] an omitted one passes has no union-shaped spelling,
    /// so an option admitting `null` states its omission with
    /// [`Const::NeverWritten`] and one admitting none states it with
    /// [`Const::Null`] or a literal. Getting that pairing wrong is what would
    /// make an omitted option and a written `null` the same argument.
    #[test]
    fn a_nullable_option_omits_as_the_never_written_marker() {
        for class in CLASSES {
            for method in class.members() {
                for member in method.options().unwrap_or(&[]) {
                    assert_eq!(
                        admits_null(&member.ty),
                        matches!(member.default, Const::NeverWritten),
                        "{}::{}'s option `{}` pairs a `{}` type with a `{:?}` default, so an \
                         omitted option and a written one would reach the helper as the same \
                         argument",
                        class.name,
                        method.name,
                        member.name,
                        if admits_null(&member.ty) {
                            "nullable"
                        } else {
                            "non-nullable"
                        },
                        member.default
                    );
                }
            }
        }
    }

    /// A [`CoreTy::Shape`] flattens into one ABI argument per field of its
    /// merged arms, so it only means anything as a *whole parameter*: nested
    /// in an array, a union, an option or another shape's field there would be
    /// nothing for it to flatten into — `rule:core-api/shape-parameter`, the restriction
    /// `a_shape_of_callables_is_only_ever_a_whole_parameter` already holds
    /// for [`CoreTy::ShapeOfCallables`]. The emptiness half rides along here because
    /// it is the same walk: a shape with no arms, or an arm with no fields,
    /// accepts nothing a call site could write.
    #[test]
    fn a_shape_is_only_ever_a_whole_parameter() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Shape(_) => true,
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
                // The one row a helper never answers: `nvs_types::program`
                // expands `implementingWith` and builds each `{instance,
                // attribute}` row itself, and the row spells the shape so the
                // card prints what the call answers. See `crate::program::ROW`.
                let expanded =
                    class.name == crate::program::NAME && method.name == "implementingWith";
                assert!(
                    expanded || !nests_one(&method.return_ty),
                    "{}::{} returns a shape, which has no runtime representation to answer with",
                    class.name,
                    method.name
                );
                for param in method.params {
                    let CoreTy::Shape(arms) = param else {
                        assert!(
                            !nests_one(param),
                            "{}::{} nests a shape inside a parameter",
                            class.name,
                            method.name
                        );
                        continue;
                    };
                    assert!(
                        !arms.is_empty(),
                        "{}::{} declares a shape with no arms",
                        class.name,
                        method.name
                    );
                    for arm in *arms {
                        assert!(
                            !arm.is_empty(),
                            "{}::{} declares a shape arm with no fields",
                            class.name,
                            method.name
                        );
                        for field in *arm {
                            assert!(
                                !nests_one(&field.ty),
                                "{}::{}'s shape field `{}` is itself a shape",
                                class.name,
                                method.name,
                                field.name
                            );
                        }
                    }
                }
            }
        }
    }

    /// `rule:core-api/shape-arms-are-disjoint`: checking a written literal is "exactly one arm accepts
    /// it", so two arms that could both accept one is a **registry** bug and
    /// is refused here rather than at a call site. A pair is proved disjoint
    /// either by a field name both declare whose declared types share no
    /// value, or by one arm requiring a key the other does not declare at all
    /// — an exact-key check refuses the extra key, which is the whole of why
    /// `Db\Settings` needs no declared discriminant.
    #[test]
    fn a_shapes_arms_are_pairwise_disjoint() {
        /// The values a type admits, as atoms, or `None` for a type that is
        /// not a closed set of them. `rule:types/literal-types`'s enum-case types are what
        /// separate real arms, so this stays deliberately small: anything
        /// wider is simply not a proof, and the pair must be separated by a
        /// required key instead.
        fn atoms(ty: &CoreTy) -> Option<Vec<String>> {
            match ty {
                CoreTy::EnumCase(name, case) => Some(vec![format!("{name}::{case}")]),
                CoreTy::IntLiteral(value) => Some(vec![format!("int {value}")]),
                CoreTy::Union(members) => {
                    let mut all = Vec::new();
                    for member in *members {
                        all.extend(atoms(member)?);
                    }
                    Some(all)
                }
                _ => None,
            }
        }
        fn types_are_disjoint(left: &CoreTy, right: &CoreTy) -> bool {
            match (atoms(left), atoms(right)) {
                (Some(left), Some(right)) => !left.iter().any(|one| right.contains(one)),
                _ => false,
            }
        }
        fn requires_a_key_the_other_lacks(arm: &[CoreField], other: &[CoreField]) -> bool {
            arm.iter().any(|field| {
                field.default.is_none() && !other.iter().any(|one| one.name == field.name)
            })
        }
        for class in CLASSES {
            for method in class.members() {
                for param in method.params {
                    let CoreTy::Shape(arms) = param else {
                        continue;
                    };
                    for (index, arm) in arms.iter().enumerate() {
                        for other in &arms[index + 1..] {
                            let separated = arm.iter().any(|field| {
                                other
                                    .iter()
                                    .filter(|one| one.name == field.name)
                                    .any(|one| types_are_disjoint(&field.ty, &one.ty))
                            }) || requires_a_key_the_other_lacks(arm, other)
                                || requires_a_key_the_other_lacks(other, arm);
                            assert!(
                                separated,
                                "{}::{}'s shape arms {index} and a later one are not provably \
                                 disjoint, so a literal could be accepted by both and arm \
                                 selection would have to guess",
                                class.name, method.name
                            );
                        }
                    }
                }
            }
        }
    }

    /// `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`'s
    /// pairing, over a shape's arms: every slot the written literal does not
    /// fill passes a [`Const`], so "filled" is only readable while the
    /// constant an unfilled slot passes is one the field's own type cannot
    /// also hold. A field admitting `null` — a [`CoreTy::Nullable`], a
    /// [`CoreTy::Union`] with a null arm, or [`CoreTy::Mixed`], which admits
    /// one without spelling it — is therefore admitted exactly where its
    /// default is [`Const::NeverWritten`], and a field admitting none is
    /// admitted exactly where its default is anything else.
    #[test]
    fn a_nullable_shape_field_omits_as_the_never_written_marker() {
        for class in CLASSES {
            for method in class.members() {
                for param in method.params {
                    let CoreTy::Shape(arms) = param else {
                        continue;
                    };
                    for arm in *arms {
                        for field in *arm {
                            let Some(default) = field.default else {
                                // Required: no call site omits it, so there is
                                // no constant to pair with anything.
                                continue;
                            };
                            assert_eq!(
                                admits_null(&field.ty),
                                matches!(default, Const::NeverWritten),
                                "{}::{}'s shape field `{}` pairs a `{}` type with a `{default:?}` \
                                 default, so an omitted field and a written one would reach the \
                                 helper as the same argument",
                                class.name,
                                method.name,
                                field.name,
                                if admits_null(&field.ty) {
                                    "nullable"
                                } else {
                                    "non-nullable"
                                }
                            );
                        }
                    }
                }
            }
        }
    }

    /// The tag the slot an omitting call site fills carries, or `None` where
    /// the default is a **literal** of the option's own type.
    ///
    /// The two guards above hold which [`Const`] a row declares; this is the
    /// value half of the same question, because a helper branches on a tag
    /// rather than on a registry constant. It goes through
    /// `nvs_runtime::Value`'s own constructors rather than naming a tag, so a
    /// runtime that moved either one moves this with it. A literal states no
    /// third state at all — it is a value a call site could equally have
    /// written — which is why there is nothing here to hold about one.
    fn omitted_tag(default: Const) -> Option<nvs_runtime::Tag> {
        match default {
            Const::NeverWritten => nvs_runtime::Value::unset().tag(),
            Const::Null => nvs_runtime::Value::null().tag(),
            _ => None,
        }
    }

    /// `rule:core-api/the-bag-abi-is-unchanged`'s one moving part, read off
    /// every registered row: which constant fills an omitted slot. A nullable
    /// option or shape field fills it with the never-written marker, whose tag
    /// no value the type system can spell carries
    /// (`rule:core-api/the-marker-never-reaches-a-program`); everything else
    /// fills it with a null or a literal, and lowers exactly as it did before
    /// the distinction existed.
    ///
    /// The count at the end is what keeps this from passing vacuously the day
    /// the rows stop spending the distinction: `Core\Uri::with`'s three
    /// removable components are the floor
    /// (`rule:core-classes/uri-removable-components`).
    #[test]
    fn a_nullable_option_omits_as_unset_and_a_non_nullable_one_omits_as_null() {
        let marker = nvs_runtime::Value::unset().tag();
        let mut nullable = 0;
        // One walk over both spellings of the same checked type
        // (`rule:core-api/one-checked-shape-type`), so an option and a shape
        // field cannot come to fill a slot differently.
        for class in CLASSES {
            for method in class.members() {
                let options = method.options().unwrap_or(&[]);
                let fields = method.params.iter().flat_map(|param| match param {
                    CoreTy::Shape(arms) => arms.iter().flat_map(|arm| arm.iter()).collect(),
                    _ => Vec::new(),
                });
                let declared = options
                    .iter()
                    .map(|option| (option.name, &option.ty, Some(option.default)))
                    .chain(fields.map(|field| (field.name, &field.ty, field.default)));
                for (name, ty, default) in declared {
                    // Required: no call site omits it, so no constant fills
                    // anything.
                    let Some(default) = default else {
                        continue;
                    };
                    if admits_null(ty) {
                        nullable += 1;
                        assert_eq!(
                            omitted_tag(default),
                            marker,
                            "{}::{}'s `{name}` admits `null`, so an omission that arrived as one \
                             would be the argument a written `null` already is",
                            class.name,
                            method.name
                        );
                    } else {
                        assert_ne!(
                            omitted_tag(default),
                            marker,
                            "{}::{}'s `{name}` has no `null` for the marker to be told apart \
                             from, so it would be a third state no helper can read",
                            class.name,
                            method.name
                        );
                    }
                }
            }
        }
        assert!(
            nullable >= 3,
            "only {nullable} registered field(s) spend the omitted-versus-null distinction, \
             which is fewer than `Core\\Uri::with`'s three removable components"
        );
    }

    /// Why the checker's refusal is load-bearing rather than a courtesy: a
    /// non-nullable option's *omission* already arrives under `Tag::Null`, so
    /// a written `null` that got past the checker would be the very argument
    /// the helper reads as **not given**. There is no run-time state left for
    /// one to occupy, and no member can grow a removal for a component whose
    /// row does not admit one.
    /// `tests/conformance/reject/a-uri-component-with-no-removal-refuses-a-written-null.nvst`
    /// pins that refusal from the language side; this holds the rows it rests
    /// on.
    ///
    /// `Core\Uri::with` is named directly at the end because
    /// `rule:core-classes/uri-removable-components` splits its six components
    /// three and three on exactly this question — a fourth removal, or a lost
    /// one, is a rule change rather than a row edit.
    #[test]
    fn a_null_written_into_a_non_nullable_option_is_still_refused() {
        let written_null = nvs_runtime::Value::null().tag();
        for class in CLASSES {
            for method in class.members() {
                for option in method.options().unwrap_or(&[]) {
                    if admits_null(&option.ty) {
                        continue;
                    }
                    assert!(
                        omitted_tag(option.default).is_none()
                            || omitted_tag(option.default) == written_null,
                        "{}::{}'s option `{}` omits as {:?}, which is neither the null a written \
                         one would be nor a literal of its own type",
                        class.name,
                        method.name,
                        option.name,
                        option.default
                    );
                }
            }
        }

        let with = crate::uri::CLASS
            .members()
            .find(|method| method.name == "with")
            .expect("`Core\\Uri::with` is registered");
        let removable: Vec<&str> = with
            .options()
            .expect("`with` takes one options bag and nothing else")
            .iter()
            .filter(|option| admits_null(&option.ty))
            .map(|option| option.name)
            .collect();
        assert_eq!(removable, ["port", "query", "fragment"]);
    }

    /// Whether a declared type admits a written `null` — the left-hand column
    /// of `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`'s
    /// pairing, shared by the two guards that hold it so an option and a shape
    /// field cannot come to answer it differently.
    fn admits_null(ty: &CoreTy) -> bool {
        match ty {
            CoreTy::Nullable(_) | CoreTy::Mixed => true,
            CoreTy::Union(members) => members.iter().any(admits_null),
            _ => false,
        }
    }

    /// The members that still owe `rule:security/unclassified-parameter-refuses-tainted` a classification, frozen at the
    /// size the gate below landed at.
    ///
    /// **Only deletions.** A member reaches this list once, when the gate is
    /// written, and leaves it when its parameters are classified;
    /// `every_member_parameter_carries_a_qualifier_classification` fails on a
    /// member that is here *and* classified, so an entry cannot go stale and a
    /// new member cannot be added to it.
    const UNCLASSIFIED: &[(&str, &str)] = &[
        ("Core\\Hash", "of"),
        ("Core\\Hash", "hmac"),
        ("Core\\Hash", "equals"),
        ("Core\\Hash\\Stream", "update"),
        ("Core\\Router", "url"),
        ("Core\\Router", "urlAbsolute"),
        ("Core\\Csv", "parse"),
        ("Core\\Csv", "format"),
    ];

    /// `rule:security/unclassified-parameter-refuses-tainted`: every `string`/`bytes` parameter of a `Core` member
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
    /// element type; `rule:security/unclassified-parameter-refuses-tainted` is written about the parameter, and
    /// classifying an element type would be a claim about flow through a
    /// container that no member makes yet.
    #[test]
    fn every_member_parameter_carries_a_qualifier_classification() {
        fn unclassified(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Nullable(inner) | CoreTy::Variadic(inner) => unclassified(inner),
                CoreTy::Union(members) => members.iter().any(unclassified),
                CoreTy::Options(options) => options.iter().any(|option| unclassified(&option.ty)),
                other => other.is_unclassified_string(),
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

    /// A union parameter answers the mark its arms declare, both halves:
    /// `Core\Regex`'s `Pattern|string` positions are
    /// `rule:security/regex-pattern-is-a-sink`'s sink because their text arm
    /// says so, and a union that declares nothing usable answers `None`, which
    /// refuses.
    ///
    /// The table asks the second half where the registry has no row to ask it
    /// of: a disagreement between two arms, and an arm that could carry a mark
    /// and does not.
    #[test]
    fn a_union_parameter_carries_the_classification_its_arms_declare() {
        let mut positions = 0;
        for method in crate::regex::CLASS.members() {
            for param in method.params {
                if matches!(param, CoreTy::Union(_)) {
                    positions += 1;
                    assert_eq!(
                        param.classification(),
                        Some(Qual::Sink),
                        "`Core\\Regex::{}`'s `Pattern|string` no longer reads as a sink, so its \
                         refusal of a tainted pattern is back to being the default",
                        method.name
                    );
                }
            }
        }
        assert!(
            positions >= 6,
            "{positions} `Pattern|string` position(s) found, and the rows that take one are the \
             reason this reads a mark out of a union at all"
        );

        const KEY: &[CoreTy] = &[CoreTy::Int, CoreTy::Str];
        const NEUTRAL_KEY: &[CoreTy] = &[CoreTy::Int, CoreTy::Text(Qual::Neutral)];
        const DATA: &[CoreTy] = &[
            CoreTy::Blob(Qual::Contagious),
            CoreTy::Text(Qual::Contagious),
        ];
        const DISAGREEING: &[CoreTy] = &[CoreTy::Blob(Qual::Sink), CoreTy::Text(Qual::Neutral)];

        assert_eq!(
            CoreTy::Union(KEY).classification(),
            None,
            "an `int|string` whose text arm is the unclassified spelling declares nothing"
        );
        assert_eq!(
            CoreTy::Union(NEUTRAL_KEY).classification(),
            Some(Qual::Neutral),
            "one arm carries the mark and the `int` beside it was never a qualifier question"
        );
        assert_eq!(
            CoreTy::Union(DATA).classification(),
            Some(Qual::Contagious),
            "two arms agreeing are the mark, not a disagreement"
        );
        assert_eq!(
            CoreTy::Union(DISAGREEING).classification(),
            None,
            "two arms disagreeing answer the refusing default rather than either one"
        );
    }

    /// [`implements_parses`] at the one class that satisfies it, and at every
    /// class carrying neither half of the pair — the ordinary bound beside
    /// [`the_parses_roster_is_the_classes_carrying_the_whole_pair`]'s two near
    /// misses, which are the classes carrying half of it.
    #[test]
    fn implements_parses_is_true_for_core_uuid_and_false_for_a_class_without_both_members() {
        assert!(implements_parses(crate::uuid::NAME));

        // Swept rather than named, because which class carries neither member
        // is the roster's business and changes without this question changing:
        // a class with no `parse` and no `tryParse` has nothing for either name
        // comparison to find, whatever else it declares.
        for class in CLASSES {
            let half = class
                .methods
                .iter()
                .any(|member| member.name == "parse" || member.name == "tryParse");
            assert!(
                half || !implements_parses(class.name),
                "`{}` carries neither half of the pair and is not an implementor",
                class.name
            );
        }

        // A name the roster does not carry answers `false` rather than
        // panicking. The predicate is asked by a binding site about whatever
        // type a parameter declared, which is most often a user's class and not
        // a `Core` one at all — that answer comes from the other table
        // (`nvs_types::commands`' `reaches_parses`), and this one has to hand it
        // over rather than assume the name is its own.
        assert!(!implements_parses(r"App\Models\Slug"));
        assert!(!implements_parses("Parses"));
    }

    /// The half of the pair the interface is read through: `tryParse` **is**
    /// `parse` with the throw caught, so it reads that same one text and
    /// answers the nullable of its own class. A nullable over anything else is
    /// a different pair — asserted as the shape the rows carry rather than by
    /// restating [`implements_parses`]' own match arms.
    #[test]
    fn implements_parses_requires_try_parse_to_answer_the_nullable_self() {
        let uuid = CLASSES
            .iter()
            .find(|class| class.name == crate::uuid::NAME)
            .expect("`Core\\Uuid` is on the roster");
        let try_parse = uuid
            .methods
            .iter()
            .find(|member| member.name == "tryParse")
            .expect("the pair's second half");
        assert!(
            matches!(try_parse.params, [CoreTy::Text(Qual::Neutral)])
                && try_parse.defaults.is_empty(),
            "the same one text `parse` reads, with nothing beside it"
        );
        assert!(
            matches!(
                try_parse.return_ty,
                CoreTy::Nullable(CoreTy::Instance(answered)) if *answered == crate::uuid::NAME
            ),
            "the nullable of its own class, which is what makes it `parse` with the throw caught"
        );

        // The sweep one row cannot make: no class anywhere on the roster
        // carries a `tryParse` answering the nullable of *another* class, so
        // the predicate never has to choose between two readings of the pair,
        // and a member that grew one fails here before it reaches a binding
        // site.
        for class in CLASSES {
            for member in class.methods {
                assert!(
                    member.name != "tryParse"
                        || matches!(
                            member.return_ty,
                            CoreTy::Nullable(CoreTy::Instance(answered))
                                if *answered == class.name
                        ),
                    "`{}::tryParse` answers something other than the nullable of its own class",
                    class.name
                );
            }
        }
    }

    /// [`implements_parses`]' answer over the whole roster rather than at the
    /// one class that satisfies it. A member grown beside the pair, a `parse`
    /// reclassified, or a `tryParse` whose nullable stops naming its own class
    /// changes this list before it changes a binding site — which is the only
    /// place the interface is asked for.
    #[test]
    fn the_parses_roster_is_the_classes_carrying_the_whole_pair() {
        let implementors = CLASSES
            .iter()
            .map(|class| class.name)
            .filter(|name| implements_parses(name))
            .collect::<Vec<_>>();
        assert_eq!(
            implementors,
            vec![crate::uuid::NAME],
            "`Core\\Uuid` is the whole roster today"
        );

        assert!(
            !implements_parses(crate::uri::NAME),
            "`Core\\Uri::parse` is `Qual::Contagious` over an object result, so it refuses the \
             `tainted string` the interface declares. Widening it is a decision and not a typo: a \
             URI's components come back out as plain `string`s, so a `Neutral` parse there would \
             launder attacker text through `scheme()` and `path()`, which is not what a checked \
             conversion buys"
        );
        assert!(
            !implements_parses(crate::time::DURATION_NAME),
            "`Core\\Time\\Duration::parse` reads one `Qual::Neutral` text and answers its own \
             class, and carries no `tryParse` — half the pair is not the interface"
        );
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

    /// [`CoreTy::ShapeOfCallables`] binds a variable its member's return type
    /// reads back, so it only means anything as a whole parameter: nested in
    /// an array, a union or an option it would name a variable nothing ever
    /// binds, and in return position it would name one at the moment it is
    /// meant to be read.
    #[test]
    fn a_shape_of_callables_is_only_ever_a_whole_parameter() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::ShapeOfCallables(_) => true,
                CoreTy::Array(elem)
                | CoreTy::Nullable(elem)
                | CoreTy::Variadic(elem)
                | CoreTy::Iterated(elem) => nests_one(elem),
                CoreTy::Union(members) => members.iter().any(nests_one),
                CoreTy::Options(options) => options.iter().any(|option| nests_one(&option.ty)),
                // A written signature is a type and may be nested anywhere,
                // but a shape of callables inside one is as meaningless as one
                // inside an array — the variable it names would be bound from
                // a position no argument occupies.
                CoreTy::CallableSig(params, ret) => params.iter().any(nests_one) || nests_one(ret),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.members() {
                assert!(
                    !nests_one(&method.return_ty),
                    "{}::{} returns a shape of callables",
                    class.name,
                    method.name
                );
                for param in method.params {
                    if matches!(param, CoreTy::ShapeOfCallables(_)) {
                        continue;
                    }
                    assert!(
                        !nests_one(param),
                        "{}::{} nests a shape of callables inside a parameter",
                        class.name,
                        method.name
                    );
                }
            }
        }
    }

    /// The variable a [`CoreTy::ShapeOfCallables`] binds is one the member
    /// actually reads back — a row naming `S` in the callback and `V` in the
    /// result would type-check every call to `mixed` with nothing to say why.
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
                    let CoreTy::ShapeOfCallables(name) = param else {
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

    /// One row, and one only, names a callable parameter that is not a written
    /// signature. `rule:types/callable-signature` is why: a signature says what
    /// a callback answers at a position every layer downstream already
    /// understands, so a `Core`-only spelling has to earn itself.
    ///
    /// `Core\Task::all` earns it, because its answer is a *shape* of its
    /// fields' results rather than one callback's and no written signature
    /// spells that: `rule:concurrency/all-answers-a-typed-shape` wants the
    /// argument's own field names back, and the call site is what chooses
    /// them. Asserted as the whole list rather than row by row, so a second
    /// row reaching for it says so here instead of passing quietly.
    #[test]
    fn only_task_all_names_a_shape_of_callables() {
        let mut rows = Vec::new();
        for class in CLASSES {
            for method in class.members() {
                for param in method.params {
                    if matches!(param, CoreTy::ShapeOfCallables(_)) {
                        rows.push(format!("{}::{}", class.name, method.name));
                    }
                }
            }
        }
        assert_eq!(
            rows,
            [r"Core\Task::all"],
            "a callback's result is its written signature's return type, not a \
             variable named beside an opaque `callable`"
        );
    }

    /// `rule:types/callable-signature`'s whole point, swept: a callback a
    /// `Core` member *calls* says what it is handed and what it must answer,
    /// so a mismatch is refused where the call is written rather than by
    /// `nvs_runtime::call_closure`'s per-argument tag test one frame in. That
    /// check is `rule:security/isolate-shares-nothing`'s guard between a
    /// mismatched argument and an arbitrary dereference, and a proven call
    /// site stops paying it.
    ///
    /// Two kinds of row still write bare [`CoreTy::Callable`], and both are
    /// named here so a third cannot join by accident. A **reference to a
    /// declaration** is not a callback at all: nothing in `Core\Attributes`
    /// calls `$target`, it reads the attributes off whatever declaration the
    /// reference names, so there is no signature the member could state. An
    /// **option bag's field** is not a parameter and is not swept — `{by?:
    /// callable}` and its six siblings still stand at the top of the lattice,
    /// which is `rule:types/callable-signature`'s own allowance rather than an
    /// oversight, and they have had no pass of their own yet.
    #[test]
    fn every_callback_parameter_declares_its_signature() {
        /// Whether a *parameter* is, or wraps, a bare `callable`. An
        /// options bag is not descended into: its fields are not parameters,
        /// and R2 gives them their own rules.
        fn bare(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Callable => true,
                CoreTy::Array(inner)
                | CoreTy::Nullable(inner)
                | CoreTy::Variadic(inner)
                | CoreTy::Iterated(inner) => bare(inner),
                CoreTy::Union(members) => members.iter().any(bare),
                _ => false,
            }
        }
        let rows = CLASSES
            .iter()
            .flat_map(|class| {
                class
                    .members()
                    .map(move |method| (class.name, method.name, method.params))
            })
            .chain(
                CONSTRUCTORS
                    .iter()
                    .map(|(class, method)| (*class, method.name, method.params)),
            );
        let mut unspelled = Vec::new();
        for (class, member, params) in rows {
            if params.iter().any(bare) {
                unspelled.push(format!("{class}::{member}"));
            }
        }
        unspelled.sort_unstable();
        assert_eq!(
            unspelled,
            [r"Core\Attributes::all", r"Core\Attributes::get"],
            "a `Core` callback writes `CoreTy::CallableSig`; a bare `callable` \
             parameter is either a reference to a declaration or a row that \
             has not been given its signature yet"
        );
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

    /// A `Core` enum's name and cases follow `rule:core-api/identifier-casing`'s casing rules too —
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

    /// Every [`CaseDoc`] names a case its enum actually declares, in the
    /// enum's own order — the check that keeps an enum's card and its case
    /// table from drifting apart, since `nvs meta --json` emits the card by
    /// name and a consumer looks a case up by it.
    #[test]
    fn every_enum_case_doc_names_a_real_case() {
        for declared in ENUMS {
            let Some(doc) = declared.doc else {
                continue;
            };
            let documented: Vec<&str> = doc.cases.iter().map(|case| case.name).collect();
            let cases: Vec<&str> = declared
                .cases
                .iter()
                .map(|(name, _)| *name)
                .filter(|name| documented.contains(name))
                .collect();
            for case in &documented {
                assert!(
                    cases.contains(case),
                    "{}'s card documents `{case}`, which is not a case",
                    declared.name
                );
            }
            assert_eq!(
                documented, cases,
                "{}'s card lists its cases out of declaration order",
                declared.name
            );
        }
    }

    /// `rule:core-api/shape-rules` R2's names, structurally: one per positional slot, in that
    /// order, `camelCase`, distinct within a row, and never the bag's own
    /// [`OPTIONS_NAME`] — which is not on the row at all.
    ///
    /// The alignment is what every consumer relies on, because a name resolves
    /// to a slot by its *index* here: `nvs_types::core_lib` reads
    /// [`CoreMethod::names`] the way it reads [`CoreMethod::positional`], and
    /// the two lists disagreeing would bind an argument to the wrong
    /// parameter rather than reject it. The spec column is
    /// `every_registry_rows_names_are_the_specs_signature_column`'s subject;
    /// this is the half that holds for the rows §§ 1-12 do not write.
    #[test]
    fn every_registry_row_names_one_parameter_per_positional_slot() {
        for class in CLASSES {
            for method in class.members() {
                assert_eq!(
                    method.names.len(),
                    method.positional().len(),
                    "{}::{} names {:?} for {} positional parameter(s)",
                    class.name,
                    method.name,
                    method.names,
                    method.positional().len()
                );
                for name in method.names {
                    assert!(
                        name.starts_with(|c: char| c.is_ascii_lowercase())
                            && name.chars().all(|c| c.is_ascii_alphanumeric()),
                        "{}::{}'s parameter `{name}` is not camelCase",
                        class.name,
                        method.name
                    );
                    assert_ne!(
                        *name, OPTIONS_NAME,
                        "{}::{} gives a positional parameter the trailing bag's own name",
                        class.name, method.name
                    );
                }
                let distinct: BTreeSet<&str> = method.names.iter().copied().collect();
                assert_eq!(
                    distinct.len(),
                    method.names.len(),
                    "{}::{} names two parameters the same: {:?}",
                    class.name,
                    method.name,
                    method.names
                );
            }
        }
    }

    /// [`CoreTy::Entry`] is checked at one call path only —
    /// `nvs_types::expr::calls`'s `infer_static_call`, where an argument's
    /// *written shape* is still visible — so the mark on an **instance** row
    /// would be a parameter that quietly accepts the `callable` `rule:security/isolate-shares-nothing`
    /// refuses. Nothing in the checker can see that mistake, and this roster
    /// can: the mark is legal on a static row and nowhere else.
    ///
    /// The second assertion is that some row still writes it. The rule's whole
    /// mechanism — the variant, `entry_parameter`, the hook — is reachable only
    /// through a marked row, so a registry that stopped marking one would leave
    /// every piece of it compiling and testing green while checking nothing.
    #[test]
    fn an_entry_parameter_is_declared_only_on_a_static_row() {
        let mut marked = 0_usize;
        for class in CLASSES {
            for method in class.instance {
                assert!(
                    !method.params.iter().any(|ty| matches!(ty, CoreTy::Entry)),
                    "{}::{} marks an isolate entry on an instance row, where the \
                     checker's rule never runs",
                    class.name,
                    method.name
                );
            }
            for method in class.methods {
                marked += method
                    .params
                    .iter()
                    .filter(|ty| matches!(ty, CoreTy::Entry))
                    .count();
            }
        }
        assert!(
            marked > 0,
            "`rule:concurrency/an-upgrade-is-spawn-shaped`'s `Core\\Socket::upgrade` declares one"
        );
    }

    /// Every member on [`CALL_SITE_MEMBERS`] is a registered row whose argument
    /// list has a fixed length, which is what makes the call site's constant
    /// readable as the **last** slot.
    ///
    /// A variadic member's own tail is its last slot and its position is what
    /// the site constant would take, so the two cannot share a member: that is
    /// [`SOURCE_MEMBERS`]' whole reason for claiming argument 0 instead, and
    /// this is the check that keeps the cheaper choice honest here.
    #[test]
    fn every_call_site_member_takes_a_fixed_argument_list() {
        for (owner, name) in CALL_SITE_MEMBERS {
            let class = self::class(owner).unwrap_or_else(|| {
                panic!("`{owner}` takes a call site and is not a registered class");
            });
            let member = class
                .members()
                .find(|member| member.name == *name)
                .unwrap_or_else(|| panic!("`{owner}::{name}` takes a call site and is no row"));
            assert!(
                member.variadic().is_none(),
                "`{owner}::{name}` is variadic, so its last argument is its own tail"
            );
        }
    }

    /// The classes that landed before a class carried a card of its own, and
    /// still owe one. A class is deleted from here the session it gains its
    /// [`ClassDoc`], the test below says so by name, and a class added after
    /// this list was written is never added to it: it lands with its card, as
    /// a member does. Goal `core-class-cards` is what empties the list.
    const CLASSES_STILL_OWING_A_CARD: &[&str] = &[
        r"Core\Arr",
        r"Core\Ast",
        r"Core\Ast\Node",
        r"Core\Attributes",
        r"Core\BigInt",
        r"Core\Budget",
        r"Core\Bytes",
        r"Core\Cache",
        r"Core\Cache\SecretEntry",
        r"Core\Cache\Store",
        r"Core\Cap",
        r"Core\Cldr",
        r"Core\Cli",
        r"Core\Cli\Color",
        r"Core\Cli\Live",
        r"Core\Cli\Progress",
        r"Core\Cli\Style",
        r"Core\Cli\Text",
        r"Core\Command",
        r"Core\Compress",
        r"Core\Compress\Compressor",
        r"Core\Compress\Decompressor",
        r"Core\Config",
        r"Core\Crypto",
        r"Core\Crypto\KeyPair",
        r"Core\Crypto\PublicKey",
        r"Core\Csrf",
        r"Core\Csv",
        r"Core\Csv\Rows",
        r"Core\Db",
        r"Core\Db\Column",
        r"Core\Db\Connection",
        r"Core\Db\InList",
        r"Core\Db\Plan",
        r"Core\Db\Plan\Step",
        r"Core\Db\Row",
        r"Core\Db\Rows",
        r"Core\Db\Schema",
        r"Core\Db\Stream",
        r"Core\Db\Transaction",
        r"Core\Db\Write",
        r"Core\Debug",
        r"Core\Decimal",
        r"Core\Encoding",
        r"Core\Env",
        r"Core\Fatal",
        r"Core\Hash",
        r"Core\Hash\Stream",
        r"Core\Heap",
        r"Core\Html",
        r"Core\Html\Markup",
        r"Core\Http",
        r"Core\Http\Client",
        r"Core\Http\Identity",
        r"Core\Http\Part",
        r"Core\Http\Target",
        r"Core\IO\Lines",
        r"Core\IO\Walk",
        r"Core\Regex",
        r"Core\Regex\Match",
        r"Core\Regex\Pattern",
        r"Core\Request",
        r"Core\Request\BodyStream",
        r"Core\Request\Files",
        r"Core\Request\Mount",
        r"Core\Request\Part",
        r"Core\Request\PartContent",
        r"Core\Response",
        r"Core\Response\Stream",
        r"Core\Router",
        r"Core\Router\Match",
        r"Core\Script",
        r"Core\Script\ExitReport",
        r"Core\Script\Handle",
        r"Core\Secret",
        r"Core\Serialize",
        r"Core\Server",
        r"Core\Session",
        r"Core\Signal",
        r"Core\Signature",
        r"Core\SignedCookie",
        r"Core\Socket",
        r"Core\Socket\Message",
        r"Core\Sse",
        r"Core\Sse\Message",
        r"Core\Storage",
        r"Core\Str",
        r"Core\Taint",
        r"Core\Task",
        r"Core\Task\Channel",
        r"Core\Test",
        r"Core\Test\Response",
        r"Core\Test\SentRequest",
        r"Core\Time",
        r"Core\Time\Date",
        r"Core\Time\DateTime",
        r"Core\Time\Duration",
        r"Core\Time\Instant",
        r"Core\Time\TimeOfDay",
        r"Core\Time\Zone",
        r"Core\Topic",
        r"Core\Totp",
        r"Core\Uri",
        r"Core\Uuid",
        r"Core\Validate",
        r"Core\Xml",
        r"Core\Xml\Node",
        r"Core\Xml\Reader",
        r"Core\Xml\Writer",
        r"Core\Zip",
    ];

    /// Every row, enum, constant and class carries its
    /// `rule:core-api/reference-card` card — the second of conventions.md's five
    /// edits, and the one nothing at a call site would miss. The types keep
    /// their `Option` and empty-string spellings for the emitter's sake
    /// ([`MethodDoc`] owns why), so *this* is where "not written yet" stops
    /// being a state a member can ship in. A class is the one exception, and
    /// only for the classes [`CLASSES_STILL_OWING_A_CARD`] names.
    ///
    /// A card's own fields are held to the same floor where a floor makes
    /// sense: a `short`, a `ret` and every `desc` are written, and
    /// [`MethodDoc::errors`] is the one field legitimately empty, since a
    /// member that throws nothing has nothing to list.
    #[test]
    fn every_registry_row_carries_a_reference_card() {
        let mut missing = Vec::new();
        for class in CLASSES {
            for method in class.members() {
                let Some(doc) = method.doc else {
                    missing.push(format!("{}::{} has no card", class.name, method.name));
                    continue;
                };
                if doc.short.trim().is_empty() {
                    missing.push(format!(
                        "{}::{}'s card has no `short`",
                        class.name, method.name
                    ));
                }
                if doc.ret.trim().is_empty() {
                    missing.push(format!(
                        "{}::{}'s card has no `ret`",
                        class.name, method.name
                    ));
                }
                for param in doc.params {
                    if param.desc.trim().is_empty() {
                        missing.push(format!(
                            "{}::{}'s card says nothing of `${}`",
                            class.name, method.name, param.name
                        ));
                    }
                    for key in param.shape {
                        if key.desc.trim().is_empty() {
                            missing.push(format!(
                                "{}::{}'s card says nothing of `{}.{}`",
                                class.name, method.name, param.name, key.key
                            ));
                        }
                    }
                }
                for error in doc.errors {
                    if error.desc.trim().is_empty() {
                        missing.push(format!(
                            "{}::{}'s card says nothing of when `{}` is thrown",
                            class.name, method.name, error.error
                        ));
                    }
                }
            }
            for constant in class.constants {
                if constant.desc.trim().is_empty() {
                    missing.push(format!(
                        "{}::{} has no description",
                        class.name, constant.name
                    ));
                }
            }
            match class.doc {
                Some(doc) if doc.short.trim().is_empty() => {
                    missing.push(format!("{}'s card has no `short`", class.name));
                }
                Some(_) => {}
                None if CLASSES_STILL_OWING_A_CARD.contains(&class.name) => {}
                None => missing.push(format!("{} has no card", class.name)),
            }
        }
        for owed in CLASSES_STILL_OWING_A_CARD {
            let carded = CLASSES
                .iter()
                .any(|class| class.name == *owed && class.doc.is_some());
            assert!(
                !carded,
                "{owed} now carries its card — delete it from `CLASSES_STILL_OWING_A_CARD`"
            );
        }
        for declared in ENUMS {
            let Some(doc) = declared.doc else {
                missing.push(format!("{} has no card", declared.name));
                continue;
            };
            if doc.short.trim().is_empty() {
                missing.push(format!("{}'s card has no `short`", declared.name));
            }
            for case in doc.cases {
                if case.desc.trim().is_empty() {
                    missing.push(format!(
                        "{}'s card says nothing of `{}`",
                        declared.name, case.name
                    ));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "{} registry entries ship undocumented — `rule:core-api/reference-card`, conventions.md's second edit:\n  {}",
            missing.len(),
            missing.join("\n  ")
        );
    }

    /// A card states the fact, never the decision's file name.
    ///
    /// `rule:core-api/reference-card`'s card is reference documentation for someone writing
    /// Novis, and it ships **raw** through `nvs meta --json`: `bun nv
    /// reference` rewrites citations on its way to the website, but that
    /// consumer never sees the rewrite, so "throws as `rule:core-classes/regex-two-tiers`'s two engines
    /// require" reaches a reader who has no ADR tree and cannot follow it.
    /// The rule is therefore on the card itself rather than on any renderer —
    /// say *what is true*, and leave the reason to the Rust doc comment
    /// directly above the card, which is where a contributor looks.
    #[test]
    fn no_registry_card_cites_an_adr() {
        /// The citation as it is written in prose: the bare word, or the
        /// possessive/section forms a card reaches for.
        fn cites(text: &str) -> bool {
            text.contains("ADR")
        }

        let mut cited = Vec::new();
        let mut note = |place: String, text: &str| {
            if cites(text) {
                cited.push(format!("{place}: {text}"));
            }
        };
        for class in CLASSES {
            for method in class.members() {
                let Some(doc) = method.doc else { continue };
                let member = format!("{}::{}", class.name, method.name);
                note(format!("{member} short"), doc.short);
                note(format!("{member} ret"), doc.ret);
                for param in doc.params {
                    note(format!("{member} ${}", param.name), param.desc);
                    for key in param.shape {
                        note(format!("{member} ${}.{}", param.name, key.key), key.desc);
                    }
                }
                for error in doc.errors {
                    note(format!("{member} throws {}", error.error), error.desc);
                }
            }
            for constant in class.constants {
                note(format!("{}::{}", class.name, constant.name), constant.desc);
            }
        }
        for declared in ENUMS {
            let Some(doc) = declared.doc else { continue };
            note(format!("{} short", declared.name), doc.short);
            for case in doc.cases {
                note(format!("{}::{}", declared.name, case.name), case.desc);
            }
        }
        assert!(
            cited.is_empty(),
            "{} card field(s) cite an ADR, which `nvs meta --json` ships verbatim \
             to a reader who has no ADR tree — state the fact instead:\n  {}",
            cited.len(),
            cited.join("\n  ")
        );
    }

    /// `rule:core-api/reference-card`'s card keys itself by `rule:core-api/shape-rules` R2's names, so the two cannot
    /// be written independently: [`MethodDoc::params`] is one entry per
    /// positional parameter under the row's own [`CoreMethod::names`], then
    /// one per option of a trailing bag under the option's name.
    ///
    /// Before this, a card's parameter names were the only place a name
    /// existed and nothing could check them — [`ParamDoc::name`]'s own docs
    /// record that. Now the row is the name's home and the card is a
    /// description hung off it, which is a thing a test can hold.
    #[test]
    fn a_documented_rows_param_docs_agree_with_its_names() {
        for class in CLASSES {
            for method in class.members() {
                let Some(doc) = method.doc else { continue };
                let mut want: Vec<&str> = method.names.to_vec();
                want.extend(method.options().unwrap_or(&[]).iter().map(|o| o.name));
                let found: Vec<&str> = doc.params.iter().map(|param| param.name).collect();
                assert_eq!(
                    found, want,
                    "{}::{}'s reference card documents {found:?}, but the row declares {want:?}",
                    class.name, method.name
                );
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
                CoreTy::InstanceAt(_, args) => {
                    for arg in *args {
                        check(arg, what);
                    }
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

    /// `rule:core-api/identifier-casing`'s `SCREAMING_SNAKE_CASE` for every registered constant, and
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

    /// The classes a [`CoreTy::Instance`] may name that [`CLASSES`] does not
    /// hold — spec § 10's exception tree, which `nvs_hir::errors::TREE`
    /// declares and `nvs_types::error_lib` seeds into the same class table a
    /// `Core` instance type is interned against.
    ///
    /// **`Core` is two rosters here exactly as it is for a `catch` name**, and
    /// this is the second one. It stays a written list rather than a reach into
    /// `nvs-hir` — which this crate does not depend on — because a row naming
    /// an exception class is a rare thing and a wrong name in one is caught by
    /// the conformance case that calls the member.
    /// `Core\Script\ExitReport::error` is the row that wanted it first.
    const EXCEPTION_TREE: &[&str] = &["Throwable"];

    /// The names a [`CoreTy::Instance`] may hold that are **interfaces** rather
    /// than classes — the compiler-declared global roster `nvs_hir::interfaces`
    /// holds, seeded into the same class table an instance type is interned
    /// against.
    ///
    /// A written list for [`EXCEPTION_TREE`]'s reason exactly, and a rare thing
    /// for the same reason: a member answers an interface only where the set of
    /// classes it means is open. `Core\Router\Match::params` is the row that
    /// wanted it first — a capture typed as a class built from text is any
    /// implementor of `Parses`, and no registered class names that set.
    const GLOBAL_INTERFACES: &[&str] = &[crate::router::PARSES_NAME];

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
                    class(name).is_some()
                        || EXCEPTION_TREE.contains(name)
                        || GLOBAL_INTERFACES.contains(name),
                    "{what} names the unregistered class `{name}`"
                ),
                // The same question, plus the one only this variant can get
                // wrong: the arguments are positional against the class's
                // [`GENERIC_CLASSES`] row, so a name that is not generic at all
                // or a count that does not match its row would intern a class
                // type the checker refuses at every call site.
                CoreTy::InstanceAt(name, args) => {
                    assert!(
                        class(name).is_some() || EXCEPTION_TREE.contains(name),
                        "{what} names the unregistered class `{name}`"
                    );
                    let params = class_type_params(name).unwrap_or_else(|| {
                        panic!("{what} writes arguments for `{name}`, which is not generic")
                    });
                    assert_eq!(
                        params.len(),
                        args.len(),
                        "{what} writes {} argument(s) for `{name}`, which takes {}",
                        args.len(),
                        params.len()
                    );
                    for arg in *args {
                        check(arg, what);
                    }
                }
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

    /// The classes `rule:security/protocol-roster` closes the list of, and the
    /// two that carry a key for one of them.
    ///
    /// Written rather than derived, for [`EXCEPTION_TREE`]'s reason and one of
    /// its own: the roster is closed by that rule, so a new entry is a
    /// deliberate act that edits this line, and a derivation over "every class
    /// handed a `secret`" would sweep in `Core\Crypto`'s primitives — which
    /// answer key material, and are the one place that is right.
    const PROTOCOLS: &[&CoreClass] = &[
        &crate::signed_cookie::CLASS,
        &crate::csrf::CLASS,
        &crate::totp::CLASS,
        &crate::jwt::CLASS,
        &crate::jwt::KEY_SET,
        &crate::jwe::CLASS,
        &crate::jwe::KEY,
        &crate::signature::CLASS,
    ];

    /// `rule:security/verification-throws-and-compares-in-constant-time`'s
    /// second sentence, off the rows: no member of a protocol class hands a
    /// secret-derived value back for the caller to compare.
    ///
    /// Two claims, because a raw-value accessor can be either. **Nothing
    /// answers secret material at all**, so a key never leaves the protocol it
    /// was handed to. And **nothing answers text it was not handed a key to
    /// derive** — which is exactly what an accessor is: a member reading a
    /// stored value off a carrier takes no key, while `Totp::code` and
    /// `Csrf::issue` are given the secret they derive from in the same call. A
    /// member that compares instead answers a `bool` or an `int` and is reached
    /// by neither claim, which is the rule's own point: where a token is
    /// compared, the comparison is the exposed operation.
    ///
    /// This is what stands in for a test of constant-time behaviour, which is
    /// not reliably measurable in CI — `docs/decisions/0060.md` § *Verification*
    /// asks for exactly this in its place.
    #[test]
    fn no_protocol_class_exposes_a_raw_value_accessor() {
        /// The classes a key reaches a member *inside*, since not every key is
        /// written as a `secret` leaf: an RSA key pair, a JWKS and a JWE key
        /// are objects holding one, and a row names the object.
        const CARRIERS: &[&str] = &[
            crate::crypto::KEY_PAIR_NAME,
            crate::jwt::KEY_SET_NAME,
            crate::jwe::KEY_NAME,
        ];

        /// Whether `ty` carries secret material anywhere inside it.
        fn secret(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::SecretBytes
                | CoreTy::SecretBlob(_)
                | CoreTy::SecretStr
                | CoreTy::SecretText(_)
                | CoreTy::SecretTaintedStr
                | CoreTy::SecretTaintedBytes => true,
                CoreTy::Array(inner)
                | CoreTy::Nullable(inner)
                | CoreTy::Variadic(inner)
                | CoreTy::Iterated(inner) => secret(inner),
                CoreTy::Union(members) | CoreTy::InstanceAt(_, members) => {
                    members.iter().any(secret)
                }
                CoreTy::Options(options) => options.iter().any(|option| secret(&option.ty)),
                CoreTy::Shape(arms) => arms
                    .iter()
                    .flat_map(|arm| arm.iter())
                    .any(|field| secret(&field.ty)),
                // A variant that carries no nested type carries no secret
                // either, and `CoreTy` is `non_exhaustive`.
                _ => false,
            }
        }

        /// Whether a parameter of `ty` hands the member key material — a
        /// `secret`, or one of the objects that holds one.
        fn keyed(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Instance(name) | CoreTy::InstanceAt(name, _) => CARRIERS.contains(name),
                CoreTy::Array(inner)
                | CoreTy::Nullable(inner)
                | CoreTy::Variadic(inner)
                | CoreTy::Iterated(inner) => keyed(inner),
                CoreTy::Union(members) => members.iter().any(keyed),
                CoreTy::Options(options) => options.iter().any(|option| keyed(&option.ty)),
                CoreTy::Shape(arms) => arms
                    .iter()
                    .flat_map(|arm| arm.iter())
                    .any(|field| keyed(&field.ty)),
                other => secret(other),
            }
        }

        /// Whether a value of `ty` is text a program could write `==` over —
        /// the shape a raw value comes back in, `bytes` included.
        fn comparable(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Str
                | CoreTy::Bytes
                | CoreTy::Text(_)
                | CoreTy::Blob(_)
                | CoreTy::TaintedStr
                | CoreTy::TaintedBytes => true,
                CoreTy::Nullable(inner) => comparable(inner),
                _ => false,
            }
        }

        for held in PROTOCOLS {
            for method in held.members() {
                let what = format!("{}::{}", held.name, method.name);
                assert!(
                    !secret(&method.return_ty),
                    "{what} answers secret material, which a protocol hands to no caller"
                );
                assert!(
                    !comparable(&method.return_ty) || method.params.iter().any(keyed),
                    "{what} answers text without being handed a key to derive it from, \
                     so it reads a value back for the caller to compare"
                );
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
    /// by name rather than given a member the spec does not write.
    /// `Core\Regex\Pattern` is read by `Core\Regex`'s members.
    /// `Core\Script\Handle`'s one slot is read by the lowering of `await`, and
    /// its emptiness of members is the whole point of it
    /// ([`crate::script`]). `Core\IO\Lines`'s one slot is read by the
    /// `iterate()` on [`crate::instance`]'s dispatch roster — spec § 14
    /// writes `lines(string $path): Iterable<string>` and no member *on* the
    /// thing it answers with, so a `foreach` is the whole of its surface.
    /// `Core\Http\Target`'s two slots are read by the member that connects:
    /// `rule:http-server/allow-url-pins-the-address` pins an approved address into it, and a member
    /// handing that address back would let a program rebuild the request
    /// around a different one ([`crate::http`]).
    /// `Core\Cli\Color` and `Core\Cli\Style` hold the slots a
    /// `Core\Cli\Text`'s runs are rendered from: `rule:tooling/styling-is-a-value-not-a-grammar` writes
    /// two constructors and a shape of options and no member on either result,
    /// because a style is built and worn rather than interrogated.
    /// `Core\Html\Markup` is the sink carrier that belongs here —
    /// `value_to_string` reads its one slot — with
    /// `rule:core-classes/html-auto-escape` adding that it has no constructor either, a member taking
    /// a runtime string being the bypass that section closes
    /// ([`crate::html`]). `Core\Db\InList`'s one slot is read by the
    /// bind: `rule:core-classes/db-parameters`'s expansion marker is accepted at exactly one
    /// position and nowhere else, so a member answering the values back would
    /// be a surface on a thing whose whole content is where it may appear
    /// ([`crate::db`]). `Core\Request\BodyStream`'s one slot is written and
    /// read by the `advance()`/`current()` pair on [`crate::instance`]'s dispatch roster
    /// — `Core\IO\Lines`'s entry for `Core\IO\Lines`'s reason,
    /// spec § 15 writing `bodyStream(): Iterable<bytes>` and no member on the
    /// thing it answers with. `Core\Db\Stream` is that entry again for § 18's
    /// `stream()`, and its three slots are the connection's key, the block that
    /// opened it and the one row the walk is holding. `Core\Jwe\Key` is the
    /// same shape over a secret: its three slots are read by `Core\Jwe`'s two
    /// members and by nothing else, and a member handing the material back
    /// would be the accessor whose absence is the whole point of the class —
    /// `rule:security/jwe-compact-subset` writes four constructors and no
    /// member on what they answer ([`crate::jwe`]). `Core\Http\Part` is that
    /// shape over a request body: its slots are read where the body is framed
    /// and nowhere else, and a member answering the octets back would make a
    /// `file` part hold the file it exists in order not to hold
    /// ([`crate::http`]). `Core\Cache\SecretEntry` is `Core\Jwe\Key`'s shape
    /// again: a fill builds one, the sealing path reads its two slots, and a
    /// member answering the secret back would be a second door onto a value
    /// `getSecret` is the only door onto ([`crate::cache`]). Classes have left
    /// this list, each the same way: `Core\Db\Connection` when `query` landed
    /// on it, `Core\Db\Rows` when its readers did, and `Core\Cli\Text` when
    /// `text()` did — its runs are still rendered by `value_to_string`, but a
    /// class the spec writes a member on is no longer a handle.
    ///
    /// The other exception is the mirror image: a class with members and **no
    /// slots**, because its receiver's whole state is the *context*'s rather
    /// than the object's. `Core\Socket` is the first — ADR 0083 § 1 gives a
    /// connection isolate one peer and § 3 makes `current()` a handle onto it,
    /// so the socket and the topic queue are `nvs_runtime::Ctx` fields that
    /// outlive every value a program makes from them. `Core\Response\Stream` is
    /// the second, over the same crate's writing half of a streaming response
    /// body: `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
    /// gives a request one body, so the handle says a stream was opened and the
    /// context owns it ([`crate::response`]). A slot holding a copy of any of
    /// that would be a second owner of a descriptor, which is the one thing
    /// [`crate::instance`]'s first decision refuses. This list may only grow for
    /// that same reason — a class here has to be able to say which context field
    /// is its state.
    #[test]
    fn a_class_with_slots_has_instance_members_and_the_reverse() {
        /// A class whose state is its context's, so it has members and no
        /// slots — the doc above owns what qualifies one.
        const CONTEXTUAL: &[&str] = &[
            crate::socket::NAME,
            crate::sse::NAME,
            crate::response::STREAM_NAME,
        ];

        const HANDLES: &[&str] = &[
            r"Core\Regex\Pattern",
            nvs_runtime::CARRIER_HTML_MARKUP,
            crate::script::HANDLE_NAME,
            crate::io::LINES_NAME,
            crate::io::WALK_NAME,
            crate::csv::ROWS_NAME,
            crate::http::TARGET_NAME,
            crate::http::IDENTITY_NAME,
            crate::http::PART_NAME,
            crate::http::stream::EVENTS_NAME,
            crate::http::stream::LINES_NAME,
            crate::http::stream::CHUNKS_NAME,
            crate::cli::COLOR_NAME,
            crate::cli::STYLE_NAME,
            crate::db::IN_LIST_NAME,
            crate::db::STREAM_NAME,
            crate::queue::ID_NAME,
            crate::request::BODY_STREAM_NAME,
            crate::request::FILES_NAME,
            crate::request::PART_CONTENT_NAME,
            crate::jwe::KEY_NAME,
            crate::jwt::KEY_SET_NAME,
            crate::cache::SECRET_ENTRY_NAME,
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
            if CONTEXTUAL.contains(&class.name) {
                assert!(
                    class.slots.is_empty() && !class.instance.is_empty(),
                    "{} is listed as contextual but is not one",
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

    /// Every page under `docs/reference/core/` names a class the registry
    /// holds. A page for a class the registry does not know is one nobody can
    /// reach from a name, and it is reported here rather than deleted.
    #[test]
    fn every_intro_page_names_a_registry_class() {
        let orphans: Vec<&str> = intros::INTROS
            .iter()
            .filter(|(name, _)| class(name).is_none())
            .map(|(name, _)| *name)
            .collect();
        assert!(
            orphans.is_empty(),
            "pages naming no registry class: {orphans:?}"
        );
        assert!(
            !intros::INTROS.is_empty(),
            "the reference tree has pages, and none was compiled in"
        );
    }

    /// The page's front matter is cut and its body is kept whole.
    #[test]
    fn an_intro_starts_below_its_front_matter() {
        let intro = class(r"Core\Str")
            .and_then(CoreClass::intro)
            .expect("`Core\\Str` has a page");
        assert!(!intro.starts_with("---"));
        assert!(!intro.contains("keywords:"));
        assert!(intro.contains("`Core\\Str`"));
    }
}
