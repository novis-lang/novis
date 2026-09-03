# The ground rules

One sentence per rule, and the link to the file that owns it. This is the index of everything a decision
has already settled — it is not itself normative. **The linked ADR's body is the rule**; this page exists
so you can find out that a rule exists without reading ninety ADRs.

It lives here rather than in [AGENTS.md](../../AGENTS.md) because it grows a bullet per ADR, and AGENTS.md
is charged to every agent before it does anything. Nothing reads this file in full: `python
tools/brief.py --where <keyword>` prints the rows of [README.md](README.md)'s routing table that match a
topic, and `python tools/orient.py` prints only the rules a goal's `[context]` manifest names.

**If you need more than a sentence, it belongs in the ADR.** The linked file holds the mechanism, the exact
spellings rejected, and the reasoning.

## Implementation invariants

- **Novis is built first for platforms that run code, and data, they do not control** — so the framework and
  the dependency story outrank new breadth, the pitch is isolation and qualifiers rather than speed, and no
  document may claim PHP compatibility ([0080](0080-the-audience-nvs-is-built-for.md)).
- **Memory is spent to buy security, semantics, latency and simplicity, in that order** — and never to
  buy a leak: what is spent stays attributable to a request, under an enforceable cap, and O(in-flight)
  rather than O(requests served) ([0004](0004-memory-for-simplicity.md)).
- **`unsafe` is forbidden workspace-wide**; only `nvs-runtime`, `nvs-codegen`, `nvs-stdlib` and
  `benches/abi-probe` opt down to `deny` with narrow, reasoned allows ([Cargo.toml](../../Cargo.toml)).
- **Nothing unwinds through a JIT frame** — every call returns a checked status
  ([0002](0002-error-propagation.md)).
- **Pure-Rust dependencies by default**, enforced by [deny.toml](../../deny.toml) in CI; a C dependency must
  pass [0051](0051-standard-library-tiers.md) § 4's two questions.
- **Domain logic is an existing first-class Rust crate; compiler passes and scheduler primitives are ours** —
  anything with an external specification is a dependency, and if no crate exists the feature is not built
  ([README.md](README.md) § *Decisions taken at project start*).
- **SIMD comes from a dependency that dispatches at runtime, never from an intrinsic we wrote or a
  `target-cpu` flag, and the JIT emits scalar code by design**
  ([README.md](README.md) § *Decisions taken at project start*).
- **Architecture assumptions are tested, not remembered** — [benches/abi-probe/](../../benches/abi-probe/)
  guards them on every CI run; if one fails, revisit the ADR it points at rather than the threshold.
- **Every third-party notice Novis owes is generated, committed and embedded in the binary**
  ([0065](0065-third-party-attribution-and-nvs-info.md)).
- **Dependencies stay current, and from 0.1.0 a break in one is absorbed rather than forwarded to Novis
  programs** — until then, update anything freely; the sweep is a pass the *user* fires, never an agent
  ([0068](0068-dependency-currency-and-the-version-contract.md),
  [docs/agent/dependency-update.md](../agent/dependency-update.md)).

## The type system

- **Nothing is untyped, and no type ever changes by itself** — every binding declares a type, `mixed` is
  the one unchecked position, and there is no `resource` type
  ([0007](0007-explicit-type-system.md)).
- **`var $name = expr;` infers a local's type from its initializer and fixes it forever** — a bare
  array-literal initializer is the one shape it refuses
  ([0037](0037-var-local-type-inference.md)).
- **A `for` header's init clause is one typed local declaration or a list of expressions, never both**,
  and the counter stays function-scoped either way
  ([0109](0109-a-for-header-declares-its-own-counter.md)).
- **`as` is the only conversion spelling**; PHP's `(int)$x` does not parse
  ([0034](0034-legacy-cast-syntax-rejected.md)).
- **`as ?T` converts without throwing, yielding `null` where `as T` would throw**
  ([0066](0066-nullable-conversion-operator.md)).
- **A condition is the one place a value is tested without `as`**, resolving PHP's full truthy table
  ([0035](0035-truthy-boolean-context.md)).
- **`string` is guaranteed-valid UTF-8 and counts grapheme clusters; binary data is the separate `bytes`
  type, counting bytes** ([0009](0009-string-and-bytes.md)).
- **A duration is a literal — `30s`, `1h30m`** — typed `Core\Time\Duration`, folded to a constant, and
  written in the one grammar `Duration::parse` and `nvs.toml` share
  ([0070](0070-duration-literals.md)).
- **`decimal` is a scalar, not a class**, and `decimal ⊕ float` is a compile error
  ([0054](0054-decimal-scalar-type.md)).
- **Enums are a closed, named integer type**, never PHP's class-like construct
  ([0010](0010-enums-are-a-value-type.md)).
- **A `string`/`int` literal and a named enum case are each their own type**, unioned to declare a closed
  set ([0047](0047-literal-and-enum-case-types.md)).
- **`object` is the opaque top of every class type, and `{a: 1}` builds an anonymous methodless instance** —
  an inline `{name: T}` shape type is the one structurally checked exception
  ([0036](0036-anonymous-object-shapes.md)).
- **Every constructor must definitely assign every property it declares**
  ([0022](0022-definite-property-initialization.md)); `lateinit` is the one opt-out
  ([0038](0038-lateinit-property-modifier.md)).

## The language surface

- **Every function is a method, every constant a class constant** — no free function, no global constant
  ([0011](0011-functions-and-constants-are-class-members.md)).
- **`static` marks a class member; nothing else holds state behind a function's back**
  ([0008](0008-static-and-global.md)).
- **No variable is ever populated by the host** — PHP's superglobals become `Core` accessor classes
  ([0012](0012-no-superglobals.md)).
- **Nothing gets a second runtime-reachable name** — a compile-time `type` alias is the one exception
  ([0015](0015-no-name-aliasing.md)).
- **There is no `trait`** — shared behavior is an interface method with a body, shared state is
  `implements Interface by $field;` delegation
  ([0043](0043-interface-default-methods-and-delegation-replace-traits.md)).
- **Ordering two objects requires the global `Comparable` interface**, with no property-walk fallback
  ([0013](0013-comparable-interface.md)).
- **A property access runs its own hook, then a declared `PropertyObserver`** — an undeclared property is
  always a hard error, and `__call`/`__callStatic` do not exist
  ([0014](0014-property-observer.md)).
- **`Stringable` replaces `__toString`, and Novis has no destructors, `__debugInfo` or `__set_state`**
  ([0028](0028-closing-the-remaining-magic-methods.md)).
- **`callable` is satisfied by exactly one shape of value, a closure, and there is no `__invoke`**
  ([0027](0027-callable-is-closures-only.md)).
- **`fn` is the only closure literal, closures have no `use` clause, and `callable` is the only type name**
  ([0031](0031-callable-is-the-only-closure-type.md)).
- **A `callable` may carry its signature** — `callable(int): string`, the return mandatory, arity a prefix
  match, parameters contravariant and the return covariant, and a `fn` literal taking its parameter types
  from the position it is written in ([0136](0136-a-callable-carries-its-signature.md)).
- **What PHP 8.6 deprecates is refused outright, and its partial application is not adopted** — no `return`
  leaves a `finally` or carries a value out of a constructor, `let`/`is` are reserved, a `readonly`
  property has no default, and a session id the store did not issue is always rejected
  ([0124](0124-php-86-lands-as-four-refusals-and-one-session-rule.md)).
- **`Iterable`/`Iterator` are the only iteration interfaces, and generators lower to a state machine** —
  no `ArrayAccess`, no `Countable` ([0053](0053-iteration-and-generators.md)).
- **`clone` is PHP's shallow copy; `serialize` shares one graph-copy operation with the `spawn` boundary** —
  neither has a customization hook
  ([0023](0023-clone-serialize-and-cross-boundary-copy.md)).
- **`#[...]` attributes are shape-literal metadata, never a declared attribute class**
  ([0046](0046-attributes-shape-literal-metadata.md)).
- **`require` is the only same-frame file-inclusion construct**
  ([0021](0021-single-file-inclusion-construct.md)).
- **A name reaches its file through a compile-time `autoload` declaration, never a runtime loader**
  ([0061](0061-compile-time-autoload-and-program-discovery.md)).
- **`&&`/`||` are the only logical connectives**; `and`/`or`/`xor` do not parse
  ([0045](0045-and-or-xor-keyword-operators-rejected.md)).
- **`==` is the only equality operator, it never converts, and two statically disjoint types do not
  compile** — strings, arrays and objects each take the strict reading
  ([0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md)).
- **`<?nvs` is the only code-mode open tag and `exit` the only termination keyword** — `<?=` is sugar for
  `<?nvs echo`, not a second tag ([0049](0049-single-open-tag-and-single-exit-keyword.md)).
- **`[...]` is the only destructuring spelling**
  ([0050](0050-list-destructuring-spelling-rejected.md)).
- **A name with a `\` in it is absolute and a leading `\` does not parse** — a name without one resolves
  through the imports then the enclosing namespace, and nowhere else
  ([0113](0113-a-qualified-name-is-absolute.md)).
- **Identifier casing is a hard compiler error with no suppression**, checked on the leading character only
  ([0029](0029-identifier-casing-is-checked.md)).
- **No identifier may start with `_`, and the constructor is spelled `constructor`**
  ([0030](0030-no-leading-underscores-constructor-spelling.md)).
- **Every member declaration writes a visibility and there is no implicit `public`** — a plain constructor
  parameter is exempt, because visibility is what promotes one to a property
  ([0094](0094-visibility-is-written-at-every-member-declaration.md)).
- **Nothing depends on the case something was typed in, or on the filesystem's opinion of it**
  ([0062](0062-case-sensitivity-is-a-compiler-property.md)).
- **Input whose spelling and resolution can differ is refused, never repaired** — a closed ambiguity list
  for an HTTP message in both directions, byte-exact cookie names with the prefixes enforced, a multipart
  part count, and path components that do not spell what they open
  ([0095](0095-ambiguous-input-is-refused-never-repaired.md)).
- **A `#[Route]` without a sibling `#[Access]` does not compile**, and CSRF is on by default for unsafe
  methods ([0096](0096-a-route-without-a-declared-access-decision-does-not-compile.md)); the server
  enforces the CSRF check and the **dispatcher** enforces the access decision, never both
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 8).
- **A bidirectional control that opens a scope and never closes it is a compile error in source and is
  substituted at both output sinks** ([0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md)).
- **A by-reference binding is spelled `inout` before the type and written again at the call site**, so a
  reader sees at the call that an argument will be written; `&` is no longer a by-reference marker in any
  position and keeps only bitwise AND and the intersection type
  ([0107](0107-by-reference-parameters-are-spelled-inout-at-both-ends.md)).

## Security and isolation

- **Untrusted input carries a `tainted` qualifier that `Core` sinks refuse until it is laundered** — the
  HTML sink additionally auto-escapes by default
  ([0024](0024-taint-tracking-for-injection-sinks.md)).
- **A launderer answers its sink's carrier type when that sink escapes on its own and the escape is not
  idempotent** — so `Core\Html::escape` answers `Core\Html\Markup` and cannot be escaped a second time,
  every other launderer on the roster keeps its plain `string`, and `Core\Html::toSource` with a written
  reason is the only way back out
  ([0133](0133-a-launderer-answers-its-sinks-carrier-and-only-an-idempotent-escape-answers-a-string.md)).
- **A sink is a parameter whose content becomes an instruction, and an unclassified one refuses** — `echo`
  binds to the terminal sink everywhere but an HTTP request, where a body is one typed `Core\Response`
  member ([0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md)).
- **A confidential value carries a `secret` qualifier that output, logs, dumps, `Throwable` messages and
  serialization all refuse** ([0033](0033-secret-qualifier-for-confidential-values.md)).
- **An extension's manifest can only tighten the qualifier analysis, never loosen it**
  ([0055](0055-extension-qualifier-declarations.md)).
- **Extensions are sandboxed wasm, never `dlopen`** ([0003](0003-extension-system.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget**; its entry is a file
  path or a static method whose parameters `args:` binds by name, never a closure
  ([0006](0006-isolated-script-execution.md)).
- **A WebSocket or SSE connection is its own root isolate, opened with `spawn script`'s operand**, its
  `receive()` is the one wait over the peer and its topics, and `Core\Topic` closes a slow subscriber
  rather than blocking a publisher ([0083](0083-persistent-connections-are-isolates.md)).
- **Compiled code is the only thing a request shares with any other**
  ([0017](0017-hot-reload-without-restart.md)).
- **Cross-request state is explicit: `Core\Cache` is per-core, copied in and out, charged to the core**
  ([0059](0059-cross-request-state-is-explicit.md)).
- **`Core\Process` is the only way to run another program, and it never accepts a shell string**
  ([0044](0044-core-process-argv-only-no-shell.md)).
- **An outbound URL is a sink, and the connection is made to a pinned address**
  ([0058](0058-outbound-request-policy.md)).
- **Regex runs on a linear-time engine by default**, under a step budget that throws
  ([0056](0056-regex-engine-policy.md)).
- **A fatal error never reaches an ordinary `catch`** — it escalates through a zero-retry, reserved-budget
  handler ladder ([0020](0020-error-escalation-ladder.md)).
- **Four doors stay shut: no FFI, no stream wrappers, no cross-request state, no `eval`** — none has an
  opt-in ([0052](0052-closed-doors.md)).
- **A closed four-entry roster of application-layer security protocols lives in `Core`**
  ([0060](0060-application-security-protocols.md)).
- **HTTP defaults are safe inbound and finite outbound** — secure headers and closed CORS with nothing
  configured, and no spelling for an unbounded outbound wait
  ([0074](0074-http-defaults-safe-and-finite.md)).
- **`Core\RateLimit` limits what only the application knows** — per account, per tenant — while edge and
  flood limiting stay the proxy's ([0075](0075-core-ratelimit.md)).
- **The built-in server is a development server and a proxied origin, and a URL selects a mount rather than
  a path** — no TLS listener, no h2c, no FastCGI and no compression, and every executable path is enumerated
  at boot ([0097](0097-development-server-and-proxied-origin.md)).
- **An uploaded file is a stream and there is one way to receive it** — `Core\Request::files()` yields parts
  lazily, a part is read into a bounded buffer or written straight to disk, and there is no temp file
  because the destination is the application's own call
  ([0105](0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)).
- **Nothing a request can send terminates or wedges a worker** — containment extends past the helper to the
  worker task, no path reaches `abort()`, every depth and duration a request drives is bounded on the
  engine's stack and inside a single helper, a core never blocks on a syscall, and admission is arithmetic
  against the memory budget; a request the client abandons is cancelled at the connection's drop, never
  waited out; the residue is one named class — a memory-safety fault or a miscompile — that
  the process boundary was examined for and rejected
  ([0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)).
- **Every platform reports readiness, never completion, and a stream tries the syscall before it parks** —
  one contract over epoll, kqueue and Windows' AFD, a wake is a hint the task re-tries rather than a
  promise, and a task's 1 MiB stack is reserved wide, resident narrow and pooled per worker
  ([0115](0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md)).

## Runtime, tooling and the standard library

- **A stdlib candidate is placed by six ordered tests, and only Tier 0 may claim the `Core` prefix** — and
  no client holding state across calls may be Tier 1, which closes the door on every broker, directory and
  session-bearing protocol as an extension ([0051](0051-standard-library-tiers.md)).
- **Every `Core` member has the same shape: subject first, one trailing options shape, nothing mutates,
  failure throws, absence is `?T`, and every parameter is callable by the spec's `$name`**
  ([0063](0063-core-api-conventions.md)).
- **A fixed-key shape parameter is one `CoreTy` carrying its arms, and it flattens at the ABI exactly as an
  options bag does** — written as a literal at the call site, arms pairwise disjoint so the discriminant
  falls out of checking
  ([0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md)).
- **Arrays combine by the member's name, never by a key's type** — `overlay`/`underlay`/`appendAll`, no
  `merge`, and `array + array` does not compile
  ([0069](0069-array-combination-is-key-type-independent.md)).
- **The compiler validates a closed list of `Core` intrinsics' literal arguments during checking**
  ([0057](0057-intrinsic-literal-folding.md)).
- **Reflection and AST parsing are built into `Core`, not left to extensions**
  ([0019](0019-reflection-and-ast-parsing-are-core-features.md)).
- **One database API: a connection is named in root-owned config, every statement is prepared, and a
  transaction is a closure** — and a released connection rejoins a per-core pool only after a reset that is
  a security boundary ([0067](0067-core-db.md), § 13 for the pool).
- **A dependency is a content-addressed archive resolved by minimal version selection, no package code runs
  before your program does, and a package's capabilities are granted one line at a time rather than
  inherited** ([0081](0081-packages-are-digests-resolution-is-a-maximum.md)).
- **Authority is keyed on the namespace enclosing the code — longest prefix wins, a subtree is spelled
  `"Vendor\*"`, and the rule reaches an overriding file and a hand-vendored tree as readily as a fetched
  package. A required capability that is not granted fails the build; one a package declares *optional*
  compiles and throws only if reached, so a library degrades through `Core\Cap::has` instead of refusing to
  install** ([0112](0112-authority-is-keyed-on-the-enclosing-namespace.md)).
- **Novis ships its own framework, split by ADR 0051's six tests** — privileged halves in `Core`, the
  opinionated layer as the `nvs/web` package, no ORM and no runtime container
  ([0082](0082-the-first-party-framework.md)).
- **A background job is a row in a `Core\Db` table, so an enqueue commits with the write that caused it**,
  and delivery is at-least-once with bounded retries
  ([0084](0084-durable-background-jobs.md)).
- **The OpenAPI document is generated while compiling, and an `#[Api]` that contradicts the code is a
  compile error** ([0085](0085-openapi-is-generated-from-the-route-table.md)).
- **`#[Json\Derive]`/`#[Db\Derive]` generate a codec from a class's declared properties, and a failed decode
  reports every bad field at once** — a compiler-recognized attribute is matched by name, unlike
  `Core\Attributes` retrieval ([0071](0071-derived-codecs.md)).
- **Configuration is TOML, in a root-owned `nvs.toml`**
  ([0064](0064-configuration-file-format.md)); it states defaults, not ceilings
  ([0005](0005-config-changeability.md)).
- **A run mode is `development` or `production`, defaults to production, and only selects the defaults of
  four named directives** — no environment variable is ever read for it
  ([0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)).
- **Every developer-facing output is one closed record the sink in force renders as plaintext, JSON or
  HTML** — no call site names a format, and a dump reaches a response body only in development mode
  ([0092](0092-one-diagnostic-record-three-renderings.md)).
- **`nvs ctl reload` replaces the whole config snapshot over a local socket — no control port, no token** —
  and a directive that still needs a restart is named in the result rather than ignored
  ([0078](0078-config-reload-and-control-socket.md)).
- **`nvs service` registers this binary with the platform's service manager, storing one verbatim argv, and
  the installer is a sink that refuses any subcommand but `serve`/`run`**
  ([0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md)).
- **`Core\Task::all`/`::map` return with nothing still running, and `afterResponse` keeps the request tree
  alive past the connection** — cancellation runs no user code
  ([0072](0072-core-task-structured-concurrency.md)).
- **Scheduled work is a `[[schedule]]` block firing a `spawn script`, never an API** — `scope` is mandatory
  ([0073](0073-scheduled-work-is-config.md)).
- **`#[Route]` builds the route table while compiling, and the router stops at matching**
  ([0077](0077-compile-time-routing.md)).
- **One method's repeated routes may share a `name` when they share a `path`, and there is no wildcard
  verb** — `methodsFor` owes an exact `Allow:` header and CSRF is classified per verb
  ([0110](0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md)).
- **The server matches each request once before the handler, and `Core\Request::route()` is that match**
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)).
- **A capture narrows to a closed set with a type and never with a regex**, because a pattern over the
  request path runs before rate limiting
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 5).
- **A link is built through `url`/`urlAbsolute`, and an absolute one's origin is configured per mount,
  never read from a header**
  ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 6).
- **The terminal is a sink that substitutes control bytes visibly, styling is the `Cli\Text` value type,
  and `#[Command]` builds the argument table while compiling**
  ([0086](0086-core-cli-terminal-is-a-sink.md)).
- **The runtime exports what it already measures, and a metric label refuses `tainted`**
  ([0076](0076-observability-export.md)).
- **A test is a `#[Test]` method whose table is built while compiling, and every test is its own isolate** —
  assertions are generic, so a type-mismatched comparison never runs
  ([0079](0079-testing-is-a-language-feature.md)).
- **Coverage, tracing and profiling are always-emitted, flag-gated probes, never a second compiled tier**
  ([0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)); GC and spawn events
  are instrumented in their own routines, off the hot path
  ([0041](0041-timeline-export-and-gc-spawn-trace-events.md)).
- **The on-disk artifact cache is one immutable file per unit, verified before any page is made executable**
  ([0042](0042-on-disk-artifact-cache-format.md)).
- **A portable single-file executable ships source appended to the host binary**
  ([0048](0048-portable-single-file-executables.md)).
- **`nvs fmt` has one unconfigurable style, never reflows, and is never wired into `nvs check`**
  ([0039](0039-canonical-code-formatting.md)).
- **`nvs fmt` changes layout only — it never renames, never supplies a missing keyword, and never reorders
  class members; an editor composes it with quick fixes on save, which is a client concern**
  ([0039](0039-canonical-code-formatting.md) §§ 9-11,
  [0040](0040-vscode-deep-tooling-and-resilient-parsing.md) § 3).
- **`nvs convert` is one deterministic rule table read through two modes, and an "identical" rewrite is one
  a differential case against the PHP oracle proves**
  ([0089](0089-convert-is-one-rule-table-with-two-modes.md)).
- **`nvs-syntax` exposes a second, lossless, error-recovering parse entry point for editor tooling only**
  ([0040](0040-vscode-deep-tooling-and-resilient-parsing.md)); IDE smarts live once, in `nvs-lsp`
  ([0016](0016-ide-integration.md)).
- **There is no browser compile target** — the wasm32 one is retired, and wasm reaches Novis only as the
  *host* an `.nvsx` extension runs inside ([0003](0003-extension-system.md)); what the target would have
  cost, and what would reopen it, is [0025](0025-wasm-browser-target.md) *Revisiting*.
- **Performance history is callgrind instruction counts on a dedicated Linux runner, never raw wall-clock
  across machines** ([0026](0026-performance-measurement-methodology.md)).
- **`|>` substitutes the hole `$_`, required exactly once, in the parser — so a pipeline is the same AST
  the nested call produces, and it is not PHP 8.5's callable-applying `|>`**
  ([0098](0098-pipeline-operator-is-a-hole-substituted-at-parse-time.md)); scalars still gain no methods
  ([0063](0063-core-api-conventions.md) R19).
- **`expr catch (Class $e) => value` guards one expression and lowers to the block form** — arms chain
  like clauses, an arm holds an expression so `throw` is in and `return` is out, the result type is the
  union of the sides, and `finally` stays the block form's alone
  ([0119](0119-an-expression-level-catch-is-a-typed-arm-on-one-guarded-expression.md)).
- **`class<T>` is a type whose value is the run-time class descriptor, and `as` is its only source** —
  `new $cls(...)`, `$cls::f()` and `$x instanceof $cls` accept it and nothing else, it widens with its
  argument and never back, and a `new` over it is refused when an implementor of `T` declares an
  incompatible constructor
  ([0125](0125-a-class-reference-is-a-type-and-as-is-its-only-source.md)).
- **`property<T>` is a type whose values are `T`'s public declared property names, and `as` is its only
  source** — `$obj->$key` is admitted for that operand alone and `E0235` moves to the checker for every
  other, a read is the union of the set's types, a write is the checked erased store, and a class with a
  `readonly` property refuses the write
  ([0126](0126-a-property-key-is-a-checked-name-and-as-is-its-only-source.md)).
- **One grammar and one tree: the resilient parse is the AST plus a trivia layer and an offset index,
  `nvs-lsp` is synchronous on `lsp-server`/`lsp-types` so no async runtime enters the workspace, an LSP
  answer is frozen as a `.lspt` case, and syntax highlighting is two layers with two tests**
  ([0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md)).
- **The editor offers a value only where the compiler already derived it for another reason** — a route
  name, a config directive, an `#[Api]` field — never from a convention scan, an annotation dialect or a
  network request, which is the whole of Novis's framework support; one workspace index answers references,
  occurrence highlight, CodeLens, type hierarchy and unused-member dimming; and an inline-HTML region gets
  the editor's own HTML/CSS/JS services but **no** formatter beside `nvs fmt`
  ([0108](0108-one-reference-index-completion-from-derived-facts-and-services-in-a-template-region.md)).
- **A PHP built-in completes to its Novis destination: every name the differential oracle's inventory holds
  is a candidate, the migration table is the answer, and an item inserts only a member the `Core` registry
  already holds — so a dropped, undecided or not-yet-built destination appears and types nothing**
  ([0111](0111-a-php-builtin-completes-to-its-novis-destination.md)).
- **An implemented `Core` member's reference documentation lives in its registry declaration — every row,
  enum and constant carries its card, and the crate's tests refuse one without — and `nvs meta --json`
  prints it; the spec stays authoritative for the surface, and a doc field the registry carries wins over
  the spec, field by field**
  ([0117](0117-an-implemented-core-member-documents-itself-in-the-registry.md)).
- **An `array<mixed>` annotation narrows to the type of the literal under it through one editor action —
  element types widened to their base and joined, offered only where that literal is right there, never
  where the value arrived from input, and never on save because `array<T>` is invariant — and the synthesis
  it calls sits in `nvs-types` unreachable from every compile path**
  ([0114](0114-an-array-literals-own-type-is-synthesized-for-one-code-action.md)).
- **A `secret` value's bytes are concealed in the editor by default, on ranges the server computes and the
  client only draws; `tainted` gets no default decoration, because how a construct looks is the user's
  theme's to decide** ([0101](0101-secret-is-redacted-in-the-editor-and-the-range-comes-from-the-server.md);
  the token modifiers themselves are [0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 4).
- **Against Python Novis claims the tool that gets handed over, never speed or replacement: a file opening
  `#!` starts in code mode with no tag, there is no REPL, and the userland suite measures three engines**
  ([0100](0100-against-python-nvs-claims-the-tool-that-gets-handed-over.md)).
- **The configuration is a tree of files resolved as one ordered stream where later wins and every override
  is reported; every file in it must be unwritable by any account but its owner, and an absent `optional`
  include puts that check on the directory that would hold it**
  ([0103](0103-configuration-is-a-tree-of-files.md)).
- **An application is its entry file path: `[[app]]` is keyed on a canonicalized `root` prefix or exact
  `entry`, every matching block layers least-specific first, and a block may grant as well as narrow,
  bounded by the global `[limits.hard]`** ([0104](0104-an-application-is-an-entry-file-path.md)).
- **A capability is checked inside the one function that performs the effect, never beside it, and what
  each member needs is declared in one auditable table no execution path reads** — a member needing none
  pays nothing, and a denial is a catchable `RuntimeError` naming the capability
  ([0118](0118-a-capability-is-checked-at-the-door-to-the-effect.md)).
- **An isolate's arena is an ownership root, not an address range: entering one maps nothing, releasing one
  wholesale is a drain of the refcount worklist plus a sweep of the cyclic objects the refcounts could not
  free — both run native teardown — its statics base is its own, and
  a crossing at refcount 1 is a pointer handoff** ([0116](0116-an-isolates-arena-is-an-ownership-root.md)).
- **The image component is a Novis builder over a handful of coarse sandboxed entry points, one crossing
  per terminal, with one pixel model, a header-read pixel cap, upright colour-correct decoding and
  metadata stripped on encode — and none of gd's palette mode, mode flags, drawing primitives or formats**
  ([0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md)).
- **PDF generation is a first-party Tier 1 component, unscheduled, that renders HTML with no I/O — assets
  are bytes passed in, an unresolved reference throws, unsupported CSS is dropped and reported, the output
  is inert and byte-reproducible, and the programmatic path is a builder emitting the same HTML**
  ([0121](0121-pdf-generation-is-sandboxed-html-rendering-with-no-io.md)).
- **HTML parses by the WHATWG algorithm through an entry on `Core\Html` — never a mode of `Core\Xml`,
  never failing where `Core\Xml` refuses — producing `Core\Xml`'s own tree over the parser the PDF
  engine shares, and landing in whatever milestone builds `Core\Xml`**
  ([0122](0122-html-parsing-is-a-whatwg-entry-on-core-html-over-core-xmls-tree.md)).
- **Spreadsheets are a first-party Tier 1 component, unscheduled — workbooks cross as bytes with nothing
  fetched and nothing executed, a formula is a typed value so a string is always a text cell, macros are
  never written, CSV stays `Core\Csv`, and equal input gives byte-equal output**
  ([0123](0123-spreadsheet-reading-and-generation-are-one-sandboxed-component-with-no-io.md)).
- **A PDF page is a decode source of the image component — `open` takes `page` and `dpi`, `info`'s
  `frames` is the page count read without a pixel, the page box at the requested dpi is priced against
  the pixel cap before any buffer, encryption and an undecodable embedded object throw, and text
  extraction stays third-party** ([0128](0128-a-pdf-page-is-a-decode-source-of-the-image-component.md)).
- **The end of a script is observable: `Core\Script::onExit` hooks run FIFO as the last user code at
  every non-fatal ending — normal, `exit`, uncaught throw — never on a `FATAL` or a cancellation, and
  they observe the ending rather than change it** ([0127](0127-the-end-of-a-script-is-observable.md)).
- **`Core\Password::verify` and `::needsRehash` read a PHP-stored bcrypt hash — `verify` verifies it,
  `needsRehash` answers `true` for every one — while `hash` writes only Argon2id, so a migrated user
  table upgrades itself one login at a time**
  ([0129](0129-password-verify-reads-a-stored-bcrypt-hash.md)).
- **Usage telemetry is two independent opt-ins, both off by default: any `nvs` invocation records a
  closed counter set locally — the subcommand that ran, never argv and never anything from request
  traffic — and last week's aggregate uploads at most weekly, riding a short-lived command with a
  3-second budget that can never fail it; a process serving traffic records but never uploads**
  ([0130](0130-usage-telemetry-is-opt-in-and-a-serving-process-never-uploads.md)).
- **A temporary directory dies with its script: the runtime deletes what `Core\IO::temporaryDir` handed
  out when the script ends, without ever throwing, and reclaims a dead process's leftovers — keyed on
  owner liveness, never age — only at `nvs serve` boot and under `nvs tmp clean`; there is no
  `temporaryFile` and no per-call persist, only the reloadable operator key `[debug] keep_temporary`**
  ([0131](0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md)).
- **A database driver is a borrowed sans-IO codec plus a state machine we write, over the parking stream** —
  the five live in `crates/nvs-db` as an enum rather than behind a trait, TLS is `nvs-host`'s one client
  generalised over its transport, and a connection whose wire is not at a known message boundary is closed
  rather than reset ([0132](0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)).
- **A shipped feature is not finished until four proofs exist for it** — its behaviour pinned from Novis
  *and* from Rust, three real-world examples under `docs/examples/`, one measured figure in
  `benches/members/` recorded against a machine fingerprint, and one program in `tests/hostile/` written
  to break it; the roster of features is derived from `nvs meta --json` and the reference chapters rather
  than kept anywhere, and `python tools/dossier.py` is the whole mechanism
  ([0134](0134-every-shipped-feature-owes-four-proofs.md)).
