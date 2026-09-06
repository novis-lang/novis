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

- **Novis is built to serve web applications of every kind** — the safety properties are how it is built
  rather than who it is for, so the framework and the dependency story outrank new breadth, the pitch is
  isolation and qualifiers rather than speed, and no document may claim PHP compatibility
  ([0080](../decisions/0080.md)).
- **Memory is spent to buy security, semantics, latency and simplicity, in that order** — and never to
  buy a leak: what is spent stays attributable to a request, under an enforceable cap, and O(in-flight)
  rather than O(requests served) ([0004](../decisions/0004.md)).
- **`unsafe` is forbidden workspace-wide**; only `nvs-runtime`, `nvs-codegen`, `nvs-stdlib` and
  `benches/abi-probe` opt down to `deny` with narrow, reasoned allows ([Cargo.toml](../../Cargo.toml)).
- **Nothing unwinds through a JIT frame** — every call returns a checked status
  ([0002](../decisions/0002.md)).
- **Pure-Rust dependencies by default**, enforced by [deny.toml](../../deny.toml) in CI; a C dependency must
  pass [0051](../decisions/0051.md) § 4's two questions.
- **Domain logic is an existing first-class Rust crate; compiler passes and scheduler primitives are ours** —
  anything with an external specification is a dependency, and if no crate exists the feature is not built
  ([README.md](README.md) § *Decisions taken at project start*).
- **SIMD comes from a dependency that dispatches at runtime, never from an intrinsic we wrote or a
  `target-cpu` flag, and the JIT emits scalar code by design**
  ([README.md](README.md) § *Decisions taken at project start*).
- **Architecture assumptions are tested, not remembered** — [benches/abi-probe/](../../benches/abi-probe/)
  guards them on every CI run; if one fails, revisit the ADR it points at rather than the threshold.
- **Every third-party notice Novis owes is generated, committed and embedded in the binary**
  ([0065](../decisions/0065.md)).
- **Dependencies stay current, and from 0.1.0 a break in one is absorbed rather than forwarded to Novis
  programs** — until then, update anything freely; the sweep is a pass the *user* fires, never an agent
  ([0068](../decisions/0068.md),
  [docs/agent/dependency-update.md](../agent/dependency-update.md)).

## The type system

- **Nothing is untyped, and no type ever changes by itself** — every binding declares a type, `mixed` is
  the one unchecked position, and there is no `resource` type
  ([0007](../decisions/0007.md)).
- **`var $name = expr;` infers a local's type from its initializer and fixes it forever** — a bare
  array-literal initializer is the one shape it refuses
  ([0037](../decisions/0037.md)).
- **A `for` header's init clause is one typed local declaration or a list of expressions, never both**,
  and the counter stays function-scoped either way
  ([0109](../decisions/0109.md)).
- **`as` is the only conversion spelling**; PHP's `(int)$x` does not parse
  ([0034](../decisions/0034.md)).
- **`as ?T` converts without throwing, yielding `null` where `as T` would throw**
  ([0066](../decisions/0066.md)).
- **A condition is the one place a value is tested without `as`**, resolving PHP's full truthy table
  ([0035](../decisions/0035.md)).
- **`string` is guaranteed-valid UTF-8 and counts grapheme clusters; binary data is the separate `bytes`
  type, counting bytes** ([0009](../decisions/0009.md)).
- **A duration is a literal — `30s`, `1h30m`** — typed `Core\Time\Duration`, folded to a constant, and
  written in the one grammar `Duration::parse` and `nvs.toml` share
  ([0070](../decisions/0070.md)).
- **`decimal` is a scalar, not a class**, and `decimal ⊕ float` is a compile error
  ([0054](../decisions/0054.md)).
- **Enums are a closed, named integer type**, never PHP's class-like construct
  ([0010](../decisions/0010.md)).
- **A `string`/`int` literal and a named enum case are each their own type**, unioned to declare a closed
  set ([0047](../decisions/0047.md)).
- **`object` is the opaque top of every class type, and `{a: 1}` builds an anonymous methodless instance** —
  an inline `{name: T}` shape type is the one structurally checked exception
  ([0036](../decisions/0036.md)).
- **Every constructor must definitely assign every property it declares**
  ([0022](../decisions/0022.md)); `lateinit` is the one opt-out
  ([0038](../decisions/0038.md)).

## The language surface

- **Every function is a method, every constant a class constant** — no free function, no global constant
  ([0011](../decisions/0011.md)).
- **`static` marks a class member; nothing else holds state behind a function's back**
  ([0008](../decisions/0008.md)).
- **No variable is ever populated by the host** — PHP's superglobals become `Core` accessor classes
  ([0012](../decisions/0012.md)).
- **Nothing gets a second runtime-reachable name** — a compile-time `type` alias is the one exception
  ([0015](../decisions/0015.md)).
- **There is no `trait`** — shared behavior is an interface method with a body, shared state is
  `implements Interface by $field;` delegation
  ([0043](../decisions/0043.md)).
- **Ordering two objects requires the global `Comparable` interface**, with no property-walk fallback
  ([0013](../decisions/0013.md)).
- **A property access runs its own hook, then a declared `PropertyObserver`** — an undeclared property is
  always a hard error, and `__call`/`__callStatic` do not exist
  ([0014](../decisions/0014.md)).
- **`Stringable` replaces `__toString`, and Novis has no destructors, `__debugInfo` or `__set_state`**
  ([0028](../decisions/0028.md)).
- **`callable` is satisfied by exactly one shape of value, a closure, and there is no `__invoke`**
  ([0027](../decisions/0027.md)).
- **`fn` is the only closure literal, closures have no `use` clause, and `callable` is the only type name**
  ([0031](../decisions/0031.md)).
- **A `callable` may carry its signature** — `callable(int): string`, the return mandatory, arity a prefix
  match, parameters contravariant and the return covariant, and a `fn` literal taking its parameter types
  from the position it is written in ([0136](../decisions/0136.md)).
- **A doc comment is `///` and carries prose plus exactly `@see` and `@example`** — every other `@tag` is
  a diagnostic, because a signature, an attribute and a capability already say what PHPDoc's tags said
  ([0137](../decisions/0137.md)).
- **What PHP 8.6 deprecates is refused outright, and its partial application is not adopted** — no `return`
  leaves a `finally` or carries a value out of a constructor, `let`/`is` are reserved, a `readonly`
  property has no default, and a session id the store did not issue is always rejected
  ([0124](../decisions/0124.md)).
- **`Iterable`/`Iterator` are the only iteration interfaces, and generators lower to a state machine** —
  no `ArrayAccess`, no `Countable` ([0053](../decisions/0053.md)).
- **`clone` is PHP's shallow copy; `serialize` shares one graph-copy operation with the `spawn` boundary** —
  neither has a customization hook
  ([0023](../decisions/0023.md)).
- **`#[...]` attributes are shape-literal metadata, never a declared attribute class**
  ([0046](../decisions/0046.md)).
- **`require` is the only same-frame file-inclusion construct**
  ([0021](../decisions/0021.md)).
- **A name reaches its file through a compile-time `autoload` declaration, never a runtime loader**
  ([0061](../decisions/0061.md)).
- **`&&`/`||` are the only logical connectives**; `and`/`or`/`xor` do not parse
  ([0045](../decisions/0045.md)).
- **`==` is the only equality operator, it never converts, and two statically disjoint types do not
  compile** — strings, arrays and objects each take the strict reading
  ([0090](../decisions/0090.md)).
- **`<?nvs` is the only code-mode open tag and `exit` the only termination keyword** — `<?=` is sugar for
  `<?nvs echo`, not a second tag ([0049](../decisions/0049.md)).
- **`[...]` is the only destructuring spelling**
  ([0050](../decisions/0050.md)).
- **A name with a `\` in it is absolute and a leading `\` does not parse** — a name without one resolves
  through the imports then the enclosing namespace, and nowhere else
  ([0113](../decisions/0113.md)).
- **Identifier casing is a hard compiler error with no suppression**, checked on the leading character only
  ([0029](../decisions/0029.md)).
- **No identifier may start with `_`, and the constructor is spelled `constructor`**
  ([0030](../decisions/0030.md)).
- **Every member declaration writes a visibility and there is no implicit `public`** — a plain constructor
  parameter is exempt, because visibility is what promotes one to a property
  ([0094](../decisions/0094.md)).
- **Nothing depends on the case something was typed in, or on the filesystem's opinion of it**
  ([0062](../decisions/0062.md)).
- **Input whose spelling and resolution can differ is refused, never repaired** — a closed ambiguity list
  for an HTTP message in both directions, byte-exact cookie names with the prefixes enforced, a multipart
  part count, and path components that do not spell what they open
  ([0095](../decisions/0095.md)).
- **A `#[Route]` without a sibling `#[Access]` does not compile**, and CSRF is on by default for unsafe
  methods ([0096](../decisions/0096.md)); the server
  enforces the CSRF check and the **dispatcher** enforces the access decision, never both
  ([0102](../decisions/0102.md) § 8).
- **A bidirectional control that opens a scope and never closes it is a compile error in source and is
  substituted at both output sinks** ([0087](../decisions/0087.md)).
- **A by-reference binding is spelled `inout` before the type and written again at the call site**, so a
  reader sees at the call that an argument will be written; `&` is no longer a by-reference marker in any
  position and keeps only bitwise AND and the intersection type
  ([0107](../decisions/0107.md)).
- `::class` answers the class the value **is**, which is folded only where the compiler already
  knows it ([0144](../decisions/0144.md)).
- An optional field of a `Core` options bag or shape parameter may be nullable, and wherever a `Core`
  member admits a written `null` it means *remove*, while an omitted key means *leave alone*
  ([0147](../decisions/0147.md)).

## Security and isolation

- **Untrusted input carries a `tainted` qualifier that `Core` sinks refuse until it is laundered** — the
  HTML sink additionally auto-escapes by default
  ([0024](../decisions/0024.md)).
- **A launderer answers its sink's carrier type when that sink escapes on its own and the escape is not
  idempotent** — so `Core\Html::escape` answers `Core\Html\Markup` and cannot be escaped a second time,
  every other launderer on the roster keeps its plain `string`, and `Core\Html::toSource` with a written
  reason is the only way back out
  ([0133](../decisions/0133.md)).
- **A sink is a parameter whose content becomes an instruction, and an unclassified one refuses** — `echo`
  binds to the terminal sink everywhere but an HTTP request, where a body is one typed `Core\Response`
  member ([0088](../decisions/0088.md)).
- **A confidential value carries a `secret` qualifier that output, logs, dumps, `Throwable` messages and
  serialization all refuse** ([0033](../decisions/0033.md)).
- **An extension's manifest can only tighten the qualifier analysis, never loosen it**
  ([0055](../decisions/0055.md)).
- **Extensions are sandboxed wasm, never `dlopen`** ([0003](../decisions/0003.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget**; its entry is a file
  path or a static method whose parameters `args:` binds by name, never a closure
  ([0006](../decisions/0006.md)).
- **A WebSocket or SSE connection is its own root isolate, opened with `spawn script`'s operand**, its
  `receive()` is the one wait over the peer and its topics, and `Core\Topic` closes a slow subscriber
  rather than blocking a publisher ([0083](../decisions/0083.md)).
- **Compiled code is the only thing a request shares with any other**
  ([0017](../decisions/0017.md)).
- **Cross-request state is explicit: `Core\Cache` is per-core, copied in and out, charged to the core**
  ([0059](../decisions/0059.md)).
- **A session is a record its store issued, and `[session] backend` names the shared tier or the
  database — never the local one, which is refused at boot**
  ([0139](../decisions/0139.md)).
- **`Core\Process` is the only way to run another program, and it never accepts a shell string**
  ([0044](../decisions/0044.md)).
- **An outbound URL is a sink, and the connection is made to a pinned address**
  ([0058](../decisions/0058.md)).
- **A store an operator configured is authorized by the configuring** — `cache.shared` names the store
  and not a host, so that store may be a Unix socket; a socket path a *program* supplies is refused
  ([0142](../decisions/0142.md)).
- **Regex runs on a linear-time engine by default**, under a step budget that throws
  ([0056](../decisions/0056.md)).
- **A fatal error never reaches an ordinary `catch`** — it escalates through a zero-retry, reserved-budget
  handler ladder ([0020](../decisions/0020.md)).
- **Four doors stay shut: no FFI, no stream wrappers, no cross-request state, no `eval`** — none has an
  opt-in ([0052](../decisions/0052.md)).
- **A closed four-entry roster of application-layer security protocols lives in `Core`**
  ([0060](../decisions/0060.md)).
- **HTTP defaults are safe inbound and finite outbound** — secure headers and closed CORS with nothing
  configured, and no spelling for an unbounded outbound wait
  ([0074](../decisions/0074.md)).
- **`Core\RateLimit` limits what only the application knows** — per account, per tenant — while edge and
  flood limiting stay the proxy's ([0075](../decisions/0075.md)).
- **The built-in server is a development server and a proxied origin, and a URL selects a mount rather than
  a path** — no TLS listener, no h2c, no FastCGI and no compression, and every executable path is enumerated
  at boot ([0097](../decisions/0097.md)).
- **An uploaded file is a stream and there is one way to receive it** — `Core\Request::files()` yields parts
  lazily, a part is read into a bounded buffer or written straight to disk, and there is no temp file
  because the destination is the application's own call
  ([0105](../decisions/0105.md)).
- **Nothing a request can send terminates or wedges a worker** — containment extends past the helper to the
  worker task, no path reaches `abort()`, every depth and duration a request drives is bounded on the
  engine's stack and inside a single helper, a core never blocks on a syscall, and admission is arithmetic
  against the memory budget; a request the client abandons is cancelled at the connection's drop, never
  waited out; the residue is one named class — a memory-safety fault or a miscompile — that
  the process boundary was examined for and rejected
  ([0106](../decisions/0106.md)).
- **Every platform reports readiness, never completion, and a stream tries the syscall before it parks** —
  one contract over epoll, kqueue and Windows' AFD, a wake is a hint the task re-tries rather than a
  promise, and a task's 1 MiB stack is reserved wide, resident narrow and pooled per worker
  ([0115](../decisions/0115.md)).
- **A `Future` is driven by the coroutine that owns it, and a `Waker` is one permission to poll again** —
  a connection is one future on one task's stack, a wake decides nothing and never re-polls, and a wake
  fired from another thread queues an id on the parked task's own core rather than moving the task
  ([0138](../decisions/0138.md)).
- A signature is computed over a canonical *payload*, never over assembled URL text, and the
  canonical form of a URL is the one `$uri->compareTo` already defines
  ([0146](../decisions/0146.md)).

## Runtime, tooling and the standard library

- **A stdlib candidate is placed by six ordered tests, and only Tier 0 may claim the `Core` prefix** — and
  no client holding state across calls may be Tier 1, which closes the door on every broker, directory and
  session-bearing protocol as an extension ([0051](../decisions/0051.md)).
- **Every `Core` member has the same shape: subject first, one trailing options shape, nothing mutates,
  failure throws, absence is `?T`, and every parameter is callable by the spec's `$name`**
  ([0063](../decisions/0063.md)).
- **A fixed-key shape parameter is one `CoreTy` carrying its arms, and it flattens at the ABI exactly as an
  options bag does** — written as a literal at the call site, arms pairwise disjoint so the discriminant
  falls out of checking
  ([0135](../decisions/0135.md)).
- **Arrays combine by the member's name, never by a key's type** — `overlay`/`underlay`/`appendAll`, no
  `merge`, and `array + array` does not compile
  ([0069](../decisions/0069.md)).
- **The compiler validates a closed list of `Core` intrinsics' literal arguments during checking**
  ([0057](../decisions/0057.md)).
- **Reflection and AST parsing are built into `Core`, not left to extensions**
  ([0019](../decisions/0019.md)).
- **One database API: a connection is named in root-owned config, every statement is prepared, and a
  transaction is a closure** — and a released connection rejoins a per-core pool only after a reset that is
  a security boundary ([0067](../decisions/0067.md), § 13 for the pool).
- **A dependency is a content-addressed archive resolved by minimal version selection, no package code runs
  before your program does, and a package's capabilities are granted one line at a time rather than
  inherited** ([0081](../decisions/0081.md)).
- **Authority is keyed on the namespace enclosing the code — longest prefix wins, a subtree is spelled
  `"Vendor\*"`, and the rule reaches an overriding file and a hand-vendored tree as readily as a fetched
  package. A required capability that is not granted fails the build; one a package declares *optional*
  compiles and throws only if reached, so a library degrades through `Core\Cap::has` instead of refusing to
  install** ([0112](../decisions/0112.md)).
- **Novis ships its own framework, split by `rule:core-api/tier-placement`'s six tests** — privileged halves in `Core`, the
  opinionated layer as the `nvs/web` package, no ORM and no runtime container
  ([0082](../decisions/0082.md)).
- **A background job is a row in a `Core\Db` table, so an enqueue commits with the write that caused it**,
  and delivery is at-least-once with bounded retries
  ([0084](../decisions/0084.md)).
- **The OpenAPI document is generated while compiling, and an `#[Api]` that contradicts the code is a
  compile error** ([0085](../decisions/0085.md)).
- **`#[Json\Derive]`/`#[Db\Derive]` generate a codec from a class's declared properties, and a failed decode
  reports every bad field at once** — a compiler-recognized attribute is matched by name, unlike
  `Core\Attributes` retrieval ([0071](../decisions/0071.md)).
- **Configuration is TOML, in a root-owned `nvs.toml`**
  ([0064](../decisions/0064.md)); it states defaults, not ceilings
  ([0005](../decisions/0005.md)).
- **A run mode is `development` or `production`, defaults to production, and only selects the defaults of
  four named directives** — no environment variable is ever read for it
  ([0091](../decisions/0091.md)).
- **Every developer-facing output is one closed record the sink in force renders as plaintext, JSON or
  HTML** — no call site names a format, and a dump reaches a response body only in development mode
  ([0092](../decisions/0092.md)).
- **`nvs ctl reload` replaces the whole config snapshot over a local socket — no control port, no token** —
  and a directive that still needs a restart is named in the result rather than ignored
  ([0078](../decisions/0078.md)).
- **`nvs service` registers this binary with the platform's service manager, storing one verbatim argv, and
  the installer is a sink that refuses any subcommand but `serve`/`run`**
  ([0093](../decisions/0093.md)).
- **`Core\Task::all`/`::map` return with nothing still running, and `afterResponse` keeps the request tree
  alive past the connection** — cancellation runs no user code
  ([0072](../decisions/0072.md)).
- **Scheduled work is a `[[schedule]]` block firing a `spawn script`, never an API** — `scope` is mandatory
  ([0073](../decisions/0073.md)).
- **`#[Route]` builds the route table while compiling, and the router stops at matching**
  ([0077](../decisions/0077.md)).
- **One method's repeated routes may share a `name` when they share a `path`, and there is no wildcard
  verb** — `methodsFor` owes an exact `Allow:` header and CSRF is classified per verb
  ([0110](../decisions/0110.md)).
- **The server matches each request once before the handler, and `Core\Request::route()` is that match**
  ([0102](../decisions/0102.md)).
- **A capture narrows to a closed set with a type and never with a regex**, because a pattern over the
  request path runs before rate limiting
  ([0102](../decisions/0102.md) § 5).
- **A link is built through `url`/`urlAbsolute`, and an absolute one's origin is configured per mount,
  never read from a header**
  ([0102](../decisions/0102.md) § 6).
- **The terminal is a sink that substitutes control bytes visibly, styling is the `Cli\Text` value type,
  and `#[Command]` builds the argument table while compiling**
  ([0086](../decisions/0086.md)).
- **The runtime exports what it already measures, and a metric label refuses `tainted`**
  ([0076](../decisions/0076.md)).
- **A test is a `#[Test]` method whose table is built while compiling, and every test is its own isolate** —
  assertions are generic, so a type-mismatched comparison never runs
  ([0079](../decisions/0079.md)).
- **Coverage, tracing and profiling are always-emitted, flag-gated probes, never a second compiled tier**
  ([0018](../decisions/0018.md)); GC and spawn events
  are instrumented in their own routines, off the hot path
  ([0041](../decisions/0041.md)).
- **The on-disk artifact cache is one immutable file per unit, verified before any page is made executable**
  ([0042](../decisions/0042.md)).
- **A portable single-file executable ships source appended to the host binary**
  ([0048](../decisions/0048.md)).
- **`nvs fmt` has one unconfigurable style, never reflows, and is never wired into `nvs check`**
  ([0039](../decisions/0039.md)).
- **`nvs fmt` changes layout only — it never renames, never supplies a missing keyword, and never reorders
  class members; an editor composes it with quick fixes on save, which is a client concern**
  ([0039](../decisions/0039.md) §§ 9-11,
  [0040](../decisions/0040.md) § 3).
- **`nvs convert` is one deterministic rule table read through two modes, and an "identical" rewrite is one
  a differential case against the PHP oracle proves**
  ([0089](../decisions/0089.md)).
- **`nvs-syntax` exposes a second, lossless, error-recovering parse entry point for editor tooling only**
  ([0040](../decisions/0040.md)); IDE smarts live once, in `nvs-lsp`
  ([0016](../decisions/0016.md)).
- **There is no browser compile target** — the wasm32 one is retired, and wasm reaches Novis only as the
  *host* an `.nvsx` extension runs inside ([0003](../decisions/0003.md)); what the target would have
  cost, and what would reopen it, is [0025](../decisions/0025.md) *Revisiting*.
- **Performance history is callgrind instruction counts on a dedicated Linux runner, never raw wall-clock
  across machines** ([0026](../decisions/0026.md)).
- **`|>` substitutes the hole `$_`, required exactly once, in the parser — so a pipeline is the same AST
  the nested call produces, and it is not PHP 8.5's callable-applying `|>`**
  ([0098](../decisions/0098.md)); scalars still gain no methods
  ([0063](../decisions/0063.md) R19).
- **`expr catch (Class $e) => value` guards one expression and lowers to the block form** — arms chain
  like clauses, an arm holds an expression so `throw` is in and `return` is out, the result type is the
  union of the sides, and `finally` stays the block form's alone
  ([0119](../decisions/0119.md)).
- **`class<T>` is a type whose value is the run-time class descriptor, and `as` is its only source** —
  `new $cls(...)`, `$cls::f()` and `$x instanceof $cls` accept it and nothing else, it widens with its
  argument and never back, and a `new` over it is refused when an implementor of `T` declares an
  incompatible constructor
  ([0125](../decisions/0125.md)).
- **`property<T>` is a type whose values are `T`'s public declared property names, and `as` is its only
  source** — `$obj->$key` is admitted for that operand alone and `E0235` moves to the checker for every
  other, a read is the union of the set's types, a write is the checked erased store, and a class with a
  `readonly` property refuses the write
  ([0126](../decisions/0126.md)).
- **One grammar and one tree: the resilient parse is the AST plus a trivia layer and an offset index,
  `nvs-lsp` is synchronous on `lsp-server`/`lsp-types` so no async runtime enters the workspace, an LSP
  answer is frozen as a `.lspt` case, and syntax highlighting is two layers with two tests**
  ([0099](../decisions/0099.md)).
- **The editor offers a value only where the compiler already derived it for another reason** — a route
  name, a config directive, an `#[Api]` field — never from a convention scan, an annotation dialect or a
  network request, which is the whole of Novis's framework support; one workspace index answers references,
  occurrence highlight, CodeLens, type hierarchy and unused-member dimming; and an inline-HTML region gets
  the editor's own HTML/CSS/JS services but **no** formatter beside `nvs fmt`
  ([0108](../decisions/0108.md)).
- **A PHP built-in completes to its Novis destination: every name the differential oracle's inventory holds
  is a candidate, the migration table is the answer, and an item inserts only a member the `Core` registry
  already holds — so a dropped, undecided or not-yet-built destination appears and types nothing**
  ([0111](../decisions/0111.md)).
- **An implemented `Core` member's reference documentation lives in its registry declaration — every row,
  enum and constant carries its card, and the crate's tests refuse one without — and `nvs meta --json`
  prints it; the spec stays authoritative for the surface, and a doc field the registry carries wins over
  the spec, field by field**
  ([0117](../decisions/0117.md)).
- **An `array<mixed>` annotation narrows to the type of the literal under it through one editor action —
  element types widened to their base and joined, offered only where that literal is right there, never
  where the value arrived from input, and never on save because `array<T>` is invariant — and the synthesis
  it calls sits in `nvs-types` unreachable from every compile path**
  ([0114](../decisions/0114.md)).
- **A `secret` value's bytes are concealed in the editor by default, on ranges the server computes and the
  client only draws; `tainted` gets no default decoration, because how a construct looks is the user's
  theme's to decide** ([0101](../decisions/0101.md);
  the token modifiers themselves are [0099](../decisions/0099.md) § 4).
- **Against Python Novis claims the tool that gets handed over, never speed or replacement: a file opening
  `#!` starts in code mode with no tag, there is no REPL, and the userland suite measures three engines**
  ([0100](../decisions/0100.md)).
- **The configuration is a tree of files resolved as one ordered stream where later wins and every override
  is reported; every file in it must be unwritable by any account but its owner, and an absent `optional`
  include puts that check on the directory that would hold it**
  ([0103](../decisions/0103.md)).
- **An application is its entry file path: `[[app]]` is keyed on a canonicalized `root` prefix or exact
  `entry`, every matching block layers least-specific first, and a block may grant as well as narrow,
  bounded by the global `[limits.hard]`** ([0104](../decisions/0104.md)).
- **A capability is checked inside the one function that performs the effect, never beside it, and what
  each member needs is declared in one auditable table no execution path reads** — a member needing none
  pays nothing, and a denial is a catchable `RuntimeError` naming the capability
  ([0118](../decisions/0118.md)).
- **An isolate's arena is an ownership root, not an address range: entering one maps nothing, releasing one
  wholesale is a drain of the refcount worklist plus a sweep of the cyclic objects the refcounts could not
  free — both run native teardown — its statics base is its own, and
  a crossing at refcount 1 is a pointer handoff** ([0116](../decisions/0116.md)).
- **The image component is a Novis builder over a handful of coarse sandboxed entry points, one crossing
  per terminal, with one pixel model, a header-read pixel cap, upright colour-correct decoding and
  metadata stripped on encode — and none of gd's palette mode, mode flags, drawing primitives or formats**
  ([0120](../decisions/0120.md)).
- **PDF generation is a first-party Tier 1 component, unscheduled, that renders HTML with no I/O — assets
  are bytes passed in, an unresolved reference throws, unsupported CSS is dropped and reported, the output
  is inert and byte-reproducible, and the programmatic path is a builder emitting the same HTML**
  ([0121](../decisions/0121.md)).
- **HTML parses by the WHATWG algorithm through an entry on `Core\Html` — never a mode of `Core\Xml`,
  never failing where `Core\Xml` refuses — producing `Core\Xml`'s own tree over the parser the PDF
  engine shares, and landing in whatever milestone builds `Core\Xml`**
  ([0122](../decisions/0122.md)).
- **Spreadsheets are a first-party Tier 1 component, unscheduled — workbooks cross as bytes with nothing
  fetched and nothing executed, a formula is a typed value so a string is always a text cell, macros are
  never written, CSV stays `Core\Csv`, and equal input gives byte-equal output**
  ([0123](../decisions/0123.md)).
- **A PDF page is a decode source of the image component — `open` takes `page` and `dpi`, `info`'s
  `frames` is the page count read without a pixel, the page box at the requested dpi is priced against
  the pixel cap before any buffer, encryption and an undecodable embedded object throw, and text
  extraction stays third-party** ([0128](../decisions/0128.md)).
- **The end of a script is observable: `Core\Script::onExit` hooks run FIFO as the last user code at
  every non-fatal ending — normal, `exit`, uncaught throw — never on a `FATAL` or a cancellation, and
  they observe the ending rather than change it** ([0127](../decisions/0127.md)).
- **`Core\Password::verify` and `::needsRehash` read a PHP-stored bcrypt hash — `verify` verifies it,
  `needsRehash` answers `true` for every one — while `hash` writes only Argon2id, so a migrated user
  table upgrades itself one login at a time**
  ([0129](../decisions/0129.md)).
- **Usage telemetry is two independent opt-ins, both off by default: any `nvs` invocation records a
  closed counter set locally — the subcommand that ran, never argv and never anything from request
  traffic — and last week's aggregate uploads at most weekly, riding a short-lived command with a
  3-second budget that can never fail it; a process serving traffic records but never uploads**
  ([0130](../decisions/0130.md)).
- **A temporary directory dies with its script: the runtime deletes what `Core\IO::temporaryDir` handed
  out when the script ends, without ever throwing, and reclaims a dead process's leftovers — keyed on
  owner liveness, never age — only at `nvs serve` boot and under `nvs tmp clean`; there is no
  `temporaryFile` and no per-call persist, only the reloadable operator key `[debug] keep_temporary`**
  ([0131](../decisions/0131.md)).
- **A database driver is a borrowed sans-IO codec plus a state machine we write, over the parking stream** —
  the five live in `crates/nvs-db` as an enum rather than behind a trait, TLS is `nvs-host`'s one client
  generalised over its transport, and a connection whose wire is not at a known message boundary is closed
  rather than reset ([0132](../decisions/0132.md)).
- **A shipped feature is not finished until four proofs exist for it** — its behaviour pinned from Novis
  *and* from Rust, three real-world examples under `docs/examples/`, one measured figure in
  `benches/members/` recorded against a machine fingerprint, and one program in `tests/hostile/` written
  to break it; the roster of features is derived from `nvs meta --json` and the reference chapters rather
  than kept anywhere, and `python tools/dossier.py` is the whole mechanism
  ([0134](../decisions/0134.md)).
- A push runs only the jobs its diff can break; everything else runs nightly and at release, and no
  job exists in two workflows ([0143](../decisions/0143.md)).
- **A schema is a value over a closed vocabulary and a plan is the difference between it and a live
  server — there is no version number, no raw SQL inside a schema, no DDL parser, and absence never
  drops anything** ([0145](../decisions/0145.md))
  ([0145](../decisions/0145.md)).
