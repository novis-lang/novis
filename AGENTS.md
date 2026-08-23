# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This is the only file you always need to read. It carries the rules that are easy to get wrong, and names
the two places that route you to everything else.

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named as the home is authoritative and the other is a bug — fix it rather than reconciling it in your head.

Any agent, any harness. `CLAUDE.md` at the root is a pointer to this file; nothing here is Claude-specific.

## Where to look

Do not read the docs tree breadth-first: it is several hundred KB and only grows as ADRs are added, and
most of it is reasoning you only need when you are about to overturn a decision. Two things route you:

**1. Run `python tools/brief.py` first, every session.** One call: the plan's status block, a one-line map
of every milestone plus the lead of the current one, the one-line title and status of every ADR, what the
guard tests actually hold with their thresholds, and what exists on disk. It stores no facts — it slices
the live files and names each source, so it cannot go stale, and it says so loudly if a slice comes back
empty. It deliberately does not print each ADR's full rule or a milestone's full text — open the file it
names, or the plan at the line number it prints, for those.

**2. [docs/adr/README.md](docs/adr/README.md) § *Where to look*** is the topic → file routing table: one
row per topic, naming the one file that owns it. `python tools/brief.py --where <keyword>` prints just the
rows that match, which is usually faster than opening the file. That table lives there rather than here
because it grows a row per ADR, and this file is read in full at the start of every session.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[ADR 0004](docs/adr/0004-memory-for-simplicity.md).

1. **Security and request isolation** — not traded for anything.
2. **Correctness of language semantics** — PHP-compatible observable behaviour.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation.
5. **Memory footprint** — last, and spent deliberately to buy any of the above.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. MWL is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or ADR
  that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## Ground rules enforced elsewhere

Each bullet is the one-sentence rule; the full mechanism, the exact spellings rejected, and the reasoning
live only in the ADR it links to. **When you add a new decision, add one bullet here — not a paragraph.**
If you find yourself restating more than a sentence, that detail belongs in the ADR instead.

- **`unsafe` is forbidden workspace-wide**; only `mwl-runtime`, `mwl-codegen`, `mwl-stdlib` and
  `benches/abi-probe` opt down to `deny` with narrow, reasoned allows. Lint policy is in
  [Cargo.toml](Cargo.toml).
- **Nothing unwinds through a JIT frame** — every call returns a checked status instead, never
  `extern "C-unwind"` ([ADR 0002](docs/adr/0002-error-propagation.md)).
- **Pure-Rust dependencies by default**, enforced by [deny.toml](deny.toml) in CI. A C dependency is
  accepted only against two questions — does attacker-controlled data reach it, and if so does it have a
  demonstrable, exceptional verification record — and is otherwise confined to wasm
  ([ADR 0051](docs/adr/0051-standard-library-tiers.md) § 4, which replaces "argued individually").
- **Extensions are sandboxed wasm, never `dlopen`** ([ADR 0003](docs/adr/0003-extension-system.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget** — the same value-crossing
  rules as cross-core worker dispatch, limits accounted at the request tree's root, never per isolate
  ([ADR 0006](docs/adr/0006-isolated-script-execution.md)).
- **Nothing is untyped, and no type ever changes by itself** — every binding declares a type; `mixed` is the
  one unchecked position; `int + uint` is a compile error; overflow throws rather than becoming a `float`
  ([ADR 0007](docs/adr/0007-explicit-type-system.md)). The one exception is a local: `var $name = expr;`
  infers and fixes its type from `expr`, sugar over the checker's existing synthesis path rather than a
  solver — a bare array-literal initializer is the one shape it refuses ([ADR 0037](docs/adr/0037-var-local-type-inference.md)).
  A `foreach` binding and a destructuring target have no such spelling; inferring those still belongs only
  in `mwl convert`, never in the compiler.
- **`string` is guaranteed-valid UTF-8; binary data is the separate `bytes` type** — not yet Accepted, see
  the table above ([ADR 0009](docs/adr/0009-string-and-bytes.md)).
- **Every function is a method, every constant a class constant** — no free function, no global constant,
  no exception for the standard library; built-ins live under `Core` domain classes
  ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)).
- **`static` marks a class member; nothing else holds state behind a function's back** — no function-scope
  `static`, no `static fn`, no `global`; a second per-isolate slot table for one would undo this decision
  ([ADR 0008](docs/adr/0008-static-and-global.md)).
- **No variable is ever populated by the host** — PHP's superglobals become `Core` accessor classes;
  `$GLOBALS`/`$_REQUEST` have no replacement at all ([ADR 0012](docs/adr/0012-no-superglobals.md)).
- **Ordering two objects requires the global `Comparable` interface** — there is no property-walk fallback,
  and no cross-class overload ([ADR 0013](docs/adr/0013-comparable-interface.md)).
- **A property access runs its own hook, then a declared `PropertyObserver`, in that order** — accessing an
  undeclared property is always a hard error, and `__call`/`__callStatic` do not exist
  ([ADR 0014](docs/adr/0014-property-observer.md)).
- **Nothing gets a second runtime-reachable name** — no `class_alias`, no import `as`; a compile-time-only
  `type` alias for a type expression is the one exception ([ADR 0015](docs/adr/0015-no-name-aliasing.md)).
- **There is no `trait`.** Shared behavior is an interface method with a body — `public` is an inherited,
  overridable default, `private` is an internal-only helper — and shared state is explicit
  `implements Interface by $field;` delegation to an ordinary property. A method reachable from more than one
  default/delegated source with no class override is always a compile error; there is no `insteadof`
  ([ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)).
- **Compiled code is the only thing a request shares with any other** — an edited source file is picked up
  by revalidating a small per-path pointer, never by a filesystem watcher or a process restart, and no
  request-serving core is ever blocked on a recompile it did not ask for
  ([ADR 0017](docs/adr/0017-hot-reload-without-restart.md)).
- **Coverage, tracing and profiling are always-emitted, flag-gated probes, never a second compiled tier** —
  they must be start/stoppable mid-request, and `[debug] mode` is `RuntimeTighten` so no request can turn on
  more observability than the operator's `mwl.toml` ceiling allows ([ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)).
- **Architecture assumptions are tested, not remembered.** [benches/abi-probe/](benches/abi-probe/) guards
  the ABI, coroutine, sandbox and cost claims on every CI run. If a change makes one of those tests fail,
  the ADR it points at needs revisiting — do not adjust the threshold to make it pass.
- **Reflection and AST parsing are built into `Core`, not left to extensions** — `Core\Reflect` enforces the
  same visibility/hook checks as ordinary code (no `setAccessible(true)`), and `Core\Ast::parse()` returns
  inert typed data through the compiler's own single parser, never a second grammar or a path back into
  execution ([ADR 0019](docs/adr/0019-reflection-and-ast-parsing-are-core-features.md)).
- **A fatal error never reaches an ordinary `catch`** — a resource-limit report is not a `Throwable` at the
  type level; it and every other unhandled failure (uncaught throw, internal panic, compile error) escalates
  through a zero-retry, reserved-budget handler ladder ending in a hardcoded engine floor, so nothing goes
  unlogged and no handler can loop trying ([ADR 0020](docs/adr/0020-error-escalation-ladder.md)).
- **`require` is the only same-frame file-inclusion construct** — `include`, `include_once` and
  `require_once` are all rejected with a diagnostic naming `require`; it already throws on a missing file
  and runs every time it is reached, so no second spelling was needed for either axis
  ([ADR 0021](docs/adr/0021-single-file-inclusion-construct.md)).
- **A name reaches its file through a compile-time `autoload` declaration, never a runtime loader** — paths
  resolve relative to the file that declares it, so no manifest, walk-up search or `mwl.toml` directive is
  involved; a file reached that way declares exactly one thing, and `Core\Program::implementing<T>()` is the
  one query answering what nothing references by name — the only thing in MWL that makes a compiled unit
  depend on a directory's contents ([ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md)).
- **Nothing depends on the case something was typed in, or on the filesystem's opinion of it** — names
  resolve case-sensitively, keywords/contextual keywords/`<?mwl` are lower case and nothing else, and a
  `require`/`autoload` path is compared to the on-disk entry exactly, so a mis-cased path never compiles
  clean on any OS; a mis-cased keyword gets no diagnostic of its own, because ADR 0029/0032 make `IF` a
  legal class name ([ADR 0062](docs/adr/0062-case-sensitivity-is-a-compiler-property.md)).
- **Every constructor must definitely assign every property it declares** — the same flow analysis that
  checks local variables, extended to a second binding kind; there is no new `undefined` type and no
  per-type silent default, and the one residual runtime throw is scoped to a `Core\Reflect`-constructed
  object that never ran a constructor at all ([ADR 0022](docs/adr/0022-definite-property-initialization.md)).
  The one opt-out is `lateinit` on a non-nullable class/interface-typed property, which reuses that same
  throw instead of the constructor obligation — refused on `?T`, a promoted parameter, or alongside
  `readonly` ([ADR 0038](docs/adr/0038-lateinit-property-modifier.md)).
- **`mwl fmt` has exactly one style, is unconfigurable, and never reflows a wrapped expression to fit a
  width** — PER-based, gofmt's preserve-the-author's-line-breaks model rather than Prettier's, and never
  wired into `mwl check`/`mwl run`: an unformatted file is not a diagnostic, only something `mwl fmt --check`
  can report ([ADR 0039](docs/adr/0039-canonical-code-formatting.md)).
- **`mwl-syntax` exposes a second, lossless, error-recovering parse entry point alongside its strict one**
  — used only by editor tooling (`mwl-lsp`, starting in the new milestone M4B, ahead of M10), never by
  `mwl check`/`mwl run`, so a document that is mid-edit and syntactically invalid still yields usable
  completion/hover around the error ([ADR 0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md)).
- **A GC pause or an isolate spawn/join is instrumented inside its own already-rare runtime routine, never
  inside the safepoint poll or any per-statement/per-call site** — so a request's timeline can show them
  without adding a single check to the hot path ADR 0018 already measures
  ([ADR 0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md)).
- **The on-disk artifact cache is one immutable file per compiled unit, keyed by content hash *and* target/
  compiler environment, verified by mmap-then-checksum before any page is ever made executable** — a bad
  entry is always an ordinary cache miss, never a panic, and the checksum is never treated as a substitute
  for refusing a world-writable or wrong-owner cache directory
  ([ADR 0042](docs/adr/0042-on-disk-artifact-cache-format.md)).
- **`clone` is PHP's shallow, same-heap copy; `serialize`/`unserialize` share one recursive graph-copy
  operation with the `spawn`/`spawn worker`/`spawn script` boundary** — neither depth has a customization
  hook (no `__clone`, `__serialize`, `__unserialize`, `__sleep`, `__wakeup`), and `unserialize()` accepts
  only bytes MWL's own `serialize()` produced ([ADR 0023](docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)).
- **A wasm32 browser target is a second codegen backend behind the same IR, not a second language** —
  `spawn worker`/`spawn script`, coroutine-based suspension, and `.mwlx` extensions are unavailable there
  (a diagnostic, never a silent downgrade), and a seventh `Core` accessor domain covers DOM/window state;
  every other language and stdlib feature is unchanged across targets
  ([ADR 0025](docs/adr/0025-wasm-browser-target.md)).
- **Untrusted input carries a `tainted` qualifier that a handful of `Core` sinks (HTML output, SQL text,
  process arguments, HTTP headers, filesystem paths) refuse until it is laundered by a narrow, sink-named
  `Core` function** — the qualifier is compile-time-only, and the HTML sink additionally auto-escapes any
  non-`Markup` value by default, the one deliberate exception to "nothing happens implicitly"
  ([ADR 0024](docs/adr/0024-taint-tracking-for-injection-sinks.md)).
- **A confidential value carries a `secret` qualifier, independent of and composable with `tainted`, that
  HTML output, `Core\Log`, debug dumps, `Throwable` messages, and serialize/isolate-crossing all refuse by
  default** — compile-time-only like `tainted`, but with no ambient source and no runtime memory zeroing;
  `Core\Secret::reveal()` is the one narrow, greppable way to remove it outside a checked conversion
  ([ADR 0033](docs/adr/0033-secret-qualifier-for-confidential-values.md)).
- **`Core\Process` is the only way to run another program, and it never accepts a shell string** — an
  executable path plus an `array<string>` argv only, no `$useShell` flag and no shell-invoking convenience
  method; a Windows `.bat`/`.cmd`/`.ps1` target is refused outright since no argv quoting closes the
  `cmd.exe`/`powershell.exe` re-parsing gap that has already produced real CVEs elsewhere, and both
  `run()`'s wait and every `spawn()` read/write suspend the calling coroutine rather than blocking a worker
  thread ([ADR 0044](docs/adr/0044-core-process-argv-only-no-shell.md)).
- **`object` is the opaque top of every class type, and `{a: 1, b: 2}` builds an anonymous, methodless
  instance of it with no declared class** — an inline `{name: T, ...}` shape type is the one deliberate,
  tightly scoped exception to MWL's otherwise fully nominal typing, checked structurally by width subtyping
  at each call/assignment site; `Comparable`, `PropertyObserver` and ordinary `interface` satisfaction stay
  exactly as nominal as before ([ADR 0036](docs/adr/0036-anonymous-object-shapes.md)).
- **Cross-machine/OS performance history is tracked by callgrind instruction counts on a dedicated,
  non-shared Linux/WSL runner, never by comparing raw wall-clock across machines** — per-PR CI regression
  guards keep using `perf_guards.rs`'s self-relative wall-clock ratios everywhere, unchanged
  ([ADR 0026](docs/adr/0026-performance-measurement-methodology.md)).
- **`callable` is satisfied by exactly one shape of value, never PHP's string/array callable spellings, and
  MWL has no `__invoke`** — no object can ever be called with `()` syntax; taking a reference to a method or
  function is always first-class callable syntax (`Foo::bar(...)`/`$obj->method(...)`), which already
  produces that one statically resolvable value ([ADR 0027](docs/adr/0027-callable-is-closures-only.md)).
- **`fn` is the only closure literal (with or without a body), closures have no `use` clause of any kind, and
  `callable` is the only surviving type name** — `Closure` does not exist as a separate spelling; a closure
  that must call itself carries an optional self-name visible only inside its own body, and sharing mutable
  state between two independent closures is an ordinary object in user code, not a language feature
  ([ADR 0031](docs/adr/0031-callable-is-the-only-closure-type.md)).
- **`Stringable` replaces `__toString`; MWL has no destructors, `__debugInfo`, or `__set_state` at all, and
  `unset()` on a declared object property is always a diagnostic** — closing every PHP magic method no
  earlier ADR addressed, in one place, including why a destructor has no sound spot to report a throw and
  why keeping one would undo the wholesale-heap-drop request model
  ([ADR 0028](docs/adr/0028-closing-the-remaining-magic-methods.md)).
- **Identifier casing is a hard compiler error with no suppression mechanism** — `PascalCase` for
  classes/interfaces/traits/enums/enum-cases/namespace segments, `camelCase` for methods/properties/
  parameters/locals, `SCREAMING_SNAKE_CASE` for class constants
  ([ADR 0029](docs/adr/0029-identifier-casing-is-checked.md)).
- **An identifier's casing check inspects only its first character** — an all-caps acronym anywhere in a
  name (`HTTPClient`, `parseXMLPayload`) is accepted; ADR 0029 § 1's "acronyms are one word" rule is revoked
  ([ADR 0032](docs/adr/0032-acronym-casing-rule-revoked.md)).
- **No identifier may start with `_`, ever, and the constructor is spelled `constructor`** — not PHP's
  `__construct` — so the casing check has zero exceptions, of any kind, for any category
  ([ADR 0030](docs/adr/0030-no-leading-underscores-constructor-spelling.md)).
- **`as` is the only conversion spelling** — PHP's legacy `(int)$x`/`(string)$x` cast syntax does not parse
  at all, diagnosed at parse time naming the equivalent `as` expression, with no alias kept
  ([ADR 0034](docs/adr/0034-legacy-cast-syntax-rejected.md)).
- **`as ?T` converts without throwing, yielding `null` where `as T` would throw** — the same operator over
  a nullable target the grammar already accepted, so a failed conversion is a value you test rather than
  control flow you catch; `null` in gives `null` out, a conversion that cannot fail or does not exist is a
  diagnostic instead, it never launders `tainted`/`secret`, and it is why `Core\Validate` has no
  `isInteger` ([ADR 0066](docs/adr/0066-nullable-conversion-operator.md)).
- **A condition is the one place a value is tested without `as`** — `if`/`while`/`for`'s middle clause/`?:`/
  `&&`/`||`/`!` accept any type and resolve PHP's full truthy table at runtime; every other `bool` position
  (a parameter, property, `==`/`===`) still needs an explicit `as bool` or comparison
  ([ADR 0035](docs/adr/0035-truthy-boolean-context.md)).
- **`&&`/`||` are the only logical connectives** — PHP's `and`/`or`/`xor` keyword operators do not parse;
  `and`/`or` are diagnosed naming `&&`/`||` as the replacement, and `xor` is diagnosed with no one-token
  replacement at all, since MWL has no `^^` ([ADR 0045](docs/adr/0045-and-or-xor-keyword-operators-rejected.md)).
- **A `string`/`int` literal and a named enum case are each their own type, unioned to declare a closed
  set** — a class constant used in type position folds to its own literal type, but an enum case never
  folds to its backing value, since doing so would reopen the raw-int-accepted-where-enum-required hole
  ADR 0010 closed; there is no wildcard/glob spelling over constant or case names
  ([ADR 0047](docs/adr/0047-literal-and-enum-case-types.md)).
- **`#[...]` attributes are shape-literal metadata, never a declared attribute class** — a named form checks
  the literal against a `type` alias, a bare form checks nothing beyond well-formedness, every field value
  must be a compile-time constant, and retrieval is the narrow `Core\Attributes::get<T>`/`::all<T>` accessor,
  resolved structurally at compile time — not `Core\Reflect`'s general-purpose walk
  ([ADR 0046](docs/adr/0046-attributes-shape-literal-metadata.md)).
- **A portable single-file executable ships source, appended to the host binary, never precompiled
  artifacts** — `mwl build --compile` reuses the existing artifact cache and ADR 0025's static-`require`-graph
  rule unchanged, is CLI-only, and does not extend to bundling a web-serving deployment
  ([ADR 0048](docs/adr/0048-portable-single-file-executables.md)).
- **`<?mwl` is the only code-mode open tag, and `exit` is the only process-termination keyword** — `<?php`
  and `die` are each diagnosed at parse time naming the survivor; the lexer still recognizes both purely so
  the diagnostic can point at the fix, and neither reaches the AST as anything but a rejected construct
  ([ADR 0049](docs/adr/0049-single-open-tag-and-single-exit-keyword.md)).
- **`[...]` is the only destructuring spelling** — `list(...)` is diagnosed at parse time naming it, parsed
  through to its `;` only so the diagnostic can span the real statement, then discarded rather than reaching
  the AST; the per-leaf element grammar is identical either way, so nothing downstream ever saw the
  difference ([ADR 0050](docs/adr/0050-list-destructuring-spelling-rejected.md)).
- **A stdlib candidate is placed by six ordered tests, not by PHP's extension list** — runtime privilege,
  sink-or-launderer, waits-on-the-world, per-call cost, parses-hostile-bytes, would-two-exist — and the
  bloat being guarded against is API surface and the unsandboxed dependency set, never binary size or
  runtime memory; only Tier 0 may claim the `Core` prefix
  ([ADR 0051](docs/adr/0051-standard-library-tiers.md)).
- **Every `Core` member has the same shape: subject first, one trailing options shape, nothing mutates,
  failure throws and absence is `?T`** — and no operation is ever reachable two ways: no procedural twin of
  a class API, no mutable/immutable type pair, no `from`/`tryFrom` pair, no methods on scalars or
  `array<T>` ([ADR 0063](docs/adr/0063-core-api-conventions.md)).
- **Every third-party notice MWL owes is generated, committed and embedded in the binary** — a
  dependency added without its license text fails CI rather than shipping unattributed, and `mwl info`
  (`mwl -i`) prints the build facts plus the component table, `--licenses` the full texts
  ([ADR 0065](docs/adr/0065-third-party-attribution-and-mwl-info.md)).
- **Four doors stay shut: no FFI, no stream wrappers, no cross-request state, no `eval`** — each rejected
  from a commitment already made rather than on taste, and none has an opt-in, an ini flag or a trusted
  mode ([ADR 0052](docs/adr/0052-closed-doors.md)).
- **`Iterable`/`Iterator` are the only iteration interfaces, and generators lower to a state machine** —
  `ArrayAccess` and `Countable` do not exist, `foreach` accepts only an array, an `Iterable` or an
  `Iterator`, and `yield` is confined to the generator's own body so every compile target keeps generators
  ([ADR 0053](docs/adr/0053-iteration-and-generators.md)).
- **`decimal` is a scalar, not a class** — because MWL has no operator overloading, a class would mean
  method chains forever, which is the ergonomics gap that drives PHP to floats for money; `decimal ⊕ float`
  is a compile error on the same grounds `int ⊕ uint` is, division rounds half-even at a fixed
  unconfigurable scale, and `bcmath`/`gmp` are retired in favour of it plus `Core\BigInt`
  ([ADR 0054](docs/adr/0054-decimal-scalar-type.md)).
- **An extension's manifest can only tighten the qualifier analysis, never loosen it** — contagion applies
  at the boundary with nothing declared, a manifest may only add a refusal or add a taint, no extension may
  ever launder, and `secret` does not cross at all
  ([ADR 0055](docs/adr/0055-extension-qualifier-declarations.md)).
- **Regex runs on a linear-time engine by default** — backtracking is reached only by patterns the linear
  engine cannot express, under a step budget that **throws** rather than returning PHP's silent falsy
  value; a literal pattern's tier is known at compile time, and the *pattern* argument is a sink
  ([ADR 0056](docs/adr/0056-regex-engine-policy.md)).
- **The compiler knows a closed list of `Core` intrinsics and validates their literal arguments during
  checking** — a malformed pattern, URI or format string is a compile error, preparation is stored in the
  artifact cache, and the prepared and runtime paths share one implementation so behaviour cannot diverge
  ([ADR 0057](docs/adr/0057-intrinsic-literal-folding.md)).
- **An outbound URL is a sink, and the connection is made to a pinned address** — a `tainted` URL must pass
  `Core\Http::allowUrl`, which resolves, checks and pins; the `net.connect` capability carries an address
  policy denying loopback, private and link-local ranges by default, re-checked on every redirect hop
  ([ADR 0058](docs/adr/0058-outbound-request-policy.md)).
- **Cross-request state is explicit: `Core\Cache` is per-core, copied in and out, and charged to the core**
  — the local tier may lose any entry at any time and is never coherent across cores, `Core\Session` may
  not use it, and the memory it spends is O(cores × working set) under its own cap
  ([ADR 0059](docs/adr/0059-cross-request-state-is-explicit.md)).
- **Configuration is TOML, in a root-owned `mwl.toml`, read once at boot through `serde`** — a duplicate or
  unknown key is refused rather than silently taking the last one, a list is a real array and a ceiling's
  "off" a real boolean, and PHP's `ini_set`/`ini_get`/`ini_restore` are `Core\Config::set`/`::get`/
  `::restore`; `mwl.toml` is never the walked-up project manifest
  [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) rejected
  ([ADR 0064](docs/adr/0064-configuration-file-format.md)).
- **A closed four-entry roster of application-layer security protocols lives in `Core`** — signed cookies,
  CSRF, TOTP and JWT, each correct by construction (a JWT's algorithm comes from the key, never the token);
  stateless token operations are in, multi-step flows like OAuth and WebAuthn are permanently out
  ([ADR 0060](docs/adr/0060-application-security-protocols.md)).

## Commands

**A shell runs programs; it never carries file content.** Read, search, create and edit files with the
Read, Grep, Glob, Write and Edit tools — never `cat`, `head`, `tail`, `sed -n`, `grep`, `ls`, `find`, or a
heredoc that writes a file. This is not a style preference. A shell tool call is one `-c` string that the shell *parses*
before it runs anything, so an apostrophe in a doc sentence, a backtick in a commit message or an unbalanced
heredoc terminator fails the whole call with `unexpected EOF while looking for matching '` — the command
never executed, and nothing tells you which quote was at fault. The dedicated tools pass content as JSON
parameters with no shell in the path, so that failure cannot occur. This repo makes the problem worse than
most: prose full of apostrophes, backtick-quoted identifiers everywhere, and two shells with incompatible
quoting grammars (PowerShell primary, Git Bash for the Bash tool).

Use a shell for what it is for — `cargo`, `git`, `python tools/brief.py`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

**An Edit the tool cannot express goes through `python tools/splice.py <target> <old> <new>`** — write
the exact old and new blocks to files under `.agent-tmp/` (gitignored; create it if it is not there) with
the Write tool and let that script swap them.
It matches plain text and refuses anything but exactly one hit, so a stale anchor is an error rather than
a silent wrong edit. Never reach for a `sed`/`python - <<'PY'` one-liner instead: a heredoc is a shell
string, so it eats the backslashes and apostrophes this repository's Rust and prose are full of.

**One shell call runs one command, and its exit status is the last one's.** Do not `;`-chain several probes
into a single call to save a round trip. A chain reports only the final command's status, so a probe that is
*allowed* to fail — `ls` on a directory that may not exist returns 2 — marks the whole call failed while
holding a complete result, and the real output gets read as wreckage. `2>/dev/null` does not help: it
suppresses the message, not the status. When a command may legitimately fail, either give it its own call or
end it with `|| true`, and put a `&&` between steps that genuinely depend on each other.

```sh
cargo build                                                    # debug; deps still built at opt-level 2
cargo test                                                     # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

### Fuzzing and callgrind on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate, `cargo +nightly fuzz run lex|parse`) needs libFuzzer, and
`valgrind`/`callgrind` (the historical performance dashboard, [ADR 0026](docs/adr/0026-performance-measurement-methodology.md))
has no native Windows build at all — do both in WSL, not PowerShell/Git Bash. From a Windows shell,
`wsl.exe -- bash -lc "<command>"` runs a command straight in the default WSL distro, which mounts the
repo at `/mnt/d/swlang` (adjust the drive letter). One-time setup in that distro, first time only:

```sh
sudo apt-get update && sudo apt-get install -y build-essential clang valgrind
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

Then, from `/mnt/d/swlang` (not `fuzz/` itself — cargo-fuzz expects the parent directory):
`cargo +nightly fuzz run lex -- -max_total_time=300` (and `parse` likewise) for the 5 minute M1 verification
run; CI's `fuzz-smoke` job runs both for 60s on every push as a continuous regression check, same as the
plan's overall verification strategy calls for.

For the performance dashboard's instruction-count leg: `cargo build --release -p mwl-abi-probe --example
callgrind_spike`, then `valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out
./target/release/examples/callgrind_spike` — see [ADR 0026](docs/adr/0026-performance-measurement-methodology.md)
for what the count means and why it's the historical metric instead of wall-clock.

**A hand-written refcount protocol is where a leak hides, so check one before you commit it.**
`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh <fixture> …` runs `valgrind --leak-check=full` over the
`.mwl` files you name; that script's own header says why it exists next to the whole-suite sweep in
[tools/wsl-acceptance.sh](tools/wsl-acceptance.sh). Run it for **any** new refcount edge, against a
fixture that actually exercises it — this repository's one real leak went unnoticed until a fixture
happened to declare a refcounted local inside a loop.

## Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way:

- **State a fact once.** Put it in the home named in *Where to look*, and link to it from everywhere else.
  Summarising an ADR into another document creates a second copy that will silently go stale — that is how
  the plan came to carry a superseded ABI cost and a superseded unwinding design at the same time.
- **Never quote a measured number outside the ADR that owns it.** Numbers live with their guard test.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered ADR if the reasoning is subtle or
  contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](docs/adr/README.md).
- Crates for later milestones are created when their milestone starts, not left sitting empty.

### Length targets, and why nothing enforces them

These are the shapes that keep a doc readable at a glance. **Every one is guidance addressed to you, not
a check.** Nothing in this repository, in `tools/brief.py`, or in CI measures a line, a field or a file
against a number, and nothing ever will:

| Thing | Aim for |
|---|---|
| One field of the plan's status block | ~400 bytes, one overwritten paragraph |
| One `### Mn — title` milestone heading | one short line |
| One ADR index **Decision** cell | one sentence, ~160 bytes |
| One *Decisions taken at project start* title | one short phrase |
| One guard test's name + bounds | one line |
| `docs/agent/handoff.md` | ~80 lines — state, not a changelog |

Write to the target, and if a line lands a little over, **leave it**. This used to be a hard check that
failed CI and printed violations into the session digest, and the cost was not the bytes: it was five and
ten iterations per session spent shaving prose to clear a tripwire, at the end of a session, when the real
work was already done. A doc that is 10% over a target costs a reader nothing. Trimming it costs a session.

The one structural rule that *does* still matter is not about length: `tools/brief.py` may only print text
bounded by a **count of entities** — one line per milestone, per ADR, per guard test — never by a length of
prose. That is what keeps the digest from growing a page per milestone, and it holds no matter how long any
individual line gets.


## Session workflow

Every session runs the same five steps, in this order, and **stops**:

1. **Orient.** `python tools/brief.py`, this file, then `docs/agent/handoff.md` for what to pick up.
2. **Do the work.** One focused slice. Keep it small enough to finish.
3. **Verify what you touched** — `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
   `cargo fmt --check`, plus whatever the change specifically warrants (a `valgrind` run for a new refcount
   edge, per *Commands*). **This is the only place verification happens.**
4. **Write the docs and the handoff.** Update the plan's status block and any doc the change invalidates,
   then overwrite `docs/agent/handoff.md` with where the work stands now.
5. **Commit everything.** Then you are done.

**After step 5, stop.** Do not re-run `cargo build`/`test`/`clippy`/`fmt`, do not re-read the digest, do
not re-check a doc against a length. Writing prose cannot break a build, so there is nothing a second test
run could discover — it only burns a session's remaining budget after the useful work is finished. If step
4 or 5 turned up a real problem, fix it and re-verify *that* — otherwise the session is over.


## Keep work small, commit your work

Step 5 of *Session workflow* above, in detail:

- Always commit your work when a step is done. You don't need to review the history first — commit
  everything that has changed.
- The handoff is `docs/agent/handoff.md`: **overwrite it**, never append, so it describes where the work
  stands now rather than the path taken to get here. Its shape is in
  [docs/agent/session-prompt.md](docs/agent/session-prompt.md). Then show the user the same prompt in chat,
  so they can paste it into a new session.
- The docs accumulate rationale bloat as ADRs are added. Periodically (the user does this manually, you never automatically) re-run the pass
  captured in [docs/agent/doc-cleanup.md](docs/agent/doc-cleanup.md) rather than re-deciding its rules from scratch.
- Everytime we decide to add new features, change feature or remove features, decide and ask what the tradeoffs are in performance, memory, usability and simplicity for developers using the langauge. If there are huge tradeoffs, notify the user and ask for agreement before proceeding. If there are only benefits, just go ahead.
- [docs/implementation-plan.md](docs/implementation-plan.md)'s leading status block has a **fixed field
  set** — `Status`, `Done`, `On disk`, `Toolchain`, `ADR slices landed`, `Open now`, `Blocking`. Overwrite
  a field in place each session; never append a paragraph, and never add a field name — that is what stops
  the block drifting back into free prose that grows a paragraph per milestone. Session-by-session history
  already lives in `git log`; per-file known-gap detail belongs in that crate's own module doc comment, not
  in the plan. A milestone's own section is free to be as long as it needs — the digest prints only its
  heading and opening paragraph — but still name the ADR it draws a rule from and stop, rather than
  re-deriving the rule inline; that's what [docs/adr/README.md](docs/adr/README.md) already owns.