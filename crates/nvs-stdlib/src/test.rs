//! `Core\Test` — `rule:testing/assertions-are-typed`'s assertion surface, and the other half of the `QName` `#[Test]` already
//! names.
//!
//! The attribute and this class are deliberately **one name**: `#[Test]` marks
//! the method and `Test::assertEquals` is what its body calls, so a single
//! `use Core\Test;` places both. `nvs_types::derive::ATTRIBUTES` owns the
//! attribute half; this module is the class one, and until it existed the
//! assertion in § 1's own worked example resolved against nothing at all.
//!
//! # Subject first, and generic
//!
//! Every member takes `$actual` then `$expected`, which is the **opposite** of
//! PHPUnit's order and falls out of
//! `rule:core-api/shape-rules` rather than
//! out of a preference. Because reversing the two is the single commonest
//! mistake in the ecosystem this language is migrated from, every failure
//! message below labels the sides `$actual` and `$expected` by **name**, so a
//! reversed call still reads correctly.
//!
//! Both of an equality member's parameters are one [`CoreTy::Var`], which is
//! what makes § 4's
//! "a type mismatch is a compile error" true with no rule of its own:
//! `nvs_types::generics` binds `T` from the first argument and substitutes it
//! through the signature, so the second is checked against the first's type by
//! the ordinary assignability check every `Core` call already goes through.
//!
//! # The three rows, and where they differ
//!
//! § 4's table is one comparison with two substitutions at the object row:
//!
//! * `assertSame` is [`nvs_runtime::identity`] — `rule:expressions/equality-semantics`'s table exactly,
//!   so two objects are the same object and nothing else is.
//! * `assertEquals` is that comparison with the object row replaced by
//!   `rule:classes/comparable`'s
//!   `compareTo`.
//! * `assertEqualsDeep` replaces it with the structural walk below.
//!
//! Writing them as one comparison rather than three is what keeps the scalar,
//! `string` and `array<T>` rows from drifting: those are `rule:expressions/one-equality-operator`'s, not this
//! ADR's, and a second reading of them here would be a second set of
//! PHP-divergence decisions nothing keeps in step.
//!
//! **The object row's refusal is made when the comparison runs, not where it is
//! written.** Two objects under `assertEquals` whose `$actual` class declares no
//! `compareTo` are a catchable throw naming `assertEqualsDeep`, and that is the
//! bound this module holds to rather than the compile error
//! `rule:testing/assertions-are-typed` could also be read as promising:
//! refusing the program at its call site wants the argument's class graph,
//! which `nvs_types` holds and this crate does not, and the mistake surfaces
//! the moment the test runs — which is the next thing that happens to a
//! `#[Test]` method. The type mismatch beside it *is* a compile error, made by
//! the `T` above.
//!
//! # The three predicate rows, and the two types they had to decide
//!
//! § 4's example writes `assertTrue`, `assertNull` and `assertCount` beside the
//! table without giving them a signature, so each declares its subject here and
//! the choice is this module's:
//!
//! * `assertTrue` takes a **`bool`**, not a `mixed` resolved through `rule:expressions/truthy-positions`'s
//!   truthy table. That ADR makes a *condition* the one place a value is tested
//!   without `as`, and an argument is not one.
//! * `assertNull` takes a **`mixed`**, `rule:types/conversion`'s one position that admits
//!   every type — a `?T` would refuse the non-`null` half of the union the
//!   question is about.
//! * `assertCount` takes an **`array<T>`** and a `uint`, which is
//!   `Core\Arr::count`'s own signature. A length is answered per domain in this
//!   library, so a union subject would be a fourth answer to "how long is it",
//!   decided by a tag rather than by the member the author named.
//!
//! # The two rows whose subject is a `callable`
//!
//! `assertThrows` (§ 4) and `assertDoesNotThrow` (§ 20) are the one shape on
//! that roster that is not an assertion *about* its first argument: each runs
//! a `callable` and judges what came back. Both are written over the same
//! three edges — the body returned, the body threw, or the body ended the
//! request — and each consumes the throw it judged, so the assertion's own
//! `Core\Test\Failure` is what propagates rather than the exception it is
//! reporting. `assertDoesNotThrow` is also § 20's way out of the empty-ledger
//! rule, which is why `nvs_cli::runner` names it there.
//!
//! Each member's own doc comment carries the argument in full.
//!
//! # The one row whose expectation is a source literal
//!
//! § 14's `assertMatchesInline` compares `Core\Debug::render`'s text against a
//! literal in the test body rather than against a `.snap` file beside it. Its
//! own doc comment owns why the renderer is borrowed rather than grown, and
//! which two other crates `nvs test --update` needs to write into that literal
//! — this one contributes the pair of texts and holds no span for either.
//!
//! # The ledger, and the one member that discharges from it
//!
//! § 5 gives an assertion two effects, not one: it throws `Core\Test\Failure`
//! *and* it records its outcome into a per-test ledger the test's own code
//! cannot reach — which is what abolishes the silently-passing test, since the
//! runner reads the ledger rather than the exception state. The ledger is
//! `nvs_runtime::Ctx`'s (its field's own docs own why it lives there), the two
//! writers are [`held`] and [`failed`], and every member here goes through one
//! of them on every edge. `Core\Test::expectFailure(callable)` is the only way
//! an entry ever leaves it.
//!
//! # What these members do with a qualifier
//!
//! `rule:security/unclassified-parameter-refuses-tainted`'s classification, and this class is the flat case: **every
//! member answers `void`, so every one of them is [`Qual::Neutral`] in every
//! parameter** — the `Qual` enum's own first bullet, with a return type that
//! carries even less than the `bool` that bullet is written about. Two of them
//! are worth saying out loud, because both look like they might be more.
//!
//! * **[`MESSAGE`]'s `message` is not a sink.** It really does reach a
//!   terminal — [`failed`] renders it into the line the runner prints — and a
//!   reader who knows `rule:tooling/terminal-output-is-a-sink`
//!   may expect the refusal there. The refusal is the *terminal's*, made once
//!   where the bytes are written and where control bytes are substituted
//!   visibly, not made a second time at every member whose text might one day
//!   arrive. A mark here would be a claim about a sink this module does not
//!   own.
//! * **`assertThrows`'s `$expected` is not [`Qual::Sink`] either.** It is a
//!   class name matched by [`nvs_runtime::Ctx::pending_conforms_to`], and
//!   [`Qual::Sink`] is `rule:security/sink-predicate`'s predicate — content that becomes an
//!   instruction something executes, which on disk is `rule:core-api/shape-rules` R11's four
//!   grammars plus `Core\IO`'s paths, where `..` and the separators direct the
//!   resolver. A name matched
//!   against a roster compiles nothing and executes nothing. The spelling a
//!   call uses is `Core\Test\Failure::class`, which folds to a constant, so a
//!   qualified argument does not arise in practice either.
//!
//! # § 18's in-process request
//!
//! `request` runs a synthetic request through the program under test with no
//! socket and no port, and [`RESPONSE`]'s two accessors are what it answers
//! with. The mechanism is [`nvs_runtime::inproc`]'s and that module's doc is
//! the one home of it. What the request carries is the block above's gap 3.
//!
//! **`#[Test(server: true)]` is a separate mechanism, and this class holds one
//! word of it.** § 18 justifies the two as answering measurably different
//! questions; the listener itself is the runner's — `nvs_cli::runner`'s
//! `TestServer` binds it, serves the program under test on it and retires it
//! with the test — and what is here is [`serverUrl`](CLASS), the address a test
//! reads it back at. That direction is why the member answers `null` rather
//! than throwing: it is an ordinary optional reading in every context, so it
//! can be asked from anywhere, and the positive half is asserted where a
//! listener can exist at all (`nvs-cli`'s own
//! `a_test_with_server_true_gets_an_ephemeral_listener`) rather than by a
//! `.nvst` case, no case ever being inside a `#[Test]`.
//!
//! A test reaching its own listener needs `rule:http-server/allow-url-pins-the-address`'s outbound pair granted —
//! `net.connect` for the host and `net.internal` for § 3's denied loopback
//! range — because `Core\Http\Client` is the way a program speaks HTTP and
//! nothing about a listener being the test's own widens that policy. That is a
//! real cost of the mechanism rather than an oversight; `Core\Test` handing
//! back an already-pinned `Core\Http\Target` would remove it, and § 18 does
//! not decide between the two.
//!
//! # Decision: a double's descriptor is built here, at the call that asks for one
//!
//! `Core\Test::double<Clock>({now: fn (): Instant => …})` answers a value that
//! **is** a `Clock` (`rule:testing/doubles`), which means a
//! [`nvs_runtime::ClassDesc`] naming `Clock` among the classes an instance also
//! is and carrying a method row per interface method. Two places could build
//! it: `nvs-ir`, synthesizing a class per call site the way its `lower_new`
//! already rewrites `new` on a `Core` class, or this crate, at the moment the
//! helper runs.
//!
//! **It is built here, and `nvs-ir` learns nothing about doubles at all.** Read
//! properly, that precedent points this way: `lower_new` lowers to a *call to
//! this crate's helper*, and the one thing the call site owes such a helper is
//! the class it wrote, which [`crate::registry::WRITTEN_CLASS_MEMBERS`] already
//! delivers as argument 0 — the interface's own descriptor, ahead of everything
//! including a receiver. So `double` is an ordinary `InstKind::CoreCall` on a
//! roster the front end already reads. A synthesized class would instead put a
//! class *the author never wrote* into the program's own class list, where every
//! consumer that walks one meets it, and `nvs-ir` would be assembling a method
//! table out of addresses only this crate holds.
//!
//! **The helper has everything the rows need.** The interface's descriptor is
//! what the double conforms to; the shape argument is an object whose own
//! descriptor is a `$shape{…}` class ([`nvs_runtime::ClassDesc::is_shape`])
//! whose field names are the method names and whose slots hold the closures;
//! and for `partial`, `$real`'s descriptor names the methods the shape does not
//! override. Each closure carries its declared arity and parameter tags in its
//! own slots, which `nvs_runtime`'s closure module owns, so every
//! [`nvs_runtime::MethodRow`] a
//! double publishes is **the closure's** declared shape rather than a guess:
//! a call that arrives through an erased view is checked against the body that
//! will answer it, by the one implementation `check_param_tags` already is.
//!
//! **The closure is also the only source for it.** An interface's descriptor
//! carries a [`nvs_runtime::MethodRow`] for a *default* body and for nothing
//! else — `nvs_types::layout::ClassLayout::methods` keeps only bodied methods,
//! because a bodiless declaration names no code — so the interface cannot be
//! asked what it declares, and the shape's fields are the whole list. A
//! `partial` therefore reads its **delegated** half off `$real`'s method table
//! instead, that class being where the shape of a call it does not answer
//! itself is written down; a row is published for every method behind it but
//! its constructor, which is why the ceiling can refuse a partial whose real
//! class is fat while the interface it stands in for is small.
//!
//! **The rows are not `native`.** That flag is an ownership claim and not a
//! "written in Rust" one — the two readers of it in `nvs_runtime::dispatch`
//! refuse such a row outright — and a double's row is reached by
//! `nvs_ir::ir::InstKind::CallVirtual`, which *transfers* its arguments. So a
//! trampoline releases what it was handed, exactly as [`crate::cursor`]'s
//! roster symbols release slot 0, and the row reads to every caller as the
//! compiled method it stands in for.
//!
//! **One descriptor per `(interface, real class, overridden names)`**, in a
//! table this module leaks on [`crate::instance`]'s terms and for its reasons.
//! That triple is decided by what a call site *wrote*, so the table is
//! O(call sites) and not O(doubles made), and two doubles from one call site
//! are one class — which is what a `get_class` comparison and
//! `Core\Debug::render` read. [`nvs_runtime::ClassTable::define`] takes its
//! parents as ids of its own table and an interface's is not one, so
//! [`nvs_runtime::ClassTable::define_conforming`] beside it names a parent by
//! address instead; the bound it depends on
//! is that the interface descriptor belongs to the compiled unit under test,
//! and `rule:testing/tests-never-reach-a-build` deletes `Core\Test` from every
//! build, so the unit in question is the one `nvs test` compiled and the double
//! cannot outlive it.
//!
//! **What it spends:** one leaked descriptor per distinct call site, once per
//! process; one object per double, `FIELDS_OFFSET` bytes plus 16 per method and
//! two more slots, charged to the test's request and released with it; one
//! array append per recorded call. Nothing on a served request.
//!
//! # Decision: a trampoline carries its slot in its own identity
//!
//! [`nvs_runtime::MethodRow::code`] is a bare address with no data word beside it, so a
//! trampoline cannot be *handed* which method it is standing in for — it has to
//! **be** it. This module publishes a fixed table of native functions, one
//! generated per slot, and row *i* of a double's method table is entry *i* of
//! that table. Nothing is stored per double to recover the slot, and no lookup
//! runs to find it.
//!
//! The double's fields are its state: slot 0 the call record, slot 1 `$real`
//! (or `null`), and slot `2 + i` the closure answering method *i*, the methods
//! in the order [`nvs_runtime::ClassTable::set_methods`] sorts their rows.
//! Every field name carries a `$`, which no property name can, so an erased
//! `$double->now` (`rule:types/erased-member-access`) finds nothing and the
//! representation stays unspellable; the trampoline reads its own method name
//! back out of [`nvs_runtime::ClassDesc::field_name`] and drops the sigil,
//! which is both what the recorded call is keyed by and how the row carrying
//! its arity is found.
//!
//! An interface with more methods than that table is long is refused by
//! `double` as a `LogicError` naming both counts. A generated table has to have
//! a length, and this one is set well above the interfaces a test doubles;
//! the whole of what it costs is that many native functions in the binary, once
//! per process and nothing per double.
//!
//! Three alternatives were rejected. A **data word on [`nvs_runtime::MethodRow`]** spends
//! eight bytes per method per class in *every* program, and widens the one
//! struct every virtual call reads, to serve a member no build lowers.
//! **Emitting a thunk per slot at the call site** is real code emission, in a
//! backend that would be learning about doubles to do it, and the double is
//! built by a native helper that holds no emitter. **One shared trampoline
//! recovering its slot by finding its own address in the receiver's method
//! table** cannot work at all: every row holds that same address, so the scan
//! has nothing to tell them apart with.
//!
//! A method the shape leaves unimplemented publishes no row, and a call
//! reaching one falls through to the fallback a bodiless declaration names —
//! `nvs_runtime::nvs_abstract_method`'s reported `FATAL` — rather than to
//! anything unchecked. **No checked program reaches that fallback**:
//! `nvs_types::conformance`'s walk refuses a shape leaving a method of the
//! interface unanswered (`E0825`) and one naming a method the interface does
//! not declare (`E0826`), where the call is written. The floor stays because
//! this helper's own argument list is `Value`s, and what it must never do with
//! one it cannot explain is dispatch.
//!
//! **The record is read by name, and the name is written as `Mailer::send`.**
//! `rule:testing/interaction-after-the-fact`'s reference is spelled bare —
//! the call site writes it where a class constant would go — and the design
//! call this module makes is that it is *admitted by the row*: a parameter
//! written [`crate::registry::CoreTy::MethodRef`] is the whole set of positions
//! the spelling means anything at, `nvs_types::core_lib::method_ref_param`
//! reads that set back off the row, and everywhere else `Class::name` stays the
//! undefined constant it reads as. The checker folds the reference to the
//! method's own name there and then, so what a helper below is handed is a
//! `Tag::Str` and nothing about the ABI knows a reference exists. The
//! first-class-callable spelling the goal held in reserve is not used: a
//! `Mailer::send(...)` names an *instance* method with no receiver, which is a
//! frame `nvs-ir` cannot lower, and a closure carries no name for the record to
//! be keyed by.

use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Value, identity};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreField, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc,
    ParamDoc, Qual, ShapeKeyDoc,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Test`'s fully-qualified name, written once so the registry row, the
/// failure messages and `nvs_types::derive`'s attribute roster cannot drift
/// apart.
pub(crate) const NAME: &str = r"Core\Test";

/// The one type variable every assertion's two subjects share — see this
/// module's docs for what it buys.
const T: CoreTy = CoreTy::Var("T");

/// `{message?: string}` — `rule:core-api/shape-rules` R2's trailing bag, and § 4's own
/// `{message: "a fresh user is active"}`.
///
/// [`Const::Null`] rather than an empty `string` because "not given" and
/// `{message: ""}` are one thing to a reader and the helper takes the same path
/// for both; the option's declared type stays `string`, which is what a call
/// site may write.
const MESSAGE: &[CoreOption] = &[CoreOption {
    name: "message",
    ty: CoreTy::Text(Qual::Neutral),
    default: Const::Null,
}];

/// `{times?: uint, with?: array<mixed>, message?: string}` —
/// `Core\Test::assertCalled`'s bag, and the only one on this class wider than
/// [`MESSAGE`].
///
/// Both of the new options narrow what counts as the call being looked for, and
/// each is [`Const::Null`] for [`MESSAGE`]'s reason: a call written with
/// neither asserts on the method's name alone, which is what
/// `rule:testing/interaction-after-the-fact` gives as the default reading. They
/// are options rather than parameters because a test that only asks *whether*
/// is the common case and has nothing to write in either slot.
///
/// `message` is here so that this member reports a failure the way every other
/// assertion does (`rule:testing/assertions-are-typed`); the stage's prose
/// names the two that are new, not the one the whole class carries.
const CALLED: &[CoreOption] = &[
    CoreOption {
        name: "times",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "with",
        ty: CoreTy::Array(&CoreTy::Mixed),
        default: Const::Null,
    },
    CoreOption {
        name: "message",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
];

/// `{within: Duration}` — `Core\Test::assertCompletes`'s budget, a
/// `rule:core-api/shape-parameter` shape rather than a [`MESSAGE`]-style bag.
///
/// A bag's option is optional by construction and there is no duration this
/// member could pick on a call site's behalf, so the one thing it takes has to
/// be a field with no default — which is a shape, and which keeps ADR 0079
/// § 16's own spelling, `{within: Duration::millis(50)}`, rather than trading
/// it for a bare positional `Duration`.
const WITHIN: &[&[CoreField]] = &[&[CoreField {
    name: "within",
    ty: CoreTy::Instance(crate::time::DURATION_NAME),
    default: None,
}]];

/// `rule:core-api/shape-flattens-at-the-abi`'s flattening of [`WITHIN`] at
/// `assertCompletes`, as ABI slots: the body, the shape's one field, then the
/// trailing [`MESSAGE`] bag's one option. A shape has no runtime
/// representation, so these three are what the helper is handed and this is the
/// only place the numbers are written.
const BODY_ARG: usize = 0;
/// See [`BODY_ARG`].
const WITHIN_ARG: usize = 1;
/// See [`BODY_ARG`].
const COMPLETES_MESSAGE_ARG: usize = 2;

/// `{json?, body?, headers?}` — what a registered answer comes back with, as
/// `rule:testing/an-outbound-call-is-answered-from-a-table` writes the bag.
///
/// **`json` and `body` are two spellings of one slot**, not two things a reply
/// can hold: `json` is the value written to JSON where it is registered, so a
/// test naming an object never writes `Core\Json::encode` by hand and never
/// quotes a document into a string literal, and `body` is the bytes for
/// everything that is not a JSON document. An answer naming both is a
/// `LogicError` rather than a precedence rule nobody would remember.
///
/// The status is a parameter and not an option because every answer has one —
/// `rule:core-api/shape-rules` R2's bag is for what a call may leave out, and a
/// reply with no status is not a reply.
/// What a registered answer's `body` may be: text, or the octets of a reply
/// that is not text at all.
const ANSWER_BODY: &[CoreTy] = &[CoreTy::Text(Qual::Neutral), CoreTy::Blob(Qual::Neutral)];

/// What one entry of a registered answer's `headers` may be: the field's one
/// line, or every line a reply carried under that name.
const ANSWER_HEADER: &[CoreTy] = &[
    CoreTy::Text(Qual::Neutral),
    CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
];

/// What one frame of a scripted peer may be: RFC 6455's two payload kinds, as
/// the element's own type rather than a mode string beside it
/// (`rule:core-api/no-mode-strings`). The pair `Core\Http\Socket` splits into
/// two members for the qualifier's sake is one union here, because a script is
/// a value a test writes rather than a parameter a `tainted` argument could
/// reach.
const PEER_FRAME: CoreTy =
    CoreTy::Union(&[CoreTy::Text(Qual::Neutral), CoreTy::Blob(Qual::Neutral)]);

/// A scripted peer's one option: which subprotocol its `101` chose.
///
/// A bag rather than a third parameter, for [`ANSWER`]'s reason — what a peer
/// may leave out belongs after what every peer has, and a handshake that chose
/// no subprotocol is the ordinary case.
const PEER: &[CoreOption] = &[CoreOption {
    name: "protocol",
    ty: CoreTy::Text(Qual::Neutral),
    default: Const::Null,
}];

const ANSWER: &[CoreOption] = &[
    // `Const::NeverWritten` and not a `Null`, which is what a `mixed` option
    // owes `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`:
    // `null` is a JSON document a test may mean, so the two have to arrive
    // under different tags.
    CoreOption {
        name: "json",
        ty: CoreTy::Mixed,
        default: Const::NeverWritten,
    },
    // `string|bytes` because a reply is where the two differ: an image or an
    // archive is a body a test has to be able to register, and
    // `Core\Http\Response::bytes` is the member that reads one back.
    CoreOption {
        name: "body",
        ty: CoreTy::Union(ANSWER_BODY),
        default: Const::Null,
    },
    // The `array<string>` shape `Core\Http\Options` writes its request headers
    // as, one door over, so a test registering a reply and a program making a
    // call spell a header map the same way — widened by one arm, because a
    // reply is the direction where a field arrives twice and a map keyed by
    // name could hold only the second of them. A list under one key is those
    // lines, and `Core\Http\Response::headers` is what reads them back.
    CoreOption {
        name: "headers",
        ty: CoreTy::Array(&CoreTy::Union(ANSWER_HEADER)),
        default: Const::EmptyArray,
    },
    // A `Core\Http\TlsInfo` and not a bag of its fields, because an option is
    // never itself a bag: `Core\Test::tlsSession` is the member that takes the
    // fields and builds one (ADR 0217).
    CoreOption {
        name: "tls",
        ty: CoreTy::Instance(crate::http::TLS_INFO_NAME),
        default: Const::Null,
    },
];

/// `Core\Test::tlsSession`'s options, one per `Core\Http\TlsInfo` member that
/// a test may choose, in that class's member order. Every one has a default,
/// so `tlsSession()` alone is a verified TLS 1.3 session.
const TLS_SESSION: &[CoreOption] = &[
    CoreOption {
        name: "version",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "cipher",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "verified",
        ty: CoreTy::Bool,
        default: Const::Bool(true),
    },
    CoreOption {
        name: "subject",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "issuer",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "expiry",
        ty: CoreTy::Instance(crate::time::INSTANT_NAME),
        default: Const::Null,
    },
];

/// The subject a described session's leaf carries when the test names none.
const DEFAULT_TLS_SUBJECT: &str = "CN=example.com";
/// The issuer a described session's CA carries when the test names none.
const DEFAULT_TLS_ISSUER: &str = "CN=Novis Test CA";
/// How long after the test's clock a described session's leaf expires when
/// the test names no expiry: ninety days, the lifetime most public
/// certificates are issued for.
const DEFAULT_TLS_LIFETIME: std::time::Duration = std::time::Duration::from_secs(90 * 86_400);

/// What a synthetic request's `body` may be: text, or the octets of a body that
/// is not text at all.
///
/// [`ANSWER_BODY`]'s pair, in the other direction and for the second half of
/// its reason: the bodies a request test is written for are the ones a
/// `string` cannot hold. A payload that is not UTF-8 is exactly what
/// `Core\Request::body()` refuses and `Core\Request::bytes()` answers
/// (`rule:types/string-is-utf8`), so a bag admitting text alone could not
/// describe the request that pins either of them.
const SENT_BODY: &[CoreTy] = &[CoreTy::Text(Qual::Neutral), CoreTy::Blob(Qual::Neutral)];

/// `Core\Test::request`'s `{headers?: array<string>, body?: string|bytes,
/// mount?: string}` — the halves of a request a verb and a path do not
/// describe.
///
/// A bag rather than two more parameters, on [`ANSWER`]'s reading of
/// `rule:core-api/shape-rules` R2: every request has a verb and a path, and
/// most of the ones a test writes carry neither a field line nor a body.
///
/// **`headers` is `Core\Http\Options`' shape**, read by that class's own walk
/// ([`crate::http::headers_of`]), so a test describing a request and a program
/// making one spell a header map the same way and the names arrive as they were
/// written.
///
/// **`body`'s default is [`Const::Null`] and not an empty `bytes`**: a request
/// that carries no body and one that carries an empty body are different facts
/// (RFC 9110 § 8.6) that reach a program differently, so "not given" cannot be
/// spelled by a value the option's own type admits.
///
/// **`mount` defaults to `""` and not to [`Const::Null`]**, for the opposite
/// reason: a request that reached no mount and one whose door stripped nothing
/// are the same fact, which is what `rule:routing/a-request-reads-its-mount`
/// makes `Core\Request::mount()` never-`null` for. The key is the prefix alone
/// and not the mount's glob captures — a synthetic request describes the door
/// a link is written under, and a capture is a second fact with a reader of its
/// own.
const REQUEST_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "headers",
        ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
        default: Const::EmptyArray,
    },
    CoreOption {
        name: "body",
        ty: CoreTy::Union(SENT_BODY),
        default: Const::Null,
    },
    CoreOption {
        name: "mount",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Str(""),
    },
];

/// `Core\Test`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "The methods you call inside a `#[Test]` method. They check a result, move the fixed \
            clock and give fake replies to the web requests your code sends. `nvs test` runs the \
            test and reports each check that fails.",
};

/// `Core\Test`'s registry rows — § 4's three equality members, the three
/// predicate ones its example writes beside them, and § 5's `expectFailure`.
/// See
/// [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "assertSame",
            names: &["actual", "expected"],
            params: &[T, T, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_same",
            doc: Some(&ASSERT_SAME_DOC),
        },
        CoreMethod {
            name: "assertEquals",
            names: &["actual", "expected"],
            params: &[T, T, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_equals",
            doc: Some(&ASSERT_EQUALS_DOC),
        },
        CoreMethod {
            name: "assertEqualsDeep",
            names: &["actual", "expected"],
            params: &[T, T, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_equals_deep",
            doc: Some(&ASSERT_EQUALS_DEEP_DOC),
        },
        CoreMethod {
            name: "assertTrue",
            names: &["actual"],
            params: &[CoreTy::Bool, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_true",
            doc: Some(&ASSERT_TRUE_DOC),
        },
        CoreMethod {
            name: "assertNull",
            names: &["actual"],
            params: &[CoreTy::Mixed, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_null",
            doc: Some(&ASSERT_NULL_DOC),
        },
        CoreMethod {
            name: "assertCount",
            names: &["actual", "expected"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Options(MESSAGE),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_count",
            doc: Some(&ASSERT_COUNT_DOC),
        },
        CoreMethod {
            name: "assertContains",
            names: &["actual", "expected"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                T,
                CoreTy::Options(MESSAGE),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_contains",
            doc: Some(&ASSERT_CONTAINS_DOC),
        },
        CoreMethod {
            name: "assertMatchesInline",
            names: &["actual", "expected"],
            params: &[
                CoreTy::Mixed,
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(MESSAGE),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_matches_inline",
            doc: Some(&ASSERT_MATCHES_INLINE_DOC),
        },
        CoreMethod {
            name: "assertThrows",
            names: &["body", "expected"],
            params: &[
                CoreTy::CallableSig(&[], &CoreTy::Mixed),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(MESSAGE),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_throws",
            doc: Some(&ASSERT_THROWS_DOC),
        },
        CoreMethod {
            name: "assertDoesNotThrow",
            names: &["body"],
            params: &[
                CoreTy::CallableSig(&[], &CoreTy::Mixed),
                CoreTy::Options(MESSAGE),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_does_not_throw",
            doc: Some(&ASSERT_DOES_NOT_THROW_DOC),
        },
        CoreMethod {
            name: "expectFailure",
            names: &["body"],
            params: &[CoreTy::CallableSig(&[], &CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_expect_failure",
            doc: Some(&EXPECT_FAILURE_DOC),
        },
        CoreMethod {
            name: "advance",
            names: &["by"],
            params: &[CoreTy::Instance(crate::time::DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_advance",
            doc: Some(&ADVANCE_DOC),
        },
        CoreMethod {
            name: "serverUrl",
            names: &[],
            params: &[],
            defaults: &[],
            // `Qual::Neutral` and nullable: the text is the runner's own — the
            // scheme it chose and the port the operating system handed it — so
            // nothing a peer wrote is in it, and a context no listener was
            // armed for has no address rather than an empty one.
            return_ty: CoreTy::Nullable(&CoreTy::Text(Qual::Neutral)),
            symbol: "nvs_core_test_server_url",
            doc: Some(&SERVER_URL_DOC),
        },
        CoreMethod {
            name: "scriptAnswers",
            names: &["answers"],
            params: &[CoreTy::Array(&CoreTy::Text(Qual::Neutral))],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_script_answers",
            doc: Some(&SCRIPT_ANSWERS_DOC),
        },
        CoreMethod {
            name: "request",
            names: &["method", "path"],
            // The path is `Qual::Neutral` and not a sink: what it selects is a
            // row of a table compiled from the program's own `#[Route]`
            // declarations, so nothing it says becomes an instruction, and the
            // answer — a status and the bytes the program wrote — carries none
            // of the argument's qualifier back out.
            params: &[
                CoreTy::Enum(crate::router::METHOD_NAME),
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(REQUEST_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(RESPONSE_NAME),
            symbol: "nvs_core_test_request",
            doc: Some(&REQUEST_DOC),
        },
        CoreMethod {
            name: "answerHttp",
            names: &["url", "status"],
            // `Qual::Sink`, which is the mark that refuses a `tainted` argument
            // and says why: this text decides which of the program's outbound
            // calls are answered, so a URL that came from outside would be
            // outside input choosing which calls a test lets through.
            // `Core\Http::allowUrl` is still the one door through
            // `rule:security/outbound-url-is-a-sink` — nothing here connects to
            // the URL it is given, and a call that matches a row is answered
            // without resolving anything.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Uint,
                CoreTy::Options(ANSWER),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_answer_http",
            doc: Some(&ANSWER_HTTP_DOC),
        },
        CoreMethod {
            name: "tlsSession",
            names: &[],
            params: &[CoreTy::Options(TLS_SESSION)],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::http::TLS_INFO_NAME),
            symbol: "nvs_core_test_tls_session",
            doc: Some(&TLS_SESSION_DOC),
        },
        CoreMethod {
            name: "sentHttp",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(SENT_REQUEST_NAME)),
            symbol: "nvs_core_test_sent_http",
            doc: Some(&SENT_HTTP_DOC),
        },
        CoreMethod {
            name: "answerSocket",
            names: &["url", "frames"],
            // `Qual::Sink` for the URL, as `answerHttp`'s is and for its
            // reason. The frames are a union and therefore carry no
            // classification at all, which refuses a `tainted` element: what a
            // scripted peer says is the test's own text, and a script written
            // out of outside input would be outside input deciding what the
            // subject reads.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&PEER_FRAME),
                CoreTy::Options(PEER),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_answer_socket",
            doc: Some(&ANSWER_SOCKET_DOC),
        },
        CoreMethod {
            name: "sentSocket",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(crate::socket::MESSAGE_NAME)),
            symbol: "nvs_core_test_sent_socket",
            doc: Some(&SENT_SOCKET_DOC),
        },
        CoreMethod {
            name: "double",
            names: &["answers"],
            // `rule:types/object-top`'s erased form, which every shape type is a
            // subtype of, because the constraint this parameter really carries
            // cannot be spelled as a type at all: what the shape's fields have
            // to match is the *interface the call site wrote*, which no
            // registry row can name. The checker's own refusals are what
            // enforce it (`rule:testing/doubles`), and they read the type
            // argument and the shape together where the call is written.
            params: &[CoreTy::Object],
            defaults: &[],
            return_ty: CoreTy::Written("T"),
            symbol: "nvs_core_test_double",
            doc: Some(&DOUBLE_DOC),
        },
        CoreMethod {
            name: "partial",
            names: &["real", "answers"],
            // `$real` is the written type itself, so a partial of a `Clock` may
            // only delegate to something that already is one — the whole of
            // what makes the un-overridden half safe to forward.
            params: &[CoreTy::Written("T"), CoreTy::Object],
            defaults: &[],
            return_ty: CoreTy::Written("T"),
            symbol: "nvs_core_test_partial",
            doc: Some(&PARTIAL_DOC),
        },
        CoreMethod {
            name: "assertCalled",
            names: &["double", "method"],
            // `rule:types/object-top`'s erased form again, and for
            // [`nvs_core_test_double`]'s reason one step on: what this position
            // really takes is a *double*, which is a class no program can name
            // and no row can therefore write. The helper's own refusal is what
            // enforces it, reading the call record every double carries.
            params: &[CoreTy::Object, CoreTy::MethodRef, CoreTy::Options(CALLED)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_called",
            doc: Some(&ASSERT_CALLED_DOC),
        },
        CoreMethod {
            name: "assertNeverCalled",
            names: &["double", "method"],
            params: &[CoreTy::Object, CoreTy::MethodRef, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_never_called",
            doc: Some(&ASSERT_NEVER_CALLED_DOC),
        },
        CoreMethod {
            name: "assertCompletes",
            names: &["body", "settings"],
            // The body is `CoreTy::CallableSig` rather than a bare `Callable`
            // for [`registry`]'s reason: a `Core` callback states the signature
            // it is called with, and this one is called with nothing and its
            // answer is dropped. `assertThrows` above declares the same pair.
            params: &[
                CoreTy::CallableSig(&[], &CoreTy::Mixed),
                CoreTy::Shape(WITHIN),
                CoreTy::Options(MESSAGE),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_completes",
            doc: Some(&ASSERT_COMPLETES_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Test\Response`'s fully-qualified name, written once for the same
/// reason [`NAME`] is.
pub(crate) const RESPONSE_NAME: &str = r"Core\Test\Response";

/// What one in-process request answered with — `rule:testing/in-process-request`.
///
/// A `Core`-owned instance rather than an `rule:types/object-top` shape, which is the one
/// place this surface departs from § 18's worked example's `$rs->status`. A
/// shape would be the smaller surface, and `Core\Script\Result` is the
/// precedent for spelling a result as one; what decides it the other way is
/// that there is no registry spelling for a *returned* shape at all —
/// [`CoreTy::Shape`] is a parameter's arms, flattened at the call site into one
/// ABI argument per field, and a shape value crosses back only from a
/// construct the checker types itself (`nvs_types::expr::isolate`). Inventing
/// one for a single member would put a second shape-typing path in
/// `nvs-types` beside the one that already exists, which is a larger change
/// than the two accessors below, and `Core\Script\ExitReport` is the shape a
/// `Core`-owned result already takes here.
///
pub(crate) const RESPONSE: CoreClass = CoreClass {
    name: RESPONSE_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "status",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_test_response_status",
            doc: Some(&RESPONSE_STATUS_DOC),
        },
        CoreMethod {
            name: "body",
            names: &[],
            params: &[],
            defaults: &[],
            // Not `tainted`: the bytes are what the *program under test* wrote,
            // which is its own output and not the peer's input. `rule:security/tainted-qualifier`'s
            // qualifier travels with what arrived, and nothing that arrived
            // reaches this without the program having put it there.
            return_ty: CoreTy::Str,
            symbol: "nvs_core_test_response_body",
            doc: Some(&RESPONSE_BODY_DOC),
        },
    ],
    slots: &["status", "body"],
    constants: &[],
};

/// [`RESPONSE`]'s first slot: the status the program declared, or `200`.
const STATUS_SLOT: usize = 0;
/// [`RESPONSE`]'s second slot: the bytes the program wrote.
const BODY_SLOT: usize = 1;

/// `Core\Test\SentRequest`'s fully-qualified name, written once for the same
/// reason [`NAME`] is.
pub(crate) const SENT_REQUEST_NAME: &str = r"Core\Test\SentRequest";

/// One outbound call the program made while the answer table was armed —
/// `rule:testing/an-outbound-call-is-answered-from-a-table`.
///
/// **Nothing on it is `tainted`, which is the asymmetry with every member of a
/// reply**: what these four read back is text the program under test authored
/// — the verb its own row named, the URL it composed, the headers it wrote —
/// and `rule:security/tainted-qualifier`'s qualifier answers where a value came
/// from. Marking a sent request `tainted` would say it arrived from outside,
/// which would make a test launder its own subject's output to assert on it.
///
/// A `Core`-owned instance for [`RESPONSE`]'s reason, and readonly for a second
/// one: a record of what was sent is a fact about a call that has already been
/// made, so there is nothing on it a program could sensibly write.
pub(crate) const SENT_REQUEST: CoreClass = CoreClass {
    name: SENT_REQUEST_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "method",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(crate::router::METHOD_NAME),
            symbol: "nvs_core_test_sent_method",
            doc: Some(&SENT_METHOD_DOC),
        },
        CoreMethod {
            name: "url",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_test_sent_url",
            doc: Some(&SENT_URL_DOC),
        },
        CoreMethod {
            name: "header",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_test_sent_header",
            doc: Some(&SENT_HEADER_DOC),
        },
        CoreMethod {
            name: "body",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_test_sent_body",
            doc: Some(&SENT_BODY_DOC),
        },
    ],
    slots: &["method", "url", "headers", "body"],
    constants: &[],
};

/// [`SENT_REQUEST`]'s first slot: the [`crate::router::METHOD`] ordinal of the
/// verb the call carried.
const SENT_METHOD_SLOT: usize = 0;
/// [`SENT_REQUEST`]'s second slot: the URL as the program wrote it.
const SENT_URL_SLOT: usize = 1;
/// [`SENT_REQUEST`]'s third slot: the headers, keyed by a lower-cased name.
///
/// A slot with no member of its own name, which [`SENT_REQUEST`]'s roster is
/// the reason for: a header map is read one name at a time, and handing the
/// whole array back would be a second reading of the same slot that a test
/// could iterate in an order the request never had.
const SENT_HEADERS_SLOT: usize = 2;
/// [`SENT_REQUEST`]'s fourth slot: the bytes the call carried.
const SENT_BODY_SLOT: usize = 3;

/// `Core\Test::advance`'s reference card — `rule:core-api/reference-card`.
const ADVANCE_DOC: MethodDoc = MethodDoc {
    short: "Moves the clock that `#[Test(at: ...)]` fixed forward by `$by`. A test of something \
            that expires can then check the time after it expires, without waiting.",
    params: &[ParamDoc {
        name: "by",
        desc: "How far to move the clock. A negative duration moves it back.",
        shape: &[],
    }],
    ret: "Nothing. The next `Core\\Time::now()` returns the moved time.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The running test has no `at:`, so there is no fixed clock to move. The real \
                   clock of the computer is never moved.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The moved time is outside the range a clock can show, about the years -9999 to \
                   9999. The clock does not move.",
        },
    ],
};

/// `Core\Test::serverUrl`'s reference card — `rule:core-api/reference-card`.
const SERVER_URL_DOC: MethodDoc = MethodDoc {
    short: "Returns the address of the test server that a `#[Test(server: true)]` test starts. \
            The server runs on this computer, on a free port. Use it when a test needs a real \
            network connection.",
    params: &[],
    ret: "An address such as `http://127.0.0.1:52341`, with no `/` at the end, so you can add a \
          path directly. The result is `null` everywhere else, which includes a test without \
          `server: true` and a program started with `nvs run`.",
    errors: &[],
};

/// `Core\Test::scriptAnswers`'s reference card — `rule:core-api/reference-card`.
const SCRIPT_ANSWERS_DOC: MethodDoc = MethodDoc {
    short: "Writes down what the next `Core\\Cli` prompts will be answered with, so an \
            interactive flow is assertable instead of untestable — each prompt takes the oldest \
            line still queued rather than reading a terminal.",
    params: &[ParamDoc {
        name: "answers",
        desc: "One line per prompt, in the order the subject asks them — what a person would \
               have typed, without its ending. A `select` reads the menu number, a `confirm` \
               reads `y` or `n`, and an empty line is an empty answer rather than a silence.",
        shape: &[],
    }],
    ret: "Nothing. The lines join the tail of the queue, so scripting a flow in two calls reads \
          in one order; what no prompt drained is discarded with the test.",
    errors: &[],
};

/// `Core\Test::request`'s reference card — `rule:core-api/reference-card`.
const REQUEST_DOC: MethodDoc = MethodDoc {
    short: "Sends one request to your own program and returns the answer. It opens no network \
            connection. The request goes through the same route table and the same code as a \
            real request.",
    params: &[
        ParamDoc {
            name: "method",
            desc: "The HTTP method of the request, such as `Core\\Http\\Method::Get`.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The path of the request, such as `/orders/7`. A `?` and the text after it \
                   are the query string.",
            shape: &[],
        },
        ParamDoc {
            name: "headers",
            desc: "The headers of the request, with the header name as the key. If you give no \
                   `content-type` or `content-length`, the length is set from `body`.",
            shape: &[],
        },
        ParamDoc {
            name: "body",
            desc: "The body of the request, as a `string` or as `bytes`. If you give no body, \
                   the request has no body. An empty string is a body of length 0.",
            shape: &[],
        },
        ParamDoc {
            name: "mount",
            desc: "The prefix the program is served under, such as `/shop`. \
                   `Core\\Request::mount()` returns it, and `Core\\Router::url` adds it in front \
                   of each link. The default is `\"\"`, which means the root.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Test\\Response`. `status()` returns the status the program set, or `200` if \
          it set none. `body()` returns the text the program wrote. A path that no route matches \
          still gets an answer, because your program decides what to do with it.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "There is no program to answer, because the call is not in `nvs test` or \
               `nvs run`. The call is also an error inside a request that `request` sent, \
               because a request cannot send another request.",
    }],
};

/// `Core\Test\Response::status`'s reference card — `rule:core-api/reference-card`.
const RESPONSE_STATUS_DOC: MethodDoc = MethodDoc {
    short: "The status the program under test declared for this request.",
    params: &[],
    ret: "The declared code, or `200` where the program declared none — the same default the \
          server writes for a program that only echoed.",
    errors: &[],
};

/// `Core\Test\Response::body`'s reference card — `rule:core-api/reference-card`.
const RESPONSE_BODY_DOC: MethodDoc = MethodDoc {
    short: "The bytes the program under test wrote while answering this request.",
    params: &[],
    ret: "Everything the program echoed, in order, and an empty string for a program that wrote \
          nothing. A program that threw still answers with whatever it had written first.",
    errors: &[],
};

/// `Core\Test::answerHttp`'s reference card — `rule:core-api/reference-card`.
const ANSWER_HTTP_DOC: MethodDoc = MethodDoc {
    short: "Says what one outbound URL answers with, and takes this test off the network — from \
            the first answer registered, every `Core\\Http\\Client` call the test makes is served \
            from the table and none of them connects.",
    params: &[
        ParamDoc {
            name: "url",
            desc: "The URL this answer serves: the whole of it, or a prefix ending in `*`. \
                   Nothing is resolved and no host is looked up — this is the text a call's own \
                   URL is compared against.",
            shape: &[],
        },
        ParamDoc {
            name: "status",
            desc: "The status the call answers with, as a wire status line can write it: three \
                   digits, `100` to `999`.",
            shape: &[],
        },
        ParamDoc {
            name: "json",
            desc: "A value the answer carries as a JSON document, written exactly as \
                   `Core\\Json::encode` would write it. The answer declares \
                   `application/json` for it unless the `headers` bag names a content type \
                   itself.",
            shape: &[],
        },
        ParamDoc {
            name: "body",
            desc: "The body the answer carries, for a reply that is not a JSON document — text, \
                   or the octets of a reply that is not text at all, which is what \
                   `Core\\Http\\Response::bytes` reads back and `::text` refuses. An answer may \
                   name this or `json` and not both.",
            shape: &[],
        },
        ParamDoc {
            name: "headers",
            desc: "The headers the answer carries, keyed by name — the same shape \
                   `Core\\Http\\Options` writes a request's headers in, plus one arm it has no \
                   use for: an array of strings under a name is a reply that carried that field \
                   on that many lines, which is what `Core\\Http\\Response::headers` reads back. \
                   A name is matched case-insensitively, as a header name is.",
            shape: &[],
        },
        ParamDoc {
            name: "tls",
            desc: "The TLS session the reply reports, from `Core\\Test::tlsSession`. \
                   `Core\\Http\\Response::tls` returns it for every call this answer serves. \
                   Without it, `tls()` returns `null`.",
            shape: &[],
        },
    ],
    ret: "Nothing. Answers accumulate, so a test registers as many as it has calls; a URL \
          answered exactly wins over one answered by a prefix, and the longest prefix wins among \
          prefixes.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The answer names both `json` and `body`, which are two spellings of one body; the \
               status is not one a status line can carry; or the answer gives a `tls` session to \
               an `http://` URL, which has none.",
    }],
};

/// `Core\Test::tlsSession`'s reference card — `rule:core-api/reference-card`.
const TLS_SESSION_DOC: MethodDoc = MethodDoc {
    short: "Builds a TLS session for a test, with a real certificate chain, so a test can read a \
            `Core\\Http\\TlsInfo` without a network.",
    params: &[
        ParamDoc {
            name: "version",
            desc: "`TLSv1.3` or `TLSv1.2`. The default is `TLSv1.3`.",
            shape: &[],
        },
        ParamDoc {
            name: "cipher",
            desc: "A cipher suite this build supports for that version, under its IANA name. The \
                   default is `TLS_AES_128_GCM_SHA256` for TLS 1.3 and \
                   `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256` for TLS 1.2.",
            shape: &[],
        },
        ParamDoc {
            name: "verified",
            desc: "What `verified()` returns. The default is `true`.",
            shape: &[],
        },
        ParamDoc {
            name: "subject",
            desc: "The leaf certificate's subject, written as `subject()` returns it: `KEY=value` \
                   pairs such as `CN=api.example.com, O=Shop`, with the keys `CN`, `O`, `OU`, \
                   `C`, `ST` and `L`. The default is `CN=example.com`.",
            shape: &[],
        },
        ParamDoc {
            name: "issuer",
            desc: "The subject of the CA certificate that signs the leaf, which is what \
                   `issuer()` returns. Written like `subject`. The default is `CN=Novis Test CA`.",
            shape: &[],
        },
        ParamDoc {
            name: "expiry",
            desc: "When the leaf certificate expires, in whole seconds. The default is 90 days \
                   after `Core\\Time::now()`, so a test with a fixed clock gets the same value \
                   every run.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Http\\TlsInfo`. Its `peerChain()` has two certificates: the leaf, then the CA \
          that signed it. Pass it to `Core\\Test::answerHttp` as `tls` to make a faked reply \
          report it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The version is not `TLSv1.3` or `TLSv1.2`; the cipher is not one this build \
               supports for that version; a subject or issuer is not written as `KEY=value` \
               pairs with the keys above; or the expiry is before 1950 or after 9999.",
    }],
};

/// `Core\Test::sentHttp`'s reference card — `rule:core-api/reference-card`.
const SENT_HTTP_DOC: MethodDoc = MethodDoc {
    short: "Returns every HTTP request the program sent after the first `Core\\Test::answerHttp` \
            call. The oldest request comes first.",
    params: &[],
    ret: "One `Core\\Test\\SentRequest` for each request, in the order the program sent them. \
          The list is empty when the program sent nothing. Nothing in a request is `tainted`, \
          because the program wrote all of it.",
    errors: &[],
};

/// `Core\Test::answerSocket`'s reference card — `rule:core-api/reference-card`.
const ANSWER_SOCKET_DOC: MethodDoc = MethodDoc {
    short: "Scripts the peer one outbound WebSocket URL answers with, and takes this test off the \
            network — a socket opened to a matching URL completes its handshake without \
            connecting, receives these frames in order and then a close.",
    params: &[
        ParamDoc {
            name: "url",
            desc: "The URL this peer answers: the whole of it, or a prefix ending in `*`. \
                   Nothing is resolved and no host is looked up — this is the text \
                   `Core\\Http\\Client::openSocket`'s own URL is compared against, and it is \
                   still judged for its scheme, so a `https` URL scripted here is refused at the \
                   row that opens it.",
            shape: &[],
        },
        ParamDoc {
            name: "frames",
            desc: "What the peer sends, in order: a `string` is a text message and `bytes` is a \
                   binary one, which is the same pair `send` and `sendBytes` write. A socket that \
                   has taken the last of them reads `null` from `receive`, which is the peer \
                   having closed.",
            shape: &[],
        },
        ParamDoc {
            name: "protocol",
            desc: "The subprotocol this peer's `101` chooses. It has to be one the call offered \
                   in `protocols`, and a peer choosing a name that was never offered makes \
                   `openSocket` throw — which is the refusal a real peer would meet. Left out, \
                   the handshake chooses none.",
            shape: &[],
        },
    ],
    ret: "Nothing. Peers accumulate, so a test scripts as many as it opens sockets; a URL \
          answered exactly wins over one answered by a prefix, and the longest prefix wins among \
          prefixes. Two sockets opened to one URL each read that peer's frames from their own \
          position.",
    errors: &[],
};

/// `Core\Test::sentSocket`'s reference card — `rule:core-api/reference-card`.
const SENT_SOCKET_DOC: MethodDoc = MethodDoc {
    short: "Every frame the program under test has sent over a scripted socket, oldest first — \
            what it said, rather than what it was told.",
    params: &[],
    ret: "One `Core\\Socket\\Message` per frame, in the order the program sent them, with `text` \
          filled for a `send` and `bytes` for a `sendBytes`. The frames of two sockets open at \
          once arrive interleaved in the one order they were written in, so a test that needs \
          them apart scripts one peer at a time.",
    errors: &[],
};

/// `Core\Test::double`'s reference card — `rule:core-api/reference-card`.
const DOUBLE_DOC: MethodDoc = MethodDoc {
    short: "A stand-in for `T` built from a shape of closures, one per method, which **is** a `T` \
            and may be passed wherever one is taken.",
    params: &[ParamDoc {
        name: "answers",
        desc: "One field per method of `T`, named as the method is and holding the closure that \
               answers it. A field `T` declares no method for, and a method of `T` the shape \
               leaves out, are each refused where the call is written.",
        shape: &[],
    }],
    ret: "The double, typed as `T`. It records every call made to it, which \
          `Core\\Test::assertCalled` reads afterwards, and no class the author never wrote \
          appears in a backtrace.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`T` declares more methods than one double can stand in for, which is a fixed \
               ceiling set well above the interfaces a test doubles.",
    }],
};

/// `Core\Test::partial`'s reference card — `rule:core-api/reference-card`.
const PARTIAL_DOC: MethodDoc = MethodDoc {
    short: "A stand-in for `T` that answers the methods `$answers` names and delegates every \
            other one to `$real`.",
    params: &[
        ParamDoc {
            name: "real",
            desc: "The implementation the un-overridden methods are forwarded to, receiver and \
                   arguments unchanged.",
            shape: &[],
        },
        ParamDoc {
            name: "answers",
            desc: "One field per method being overridden, named as the method is. A field `T` \
                   declares no method for is refused where the call is written; unlike \
                   `double`, a method left out is not, that being what `$real` is for.",
            shape: &[],
        },
    ],
    ret: "The partial, typed as `T`. Every call is recorded whether the closure or `$real` \
          answered it.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`double`'s ceiling, for the same reason and with the same two counts.",
    }],
};

/// `Core\Test::assertCalled`'s reference card — `rule:core-api/reference-card`.
const ASSERT_CALLED_DOC: MethodDoc = MethodDoc {
    short: "Asserts that `$method` was called on `$double` — after the exercise, against the record \
            the double kept, rather than as an expectation declared in advance.",
    params: &[
        ParamDoc {
            name: "double",
            desc: "The double or partial the call is being asserted about.",
            shape: &[],
        },
        ParamDoc {
            name: "method",
            desc: "The method, written as `Mailer::send` — a compile-checked reference, so \
                   renaming the method updates or breaks the test.",
            shape: &[],
        },
        ParamDoc {
            name: "times",
            desc: "The exact number of calls expected. Omitted, any number above zero holds.",
            shape: &[],
        },
        ParamDoc {
            name: "with",
            desc: "The arguments one of the recorded calls must have been made with, compared as \
                   `Core\\Test::assertEquals` compares. Omitted, the arguments are not looked at.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "Prefixed to the failure, as on every other assertion.",
            shape: &[],
        },
    ],
    ret: "Nothing. A call that was not made, or was made a different number of times or with \
          different arguments, throws.",
    errors: &[
        ErrorDoc {
            error: "Core\\Test\\Failure",
            desc: "The record does not hold the call the options describe. The message names the \
                   method, what was expected of it and every call the record does hold.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$double` is an ordinary object rather than something \
                   `Core\\Test::double` or `Core\\Test::partial` built, so there is no record to \
                   read.",
        },
    ],
};

/// `Core\Test::assertNeverCalled`'s reference card — `rule:core-api/reference-card`.
const ASSERT_NEVER_CALLED_DOC: MethodDoc = MethodDoc {
    short: "Asserts that `$method` was never called on `$double` — `assertCalled`'s other half, \
            over the same record.",
    params: &[
        ParamDoc {
            name: "double",
            desc: "The double or partial the absence is being asserted about.",
            shape: &[],
        },
        ParamDoc {
            name: "method",
            desc: "The method, written as `Mailer::purge` and checked as `assertCalled`'s is.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "Prefixed to the failure, as on every other assertion.",
            shape: &[],
        },
    ],
    ret: "Nothing. One recorded call is enough to throw.",
    errors: &[
        ErrorDoc {
            error: "Core\\Test\\Failure",
            desc: "The method was called. The message names the first call the record holds and \
                   how many it holds in all.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`assertCalled`'s, for the same reason: `$double` is not a double.",
        },
    ],
};

/// `Core\Test::assertCompletes`'s reference card — `rule:core-api/reference-card`.
const ASSERT_COMPLETES_DOC: MethodDoc = MethodDoc {
    short: "Runs `$body` with the test's clock advanced by `$settings.within`, and asserts that it \
            left nothing still running. Wall-clock time is never read, so a budget written in \
            seconds is spent in microseconds and two runs answer identically.",
    params: &[
        ParamDoc {
            name: "body",
            desc: "The work to run. It is called once, on the test's own task, and whatever it \
                   answers is dropped.",
            shape: &[],
        },
        ParamDoc {
            name: "settings",
            desc: "The budget, written as a literal because there is no duration this member \
                   could pick for a caller.",
            shape: &[ShapeKeyDoc {
                key: "within",
                ty: "Core\\Time\\Duration",
                desc: "How far the virtual clock moves before `$body` runs, so a retry, a backoff \
                       or a timeout inside it elapses at once. `Core\\Test::advance` moves the \
                       same clock, and the move is permanent: the test reads the advanced clock \
                       from here on.",
            }],
        },
        ParamDoc {
            name: "message",
            desc: "Prefixed to the failure, as on every other assertion.",
            shape: &[],
        },
    ],
    ret: "Nothing. A `$body` that returned having left a task of its own still running throws, \
          naming the budget it overran — `rule:testing/task-tree-and-virtual-clock`, read at the \
          one moment it means anything.",
    errors: &[
        ErrorDoc {
            error: "Core\\Test\\Failure",
            desc: "`$body` returned with work of its own still running. The message names how \
                   many tasks and the `within` they were given.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "The test declared no `at:`, so there is no fixed clock to spend the budget \
                   against — `Core\\Test::advance`'s refusal, for its reason.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The advanced clock lies outside the range an `Instant` can hold, as on \
                   `Core\\Test::advance`.",
        },
    ],
};

/// `Core\Test\SentRequest::method`'s reference card — `rule:core-api/reference-card`.
const SENT_METHOD_DOC: MethodDoc = MethodDoc {
    short: "The verb this call carried, as the `Core\\Http\\Method` case the member that made it \
            is named for.",
    params: &[],
    ret: "The case — `Core\\Http\\Method::Get` for a `Core\\Http\\Client::get`, and so on for \
          every row.",
    errors: &[],
};

/// `Core\Test\SentRequest::url`'s reference card — `rule:core-api/reference-card`.
const SENT_URL_DOC: MethodDoc = MethodDoc {
    short: "The URL this call was made to, as the program wrote it.",
    params: &[],
    ret: "The whole URL, unchanged — not the pattern the answer was registered under, so a test \
          answering a prefix can still assert the exact path its subject asked for.",
    errors: &[],
};

/// `Core\Test\SentRequest::header`'s reference card — `rule:core-api/reference-card`.
const SENT_HEADER_DOC: MethodDoc = MethodDoc {
    short: "What this call carried under one header name, so a test can assert the \
            authorization, the content type or the trace header its subject composed.",
    params: &[ParamDoc {
        name: "name",
        desc: "The header to read, matched case-insensitively as a header name is.",
        shape: &[],
    }],
    ret: "The value, or `null` where the request carried no such header.",
    errors: &[],
};

/// `Core\Test\SentRequest::body`'s reference card — `rule:core-api/reference-card`.
const SENT_BODY_DOC: MethodDoc = MethodDoc {
    short: "The bytes this call carried, so a test can assert the document its subject sent \
            rather than only the URL it sent it to.",
    params: &[],
    ret: "The request body, and an empty `bytes` for a call that carried none.",
    errors: &[],
};

/// `Core\Test::assertSame`'s reference card — `rule:core-api/reference-card`.
const ASSERT_SAME_DOC: MethodDoc = MethodDoc {
    short: "Asserts `$actual` is identical to `$expected` — two objects are the same object \
            and nothing else is — as PHPUnit's `assertSame` does, subject first.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The value under test.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "What it must be identical to; `T` is bound from `$actual`, so a type \
                   mismatch between the two is a compile error.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "The two are not identical; the failure is recorded in the ledger before it is \
               thrown, so a `catch` cannot erase it.",
    }],
};

/// `Core\Test::assertEquals`'s reference card — `rule:core-api/reference-card`.
const ASSERT_EQUALS_DOC: MethodDoc = MethodDoc {
    short: "Asserts `$actual` equals `$expected` — identity everywhere except two objects, \
            which are compared through `Comparable::compareTo` — as PHPUnit's `assertEquals` \
            does, subject first.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The value under test.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "What it must equal; `T` is bound from `$actual`, so a type mismatch between \
                   the two is a compile error.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[
        ErrorDoc {
            error: "Core\\Test\\Failure",
            desc: "The two are not equal; the failure is recorded in the ledger before it is \
                   thrown, so a `catch` cannot erase it.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "Both are objects and `$actual`'s class declares no `compareTo`.",
        },
    ],
};

/// `Core\Test::assertEqualsDeep`'s reference card — `rule:core-api/reference-card`.
const ASSERT_EQUALS_DEEP_DOC: MethodDoc = MethodDoc {
    short: "Asserts `$actual` and `$expected` agree structurally — arrays entry by entry, \
            objects property by property, everything else by identity — and reports where \
            they first differ rather than only that they do.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The value under test.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "What it must agree with everywhere; `T` is bound from `$actual`, so a type \
                   mismatch between the two is a compile error.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[
        ErrorDoc {
            error: "Core\\Test\\Failure",
            desc: "The two differ somewhere; the diagnosis names the path, written as a \
                   program would subscript `$actual`, and the failure is recorded in the \
                   ledger before it is thrown.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The walk passed 64 levels of containment without reaching a scalar.",
        },
    ],
};

/// `Core\Test::assertTrue`'s reference card — `rule:core-api/reference-card`.
const ASSERT_TRUE_DOC: MethodDoc = MethodDoc {
    short: "Asserts `$actual` is `true`, as PHPUnit's `assertTrue` does; the subject is a \
            declared `bool`, so anything else is refused at the checker rather than read \
            through the truthy table.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The `bool` under test.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "`$actual` is `false`; the failure is recorded in the ledger before it is thrown, \
               so a `catch` cannot erase it.",
    }],
};

/// `Core\Test::assertNull`'s reference card — `rule:core-api/reference-card`.
const ASSERT_NULL_DOC: MethodDoc = MethodDoc {
    short: "Asserts `$actual` is `null`, as PHPUnit's `assertNull` does; the subject is \
            `mixed`, so a value of any type may be asked.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The value under test.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "`$actual` is not `null`; the failure is recorded in the ledger before it is \
               thrown, so a `catch` cannot erase it.",
    }],
};

/// `Core\Test::assertCount`'s reference card — `rule:core-api/reference-card`.
const ASSERT_COUNT_DOC: MethodDoc = MethodDoc {
    short: "Asserts `$actual` holds exactly `$expected` entries — `Core\\Arr::count`'s own \
            signature — as PHPUnit's `assertCount` does, subject first. A `string`'s length \
            is asserted through `Core\\Str::length`, which says which length was meant.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The array under test.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "The number of entries it must hold.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "The array holds a different number of entries; the failure is recorded in the \
               ledger before it is thrown, so a `catch` cannot erase it.",
    }],
};

/// `Core\Test::assertContains`'s reference card — `rule:core-api/reference-card`.
const ASSERT_CONTAINS_DOC: MethodDoc = MethodDoc {
    short: "Asserts some entry of `$actual` is `$expected` under strict identity — the question \
            `Core\\Arr::contains` answers, and the same answer — as PHPUnit's `assertContains` \
            does, subject first. A substring is asserted through `Core\\Str::contains`, which \
            says which containment was meant.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The array under test.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "The entry it must hold, compared by identity; `int`, `uint`, `float` and \
                   `decimal` are one numeric domain, so `1` finds `1.0`.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "No entry of the array is identical to `$expected`; the failure is recorded in \
               the ledger before it is thrown, so a `catch` cannot erase it.",
    }],
};

/// `Core\Test::assertMatchesInline`'s reference card — `rule:core-api/reference-card`.
const ASSERT_MATCHES_INLINE_DOC: MethodDoc = MethodDoc {
    short: "Asserts that `$actual`, rendered as `Core\\Debug::render` renders it, is exactly \
            `$expected` — an inline snapshot, whose expectation is a literal in the test's own \
            source rather than a file beside it.",
    params: &[
        ParamDoc {
            name: "actual",
            desc: "The value to render; a `secret` property inside it renders redacted, so a \
                   snapshot cannot become where a secret is committed.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "The rendering this value is expected to have, written inline; `nvs test \
                   --update` writes it here for you, replacing this literal and nothing else in \
                   the file.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "The rendering differs from `$expected`; the failure quotes both, and is recorded \
               in the ledger before it is thrown, so a `catch` cannot erase it.",
    }],
};

/// `Core\Test::assertThrows`'s reference card — `rule:core-api/reference-card`.
const ASSERT_THROWS_DOC: MethodDoc = MethodDoc {
    short: "Runs `$body` and asserts it throws `$expected` or a subclass of it, as PHPUnit's \
            `expectException` does; the throw it judged is consumed, so only the assertion's \
            own verdict propagates.",
    params: &[
        ParamDoc {
            name: "body",
            desc: "The closure to run; a value it returns is released.",
            shape: &[],
        },
        ParamDoc {
            name: "expected",
            desc: "The fully-qualified class name, as `ParseError::class` folds to; the thrown \
                   class or any ancestor of it matches.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger. A `FATAL` or an \
          exit from the body is nobody's assertion to judge and propagates unchanged, and a \
          failed assertion inside the body keeps its own ledger entry.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "`$body` returned without throwing, or threw a class that is not `$expected` — \
               reported with the class it did throw and that throw's message; the failure is \
               recorded in the ledger before it is thrown.",
    }],
};

/// `Core\Test::assertDoesNotThrow`'s reference card — `rule:core-api/reference-card`.
const ASSERT_DOES_NOT_THROW_DOC: MethodDoc = MethodDoc {
    short: "Runs `$body` and asserts it returns without throwing — the way out of the rule \
            that a test asserting nothing fails; the throw it judged is consumed, so only the \
            assertion's own verdict propagates.",
    params: &[
        ParamDoc {
            name: "body",
            desc: "The closure to run; a value it returns is released.",
            shape: &[],
        },
        ParamDoc {
            name: "message",
            desc: "A prefix written in front of the failure's own diagnosis; the default is \
                   none.",
            shape: &[],
        },
    ],
    ret: "Nothing; the assertion is recorded as held in the test's ledger. A `FATAL` or an \
          exit from the body is nobody's assertion to judge and propagates unchanged.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "`$body` threw — a failed assertion inside it included, which keeps its own \
               ledger entry — reported with the throw's message; the failure is recorded in \
               the ledger before it is thrown.",
    }],
};

/// `Core\Test::expectFailure`'s reference card — `rule:core-api/reference-card`.
const EXPECT_FAILURE_DOC: MethodDoc = MethodDoc {
    short: "Runs `$body` and asserts that an assertion inside it failed, then discharges those \
            failures from the test's ledger — the one greppable spelling for a failure that \
            was on purpose, and the only way an entry ever leaves the ledger.",
    params: &[ParamDoc {
        name: "body",
        desc: "The closure to run; what the ledger records decides the verdict, so a body that \
               caught its own failed assertion and returned normally still counts as having \
               failed.",
        shape: &[],
    }],
    ret: "Nothing; the failed assertions inside `$body` are discharged and the throw carrying \
          one is consumed, while a passing assertion inside it stays counted.",
    errors: &[ErrorDoc {
        error: "Core\\Test\\Failure",
        desc: "`$body` ran without any assertion failing; that failure is recorded in the \
               ledger before it is thrown, so a `catch` cannot erase it.",
    }],
};

nvs_runtime::nvs_helper! {
    /// `Core\Test::advance(Duration $by): void` — `rule:testing/determinism-declared-on-the-test`'s mutator for
    /// the clock `#[Test(at: …)]` fixed, and the only thing in the language
    /// that moves one.
    ///
    /// **A `LogicError` and not a fatal when no clock is fixed**, because it is
    /// a call written in the wrong place — a test that forgot its `at:`, or a
    /// helper reached from outside a test at all — and `docs/spec` § 10's
    /// `LogicError` is the class for exactly that. The alternative worth naming
    /// is answering silently, which is worse than either: a test that advanced
    /// a clock nothing had frozen would then assert against the host's, and
    /// pass or fail by how long the suite happened to take.
    ///
    /// The clock itself lives on [`nvs_runtime::Ctx`] and the isolate is what
    /// scopes it, so advancing it here cannot be observed by any other test —
    /// § 2 gives each one its own context, and this writes only to that.
    ///
    /// **Both refusals open on the same prefix**, which is the shape
    /// `Core\Time::parse` already uses and which matters twice here. A reader
    /// catching either is catching "the clock did not move", one member having
    /// one thing it can be asked to do and two ways of being unable to; and the
    /// coverage gate in `crates/nvs-stdlib/tests/conformance_coverage.rs` keys
    /// a site on the literal stem before its first hole, so one case discharges
    /// both — which it has to, the second refusal needing a fixed clock to
    /// reach and a `.nvst` case never being inside a `#[Test]`.
    fn nvs_core_test_advance(ctx, args: [1]) {
        let by = crate::time::nanos_of(args, 0, "advance")?;
        let Some(nanos) = ctx.fixed_clock() else {
            let why = "this test declared no `at:`, so it has no fixed clock to advance";
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("Core\\Test::advance(): {why}"),
            ));
        };
        let moved = nanos + i128::from(by);
        if crate::time::instant_at_nanos(moved).is_none() {
            let why = "the advanced clock lies outside the range an `Instant` can hold, about \
                       ±9999 years";
            return Err(Fault::thrown(format!("Core\\Test::advance(): {why}")));
        }
        ctx.set_fixed_clock(moved);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertCompletes(callable $body, {within: Duration}, {message?: string}): void`
    /// — ADR 0079 § 16's budget, spent against the virtual clock.
    ///
    /// **The clock moves before `$body` runs**, which is the ordering that
    /// makes the member worth having: a retry, a backoff or a timeout inside
    /// the body measures itself against a clock that has already been granted
    /// the budget, so it elapses at once instead of waiting out the seconds it
    /// describes. Moving it afterwards would leave the body running against
    /// the clock it started on and the advance with nothing to affect. The
    /// move is `Core\Test::advance`'s, down to the range check, so a test
    /// mixing the two members is moving one clock.
    ///
    /// **What "still running" means is read the way the runner reads it** —
    /// `nvs_host::children_still_running` against a count taken before the
    /// call, so a task the test spawned earlier is not charged to this body and
    /// one the body left behind is. `rule:testing/task-tree-and-virtual-clock`
    /// is the same fact at the test's own boundary, and a body whose children
    /// were all awaited is out of the tree by the time it returns.
    ///
    /// A `$body` that threw is not this member's failure: the [`Fault`] goes
    /// back up as it would from any other frame, so the test reports the throw
    /// rather than a budget it never reached.
    fn nvs_core_test_assert_completes(ctx, args: [3]) {
        let within = crate::time::nanos_of(args, WITHIN_ARG, "assertCompletes")?;
        let Some(nanos) = ctx.fixed_clock() else {
            let why = "this test declared no `at:`, so it has no fixed clock to spend `within` \
                       against";
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("Core\\Test::assertCompletes(): {why}"),
            ));
        };
        let moved = nanos + i128::from(within);
        if crate::time::instant_at_nanos(moved).is_none() {
            let why = "the advanced clock lies outside the range an `Instant` can hold, about \
                       ±9999 years";
            return Err(Fault::thrown(format!("Core\\Test::assertCompletes(): {why}")));
        }
        ctx.set_fixed_clock(moved);
        let before = nvs_host::children_still_running();
        let answered = nvs_runtime::call_closure(ctx, args[BODY_ARG], &[])?;
        #[expect(
            unsafe_code,
            reason = "`call_closure` hands back a value the caller owns, and this one is \
                      never handed on"
        )]
        unsafe {
            answered.release();
        }
        let left = nvs_host::children_still_running().saturating_sub(before);
        if left > 0 {
            let budget = nvs_syntax::duration::render(within);
            let detail = format!(
                "`$body` returned with {left} task(s) of its own still running, after the \
                 {budget} `within` named"
            );
            return Err(failed(
                ctx,
                "assertCompletes",
                &detail,
                args[COMPLETES_MESSAGE_ARG],
            ));
        }
        Ok(held(ctx, "assertCompletes"))
    }
}

/// The `array<string>` a scripted flow is written as, copied out line by line.
///
/// Owned `String`s for [`crate::process`]'s `argv_of` reason: the queue outlives
/// this call, and a slot's `Value` is borrowed from the caller's array rather
/// than retained.
///
/// # Errors
///
/// A [`Fault::fatal`] for an element that is not text — unreachable from
/// source, since the row declares `array<string>` and `E0401` refuses anything
/// else a phase earlier.
fn lines_of(value: Value) -> Result<Vec<String>, Fault> {
    let array = value.array_ptr().ok_or_else(|| {
        // Unreachable from source: the row declares `array<string>` here, so
        // `E0401` refuses any other argument a phase before this runs.
        Fault::fatal(format!(
            "Core\\Test::scriptAnswers expected an `array`, got tag {}",
            value.tag_byte()
        ))
    })?;
    let array = crate::arr::borrowed(array);
    let mut lines = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let element = array
            .value_at(slot)
            .expect("next_slot only names live entries");
        let line = element.as_text().ok_or_else(|| {
            // Unreachable from source, for the element as for the array: the
            // declared `array<string>` is what `E0401` checks, so a non-text
            // element never reaches this crate.
            Fault::fatal(format!(
                "Core\\Test::scriptAnswers expected a `string` answer, got tag {}",
                element.tag_byte()
            ))
        })?;
        lines.push(line.to_owned());
    }
    Ok(lines)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::scriptAnswers(array<string> $answers): void` — `rule:tooling/a-prompt-is-a-core-member`
    /// 's last paragraph, which is the whole of what makes an interactive
    /// flow assertable: prompts drain a queue the test supplied rather than
    /// reading a terminal.
    ///
    /// **One member for all five prompts, and it names none of them.** The
    /// queue is drained in `nvs_stdlib::cli`'s `ask_terminal`, which every
    /// prompt already goes through, so `ask`, `confirm`, `select`,
    /// `multiSelect` and `secret` gain this at once and a sixth prompt would
    /// gain it by construction. A per-member spelling — `answerAsk`,
    /// `answerSelect` — would make the test say which member asked, which is
    /// exactly the coupling to the subject's internals a test should not have.
    ///
    /// **Why it lives here rather than on `Core\Cli`.** Scripting an answer is
    /// something a *test* does to its subject, and `Core\Cli`'s own members are
    /// what the subject calls; a filler on `Core\Cli` would be a way for
    /// production code to answer its own prompts, which is a door `rule:tooling/a-prompt-is-a-core-member`
    /// has no reason to open. The state is on `nvs_runtime::Ctx` beside the
    /// fixed clock, and `rule:testing/isolate-per-test`'s per-test isolate is what scopes it —
    /// that field's docs are the home of both decisions.
    ///
    /// **No refusal for a call outside a test**, which is where this differs
    /// from `Core\Test::advance`. `advance` has a precondition it can check —
    /// a clock something else fixed — while a queue nothing drains is simply a
    /// queue nothing drains: there is nothing to be wrong about, and the honest
    /// answer to "who else could reach this" is the class name it is spelled
    /// under.
    fn nvs_core_test_script_answers(ctx, args: [1]) {
        ctx.script_answers(lines_of(args[0])?);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::serverUrl(): ?string` — `rule:testing/in-process-request`'s second mechanism,
    /// as the one thing a test can observe of it.
    ///
    /// **`null` rather than a throw for a context with no listener**, which is
    /// the opposite of the choice `advance` made one member up, and the
    /// difference is what the two answer. A clock that was never fixed cannot
    /// be advanced *at all*, so there is nothing to hand back and the mistake
    /// is worth naming; an address is an ordinary optional reading, and a `null`
    /// is what lets the member be asked from anywhere — which is what makes its
    /// conformance cases readings rather than three spellings of one refusal.
    /// The positive half needs a listener, so `nvs_cli::runner`'s own
    /// `a_test_with_server_true_gets_an_ephemeral_listener` is where it is
    /// asserted; no `.nvst` case is ever inside a `#[Test]`.
    ///
    /// Nothing here binds anything: the socket was bound on the parent's side,
    /// before this isolate existed, and what crossed is the text
    /// (`nvs_runtime::Ctx::test_server`).
    fn nvs_core_test_server_url(ctx, args: [0]) {
        let _ = args;
        match ctx.test_server() {
            Some(url) => Ok(Value::str(nvs_runtime::NvsStr::new(url.as_bytes()))),
            None => Ok(Value::null()),
        }
    }
}

/// The widest status a status line can carry, and the narrowest.
///
/// Three digits is what the wire has room for, and nothing narrower is the
/// bound: a test of a client's behaviour over a status no registry names is a
/// test worth writing, while `0` and `1000` are answers no origin can send and
/// so are refusals rather than fixtures.
const STATUS_FLOOR: u64 = 100;
/// See [`STATUS_FLOOR`].
const STATUS_CEILING: u64 = 999;

/// The `content-type` an answer written from `json` declares for itself.
const JSON_CONTENT_TYPE: &str = "application/json";

/// A registered answer's `headers` bag, as the field lines a reply carries:
/// names lower-cased, and one entry per line rather than per name.
///
/// The lower-casing happens here and not in [`crate::http::headers_of`], which
/// reads the *request* half of the same shape and keeps the names the program
/// wrote. What this adds beyond that walk is [`ANSWER_HEADER`]'s second arm: a
/// value that is itself a list registers that name's lines in order, which is
/// the only way a table can say what a reply that repeated a field said.
///
/// # Errors
///
/// A [`Fault::fatal`] for a bag, an entry or a line that is not what
/// [`ANSWER`] declares — all of them refused by `E0401` a phase earlier, so
/// none is reachable from source.
fn answer_headers_of(bag: Value, member: &str) -> Result<Vec<(String, String)>, Fault> {
    let mistyped = |what: &str, value: Value| {
        Fault::fatal(format!(
            "{member} expected {what} for `headers`, got tag {}",
            value.tag_byte()
        ))
    };
    let array = bag.array_ptr().ok_or_else(|| mistyped("an `array`", bag))?;
    let array = crate::arr::borrowed(array);
    let mut headers = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let key = array
            .key_at(slot)
            .expect("next_slot only names live entries");
        // An array key is `int|string` and neither can be invalid UTF-8, for
        // the reasons `Core\Str::replaceAll`'s own cursor states in full.
        let name = std::str::from_utf8(key.as_bytes())
            .map_err(|_| Fault::fatal(format!("{member} found a header name that is not text")))?
            .to_ascii_lowercase();
        let held = array
            .value_at(slot)
            .expect("next_slot only names live entries");
        if let Some(line) = held.as_text() {
            headers.push((name, line.to_owned()));
            continue;
        }
        let lines = held
            .array_ptr()
            .ok_or_else(|| mistyped("a `string` or an `array<string>`", held))?;
        let lines = crate::arr::borrowed(lines);
        let mut at = 0_usize;
        while let Some(inner) = lines.next_slot(at) {
            at = inner + 1;
            let line = lines
                .value_at(inner)
                .expect("next_slot only names live entries");
            let line = line.as_text().ok_or_else(|| mistyped("a `string`", line))?;
            headers.push((name.clone(), line.to_owned()));
        }
    }
    Ok(headers)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::answerHttp(string $url, uint $status, {json?, body?, headers?, tls?}): void`
    /// — `rule:testing/an-outbound-call-is-answered-from-a-table`, and the
    /// switch that takes a test off the network.
    ///
    /// **The first call is what arms the table**, and everything after it is
    /// one more row: there is no member that turns faking *on* without
    /// registering an answer, because the all-or-nothing rule is about what a
    /// call that matches no row does, and a table with no rows in it would then
    /// make every outbound call in the test a refusal it never asked for.
    ///
    /// **No refusal for a call outside a test**, which is
    /// `Core\Test::scriptAnswers`'s reason one member up: a table nothing
    /// consults is a table nothing consults. What is worth naming is that this
    /// door only ever *narrows* what the program can reach — a call answered
    /// from here opens no socket, resolves no name and asks no capability,
    /// because there is nothing for a capability to be asked about.
    ///
    /// **`{json: null}` is an answer whose body is the JSON document `null`**,
    /// and omitting `json` is no body at all: the option admits `null`, so it
    /// omits as the never-written marker
    /// (`rule:core-api/omission-is-not-a-written-null`) and the two arrive
    /// under different tags rather than as one argument.
    fn nvs_core_test_answer_http(ctx, args: [6]) {
        let member = "Core\\Test::answerHttp";
        let url = args[0].as_text().ok_or_else(|| {
            // Unreachable from source: the row's first parameter is
            // `CoreTy::Text`, so `E0401` refuses anything else a phase earlier.
            Fault::fatal(format!("{member} expected a `string` URL, got tag {}", args[0].tag_byte()))
        })?;
        let status = args[1].as_uint().ok_or_else(|| {
            // Unreachable from source, for the row's `CoreTy::Uint` and the
            // same reason as the URL above.
            Fault::fatal(format!("{member} expected a `uint` status, got tag {}", args[1].tag_byte()))
        })?;
        if !(STATUS_FLOOR..=STATUS_CEILING).contains(&status) {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{member}(): a status line carries three digits, so an answer's status is \
                     between {STATUS_FLOOR} and {STATUS_CEILING}, and this one is {status}"
                ),
            ));
        }

        let written = matches!(args[3].tag(), Some(Tag::Str | Tag::Bytes));
        let encoded = if matches!(args[2].tag(), Some(Tag::Unset)) {
            None
        } else {
            Some(crate::json::written(ctx, args[2], member)?)
        };
        if written && encoded.is_some() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{member}(): `json` and `body` are two spellings of one body, so an answer \
                     names one of them"
                ),
            ));
        }

        let tls = if matches!(args[5].tag(), Some(Tag::Null)) {
            None
        } else {
            Some(crate::http::answered_tls(args[5], member)?)
        };
        if tls.is_some()
            && url
                .get(..7)
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
        {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{member}(): {url} is a plain `http` URL, and a reply over plain `http` has \
                     no TLS session to report"
                ),
            ));
        }

        let mut headers = answer_headers_of(args[4], member)?;
        let body = match encoded {
            Some(document) => {
                if !headers.iter().any(|(name, _)| name == "content-type") {
                    headers.push(("content-type".to_owned(), JSON_CONTENT_TYPE.to_owned()));
                }
                document.into_bytes()
            }
            None => args[3]
                .as_bytes()
                .or_else(|| args[3].as_text().map(str::as_bytes))
                .unwrap_or_default()
                .to_vec(),
        };

        ctx.faked_http_mut().answer(nvs_runtime::HttpAnswer {
            url: url.to_owned(),
            status: u16::try_from(status).unwrap_or(u16::MAX),
            headers,
            body,
            tls,
        });
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::tlsSession({version?, cipher?, verified?, subject?, issuer?, expiry?}):
    /// Core\Http\TlsInfo` — a TLS session a test describes, for the reply
    /// `Core\Test::answerHttp`'s `tls` option makes report it (ADR 0217).
    ///
    /// `nvs_host::tls::described` checks the version and the suite and issues
    /// the chain; this reads the options and fills the two defaults that need
    /// the test's own clock, so a `#[Test(at: …)]` gets the same expiry every
    /// run.
    ///
    /// **No refusal outside a test**, for `answerHttp`'s reason: the session
    /// is one the program built itself, and nothing trusts it.
    ///
    /// # Errors
    ///
    /// A `LogicError` carrying `described`'s sentence.
    fn nvs_core_test_tls_session(ctx, args: [6]) {
        let member = "Core\\Test::tlsSession";
        let now = crate::time::wall_clock(ctx).ok_or_else(|| {
            // Unreachable from source: `Core\Test::advance` refuses to store a
            // reading outside the representable range.
            Fault::fatal(format!("{member} read a fixed clock no instant names"))
        })?;
        let now = std::time::SystemTime::from(now);
        let expiry = if matches!(args[5].tag(), Some(Tag::Null)) {
            now + DEFAULT_TLS_LIFETIME
        } else {
            std::time::SystemTime::from(crate::time::instant_of(args, 5, member)?)
        };
        let description = nvs_host::tls::Description {
            version: args[0].as_text(),
            cipher: args[1].as_text(),
            subject: args[3].as_text().unwrap_or(DEFAULT_TLS_SUBJECT),
            issuer: args[4].as_text().unwrap_or(DEFAULT_TLS_ISSUER),
            expiry,
            now,
        };
        let session = nvs_host::tls::described(&description)
            .map_err(|why| Fault::thrown_as(ThrownClass::Logic, format!("{member}(): {why}")))?;
        let verified = args[2].as_bool().unwrap_or(true);
        Ok(crate::http::tls_info_of(&session, verified))
    }
}

/// A scripted peer's `frames` list, as the payloads it plays.
///
/// # Errors
///
/// A [`Fault::fatal`] for a list or an element that is not what [`PEER_FRAME`]
/// declares, both refused by `E0401` a phase earlier and so unreachable from
/// source.
fn peer_frames_of(list: Value, member: &str) -> Result<Vec<nvs_runtime::SocketFrame>, Fault> {
    let mistyped = |what: &str, value: Value| {
        Fault::fatal(format!(
            "{member} expected {what} for `frames`, got tag {}",
            value.tag_byte()
        ))
    };
    let array = list
        .array_ptr()
        .ok_or_else(|| mistyped("an `array`", list))?;
    let array = crate::arr::borrowed(array);
    let mut frames = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let held = array
            .value_at(slot)
            .expect("next_slot only names live entries");
        // `bytes` before `string`, because the two tags are distinct and the
        // octets of a binary frame are not text this has to decide about: what
        // the element was written as is what the peer sends.
        if let Some(octets) = held.as_bytes() {
            frames.push(nvs_runtime::SocketFrame::Bytes(octets.to_vec()));
            continue;
        }
        let text = held
            .as_text()
            .ok_or_else(|| mistyped("a `string` or `bytes`", held))?;
        frames.push(nvs_runtime::SocketFrame::Text(text.to_owned()));
    }
    Ok(frames)
}

/// One scripted or sent frame, as the `Core\Socket\Message` both halves of RFC
/// 6455 answer in: `topic` and `value` are `null`, which is what a frame off a
/// wire fills them with.
fn frame_message(frame: &nvs_runtime::SocketFrame) -> Value {
    let (text, bytes) = match frame {
        nvs_runtime::SocketFrame::Text(payload) => (
            Value::str(nvs_runtime::NvsStr::new(payload.as_bytes())),
            Value::null(),
        ),
        nvs_runtime::SocketFrame::Bytes(payload) => (
            Value::null(),
            Value::bytes(nvs_runtime::NvsStr::new(payload)),
        ),
    };
    crate::instance::build(
        &crate::socket::MESSAGE,
        [Value::null(), text, bytes, Value::null()],
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::answerSocket(string $url, array<string|bytes> $frames, {protocol?}): void`
    /// — `rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`, and
    /// the same switch `Core\Test::answerHttp` throws.
    ///
    /// **One switch and not two**: a test that scripts a peer has taken itself
    /// off the network for calls as well, because the all-or-nothing rule is
    /// about what an outbound anything that matches no row does, and a table
    /// armed for one half only would let the other half reach a host.
    fn nvs_core_test_answer_socket(ctx, args: [3]) {
        let member = "Core\\Test::answerSocket";
        let url = args[0].as_text().ok_or_else(|| {
            // Unreachable from source: the row's first parameter is
            // `CoreTy::Text`, so `E0401` refuses anything else a phase earlier.
            Fault::fatal(format!(
                "{member} expected a `string` URL, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let frames = peer_frames_of(args[1], member)?;
        let protocol = args[2].as_text().map(str::to_owned);
        ctx.faked_http_mut().answer_socket(nvs_runtime::SocketAnswer {
            url: url.to_owned(),
            protocol,
            frames,
        });
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::sentSocket(): array<Core\Socket\Message>` — every frame the
    /// program wrote to a scripted peer, oldest first.
    ///
    /// **The messages are built here rather than held as values**, which is
    /// [`nvs_core_test_sent_http`]'s reason one shape over: a frame that is
    /// never asked about costs its own octets and nothing else, and no
    /// reference to a `Core`-owned instance is held across the calls between a
    /// send and the assertion about it.
    fn nvs_core_test_sent_socket(ctx, args: [0]) {
        let _ = args;
        let mut out = nvs_runtime::NvsArray::new();
        for frame in ctx.faked_http().sent_frames() {
            out.append(frame_message(frame));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::sentHttp(): array<Core\Test\SentRequest>` — what the program
    /// sent while the table answered it, oldest first.
    ///
    /// **The records are built here rather than held as values**, which is what
    /// keeps `nvs_runtime::AnswerTable` plain data: a request that is never
    /// asked about costs a `String` and two `Vec`s, and no reference to a
    /// `Core`-owned instance is held across the calls between one send and the
    /// assertion about it.
    fn nvs_core_test_sent_http(ctx, args: [0]) {
        let _ = args;
        let mut out = nvs_runtime::NvsArray::new();
        for sent in ctx.faked_http().sent() {
            let mut headers = nvs_runtime::NvsArray::new();
            for (name, value) in &sent.headers {
                headers.set(
                    nvs_runtime::NvsStr::new(name.as_bytes()),
                    Value::str(nvs_runtime::NvsStr::new(value.as_bytes())),
                );
            }
            let ordinal = crate::router::method_case(&sent.verb).ok_or_else(|| {
                // Unreachable from source: the verb was written by the
                // `Core\Http\Client` row that recorded the call, and every one
                // of those is a case of this roster.
                Fault::fatal(format!(
                    "Core\\Test::sentHttp found a verb `Core\\Http\\Method` does not name: {}",
                    sent.verb
                ))
            })?;
            out.append(crate::instance::build(
                &SENT_REQUEST,
                [
                    Value::int(ordinal),
                    Value::str(nvs_runtime::NvsStr::new(sent.url.as_bytes())),
                    Value::array(headers),
                    Value::bytes(nvs_runtime::NvsStr::new(&sent.body)),
                ],
            ));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentRequest::method(): Core\Http\Method` — the verb the call
    /// carried, as the case the member that made it is named for.
    fn nvs_core_test_sent_method(_ctx, args: [1]) {
        sent_slot(args, SENT_METHOD_SLOT, "method")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentRequest::url(): string` — the URL as the program wrote it.
    fn nvs_core_test_sent_url(_ctx, args: [1]) {
        sent_slot(args, SENT_URL_SLOT, "url")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentRequest::body(): bytes` — the bytes the call carried.
    fn nvs_core_test_sent_body(_ctx, args: [1]) {
        sent_slot(args, SENT_BODY_SLOT, "body")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentRequest::header(string $name): ?string` — one header the
    /// call carried.
    ///
    /// **Case-insensitively, which is what a header name is**: the slot is
    /// keyed by a lower-cased name where the record was written, so the lookup
    /// lower-cases what it is asked and the two cannot disagree. A test
    /// asserting `Authorization` therefore reads the header a subject wrote as
    /// `authorization`, which is the same header and not a second one.
    fn nvs_core_test_sent_header(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &SENT_REQUEST, "header")?;
        let name = args[1].as_text().ok_or_else(|| {
            // Unreachable from source: the row's parameter is `CoreTy::Text`.
            Fault::fatal(format!(
                "Core\\Test\\SentRequest::header expected a `string` name, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let held = crate::instance::slot(receiver, SENT_HEADERS_SLOT);
        let array = held.array_ptr().ok_or_else(|| {
            // Unreachable from source: the slot is written by `sentHttp` above
            // and holds the array it built there.
            Fault::fatal(
                "Core\\Test\\SentRequest::header found a record it cannot read".to_owned(),
            )
        })?;
        let array = crate::arr::borrowed(array);
        let Some(value) = array.get(name.to_ascii_lowercase().as_bytes()) else {
            return Ok(Value::null());
        };
        #[expect(
            unsafe_code,
            reason = "`NvsArray::get` borrows the entry's reference from the receiver's own \
                      slot, which is live for the length of the call, and this value is being \
                      handed to the caller — which is exactly `Value::retain`'s obligation"
        )]
        // SAFETY: the receiver owns the entry's reference and outlives this call.
        unsafe {
            value.retain();
        }
        Ok(value)
    }
}

/// One [`SENT_REQUEST`] slot, handed to the caller with a reference of its own
/// — [`response_slot`]'s accounting, one class over.
fn sent_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &SENT_REQUEST, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    // SAFETY: the receiver owns the slot's reference and outlives this call.
    unsafe {
        held.retain();
    }
    Ok(held)
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_test_advance" => (nvs_core_test_advance as *const ()).cast(),
        "nvs_core_test_assert_completes" => (nvs_core_test_assert_completes as *const ()).cast(),
        "nvs_core_test_answer_http" => (nvs_core_test_answer_http as *const ()).cast(),
        "nvs_core_test_tls_session" => (nvs_core_test_tls_session as *const ()).cast(),
        "nvs_core_test_sent_http" => (nvs_core_test_sent_http as *const ()).cast(),
        "nvs_core_test_answer_socket" => (nvs_core_test_answer_socket as *const ()).cast(),
        "nvs_core_test_sent_socket" => (nvs_core_test_sent_socket as *const ()).cast(),
        "nvs_core_test_sent_method" => (nvs_core_test_sent_method as *const ()).cast(),
        "nvs_core_test_sent_url" => (nvs_core_test_sent_url as *const ()).cast(),
        "nvs_core_test_sent_header" => (nvs_core_test_sent_header as *const ()).cast(),
        "nvs_core_test_sent_body" => (nvs_core_test_sent_body as *const ()).cast(),
        "nvs_core_test_script_answers" => (nvs_core_test_script_answers as *const ()).cast(),
        "nvs_core_test_server_url" => (nvs_core_test_server_url as *const ()).cast(),
        "nvs_core_test_assert_same" => (nvs_core_test_assert_same as *const ()).cast(),
        "nvs_core_test_assert_equals" => (nvs_core_test_assert_equals as *const ()).cast(),
        "nvs_core_test_assert_equals_deep" => {
            (nvs_core_test_assert_equals_deep as *const ()).cast()
        }
        "nvs_core_test_assert_true" => (nvs_core_test_assert_true as *const ()).cast(),
        "nvs_core_test_assert_null" => (nvs_core_test_assert_null as *const ()).cast(),
        "nvs_core_test_assert_count" => (nvs_core_test_assert_count as *const ()).cast(),
        "nvs_core_test_assert_matches_inline" => {
            (nvs_core_test_assert_matches_inline as *const ()).cast()
        }
        "nvs_core_test_assert_contains" => (nvs_core_test_assert_contains as *const ()).cast(),
        "nvs_core_test_assert_throws" => (nvs_core_test_assert_throws as *const ()).cast(),
        "nvs_core_test_assert_does_not_throw" => {
            (nvs_core_test_assert_does_not_throw as *const ()).cast()
        }
        "nvs_core_test_expect_failure" => (nvs_core_test_expect_failure as *const ()).cast(),
        "nvs_core_test_request" => (nvs_core_test_request as *const ()).cast(),
        "nvs_core_test_response_status" => (nvs_core_test_response_status as *const ()).cast(),
        "nvs_core_test_response_body" => (nvs_core_test_response_body as *const ()).cast(),
        "nvs_core_test_double" => (nvs_core_test_double as *const ()).cast(),
        "nvs_core_test_partial" => (nvs_core_test_partial as *const ()).cast(),
        "nvs_core_test_assert_called" => (nvs_core_test_assert_called as *const ()).cast(),
        "nvs_core_test_assert_never_called" => {
            (nvs_core_test_assert_never_called as *const ()).cast()
        }
        _ => return None,
    })
}

// ============================================================================
// The members
// ============================================================================

/// The wire token one [`crate::router::METHOD`] ordinal spells.
///
/// The reverse of that module's `method_case`, and it reads the same roster
/// rather than a second copy of the eight names: an enum case's own spelling
/// upper-cased *is* the token, which is why there is no table here.
fn verb_of(ordinal: i64) -> Option<String> {
    crate::router::METHOD
        .cases
        .iter()
        .find(|(_, value)| *value == ordinal)
        .map(|(case, _)| case.to_ascii_uppercase())
}

/// The request `Core\Test::request`'s `target` and [`REQUEST_OPTIONS`] bag
/// describe, ready to be built into a carrier.
///
/// **A spec rather than a carrier assembled here.** `nvs_runtime::InboundSpec`
/// is what derives `content-type` and `content-length` from a body and what
/// leaves a field line the bag wrote alone; doing either here would be that
/// derivation's second copy, free to disagree with the served one the day
/// either moves.
///
/// Split out from the member so that what the bag becomes is assertable on its
/// own: answering the request wants a compiled unit under test, and what this
/// file owns is the reading of the bag rather than the answering.
///
/// # Errors
///
/// A [`Fault::fatal`] for a `headers` bag that is not `array<string>`, which
/// `E0401` refuses a phase earlier and so is unreachable from source.
fn described(verb: &str, target: &str, args: &[Value]) -> Result<nvs_runtime::InboundSpec, Fault> {
    // Split exactly as the door does: everything after the first `?` is the
    // query, undecoded, and a target with none has an empty one rather than no
    // query at all.
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut spec = nvs_runtime::InboundSpec::new(verb, path);
    spec.set_query(query);
    for (name, value) in crate::http::headers_of(args, 2, "Core\\Test::request")? {
        spec.push_header(&name, value.as_bytes());
    }
    // A body is described only where one was written: the omitted default is
    // `null`, and a request carrying no body is not one carrying an empty body.
    // `SpecBody::Raw` frames nothing and declares nothing, which is what the
    // bag's `body` promised.
    if let Some(octets) = args[3]
        .as_bytes()
        .or_else(|| args[3].as_text().map(str::as_bytes))
    {
        spec.set_body(nvs_runtime::SpecBody::Raw(octets.to_vec()))
            .expect("this bag spells a body one way, so there is no second spelling to refuse");
    }
    // The door the request is to have come through, and the root for a bag
    // naming none. Unreachable as anything but text — the option declares
    // `string` and defaults to `""`, so `E0401` refuses the rest a phase
    // earlier — and the path is the mount-stripped one either way, which is
    // what a `#[Route]` is declared against.
    spec.set_mount(args[4].as_text().unwrap_or(""), &[]);
    Ok(spec)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::request(Core\Http\Method $method, string $path, {headers?: array<string>, body?: string|bytes, mount?: string}): Core\Test\Response`
    /// — `rule:testing/in-process-request`'s in-process request.
    ///
    /// **Neither the match nor the dispatch happens here.** `rule:routing/matched-once-before-the-handler`'s
    /// match is taken on the far side of `nvs_runtime::inproc`, whose
    /// `Answering::answer` owns why: the table is a compile product of the unit
    /// under test, and a `#[Test]` method's own isolate shares compiled code
    /// with that unit and nothing else, so matching against *this* context's
    /// table would match against nothing. The dispatch is nobody's — `rule:routing/a-match-is-not-invocable`
    /// is why the matched handler is not called from anywhere: routes do
    /// not share a signature, so invoking one would be pre-binding converted
    /// parameters, which is dispatch. What runs is the program's own entry,
    /// exactly as a served request runs it.
    ///
    /// **What the bag carries arrives `tainted`**, which is
    /// `rule:testing/in-process-request`'s second sentence and needs nothing
    /// written here to hold: a field line and a body reach the program through
    /// `Core\Request`'s own members, and those rows declare the qualifier for
    /// every request alike. So a handler that forgets to launder a header fails
    /// its test rather than production, and the test that pins it says nothing
    /// about being synthetic.
    fn nvs_core_test_request(ctx, args: [5]) {
        let ordinal = args[0].as_int().unwrap_or(-1);
        let Some(verb) = verb_of(ordinal) else {
            // Unreachable from source — the parameter is `CoreTy::Enum`, so
            // `E0401` refuses anything but a case of the eight — and a fatal
            // rather than a throw for that reason: what it catches is a
            // lowering that put something else in the slot.
            return Err(Fault::fatal(format!(
                "Core\\Test::request expected a Core\\Http\\Method case, got {ordinal}"
            )));
        };
        let Some(target) = args[1].as_text() else {
            // Unreachable from source, exactly as the arm above is: the row's
            // second parameter is `CoreTy::Text`, so `E0401` refuses anything
            // that is not a string before any of this runs.
            return Err(Fault::fatal(
                "Core\\Test::request expected a string for its path".to_owned(),
            ));
        };
        let mut completion = match nvs_runtime::inproc::answer(
            ctx,
            Box::new(described(&verb, target, args)?.build()),
        ) {
            Ok(completion) => completion,
            Err(refusal) => return Err(Fault::thrown(format!(
                "Core\\Test::request could not run the request: {refusal}"
            ))),
        };
        // Nobody reads what the program's script frame returned — a response is
        // its status and its bytes — so the reference is discharged rather than
        // leaked (`Completion::discard_value`).
        completion.discard_value();
        // Spec § 15's default, applied here rather than left `null`: a program
        // that only echoed answered `200`, and making a test say so would be
        // making every test say so.
        let status = i64::from(completion.status.unwrap_or(200));
        Ok(crate::instance::build(
            &RESPONSE,
            [
                Value::int(status),
                Value::str(nvs_runtime::NvsStr::new(&completion.output)),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\Response::status(): uint` — the code the program declared.
    fn nvs_core_test_response_status(_ctx, args: [1]) {
        response_slot(args, STATUS_SLOT, "status")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\Response::body(): string` — the bytes the program wrote.
    fn nvs_core_test_response_body(_ctx, args: [1]) {
        response_slot(args, BODY_SLOT, "body")
    }
}

/// One [`RESPONSE`] slot, handed to the caller with a reference of its own.
///
/// `Core\Script\ExitReport`'s `slot_of` one class over, and the accounting is
/// the same: the slot's reference belongs to the receiver, and a value crossing
/// out of a member needs one that does not.
fn response_slot(args: &[Value], index: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &RESPONSE, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot's reference belongs to the receiver, which is live for \
                  the length of the call, and this value is being handed to the \
                  caller — which is exactly `Value::retain`'s obligation"
    )]
    // SAFETY: the receiver owns the slot's reference and outlives this call.
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertSame(T $actual, T $expected, {message?: string}): void`
    /// — § 4's identity row, which is [`identity::value_identical`] and nothing
    /// added to it.
    fn nvs_core_test_assert_same(ctx, args: [3]) {
        if identity::value_identical(args[0], args[1]) {
            return Ok(held(ctx, "assertSame"));
        }
        Err(failed(
            ctx,
            "assertSame",
            &format!(
                "`$actual` is {}, `$expected` is {}",
                shown(args[0]),
                shown(args[1])
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertEquals(T $actual, T $expected, {message?: string}): void`
    /// — § 4's value row: [`identity::value_identical`] everywhere except two
    /// objects, which are compared through `rule:classes/comparable`'s `Comparable::compareTo`.
    fn nvs_core_test_assert_equals(ctx, args: [3]) {
        if equals(ctx, args[0], args[1])? {
            return Ok(held(ctx, "assertEquals"));
        }
        Err(failed(
            ctx,
            "assertEquals",
            &format!(
                "`$actual` is {}, `$expected` is {}",
                shown(args[0]),
                shown(args[1])
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertEqualsDeep(T $actual, T $expected, {message?: string}): void`
    /// — § 4's structural walk, which reports **where** the two differ rather
    /// than only that they do.
    fn nvs_core_test_assert_equals_deep(ctx, args: [3]) {
        let Some(diff) = difference(args[0], args[1], 0, "", &mut Proven::new())? else {
            return Ok(held(ctx, "assertEqualsDeep"));
        };
        let at = if diff.path.is_empty() {
            "the value itself".to_owned()
        } else {
            format!("`$actual{}`", diff.path)
        };
        Err(failed(
            ctx,
            "assertEqualsDeep",
            &format!(
                "the two differ at {at}: `$actual` is {}, `$expected` is {}",
                diff.actual, diff.expected
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertTrue(bool $actual, {message?: string}): void` — § 4's
    /// predicate row, and the one member whose subject is a *declared* `bool`
    /// rather than a [`CoreTy::Var`].
    ///
    /// A `mixed` subject resolved through `rule:expressions/truthy-positions`'s truthy table would have
    /// been the PHPUnit reading, and it is the wrong one here: that ADR makes a
    /// **condition** the one place a value is tested without `as`, and an
    /// argument is not one. So `assertTrue($rows)` is refused where it is
    /// written, exactly as `if` would accept it and `bool $b = $rows;` would
    /// not, and a test meaning to assert a non-empty array says
    /// `Core\Arr::isEmpty($rows)` or writes the comparison out.
    fn nvs_core_test_assert_true(ctx, args: [2]) {
        // Unreachable from source, and the doc comment above says why the
        // parameter is spelled that way: it is a declared `bool` in `CLASS`,
        // so `assertTrue($rows)` over anything else is `E0401: expected bool,
        // found mixed` at the checker rather than a truthy conversion here.
        let actual = args[0].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertTrue expected {:?}, got tag {}",
                Tag::Bool,
                args[0].tag_byte()
            ))
        })?;
        if actual {
            return Ok(held(ctx, "assertTrue"));
        }
        Err(failed(ctx, "assertTrue", "`$actual` is false", args[1]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertNull(mixed $actual, {message?: string}): void` — § 4's
    /// second predicate row.
    ///
    /// The subject is `mixed` rather than a `?T`: `rule:types/conversion` makes `mixed`
    /// the one position that admits every type, and a `?T` parameter would
    /// refuse the `string` half of the very question this member asks about a
    /// union. What it costs is that a subject whose declared type cannot hold
    /// `null` at all still compiles — a mistake `rule:types/literal-types`'s literal types
    /// would have to be extended to `null` to catch, which is not this
    /// member's to decide.
    fn nvs_core_test_assert_null(ctx, args: [2]) {
        if matches!(args[0].tag(), None | Some(Tag::Null)) {
            return Ok(held(ctx, "assertNull"));
        }
        Err(failed(
            ctx,
            "assertNull",
            &format!("`$actual` is {}", shown(args[0])),
            args[1],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertCount(array<T> $actual, uint $expected, {message?: string}): void`
    /// — § 4's third predicate row, and the one whose subject row had to be
    /// decided rather than read off the ADR.
    ///
    /// The subject is an `array<T>` and nothing else, and the count is a
    /// `uint`, which is `Core\Arr::count`'s own signature: a length is a
    /// question the spec answers per domain — `Core\Str::length` counts
    /// characters and `Core\Bytes::length` counts bytes — so a union subject
    /// here would be a *fourth* answer to "how long is it", decided by a tag
    /// rather than by the member the author named. A `string` subject is
    /// therefore written `Core\Test::assertSame(Core\Str::length($s), 3)`,
    /// which says which length it meant.
    fn nvs_core_test_assert_count(ctx, args: [3]) {
        // Unreachable from source: `array<T>` in `CLASS`, so a `mixed`
        // subject is `E0401: expected array<mixed>, found mixed` and a
        // `?array<string>` is the same code over the union.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertCount expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for the length of this call"
        )]
        let count = unsafe { nvs_runtime::nvs_array_count(array) }.cast_unsigned();
        // Unreachable from source for the same reason as the subject above:
        // parameter 1 is `CoreTy::Uint`, so a `mixed` count is `E0401:
        // expected uint, found mixed` at the second argument.
        let expected = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertCount expected {:?}, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        if count == expected {
            return Ok(held(ctx, "assertCount"));
        }
        Err(failed(
            ctx,
            "assertCount",
            &format!("`$actual` holds {count} entries, `$expected` is {expected}"),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertContains(array<T> $actual, T $expected, {message?: string}): void`
    /// — § 4's membership row, whose subject and comparison are
    /// [`crate::arr`]'s `Core\Arr::contains` rather than a second reading of
    /// either.
    ///
    /// The subject is an `array<T>` and the needle is that same `T`, which is
    /// the signature `Core\Arr::contains` already carries, so
    /// `Core\Test::assertContains($xs, $x)` holds exactly when
    /// `Core\Arr::contains($xs, $x)` is `true` and the library asks membership
    /// once. What "is the needle" means is therefore
    /// [`identity::value_identical`] — `rule:expressions/equality-semantics`'s numeric row included, so
    /// `[1.0]` contains `1` — and not `rule:classes/comparable`'s `compareTo`: `assertEquals` is
    /// the member that names an object comparison, and a membership test whose
    /// comparison changed with the element type is the silent fallback 0013
    /// refused.
    ///
    /// A `string` subject is refused for `assertCount`'s reason: a substring is
    /// `Core\Str`'s question, asked as
    /// `Core\Test::assertTrue(Core\Str::contains($s, "x"))`, which says which
    /// containment was meant.
    fn nvs_core_test_assert_contains(ctx, args: [3]) {
        // Unreachable from source: `array<T>` in `CLASS`, so a `mixed` subject
        // is `E0401: expected array<mixed>, found mixed` and a `?array<string>`
        // is the same code over the union — `assertCount`'s subject exactly.
        let array = crate::arr::borrowed(args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertContains expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?);
        let mut from = 0usize;
        let mut count = 0usize;
        while let Some(slot) = array.next_slot(from) {
            from = slot + 1;
            count += 1;
            let value = array.value_at(slot).unwrap_or_else(Value::null);
            if identity::value_identical(value, args[1]) {
                return Ok(held(ctx, "assertContains"));
            }
        }
        Err(failed(
            ctx,
            "assertContains",
            &format!(
                "`$actual` holds {count} entries and none is `$expected`, which is {}",
                shown(args[1])
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertMatchesInline(mixed $actual, string $expected, {message?: string}):
    /// void` — `rule:testing/inline-snapshots`'s inline snapshot.
    ///
    /// The comparison is between two *renderings*, and only one of them is
    /// built here: `$actual` goes through [`crate::debug::rendered`], which is
    /// `Core\Debug::render`'s own text, and `$expected` is the literal the
    /// author wrote. Reusing that renderer rather than growing one is what
    /// makes a snapshot something a developer can produce by dumping the value
    /// — and it is also what carries `rule:errors/record-transformations`'s redaction into a snapshot,
    /// so a `secret` property renders as its placeholder and a snapshot cannot
    /// become the place a secret is committed (§ 14's own last sentence).
    ///
    /// The subject is `mixed` and not `T`: the whole point is that a value of
    /// any shape has *one* canonical text, and a type variable here would only
    /// name the type of a thing that is about to become a string.
    ///
    /// **The `--update` half of § 14 is split across three crates, and this is
    /// its runtime end.** Splicing the produced value back into the source
    /// needs the *span* of the `$expected` literal, and nothing at runtime
    /// holds one — a helper is called with a value, not with the expression
    /// that built it. So all this member contributes is the pair of texts
    /// ([`nvs_runtime::SnapshotMismatch`]); the span comes from
    /// `nvs_types::ExprTypeTable::inline_snapshots`, a compile-time row per
    /// *written* call carrying the literal's file, span and enclosing method,
    /// and `nvs-cli`'s runner joins the two by the expected text within the
    /// test that produced it. Searching the source for the literal instead was
    /// considered and refused: the workflow § 14 describes starts from an empty
    /// `""`, which occurs everywhere.
    ///
    /// The record is written on every mismatch, whatever the run was started
    /// with, for the reason the ledger entry beside it is: a flag may decide
    /// what is *written to disk* and may not decide what a run observed.
    fn nvs_core_test_assert_matches_inline(ctx, args: [3]) {
        // Unreachable from source: parameter 1 is `CoreTy::Text` in `CLASS`, so
        // a non-string expectation is `E0401` at the checker. Parameter 0 is
        // `mixed` and needs no check at all.
        let expected = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertMatchesInline expected {:?}, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?.to_owned();
        let produced = crate::debug::rendered(args[0]);
        if produced == expected {
            return Ok(held(ctx, "assertMatchesInline"));
        }
        ctx.record_snapshot_mismatch(expected.clone(), produced.clone());
        Err(failed(
            ctx,
            "assertMatchesInline",
            &format!("the rendering is {produced:?}, and the snapshot holds {expected:?}"),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertThrows(callable $body, string $expected, {message?: string}): void`
    /// — `rule:testing/assertions-are-typed`'s last row, and the one member whose subject is a
    /// **`callable`** and whose expectation is a class.
    ///
    /// The expectation is a `string` because that is what
    /// `rule:statements/static-is-a-member-modifier`'s
    /// `ParseError::class` folds to — a fully qualified name, compiled in as a
    /// constant — so the match is by name and no class value has to exist for
    /// a member to take one. What decides it is
    /// [`nvs_runtime::Ctx::pending_conforms_to`], whose own docs own the
    /// decision that an **ancestor** matches and the wiring that reading rests
    /// on: a context holding no exception class table has no ancestry to read,
    /// which that method asserts on rather than letting it arrive here as a
    /// non-match (`nvs_runtime::ctx::wiring`).
    ///
    /// The three edges are [`nvs_core_test_assert_does_not_throw`]'s, judged
    /// the other way round: a body that returned is the failure, a body that
    /// threw the wrong class is the other failure — reported with the class it
    /// *did* throw and that throw's message, which is the whole of what tells
    /// a reader whether the test or the code is wrong — and a `FATAL` or an
    /// `EXITED` is nobody's assertion to judge and propagates unchanged.
    ///
    /// The matched throw is consumed here for
    /// [`nvs_core_test_assert_does_not_throw`]'s reason, and so is the
    /// unmatched one: this member's verdict is what propagates on both edges,
    /// so a `catch` around it sees one class rather than two depending on
    /// which failure it was.
    ///
    /// Consuming the throw is not discharging a **ledger** entry, and a body
    /// whose throw was a failed assertion keeps its own: it really did fail,
    /// and `Core\Test::expectFailure` is the one member that discharges one.
    /// So `assertThrows(…, Core\Test\Failure::class)` holds *and* leaves the
    /// provoked failure recorded, which is why a test asserting that an
    /// assertion fails writes `expectFailure` instead.
    fn nvs_core_test_assert_throws(ctx, args: [3]) {
        // Unreachable from source: parameter 1 is `CoreTy::Str` in `CLASS`,
        // so the class name is `E0401: expected string, found mixed` at the
        // checker before this body sees it. `Core\Test\Failure::class` is the
        // spelling a call uses, and `::class` is a `string`.
        let expected = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertThrows expected {:?}, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?.to_owned();
        match nvs_runtime::call_closure(ctx, args[0], &[]) {
            Ok(result) => {
                #[expect(
                    unsafe_code,
                    reason = "`call_closure` hands back a value the caller owns, and \
                              this one is never handed on"
                )]
                unsafe {
                    result.release();
                }
                Err(failed(
                    ctx,
                    "assertThrows",
                    &format!("`$body` returned without throwing {expected}"),
                    args[2],
                ))
            }
            Err(Fault::Pending(nvs_runtime::THROWN)) => {
                if ctx.pending_conforms_to(&expected) {
                    drop(ctx.take_pending());
                    return Ok(held(ctx, "assertThrows"));
                }
                let thrown = ctx
                    .pending_class()
                    .unwrap_or_else(|| "an exception with no class".to_owned());
                let detail = match ctx.take_pending() {
                    Some(message) => {
                        format!("`$body` threw {thrown}, not {expected}: {message}")
                    }
                    None => format!("`$body` threw {thrown}, not {expected}"),
                };
                Err(failed(ctx, "assertThrows", &detail, args[2]))
            }
            Err(other) => Err(other),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertDoesNotThrow(callable $body, {message?: string}): void`
    /// — `rule:testing/runner-is-strict`'s member, and the first of the two whose subject is a
    /// **`callable`** rather than a value.
    ///
    /// It is § 20's own way out of the empty-ledger rule: a test whose whole
    /// claim is that some call completes has an assertion to write, so
    /// `nvs_cli::runner` can name this member rather than only the rule.
    ///
    /// Three edges, and only the middle one is this member's:
    ///
    /// * The body **returned**. Its value is released — a `callable`'s answer
    ///   is the caller's, and this member is not the one that wants it — and
    ///   the assertion held.
    /// * The body **threw**, which is [`nvs_runtime::THROWN`] and nothing else.
    ///   The pending throw is taken here, because the assertion's own verdict
    ///   is what propagates: leaving it set would replace this member's
    ///   `Core\Test\Failure` with the exception it is reporting, and a `catch`
    ///   around the assertion would then see the wrong class. Its message is
    ///   quoted into the detail, which is the whole of what the reader needs.
    /// * The body ended the **request** — a [`nvs_runtime::FATAL`] or an
    ///   [`nvs_runtime::EXITED`], and an internal [`Fault`] with them. None of
    ///   those is a throw a `catch` could see either (`rule:errors/escalation-ladder`), so none is
    ///   this member's to judge: it propagates unchanged.
    ///
    /// A **failed assertion** inside the body throws like anything else, so it
    /// fails this one too — and its own ledger entry stays, since it really
    /// did fail. `Core\Test::expectFailure` is the member that discharges one;
    /// this member never does.
    fn nvs_core_test_assert_does_not_throw(ctx, args: [2]) {
        match nvs_runtime::call_closure(ctx, args[0], &[]) {
            Ok(result) => {
                #[expect(
                    unsafe_code,
                    reason = "`call_closure` hands back a value the caller owns, and \
                              this one is never handed on"
                )]
                unsafe {
                    result.release();
                }
                Ok(held(ctx, "assertDoesNotThrow"))
            }
            Err(Fault::Pending(nvs_runtime::THROWN)) => {
                let detail = match ctx.take_pending() {
                    Some(message) => format!("`$body` threw: {message}"),
                    None => "`$body` threw".to_owned(),
                };
                Err(failed(ctx, "assertDoesNotThrow", &detail, args[1]))
            }
            Err(other) => Err(other),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::expectFailure(callable $body): void` — `rule:testing/failure-ledger`'s one
    /// greppable spelling for "this failure was on purpose".
    ///
    /// It runs `$body`, requires that an assertion inside it **failed**, and
    /// removes that entry from the ledger — which is the only way an entry ever
    /// leaves it. Everything about the shape follows from the ledger being the
    /// record and the throw being control flow:
    ///
    /// * The question asked is *what the ledger says*, not what class was
    ///   thrown. A body that caught its own failure and returned normally still
    ///   failed, and § 5's silently-passing test is precisely the program that
    ///   would otherwise sneak through here.
    /// * A body in which nothing failed is itself a failure, recorded and
    ///   raised through [`failed`] like any other — so wrapping *this* call in
    ///   a `catch` cannot erase it either.
    /// * Only the failures are discharged. A passing assertion inside `$body`
    ///   really ran, and § 20 counts it.
    ///
    /// The pending throw is taken where a failure was discharged, because that
    /// throw *is* the failure this member consumed. **Known gap:** a body that
    /// swallowed a failed assertion and then raised something else has its
    /// second throw consumed here too, this member having no way to tell the
    /// two apart from the ledger alone; naming the class would need the
    /// pending exception's descriptor, which is `nvs-runtime`'s and not a
    /// question a `Fault` answers.
    fn nvs_core_test_expect_failure(ctx, args: [1]) {
        let mark = ctx.assertion_count();
        let outcome = nvs_runtime::call_closure(ctx, args[0], &[]);
        let discharged = ctx.discharge_failures_from(mark);
        if let Ok(result) = outcome {
            #[expect(
                unsafe_code,
                reason = "`call_closure` hands back a value the caller owns, and \
                          this one is never handed on"
            )]
            unsafe {
                result.release();
            }
        } else if discharged == 0 {
            return outcome;
        } else {
            // The body's throw was the failure just discharged, so it is this
            // member's to consume rather than to propagate.
            drop(ctx.take_pending());
        }
        if discharged == 0 {
            return Err(failed(
                ctx,
                "expectFailure",
                "the callable ran without a failed assertion",
                Value::null(),
            ));
        }
        Ok(held(ctx, "expectFailure"))
    }
}

/// The [`Fault`] every failed assertion raises, with the `{message?: string}`
/// option in front of it where one was given.
///
/// A **catchable** throw rather than a [`Fault::fatal`]: `rule:testing/failure-ledger` makes the
/// ledger the record and the throw the control flow, and a test that means to
/// assert its subject throws has to be able to run one inside a `try` — which
/// is what `Core\Test::assertThrows` is written over.
///
/// The class is `Core\Test\Failure` — [`ThrownClass::TestFailure`], one row of
/// `nvs_hir::errors::TREE` like any other — rather than the bare
/// [`Fault::thrown`]'s `RuntimeError`, because § 5's own worked example catches
/// it **by name**: a composite assertion that meant to intercept a failed
/// assertion would otherwise have to catch every "the world said no" beside it.
/// This is the one throw site the whole surface funnels through, so naming the
/// class here is what names it for all three members.
///
/// It is also the one place a **failed** entry reaches § 5's ledger, and the
/// order matters: the entry is recorded before the [`Fault`] is handed back, so
/// there is no edge on which the throw exists and the record does not. That is
/// the whole of what makes the ledger something a `catch` cannot erase.
fn failed(ctx: &mut Ctx, member: &'static str, detail: &str, message: Value) -> Fault {
    let text = match message.as_text() {
        Some(given) if !given.is_empty() => {
            format!("{given}: Core\\Test::{member} failed: {detail}")
        }
        _ => format!("Core\\Test::{member} failed: {detail}"),
    };
    ctx.record_assertion(member, Some(text.clone()));
    Fault::thrown_as(ThrownClass::TestFailure, text)
}

/// [`failed`]'s other half: the entry an assertion that **held** leaves, and
/// the `void` it answers.
///
/// A passing assertion records too, because § 20's "a test that asserts nothing
/// fails" is a question about how many entries a test produced — a ledger of
/// failures alone cannot answer it. It allocates nothing: `member` is the
/// literal the member names itself with.
fn held(ctx: &mut Ctx, member: &'static str) -> Value {
    ctx.record_assertion(member, None);
    Value::null()
}

// ============================================================================
// The object rows — what `assertEquals` and `assertEqualsDeep` substitute
// ============================================================================

/// § 4's value comparison: `rule:expressions/equality-semantics`'s table, with the object row answered
/// by `rule:classes/comparable`'s
/// `compareTo` instead of by pointer identity.
///
/// # Errors
///
/// A catchable [`Fault::thrown`] naming `assertEqualsDeep` where both operands
/// are objects and `$actual`'s class declares no `compareTo` — the module doc's
/// object row states why that refusal is made here rather than at the call
/// site. Or the callee's own failure, propagated as [`Fault::Pending`] by
/// `nvs_runtime::call_method`.
fn equals(ctx: &mut Ctx, actual: Value, expected: Value) -> Result<bool, Fault> {
    if actual.tag() != Some(Tag::Object) || expected.tag() != Some(Tag::Object) {
        return Ok(identity::value_identical(actual, expected));
    }
    let what = "Core\\Test::assertEquals";
    let Some(answer) = nvs_runtime::call_method(ctx, actual, "compareTo", &[expected], what)?
    else {
        return Err(Fault::thrown(format!(
            "Core\\Test::assertEquals compares an object only through `Comparable`, and \
             {} declares no `compareTo` — write `Core\\Test::assertEqualsDeep` for a \
             structural comparison, or `Core\\Test::assertSame` for identity",
            shown(actual)
        )));
    };
    answer
        .as_int()
        .map(|ordering| ordering == 0)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertEquals expected an `int` from `compareTo`, got tag {}",
                answer.tag_byte()
            ))
        })
}

/// Where `actual` and `expected` first differ under § 4's structural walk, or
/// `None` when they agree everywhere.
///
/// The scalar, `string` and `bytes` rows are [`identity::value_identical`]'s,
/// so nothing about `rule:expressions/equality-semantics` is read a second time here; what this adds is
/// the two container rows and the object one, each of which the deep member
/// exists to descend rather than to answer by identity.
///
/// # Errors
///
/// A catchable [`Fault::thrown`] past [`MAX_DEPTH`] levels of containment.
/// An Novis array cannot contain itself (`rule:expressions/equality-semantics`), so the only structure
/// that can recur is an object graph, and a depth cap is what bounds it —
/// refusing to answer is the honest outcome, an assertion that quietly compared
/// half a graph being worse than one that says it could not.
fn difference(
    actual: Value,
    expected: Value,
    depth: usize,
    path: &str,
    proven: &mut Proven,
) -> Result<Option<Diff>, Fault> {
    if depth >= MAX_DEPTH {
        return Err(Fault::thrown(format!(
            "Core\\Test::assertEqualsDeep walked {MAX_DEPTH} levels of containment at \
             `$actual{path}` without reaching a scalar — compare a narrower subject, or \
             `Core\\Test::assertSame` if identity is what was meant"
        )));
    }
    match (actual.tag(), expected.tag()) {
        (Some(Tag::Array), Some(Tag::Array)) => {
            array_difference(actual, expected, depth, path, proven)
        }
        (Some(Tag::Object), Some(Tag::Object)) => {
            object_difference(actual, expected, depth, path, proven)
        }
        _ => Ok(leaf(actual, expected, path)),
    }
}

/// How deep [`difference`] descends before it refuses to answer.
const MAX_DEPTH: usize = 64;

/// The pairs of objects one [`difference`] walk has already found equal, by
/// address.
///
/// Two graphs that each reach one node along many paths — a node whose two
/// properties hold the same child, repeated down [`MAX_DEPTH`] levels — would
/// otherwise be walked once per path, which is exponential in the depth and
/// hangs the assertion. With this set each pair of objects is walked once. It
/// costs one entry per pair of objects found equal, for the length of one
/// assertion call. An address is a sound key here because both arguments own
/// their graphs for the whole walk and the walk runs no program code, so no
/// object is freed and no address is reused while the set is alive. Only a
/// finished comparison is recorded, so a cycle still reaches the depth cap.
type Proven = std::collections::HashSet<(usize, usize)>;

/// One difference: where it is, and what each side holds there.
struct Diff {
    /// The path from the subject, written as a program would subscript it —
    /// empty for the subject itself.
    path: String,
    /// What `$actual` holds there, rendered by [`shown`].
    actual: String,
    /// What `$expected` holds there.
    expected: String,
}

/// [`difference`]'s non-container rows, answered by identity.
fn leaf(actual: Value, expected: Value, path: &str) -> Option<Diff> {
    (!identity::value_identical(actual, expected)).then(|| Diff {
        path: path.to_owned(),
        actual: shown(actual),
        expected: shown(expected),
    })
}

/// `rule:expressions/equality-semantics`'s array row, descended rather than answered: the same length,
/// the same keys in the same order, and every value equal by this walk.
fn array_difference(
    actual: Value,
    expected: Value,
    depth: usize,
    path: &str,
    proven: &mut Proven,
) -> Result<Option<Diff>, Fault> {
    let (Some(left), Some(right)) = (entries(actual), entries(expected)) else {
        return Ok(leaf(actual, expected, path));
    };
    if left.len() != right.len() {
        return Ok(Some(Diff {
            path: path.to_owned(),
            actual: format!("an array of {}", left.len()),
            expected: format!("an array of {}", right.len()),
        }));
    }
    for ((key, mine), (theirs_key, theirs)) in left.into_iter().zip(right) {
        let at = format!("{path}[\"{key}\"]");
        if key != theirs_key {
            return Ok(Some(Diff {
                path: path.to_owned(),
                actual: format!("a key `{key}`"),
                expected: format!("a key `{theirs_key}`"),
            }));
        }
        if let Some(diff) = difference(mine, theirs, depth + 1, &at, proven)? {
            return Ok(Some(diff));
        }
    }
    Ok(None)
}

/// Every live `(key, value)` of an array, in iteration order — which `rule:types/arrays` makes part of the value, so this walk preserves it rather than keying a
/// map with it.
fn entries(value: Value) -> Option<Vec<(String, Value)>> {
    let array = crate::arr::borrowed(value.array_ptr()?);
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let Some(key) = array.key_at(slot) else {
            continue;
        };
        out.push((
            String::from_utf8_lossy(key.as_bytes()).into_owned(),
            array.value_at(slot).unwrap_or_else(Value::null),
        ));
    }
    Some(out)
}

/// § 4's walk of an object: the same class, then every declared property
/// compared by this walk.
///
/// The class is compared first and by **name**, because two objects of two
/// classes that happen to share a field set are not the same value and saying
/// so at the first differing property would name the wrong thing.
fn object_difference(
    actual: Value,
    expected: Value,
    depth: usize,
    path: &str,
    proven: &mut Proven,
) -> Result<Option<Diff>, Fault> {
    let (Some(left), Some(right)) = (actual.obj_ptr(), expected.obj_ptr()) else {
        return Ok(leaf(actual, expected, path));
    };
    // The one shortcut, and the reason a shared sub-object in both graphs does
    // not spend the depth budget twice: one allocation is equal to itself under
    // every row of this walk.
    if std::ptr::eq(left, right) || proven.contains(&(left as usize, right as usize)) {
        return Ok(None);
    }
    let pair = (left as usize, right as usize);
    let left = handle(left);
    let right = handle(right);
    if left.class_name() != right.class_name() {
        return Ok(Some(Diff {
            path: path.to_owned(),
            actual: format!("a `{}`", left.class_name()),
            expected: format!("a `{}`", right.class_name()),
        }));
    }
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by the unit's class table, which \
                  outlives every instance of the class it describes"
    )]
    let desc = unsafe { &*left.class() };
    for slot in 0..left.field_count().min(right.field_count()) {
        let name = desc
            .field_name(slot)
            .map_or_else(|| slot.to_string(), ToOwned::to_owned);
        let at = format!("{path}->{name}");
        let Some(diff) = difference(left.field(slot), right.field(slot), depth + 1, &at, proven)?
        else {
            continue;
        };
        // `rule:errors/record-transformations`'s redaction row, at the one place this module renders a
        // value a program declared `secret`: the comparison still descends —
        // whether two secrets agree is not itself a secret — but neither side is
        // quoted back into a message a build log keeps.
        return Ok(Some(if desc.field_is_secret(slot) {
            Diff {
                path: at,
                actual: REDACTED.to_owned(),
                expected: REDACTED.to_owned(),
            }
        } else {
            diff
        }));
    }
    proven.insert(pair);
    Ok(None)
}

/// What a `secret`-typed property renders as wherever this module would
/// otherwise quote its value.
const REDACTED: &str = "«redacted»";

/// A borrowed handle on a live object allocation, for reading its class and its
/// slots without taking a reference this walk would then have to release.
fn handle(ptr: *mut nvs_runtime::ObjHeader) -> std::mem::ManuallyDrop<nvs_runtime::NvsObj> {
    #[expect(
        unsafe_code,
        reason = "the caller's value owns a reference to a live allocation, so it \
                  is live for this borrow; the handle is never dropped, so the \
                  reference is not released twice"
    )]
    std::mem::ManuallyDrop::new(unsafe { nvs_runtime::NvsObj::from_raw(ptr) })
}

// ============================================================================
// Rendering a side of a failure
// ============================================================================

/// `value` as a failure message quotes it — short, and never the whole of a
/// container.
///
/// A message is written to a build log, so a subject the test happened to build
/// out of a large input would otherwise choose how many bytes that log gains —
/// the same bound [`crate::uuid`] puts on a rejected operand, for the same
/// reason. A container names its size rather than its contents, because
/// `assertEqualsDeep` has already said *where* the two differ and the leaf it
/// names is what a reader wants quoted.
fn shown(value: Value) -> String {
    match value.tag() {
        None | Some(Tag::Null) => "null".to_owned(),
        Some(Tag::Unset) => "never written".to_owned(),
        Some(Tag::Bool) => (value.as_bool() == Some(true)).to_string(),
        Some(Tag::Int) => value.as_int().unwrap_or(0).to_string(),
        Some(Tag::Uint) => value.as_uint().unwrap_or(0).to_string(),
        Some(Tag::Float) => value.as_float().unwrap_or(f64::NAN).to_string(),
        Some(Tag::Decimal) => value
            .as_decimal()
            .map_or_else(|| "0".to_owned(), |decimal| decimal.to_string()),
        Some(Tag::Str) => format!("\"{}\"", quoted(value.as_str_bytes().unwrap_or_default())),
        Some(Tag::Bytes) => format!("{} bytes", value.as_bytes().unwrap_or_default().len()),
        Some(Tag::Array) => entries(value).map_or_else(
            || "an array".to_owned(),
            |entries| format!("an array of {}", entries.len()),
        ),
        Some(Tag::Object) => value.obj_ptr().map_or_else(
            || "an object".to_owned(),
            |ptr| format!("a `{}`", handle(ptr).class_name()),
        ),
    }
}

/// A `string`'s first [`SHOWN_CHARS`] characters, with an ellipsis where
/// anything was dropped. Decoded lossily for [`crate::debug`]'s reason: a
/// `string` is UTF-8 by `rule:types/bytes`'s promise, so a run that is not is exactly
/// what a failing assertion is there to show.
fn quoted(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out: String = text.chars().take(SHOWN_CHARS).collect();
    if text.chars().nth(SHOWN_CHARS).is_some() {
        out.push('…');
    }
    out
}

/// How much of a quoted `string` [`quoted`] keeps.
const SHOWN_CHARS: usize = 64;

// ============================================================================
// A double's representation — the trampoline table and the descriptor cache
// ============================================================================

/// How many methods an interface a test doubles may declare: the length of
/// [`TRAMPOLINES`], and a bound on nothing else.
///
/// A generated table has to have a length, and this one is set well above the
/// interfaces a test doubles — this module's *a trampoline carries its slot in
/// its own identity* owns what it costs. [`descriptor_for`] is where an
/// interface past it is refused by name.
pub(crate) const METHOD_CEILING: usize = 32;

/// Slot 0 of a double: the calls it has answered, keyed by method name.
pub(crate) const RECORD_SLOT: usize = 0;

/// Slot 1: the real implementation a `partial` delegates to, or `null` for a
/// double that delegates to nothing.
pub(crate) const REAL_SLOT: usize = 1;

/// Slot `FIRST_METHOD_SLOT + i`: the closure answering method *i* of the
/// receiver's own method table, in the name order
/// [`nvs_runtime::ClassTable::set_methods`] sorts that table into.
pub(crate) const FIRST_METHOD_SLOT: usize = 2;

/// What every one of a double's field names starts with, and what a
/// trampoline drops to recover its method name.
///
/// No property name can carry it, so an erased `$double->now`
/// (`rule:types/erased-member-access`) finds nothing and the representation
/// stays unspellable.
const SIGIL: char = '$';

/// [`RECORD_SLOT`]'s field name.
const RECORD_FIELD: &str = "$calls";

/// [`REAL_SLOT`]'s field name.
const REAL_FIELD: &str = "$real";

/// One method a double answers, as [`descriptor_for`] needs it.
///
/// [`Self::arity`] and [`Self::params`] are the **closure's** own declared
/// shape, read off its slots by the caller — this module's *a double's
/// descriptor is built here* says why a row's shape is the body's rather than
/// the interface's, and an interface's descriptor could not answer it anyway:
/// `nvs_types::layout::ClassLayout::methods` keeps only bodied methods, so a
/// bodiless declaration has no row there to copy. For a method a `partial`
/// delegates they are `$real`'s row's, that row being the one that will
/// answer the call.
pub(crate) struct Answer {
    /// The method's name, without [`SIGIL`].
    pub(crate) name: String,
    /// How many parameters it declares, the receiver excluded —
    /// [`nvs_runtime::MethodRow::arity`].
    pub(crate) arity: u32,
    /// Which [`Tag`] each of those requires —
    /// [`nvs_runtime::MethodRow::param_tags`].
    pub(crate) params: u64,
    /// Whether the shape left this one to `$real` rather than answering it
    /// itself.
    pub(crate) delegated: bool,
}

/// The class a double of `interface` belongs to, defined on first use — one
/// per `(interface, real class, overridden names)`, which is this module's
/// *one descriptor per …* decision and what makes the table O(call sites).
///
/// `real` is the class a `partial` delegates to and `None` for a plain double.
/// `answers` is one entry per method the double publishes a row for; a method
/// the shape leaves unimplemented is not among them.
///
/// # Errors
///
/// A catchable [`ThrownClass::Logic`] naming both counts where the interface
/// declares more methods than [`METHOD_CEILING`].
pub(crate) fn descriptor_for(
    member: &str,
    interface: *const nvs_runtime::ClassDesc,
    real: Option<*const nvs_runtime::ClassDesc>,
    answers: &[Answer],
) -> Result<*const nvs_runtime::ClassDesc, Fault> {
    let mut doubles = doubles();
    let real = real.map(|desc| doubles.behind(desc));
    #[expect(
        unsafe_code,
        reason = "both descriptors belong to the compiled unit under test, \
                  which handed this call the first as argument 0 and the \
                  second as its receiver's class, so each is live for as long \
                  as that unit is — and `Doubles::behind` only ever answers \
                  the second or a class a key already held"
    )]
    let (name, behind) = unsafe {
        (
            (*interface).name(),
            real.map(|desc| (*desc).name().to_owned()),
        )
    };
    if answers.len() > METHOD_CEILING {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): `{name}` declares {} method(s), and one double stands in \
                 for at most {METHOD_CEILING}",
                answers.len()
            ),
        ));
    }

    // The field order **is** the row order: `set_methods` sorts its rows by
    // name, and slot `FIRST_METHOD_SLOT + i` has to hold method `i` of that
    // sorted table for a trampoline to find its own closure by its own slot.
    let mut ordered: Vec<&Answer> = answers.iter().collect();
    ordered.sort_by(|left, right| left.name.cmp(&right.name));

    let key = Key {
        interface: interface as usize,
        real: real.map_or(0, |desc| desc as usize),
        overridden: ordered
            .iter()
            .filter(|answer| !answer.delegated)
            .map(|answer| answer.name.clone())
            .collect(),
    };
    if let Some(id) = doubles.id_of(&key) {
        return Ok(doubles.table.desc(id));
    }

    let mut fields = Vec::with_capacity(FIRST_METHOD_SLOT + ordered.len());
    fields.push(RECORD_FIELD.to_owned());
    fields.push(REAL_FIELD.to_owned());
    let mut rows = Vec::with_capacity(ordered.len());
    for (slot, answer) in ordered.iter().enumerate() {
        fields.push(format!("{SIGIL}{}", answer.name));
        rows.push(nvs_runtime::MethodRow {
            name: answer.name.clone(),
            code: (TRAMPOLINES[slot] as *const ()).cast(),
            arity: answer.arity,
            param_tags: answer.params,
            // Nothing spelled these rows. The declaration a reader would take
            // a parameter's name or written type off is the interface's, and
            // `MethodRow`'s own docs make an empty vector exactly that answer.
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            // Not `native`: the row is reached by
            // `nvs_ir::ir::InstKind::CallVirtual`, which transfers its
            // arguments — this module's *the rows are not native*.
            native: false,
        });
    }
    let label = match behind {
        Some(behind) => format!(
            "{SIGIL}partial{{{name} by {behind}: {}}}",
            key.overridden.join(", ")
        ),
        None => format!("{SIGIL}double{{{name}}}"),
    };
    #[expect(
        unsafe_code,
        reason = "the parent is the interface's own descriptor, which belongs \
                  to the compiled unit under test — and `Core\\Test` is \
                  deleted from every build (`rule:testing/tests-never-reach-a-build`), \
                  so the unit in question is the one `nvs test` compiled and no \
                  double of it can outlive its descriptor"
    )]
    let id = unsafe {
        doubles
            .table
            .define_conforming(label, &fields, &[interface])
    };
    doubles.table.set_methods(id, rows);
    doubles.keys.push((key, id));
    Ok(doubles.table.desc(id))
}

/// A fresh double of `class`, as the [`Value`] a helper answers with: its
/// ledger empty, `$real` holding `real`, and each named closure in the field
/// [`descriptor_for`] laid out for it.
///
/// Placed **by name** rather than in the order they were read, so the sort
/// that decides a double's layout lives in [`descriptor_for`] alone and a
/// caller that reads a shape's fields in source order owes nothing. A method a
/// `partial` delegates is named by no closure here and keeps the `null` its
/// slot was born with, which is what [`answered`] reads to delegate it.
///
/// Takes over `real`'s reference and each closure's, exactly as
/// [`crate::instance::build`] does — this is that same object, built against a
/// descriptor from the table above rather than from a
/// [`crate::registry::CoreClass`].
///
/// # Panics
///
/// Panics naming the method if `class` has no field for one of `answers`: the
/// layout is this module's on both sides, so a mismatch is a paste error
/// rather than anything a program can cause.
pub(crate) fn double_of(
    class: *const nvs_runtime::ClassDesc,
    real: Value,
    answers: Vec<(String, Value)>,
) -> Value {
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by the `DOUBLES` table, which a \
                  `static` never drops, so it outlives every instance made \
                  from it — `NvsObj::new`'s whole safety obligation"
    )]
    let (desc, object) = unsafe { (&*class, nvs_runtime::NvsObj::new(class)) };
    object.set_field(RECORD_SLOT, Value::array(nvs_runtime::NvsArray::new()));
    object.set_field(REAL_SLOT, real);
    for (name, closure) in answers {
        let slot = desc
            .field_slot(&format!("{SIGIL}{name}"), FIRST_METHOD_SLOT)
            .unwrap_or_else(|| panic!("{} has no field answering `{name}`", desc.name()));
        object.set_field(slot, closure);
    }
    Value::object(object)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::double<T>(object $answers): T` — a shape of closures that
    /// **is** a `T` (`rule:testing/doubles`).
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument**,
    /// not values: `crate::registry::WRITTEN_CLASS_MEMBERS` puts this member on
    /// the roster whose helper is handed a `nvs_runtime::ClassDesc`, the
    /// `array<...>` flag and an inline shape's wire contract ahead of its
    /// declared parameters, and that roster's docs own why. So the arity here is
    /// three more than the registry row's, and `$answers` is argument 3.
    ///
    /// **The shape's own fields are the rows, and there is no other source for
    /// them**: an interface's descriptor carries no row for a method it merely
    /// declares. A method of `T` the shape leaves out therefore publishes
    /// nothing, and a call reaching it lands on the fallback a bodiless
    /// declaration already names rather than on anything unchecked — which is
    /// this module's last paragraph, and what the checker's own refusal
    /// replaces.
    fn nvs_core_test_double(_ctx, args: [4]) {
        let interface = written_interface(args[0], "double")?;
        let given = answers_of(args[3], "double")?;
        let class = descriptor_for("double", interface, None, &rows_of(&given))?;
        Ok(double_of(class, Value::null(), retained(given)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::partial<T>(T $real, object $answers): T` — the double that
    /// answers what it was given and delegates the rest (`rule:testing/doubles`).
    ///
    /// Arguments 0 to 2 are [`nvs_core_test_double`]'s three, so `$real` is
    /// argument 3 and `$answers` argument 4.
    ///
    /// **The delegated rows are `$real`'s**, for the reason that member's own
    /// docs give: the class behind the double is the only place the shape of a
    /// call it does not answer itself is written down. That is also why the
    /// ceiling can refuse a partial whose *real* class is fat while the
    /// interface is small — a row is published for every method behind it, and
    /// `descriptor_for`'s message names the count it found.
    fn nvs_core_test_partial(_ctx, args: [5]) {
        let interface = written_interface(args[0], "partial")?;
        let real = args[3];
        let (_, behind) = object_of(real, "partial")?;
        let given = answers_of(args[4], "partial")?;
        let mut rows = rows_of(&given);
        rows.extend(delegated_rows(behind, &given));
        let class = descriptor_for(
            "partial",
            interface,
            Some(std::ptr::from_ref(behind)),
            &rows,
        )?;
        #[expect(
            unsafe_code,
            reason = "this frame borrows `$real` as every helper borrows its \
                      arguments, and the double below takes over the reference \
                      this adds"
        )]
        unsafe {
            real.retain();
        }
        Ok(double_of(class, real, retained(given)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertCalled(object $double, method $method, {times?: uint,
    /// with?: array<mixed>, message?: string}): void` — the call record read
    /// after the exercise (`rule:testing/interaction-after-the-fact`).
    ///
    /// The options flatten as `rule:core-api/shape-rules` R2's bag does, so
    /// `$times` is argument 2, `$with` argument 3 and `$message` argument 4.
    ///
    /// **Nothing here holds a pointer into the record across a comparison.**
    /// `$with` is compared as [`equals`] compares, which reaches a `compareTo`
    /// the test itself wrote, and that body may call the double again — an
    /// append that grows the ledger would leave a held pointer dangling. Every
    /// read therefore re-derives the ledger from the receiver's own slot, which
    /// costs a lookup per argument compared and cannot be wrong.
    fn nvs_core_test_assert_called(ctx, args: [5]) {
        let member = "assertCalled";
        let receiver = double_receiver(args[0], member)?;
        let method = method_name(args[1], member)?;
        let mut matched = 0_usize;
        for call in 0..recorded_count(receiver, &method) {
            if arguments_match(ctx, receiver, &method, call, args[3])? {
                matched += 1;
            }
        }
        // A `times` of its own is the exact count, and no `times` is § 11's
        // plain reading of "it was called": once is enough and twice is not a
        // failure, because a test that cares how many times says so.
        let holds = match args[2].as_uint() {
            Some(times) => u64::try_from(matched).is_ok_and(|found| found == times),
            None => matched > 0,
        };
        if holds {
            return Ok(held(ctx, member));
        }
        let expected = match args[2].as_uint() {
            Some(times) => format!("{times} call(s)"),
            None => "at least one call".to_owned(),
        };
        let with = match args[3].array_ptr() {
            Some(want) => format!(" with {}", arguments_shown(want)),
            None => String::new()
        };
        Err(failed(
            ctx,
            member,
            &format!(
                "expected {expected} to `{method}`{with}, found {matched} — the record holds {}",
                record_shown(receiver, &method)
            ),
            args[4],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertNeverCalled(object $double, method $method,
    /// {message?: string}): void` — [`nvs_core_test_assert_called`]'s other
    /// half, over the same record.
    ///
    /// The ledger holds no key for a method that was never called ([`record`]),
    /// so the whole question is whether one is there; a method called and then
    /// somehow un-called is not a state the record has.
    fn nvs_core_test_assert_never_called(ctx, args: [3]) {
        let member = "assertNeverCalled";
        let receiver = double_receiver(args[0], member)?;
        let method = method_name(args[1], member)?;
        let calls = recorded_count(receiver, &method);
        if calls == 0 {
            return Ok(held(ctx, member));
        }
        Err(failed(
            ctx,
            member,
            &format!(
                "`{method}` was called {calls} time(s), the first as {}",
                call_shown(receiver, &method, 0)
            ),
            args[2],
        ))
    }
}

/// The double behind `value`, which is the only object carrying a record to
/// read.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not an object, and a catchable
/// [`ThrownClass::Logic`] for one that is not a double: the parameter is
/// `rule:types/object-top`'s erased form, so every instance satisfies it at the
/// call site and this is where the narrower question is asked.
fn double_receiver(value: Value, member: &str) -> Result<*mut nvs_runtime::ObjHeader, Fault> {
    let (object, desc) = object_of(value, member)?;
    if desc.field_name(RECORD_SLOT) != Some(RECORD_FIELD) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): `{}` is an ordinary object, and the record this reads is \
                 kept only by a double `{NAME}::double` or `{NAME}::partial` built",
                desc.name()
            ),
        ));
    }
    Ok(object)
}

/// The method name argument 1 carries.
///
/// # Errors
///
/// A [`Fault::fatal`] where the slot holds no text.
fn method_name(value: Value, member: &str) -> Result<String, Fault> {
    // Unreachable from source: the parameter is
    // `crate::registry::CoreTy::MethodRef`, which is admitted only as
    // `Class::method` written at the call site and folded there to that
    // method's own name, so what arrives is always a constant `string`.
    value.as_text().map(str::to_owned).ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: `{NAME}::{member}` was handed tag {} where a method reference was \
             declared",
            value.tag_byte()
        ))
    })
}

/// The calls recorded under `method`, or `None` for a method that was never
/// called — which [`record`] spells as no key at all.
fn recorded_calls(
    receiver: *mut nvs_runtime::ObjHeader,
    method: &str,
) -> Option<*mut nvs_runtime::ArrayHeader> {
    let ledger = crate::instance::slot(receiver, RECORD_SLOT).array_ptr()?;
    crate::arr::borrowed(ledger)
        .get(method.as_bytes())
        .and_then(|calls| calls.array_ptr())
}

/// How many calls the record holds under `method`.
fn recorded_count(receiver: *mut nvs_runtime::ObjHeader, method: &str) -> usize {
    recorded_calls(receiver, method).map_or(0, |calls| crate::arr::borrowed(calls).count())
}

/// The arguments of one recorded call, as the array [`record`] appended.
fn recorded_call(
    receiver: *mut nvs_runtime::ObjHeader,
    method: &str,
    call: usize,
) -> Option<*mut nvs_runtime::ArrayHeader> {
    let calls = recorded_calls(receiver, method)?;
    crate::arr::borrowed(calls)
        .get_index(i64::try_from(call).ok()?)
        .and_then(|entry| entry.array_ptr())
}

/// One argument of one recorded call, re-derived from the receiver for the
/// reason [`nvs_core_test_assert_called`]'s docs give.
fn recorded_arg(
    receiver: *mut nvs_runtime::ObjHeader,
    method: &str,
    call: usize,
    index: usize,
) -> Option<Value> {
    let entry = recorded_call(receiver, method, call)?;
    crate::arr::borrowed(entry).get_index(i64::try_from(index).ok()?)
}

/// Whether recorded call `call` was made with exactly the arguments `with`
/// names, compared as `Core\Test::assertEquals` compares them.
///
/// A `with` that was not given matches every call: the assertion is then about
/// the method's name alone, which is `rule:testing/interaction-after-the-fact`'s
/// default reading.
///
/// # Errors
///
/// Whatever [`equals`] raises — an object with no `compareTo` on either side,
/// or a `compareTo` of the test's own that threw.
fn arguments_match(
    ctx: &mut Ctx,
    receiver: *mut nvs_runtime::ObjHeader,
    method: &str,
    call: usize,
    with: Value,
) -> Result<bool, Fault> {
    let Some(want) = with.array_ptr() else {
        return Ok(true);
    };
    let count = crate::arr::borrowed(want).count();
    if recorded_call(receiver, method, call).map_or(0, |entry| crate::arr::borrowed(entry).count())
        != count
    {
        return Ok(false);
    }
    for index in 0..count {
        let Ok(at) = i64::try_from(index) else {
            return Ok(false);
        };
        let (Some(expected), Some(actual)) = (
            crate::arr::borrowed(want).get_index(at),
            recorded_arg(receiver, method, call, index),
        ) else {
            return Ok(false);
        };
        if !equals(ctx, actual, expected)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Every call the record holds under `method`, as a failure names them.
fn record_shown(receiver: *mut nvs_runtime::ObjHeader, method: &str) -> String {
    let count = recorded_count(receiver, method);
    if count == 0 {
        return format!("no call to `{method}`");
    }
    (0..count)
        .map(|call| call_shown(receiver, method, call))
        .collect::<Vec<String>>()
        .join(", ")
}

/// One recorded call, written as the call site wrote it.
fn call_shown(receiver: *mut nvs_runtime::ObjHeader, method: &str, call: usize) -> String {
    let arguments = recorded_call(receiver, method, call).map_or_else(String::new, arguments_shown);
    format!("{method}({arguments})")
}

/// One argument list — a recorded call's, or the `with` option's — with each
/// entry rendered as every other failure on this class renders a value.
fn arguments_shown(entry: *mut nvs_runtime::ArrayHeader) -> String {
    let entry = crate::arr::borrowed(entry);
    (0..entry.count())
        .map(|index| {
            i64::try_from(index)
                .ok()
                .and_then(|at| entry.get_index(at))
                .map_or_else(|| "?".to_owned(), shown)
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// The interface descriptor argument 0 carries.
///
/// # Errors
///
/// A [`Fault::fatal`] where the slot holds no descriptor.
fn written_interface(value: Value, member: &str) -> Result<*const nvs_runtime::ClassDesc, Fault> {
    // Unreachable from source, because argument 0 is not a program's value:
    // `crate::registry::WRITTEN_CLASS_MEMBERS` is what puts the resolved
    // descriptor there, and a call naming no type argument is `E0442` —
    // `takes 1 type argument(s)` — before any of this runs.
    value.as_class_desc().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: `{NAME}::{member}` was called with no interface in argument 0"
        ))
    })
}

/// The object in `value` and the class it is an instance of.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not an object, or is one with no
/// descriptor.
fn object_of(
    value: Value,
    member: &str,
) -> Result<(*mut nvs_runtime::ObjHeader, &'static nvs_runtime::ClassDesc), Fault> {
    #[expect(
        unsafe_code,
        reason = "this frame borrows the argument, so the allocation and its \
                  descriptor are both live for this read"
    )]
    let found = value.obj_ptr().and_then(|object| {
        unsafe { nvs_runtime::NvsObj::class_of(object).as_ref() }.map(|desc| (object, desc))
    });
    // Unreachable from source: both arguments read through here are declared —
    // `$real` as the written type itself and `$answers` as an `object` — so
    // `E0401` refuses anything that is not an instance before the call runs.
    found.ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: `{NAME}::{member}` was handed tag {} where an object was declared",
            value.tag_byte()
        ))
    })
}

/// One field of a `{name: fn …}` argument: the method it answers, and the
/// closure answering it **borrowed** from the shape that holds it.
type Given = (String, Value);

/// Every field of the shape in `value`, in the order the shape lays them out.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not an object.
fn answers_of(value: Value, member: &str) -> Result<Vec<Given>, Fault> {
    let (object, desc) = object_of(value, member)?;
    Ok((0..desc.field_count())
        .filter_map(|slot| {
            desc.field_name(slot)
                .map(|name| (name.to_owned(), crate::instance::slot(object, slot)))
        })
        .collect())
}

/// One [`Answer`] per field the shape gave, each carrying its own closure's
/// declared shape.
fn rows_of(given: &[Given]) -> Vec<Answer> {
    given
        .iter()
        .map(|(name, closure)| {
            let (arity, params) = closure_shape(*closure);
            Answer {
                name: name.clone(),
                arity,
                params,
                delegated: false,
            }
        })
        .collect()
}

/// One [`Answer`] per method of `real`'s class that `overridden` does not name
/// — the half a `partial` forwards.
///
/// A constructor is skipped: it is not a method an interface view can name, and
/// a row for it would spend a slot against the ceiling for nothing.
fn delegated_rows(real: &nvs_runtime::ClassDesc, overridden: &[Given]) -> Vec<Answer> {
    (0..real.method_count())
        .filter_map(|index| real.method_at(index))
        .filter(|row| {
            row.name != nvs_runtime::CONSTRUCTOR
                && !overridden.iter().any(|(name, _)| *name == row.name)
        })
        .map(|row| Answer {
            name: row.name.clone(),
            arity: row.arity,
            params: row.param_tags,
            delegated: true,
        })
        .collect()
}

/// What `closure` declares, as a [`nvs_runtime::MethodRow`] carries it.
///
/// `(0, 0)` for a value that is not a closure at all, which until the
/// checker's refusals land is a shape field it still admits: the row published
/// for one answers through `nvs_runtime::call_closure`, whose own refusal names
/// the value, rather than being read here as a shape it does not have.
fn closure_shape(closure: Value) -> (u32, u64) {
    let Some(object) = closure.obj_ptr() else {
        return (0, 0);
    };
    #[expect(
        unsafe_code,
        reason = "this frame borrows the shape holding this slot, so the \
                  allocation and its descriptor are live — and the two slots \
                  below are read only once the class says it is a closure's, \
                  which is what puts them in range"
    )]
    let closure_class = unsafe { nvs_runtime::NvsObj::class_of(object).as_ref() }
        .is_some_and(nvs_runtime::ClassDesc::is_closure);
    if !closure_class {
        return (0, 0);
    }
    let arity = crate::instance::slot(object, nvs_runtime::CLOSURE_ARITY_SLOT)
        .as_int()
        .unwrap_or(0);
    // The sixteenth nibble sits in the sign bit; the slot holds the same 64
    // bits either way, and only the nibbles are ever read.
    let tags = crate::instance::slot(object, nvs_runtime::CLOSURE_PARAM_TAGS_SLOT)
        .as_int()
        .unwrap_or(0);
    (
        u32::try_from(arity).unwrap_or(0),
        u64::from_ne_bytes(tags.to_ne_bytes()),
    )
}

/// `given`, with a reference taken on each closure for [`double_of`] to hand
/// over to the double's own slot.
fn retained(given: Vec<Given>) -> Vec<Given> {
    #[expect(
        unsafe_code,
        reason = "each value is a slot of the shape argument, which this frame \
                  borrows for the length of the call, and the double takes over \
                  the reference this adds"
    )]
    unsafe {
        for (_, closure) in &given {
            closure.retain();
        }
    }
    given
}

/// The process's double classes, written once per call site and read on every
/// double built from one.
///
/// A [`Mutex`](std::sync::Mutex) rather than [`crate::instance`]'s
/// `OnceLock<&'static ClassTable>`, because unlike that table this one is not
/// complete before it is first read: a call site becomes known the first time
/// it runs. Handing addresses out of a table that still grows is safe because
/// [`nvs_runtime::ClassTable`] boxes every descriptor it holds, so a later
/// class leaves an earlier one's address exactly where it was, and a `static`
/// is never dropped.
static DOUBLES: std::sync::OnceLock<std::sync::Mutex<Doubles>> = std::sync::OnceLock::new();

/// [`DOUBLES`], locked.
///
/// A lock an earlier panic poisoned is taken anyway: what it guards is only
/// ever pushed to, so the worst an interrupted build left behind is a class no
/// key names, and refusing every later double over it would end a whole run
/// for one test's bug.
fn doubles() -> std::sync::MutexGuard<'static, Doubles> {
    DOUBLES
        .get_or_init(|| {
            std::sync::Mutex::new(Doubles {
                table: nvs_runtime::ClassTable::new(),
                keys: Vec::new(),
            })
        })
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// [`DOUBLES`]'s contents: the classes, and what each one was keyed by.
struct Doubles {
    /// The one table every double's class is defined in.
    table: nvs_runtime::ClassTable,
    /// One entry per class in [`Self::table`], in definition order.
    keys: Vec<(Key, nvs_runtime::ClassId)>,
}

impl Doubles {
    /// The class already defined for `key`, if this call site has run before.
    ///
    /// A linear scan over a list that is O(call sites): a program's whole test
    /// suite writes a handful of them, and a sorted index would be a second
    /// structure to keep in step with the table beside it.
    fn id_of(&self, key: &Key) -> Option<nvs_runtime::ClassId> {
        self.keys
            .iter()
            .find(|(held, _)| held == key)
            .map(|(_, id)| *id)
    }

    /// The class a partial over an instance of `real` is keyed and named by:
    /// the class behind `real` when `real` is itself a partial's class, and
    /// `real` otherwise.
    ///
    /// This is what keeps the table O(call sites) when a partial wraps a
    /// partial. Keyed by the inner partial's own class, each wrap at one call
    /// site would define a new class, and each label would contain the one
    /// before it. The rows agree either way: a partial publishes a row for
    /// every method of the class behind it, so the rows it delegates are the
    /// rows that class delegates, and the key's overridden names are the rest.
    fn behind(&self, real: *const nvs_runtime::ClassDesc) -> *const nvs_runtime::ClassDesc {
        self.keys
            .iter()
            .find(|(key, id)| key.real != 0 && std::ptr::eq(self.table.desc(*id), real))
            .map_or(real, |(key, _)| std::ptr::with_exposed_provenance(key.real))
    }
}

/// What makes two doubles one class — the triple this module's *one descriptor
/// per `(interface, real class, overridden names)`* names.
///
/// Each descriptor is held as the **address** it is rather than as a pointer,
/// which is the same identity and is plain `Send` data, so nothing has to be
/// said about the table beside it. [`Self::overridden`] is kept in the sorted
/// order [`descriptor_for`] publishes its rows in, so two call sites writing
/// one interface's methods in two orders are one class.
#[derive(PartialEq, Eq)]
struct Key {
    /// The interface the double stands in for.
    interface: usize,
    /// The class a `partial` delegates to, or `0` for a plain double.
    real: usize,
    /// The names the shape answered itself.
    overridden: Vec<String>,
}

/// One native entry per slot: row *i* of a double's method table is entry *i*
/// of this, which is the whole of how a trampoline knows which method it is —
/// this module's *a trampoline carries its slot in its own identity*.
///
/// One generic function and a list of its instantiations rather than
/// [`nvs_runtime::nvs_helper`] thirty-two times: a trampoline's arity is its
/// own row's and is not known until the receiver has been read, where that
/// macro takes a literal. None of these is a symbol anything resolves by name
/// — a row carries the address — so none is `#[unsafe(no_mangle)]` and
/// [`address`] has no arm for them.
const TRAMPOLINES: [nvs_runtime::NvsFn; METHOD_CEILING] = [
    trampoline::<0>,
    trampoline::<1>,
    trampoline::<2>,
    trampoline::<3>,
    trampoline::<4>,
    trampoline::<5>,
    trampoline::<6>,
    trampoline::<7>,
    trampoline::<8>,
    trampoline::<9>,
    trampoline::<10>,
    trampoline::<11>,
    trampoline::<12>,
    trampoline::<13>,
    trampoline::<14>,
    trampoline::<15>,
    trampoline::<16>,
    trampoline::<17>,
    trampoline::<18>,
    trampoline::<19>,
    trampoline::<20>,
    trampoline::<21>,
    trampoline::<22>,
    trampoline::<23>,
    trampoline::<24>,
    trampoline::<25>,
    trampoline::<26>,
    trampoline::<27>,
    trampoline::<28>,
    trampoline::<29>,
    trampoline::<30>,
    trampoline::<31>,
];

/// The native entry standing in for a double's method at slot `SLOT`.
///
/// # Safety
///
/// `ctx`, `args` and `out` are `rule:errors/propagation`'s three pointers, and
/// `args` must point at the receiver followed by as many values as that
/// receiver's row at `SLOT` declares — which is what
/// `nvs_ir::ir::InstKind::CallVirtual` writes for the row this address came
/// out of.
#[expect(
    unsafe_code,
    reason = "a `MethodRow`'s `code` is a bare address carrying the compiled \
              method ABI, so a trampoline's signature is that ABI and its \
              pointer contract cannot be expressed in the type"
)]
unsafe extern "C" fn trampoline<const SLOT: usize>(
    ctx: *mut Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    // `run_helper` is told how many slots to read before the body runs, and a
    // trampoline's count is its own row's — so the receiver, which is slot 0
    // of every method call, is read here to find it.
    let receiver = unsafe { *args };
    let arity = answered_arity(receiver, SLOT);
    unsafe {
        nvs_runtime::run_helper(ctx, args, arity + 1, out, |ctx, args| {
            answer(SLOT, ctx, args)
        })
    }
}

/// How many parameters the row at `slot` of `receiver`'s class declares — the
/// count past the receiver that [`trampoline`] was handed.
///
/// `0` for a receiver that is not an object, or whose class has no such row,
/// which leaves [`answer`] the one slot it needs in order to report either.
fn answered_arity(receiver: Value, slot: usize) -> usize {
    let Some(object) = receiver.obj_ptr() else {
        return 0;
    };
    #[expect(
        unsafe_code,
        reason = "the caller transferred a reference to this object, so the \
                  allocation and its descriptor are both live for this read"
    )]
    let row = unsafe { nvs_runtime::NvsObj::class_of(object).as_ref() }
        .and_then(|desc| desc.method_at(slot));
    row.map_or(0, |row| usize::try_from(row.arity).unwrap_or(0))
}

/// A double's method at `slot`, answered: the call recorded, then the closure
/// standing in for it run — or `$real`'s own method where a `partial` left
/// this one to it.
fn answer(slot: usize, ctx: &mut Ctx, args: &[Value]) -> nvs_runtime::HelperResult {
    let outcome = answered(slot, ctx, args);
    // `nvs_ir::ir::InstKind::CallVirtual` transferred every slot, receiver
    // included, exactly as it does to the compiled method this row stands in
    // for — so this frame owes each one a release on every edge, which is
    // this module's *the rows are not native*. Everything the body passed on
    // was borrowed: `nvs_runtime::call_closure` and
    // `nvs_runtime::call_method` each retain what they pass.
    #[expect(
        unsafe_code,
        reason = "this frame holds the one reference the call site transferred \
                  for each slot, and nothing below it took that reference over"
    )]
    unsafe {
        for value in args {
            value.release();
        }
    }
    outcome
}

/// [`answer`]'s body, with the releases left to it.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not a double of this module's own
/// making, and the callee's own failure — propagated as [`Fault::Pending`] —
/// for a closure or a delegate that threw.
fn answered(slot: usize, ctx: &mut Ctx, args: &[Value]) -> nvs_runtime::HelperResult {
    // Unreachable from source: the address of this row was read off the
    // receiver's own descriptor, so `nvs_ir::ir::InstKind::CallVirtual` put an
    // object in slot 0 — a call on anything else resolves to no row at all.
    let receiver = args[0].obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{NAME}` double's slot {slot} was called on tag {}",
            args[0].tag_byte()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the call site transferred a reference to the receiver, so \
                  the allocation and its descriptor are both live for this \
                  read"
    )]
    // Unreachable from source for the same reason: an allocation reached
    // through a row of its own class has that class.
    let desc = unsafe { nvs_runtime::NvsObj::class_of(receiver).as_ref() }.ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{NAME}` double's slot {slot} was called on an object with no \
             class descriptor"
        ))
    })?;
    // Unreachable from source: `descriptor_for` publishes the fields and the
    // rows in one loop, so a row at this slot has a field at it.
    let method = desc
        .field_name(FIRST_METHOD_SLOT + slot)
        .and_then(|field| field.strip_prefix(SIGIL))
        .ok_or_else(|| {
            Fault::fatal(format!(
                "internal error: `{}` has no double field at slot {slot}",
                desc.name()
            ))
        })?;
    // Before the call and not after it, so a closure that throws is still a
    // call the test can assert was made.
    record(receiver, method, &args[1..]);
    let answering = crate::instance::slot(receiver, FIRST_METHOD_SLOT + slot);
    if answering.obj_ptr().is_none() {
        return delegated(ctx, receiver, method, &args[1..]);
    }
    nvs_runtime::call_closure(ctx, answering, &args[1..])
}

/// What a `partial` does with a method its shape did not override: the same
/// call on `$real`, which is `rule:classes/delegation-by-field`'s forward
/// reached without a class to declare it on.
///
/// # Errors
///
/// A [`Fault::fatal`] where there is nothing behind the double to delegate to,
/// which is [`descriptor_for`] having published a row for a method no closure
/// and no real implementation answers. The delegate's own failure otherwise.
fn delegated(
    ctx: &mut Ctx,
    receiver: *mut nvs_runtime::ObjHeader,
    method: &str,
    args: &[Value],
) -> nvs_runtime::HelperResult {
    // `rule:errors/on-limit`'s soft address, compared once per forward. A
    // partial whose real object is another partial forwards natively, and a
    // leaf method at the bottom of the chain runs no check of its own, so
    // without this compare a chain of partials deep enough runs past the
    // stack's floor and takes the process down with it. The address of a local
    // is this frame's stack pointer to within the frame.
    let here = 0_u8;
    if (&raw const here).addr() < ctx.stack_bounds().0 {
        return Err(Fault::thrown_as(
            ThrownClass::Recursion,
            "the call stack is too deep",
        ));
    }
    let real = crate::instance::slot(receiver, REAL_SLOT);
    // Unreachable from source: `double` publishes a row per field of the shape
    // it was given and `partial` one per method of `$real` besides, so a row
    // with no closure belongs to a double that has a `$real`.
    if real.obj_ptr().is_none() {
        return Err(Fault::fatal(format!(
            "internal error: a `{NAME}` double answers `{method}` with neither a closure nor a \
             real implementation"
        )));
    }
    // Unreachable from source, for the second half of the same reason: the row
    // this is delegating was copied from `$real`'s own method table.
    nvs_runtime::call_method(ctx, real, method, args, "a `Core\\Test` partial")?.ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: a `{NAME}` partial delegates `{method}`, and the class behind it \
             declares no such method"
        ))
    })
}

/// Records one call under `method` on `receiver`'s [`RECORD_SLOT`] ledger —
/// `rule:testing/interaction-after-the-fact`'s record, which the assertion
/// reads afterwards.
///
/// The ledger is keyed by method name and each key holds one list per call in
/// call order, each list holding that call's arguments. A method that was
/// never called therefore has no key at all, rather than an empty list a
/// reader would have to tell from one.
///
/// **Neither write separates.** The double's own slot is the ledger's only
/// owner and the ledger is each list's, so every append lands in place; what
/// the recording costs a call is one entry per argument and the reference each
/// of them takes.
fn record(receiver: *mut nvs_runtime::ObjHeader, method: &str, args: &[Value]) {
    let Some(ledger) = crate::instance::slot(receiver, RECORD_SLOT).array_ptr() else {
        return;
    };
    let mut entry = nvs_runtime::NvsArray::new();
    for value in args {
        #[expect(
            unsafe_code,
            reason = "this frame holds the reference the call site transferred \
                      for each slot and passes it on borrowed, so the entry \
                      below needs one of its own"
        )]
        unsafe {
            value.retain();
        }
        entry.append(*value);
    }
    let call = Value::array(entry);
    let mut ledger = crate::arr::borrowed(ledger);
    match ledger.get(method.as_bytes()).and_then(Value::array_ptr) {
        Some(calls) => {
            let mut calls = crate::arr::borrowed(calls);
            calls.append(call);
        }
        None => {
            let mut calls = nvs_runtime::NvsArray::new();
            calls.append(call);
            ledger.set(
                nvs_runtime::NvsStr::new(method.as_bytes()),
                Value::array(calls),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// § 4's three equality members — the rows that bind both subjects to one
    /// type variable, which the predicate ones beside them deliberately do
    /// not.
    fn equality_members() -> impl Iterator<Item = &'static CoreMethod> {
        CLASS.methods.iter().filter(|method| {
            matches!(
                method.name,
                "assertSame" | "assertEquals" | "assertEqualsDeep"
            )
        })
    }

    /// Every assertion — every row but the four that are not one: § 5's
    /// `expectFailure`, which takes a body rather than a subject; § 12's
    /// `advance`, which asserts nothing at all and is the fixed clock's
    /// mutator; `rule:tooling/a-prompt-is-a-core-member`'s `scriptAnswers`, which is the same kind of
    /// thing as `advance` — a test declaring the world its subject runs in,
    /// here the answers its prompts read; and § 18's two, `request` and
    /// `serverUrl`, which are the subject rather than a claim about one and are
    /// the only rows that answer with a value.
    ///
    /// Named rather than derived, so that adding a member to this class has to
    /// answer "is this an assertion?" here instead of quietly joining or
    /// quietly escaping § 4's shape rules.
    fn asserting_members() -> impl Iterator<Item = &'static CoreMethod> {
        CLASS.methods.iter().filter(|method| {
            !matches!(
                method.name,
                "expectFailure"
                    | "advance"
                    | "scriptAnswers"
                    | "request"
                    | "serverUrl"
                    | "double"
                    | "partial"
                    | "answerHttp"
                    | "tlsSession"
                    | "sentHttp"
                    | "answerSocket"
                    | "sentSocket"
            )
        })
    }

    /// § 4's order, which is the opposite of the one every migrated test suite
    /// was written in — so it is asserted rather than left to a reading of the
    /// rows above.
    #[test]
    fn every_assertion_is_subject_first_and_generic() {
        for method in equality_members() {
            assert_eq!(
                method.params.len(),
                3,
                "`{}` should take `$actual`, `$expected` and one options bag",
                method.name
            );
            assert!(
                matches!(method.params[0], CoreTy::Var("T"))
                    && matches!(method.params[1], CoreTy::Var("T")),
                "`{}` should bind both subjects to one type variable",
                method.name
            );
            assert!(
                matches!(method.return_ty, CoreTy::Void),
                "`{}` should answer nothing — a failure is a throw",
                method.name
            );
        }
    }

    /// § 4's example writes the three predicate members with no signature, so
    /// what each subject admits is this module's decision and is asserted here
    /// rather than left to a reading of the rows above — the module docs carry
    /// why each is the type it is.
    #[test]
    fn every_predicate_declares_the_one_subject_its_question_admits() {
        for method in asserting_members() {
            let admitted = match method.name {
                // A `bool`, not a `mixed` resolved through `rule:expressions/truthy-positions`'s truthy
                // table: an argument is not a condition.
                "assertTrue" => vec![CoreTy::Bool],
                // `rule:types/conversion`'s one position that admits every type.
                "assertNull" => vec![CoreTy::Mixed],
                // `Core\Arr::count`'s own signature, which is where a length
                // is answered for this domain.
                "assertCount" => vec![CoreTy::Array(&CoreTy::Var("T")), CoreTy::Uint],
                _ => continue,
            };
            let written = &method.params[..method.params.len() - 1];
            assert_eq!(
                format!("{written:?}"),
                format!("{admitted:?}"),
                "`{}` should take the subject its question admits",
                method.name
            );
        }
    }

    /// Every row's symbol has an address here, which is what
    /// [`crate::symbols`]' own sweep asserts across the whole registry — held
    /// once more locally so a member added above fails here rather than in a
    /// crate-wide test that names no module.
    #[test]
    fn every_row_names_a_symbol_this_module_claims() {
        for method in CLASS.methods {
            assert!(
                address(method.symbol).is_some(),
                "`{}` names `{}`, which this module does not claim",
                method.name,
                method.symbol
            );
        }
    }

    /// The option § 4 writes on every assertion, and the reason it defaults to
    /// `null` rather than to an empty `string`.
    ///
    /// A member may declare options of its own ahead of it — `assertCalled`'s
    /// `times` and `with` say *which* call is being asserted about — so what is
    /// swept is that the message is there, typed, absent by default, and
    /// **last**, which is where a reader of any row on this class finds it.
    #[test]
    fn every_assertion_carries_a_message_option_that_defaults_to_absent() {
        for method in asserting_members() {
            let last = method
                .params
                .last()
                .unwrap_or_else(|| panic!("`{}` should take a subject", method.name));
            let CoreTy::Options(bag) = last else {
                panic!(
                    "`{}`'s last parameter should be an options bag",
                    method.name
                );
            };
            let message = bag
                .last()
                .unwrap_or_else(|| panic!("`{}`'s bag should hold a message", method.name));
            assert_eq!(message.name, "message");
            assert!(matches!(message.ty, CoreTy::Text(Qual::Neutral)));
            assert!(matches!(message.default, Const::Null));
        }
    }

    /// § 12's own row, which is not an assertion at all — so it is here rather
    /// than inside [`asserting_members`]'s sweep, and this is what says its
    /// shape is deliberate and not an omission.
    #[test]
    fn advance_takes_one_duration_and_answers_nothing() {
        let advance = CLASS
            .methods
            .iter()
            .find(|method| method.name == "advance")
            .expect("§ 12's mutator is registered");
        assert_eq!(advance.names, &["by"]);
        // No `{message?:}` bag: § 4's option labels the two sides of a
        // comparison, and there is no comparison here to label.
        assert!(
            matches!(advance.params, [CoreTy::Instance(name)] if *name == crate::time::DURATION_NAME)
        );
        assert!(matches!(advance.return_ty, CoreTy::Void));
    }

    /// The far end of the fixed clock, asserted on both sides: a move onto the
    /// last instant an `Instant` can hold is taken, a move one nanosecond past
    /// it throws and leaves the clock where it was. The largest `Duration`,
    /// repeated, meets the same end in both directions — the path that used to
    /// reach `jiff`'s unchecked constructor, which `time::instant_at_nanos`'s
    /// doc comment explains.
    // covers: Core\Test::advance
    #[test]
    fn advance_stops_at_the_last_instant_and_keeps_the_clock_it_refused_to_move() {
        const NANOS_PER_SECOND: i128 = 1_000_000_000;
        // `9999-12-30T22:00:00.999999999Z` and `-9999-01-02T01:59:59Z`.
        const LAST: i128 = 253_402_207_200 * NANOS_PER_SECOND + 999_999_999;
        const FIRST: i128 = -377_705_023_201 * NANOS_PER_SECOND;
        fn step(ctx: &mut Ctx, nanos: i64) -> bool {
            let by = crate::time::duration_of(nanos);
            let answered = nvs_runtime::call(nvs_core_test_advance, ctx, &[by]);
            dropped(by);
            match answered {
                Ok(null) => {
                    dropped(null);
                    true
                }
                Err(_) => {
                    let why = ctx.take_pending().expect("a refused move says why");
                    assert!(
                        why.contains("outside the range an `Instant` can hold"),
                        "{why}"
                    );
                    false
                }
            }
        }

        let mut ctx = Ctx::buffered();
        ctx.set_fixed_clock(LAST - 1);
        assert!(
            step(&mut ctx, 1),
            "the last instant is one a clock can show"
        );
        assert_eq!(ctx.fixed_clock(), Some(LAST));
        assert!(!step(&mut ctx, 1), "one nanosecond past it is not");
        assert_eq!(
            ctx.fixed_clock(),
            Some(LAST),
            "a refused move leaves the clock"
        );

        for (direction, end) in [(i64::MAX, LAST), (-i64::MAX, FIRST)] {
            ctx.set_fixed_clock(0);
            let mut moves: i128 = 0;
            while step(&mut ctx, direction) {
                moves += 1;
            }
            let reached = moves * i128::from(direction);
            assert_eq!(
                moves,
                end / i128::from(direction),
                "every move that fits is taken"
            );
            assert_eq!(
                ctx.fixed_clock(),
                Some(reached),
                "and the one that does not fit is not"
            );
        }
    }

    /// `answerHttp`'s status, asserted on both sides: the floor and the ceiling
    /// are registered, one past either throws. An answer that throws adds no
    /// row, so refused answers alone leave the test on the network rather than
    /// arming a table with nothing in it — and an answer naming both `json` and
    /// `body` is refused the same way.
    // covers: Core\Test::answerHttp
    #[test]
    fn answer_http_takes_every_status_a_status_line_carries_and_a_refused_answer_arms_nothing() {
        fn answer(ctx: &mut Ctx, url: &str, status: u64, json: Value) -> Result<(), String> {
            let args = [
                Value::str(nvs_runtime::NvsStr::new(url.as_bytes())),
                Value::uint(status),
                json,
                Value::str(nvs_runtime::NvsStr::new(b"ok")),
                Value::array(nvs_runtime::NvsArray::new()),
                Value::null(),
            ];
            let answered = nvs_runtime::call(nvs_core_test_answer_http, ctx, &args);
            for arg in args {
                dropped(arg);
            }
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a refused answer says why")
                    .into_owned()),
            }
        }
        const URL: &str = "https://api.example.com/";

        let mut ctx = Ctx::buffered();
        for status in [STATUS_FLOOR - 1, STATUS_CEILING + 1, u64::MAX] {
            let why = answer(&mut ctx, URL, status, Value::unset())
                .expect_err("a status line carries three digits");
            assert!(
                why.contains(&format!("between {STATUS_FLOOR} and {STATUS_CEILING}")),
                "{why}"
            );
        }
        let why = answer(&mut ctx, URL, 200, Value::uint(1)).expect_err("one body, two spellings");
        assert!(why.contains("two spellings of one body"), "{why}");
        assert!(
            !ctx.faked_http_mut().is_armed(),
            "a refused answer registers no row"
        );

        for status in [STATUS_FLOOR, STATUS_CEILING] {
            let url = format!("{URL}{status}");
            answer(&mut ctx, &url, status, Value::unset()).expect("a status a line can carry");
            let row = ctx
                .faked_http_mut()
                .answer_for(&url)
                .expect("the row just registered");
            assert_eq!(u64::from(row.status), status);
            assert_eq!(row.body, b"ok");
        }
        assert!(ctx.faked_http_mut().is_armed());
    }

    /// `answerSocket`'s frames keep the kind each element was written as and
    /// the order they were written in, and the protocol option reaches the
    /// peer. A peer with no frames still arms the table, and the selection
    /// agrees with `answerHttp`'s: exact first, then the longest prefix.
    // covers: Core\Test::answerSocket
    #[test]
    fn answer_socket_keeps_each_frame_kind_in_order_and_selects_exact_before_the_longest_prefix() {
        use nvs_runtime::SocketFrame::{Bytes, Text};

        fn peer(
            ctx: &mut Ctx,
            url: &str,
            frames: &[nvs_runtime::SocketFrame],
            protocol: Option<&str>,
        ) {
            let mut list = nvs_runtime::NvsArray::new();
            for frame in frames {
                list.append(match frame {
                    Text(text) => Value::str(nvs_runtime::NvsStr::new(text.as_bytes())),
                    Bytes(octets) => Value::bytes(nvs_runtime::NvsStr::new(octets)),
                });
            }
            let args = [
                Value::str(nvs_runtime::NvsStr::new(url.as_bytes())),
                Value::array(list),
                protocol.map_or_else(Value::null, |name| {
                    Value::str(nvs_runtime::NvsStr::new(name.as_bytes()))
                }),
            ];
            let answered = nvs_runtime::call(nvs_core_test_answer_socket, ctx, &args)
                .expect("every peer a program can write is registered");
            dropped(answered);
            for arg in args {
                dropped(arg);
            }
        }
        fn kinds(ctx: &mut Ctx, url: &str) -> Vec<String> {
            ctx.faked_http_mut()
                .socket_for(url)
                .expect("a peer answers this URL")
                .frames
                .iter()
                .map(|frame| match frame {
                    Text(text) => format!("text {text}"),
                    Bytes(octets) => format!("bytes {}", octets.len()),
                })
                .collect()
        }

        let mut ctx = Ctx::buffered();
        peer(&mut ctx, "wss://feed.example.com/quiet", &[], None);
        assert!(
            ctx.faked_http_mut().is_armed(),
            "a peer with nothing to say still takes the test off the network"
        );
        assert!(kinds(&mut ctx, "wss://feed.example.com/quiet").is_empty());

        peer(
            &mut ctx,
            "wss://feed.example.com/*",
            &[Text("any".into())],
            None,
        );
        peer(
            &mut ctx,
            "wss://feed.example.com/prices/*",
            &[Text("prefix".into())],
            None,
        );
        peer(
            &mut ctx,
            "wss://feed.example.com/prices/live",
            &[
                Text("ready".into()),
                Bytes(vec![0, 1, 0xff]),
                Text("done".into()),
            ],
            Some("prices.v2"),
        );
        assert_eq!(
            kinds(&mut ctx, "wss://feed.example.com/prices/live"),
            ["text ready", "bytes 3", "text done"],
            "the exact URL wins, and its frames keep their kind and order"
        );
        let chosen = ctx
            .faked_http_mut()
            .socket_for("wss://feed.example.com/prices/live")
            .and_then(|peer| peer.protocol.clone());
        assert_eq!(chosen.as_deref(), Some("prices.v2"));
        assert_eq!(
            kinds(&mut ctx, "wss://feed.example.com/prices/old"),
            ["text prefix"],
            "the longest prefix wins among prefixes"
        );
        assert_eq!(kinds(&mut ctx, "wss://feed.example.com/news"), ["text any"]);
        assert!(
            ctx.faked_http_mut()
                .socket_for("wss://other.example.com/")
                .is_none(),
            "a URL no peer names is answered by none"
        );
    }

    /// § 5's own row, which is deliberately not shaped like § 4's: it takes the
    /// body whose failure is expected and nothing else — no `{message?:}`,
    /// because what it reports on is the ledger rather than a comparison, and
    /// no subject, because there is nothing being compared.
    #[test]
    fn expect_failure_takes_one_callable_and_answers_nothing() {
        let member = CLASS
            .methods
            .iter()
            .find(|method| method.name == "expectFailure")
            .expect("§ 5's member is registered");
        assert!(matches!(
            member.params,
            [CoreTy::CallableSig(params, ret)]
                if params.is_empty() && matches!(ret, CoreTy::Mixed)
        ));
        assert!(matches!(member.return_ty, CoreTy::Void));
        // It is one of the rows that assert nothing about a subject — this,
        // § 12's `advance`, `rule:tooling/a-prompt-is-a-core-member`'s
        // `scriptAnswers`, § 18's `request` and `serverUrl`,
        // `rule:testing/an-outbound-call-is-answered-from-a-table`'s
        // `answerHttp`, `tlsSession` and `sentHttp`, and
        // `rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`'s
        // `answerSocket` and `sentSocket`, and `rule:testing/doubles`'s
        // `double` and `partial`, which build a subject rather than claiming
        // anything about one — and [`asserting_members`] names each of them by
        // hand. This count is what makes adding a member to this class have to
        // answer "is it an assertion?": a new row joins § 4's shape sweep
        // unless it is listed there, and listing it moves this number.
        assert_eq!(asserting_members().count(), CLASS.methods.len() - 12);
        assert_eq!(equality_members().count(), 3);
    }

    /// The release a compiled call site owes for a value a member answered.
    fn dropped(value: Value) {
        #[expect(unsafe_code, reason = "the member transferred what it answered")]
        // SAFETY: a member's answer carries a reference of its own, and this
        // driver is the caller that would otherwise hold it.
        unsafe {
            value.release();
        }
    }

    /// `Core\Test::request`'s bag reaches the program as the request's own
    /// headers and body — `rule:testing/in-process-request`'s second paragraph,
    /// which promises a synthetic request's input arrives exactly as an arrived
    /// one's does.
    ///
    /// Asserted through `Core\Request`'s own members rather than off the spec,
    /// because what the bag is worth is what the program under test reads back.
    /// The body is not UTF-8 on purpose: it is the body a `.nvst` case cannot
    /// write for itself — a case file crosses as text — and the one
    /// `Core\Request::bytes` exists for.
    ///
    /// The derived field lines are the other half. `content-length` is the
    /// spec's, written because the bag wrote neither it nor a `content-type`,
    /// and `x-signature` is the bag's own, spelled as the program wrote it.
    #[test]
    fn a_synthetic_request_carries_its_headers_and_body_into_the_program() {
        const BODY: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

        let mut headers = nvs_runtime::NvsArray::new();
        headers.set(
            nvs_runtime::NvsStr::new(b"X-Signature"),
            Value::str(nvs_runtime::NvsStr::new(b"t=1,v1=deadbeef")),
        );
        // The slots the ABI hands a member, of which this reading uses the bag's
        // alone: the verb is the caller's `verb_of` and the target is a
        // parameter rather than a slot. The last is `mount`, written as the
        // default an omitting call site passes, which describes a request served
        // at the root.
        let args = [
            Value::int(0),
            Value::str(nvs_runtime::NvsStr::new(b"/hooks?since=2")),
            Value::array(headers),
            Value::bytes(nvs_runtime::NvsStr::new(BODY)),
            Value::str(nvs_runtime::NvsStr::new(b"")),
        ];
        let described =
            described("POST", "/hooks?since=2", &args).expect("this bag is what the row declares");
        let mut ctx = Ctx::buffered();
        ctx.set_inbound(described.build());

        let read = |ctx: &mut Ctx, member, arguments: &[Value]| -> Vec<u8> {
            let answered = nvs_runtime::call(member, ctx, arguments)
                .expect("this request carries what it was described with");
            let octets = answered
                .as_bytes()
                .or_else(|| answered.as_text().map(str::as_bytes))
                .expect("every reader here answers a string or a `bytes`")
                .to_vec();
            dropped(answered);
            octets
        };

        assert_eq!(
            read(&mut ctx, crate::request::nvs_core_request_bytes, &[]),
            BODY,
            "the bag's body is the request's body, octet for octet"
        );
        let signature = Value::str(nvs_runtime::NvsStr::new(b"x-signature"));
        assert_eq!(
            read(
                &mut ctx,
                crate::request::nvs_core_request_header,
                &[signature]
            ),
            b"t=1,v1=deadbeef",
            "and its header is a field line the program reads back under that name"
        );
        dropped(signature);
        let length = Value::str(nvs_runtime::NvsStr::new(b"content-length"));
        assert_eq!(
            read(&mut ctx, crate::request::nvs_core_request_header, &[length]),
            b"8",
            "with the length derived from the body, since the bag declared none"
        );
        dropped(length);
        let since = Value::str(nvs_runtime::NvsStr::new(b"since"));
        assert_eq!(
            read(&mut ctx, crate::request::nvs_core_request_query, &[since]),
            b"2",
            "and the target's query is split from its path exactly as the door splits one"
        );
        dropped(since);
    }

    /// The `int` [`answering`] hands back, so a call that reached the closure
    /// is told from one that did not.
    const ANSWERED: i64 = 7;

    /// A closure's `invoke`, hand-written: it sweeps the references
    /// `nvs_runtime::call_closure` retained for it — itself and its one
    /// parameter — and answers [`ANSWERED`].
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes exactly the closure and one parameter, \
                  each retained for this callee to release, and the result \
                  pointer is one value wide"
    )]
    unsafe extern "C" fn answering(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        unsafe {
            for slot in 0..2 {
                (*args.add(slot)).release();
            }
            out.write(Value::int(ANSWERED));
        }
        nvs_runtime::OK
    }

    /// A closure value of one parameter whose body is [`answering`].
    ///
    /// `nvs_runtime::call_closure` reads exactly four things off a closure —
    /// its class's `ClassTable::set_closure` bit, its arity slot, its
    /// parameter-tag slot and its `invoke`'s address — so a test in this crate
    /// can hand a double a `callable` with no compiler in front of it. The
    /// table is leaked because a descriptor's *address* is its identity and it
    /// must outlive every instance made from it.
    fn closure_of() -> Value {
        let mut table = nvs_runtime::ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                code: (answering as *const ()).cast(),
                arity: 1,
                param_tags: u64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY),
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor outlives \
                      every instance made from it — `NvsObj::new`'s whole \
                      obligation"
        )]
        let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
        object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(1));
        object.set_field(
            nvs_runtime::CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY)),
        );
        Value::object(object)
    }

    /// `rule:testing/interaction-after-the-fact`'s record, over the trampoline
    /// that writes it: two calls reaching a double leave two entries under the
    /// method's own name, each holding that call's arguments.
    ///
    /// Driven through `nvs_runtime::call_method`, which is the route
    /// `nvs_ir::ir::InstKind::CallVirtual` takes — so what is asserted is the
    /// row a compiled call site finds and the transfer it makes, rather than a
    /// trampoline called directly with slots nothing owned.
    // covers: Core\Test::double
    #[test]
    fn a_double_records_every_call_it_answers() {
        let mut interface = nvs_runtime::ClassTable::new();
        let id = interface.define("Mailer", &[] as &[&str], &[]);
        let interface: &'static nvs_runtime::ClassTable = Box::leak(Box::new(interface));
        let class = descriptor_for(
            "double",
            interface.desc(id),
            None,
            &[Answer {
                name: "send".to_owned(),
                arity: 1,
                params: u64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY),
                delegated: false,
            }],
        )
        .expect("one method is well under the ceiling");

        let mut ctx = Ctx::buffered();
        let double = double_of(
            class,
            Value::null(),
            vec![("send".to_owned(), closure_of())],
        );
        for recipient in [b"a@example.test".as_slice(), b"b@example.test"] {
            let to = Value::str(nvs_runtime::NvsStr::new(recipient));
            let answered = nvs_runtime::call_method(&mut ctx, double, "send", &[to], "a test")
                .expect("the trampoline answers")
                .expect("`send` is a row of the double's own class");
            assert_eq!(
                answered.as_int(),
                Some(ANSWERED),
                "the closure the shape gave is what answered the call"
            );
            dropped(to);
        }

        let held = crate::instance::slot(
            double.obj_ptr().expect("a double is an object"),
            RECORD_SLOT,
        );
        let ledger = crate::arr::borrowed(held.array_ptr().expect("the ledger is an array"));
        assert_eq!(ledger.count(), 1, "one key, for the one method called");
        let under = ledger.get(b"send").expect("keyed by the method's own name");
        let calls = crate::arr::borrowed(under.array_ptr().expect("a list of calls"));
        assert_eq!(
            calls.count(),
            2,
            "one entry per call, and neither is merged"
        );
        for (index, expected) in ["a@example.test", "b@example.test"].iter().enumerate() {
            let entry = calls
                .get_index(i64::try_from(index).expect("a test-sized index"))
                .expect("a call, in the order it was made");
            let entry = crate::arr::borrowed(entry.array_ptr().expect("a list of arguments"));
            assert_eq!(entry.count(), 1, "the one argument `send` declares");
            let argument = entry.get_index(0).expect("the argument itself");
            assert_eq!(
                argument.as_text(),
                Some(*expected),
                "recorded as the value the call site passed"
            );
        }
        dropped(double);
    }

    /// A partial publishes a delegated row for every method of the class
    /// behind it that its shape does not answer, and none for the constructor
    /// or for a method the shape overrides. Each delegated row keeps the real
    /// method's arity, and the rows together build a descriptor.
    // covers: Core\Test::partial
    #[test]
    fn a_partial_delegates_every_real_method_its_shape_leaves_out() {
        fn row(name: &str, arity: u32) -> nvs_runtime::MethodRow {
            nvs_runtime::MethodRow {
                name: name.to_owned(),
                code: (answering as *const ()).cast(),
                arity,
                param_tags: u64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY),
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }
        }

        let mut table = nvs_runtime::ClassTable::new();
        let interface = table.define("Prices", &[] as &[&str], &[]);
        let real = table.define("Table", &["base"], &[interface]);
        table.set_methods(
            real,
            vec![
                row(nvs_runtime::CONSTRUCTOR, 1),
                row("rate", 1),
                row("total", 1),
                row("currency", 0),
            ],
        );
        let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
        #[expect(
            unsafe_code,
            reason = "the table above is leaked, so the descriptor lives for the \
                      rest of the process"
        )]
        let behind = unsafe { &*table.desc(real) };

        let given: Vec<Given> = vec![("total".to_owned(), Value::null())];
        let delegated = delegated_rows(behind, &given);
        let mut names: Vec<(&str, u32)> = delegated
            .iter()
            .map(|answer| (answer.name.as_str(), answer.arity))
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [("currency", 0), ("rate", 1)],
            "every real method but the constructor and the overridden one"
        );
        assert!(
            delegated.iter().all(|answer| answer.delegated),
            "a row the shape did not answer forwards to the real object"
        );

        let mut rows = rows_of(&given);
        assert!(
            rows.iter().all(|answer| !answer.delegated),
            "an overridden method answers from its own closure"
        );
        rows.extend(delegated);
        let first = descriptor_for(
            "partial",
            table.desc(interface),
            Some(std::ptr::from_ref(behind)),
            &rows,
        )
        .expect("three methods are well under the ceiling");

        // A partial whose real object is that partial, overriding the same
        // method, is the same class, so a loop that wraps a partial again and
        // again defines one class and not one per wrap.
        #[expect(
            unsafe_code,
            reason = "the `DOUBLES` table is a `static`, so the class it \
                      defined lives for the rest of the process"
        )]
        let inner = unsafe { &*first };
        let mut again = rows_of(&given);
        again.extend(delegated_rows(inner, &given));
        let wrapped = descriptor_for("partial", table.desc(interface), Some(first), &again)
            .expect("the same three methods");
        assert!(
            std::ptr::eq(first, wrapped),
            "a partial of a partial is keyed by the class behind the inner one"
        );
    }

    /// `sentSocket` returns one `Core\Socket\Message` per recorded frame,
    /// oldest first. A text frame fills the `text` slot and a binary frame the
    /// `bytes` slot, and the other slot is `null`. Before any frame the list is
    /// empty, and reading it twice returns the same frames both times.
    // covers: Core\Test::sentSocket
    #[test]
    fn sent_socket_returns_one_message_per_frame_in_the_order_they_were_sent() {
        use nvs_runtime::SocketFrame::{Bytes, Text};

        fn sent(ctx: &mut Ctx) -> Vec<String> {
            let answered = nvs_runtime::call(nvs_core_test_sent_socket, ctx, &[] as &[Value])
                .expect("reading the record cannot fail");
            let list = crate::arr::borrowed(answered.array_ptr().expect("a list of messages"));
            let mut seen = Vec::new();
            let mut index = 0_i64;
            while let Some(message) = list.get_index(index) {
                let object = message.obj_ptr().expect("a `Core\\Socket\\Message`");
                let text = crate::instance::slot(object, 1);
                let bytes = crate::instance::slot(object, 2);
                seen.push(match (text.as_text(), bytes.as_bytes()) {
                    (Some(text), None) => format!("text {text}"),
                    (None, Some(octets)) => format!("bytes {octets:?}"),
                    _ => panic!("exactly one of `text` and `bytes` is filled"),
                });
                index += 1;
            }
            dropped(answered);
            seen
        }

        let mut ctx = Ctx::buffered();
        assert!(sent(&mut ctx).is_empty(), "nothing was sent yet");

        ctx.faked_http_mut()
            .record_frame(Text("subscribe mug".into()));
        ctx.faked_http_mut().record_frame(Bytes(vec![0, 1, 0xff]));
        ctx.faked_http_mut().record_frame(Text("bye".into()));
        let expected = ["text subscribe mug", "bytes [0, 1, 255]", "text bye"];
        assert_eq!(
            sent(&mut ctx),
            expected,
            "each frame keeps its kind and its place"
        );
        assert_eq!(
            sent(&mut ctx),
            expected,
            "reading the record does not consume it"
        );
    }

    /// `sentHttp` returns one `Core\Test\SentRequest` per recorded call, oldest
    /// first, with the verb as its `Core\Http\Method` case, the URL unchanged,
    /// the headers read case-insensitively and the body as the call carried
    /// it. Before any call the list is empty, and reading it twice returns the
    /// same records both times.
    // covers: Core\Test::sentHttp
    #[test]
    fn sent_http_returns_one_record_per_call_in_the_order_they_were_made() {
        fn text(of: &str) -> Value {
            Value::str(nvs_runtime::NvsStr::new(of.as_bytes()))
        }
        fn sent(ctx: &mut Ctx) -> Vec<String> {
            let answered = nvs_runtime::call(nvs_core_test_sent_http, ctx, &[] as &[Value])
                .expect("reading the record cannot fail");
            let list = crate::arr::borrowed(answered.array_ptr().expect("a list of records"));
            let mut seen = Vec::new();
            let mut index = 0_i64;
            while let Some(record) = list.get_index(index) {
                let object = record.obj_ptr().expect("a `Core\\Test\\SentRequest`");
                let verb = crate::instance::slot(object, SENT_METHOD_SLOT)
                    .as_int()
                    .expect("the verb is a case ordinal");
                let url = crate::instance::slot(object, SENT_URL_SLOT);
                let body = crate::instance::slot(object, SENT_BODY_SLOT);
                let name = text("Authorization");
                let header = nvs_runtime::call(nvs_core_test_sent_header, ctx, &[record, name])
                    .expect("reading a header cannot fail");
                dropped(name);
                seen.push(format!(
                    "{verb} {} {:?} {:?}",
                    url.as_text().expect("the URL is a string"),
                    header.as_text(),
                    body.as_bytes().expect("the body is bytes"),
                ));
                dropped(header);
                index += 1;
            }
            dropped(answered);
            seen
        }

        let mut ctx = Ctx::buffered();
        assert!(sent(&mut ctx).is_empty(), "nothing was sent yet");

        ctx.faked_http_mut().record(nvs_runtime::HttpSent {
            verb: "GET".into(),
            url: "https://shop.example.com/items?page=2".into(),
            headers: vec![("authorization".into(), "Bearer abc".into())],
            body: Vec::new(),
        });
        ctx.faked_http_mut().record(nvs_runtime::HttpSent {
            verb: "POST".into(),
            url: "https://shop.example.com/orders".into(),
            headers: vec![("content-type".into(), "application/json".into())],
            body: b"{\"sku\":\"mug\"}".to_vec(),
        });
        let get = crate::router::method_case("GET").expect("a verb the roster names");
        let post = crate::router::method_case("POST").expect("a verb the roster names");
        let expected = [
            format!("{get} https://shop.example.com/items?page=2 Some(\"Bearer abc\") []"),
            format!(
                "{post} https://shop.example.com/orders None {:?}",
                b"{\"sku\":\"mug\"}"
            ),
        ];
        assert_eq!(
            sent(&mut ctx),
            expected,
            "each call keeps its fields and its place"
        );
        assert_eq!(
            sent(&mut ctx),
            expected,
            "reading the record does not consume it"
        );
    }

    /// `request` hands the unit under test one request built from its
    /// arguments, and returns a `Core\Test\Response` carrying the status and
    /// the bytes the unit wrote. A unit that declares no status gives `200`.
    /// With no unit under test, and from inside a request, the call throws and
    /// says why.
    // covers: Core\Test::request
    #[test]
    fn request_returns_the_units_status_and_bytes_and_throws_with_no_unit() {
        /// A unit that writes back the request it was handed, and declares the
        /// status it was built with.
        #[derive(Debug)]
        struct Echo(Option<u16>);

        impl nvs_runtime::inproc::Answering for Echo {
            fn answer(
                &self,
                _ctx: &mut Ctx,
                inbound: Box<nvs_runtime::Inbound>,
            ) -> Result<nvs_runtime::host::Completion, String> {
                let written = format!(
                    "{} {} query={} mount={} headers={}",
                    inbound.method(),
                    inbound.path(),
                    inbound.query(),
                    inbound.mount_prefix(),
                    inbound.headers().len(),
                );
                Ok(nvs_runtime::host::Completion {
                    ok: true,
                    value: Value::null(),
                    output: written.into_bytes(),
                    content_type: None,
                    file_body: None,
                    status: self.0,
                    headers: Vec::new(),
                    error: None,
                    wall: None,
                    trace: Vec::new(),
                })
            }
        }

        fn asked(ctx: &mut Ctx, verb: &str, target: &str, mount: &str) -> Result<Value, i32> {
            let args = [
                Value::int(crate::router::method_case(verb).expect("a verb the roster names")),
                Value::str(nvs_runtime::NvsStr::new(target.as_bytes())),
                Value::array(nvs_runtime::NvsArray::new()),
                Value::null(),
                Value::str(nvs_runtime::NvsStr::new(mount.as_bytes())),
            ];
            let answered = nvs_runtime::call(nvs_core_test_request, ctx, &args);
            for arg in args {
                dropped(arg);
            }
            answered
        }
        fn read(answered: Value) -> (i64, String) {
            let object = answered.obj_ptr().expect("a `Core\\Test\\Response`");
            let status = crate::instance::slot(object, STATUS_SLOT)
                .as_int()
                .expect("the status is an integer");
            let body = crate::instance::slot(object, BODY_SLOT)
                .as_text()
                .expect("the body is a string")
                .to_owned();
            dropped(answered);
            (status, body)
        }

        let mut ctx = Ctx::buffered();
        let silent = nvs_runtime::inproc::scoped(&Echo(None), || {
            asked(&mut ctx, "GET", "/users/7?tab=2", "/shop")
        })
        .expect("an installed unit answers");
        assert_eq!(
            read(silent),
            (
                200,
                "GET /users/7 query=tab=2 mount=/shop headers=0".to_owned()
            ),
            "the target splits at `?`, the mount crosses, and no status reads as `200`"
        );
        let declared = nvs_runtime::inproc::scoped(&Echo(Some(201)), || {
            asked(&mut ctx, "POST", "/orders", "")
        })
        .expect("an installed unit answers");
        assert_eq!(
            read(declared),
            (201, "POST /orders query= mount= headers=0".to_owned()),
            "the status the unit declared is the one returned"
        );

        assert!(
            asked(&mut ctx, "GET", "/", "").is_err(),
            "a run with no unit under test throws"
        );
        let message = ctx.take_pending().unwrap_or_default();
        assert!(
            message.starts_with("Core\\Test::request could not run the request: there is no unit"),
            "the error names the missing unit: {message}"
        );

        ctx.set_inbound(nvs_runtime::Inbound::new("GET", "/served", ""));
        let inside =
            nvs_runtime::inproc::scoped(&Echo(None), || asked(&mut ctx, "GET", "/again", ""));
        assert!(inside.is_err(), "a request made inside a request throws");
        let message = ctx.take_pending().unwrap_or_default();
        assert!(
            message.contains("may not be made from inside one"),
            "the error names the re-entry: {message}"
        );
    }

    /// `serverUrl` returns the address the runner armed on this context, and
    /// `null` on a context nobody armed one on.
    // covers: Core\Test::serverUrl
    #[test]
    fn server_url_returns_the_armed_address_and_null_without_one() {
        let mut ctx = Ctx::buffered();
        let unarmed = nvs_runtime::call(nvs_core_test_server_url, &mut ctx, &[] as &[Value])
            .expect("reading the address cannot fail");
        assert!(
            matches!(unarmed.tag(), Some(Tag::Null)),
            "a context nobody armed has no address"
        );

        ctx.set_test_server("http://127.0.0.1:49152".to_owned());
        let armed = nvs_runtime::call(nvs_core_test_server_url, &mut ctx, &[] as &[Value])
            .expect("reading the address cannot fail");
        assert_eq!(
            armed.as_text(),
            Some("http://127.0.0.1:49152"),
            "the address is the one the runner armed, unchanged"
        );
        dropped(armed);
    }

    /// `assertCalled` and `assertNeverCalled` read one record and agree on it.
    /// After two calls to `send` and none to `purge`, `times` holds at the
    /// exact count and fails one either side of it, `with` finds the one call
    /// it names among two, and the pair gives opposite answers for each method.
    // covers: Core\Test::assertCalled, Core\Test::assertNeverCalled
    #[test]
    fn assert_called_and_assert_never_called_agree_over_one_record() {
        fn text(of: &str) -> Value {
            Value::str(nvs_runtime::NvsStr::new(of.as_bytes()))
        }
        fn outcome(ctx: &mut Ctx, answered: Result<Value, i32>) -> Result<(), String> {
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a failed assertion says why")
                    .into_owned()),
            }
        }
        fn called(
            ctx: &mut Ctx,
            double: Value,
            method: &str,
            times: Option<u64>,
            with: Option<&str>,
        ) -> Result<(), String> {
            let with = with.map_or_else(Value::null, |argument| {
                let mut list = nvs_runtime::NvsArray::new();
                list.append(text(argument));
                Value::array(list)
            });
            let args = [
                double,
                text(method),
                times.map_or_else(Value::null, Value::uint),
                with,
                Value::null(),
            ];
            let answered = nvs_runtime::call(nvs_core_test_assert_called, ctx, &args);
            for arg in &args[1..] {
                dropped(*arg);
            }
            outcome(ctx, answered)
        }
        fn never_called(ctx: &mut Ctx, double: Value, method: &str) -> Result<(), String> {
            let args = [double, text(method), Value::null()];
            let answered = nvs_runtime::call(nvs_core_test_assert_never_called, ctx, &args);
            dropped(args[1]);
            outcome(ctx, answered)
        }

        let mut interface = nvs_runtime::ClassTable::new();
        let id = interface.define("Mailer", &[] as &[&str], &[]);
        let interface: &'static nvs_runtime::ClassTable = Box::leak(Box::new(interface));
        let class = descriptor_for(
            "double",
            interface.desc(id),
            None,
            &[Answer {
                name: "send".to_owned(),
                arity: 1,
                params: u64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY),
                delegated: false,
            }],
        )
        .expect("one method is well under the ceiling");

        let mut ctx = Ctx::buffered();
        let double = double_of(
            class,
            Value::null(),
            vec![("send".to_owned(), closure_of())],
        );
        for recipient in ["a@example.test", "b@example.test"] {
            let to = text(recipient);
            let answered = nvs_runtime::call_method(&mut ctx, double, "send", &[to], "a test")
                .expect("the trampoline answers")
                .expect("`send` is a row of the double's own class");
            dropped(answered);
            dropped(to);
        }

        called(&mut ctx, double, "send", None, None).expect("called at all");
        called(&mut ctx, double, "send", Some(2), None).expect("called exactly twice");
        for wrong in [1, 3] {
            let why = called(&mut ctx, double, "send", Some(wrong), None)
                .expect_err("the count is exact");
            assert!(
                why.contains(&format!("expected {wrong} call(s) to `send`")),
                "{why}"
            );
        }
        called(&mut ctx, double, "send", Some(1), Some("b@example.test"))
            .expect("one of the two calls names the second address");
        let why = called(&mut ctx, double, "send", None, Some("c@example.test"))
            .expect_err("no call names this address");
        assert!(why.contains("found 0"), "{why}");

        let why = never_called(&mut ctx, double, "send").expect_err("`send` was called");
        assert!(why.contains("`send` was called 2 time(s)"), "{why}");
        never_called(&mut ctx, double, "purge").expect("`purge` never was");
        called(&mut ctx, double, "purge", None, None).expect_err("so it was not called");
        dropped(double);
    }

    /// `assertEquals` and `assertEqualsDeep` share one comparison below the
    /// object row: over a table of scalar and list pairs they agree on every
    /// verdict, `assertEquals` quotes both sides, and `assertEqualsDeep` names
    /// the first index where two lists part.
    // covers: Core\Test::assertEquals, Core\Test::assertEqualsDeep
    #[test]
    fn assert_equals_and_assert_equals_deep_agree_below_the_object_row() {
        fn list(of: &[i64]) -> Value {
            let mut list = nvs_runtime::NvsArray::new();
            for n in of {
                list.append(Value::int(*n));
            }
            Value::array(list)
        }
        fn asserted(
            ctx: &mut Ctx,
            member: nvs_runtime::NvsFn,
            actual: Value,
            expected: Value,
        ) -> Result<(), String> {
            let args = [actual, expected, Value::null()];
            let answered = nvs_runtime::call(member, ctx, &args);
            dropped(actual);
            dropped(expected);
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a failed assertion says why")
                    .into_owned()),
            }
        }

        type Side = fn() -> Value;

        let mut ctx = Ctx::buffered();
        let pairs: [(Side, Side, bool); 6] = [
            (|| Value::int(7), || Value::int(7), true),
            (|| Value::int(7), || Value::int(8), false),
            (|| Value::null(), || Value::null(), true),
            (|| list(&[1, 2, 3]), || list(&[1, 2, 3]), true),
            (|| list(&[1, 2, 3]), || list(&[1, 9, 3]), false),
            (|| list(&[1, 2]), || list(&[1, 2, 3]), false),
        ];
        for (row, (actual, expected, equal)) in pairs.iter().enumerate() {
            let flat = asserted(&mut ctx, nvs_core_test_assert_equals, actual(), expected());
            let deep = asserted(
                &mut ctx,
                nvs_core_test_assert_equals_deep,
                actual(),
                expected(),
            );
            assert_eq!(flat.is_ok(), *equal, "row {row}: {flat:?}");
            assert_eq!(deep.is_ok(), *equal, "row {row}: {deep:?}");
        }

        let why = asserted(
            &mut ctx,
            nvs_core_test_assert_equals,
            Value::int(7),
            Value::int(8),
        )
        .expect_err("7 is not 8");
        assert!(why.contains("`$actual` is 7, `$expected` is 8"), "{why}");
        let why = asserted(
            &mut ctx,
            nvs_core_test_assert_equals_deep,
            list(&[1, 2, 3]),
            list(&[1, 9, 3]),
        )
        .expect_err("the lists part at index 1");
        assert!(
            why.contains("differ at `$actual[\"1\"]`: `$actual` is 2, `$expected` is 9"),
            "{why}"
        );
    }

    /// `assertCount` passes on the exact count and fails one entry either
    /// side of it, and `assertContains` finds a value by the equality
    /// semantics' numeric row, so `[1.0, 2.5]` contains `1` and not `3`. Both
    /// failures name the subject's count.
    // covers: Core\Test::assertCount, Core\Test::assertContains
    #[test]
    fn assert_count_bounds_the_count_and_assert_contains_finds_across_the_numeric_row() {
        fn asserted(
            ctx: &mut Ctx,
            member: nvs_runtime::NvsFn,
            actual: Value,
            expected: Value,
        ) -> Result<(), String> {
            let args = [actual, expected, Value::null()];
            let answered = nvs_runtime::call(member, ctx, &args);
            dropped(actual);
            dropped(expected);
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a failed assertion says why")
                    .into_owned()),
            }
        }
        fn floats(of: &[f64]) -> Value {
            let mut list = nvs_runtime::NvsArray::new();
            for n in of {
                list.append(Value::float(*n));
            }
            Value::array(list)
        }

        let mut ctx = Ctx::buffered();
        for (expected, holds) in [(1, false), (2, true), (3, false)] {
            let answered = asserted(
                &mut ctx,
                nvs_core_test_assert_count,
                floats(&[1.0, 2.5]),
                Value::uint(expected),
            );
            assert_eq!(answered.is_ok(), holds, "{expected}: {answered:?}");
        }
        let why = asserted(
            &mut ctx,
            nvs_core_test_assert_count,
            floats(&[]),
            Value::uint(1),
        )
        .expect_err("an empty list has no entries");
        assert!(
            why.contains("`$actual` holds 0 entries, `$expected` is 1"),
            "{why}"
        );

        asserted(
            &mut ctx,
            nvs_core_test_assert_contains,
            floats(&[1.0, 2.5]),
            Value::int(1),
        )
        .expect("1.0 is 1 under the numeric row");
        let why = asserted(
            &mut ctx,
            nvs_core_test_assert_contains,
            floats(&[1.0, 2.5]),
            Value::int(3),
        )
        .expect_err("no entry is 3");
        assert!(
            why.contains("`$actual` holds 2 entries and none is `$expected`, which is 3"),
            "{why}"
        );
    }

    /// `assertNull` holds for `null` alone: `0`, `false`, `""` and an empty
    /// array are falsy and still fail. `assertSame` compares by
    /// `nvs_runtime::identity`'s rows, so two separately built equal strings
    /// and arrays hold, and `1` beside `1.0` holds because the numeric row is
    /// one domain, where PHP's `===` says `false`. Both failures render the
    /// subjects they judged.
    // covers: Core\Test::assertNull, Core\Test::assertSame
    #[test]
    fn assert_null_holds_for_null_alone_and_assert_same_reads_one_numeric_domain() {
        fn asserted(
            ctx: &mut Ctx,
            member: nvs_runtime::NvsFn,
            args: &[Value],
        ) -> Result<(), String> {
            let answered = nvs_runtime::call(member, ctx, args);
            for arg in args {
                dropped(*arg);
            }
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a failed assertion says why")
                    .into_owned()),
            }
        }
        fn text(of: &str) -> Value {
            Value::str(nvs_runtime::NvsStr::new(of.as_bytes()))
        }
        fn list(of: &[i64]) -> Value {
            let mut list = nvs_runtime::NvsArray::new();
            for n in of {
                list.append(Value::int(*n));
            }
            Value::array(list)
        }

        let mut ctx = Ctx::buffered();
        asserted(
            &mut ctx,
            nvs_core_test_assert_null,
            &[Value::null(), Value::null()],
        )
        .expect("null is null");
        type Subject = fn() -> Value;
        let falsy: [(Subject, &str); 4] = [
            (|| Value::int(0), "`$actual` is 0"),
            (|| Value::bool(false), "`$actual` is false"),
            (|| text(""), "`$actual` is \"\""),
            (|| list(&[]), "`$actual` is an array of 0"),
        ];
        for (subject, reported) in falsy {
            let why = asserted(
                &mut ctx,
                nvs_core_test_assert_null,
                &[subject(), Value::null()],
            )
            .expect_err("a falsy value is not null");
            assert!(why.contains(reported), "{why}");
        }

        asserted(
            &mut ctx,
            nvs_core_test_assert_same,
            &[text("10"), text("10"), Value::null()],
        )
        .expect("two strings with the same bytes are identical");
        asserted(
            &mut ctx,
            nvs_core_test_assert_same,
            &[list(&[1, 2, 3]), list(&[1, 2, 3]), Value::null()],
        )
        .expect("two arrays with identical entries are identical");
        let why = asserted(
            &mut ctx,
            nvs_core_test_assert_same,
            &[text("010"), text("10"), Value::null()],
        )
        .expect_err("the strings differ");
        assert!(
            why.contains("`$actual` is \"010\", `$expected` is \"10\""),
            "{why}"
        );
        asserted(
            &mut ctx,
            nvs_core_test_assert_same,
            &[Value::int(1), Value::float(1.0), Value::null()],
        )
        .expect("1 and 1.0 are one value of the numeric domain");
        let why = asserted(
            &mut ctx,
            nvs_core_test_assert_same,
            &[Value::int(1), Value::float(1.5), Value::null()],
        )
        .expect_err("1 is not 1.5");
        assert!(why.contains("`$actual` is 1, `$expected` is 1.5"), "{why}");
    }

    /// `assertMatchesInline` holds when `Core\Debug::render`'s text is the
    /// snapshot byte for byte, and records nothing. A mismatch, including the
    /// empty snapshot the `--update` workflow starts from, quotes both texts
    /// and leaves one `SnapshotMismatch` per failure for the runner to join.
    // covers: Core\Test::assertMatchesInline
    #[test]
    fn assert_matches_inline_compares_the_rendering_and_records_each_mismatch() {
        fn matched(ctx: &mut Ctx, actual: Value, snapshot: &str) -> Result<(), String> {
            let expected = Value::str(nvs_runtime::NvsStr::new(snapshot.as_bytes()));
            let args = [actual, expected, Value::null()];
            let answered = nvs_runtime::call(nvs_core_test_assert_matches_inline, ctx, &args);
            dropped(actual);
            dropped(expected);
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a failed assertion says why")
                    .into_owned()),
            }
        }

        let mut ctx = Ctx::buffered();
        matched(&mut ctx, Value::int(7), "int(7)").expect("7 renders as int(7)");
        assert!(ctx.take_snapshot_mismatches().is_empty());

        let why = matched(&mut ctx, Value::int(7), "int(8)").expect_err("7 is not int(8)");
        assert!(
            why.contains("the rendering is \"int(7)\", and the snapshot holds \"int(8)\""),
            "{why}"
        );
        let why = matched(&mut ctx, Value::bool(true), "").expect_err("nothing matches \"\"");
        assert!(why.contains("the snapshot holds \"\""), "{why}");

        let recorded: Vec<(String, String)> = ctx
            .take_snapshot_mismatches()
            .into_iter()
            .map(|m| (m.expected, m.produced))
            .collect();
        assert_eq!(
            recorded,
            [
                ("int(8)".to_owned(), "int(7)".to_owned()),
                (String::new(), "bool(true)".to_owned()),
            ]
        );
    }

    /// The three members that take a `callable` judge how it ended.
    /// `assertDoesNotThrow` holds for a body that returned and turns a body's
    /// throw into its own failure quoting the message. `assertCompletes` throws
    /// before it calls the body where no clock is fixed, and where one is, moves
    /// it by `within` before the body reads it. `assertThrows` fails a body that
    /// returned, naming the class it expected. Judging a throw's class needs an
    /// exception class table, so the examples and the hostile case pin that half
    /// from Novis.
    // covers: Core\Test::assertDoesNotThrow, Core\Test::assertCompletes, Core\Test::assertThrows
    #[test]
    fn every_member_taking_a_body_judges_it_by_how_it_ended() {
        use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

        static RAN: AtomicUsize = AtomicUsize::new(0);
        static SAW: AtomicI64 = AtomicI64::new(-1);

        /// `fn (): void => {}`, counting its calls and recording the fixed
        /// clock it read.
        #[expect(
            unsafe_code,
            reason = "`call_closure` passes a live context and exactly one retained \
                      value, the receiver, and `abi::call` passes the address of a \
                      live `Value` for the result"
        )]
        unsafe extern "C" fn returns(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
            let ctx = unsafe { &mut *ctx };
            RAN.fetch_add(1, Ordering::SeqCst);
            let nanos = ctx
                .fixed_clock()
                .map_or(-1, |at| i64::try_from(at).unwrap_or(-2));
            SAW.store(nanos, Ordering::SeqCst);
            unsafe {
                (*args).release();
                out.write(Value::null());
            }
            nvs_runtime::OK
        }

        /// A body that throws: `Core\Test::advance(1ns)` on a context with no
        /// fixed clock, whose status it passes on.
        #[expect(
            unsafe_code,
            reason = "`call_closure` passes a live context and exactly one retained \
                      value, the receiver, and `abi::call` passes the address of a \
                      live `Value` for the result"
        )]
        unsafe extern "C" fn throws(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
            let ctx = unsafe { &mut *ctx };
            unsafe { (*args).release() };
            let by = crate::time::duration_of(1);
            let answered = nvs_runtime::call(nvs_core_test_advance, ctx, &[by]);
            dropped(by);
            match answered {
                Ok(value) => {
                    unsafe { out.write(value) };
                    nvs_runtime::OK
                }
                Err(status) => status,
            }
        }

        fn body_of(invoke: nvs_runtime::NvsFn) -> Value {
            let mut table = nvs_runtime::ClassTable::new();
            let id = table.define("{closure}", &["arity", "params"], &[]);
            table.set_methods(
                id,
                vec![nvs_runtime::MethodRow {
                    name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                    code: (invoke as *const ()).cast(),
                    arity: 0,
                    param_tags: 0,
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    public: true,
                    protected: false,
                    native: false,
                }],
            );
            table.set_closure(id);
            let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
            #[expect(
                unsafe_code,
                reason = "the table above is leaked, so the descriptor outlives \
                          every instance made from it — `NvsObj::new`'s whole \
                          obligation"
            )]
            let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
            object.set_field(nvs_runtime::CLOSURE_ARITY_SLOT, Value::int(0));
            object.set_field(nvs_runtime::CLOSURE_PARAM_TAGS_SLOT, Value::int(0));
            Value::object(object)
        }

        fn judged(ctx: &mut Ctx, member: nvs_runtime::NvsFn, args: &[Value]) -> Result<(), String> {
            let answered = nvs_runtime::call(member, ctx, args);
            for arg in args {
                dropped(*arg);
            }
            match answered {
                Ok(null) => {
                    dropped(null);
                    Ok(())
                }
                Err(_) => Err(ctx
                    .take_pending()
                    .expect("a failed assertion says why")
                    .into_owned()),
            }
        }

        let mut ctx = Ctx::buffered();
        judged(
            &mut ctx,
            nvs_core_test_assert_does_not_throw,
            &[body_of(returns), Value::null()],
        )
        .expect("a body that returned threw nothing");
        assert_eq!(RAN.load(Ordering::SeqCst), 1);
        let why = judged(
            &mut ctx,
            nvs_core_test_assert_does_not_throw,
            &[body_of(throws), Value::null()],
        )
        .expect_err("the body threw");
        assert!(
            why.contains("`$body` threw: Core\\Test::advance(): this test declared no `at:`"),
            "{why}"
        );

        let why = judged(
            &mut ctx,
            nvs_core_test_assert_completes,
            &[
                body_of(returns),
                crate::time::duration_of(50_000_000),
                Value::null(),
            ],
        )
        .expect_err("no clock is fixed");
        assert!(
            why.contains("no fixed clock to spend `within` against"),
            "{why}"
        );
        assert_eq!(RAN.load(Ordering::SeqCst), 1, "the refused body never ran");

        ctx.set_fixed_clock(0);
        judged(
            &mut ctx,
            nvs_core_test_assert_completes,
            &[
                body_of(returns),
                crate::time::duration_of(50_000_000),
                Value::null(),
            ],
        )
        .expect("a body that returned left no task running");
        assert_eq!(RAN.load(Ordering::SeqCst), 2);
        assert_eq!(
            SAW.load(Ordering::SeqCst),
            50_000_000,
            "the clock moved first"
        );
        assert_eq!(ctx.fixed_clock(), Some(50_000_000));

        let mut bare = Ctx::buffered();
        let expected = || Value::str(nvs_runtime::NvsStr::new(b"LogicError"));
        let why = judged(
            &mut bare,
            nvs_core_test_assert_throws,
            &[body_of(returns), expected(), Value::null()],
        )
        .expect_err("the body returned");
        assert!(
            why.contains("`$body` returned without throwing LogicError"),
            "{why}"
        );
        assert_eq!(RAN.load(Ordering::SeqCst), 3);
    }
}
