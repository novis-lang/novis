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
//! # Known gaps
//!
//! 1. **§ 4's two compile errors are runtime throws for now.** A non-
//!    `Comparable` object under `assertEquals` is refused where it is *written*
//!    by the ADR, and is a catchable throw naming `assertEqualsDeep` here; that
//!    refusal is `nvs_types`' to make and wants the class graph this crate does
//!    not hold. The mismatch error is already made, by the `T` above.
//!    Decided: No: keep the runtime throw naming assertEqualsDeep — No rule change; the mistake shows
//!    up when the test runs, which is soon anyway.
//!    — owner: unowned-closures
//! 2. **`assertThrows` matches a class by name, so a failure with no class
//!    installed matches nothing.** `nvs_runtime::Ctx::pending_conforms_to`
//!    reads the ancestry off a descriptor, and a helper-raised failure carries
//!    none until `Ctx::set_runtime_error_class` has installed one — which a
//!    compiled unit always has, so this is reachable only from a host embedding
//!    the runtime without one.
//!    Decided: No: state it in the embedding contract and assert it when a Ctx is built — One
//!    assertion; the silent non-match becomes impossible.
//!    — owner: unowned-closures
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
//! # § 18's in-process request, and what of it is still owed
//!
//! `request` runs a synthetic request through the program under test with no
//! socket and no port, and [`RESPONSE`]'s two accessors are what it answers
//! with. The mechanism is [`nvs_runtime::inproc`]'s and that module's doc is
//! the one home of it; what is recorded here is the gap.
//!
//! **A synthetic request carries no headers and no body yet.** § 18's worked
//! example passes a `{headers: ...}` bag, and its second paragraph says the
//! body and parameters arrive `tainted` exactly as a real request's would —
//! true of the path's query, which crosses on the carrier, and vacuous for the
//! other two, which have no spelling to arrive through. The bag is the next
//! slice's; a body needs a `nvs_runtime::RequestBody` over held bytes, which
//! nothing in this crate builds today.
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

use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Value, identity};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
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
];

/// `Core\Test`'s registry rows — § 4's three equality members, the three
/// predicate ones its example writes beside them, and § 5's `expectFailure`.
/// See
/// [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
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
            name: "sentHttp",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Instance(SENT_REQUEST_NAME)),
            symbol: "nvs_core_test_sent_http",
            doc: Some(&SENT_HTTP_DOC),
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
    short: "Moves the fixed clock a `#[Test(at: ...)]` declared forward by `$by`, so a test of \
            something that expires can reach the far side of the expiry without waiting — the \
            one mutator that clock has.",
    params: &[ParamDoc {
        name: "by",
        desc: "The exact duration to move the clock forward; a negative one moves it back.",
        shape: &[],
    }],
    ret: "Nothing. The next `Core\\Time::now()` reads the moved clock.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The running test declared no `at:`, so there is no fixed clock to move — the \
                   host's clock is never advanced.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The moved reading lies outside the representable range, about ±9999 years.",
        },
    ],
};

/// `Core\Test::serverUrl`'s reference card — `rule:core-api/reference-card`.
const SERVER_URL_DOC: MethodDoc = MethodDoc {
    short: "The base URL of the listener a `#[Test(server: true)]` case was given — a real socket \
            on a port the operating system chose, for the cases that genuinely need the wire \
            rather than an in-process request.",
    params: &[],
    ret: "`http://127.0.0.1:<port>` with no trailing slash, so a path appends directly; `null` \
          anywhere no listener was bound, which is every context but a `server: true` test.",
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
    short: "Runs one request through the program under test in this process — the compiled route \
            table and the real handler chain, with no socket and no port — and answers with what \
            the program wrote.",
    params: &[
        ParamDoc {
            name: "method",
            desc: "The verb the synthetic request carries, matched against the table exactly as \
                   an arrived one is.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The path to ask for, mount prefix already stripped — what a handler's \
                   `#[Route]` is declared against. A `?` and everything after it is the query.",
            shape: &[],
        },
    ],
    ret: "The status the program declared and the bytes it wrote. A path the table does not claim \
          is still answered: nothing here dispatches, so the program decides what a miss means.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "There is no program under test — the call is outside a `nvs test` or `nvs run` \
               invocation — or the call is already inside an in-process request, which is \
               refused because the program answering one is the program that asked.",
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
    ],
    ret: "Nothing. Answers accumulate, so a test registers as many as it has calls; a URL \
          answered exactly wins over one answered by a prefix, and the longest prefix wins among \
          prefixes.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The answer names both `json` and `body`, which are two spellings of one body; or \
               the status is not one a status line can carry.",
    }],
};

/// `Core\Test::sentHttp`'s reference card — `rule:core-api/reference-card`.
const SENT_HTTP_DOC: MethodDoc = MethodDoc {
    short: "Every outbound call the program under test has made since the answer table was \
            armed, oldest first — what was sent, rather than what came back.",
    params: &[],
    ret: "One `Core\\Test\\SentRequest` per call, in the order the program made them, and an \
          empty array for a test that registered answers nobody asked for. Nothing on a record \
          is `tainted`: it is the program's own text.",
    errors: &[],
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
    /// `Core\Test::answerHttp(string $url, uint $status, {json?, body?, headers?}): void`
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
    fn nvs_core_test_answer_http(ctx, args: [5]) {
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
            Some(crate::json::written(args[2], member)?)
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
        });
        Ok(Value::null())
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
        "nvs_core_test_answer_http" => (nvs_core_test_answer_http as *const ()).cast(),
        "nvs_core_test_sent_http" => (nvs_core_test_sent_http as *const ()).cast(),
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

nvs_runtime::nvs_helper! {
    /// `Core\Test::request(Core\Http\Method $method, string $path): Core\Test\Response`
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
    /// **Known gap, `rule:testing/in-process-request`'s second sentence:** the worked example's
    /// `{headers: ...}` bag and a synthetic body are not here yet, so nothing a
    /// synthetic request carries arrives `tainted` because it carries nothing.
    /// The module doc's own gap list is the home of that.
    fn nvs_core_test_request(ctx, args: [2]) {
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
        // Split exactly as the door does: everything after the first `?` is the
        // query, undecoded, and a target with none has an empty one rather than
        // no query at all.
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let inbound = nvs_runtime::Inbound::new(&verb, path, query);
        let mut completion = match nvs_runtime::inproc::answer(ctx, Box::new(inbound)) {
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
        let Some(diff) = difference(args[0], args[1], 0, "")? else {
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
    /// decision that an **ancestor** matches.
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
/// A catchable [`Fault::thrown`] naming `assertEqualsDeep` where the receiver's
/// class declares no `compareTo` — this module's known gap 1, since the ADR
/// refuses that program where it is *written*. Or the callee's own failure,
/// propagated as [`Fault::Pending`] by `nvs_runtime::call_method`.
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
) -> Result<Option<Diff>, Fault> {
    if depth >= MAX_DEPTH {
        return Err(Fault::thrown(format!(
            "Core\\Test::assertEqualsDeep walked {MAX_DEPTH} levels of containment at \
             `$actual{path}` without reaching a scalar — compare a narrower subject, or \
             `Core\\Test::assertSame` if identity is what was meant"
        )));
    }
    match (actual.tag(), expected.tag()) {
        (Some(Tag::Array), Some(Tag::Array)) => array_difference(actual, expected, depth, path),
        (Some(Tag::Object), Some(Tag::Object)) => object_difference(actual, expected, depth, path),
        _ => Ok(leaf(actual, expected, path)),
    }
}

/// How deep [`difference`] descends before it refuses to answer.
const MAX_DEPTH: usize = 64;

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
        if let Some(diff) = difference(mine, theirs, depth + 1, &at)? {
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
) -> Result<Option<Diff>, Fault> {
    let (Some(left), Some(right)) = (actual.obj_ptr(), expected.obj_ptr()) else {
        return Ok(leaf(actual, expected, path));
    };
    // The one shortcut, and the reason a shared sub-object in both graphs does
    // not spend the depth budget twice: one allocation is equal to itself under
    // every row of this walk.
    if std::ptr::eq(left, right) {
        return Ok(None);
    }
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
        let Some(diff) = difference(left.field(slot), right.field(slot), depth + 1, &at)? else {
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
        Some(Tag::Closure) => "a closure".to_owned(),
        Some(Tag::Resource) => "a resource".to_owned(),
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
                    | "answerHttp"
                    | "sentHttp"
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

    /// The one option § 4 writes, and the reason it defaults to `null` rather
    /// than to an empty `string`.
    #[test]
    fn the_only_option_is_a_message_that_defaults_to_absent() {
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
            assert_eq!(bag.len(), 1);
            assert_eq!(bag[0].name, "message");
            assert!(matches!(bag[0].ty, CoreTy::Text(Qual::Neutral)));
            assert!(matches!(bag[0].default, Const::Null));
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
        // `scriptAnswers`, § 18's `request` and `serverUrl`, and
        // `rule:testing/an-outbound-call-is-answered-from-a-table`'s
        // `answerHttp` and `sentHttp` — and [`asserting_members`] names each of
        // them by hand. This count is what makes adding a member to this class
        // have to answer "is it an assertion?": a new row joins § 4's shape
        // sweep unless it is listed there, and listing it moves this number.
        assert_eq!(asserting_members().count(), CLASS.methods.len() - 7);
        assert_eq!(equality_members().count(), 3);
    }
}
