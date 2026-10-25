//! The type checker's codes, continued from `types_e07.rs`: `E08xx`. Once
//! this range is full, the area continues in the reserve range, `E10xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E08xx` fails the
//! crate's tests.

codes! {
    range = "E08";

    //
    // The third band for one stage. `E07xx` filled at `E0799`, and `E08xx`,
    // held in reserve for whichever band filled next, continues it; `E10xx`
    // is the reserve. The paragraph under this module's legend table
    // owns the reasoning.
    //

    /// `callable(int $x): string` — a parameter name inside a `callable` type,
    /// `rule:types/callable-signature`.
    ///
    /// The band's first code, claimed for `rule:types/callable-signature` before
    /// anything below it was written, which is why it sits ahead of the
    /// numbers rather than filling a hole. A name in the type would imply
    /// calling through the value by name, which nothing supports; reported
    /// once per type however many parameters carry one, since writing PHP's
    /// parameter spelling is one mistake and deleting the names is one fix.
    pub const E_CALLABLE_TYPE_NAMES_A_PARAMETER: Code = Code::new("E0800").card(
        "A `callable` type lists only the types of its parameters, not their names. Write \
         `callable(int): string`, not `callable(int $x): string`.",
    );

    /// Two writers of one response body — `echo` or page text outside
    /// `<?nvs … ?>` and a typed body member, or two different typed members —
    /// `rule:security/response-body-is-one-typed-member`
    /// 's sixth row.
    ///
    /// They disagree about the body's type and its `Content-Type`, and
    /// letting the last one win is how a JSON endpoint acquires an HTML
    /// prelude. One member called twice is one writer and is not this
    /// diagnostic. Reported inside a `#[Route]` handler and nowhere else, because
    /// § 3 binds `echo` by context and a handler is the one body a request is
    /// statically certain to reach — `nvs_types::response`'s module doc owns
    /// that scope, the reach inside a handler, and the entry script the scope
    /// leaves to the run-time default.
    pub const E_ECHO_BESIDE_A_BODY_MEMBER: Code = Code::new("E0801").card(
        "This route handler writes its response body in two ways, for example with `echo` and \
         with a body method. A response has one body, so write it in one way only.",
    );

    /// A `spawn script` operand that is neither of
    /// `rule:security/isolate-shares-nothing`'s two entry forms — an `fn` anonymous function, or a value whose type
    /// is `callable`.
    ///
    /// One code for both because it is one rule: the entry is **decided
    /// syntactically at the spawn site**, a path or a `Class::method(...)`
    /// reference written there, and everything else is refused for the same
    /// reason — whether a callable captures is not a question the spawn site
    /// can answer, and an isolate that captured would share more than compiled
    /// code. The two halves differ only in the `help:`, because an anonymous
    /// function has a mechanical way out (give it a name and a class) and a variable
    /// does not.
    ///
    /// Deliberately not [`E_TYPE_MISMATCH`], which is what a `callable` against
    /// a `string` parameter would read as: the operand position accepts two
    /// unrelated shapes, so "expected `string`" describes half the rule and
    /// sends the reader to fix the wrong half.
    pub const E_SPAWN_ENTRY_NOT_A_PATH_OR_METHOD: Code = Code::new("E0802").card(
        "`spawn script` needs a path or a `static` method written as `Class::method(...)`. An \
         anonymous function written with `fn` or a `callable` variable is not allowed. Move the code into a `static` \
         method and spawn that method.",
    );

    // `E0804` is retired and is never reused: it refused a method entry that
    // declared a parameter. `rule:security/isolate-shares-nothing`'s binding lowers — the
    // entry's parameter names ride on the spawn symbol and
    // `nvs_stdlib::script` binds `args:`'s entries to them by name — so that
    // shape is the shape that runs, and a mismatch is the ordinary
    // named-argument error that rule names, raised at the spawn.

    /// A written reason that is not a string literal, at a member whose reason
    /// exists to be read by the next person —
    /// `rule:core-classes/html-to-source`'s second sentence, which `nvs_types::reasons` owns the roster of.
    ///
    /// **Only the half a compiler can answer.** The same sentence also refuses
    /// an *empty* reason, and that stays the member's own throw: emptiness is a
    /// property of the text, which the body reads whatever the site wrote, and
    /// a second refusal here would only make the throw unreachable.
    /// `nvs_types::reasons`' module doc is the home of the split.
    ///
    /// A `const` counts as written here, deliberately: it folds, so the
    /// text is still in the source and still greppable, and pulling a long
    /// justification out to a named constant is the shape this hatch wants
    /// rather than the one it refuses.
    pub const E_REASON_NOT_WRITTEN_DIRECTLY: Code = Code::new("E0805").card(
        "The reason passed to this method must be a string in quotes or a `const`. A computed \
         value is not allowed, because a reader must be able to see the reason in the code.",
    );

    /// A `Core\Db\…::queryAs<T>` whose `T` no row can be hydrated into —
    /// `rule:core-classes/db-column-types`'s type map, asked at the call
    /// rather than at a declaration.
    ///
    /// One code for the three ways a written `T` fails that question, because
    /// they are one rule read at one site: the type is a **list**
    /// (`array<Row>`, where § 4 already answers `Core\Db\Rows` of one), the
    /// class carries no `#[Db\Derive]` and so has no mapping at all, or it
    /// carries one whose mapped fields cannot fill its `constructor` — a
    /// property `#[Db\Field(skip: true)]` took off the mapping is the shape
    /// that reaches this, and `nvs_types::derive`'s own doc says why that
    /// refusal cannot be made where the class is declared.
    ///
    /// Deliberately not [`E_DERIVE_FIELD_NOT_CODEC_REACHABLE`], which is the
    /// declaration's own refusal and fires whether or not anything ever calls
    /// `queryAs`: this one is about a *call*, and the class it names may be
    /// perfectly well formed for every other purpose it has.
    pub const E_QUERY_AS_NOT_A_ROW_CLASS: Code = Code::new("E0806").card(
        "`queryAs<T>` cannot build this class from a database row. `T` must be one class and \
         not a list such as `array<Row>`, and the class needs `#[Db\\Derive]`. Every \
         `constructor` parameter of the class must be filled from a column.",
    );

    /// `callable(int)` — a `callable` type written with a parameter list and
    /// no return type, `rule:types/callable-signature`.
    ///
    /// The return type is mandatory because `callable(int)` says strictly less
    /// than bare `callable` while costing a second spelling of "unknown": the
    /// arguments would be proven where the call is written and the result
    /// would not, so the site keeps the dynamic path anyway. `void` and
    /// `never` are writable there as in any other return position, which is
    /// what the help names.
    pub const E_CALLABLE_TYPE_WITHOUT_RETURN: Code = Code::new("E0807").card(
        "A `callable` type with a parameter list needs a return type. Write \
         `callable(int): void` if it returns nothing.",
    );

    /// `fn ($n) => …` written where nothing says what `$n` holds —
    /// `rule:types/anonymous-function-parameter-inference`.
    ///
    /// An anonymous function's parameter may leave its type out, and then takes it
    /// from the position the function is written in. Only a written
    /// `callable(...)` signature is such a position: bare `callable` is the top
    /// of the lattice and names no parameter, and a signature shorter than the
    /// function names this one no type either. The parameter is checked as
    /// `mixed` afterwards so the body is still checked at all.
    pub const E_ANON_FN_PARAMETER_TYPE_NOT_INFERABLE: Code = Code::new("E0808").card(
        "This parameter of an anonymous function has no type, and the compiler cannot tell which type it should \
         have. Write the type, as in `fn (User $u) => $u->name`.",
    );

    /// `$f(1)` where `$f` is a `callable(int, string): R` — a call through a
    /// written signature passing a number of arguments the signature does not
    /// name, `rule:types/callable-signature`.
    ///
    /// The count is exact in this one direction only: the *value* may hold a
    /// callable of any arity up to the signature's
    /// (`rule:types/callable-arity`), and the runtime hands that callable only
    /// the leading arguments it declares — so a site passing fewer than the
    /// signature names can leave a parameter of the callable actually held
    /// unfilled, which is a fault below the language rather than a throw. A
    /// site passing more names a parameter the type does not have. Bare
    /// `callable` keeps no count at all: nothing there says what to compare
    /// against.
    pub const E_CALLABLE_CALL_ARITY: Code = Code::new("E0809").card(
        "This call passes a different number of arguments than the `callable` type lists. \
         Pass exactly the parameters the type names.",
    );

    /// `Core\Request::jsonAs<T>()` over a `T` holding a `string` or `bytes`
    /// property that is not written `tainted` —
    /// `rule:security/derived-codec-qualifiers`.
    ///
    /// A derived codec assigns a peer's octets straight into the declared
    /// property types, so the qualifier a field declares has to be the one the
    /// payload carries. The codec cannot ask that question: it is derived from
    /// a class that says nothing about where its documents come from, and the
    /// same class is legitimate over one the program built itself. The call
    /// that decodes is where the qualifier is statically known, so that is
    /// where it is asked.
    ///
    /// The message names the **property**, not the call, because writing
    /// `tainted` on that property is the whole fix and the site has nothing to
    /// change. Only `string` and `bytes` carry a qualifier
    /// (`rule:security/tainted-qualifier`), so a `T` of integers, decimals,
    /// enums or instants never reaches this.
    ///
    /// It is `jsonAs`'s alone rather than every decoder's: `Core\Json::decodeAs`
    /// takes its document through a plain `string` parameter, so a tainted
    /// argument is already refused where it is passed.
    pub const E_DECODED_FIELD_NOT_TAINTED: Code = Code::new("E0810").card(
        "A method such as `Core\\Request::jsonAs<T>()` or `Core\\Db\\Connection::queryAs<T>()` \
         fills this property with data from outside the program. A `string` or `bytes` \
         property must then be `tainted` (marked as coming from outside the program). Write \
         `tainted` on the property, as in `public tainted string $name`.",
    );

    /// `$x is void`, `$x is never` — a type no value inhabits, written where
    /// `is` asks whether a value holds one (`rule:types/type-test`, the second
    /// of its two refusals).
    ///
    /// Not a knowable answer this refuses but an unaskable question: `void` is
    /// what a function returns instead of a value and `never` is what one
    /// returns by not returning, so neither names a shape a subject could be
    /// carrying. `is` is total over every type a value can inhabit
    /// (`rule:types/type-test`), and these two are the boundary of that set rather than
    /// an exception inside it.
    pub const E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE: Code = Code::new("E0811").card(
        "No value can have the type `void` or `never`, so `$x is void` has no answer. Test \
         against a type that a value can have.",
    );

    // `E0812` (`E_TYPE_TEST_AGAINST_A_VALUE`) is **retired**, not reused.

    /// `$x is tainted string`, `$x is secret bytes` — a qualifier written where
    /// `is` takes a type (`rule:types/type-test`, the first of its two
    /// refusals).
    ///
    /// `tainted` and `secret` are checked once and erased before codegen
    /// (`rule:security/tainted-qualifier`, `rule:security/secret-qualifier`):
    /// no byte of a value says where it has been, so there is nothing for a
    /// run-time test to read. The answer is not merely knowable here, the way a
    /// test the declaration settles is — it is absent, which is why this is a
    /// refusal and a statically-true test is not.
    ///
    /// Numbered past its two siblings because `E0810` was already
    /// [`E_DECODED_FIELD_NOT_TAINTED`] when `rule:types/type-test` was written,
    /// and a code means one thing.
    pub const E_TYPE_TEST_AGAINST_A_QUALIFIER: Code = Code::new("E0813").card(
        "`tainted` and `secret` exist only while the program compiles, so `is` cannot test \
         them. Test the type without them, as in `$x is string`.",
    );

    /// A `catch` clause or arm naming `Core\Script\Finished`, the class
    /// `Core\Script::finish()` raises.
    ///
    /// The marker is a root of its own rather than a `Throwable`
    /// (`nvs_hir::errors::TREE`'s own docs are the mechanism), so an arm naming
    /// it already matches nothing at run time: the refusal is what keeps that
    /// from reading as a compiler bug. A site writing one has understood the
    /// ending backwards — a finish is not a failure a request recovers from,
    /// and the code that must run on the way out belongs in a `finally`, which
    /// the unwind runs, or in a `Core\Script::onExit` hook, which the ending
    /// fires.
    ///
    /// Separate from [`E_THROW_OPERAND_NOT_THROWABLE`], which refuses the
    /// *other* end of the same mistake and needs no case of its own: the marker
    /// descends from `Throwable` not at all, so `throw` already reports it as
    /// an operand outside the tree.
    pub const E_CATCH_ARM_NAMES_THE_FINISH_MARKER: Code = Code::new("E0814").card(
        "A `catch` cannot name `Core\\Script\\Finished`, because `Core\\Script::finish()` ends \
         the request and is not an error. Put code that must run at the end in a `finally` \
         block or a `Core\\Script::onExit` hook.",
    );

    /// Two of `Core\Http\Client`'s four body keys written at one call.
    ///
    /// `rule:http-server/an-outbound-request-carries-one-body`: which key a body
    /// was written under is what says how it is sent, so two of them is not a
    /// request with two bodies but a call site that has not decided what it is
    /// sending. Reportable while compiling for
    /// [`E_RETRY_WITHOUT_IDEMPOTENCY_KEY`]'s reason — `rule:core-api/shape-rules` R2 makes the bag
    /// an anonymous object written at the call site — and where the verb is dynamic the same question throws
    /// before the first attempt rather than after it.
    pub const E_TWO_REQUEST_BODIES: Code = Code::new("E0815").card(
        "This `Core\\Http\\Client` call has two body options, and a request has only one body. \
         Keep the one whose format the server expects.",
    );

    /// A body key written at a `Core\Http\Client` member whose verb carries no
    /// body.
    ///
    /// `rule:http-server/an-outbound-request-carries-one-body` again, at its
    /// other half: a `GET` and a `HEAD` ask a question, and octets attached to
    /// one are read by no server the request was worth making to. Distinct from
    /// [`E_TWO_REQUEST_BODIES`], where the keys are legal for the verb and it is
    /// their number that is the defect.
    pub const E_BODY_ON_A_BODYLESS_VERB: Code = Code::new("E0816").card(
        "A `GET` or `HEAD` request sends no body, so this body option is not allowed. Put the \
         value in the query part of the URL, or use a method that sends a body, such as \
         `post`.",
    );

    /// `contentType` written at a `Core\Http\Client` member with no `body`.
    ///
    /// The key types [`E_TWO_REQUEST_BODIES`]'s raw octets and nothing else: a
    /// `json`, a `form` and a `multipart` body each carry the media type their
    /// key already named, so a `contentType` beside one is either a second
    /// opinion the request will not send or a `body` the call site forgot to
    /// write.
    pub const E_CONTENT_TYPE_WITHOUT_A_BODY: Code = Code::new("E0817").card(
        "`contentType` gives the type of a raw `body`, and this call has no `body`. Add the \
         `body`, or delete `contentType`. A `json`, `form` or `multipart` body sets its own \
         type.",
    );

    /// `spawn script … with(on: …)` written as anything but the word
    /// `"worker"` or the word `"here"` —
    /// `rule:concurrency/on-worker-runs-the-child-on-another-core`.
    ///
    /// The placement decides which scheduler starts the child and is decided
    /// once, where the spawn is written, so `on:` takes a written word rather
    /// than a `string` value. A computed placement could only be read at run
    /// time, where a spelling nobody declared has no answer but a spawn that
    /// fails — and a program that asked for another core and got a failure, or
    /// silently got this one, is measuring something it did not ask for. Two
    /// spawns under an `if` say the same thing with the question asked here.
    ///
    /// A code of its own rather than the mismatch against a set of the two
    /// allowed values that `nvs_types::expr::isolate` could report instead: an
    /// expected type claims a position accepts values of that type, and this
    /// one accepts two words.
    pub const E_SPAWN_PLACEMENT_UNKNOWN: Code = Code::new("E0818").card(
        "`on:` in `spawn script` accepts only `\"worker\"` or `\"here\"`, written directly. \
         `\"worker\"` runs the script on another CPU core, and `\"here\"` runs it on this one.",
    );

    /// A capture declared at an enum subset admits two cases carrying one
    /// written value —
    /// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`.
    ///
    /// An alias is legal in the declaration (`rule:enums/declaration`: equality
    /// over an enum is value equality, so there is no identity for two names to
    /// collide on) and stays legal under the case-name spelling, where the names
    /// are still distinct. It is only the value spelling that cannot carry it:
    /// one segment would name two cases, and a match that picked either would be
    /// choosing for the program. Refused rather than first-wins, which is
    /// `rule:errors/ambiguous-input-refused`.
    pub const E_ROUTE_CAPTURE_CASES_SHARE_A_VALUE: Code = Code::new("E0819").card(
        "Two enum cases allowed by this route parameter have the same value, so one URL \
         segment would match both. Give the cases different values, or leave one of them out \
         of the parameter's type.",
    );

    /// A member that builds a written class out of a document — `decodeAs<T>`,
    /// `jsonAs<T>`, `queryAs<T>` on a request, `shapeAs<T>` — naming a class
    /// whose codec fills fewer constructor parameters than the constructor
    /// declares.
    ///
    /// `rule:core-classes/derive-property-list` sanctions `#[Json\Field(skip: true)]`
    /// on a property, and a skipped property that stayed a constructor
    /// parameter is the one way to leave a position no key fills. The class is
    /// well formed for every other purpose it has — it still encodes, and the
    /// program can still build one itself — so this is a refusal of the
    /// **call**, exactly as [`E_QUERY_AS_NOT_A_ROW_CLASS`] is at the row door
    /// and for the same reason: a condition about the class's own fields that
    /// cannot be moved to the declaration.
    ///
    /// The two doors keep separate codes because they refuse separate sets:
    /// a row is flat, so `E0806` also answers for a column map a document has
    /// no equivalent of.
    pub const E_DECODED_CLASS_NOT_CONSTRUCTIBLE: Code = Code::new("E0820").card(
        "This class cannot be built from a document, because a `constructor` parameter has no \
         field to fill it. This happens when a property has `#[Json\\Field(skip: true)]` and is \
         still a constructor parameter. Remove that parameter, or remove `skip: true`.",
    );
    /// A member that builds an instance out of a document names a class that
    /// declared no JSON codec at all.
    ///
    /// `rule:core-classes/derive-attribute` makes participation opt-in, and a
    /// class takes one of two doors into it: `#[Json\Derive]`, which generates
    /// the `Core\Json\Codec`, or the `fromJson` half of that interface written
    /// by hand. A class that took neither can never be read out of a document,
    /// and the call naming it is where that is known — so it is refused while
    /// compiling rather than on the first request that reaches the member.
    /// [`E_QUERY_AS_NOT_A_ROW_CLASS`] is the same refusal at the row door.
    ///
    /// Separate from [`E_DECODED_CLASS_NOT_CONSTRUCTIBLE`] because the edit is
    /// a different one: that code names a contract the class does have and a
    /// constructor position it leaves unfilled, and this one names a class with
    /// no contract to inspect.
    pub const E_DECODED_CLASS_HAS_NO_CODEC: Code = Code::new("E0821").card(
        "This class cannot be read from a document, because it has no JSON codec. Write \
         `#[Json\\Derive]` on the class, or write its `fromJson` method yourself.",
    );
    /// A written `return;` in a body whose declaration is not `void`.
    ///
    /// The defect [`E_MISSING_RETURN`] names, written out rather than reached
    /// by falling off the end: both exits lower to
    /// `nvs_ir::ir::Terminator::Return(None)`, so the caller of an `int`
    /// member reads a slot the callee never wrote. It is a code of its own
    /// because the edit is a different one — that code names a *path* an
    /// author has to find, and this one names the statement under the cursor.
    ///
    /// `void` is the whole of what makes one legal, which is why a
    /// constructor's is: it declares no return type at all, and
    /// [`E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE`] is the other half of that
    /// pair. A generator's body reaches this nowhere — `rule:iteration/one-way-only` leaves it
    /// no return value to produce, so `nvs_types::check` checks it against
    /// `void` and `return;` is its only stop.
    pub const E_VALUELESS_RETURN: Code = Code::new("E0822").card(
        "This `return;` gives no value, but the function declares a return type. Return a \
         value, or declare the return type `void`.",
    );
    /// A method declaration that writes no return type, where the constructor
    /// is the one declaration allowed to.
    ///
    /// `rule:types/declaration` makes the slot mandatory, `void` and `never`
    /// included, and the constructor is the exception that rule states: it
    /// hands back the instance rather than a value, which is the same fact
    /// [`E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE`] refuses a `return $x` in one
    /// for. Every other declaration owes a written type, because the
    /// declaration is what a caller reads and an omitted one means `mixed` —
    /// the widest type in the language, arrived at by leaving a slot empty
    /// rather than by writing it.
    ///
    /// Reported at a signature with no body too. An abstract method and an
    /// interface member are read by exactly the callers this is protecting, so
    /// the slot they leave empty is the one that costs most.
    /// [`E_ANON_FN_RETURN_TYPE_REQUIRED`] is this requirement at a
    /// block-bodied `fn`, and between them an expression-bodied anonymous function is the
    /// only callable whose return type may go unwritten — its body is one
    /// expression, which is its own answer.
    pub const E_METHOD_RETURN_TYPE_REQUIRED: Code = Code::new("E0823").card(
        "Every method needs a return type, except the `constructor`. Write `: void` if the \
         method returns nothing, or `: never` if it always throws.",
    );

    /// A `secret` value written into a container element or field whose own
    /// type does not carry the qualifier.
    ///
    /// `rule:security/secret-qualifier` puts the bit on a type, and a container
    /// keeps it only where the container's own element or field type spells it:
    /// an `array<T>` literal joins nothing, so `[$secret]` placed at an
    /// `array<mixed>` drops the qualifier at the bracket, and an anonymous object's
    /// inferred field type drops it at the assignment into a field declared
    /// something wider. Refused at the write, which is the last place the
    /// qualifier is visible — `rule:errors/record-transformations`'s redaction
    /// row reads a *declared* type, so a secret that reaches a record through a
    /// container has nothing left to be redacted off.
    ///
    /// `rule:security/secret-sinks-refuse` names three positions a credential
    /// legitimately reaches — a bound database parameter, a process argv, an
    /// outbound request — and each is written as an argument, so an argument
    /// list is where this steps aside and the call's own rules decide.
    pub const E_SECRET_INTO_CONTAINER: Code = Code::new("E0824").card(
        "A `secret` value cannot be stored in an array element or a field whose type is not \
         `secret`. Declare the type with `secret`, as in `array<secret string>` or \
         `{ name: secret string }`.",
    );

    /// A `Core\Test::double<T>` whose shape of answers leaves a method `T`
    /// requires unimplemented.
    ///
    /// [`E_INTERFACE_METHOD_MISSING`] is the same defect written as a class
    /// declaration, and the two keep separate codes because what the author
    /// does about them differs: a class declares the member, while a double
    /// either names a `method:` field whose callable answers it or is written as
    /// `Core\Test::partial` and delegates it to a real implementation.
    ///
    /// There is no answer for it to default to, which is why this is refused
    /// rather than filled in: `rule:testing/doubles` is strict because a double
    /// of `now(): Instant` has nothing legal to return.
    pub const E_DOUBLE_METHOD_MISSING: Code = Code::new("E0825").card(
        "This test double (a fake object for a test) does not implement a method that its \
         interface requires. Add a field with an anonymous function for that method, or use \
         `Core\\Test::partial` to take the method from a real implementation.",
    );
    /// A `Core\Test::double<T>`/`partial<T>` whose shape of answers names a
    /// method `T` does not declare.
    ///
    /// The other half of `rule:testing/doubles`' structural check, and the one
    /// that catches a renamed interface member: the shape's field would
    /// otherwise stand in for a method nothing calls, and the double would go
    /// on passing against a contract it no longer answers.
    ///
    /// A `private` interface method (`rule:classes/interface-private-methods`)
    /// is named by this too. It is an internal helper for that interface's own
    /// bodies rather than part of its contract, so no implementor defines it
    /// and no double may answer it either.
    pub const E_DOUBLE_METHOD_UNKNOWN: Code = Code::new("E0826").card(
        "This test double has a field for a method that its interface does not declare. \
         Delete the field, or check the spelling of the method name.",
    );
    /// A `Core\Test::double<T>`/`partial<T>` whose `T` names a declaration that
    /// is not an interface.
    ///
    /// `rule:testing/doubles` checks a shape of callables against an
    /// **interface** because that is the whole of what a double can answer: a
    /// class also carries state and method bodies, so a shape standing in for
    /// one would silently leave every property unset and every inherited body
    /// unreachable, and the members it did answer would be the ones a call site
    /// never dispatches through.
    ///
    /// [`E_PROGRAM_TYPE_ARG_NOT_A_CLASS_OR_INTERFACE`] is the nearest demand,
    /// at `rule:programs/implementing`'s enumeration, and it accepts a class:
    /// that one wants only the static type its answers carry, which a class
    /// gives too, and this one wants the contract an interface's declarations
    /// are.
    ///
    /// A type argument that is not a class or interface *name* at all takes
    /// [`E_TYPE_ARG_NOT_A_CLASS`] instead, so one mistake is still one
    /// diagnostic.
    pub const E_DOUBLE_TYPE_ARG_NOT_AN_INTERFACE: Code = Code::new("E0827").card(
        "`Core\\Test::double<T>` and `Core\\Test::partial<T>` need an interface as `T`, and \
         this is not an interface. Declare an interface with the methods you need, and use it \
         as `T`.",
    );
    /// A `Core\Test::double<T>`/`partial<T>` whose shape answers a method `T`
    /// does declare with a value that cannot stand in for it.
    ///
    /// The third question `rule:testing/doubles`' structural check asks, after
    /// the name is known to belong to the contract: a callable whose parameters
    /// refuse what the interface's own call sites pass, or whose result is not
    /// what they were promised, answers the method in name only. A field that
    /// is not a callable at all is named by this too — nothing else at the call
    /// asks what a field holds, since the declared parameter is `object`.
    ///
    /// A field typed bare `callable` is accepted: it carries no parameter list
    /// to compare (`rule:types/callable-values`), and the call it stands
    /// in for is checked one argument at a time by `nvs_runtime::callable`.
    pub const E_DOUBLE_METHOD_SIGNATURE: Code = Code::new("E0828").card(
        "The function given for this method of the test double does not fit the method's declaration. \
         It may have fewer parameters, but each parameter must accept what callers pass, and \
         its return type must match.",
    );

    /// A method reference names a method the class or interface it names does
    /// not declare.
    ///
    /// `rule:testing/interaction-after-the-fact`'s reference is compile-checked
    /// so that renaming a method updates or breaks the test rather than leaving
    /// it passing against a method that no longer exists — which is the whole
    /// reason the assertion takes `Mailer::send` rather than `"send"`.
    ///
    /// It is a code of its own rather than the undefined-member one because the
    /// spelling means nothing anywhere else: `Class::name` is a constant read
    /// everywhere but the argument positions a registry row wrote
    /// `nvs_stdlib::registry::CoreTy::MethodRef` at, and a reader who sees
    /// "no constant named `send`" there has been told about the wrong language.
    pub const E_METHOD_REF_UNDECLARED: Code = Code::new("E0829").card(
        "This method reference names a method that the class or interface does not declare. \
         Check the spelling, or update the test if the method was renamed.",
    );

    /// A position that takes a method reference was given something else.
    ///
    /// The value a reference folds to is a `string` — the method's own name —
    /// and `rule:testing/interaction-after-the-fact` is why a written one is
    /// refused anyway: `"send"` is a spelling nothing checks, so a typo in it
    /// reports as a call that never happened, at the end of a test, rather than
    /// where it was written. The reference is the whole point of the position.
    pub const E_METHOD_REF_REQUIRED: Code = Code::new("E0830").card(
        "This argument must be a method reference such as `Mailer::send`, not a string like \
         `\"send\"`. The compiler checks a reference, and it does not check a string.",
    );

    /// `static::NAME` inside a constant expression — a property or parameter
    /// default, an attribute payload.
    ///
    /// `rule:statements/static-is-a-member-modifier` makes a `static::` constant read a
    /// run-time one, bound to the class the call was made on, and a constant
    /// expression is folded once with no frame to read that class from. Refused
    /// by name rather than folded as `self::NAME`, which is what it silently
    /// was: the declaring class's value where a subclass's was meant.
    pub const E_STATIC_CONST_IN_CONSTANT_EXPRESSION: Code = Code::new("E0831").card(
        "`static::NAME` cannot be used in a default value or an attribute, because these are \
         fixed before any method is called. Write `self::NAME` instead.",
    );

    /// `static::NAME` where `NAME`'s value is not a scalar — an `array`
    /// constant, an object, a `null`, an integer outside `int`.
    ///
    /// The late-bound read answers from the called class's constant table,
    /// which carries the four scalar kinds and nothing else
    /// (`nvs_runtime::ConstantValue`), so there is no value to answer with.
    /// `self::NAME` and `Class::NAME` still inline such a constant.
    pub const E_STATIC_CONST_HAS_NO_SCALAR_VALUE: Code = Code::new("E0832").card(
        "`static::NAME` works only for a constant whose value is a `string`, `int`, `uint`, \
         `bool` or `float`. For any other constant, write `self::NAME` or `ClassName::NAME`.",
    );

    /// A class or interface redeclares a constant an ancestor already declares,
    /// at a type not assignable to the ancestor's.
    ///
    /// A `static::NAME` read is typed at the declaring class and lowered as a
    /// load at that type, and it answers whichever class the call was made on
    /// — so a redeclaration at another type would be read at the wrong
    /// representation, `secret` bit included. PHP 8.3 makes the same demand of
    /// a typed constant override.
    pub const E_CONST_REDECLARED_AT_ANOTHER_TYPE: Code = Code::new("E0833").card(
        "This class declares a constant again, with a type that does not fit the parent's \
         constant. Give it the parent's type, or a type that fits it.",
    );

    /// `static::` — a constant, a call, `::class`, `new static()` — inside an
    /// anonymous function's body.
    ///
    /// The called class is what the enclosing method's frame holds, and an
    /// anonymous function is lifted to a frame of its own that carries neither
    /// it nor a receiver, so there is nothing to bind `static` to. Refused
    /// where it is written.
    pub const E_STATIC_IN_ANON_FN: Code = Code::new("E0834").card(
        "`static::` cannot be used inside an anonymous function. Read the value into a variable \
         outside the function, and use that variable inside it.",
    );

    /// A call to an `abstract static` method on a class fixed while compiling
    /// — a written class name, or `self::`/`parent::` inside an anonymous function's body —
    /// where no class from that one upwards declares the method with a body.
    ///
    /// Such a call sets the class it dispatches on, so no subclass's override
    /// can be reached, and the dispatch would find no body to run. Refused
    /// where it is written, because at run time it is an internal error.
    pub const E_ABSTRACT_STATIC_CALLED: Code = Code::new("E0835").card(
        "This method is `abstract`, so it has no body, and this call names a class that does not \
         give it one. Call the method on a class that is not `abstract`. Inside a method, \
         `static::` calls the class the method was called on. Inside an anonymous function, read \
         the value into a variable outside the function, and use that variable inside it.",
    );

    /// A `#[Core\Path]` written anywhere but on a `string` parameter of a
    /// method — on a class, a property, a method itself, or a parameter whose
    /// declared type is not `string` or `?string`.
    ///
    /// `rule:programs/relative-paths-resolve-from-their-file`'s marker means one
    /// thing: a string literal passed to this parameter is a path, joined to
    /// the folder of the file that wrote it. Anywhere else there is no
    /// argument for it to resolve, so it is refused for the reason
    /// [`E_QUERY_WITHOUT_ROUTE`] refuses a stray `#[Query]`: a name the
    /// compiler knows, written where it does nothing, reads as a declaration
    /// that works.
    pub const E_PATH_MARKER_NOT_ON_A_STRING: Code = Code::new("E0836").card(
        "`#[Core\\Path]` works only on a method parameter whose type is `string` or `?string`. \
         Change the parameter's type to `string`, or delete `#[Core\\Path]`.",
    );

    /// A `Core\Path::thisDir($join)` whose `$join` is not a relative string
    /// literal — a variable, a constant, a concatenation or an absolute path.
    ///
    /// `rule:programs/relative-paths-resolve-from-their-file`'s fold replaces
    /// the call with a path while compiling, so its argument has to be known
    /// then. A value built at run time has a member that joins it already,
    /// and the help names it rather than letting the fold quietly fall back to
    /// a call.
    pub const E_PATH_THIS_DIR_JOIN_NOT_A_WRITTEN_RELATIVE_PATH: Code = Code::new("E0837").card(
        "`Core\\Path::thisDir` adds its argument to the folder while the program compiles, so \
         the argument must be a relative path written as a string in quotes, such as `'data'`. \
         For a path the program builds, write `Core\\Path::join(Core\\Path::thisDir(), $part)`.",
    );

    /// A `Core\Program::constructors<T, C>()` whose `C` is not a `callable`
    /// type naming its parameters, or names a return type other than `T`.
    ///
    /// `rule:programs/constructors`: each row's `make` is a callable taking
    /// `C`'s parameters and returning a new `T`, so `C` is what decides both
    /// what every constructor is checked against and what the caller may pass.
    /// Bare `callable` names no parameters to check, and a return type `T`
    /// does not satisfy is a callable the row could not hold.
    pub const E_PROGRAM_CONSTRUCTORS_TYPE_ARG_NOT_A_MAKER: Code = Code::new("E0838").card(
        "The second type argument of `Core\\Program::constructors` must be a function type with \
         its parameters and a return type that accepts the first type argument, for example \
         `constructors<Module, callable(Config): Module>()`.",
    );

    /// A class `Core\Program::constructors<T, C>()` lists has a constructor
    /// that `C`'s parameters cannot call.
    ///
    /// `rule:programs/constructors`: the row's `make` is
    /// `fn(<C's parameters>): T => new Class(<the same arguments>)`, so it is
    /// checked as that `new` would be where the call is written — too few
    /// parameters, too many, a type the constructor does not accept and a
    /// constructor that is not visible there. The class is named, because the
    /// fix is in that class or in `C`, and the ordinary error is a note.
    pub const E_PROGRAM_CONSTRUCTOR_DOES_NOT_FIT: Code = Code::new("E0839").card(
        "`Core\\Program::constructors` creates each class it finds with the parameters of its \
         function type, and this class's constructor does not accept them. Change the \
         constructor, or change the function type's parameters.",
    );

    /// A path argument that starts with a relative string literal and adds a
    /// value built at run time — `'data/' . $name` or `"data/{$name}"`.
    ///
    /// `rule:programs/relative-paths-resolve-from-their-file` joins only a
    /// whole string literal to the folder of its file. A path built this way stays
    /// relative when the program runs, and every path door throws for a
    /// relative path, so the call can never succeed. The help names
    /// `Core\Path::thisDir` and `Core\Path::join`, which build the same path
    /// from the file's folder.
    pub const E_PATH_BUILT_FROM_A_WRITTEN_RELATIVE_PATH: Code = Code::new("E0840").card(
        "This path starts with a relative path, and the rest is added while the program runs. \
         The result is still a relative path, so the call always throws an error. Start the path \
         with the folder of this file: `Core\\Path::thisDir('data') . '/' . $name`, or \
         `Core\\Path::join(Core\\Path::thisDir('data'), $name)`.",
    );
    /// A type argument written where the member needs an **enum** —
    /// `Core\Router\Match::accessAs<App\Policy>` over a class. The members that
    /// do are `nvs_stdlib::registry::WRITTEN_ENUM_MEMBERS`, and each of them
    /// answers a case of the written enum, which only an enum has
    /// (`rule:security/access-is-checked-for-presence-not-meaning`).
    pub const E_TYPE_ARG_NOT_AN_ENUM: Code = Code::new("E0841").card(
        "This method returns a case of the enum you write as its type argument, so the type \
         argument must be an enum. Write an enum such as `Core\\Audience` or `App\\Role`.",
    );
    /// A call that changes the response head — a status, a header, a redirect,
    /// a cookie, a body method or `Core\Session::regenerate` — written inside
    /// the callable passed to `Core\Html::later`. `rule:core-classes/html-later`
    /// makes the same call throw `LogicError` at run time, and this is its
    /// compile-time half, where the checker sees the anonymous function written in place.
    pub const E_LATER_CHANGES_THE_RESPONSE_HEAD: Code = Code::new("E0842").card(
        "The function passed to `Core\\Html::later` fills one part of the page. It cannot change the status, \
         the headers, the cookies or the session of the response. Make this call in the main \
         script, before or after `Core\\Html::later`.",
    );
    /// A `readonly` property that declares a `get` or `set` hook. PHP 8.4
    /// refuses the same combination. A hook runs code at every read or write,
    /// so the property would no longer be the one value its constructor wrote,
    /// and a null test could not narrow it.
    pub const E_READONLY_PROPERTY_WITH_HOOK: Code = Code::new("E0843").card(
        "A `readonly` property cannot have a `get` or `set` hook. Remove `readonly`, or remove \
         the hooks and set the value in the constructor.",
    );
    /// `#[Core\Deprecated]` written somewhere
    /// `rule:attributes/a-deprecation-names-its-replacement-as-code` does not
    /// attach it: a property hook, or a hook's parameter. A use reaches the
    /// property and never one of its hooks, so nothing could warn there.
    pub const E_DEPRECATED_ON_A_PROPERTY_HOOK: Code = Code::new("E0844").card(
        "`#[Core\\Deprecated]` cannot be written on a property hook or its parameter. Write it \
         above the property instead.",
    );
    /// A `#[Core\Deprecated]` template that is not one expression, or not one
    /// type name on a class, an interface or an enum
    /// (`rule:attributes/a-deprecation-names-its-replacement-as-code`).
    pub const E_DEPRECATION_TEMPLATE_DOES_NOT_PARSE: Code = Code::new("E0845").card(
        "The `replace` or `construct` text of `#[Core\\Deprecated]` must be one Novis \
         expression. On a class, an interface or an enum, `replace` is one type name. On a \
         parameter, it is one named argument, such as `limit: $count`.",
    );
    /// A `#[Core\Deprecated]` template that parses and does not compile in the
    /// scope of the declaration it is written on.
    pub const E_DEPRECATION_TEMPLATE_DOES_NOT_COMPILE: Code = Code::new("E0846").card(
        "The `replace` or `construct` text of `#[Core\\Deprecated]` must compile where the \
         declaration is written. It can use the parameters of the method, and `$this` in an \
         instance member.",
    );
    /// A `#[Core\Deprecated]` template whose type is not assignable to the
    /// type of the member it replaces.
    pub const E_DEPRECATION_TEMPLATE_DOES_NOT_FIT: Code = Code::new("E0847").card(
        "The `replace` text of `#[Core\\Deprecated]` must have a type that fits the member it \
         replaces. A method's replacement fits its return type, and a property's replacement \
         fits the property's type.",
    );
    /// A `#[Core\Deprecated]` template that names a member less visible than
    /// the declaration it is written on.
    pub const E_DEPRECATION_TEMPLATE_LESS_VISIBLE: Code = Code::new("E0848").card(
        "The `replace` text of `#[Core\\Deprecated]` replaces code that other classes write. \
         It cannot use a method, property or constructor that is less visible than the \
         deprecated member.",
    );
    /// A `#[Core\Deprecated]` template that names something which is itself
    /// deprecated, so applying its fix would leave a second warning.
    pub const E_DEPRECATION_TEMPLATE_NAMES_A_DEPRECATED: Code = Code::new("E0849").card(
        "The `replace` text of `#[Core\\Deprecated]` cannot use anything that is also \
         deprecated. Write the final replacement, so one fix is enough.",
    );
    /// A deprecated class whose `replace` names a class that does not declare
    /// every public member the deprecated class declares.
    pub const E_DEPRECATION_REPLACEMENT_MISSES_A_MEMBER: Code = Code::new("E0850").card(
        "The class named by `replace` must declare every public method, property and \
         constant of the deprecated class, with a type that fits. Otherwise the code that \
         used the old class does not compile after the fix.",
    );
    /// Text out of `mixed` written into a type that does not say `tainted`:
    /// a shape target of `as` or `is` over a `mixed` or `object` operand, or a
    /// `foreach` binding over a `mixed` subject
    /// (`rule:security/taint-propagation`). `as string` adds the qualifier by
    /// itself; a shape and a binding are declarations, so the program writes it.
    pub const E_UNCHECKED_TEXT_NOT_TAINTED: Code = Code::new("E0851").card(
        "Text that comes out of a `mixed` value is `tainted` (marked as coming from outside \
         the program). The same is true for an object field that is `mixed` or tainted, and \
         for the keys of an array whose values are tainted or `mixed`. A shape or a variable \
         that gets this text must say so. Write `tainted string`, or write `tainted` before a \
         shape, as in `$data as tainted { name: string }`.",
    );
    /// A call to a member on `nvs_stdlib::registry::MUST_USE_MEMBERS` written
    /// as a statement of its own, so its result is thrown away
    /// (`rule:core-classes/process-spawn`). The roster row carries the help
    /// line, which says what the result is for. A result assigned to a
    /// variable that is never read is not this error.
    pub const E_RESULT_NOT_USED: Code = Code::new("E0852").card(
        "The result of this call is not used. The program needs the result to finish what the \
         call started. Keep the result in a variable and use it. For example, keep the handle \
         that `Core\\Process::spawn` returns, and call `wait()` or `kill()` on it.",
    );
    /// An `isinit(...)` operand that is not a `lateinit` property read with
    /// `->`: a property that always has a value, a variable, or any other
    /// expression. See `rule:classes/isinit-reads-nothing`.
    pub const E_ISINIT_NOT_LATEINIT: Code = Code::new("E0853").card(
        "`isinit` needs a `lateinit` property, written as `$object->property`. Any other \
         property always has a value, so the test is not needed.",
    );
    /// An `isinit(...)` operand whose receiver has no class known at compile
    /// time, such as `object` or a shape, so no slot can be named. See
    /// `rule:classes/isinit-reads-nothing`.
    pub const E_ISINIT_UNKNOWN_CLASS: Code = Code::new("E0854").card(
        "`isinit` needs an object of a known class. Test the class first with `is`, for \
         example `if ($value is Post)`.",
    );
    /// A `#[Route]` method that is not `static`, on a class
    /// `Core\Router::dispatch` cannot build with no argument: an `abstract`
    /// class, a constructor with a required parameter, or an inherited
    /// `private` constructor. `rule:routing/a-gate-answers-around-every-handler`.
    /// Reported once per method, at its first `#[Route]`.
    pub const E_ROUTE_HANDLER_NOT_CONSTRUCTIBLE: Code = Code::new("E0855").card(
        "`Core\\Router::dispatch` creates an object of the class to call a `#[Route]` method \
         that is not `static`. It passes no arguments to the constructor. So the class must not \
         be `abstract`, and every parameter of its constructor needs a default value. A \
         constructor the class inherits must not be `private`. To fix it, make the method \
         `static`.",
    );
    /// A call of a built-in component's manifest class, or a reference to one of its methods,
    /// outside that component's own Novis source
    /// (`rule:packaging/a-components-engine-class-is-internal`). The help is the class's card,
    /// which names the public class to use.
    pub const E_ENGINE_CALL: Code = Code::new("E0856").card(
        "This class is internal to a built-in component. Only the component's own classes call \
         it. Use the public class that the help line names. For example, call \
         `Novis\\Image\\Image::info` and not `Novis\\Image\\Codec::info`.",
    );
    /// A call that writes the response — a status, a header, a redirect, a
    /// cookie, a body method, `Core\Response::slotted` or
    /// `Core\Session::regenerate` — written inside the callable passed to
    /// `Core\Task::afterResponse`. The same call throws `LogicError` at run
    /// time (`rule:concurrency/deferred-work-cannot-write-the-response`), and
    /// this is its compile-time half, where the checker sees the anonymous
    /// function written in place.
    pub const E_DEFERRED_WRITES_THE_RESPONSE: Code = Code::new("E0857").card(
        "The function passed to `Core\\Task::afterResponse` runs after the response was sent. \
         It cannot change the status, the headers, the cookies, the body or the session of \
         the response. Make this call in the main script, before `Core\\Task::afterResponse`.",
    );

    /// `clone` of a call to a generator — `rule:iteration/generator-stays-in-one-isolate`.
    /// A generator's fields encode a suspended point in its body, and a second
    /// copy resuming that point has no meaning.
    ///
    /// Raised only where the operand is a call whose declaration holds a
    /// `yield`, because that is the one place the checker knows it is a
    /// generator: a generator types as `Iterator<T>`, which a program's own
    /// classes implement too. Every other path to a generator throws at run time.
    pub const E_CLONE_OF_A_GENERATOR: Code = Code::new("E0858").card(
        "`clone` does not work with a generator. A generator is paused inside its function, \
         and a copy cannot continue from that point. Call the generator function again to get \
         a new generator that starts from the beginning.",
    );
    /// A method a class does not declare, which two of its `implements`
    /// clauses each supply — two interface defaults, or a default and a `by`
    /// delegate, or two delegates. See `rule:classes/member-conflict-is-an-error`.
    /// A superclass's concrete method is not a source: `extends` answers first.
    pub const E_INTERFACE_MEMBER_CONFLICT: Code = Code::new("E0859").card(
        "This class gets a method from two of its interfaces, and it does not declare the method \
         itself. Declare the method in the class. Its body can call one interface's default with \
         `InterfaceName::method()`, or a delegate with `$this->property->method()`.",
    );
    /// `string<K>` whose `K` is not a `property<T>`, a `propertyPath<T>` or a
    /// set of string values — `rule:types/string-holes`. Reported only where
    /// the argument lowered without a complaint of its own, as
    /// [`E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS`] is.
    pub const E_STRING_HOLES_ARGUMENT: Code = Code::new("E0860").card(
        "In `string<K>`, `K` says what each `{…}` hole may contain. It must be \
         `property<T>`, `propertyPath<T>` or a set of strings such as `'asc'|'desc'`.",
    );
    /// A string with holes whose text is not one: an empty hole `{}`, a `{`
    /// with no `}` after it, a `}` that closes no hole, or a hole whose name is
    /// not one of a set of allowed strings — `rule:types/string-holes`. A
    /// hole naming no property is [`E_UNKNOWN_MEMBER`] instead.
    pub const E_STRING_HOLE_MALFORMED: Code = Code::new("E0861").card(
        "In a `string<K>`, each `{name}` is a hole. A hole cannot be empty, and every `{` needs \
         a `}`. Write `{{` for a `{` and `}}` for a `}` that are only text.",
    );
    /// A `#[BeforeClass]`, `#[AfterClass]` or `#[AfterEachTest]` method the
    /// runner could not call as its rule says — `rule:testing/class-hooks`
    /// and `rule:testing/after-each-test`.
    ///
    /// The two class hooks are `public static` and the per-test one is a
    /// `public` instance method, and all three return `void` and have a body.
    /// `#[BeforeClass]` takes no parameter, `#[AfterClass]` at most one
    /// `Core\Test\Summary` and `#[AfterEachTest]` at most one
    /// `Core\Test\Outcome`. A method carrying two hook attributes, or a hook
    /// attribute beside `#[Test]` or `#[Fixture]`, is one method claiming to be
    /// two things. One code for all three, because the help each refusal
    /// gives names the hook it is about. A second hook of one kind is
    /// [`E_DUPLICATE_DECLARATION`].
    pub const E_TEST_HOOK_SHAPE: Code = Code::new("E0862").card(
        "A `#[BeforeClass]` or `#[AfterClass]` method must be `public static`. An \
         `#[AfterEachTest]` method must be `public` and not `static`. All three return `void`. \
         `#[AfterClass]` may take one `Core\\Test\\Summary` and `#[AfterEachTest]` one \
         `Core\\Test\\Outcome`.",
    );
    /// A `#[BeforeClass]` or `#[AfterClass]` method whose own body writes a
    /// static property — `rule:testing/class-hooks`. Every test runs in its
    /// own isolate and reads the static's declared initial value, so the write
    /// would look like setup and set up nothing.
    pub const E_CLASS_HOOK_WRITES_STATIC: Code = Code::new("E0863").card(
        "A `#[BeforeClass]` or `#[AfterClass]` method cannot write a static property. Each test \
         runs on its own and reads the value the property is declared with. Return the value \
         from a `#[Fixture]` method instead.",
    );
    /// A `catch` clause or arm naming a type no `Throwable` can be — a scalar,
    /// an array, an enum, or a class outside the `Throwable` tree —
    /// `rule:errors/throwable-hierarchy`. Such a clause never runs.
    ///
    /// An interface is allowed, since a class under `Throwable` may implement
    /// it, and so are `object` and `mixed`. [`E_THROW_OPERAND_NOT_THROWABLE`]
    /// is the same check at `throw`.
    pub const E_CATCH_TYPE_NOT_THROWABLE: Code = Code::new("E0864").card(
        "A `catch` works only with `Throwable`, a class that extends it, or an interface. This \
         type is none of them, so the `catch` never runs. Name the error class you expect, such \
         as `RuntimeError`.",
    );
    /// A `Core\Test::assertThrows` whose second argument is not a `C::class`
    /// written at the call, or names a class outside the `Throwable` tree —
    /// `rule:testing/assertions-are-typed`. A string or a variable would find
    /// a misspelt name only when the test runs, and a class outside the tree
    /// is never thrown, so the assertion could never pass.
    pub const E_ASSERT_THROWS_CLASS: Code = Code::new("E0865").card(
        "The second argument of `Core\\Test::assertThrows` must be written as `C::class`. `C` \
         must be `Throwable` or a class that extends it, such as `RuntimeError::class`. A string \
         or a variable is not allowed.",
    );
    /// A class declares an instance property again that a superclass already
    /// declares, at another type or with `readonly` on either declaration —
    /// `rule:classes/a-redeclared-property-keeps-its-parents-type`.
    ///
    /// Both declarations name one slot, and the parent's code reads and writes
    /// it at the parent's type. A narrower type lets the parent store a value
    /// the subclass reads as another class, and a wider one the reverse. A
    /// `readonly` property would be written by both constructors.
    /// [`E_CONST_REDECLARED_AT_ANOTHER_TYPE`] is the same hazard for a constant.
    pub const E_PROPERTY_REDECLARED_AT_ANOTHER_TYPE: Code = Code::new("E0866").card(
        "This class declares a property again that its parent class already declares. Both \
         classes use the same property, so it must have the parent's type. A `readonly` \
         property cannot be declared again. Use the parent's type, or give the property another \
         name.",
    );
    /// `$c is class<Dog>` — a class reference type written where `is` takes
    /// a type (`rule:types/type-test`'s refusals). A class reference carries
    /// no tag of its own that says which `T` it was declared with, so the
    /// test has no run-time row. `$c as ?class<Dog>` is the conversion that
    /// checks the class it names.
    pub const E_TYPE_TEST_AGAINST_A_CLASS_REFERENCE: Code = Code::new("E0867").card(
        "`is` cannot test for a class reference type such as `class<Dog>`. To check which class \
         a class reference names, write `$c as ?class<Dog>`. The result is `null` when the class \
         is not `Dog` or a class that extends it.",
    );
    /// A `match` arm whose body is a call that returns `void`
    /// (`rule:expressions/a-match-has-at-least-one-arm`). A `match` is an expression,
    /// and an arm is its value, so an arm with none has nothing to give,
    /// whether the `match` is used or written as a statement.
    pub const E_VOID_MATCH_ARM: Code = Code::new("E0868").card(
        "This `match` arm calls a function that returns `void`, so the arm has no value. Every \
         arm of a `match` gives a value. Use an `if` or a `switch` to run a statement for each \
         case, or return a value from the function.",
    );
    /// `$p->x` or `$p?->x` where `x` is a `static` property
    /// (`rule:statements/static-is-a-member-modifier`). A static property has
    /// one storage for the class, and an instance has no slot for it.
    pub const E_STATIC_PROPERTY_THROUGH_INSTANCE: Code = Code::new("E0869").card(
        "This property is `static`, so it belongs to the class and not to an object. Read and \
         write it with the class name, as in `Counter::$count`, or with `self::$count` inside the \
         class.",
    );
    /// `new E()` or `E::constructor(...)` where `E` is an enum
    /// (`rule:enums/no-class-machinery`). A case is an integer constant, so
    /// an enum has no object form and no constructor to call or name.
    pub const E_ENUM_HAS_NO_CONSTRUCTOR: Code = Code::new("E0870").card(
        "An enum has no constructor, so `new` cannot create a value of an enum. A value of an \
         enum is one of its cases. Write a case as `Suit::Hearts`.",
    );
    /// A user attribute written on an enum, an enum case or a class constant
    /// (`rule:attributes/attach-sites-and-forms`). `Core\Attributes` reads
    /// the attributes of a class, a method, a property and a parameter, and
    /// nothing reads one written on these three.
    pub const E_ATTRIBUTE_NO_RETRIEVAL_REACHES: Code = Code::new("E0871").card(
        "An attribute cannot be written on an enum, an enum case or a class constant. \
         `Core\\Attributes` cannot read an attribute there. Write it on a class, a method, a \
         property or a parameter.",
    );
    /// A `get` or `set` hook on a `static` property
    /// (`rule:classes/property-hooks`). A hook runs at a read or a write
    /// through an object, and a static property is never reached through one.
    pub const E_STATIC_PROPERTY_WITH_HOOK: Code = Code::new("E0872").card(
        "A `static` property cannot have a `get` or `set` hook. A hook runs when a property of \
         an object is read or written. Remove `static`, or remove the hooks.",
    );
    /// Two fields of one `#[Json\Derive]` or `#[Db\Derive]` class with the
    /// same wire key (`rule:core-classes/derive-property-list`). The key is a
    /// field's identity, so the encoder would write it twice and the decoder
    /// would fill both properties from one value.
    pub const E_DERIVE_DUPLICATE_KEY: Code = Code::new("E0873").card(
        "Two properties of a class with `#[Json\\Derive]` or `#[Db\\Derive]` have the same key. \
         Each property needs its own key. Change one key with `#[Json\\Field(name: \"...\")]` or \
         `#[Db\\Field(name: \"...\")]`, or leave one property out with `skip: true`.",
    );
    /// A compiler-recognized attribute on a declaration where the compiler
    /// builds nothing from it (`rule:attributes/attach-sites-and-forms`), such
    /// as `#[Core\Route]` on a property.
    pub const E_ATTRIBUTE_WRONG_SITE: Code = Code::new("E0874").card(
        "This attribute cannot be written on this declaration. The compiler uses it only on \
         some declarations, such as `#[Core\\Route]` on a method. Move the attribute, or \
         remove it.",
    );
    /// An assignment to `$this`, plain or compound
    /// (`rule:classes/this-is-not-assignable`). `$this` is the object the
    /// method was called on, and a write to it would change only the local
    /// name, never that object.
    pub const E_THIS_ASSIGNED: Code = Code::new("E0875").card(
        "`$this` cannot be assigned. `$this` is always the object the method was called on. \
         To work with another object, declare a new variable for it.",
    );
    /// A second `default` arm in a `match`, or a second `default:` label in
    /// a `switch` (`rule:expressions/a-match-has-at-least-one-arm`). The first one
    /// runs for every value no other arm matches, so the second never runs.
    pub const E_SECOND_DEFAULT: Code = Code::new("E0876").card(
        "A `match` or a `switch` can have only one `default`. The first `default` runs for \
         every value that no other arm matches, so a second `default` never runs. Remove one \
         of them.",
    );

    /// A `...` or a default on a `fn` anonymous function's parameter
    /// (`rule:types/anonymous-function`). A call through `callable` passes
    /// one argument for each declared parameter, so a variadic parameter never
    /// collects a list, and a default is never used.
    pub const E_ANON_FN_PARAM_SHAPE: Code = Code::new("E0877").card(
        "A parameter of an anonymous function cannot have `...` or a default value. A call \
         to an anonymous function passes exactly one argument for each parameter. Remove the \
         `...` or the default. To pass many values, declare an `array<T>` parameter.",
    );
    /// A `Core\Task::all` field whose function declares a parameter
    /// (`rule:concurrency/all-answers-a-typed-shape`). Each field's function
    /// is called with no arguments.
    pub const E_TASK_FIELD_TAKES_PARAMETERS: Code = Code::new("E0878").card(
        "`Core\\Task::all` calls the function in each field with no arguments, so the function \
         cannot have parameters. Remove the parameters. The function can read a variable from \
         the code around it, so a value it needs can come from there.",
    );
    /// A `#[Command]` `name:` or an `#[Option]` `long:` that is empty or has
    /// white space in it, or a `short:` that is not one letter or digit
    /// (`rule:tooling/commands-are-compiled`). No command line can type one.
    pub const E_COMMAND_SPELLING: Code = Code::new("E0879").card(
        "A command name and a `long:` option name cannot be empty, and they cannot contain a \
         space. A `short:` option name is one letter or one digit. Change the name, so that a \
         person can type it on a command line.",
    );
    /// A `#[Test]` `skip:` or `because:` that is empty, or a `retries:` below
    /// 1 (`rule:testing/runner-is-strict`). The first states no reason, and the
    /// second retries nothing.
    pub const E_TEST_OPTION_VALUE: Code = Code::new("E0880").card(
        "A `skip:` or `because:` in `#[Test]` must give a reason, so it cannot be empty. \
         `retries:` is how many more times a failed test runs, so it must be 1 or more. Write a \
         reason, or a number of 1 or more.",
    );
    /// A property or a method reached through a union of classes that the
    /// classes do not all share: one class lacks it, two give it different
    /// types or signatures, one hides it from the access, or a write meets a
    /// class where the property is not writable
    /// (`rule:types/a-member-every-class-of-a-union-declares`).
    pub const E_UNION_MEMBER_NOT_SHARED: Code = Code::new("E0881").card(
        "You can use a property or a method through a union of classes, such as `Box|Circle`, \
         only when every class in the union has it. The type must be the same in every class, \
         and the member must be visible here. A write also needs the property to be writable \
         in every class. Otherwise, test the value with `is` first, and use the member of that \
         one class.",
    );

    /// `covariant` written where it narrows nothing: on a parameter of a
    /// method that overrides nothing, of an anonymous function or of a
    /// constructor that does not promote it, on a static or a `readonly`
    /// property, or on a type no narrower than the topmost declaration's
    /// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`).
    pub const E_COVARIANT_MISPLACED: Code = Code::new("E0882").card(
        "`covariant` lets a method parameter or a property in a subclass have a narrower type \
         than the parent class declares. Each value is then tested when it enters. You can \
         write it only on a parameter of a method that replaces a parent's method, and on a \
         property that a parent class declares. The type must also be narrower than the \
         parent's type. Remove `covariant` everywhere else.",
    );

    /// A `covariant` type whose test is not constant time: an `array<T>`, a
    /// shape, a callable, a `class<T>`, a `property<T>` or a generic class
    /// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`).
    pub const E_COVARIANT_TYPE_NOT_TESTABLE: Code = Code::new("E0883").card(
        "A `covariant` type is tested each time a value enters, and this test must be fast. \
         So the type must be a class, an interface, an enum, a scalar, `null`, or a union of \
         these. An array, a shape and a callable are not allowed, because the test has to look \
         at every element. Use the parent's type instead, and test the value in the method body.",
    );

    /// A default the topmost declaration gives a parameter that the
    /// override's `covariant` type does not accept, which a call through the
    /// ancestor passes
    /// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`).
    pub const E_COVARIANT_DEFAULT_REJECTED: Code = Code::new("E0884").card(
        "A call through the parent class uses the default value that the parent declares. A \
         `covariant` parameter must accept that value. Otherwise, every call that leaves out \
         the argument throws an error. Use a type that includes the default, or change the \
         default in the parent class.",
    );

    /// `constructor` called other than by `new` and by one
    /// `parent::constructor(...)` on each path through a subclass's
    /// constructor (`rule:classes/constructor-is-a-method-named-constructor`).
    /// A second run would rewrite `readonly` properties and every type a test
    /// of one narrowed.
    pub const E_CONSTRUCTOR_CALLED_DIRECTLY: Code = Code::new("E0885").card(
        "A `constructor` runs when `new` creates an object. A subclass's constructor also calls \
         `parent::constructor(...)` once. No other code can call a constructor. Create a new \
         object with `new` instead.",
    );

    /// An `#[Api(security: [...])]` entry that no `#[Api\Scheme]` in the
    /// program declares (`rule:routing/a-security-scheme-is-declared-in-code`).
    /// The document would name a scheme it does not define.
    pub const E_API_SECURITY_UNDECLARED: Code = Code::new("E0886").card(
        "A `security` entry in `#[Api]` names a security scheme that the program does not \
         declare. Declare it with `#[Api\\Scheme(name: ...)]` on a class, or use one of the \
         names the message lists.",
    );

    /// Two `#[Api\Scheme]` declarations of one `name`
    /// (`rule:routing/a-security-scheme-is-declared-in-code`):
    /// `components.securitySchemes` is one map per document.
    pub const E_API_SCHEME_DECLARED_TWICE: Code = Code::new("E0887").card(
        "Two `#[Api\\Scheme]` attributes declare the same `name`. The OpenAPI file has one \
         entry for each name. Rename one of the schemes, or delete one.",
    );

    /// An `#[Api\Scheme]` field its `type` does not take, a required field
    /// left out, or an OAuth2 flow that is not one of the four or lacks the
    /// URL its kind needs (`rule:routing/a-security-scheme-is-declared-in-code`).
    pub const E_API_SCHEME_FIELD: Code = Code::new("E0888").card(
        "This `#[Api\\Scheme]` has a field that its `type` does not use, or it is missing a \
         field that its `type` needs. The message names the field. Each `type` has its own \
         list of fields.",
    );

    /// A call into test code from code that is not a test
    /// (`rule:testing/tests-never-reach-a-build`). `nvs run` and `nvs build` compile no test
    /// code, so the same program would call a method that one build has and the other does not.
    /// Refused at compile time, under every command, so a program compiles the same way for
    /// `nvs run` and for `nvs test`.
    pub const E_CALL_INTO_TEST_CODE: Code = Code::new("E0889").card(
        "A method with `#[Test]`, `#[Property]`, `#[Fixture]`, `#[BeforeClass]`, \
         `#[AfterClass]` or `#[AfterEachTest]` is test code. Only `nvs test` compiles it, so only other test code \
         can call it. Move the shared steps into a method without these attributes, and call \
         that method from both places.",
    );

    /// A `#[Property]` parameter whose declared type has no generator, a `gen:` entry that
    /// names no parameter, or a `gen:` range that does not fit the parameter's type
    /// (`rule:testing/property-testing`). The runner would have no values to call the method with.
    pub const E_PROPERTY_PARAMETER: Code = Code::new("E0890").card(
        "A `#[Property]` method gets random values for its parameters. Each value comes from \
         the parameter's type. Some types have no random values, such as a function or an \
         interface, so change the parameter's type. A `gen:` entry names a parameter and gives \
         a range, such as `gen: { age: \"0..120\" }`. The message says which part does not fit.",
    );
}
