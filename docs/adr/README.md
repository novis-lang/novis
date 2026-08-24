# Architecture Decision Records

Each ADR records a decision that would be expensive to reverse, and *why* — so that a future reader can
tell a deliberate trade-off from an accident. Add one whenever a choice constrains later work.

Files get their own document only when the reasoning is subtle or contested. Decisions that are simply
recorded, with no live tension worth arguing, live in *Decisions taken at project start* below.

**Reading one.** Every ADR opens with a metadata block and an **In short** paragraph carrying the whole
decision. If you only need the rule, stop there. `Context`, `Alternatives rejected` and `Revisiting` exist
for when you intend to *change* the decision. Numbering starts at 0002 and has gaps where an ADR was folded
into another; the project-start section below is what 0001 would have been.

**An ADR's body always states the current rule.** When a later decision changes an earlier one, the change
is folded into the earlier ADR's text, and the two carry one-line cross-links — never a paragraph in one
file describing what another file changed. So there is no patch to apply while reading, and no such thing
as a section that was true once. If you find a body that disagrees with a cross-link, the body is the bug.

**Measured numbers.** Each ADR quotes only its own measurements, and every one is guarded by a test in
[`benches/abi-probe`](../../benches/abi-probe/). The tests are authoritative; a number written anywhere
else is a copy that can go stale.

## Where to look

One row per topic, naming the **one** file that owns it. Read the row, open the file, stop. The left column
is the search surface — it carries the keywords and PHP spellings you are likely to arrive with; the right
column is only the destination. `python tools/brief.py --where <keyword>` prints just the rows that match,
so you never have to open this file to route a topic.

| Doing this | Open this |
|---|---|
| Deciding what to build next; scoping a milestone; checking what exists | [docs/implementation-plan.md](../implementation-plan.md) — status block first. The plan of record. |
| Which file holds a thing; where a symbol is defined; what a module is for | `python tools/brief.py` — one line per module, plus the file:line of the definitions most often searched for |
| Something that looks like it should work and does not — a `Core` member's four required edits, a `.mwlt` case that skips a leg, an MWL shape that will not compile | [docs/agent/playbook.md](../agent/playbook.md) — the trap list, append-mostly |
| The *shape* of something you are about to write — a commit message, a `.mwlt` case, a `Core` member, an ADR, a diagnostic code, a splice patch | [docs/agent/conventions.md](../agent/conventions.md) — skeletons plus a worked example CI runs |
| Running the build, the tests, clippy and fmt; what "verified" means for a session | `python tools/verify.py`, defined in [AGENTS.md](../../AGENTS.md) § *Commands* |
| Exceptions, the call ABI, helper signatures, panic containment | [0002](0002-error-propagation.md) — the only normative copy of the calling convention |
| Extensions, wasm, WIT, `.mwlx` | [0003](0003-extension-system.md) |
| Whether a stdlib feature belongs in `Core`, in the default binary, in an extension or nowhere; which PHP extension maps to what; whether a C dependency is acceptable | [0051](0051-standard-library-tiers.md) |
| `FFI`, `dl()`, native modules, stream wrappers, `php://`/`phar://`, `shmop`/`sysv*`/APCu, `eval`, `putenv`, `setlocale`, or "why can't userland do X at all" | [0052](0052-closed-doors.md) |
| Whether a `.mwlx` can be an injection sink or source, `tainted`/`secret` at an extension call, what the manifest may declare | [0055](0055-extension-qualifier-declarations.md) |
| What a `Core` member looks like — argument order, options, failure signalling, naming, mutation, callbacks; whether a PHP built-in survives at all | [0063](0063-core-api-conventions.md) for the shape rules; [docs/spec/01-core-library.md](../spec/01-core-library.md) for every signature |
| "What happened to `<php_function>`?" — any PHP built-in by name, and whether it became a member, a construct or nothing | [docs/spec/02-php-migration.md](../spec/02-php-migration.md), one row per name; `python tools/check-migration.py --report` lists what is still undecided |
| Durations and dates — `30s`/`1h30m` literals, `strtotime`, `DateTime` arithmetic, `sleep`, timeouts, why there is no `shift` | [0070](0070-duration-literals.md) for the literal; [docs/spec/01-core-library.md](../spec/01-core-library.md) § 4 for `Core\Time` |
| Date *patterns* — `date()`/`strftime` letters, CLDR `yyyy-MM-dd`, which letters exist, quoting, why a month name is English and there is no locale | `crates/mwl-stdlib/src/cldr.rs`'s module docs — the one grammar `DateTime::format` and `Time::parse` share |
| Combining two arrays — `array_merge`, `array_replace`, `array_combine`, `$a + $b`, `array_merge_recursive`, why there is no `Arr::merge`, `preserveKeys`, `array_splice`, `array_pad`, `array_walk` | [0069](0069-array-combination-is-key-type-independent.md) |
| Weighing memory against safety, speed or simplicity | [0004](0004-memory-for-simplicity.md) |
| `mwl.toml` directives, `Core\Config::set`, limits, capabilities, changeability classes | [0005](0005-config-changeability.md) |
| The config file's *format* — why TOML, how a list/boolean/hash-pin is spelled, whether `mwl.toml` is a project manifest | [0064](0064-configuration-file-format.md) |
| Changing a running server — `mwl ctl reload`, the control socket, adding or replacing an extension without a restart, which directives still need one, why there is no control port | [0078](0078-config-reload-and-control-socket.md) |
| `spawn script`, isolates, the request boundary | [0006](0006-isolated-script-execution.md) |
| Caching between requests, APCu, `Core\Cache`, why a cached value is copied | [0059](0059-cross-request-state-is-explicit.md) |
| `include`/`require`, loading another file into the current frame | [0021](0021-single-file-inclusion-construct.md) |
| Case sensitivity, whether `IF`/`TRUE` parse, whether `new httpclient()` resolves, or why a `require` that works on Windows must work on Linux | [0062](0062-case-sensitivity-is-a-compiler-property.md) |
| Autoloading, `spl_autoload_register`, PSR-4, Composer's `vendor/autoload.php`, or enumerating classes nothing references by name | [0061](0061-compile-time-autoload-and-program-discovery.md) |
| Uninitialized properties, `undefined`, why a typed property can't silently be `null`/zero | [0022](0022-definite-property-initialization.md) |
| `lateinit`, deferring a property's first assignment past the constructor, DI/setter injection | [0038](0038-lateinit-property-modifier.md) |
| `clone`, `serialize`/`unserialize`, `__clone`/`__sleep`/`__wakeup`, or how a value crosses a `spawn` boundary | [0023](0023-clone-serialize-and-cross-boundary-copy.md) |
| The built-in HTTP server, live cache invalidation, picking up an edited `.mwl` file without a restart | [0017](0017-hot-reload-without-restart.md) |
| The on-disk compiled-artifact cache — file layout, header format, why a tampered file is a cache miss, eviction | [0042](0042-on-disk-artifact-cache-format.md) |
| A portable single-file executable, `mwl build --compile`, bundling a CLI app's source | [0048](0048-portable-single-file-executables.md) |
| Types, `uint`, `array<T>`, unions, `mixed`, conversions, array keys, arithmetic result types, user-defined generics | [0007](0007-explicit-type-system.md) |
| `decimal`, money, `bcmath`, `gmp`, big integers, why floats aren't used for currency | [0054](0054-decimal-scalar-type.md) |
| `string` vs `bytes`, the UTF-8 guarantee, text/binary conversion, what `length` counts | [0009](0009-string-and-bytes.md) |
| `foreach` over an object, `Iterator`/`ArrayAccess`/`Countable`, generators, `yield`, lazy streaming | [0053](0053-iteration-and-generators.md) |
| `var`, local type inference, why `$x = "foo";` doesn't need its type spelled out | [0037](0037-var-local-type-inference.md) |
| PHP's `(int)$x` legacy cast syntax, why it doesn't parse | [0034](0034-legacy-cast-syntax-rejected.md) |
| A conversion that shouldn't throw, `as ?int`, `tryParse`, validating untrusted input without `try`/`catch` | [0066](0066-nullable-conversion-operator.md) |
| Whether an `if`/`while`/`?:`/`&&`/`!` condition needs an explicit `as bool`, PHP truthiness | [0035](0035-truthy-boolean-context.md) |
| PHP's `and`/`or`/`xor` keyword operators, why they don't parse | [0045](0045-and-or-xor-keyword-operators-rejected.md) |
| `<?php` as an open tag, PHP's `die` keyword | [0049](0049-single-open-tag-and-single-exit-keyword.md) |
| `list($a, $b) = $pair;`, PHP's `list()` destructuring spelling | [0050](0050-list-destructuring-spelling-rejected.md) |
| Restricting a parameter to a fixed set of values (`#[ExpectedValues]`), `"a"\|"b"` literal types, a subset of an enum's cases | [0047](0047-literal-and-enum-case-types.md) |
| `#[Attribute]`-style metadata, annotations, `Core\Attributes`, why there's no attribute base class | [0046](0046-attributes-shape-literal-metadata.md) |
| Hydrating a class from JSON or a database row — `#[Json\Derive]`, `#[Db\Derive]`, `JsonSerializable`, `PDO::FETCH_CLASS`, serde-style derives, reporting every bad field of a submitted form | [0071](0071-derived-codecs.md) |
| Routing — `#[Route]`, URL patterns and `{id}` placeholders, reverse URL generation, why the router does not dispatch, `Core\Router` | [0077](0077-compile-time-routing.md) |
| Writing a CLI program — colour and `Cli\Text`, why `echo` neutralizes escape sequences, prompts and `select`, password input, progress bars and in-place output, `#[Command]`/`#[Option]` argument parsing, `--help` and shell completions, `isTty`, terminal width | [0086](0086-core-cli-terminal-is-a-sink.md) |
| Trojan Source, right-to-left overrides, `U+202E`, a comment that renders as code, homoglyphs, zero-width characters, or why a non-ASCII identifier does not compile | [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md) |
| Porting a PHP codebase — `mwl convert`, its two modes, what a `TODO(convert:…)` means, why the default output does not run, how a rewrite is proven, where the rule table lives, which PHP parser and which PHP versions | [0089](0089-convert-is-one-rule-table-with-two-modes.md) |
| Running several things at once — `Task::all`/`::map`, `parallel_map`, a task deadline, cancellation, work after the response is sent, `fastcgi_finish_request`, why there is no job queue | [0072](0072-core-task-structured-concurrency.md) |
| Cron, scheduled jobs, a nightly task, running something once across a fleet, `[[schedule]]` | [0073](0073-scheduled-work-is-config.md) |
| Response security headers, CORS, cookie defaults, HSTS, CSP; and outbound timeouts, retries, backoff, idempotency keys | [0074](0074-http-defaults-safe-and-finite.md) |
| Rate limiting, throttling logins, per-tenant quotas, `Retry-After`, `429`, load shedding | [0075](0075-core-ratelimit.md) |
| Metrics, Prometheus, OpenTelemetry, distributed tracing, `traceparent`, `Core\Metrics`, label cardinality | [0076](0076-observability-export.md) |
| Regex, `preg_*`, `Core\Regex`, ReDoS, backreferences, lookaround | [0056](0056-regex-engine-policy.md) |
| Why a literal regex/URI/format string is checked by `mwl check`, compile-time preparation | [0057](0057-intrinsic-literal-folding.md) |
| `enum`, enum cases, backing type, anything enum-shaped | [0010](0010-enums-are-a-value-type.md) |
| `static`, `global`, scoping, where state may live at all | [0008](0008-static-and-global.md) |
| Free functions, global constants, the `Core` namespace, where a built-in lives | [0011](0011-functions-and-constants-are-class-members.md) |
| `callable`, first-class callable syntax (`Foo::bar(...)`), `__invoke`, calling an object with `()` | [0027](0027-callable-is-closures-only.md) |
| Anonymous functions, `fn`, arrow functions, closure capture, `use (...)`, recursive closures | [0031](0031-callable-is-the-only-closure-type.md) |
| `__toString`/`Stringable`, `__destruct`, `__isset`/`__unset`, `unset()` on an object property, `__debugInfo`, `__set_state`, or "what happened to PHP magic method X" | [0028](0028-closing-the-remaining-magic-methods.md) |
| `stdClass`, an anonymous object literal `{a: 1}`, the `object` type, an inline `{name: T}` shape type | [0036](0036-anonymous-object-shapes.md) |
| Naming conventions, `PascalCase`/`camelCase`/`SCREAMING_SNAKE_CASE`, acronym spelling, identifier casing | [0029](0029-identifier-casing-is-checked.md) |
| Leading underscores in identifiers, whether the constructor is `__construct` or `constructor` | [0030](0030-no-leading-underscores-constructor-spelling.md) |
| `$_SERVER`, `$_GET`/`$_POST`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$argv`, or anything PHP populates ambiently | [0012](0012-no-superglobals.md) |
| Comparing two objects with `<`/`>`/`<=>`, operator overloading, `Comparable` | [0013](0013-comparable-interface.md) |
| `==` vs `===`, loose comparison, type juggling, why `"1" == 1` does not compile, what two strings/arrays/objects compare by, `__equals`, comparing a `mixed` | [0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) |
| What "the same value" means — strict identity, `in_array`'s strict flag, `array_search`, `array_unique`, whether two objects/arrays/`NaN`/`-0.0` match | [crates/mwl-runtime/src/identity.rs](../../crates/mwl-runtime/src/identity.rs) — one row per representation, and the hash that agrees with it |
| Property hooks, `__get`/`__set`, `PropertyObserver`, undefined properties, `__call`/`__callStatic` | [0014](0014-property-observer.md) |
| `class_alias`, `use … as …`, or a `type` alias | [0015](0015-no-name-aliasing.md) |
| `trait`, horizontal code reuse, mixins, `insteadof`, how a PHP trait migrates | [0043](0043-interface-default-methods-and-delegation-replace-traits.md) |
| Code coverage, call tracing, the per-call profiler, `Core\Debug`, the `[debug]` config section | [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) |
| A timeline/flame-chart view combining calls with GC pauses and isolate boundaries, exporting to speedscope | [0041](0041-timeline-export-and-gc-spawn-trace-events.md) |
| `Core\Reflect`, `ReflectionClass`-equivalents, `Core\Ast`, runtime introspection or source parsing | [0019](0019-reflection-and-ast-parsing-are-core-features.md) |
| Uncaught exceptions, memory/CPU-limit fatals, internal panics, `Core\Fatal`, `Core\Log` | [0020](0020-error-escalation-ladder.md) |
| XSS, SQL injection, command/header/path injection, taint tracking, `tainted string`, `Core\Html\Markup` | [0024](0024-taint-tracking-for-injection-sinks.md) |
| Whether a given parameter is a sink, what an unclassified one does, what `echo` writes to in a request / a CLI / an isolate / a scheduled run, how a JSON or plain-text response body is written, `Core\Response::json` | [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) |
| A database — `Core\Db`, `PDO`/`mysqli`/`pgsql`/`sqlite3`, drivers, connections, prepared statements, transactions, result rows, an ORM | [0067](0067-core-db.md) — signatures in [spec § 18](../spec/01-core-library.md) |
| SSRF, fetching a user-supplied URL, `Core\Http\Client`, the `net.connect` address policy, DNS rebinding | [0058](0058-outbound-request-policy.md) |
| `exec`/`system`/`shell_exec`/backticks/`proc_open`, running another program, shell injection | [0044](0044-core-process-argv-only-no-shell.md) |
| Passwords, API keys, credentials, `secret string`, why a value can't be echoed/logged/dumped | [0033](0033-secret-qualifier-for-confidential-values.md) |
| JWT, CSRF tokens, TOTP, signed cookies, or whether OAuth/WebAuthn/SAML belong in `Core` | [0060](0060-application-security-protocols.md) |
| A wasm32 browser target, running MWL client-side, `Core\Browser` | [0025](0025-wasm-browser-target.md) |
| The PhpStorm plugin, `mwl-lsp`/`mwl fmt` client wiring, what "IDE integration" covers | [0016](0016-ide-integration.md) |
| `mwl fmt`'s style — indentation, braces, quoting, trailing commas, why it never reflows | [0039](0039-canonical-code-formatting.md) |
| The VS Code extension's feature catalog, why a minimal `mwl-lsp` ships in M4B, `mwl-syntax`'s resilient parse mode | [0040](0040-vscode-deep-tooling-and-resilient-parsing.md) |
| Writing a test case — the `.mwlt` sections, `--EXPECTF--`'s escapes, `--ORACLE--`/`--ORACLE-DIVERGES--`, how `mwl test` decides pass or fail, importing a `.phpt` | [crates/mwl-test/src/lib.rs](../../crates/mwl-test/src/lib.rs)'s module doc — the one home for the format |
| Testing a program *written in* MWL — `#[Test]`, `Core\Test`, assertions, doubles, fixtures, parameterized cases, property testing, snapshots, `#[Bench]`, mutation testing, why PHPUnit's mechanism does not port | [0079](0079-testing-is-a-language-feature.md) — distinct from the `.mwlt` row above, which is MWL's own conformance suite |
| Third-party licenses, attribution, what `mwl info` prints, whether a new dependency's license may ship | [0065](0065-third-party-attribution-and-mwl-info.md) |
| Updating a crate, the Rust toolchain, a CI action or the PHP oracle; SemVer, what a break costs a release, deprecation, MSRV, pinning, a stale dependency | [0068](0068-dependency-currency-and-the-version-contract.md) for the policy; [docs/agent/dependency-update.md](../agent/dependency-update.md) for the procedure — a pass the user fires by hand |
| Cross-machine performance history, callgrind instruction counts, why CI guards use wall-clock ratios | [0026](0026-performance-measurement-methodology.md) |
| Concrete *spelling* an ADR left open — file modes, the declaration-slot grammar, `as`, the `bytes` literal | [docs/spec/00-overview.md](../spec/00-overview.md) — the ADR owns semantics, this owns syntax |
| How a control-flow statement lowers — `if`/`while`/`for`/`foreach`/`switch`/`match`, `break`/`continue`, where a `finally` runs | [crates/mwl-ir/src/lower/control.rs](../../crates/mwl-ir/src/lower/control.rs)'s methods, each with its own doc comment; `continue` inside a `switch` is § *Decisions taken at project start* below |
| A decision with no ADR — thread-per-core, value layout, safepoints, shared-nothing requests | § *Decisions taken at project start* below for **why**; the plan's § *Architecture* for the **mechanics** |
| Any measured number, or checking whether an architecture assumption still holds | the guard tests in [benches/abi-probe/](../../benches/abi-probe/) — authoritative; docs quote them and can lag |
| Who MWL is for, what it claims about itself, whether "PHP compatible" may be written anywhere, why this does not end where Hack ended, or which of two slices to build first | [0080](0080-the-audience-mwl-is-built-for.md) |
| Third-party libraries — the registry, a git dependency, `package.toml`/`package.lock`, `mwl add`/`fetch`/`update`/`vendor`/`audit`/`publish`, version resolution, dependency hell, supply-chain attacks, install scripts, what authority a dependency has | [0081](0081-packages-are-digests-resolution-is-a-maximum.md) |
| The framework — `mwl new`, controllers, middleware, auth flows, mail, i18n, storage, pagination, `Web\*`, where the ORM is, where the service container is, why there is no template engine | [0082](0082-the-first-party-framework.md) |
| WebSocket, SSE, a long-lived connection, push, broadcast, `Core\Topic`, presence, or what happens to an isolate when a socket outlives its request | [0083](0083-persistent-connections-are-isolates.md) |
| Background jobs, `Core\Queue`, retries, dead-letter, workers, `mwl work`, transactional enqueue, or why there is no Redis backend | [0084](0084-durable-background-jobs.md) |
| OpenAPI, Swagger, an API contract, `#[Api]`, generated docs, or detecting a breaking API change | [0085](0085-openapi-is-generated-from-the-route-table.md) |
| Connection pooling, a reused database connection, what a connection reset must remove, `cores × max` sizing | [0067](0067-core-db.md) § 13 |
| What the language should *do* beyond the two spec files | unwritten. `docs/spec/` holds `00-overview.md` and `01-core-library.md` and nothing else — say so rather than inferring semantics. |

**Adding a decision.** Touch exactly these, in order — nothing else should ever need its own copy:

1. Write the ADR file, following the shape every other one has: metadata block, **In short**, then
   `## Context` / `## Decision` / `## Consequences` / `## Alternatives rejected` / `## Revisiting` /
   `## Verification` as needed.
2. Add its row to the index below: number, one-line decision, status.
3. **If it changes a prior ADR's rule, edit that ADR's body to state the new rule** — in the same commit,
   not as a note about what changed. Then add a one-line `Amends:` to yours naming the ADR and section, and
   add your number to that ADR's bare `Amended by:` list. Nothing else. If folding leaves the earlier ADR
   with no unique content at all, delete it instead and update every reference; the number stays retired.
4. If the topic is one an agent will search for by keyword, add one row to the *Where to look* table above —
   the only one there is, since [AGENTS.md](../../AGENTS.md) points here rather than keeping a copy — and,
   only if it is a hard invariant, one **sentence** to AGENTS.md's *Ground rules enforced elsewhere*.
5. If it changes what a milestone builds, update that milestone's paragraph in
   [the plan](../implementation-plan.md) with a link and a headline, not a restatement.

`tools/brief.py` needs no update: it slices this file's tables and the plan's status block live. Two things
to get right in a new row, both for the reader rather than for a checker — nothing enforces either: keep the
**Decision** cell to one sentence, and write any `|` inside a cell as `\|`.

| # | Decision | Status |
|---|---|---|
| [0002](0002-error-propagation.md) | Exceptions propagate by checked return, not by unwinding | Accepted |
| [0003](0003-extension-system.md) | Extensions are sandboxed WebAssembly components, not native shared libraries | Accepted |
| [0004](0004-memory-for-simplicity.md) | Memory is spent for security, speed and simplicity, in that order | Accepted |
| [0005](0005-config-changeability.md) | `mwl.toml` states defaults, not ceilings | Accepted |
| [0006](0006-isolated-script-execution.md) | Running another script is an in-process isolate, not a subprocess | Accepted |
| [0007](0007-explicit-type-system.md) | Types are declared, checked, and never change by themselves | Accepted |
| [0008](0008-static-and-global.md) | `static` marks a class member; there are no function statics and no `global` | Accepted |
| [0009](0009-string-and-bytes.md) | `string` is text; binary data is a distinct `bytes` type | Accepted |
| [0010](0010-enums-are-a-value-type.md) | Enums are a closed, named integer type, not PHP's class-like construct | Accepted |
| [0011](0011-functions-and-constants-are-class-members.md) | Functions and constants are class members; `Core` is the reserved namespace for built-ins | Accepted |
| [0012](0012-no-superglobals.md) | There are no superglobals; request, session, environment and CLI state are `Core` accessor classes | Accepted |
| [0013](0013-comparable-interface.md) | Ordering two objects requires `Comparable`; PHP's property-walk fallback is rejected | Accepted |
| [0014](0014-property-observer.md) | Property hooks feed a declared `PropertyObserver`; no undefined-property fallback, no `__call`/`__callStatic` | Accepted |
| [0015](0015-no-name-aliasing.md) | No `class_alias` or import `as`; `type` aliases are the disciplined exception | Accepted |
| [0016](0016-ide-integration.md) | IDE integration is a thin per-editor client over one language server; PhpStorm goes LSP-bridge before native | Accepted |
| [0017](0017-hot-reload-without-restart.md) | The compiled-unit cache revalidates lazily and swaps a per-path pointer; no filesystem watcher, no restart | Accepted |
| [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) | Coverage, tracing and profiling are safepoint-shaped probes, not a second compiled tier | Accepted |
| [0019](0019-reflection-and-ast-parsing-are-core-features.md) | Reflection and AST/source parsing are first-class `Core` features, not aftermarket extensions | Accepted |
| [0020](0020-error-escalation-ladder.md) | Fatal errors escalate through a reserved-budget handler ladder, never through `catch` | Accepted |
| [0021](0021-single-file-inclusion-construct.md) | `require` is the only same-frame file-inclusion construct; `include`/`include_once`/`require_once` are rejected | Accepted |
| [0022](0022-definite-property-initialization.md) | Properties are definitely initialized at compile time; no new `undefined` type, no silent defaults | Accepted |
| [0023](0023-clone-serialize-and-cross-boundary-copy.md) | `clone` stays PHP-shallow; `serialize`/`unserialize` share one graph-copy operation with the isolate boundary; neither is hookable | Accepted |
| [0024](0024-taint-tracking-for-injection-sinks.md) | Untrusted input is a distinct type; injection sinks demand laundering | Accepted |
| [0025](0025-wasm-browser-target.md) | The browser is a second compile target, not a second language | Accepted |
| [0026](0026-performance-measurement-methodology.md) | Performance history is tracked by callgrind instruction counts; wall-clock stays for CI regression guards | Accepted |
| [0027](0027-callable-is-closures-only.md) | `callable` is satisfied only by a closure value; PHP's string/array callable spellings and `__invoke` are both rejected | Accepted |
| [0028](0028-closing-the-remaining-magic-methods.md) | `Stringable` replaces `__toString`; no `__destruct`, `__debugInfo`, or `__set_state`; `unset()` is refused on an object property | Accepted |
| [0029](0029-identifier-casing-is-checked.md) | Identifier casing is a hard compiler error, checked on the leading character only, with no suppression | Accepted |
| [0030](0030-no-leading-underscores-constructor-spelling.md) | No leading underscores anywhere; the constructor is spelled `constructor`, not `__construct` | Accepted |
| [0031](0031-callable-is-the-only-closure-type.md) | `fn` is the only closure literal, with no `use` clause; `callable` absorbs `Closure` as the one surviving type name | Accepted |
| [0033](0033-secret-qualifier-for-confidential-values.md) | `secret` is a second compile-time qualifier alongside `tainted`; HTML output, `Core\Log`, debug dumps, exception messages and serialize all refuse it by default | Accepted |
| [0034](0034-legacy-cast-syntax-rejected.md) | PHP's legacy `(T)expr` cast syntax is rejected; `as` is the only conversion spelling | Accepted |
| [0035](0035-truthy-boolean-context.md) | A condition is judged by PHP's full truthy table; every other `bool` position stays checked | Accepted |
| [0036](0036-anonymous-object-shapes.md) | `object` is the opaque top of every class type; `{...}` builds an anonymous methodless instance, and `{name: T, ...}` is MWL's one structurally-checked type | Accepted |
| [0037](0037-var-local-type-inference.md) | `var $name = expr;` infers a local's type from its initializer and fixes it forever; a bare array-literal initializer is the one shape it refuses | Accepted |
| [0038](0038-lateinit-property-modifier.md) | `lateinit` defers a non-nullable object property's first assignment past the constructor, throwing on read-before-write; `?T` and `readonly` are refused | Accepted |
| [0039](0039-canonical-code-formatting.md) | `mwl fmt` is one canonical, unconfigurable, PER-based formatting style with no reflow; it is never wired into the compiler | Accepted |
| [0040](0040-vscode-deep-tooling-and-resilient-parsing.md) | The VS Code extension goes deep (inspections, refactorings, Test Explorer, debugger UI) ahead of M10; `mwl-syntax` gains a resilient parse mode | Accepted |
| [0041](0041-timeline-export-and-gc-spawn-trace-events.md) | Trace events gain a `call`/`gc`/`spawn` kind and a speedscope-evented export, so a request's timeline shows GC pauses and isolate boundaries, not just calls | Accepted |
| [0042](0042-on-disk-artifact-cache-format.md) | The on-disk artifact cache is one immutable, self-describing file per compiled unit, verified before it is ever mapped executable | Accepted |
| [0043](0043-interface-default-methods-and-delegation-replace-traits.md) | There is no `trait`; interface default/private methods share behavior and explicit `by` delegation shares state, with one conflict rule and no `insteadof` | Accepted |
| [0044](0044-core-process-argv-only-no-shell.md) | `Core\Process` is the one argv-only way to run another program; there is no shell-string form, and a Windows batch/PowerShell target is refused outright | Accepted |
| [0045](0045-and-or-xor-keyword-operators-rejected.md) | PHP's `and`/`or`/`xor` keyword operators are rejected; `&&`/`\|\|` are the only logical connectives | Accepted |
| [0046](0046-attributes-shape-literal-metadata.md) | `#[...]` attributes are shape-literal metadata, checked structurally, retrieved via `Core\Attributes::get<T>`/`::all<T>` | Accepted |
| [0047](0047-literal-and-enum-case-types.md) | A scalar literal or a named enum case is itself a type; unioning them declares an explicit closed set, checked like any other conversion | Accepted |
| [0048](0048-portable-single-file-executables.md) | A portable single-file executable appends source to the host binary; rebundling is a build-time CLI step, not a runtime one | Accepted |
| [0049](0049-single-open-tag-and-single-exit-keyword.md) | `<?php` and `die` are rejected; `<?mwl` and `exit` are the only spellings kept | Accepted |
| [0050](0050-list-destructuring-spelling-rejected.md) | `list(...)` is rejected; `[...]` is the only destructuring spelling | Accepted |
| [0051](0051-standard-library-tiers.md) | Six ordered tests place every stdlib candidate at Core, Native, Ext, dropped, or already-answered; PHP's extension partition is not inherited | Accepted |
| [0052](0052-closed-doors.md) | Four closed doors: no FFI, no stream wrappers, no cross-request state, no `eval` | Accepted |
| [0053](0053-iteration-and-generators.md) | `Iterable`/`Iterator` are the only iteration interfaces; generators exist and lower to state machines | Accepted |
| [0054](0054-decimal-scalar-type.md) | `decimal` is a scalar type; `bcmath` and `gmp` are retired | Accepted |
| [0055](0055-extension-qualifier-declarations.md) | Extension manifests carry `tainted`/`secret`; every declaration tightens, none loosens | Accepted |
| [0056](0056-regex-engine-policy.md) | Regex runs on a linear-time engine by default; backtracking is opt-in and budgeted | Accepted |
| [0057](0057-intrinsic-literal-folding.md) | Literal arguments to a closed list of intrinsic `Core` calls are validated and prepared at compile time | Accepted |
| [0058](0058-outbound-request-policy.md) | Outbound connections carry an address policy; a tainted URL must be laundered and pinned | Accepted |
| [0059](0059-cross-request-state-is-explicit.md) | `Core\Cache` is per-core, copied in and out, and charged to the core rather than a request | Accepted |
| [0060](0060-application-security-protocols.md) | A closed roster of application-layer security protocols lives in `Core` | Accepted |
| [0061](0061-compile-time-autoload-and-program-discovery.md) | `autoload` maps names to files at compile time; `Core\Program::implementing<T>()` enumerates classes nothing names | Accepted |
| [0062](0062-case-sensitivity-is-a-compiler-property.md) | Names resolve case-sensitively, reserved spellings are lower case only, and a `require`/`autoload` path must match the on-disk entry exactly | Accepted |
| [0063](0063-core-api-conventions.md) | Twenty rules fix every `Core` member's shape: subject first, options as one shape, nothing mutates, failure throws, no operation reachable two ways | Accepted |
| [0064](0064-configuration-file-format.md) | Configuration is TOML in `mwl.toml`; `ini_set` becomes `Core\Config::set` | Accepted |
| [0065](0065-third-party-attribution-and-mwl-info.md) | Third-party attribution is generated from the dependency graph, committed and embedded in the binary; `mwl info` prints it with the build facts | Accepted |
| [0066](0066-nullable-conversion-operator.md) | `expr as ?T` converts without throwing, yielding `null` on failure; `Core\Validate`'s three numeric predicates go | Accepted |
| [0067](0067-core-db.md) | One database API: connections are named in root-owned config, every statement is prepared, and a transaction is a closure | Accepted |
| [0068](0068-dependency-currency-and-the-version-contract.md) | Dependencies stay current; a break in one is absorbed rather than forwarded, and only an enumerated user-facing surface can force a major | Accepted |
| [0069](0069-array-combination-is-key-type-independent.md) | Arrays combine by the member's name, never by a key's type: `overlay`/`underlay`/`appendAll`, no `merge`, and `array + array` does not compile | Accepted |
| [0070](0070-duration-literals.md) | A duration is a literal — `30s`, `1h30m` — typed `Duration` and folded to a constant, over one grammar shared with `Duration::parse` and `mwl.toml` | Accepted |
| [0071](0071-derived-codecs.md) | `#[Json\Derive]`/`#[Db\Derive]` generate a codec from a class's declared properties, and a failed decode reports every bad field at once | Accepted |
| [0072](0072-core-task-structured-concurrency.md) | `Core\Task::all`/`::map` return with nothing still running, cancellation runs no user code, and `afterResponse` keeps the request tree alive past the connection | Accepted |
| [0073](0073-scheduled-work-is-config.md) | Scheduled work is a `[[schedule]]` block firing a `spawn script`, with no API surface and a mandatory `scope` of `"fleet"` or `"host"` | Accepted |
| [0074](0074-http-defaults-safe-and-finite.md) | HTTP response defaults are secure with nothing configured, and an outbound call has no spelling for "wait forever"; retry is opt-in, jittered and deadline-covered | Accepted |
| [0075](0075-core-ratelimit.md) | `Core\RateLimit` limits what only the application knows, over the shared store; the approximate per-core tier is a differently-named member, and edge limiting is the proxy's | Accepted |
| [0076](0076-observability-export.md) | The runtime exports what ADRs 0018/0041 already measure, plus a three-member `Core\Metrics`; a label refuses `tainted`, and a full series registry refuses new series rather than evicting old ones | Accepted |
| [0077](0077-compile-time-routing.md) | `#[Route]` builds a route table while compiling, making a duplicate route, an unbound placeholder and a stale `url()` name compile errors; the router stops at matching | Accepted |
| [0078](0078-config-reload-and-control-socket.md) | `mwl.toml` reloads over a local-socket-only control API, validated whole before it is published; every directive says whether it needs a restart, and the extension set joins the key both compiled-unit caches share | Accepted |
| [0079](0079-testing-is-a-language-feature.md) | `#[Test]` builds the runner's table while compiling and every test is its own isolate; assertions are generic, a failure is catchable but ledgered, and doubles are closure shapes checked against an interface | Accepted |
| [0080](0080-the-audience-mwl-is-built-for.md) | MWL is built first for platforms running code or data they do not control; the pitch is isolation and qualifiers rather than speed, PHP syntax is an on-ramp and never a compatibility promise, and the framework and package story outrank new breadth | Accepted |
| [0081](0081-packages-are-digests-resolution-is-a-maximum.md) | A package is a content-addressed source archive resolved by minimal version selection, git sources are root-only, no package code runs before the program does, and a dependency's capabilities are granted per package rather than inherited | Accepted |
| [0082](0082-the-first-party-framework.md) | MWL ships a first-party framework split by ADR 0051's existing six tests — privileged halves in `Core`, the opinionated layer as the `mwl/web` package — with no ORM, no runtime container and the language itself as the view layer | Accepted |
| [0083](0083-persistent-connections-are-isolates.md) | A WebSocket or SSE connection is its own root isolate opened by naming a file the way `spawn script` does, code inside it is an ordinary loop, and fan-out is a bounded `Core\Topic` that closes a slow subscriber rather than blocking the publisher | Accepted |
| [0084](0084-durable-background-jobs.md) | A job is a row in a `Core\Db` table so an enqueue commits with the write that caused it, it runs as an isolate named by file, delivery is at-least-once with bounded retries, and a fleet claims safely with no protocol of ours | Accepted |
| [0085](0085-openapi-is-generated-from-the-route-table.md) | An OpenAPI 3.1 document is generated while compiling from the route table and derived codecs, an `#[Api]` attribute that contradicts the code is a compile error, and `mwl api diff` fails a build on a breaking change | Accepted |
| [0086](0086-core-cli-terminal-is-a-sink.md) | Terminal output is a sink that substitutes control bytes with visible glyphs, styling is the `Cli\Text` value type rather than a fifth grammar, prompts are `Core` members because raw mode is unreachable from userland, in-place output is a scoped live region, and `#[Command]` builds the argument table while compiling | Accepted |
| [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md) | A directional control that opens a scope and never closes it is a hard compile error in source and is substituted at both output sinks, by one predicate with three callers; balanced controls, invisibles and homoglyphs are each left alone with their reason | Accepted |
| [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) | A parameter is a sink when its content becomes an instruction rather than data, an unclassified `string`/`bytes` parameter on a `Core` member refuses `tainted`, `echo` binds to the terminal sink in every context but an HTTP request, and each response-body shape gets its own typed member | Accepted |
| [0089](0089-convert-is-one-rule-table-with-two-modes.md) | `mwl convert` is one deterministic rule table read through two modes — a default that emits only rewrites proven identical against the PHP oracle and comments out the rest with the idiomatic shape, and a `--mode=runnable` that also emits every rewrite with a mechanical destination under a `TODO` naming how it may differ | Accepted |
| [0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) | `==` is the only equality operator and `===` does not parse; it never converts, refuses two statically disjoint types, takes the strict reading for strings, arrays and objects, and answers `false` rather than throwing when a `mixed` operand's runtime type does not match | Accepted |

Retired numbers, folded into the ADR that now states the rule: **0032** → [0029](0029-identifier-casing-is-checked.md) § 1.

## Decisions taken at project start

Recorded here rather than as individual ADRs. Promote one to its own file if it is ever seriously
challenged.

**Rust as the implementation language.** Memory safety in the runtime is a product requirement, not an
implementation preference; Cranelift, the async ecosystem and the pure-Rust protocol crates all live here.

**Cranelift JIT as the only execution tier, no interpreter.** Chosen for peak performance and a single
semantics implementation to keep correct. The cost is that the first runnable program requires the whole
front end plus a working backend. Mitigated by shipping a *baseline* tier where every operation lowers to a
call into a Rust runtime helper — mechanically close to an interpreter loop, therefore quick to get
correct — with typed inlining layered on later behind the same IR boundary.

**Thread-per-core, shared-nothing runtime.** One single-threaded executor pinned per core; a request is
assigned to a core and never migrates. This is what makes value refcounts *non-atomic* (a heap is only ever
touched by one thread), makes cross-request state contamination structurally impossible rather than merely
prevented, and still uses every core — parallelism comes from N independent executors. Compiled code is
immutable and therefore shared across all cores through `Arc` with no copying.

**Stackful coroutines for suspension.** Validated by spike #3, now the guard tests
`a_helper_can_suspend_with_jit_frames_live_above_it` and `a_coroutine_round_trip_stays_cheap` in
[`benches/abi-probe`](../../benches/abi-probe/). The decisive property is the absence of *function
colouring*: any MWL function may perform I/O and yield without being marked `async`, so converted PHP call
chains become concurrent with no rewriting. The cost is a stack per in-flight task (default 64 KiB,
configurable, grown lazily) and a small audited unsafe core for stack switching, taken as a dependency
(`corosensei`) rather than hand-rolled. The memory is paid deliberately, under
[0004](0004-memory-for-simplicity.md).

**Isolated workers for CPU parallelism.** Work dispatched to another core gets its own heap; values
crossing the boundary are deep-copied, or moved when the refcount is 1. Data races are impossible by
construction rather than by discipline, which is what lets the refcounts stay non-atomic. The same rules
govern the script-level boundary in [0006](0006-isolated-script-execution.md), deliberately: one set of
value-crossing rules, not two — and [0023](0023-clone-serialize-and-cross-boundary-copy.md) gives that one
rule its formal definition, shared with `serialize()`/`unserialize()`.

**Strict shared-nothing requests.** Only compiled code survives a request. The consequence — reconnecting
to the database every request — is accepted for v1; `mwl-host` reserves an unused `PersistentRegistry` seam
so pooling can be added later without redesign. The same isolation is reachable from inside the language:
`spawn script` runs another `.mwl` file as a child isolate of the request tree, and an inbound request is
simply the root isolate of its tree, so both paths are one implementation.

**Safepoints emitted from the first backend commit.** A poll at every loop back-edge and function entry is
the single mechanism behind CPU-time limits, client-disconnect cancellation, the cycle collector, the
profiler, debugger breakpoints and later deoptimisation. Retrofitting it would mean rewriting codegen, so
it is not deferrable.

**Server-level configuration, not per-project.** `mwl.toml` is root-owned, TOML
([0064](0064-configuration-file-format.md)), and per-app capability blocks live in the *root* config so an
application can never grant itself rights. What a script may change about its own configuration at runtime
is per-directive and is argued in [0005](0005-config-changeability.md).

**Pure-Rust dependencies by default.** A memory-safe runtime cannot contain arbitrary C. Deviations are
explicit, argued and few — currently only SQLite (`rusqlite`), where no credible pure-Rust implementation
exists. The admission test is [0051](0051-standard-library-tiers.md) § 4's.

**Domain logic is an existing first-class Rust crate; compiler passes and scheduler primitives are ours.**
The neighbouring question to the one above — not *what may we depend on*, but *what may we write ourselves*.
Anything with an external specification (a protocol, a parser, a wire format, a cipher, a codec, a cron
expression, a timezone database) is a dependency, and **if no first-class crate exists, the feature is not
built** — a second-rate implementation of somebody else's specification is a security surface we would then
own forever. Anything about *MWL's own* compiler or scheduler has no possible crate and is ours by nature.
It is the same split the project already lives with: Cranelift compiles, and the IR lowering into it is
ours; `serde_json` parses, and the derive that emits MWL IR from MWL types
([0071](0071-derived-codecs.md)) could not be a crate if we wanted it to be. When a feature is half of each,
say which half is which before writing either.

**Checked-return call sites go through one code path.** See [0002](0002-error-propagation.md): a missing
status check would silently swallow an exception, so no caller constructs a raw `call` instruction.

**`unsafe` is confined to named crates, each declaring its own policy.** The workspace sets
`unsafe_code = "forbid"`; crates that genuinely need it opt down to `deny` and allow individual blocks with
a stated reason. Currently `mwl-runtime`, `mwl-codegen`, `mwl-stdlib` and `benches/abi-probe`, which must
call JIT-compiled code to measure it. The probe is `publish = false` and is not a dependency of anything
shipped, so it does not widen the runtime's unsafe surface.

**`continue` inside a `switch` continues the enclosing loop.** PHP counts a `switch` as a looping structure
for `continue`, so a bare one there behaves as `break` — and PHP has warned since 7.3 that you probably
meant `continue 2`. MWL takes the meaning that warning points at: `switch` owns `break` and nothing else, so
`continue` always means the innermost enclosing loop. The alternative is a keyword that silently means one
thing inside a `switch` and another everywhere else, which the priority ordering's simplicity rule refuses
to buy for a compatibility PHP itself discourages. `mwl_ir::lower::Lowering::lower_switch` implements it and
`tests/differential/lang/a-switch-and-a-match-agree-with-php.mwlt` pins it against PHP's `continue 2`.

**Architecture assumptions are tested, not remembered.** Several decisions here rest on how Cranelift,
`corosensei` and Wasmtime behave rather than on our own code, and a dependency bump can invalidate them
silently. `benches/abi-probe/` checks them on every CI run, including the *premise* of
[0002](0002-error-propagation.md) — that native unwinding through JIT frames is unavailable — so if that
ever changes we are told rather than left paying for a workaround that is no longer needed. It also guards a
premise about the *platform we are replacing*: that an OS process costs orders of magnitude more than a
task, which is the whole cost argument for [0006](0006-isolated-script-execution.md).
