# Architecture Decision Records

Each ADR records a decision that would be expensive to reverse, and *why* — so that a future reader can
tell a deliberate trade-off from an accident. Add one whenever a choice constrains later work.

Files get their own document only when the reasoning is subtle or contested. Decisions that are simply
recorded, with no live tension worth arguing, live in *Decisions taken at project start* below.

**Reading one.** Every ADR opens with a metadata block and an **In short** paragraph carrying the whole
decision. If you only need the rule, stop there. `Context`, `Investigation` and `Alternatives rejected`
exist for when you intend to *change* the decision. Numbering starts at 0002; there is no 0001, and the
project-start section below is what would have been it.

**Measured numbers.** Each ADR quotes only its own measurements, and every one is guarded by a test in
[`benches/abi-probe`](../../benches/abi-probe/) named in that ADR's *Validated by* line. The tests are
authoritative; a number written anywhere else is a copy that can go stale.

## Where to look

One row per topic, naming the **one** file that owns it. Read that row, open that file, stop —
the docs tree is several hundred KB and most of it is reasoning you only need when you intend to
overturn a decision. `python tools/brief.py --where <keyword>` prints just the rows that match, so
you never have to open this file to route a topic.

Every ADR opens with a metadata block and reaches `## Decision` within ~60 lines. Read those two.
`## Context`, `## Investigation` and `## Alternatives rejected` are for when you intend to *change*
the decision — skip them otherwise.

| Doing this | Open this |
|---|---|
| Deciding what to build next; scoping a milestone; checking what exists | [docs/implementation-plan.md](../implementation-plan.md) — the status block at the top, then your milestone. This is the plan of record. |
| Exceptions, the call ABI, helper signatures, panic containment | [ADR 0002](0002-error-propagation.md). It holds the only normative copy of the calling convention, and it *supersedes* any unwinding language you find elsewhere. |
| Extensions, wasm, WIT, `.mwlx` | [ADR 0003](0003-extension-system.md) |
| Whether a stdlib feature belongs in `Core`, in the default binary, in an extension or nowhere; which PHP extension maps to what; whether a C dependency is acceptable | [ADR 0051](0051-standard-library-tiers.md). Holds the only copy of the six ordered placement tests, the roster PHP's bundled extensions map onto, the rule that only Tier 0 may claim the `Core` prefix, and the two-question C-dependency test that replaces "argued individually". |
| `FFI`, `dl()`, native modules, stream wrappers, `php://`/`phar://`, `shmop`/`sysv*`/APCu, `eval`, `putenv`, `setlocale`, or "why can't userland do X at all" | [ADR 0052](0052-closed-doors.md). Holds the only copy of the four structural closures and the argument each rests on. |
| Whether a `.mwlx` can be an injection sink or source, `tainted`/`secret` at an extension call, what the manifest may declare | [ADR 0055](0055-extension-qualifier-declarations.md). Holds the only copy of the contagion-by-default rule, the two declarable deviations, the "an extension can never launder" rule, and the monotonicity property that makes the analysis independent of signature verification. |
| What a `Core` member looks like — argument order, options, failure signalling, naming, mutation, callbacks; whether a PHP built-in survives at all; why there is no procedural twin of a class API | [ADR 0063](0063-core-api-conventions.md). Holds the only copy of the twenty shape rules and the standing reasons a PHP built-in is dropped. The member list itself is [docs/spec/01-core-library.md](../spec/01-core-library.md), which is authoritative for every signature. |
| Weighing memory against safety, speed or simplicity | [ADR 0004](0004-memory-for-simplicity.md) |
| `mwl.toml`, `Core\Config::set`, limits, capabilities | [ADR 0005](0005-config-changeability.md). Holds the only copy of the directive layout. |
| The config file's *format* — why TOML and not INI/YAML/JSON, how a list/boolean/hash-pin is spelled, what `ini_set` is called now, or whether `mwl.toml` is a project manifest | [ADR 0064](0064-configuration-file-format.md). Holds the only copy of the format decision, the duplicate-and-unknown-key refusal, the `Core\Config` signatures, and why ADR 0061's rejected walked-up manifest is a different file. |
| `spawn script`, isolates, the request boundary | [ADR 0006](0006-isolated-script-execution.md) |
| Caching between requests, APCu, `Core\Cache`, why a cached value is copied, what cache memory is charged to | [ADR 0059](0059-cross-request-state-is-explicit.md). Holds the only copy of the local/shared tier split, the copy-in/copy-out rule, and the O(cores × working set) cost statement. |
| `include`/`require`, loading another file into the current frame | [ADR 0021](0021-single-file-inclusion-construct.md). Holds the only copy of the rule that `require` is the one surviving spelling — `include`, `include_once` and `require_once` are all rejected with a diagnostic naming it. |
| Case sensitivity, whether `IF`/`TRUE` parse, whether `new httpclient()` resolves, or why a `require` that works on Windows must also work on Linux | [ADR 0062](0062-case-sensitivity-is-a-compiler-property.md). Holds the only copy of the three-axis rule — names resolve case-sensitively, reserved spellings are lower case and nothing else, and a `require`/`autoload` path is compared to the on-disk entry exactly — plus why a mis-cased keyword deliberately gets no diagnostic of its own. |
| Autoloading, `spl_autoload_register`, PSR-4, Composer's `vendor/autoload.php`, "do I have to `require` every file", or enumerating plugin/module classes nothing references by name | [ADR 0061](0061-compile-time-autoload-and-program-discovery.md). Holds the only copy of the `autoload` declaration's two forms and their relative-to-the-declaring-file rule, the one-declaration-per-autoloaded-file rule, `Core\Program::implementing<T>()`, and what each adds to the two caches' invalidation sets. |
| Uninitialized properties, `undefined`, why a typed property can't silently be `null`/zero/`undefined` | [ADR 0022](0022-definite-property-initialization.md). Holds the only copy of the rule that every constructor must definitely assign every property it declares, with no new type and no silent per-type default; the one residual runtime throw is scoped to `Core\Reflect`-bypassed construction. |
| `clone`, `serialize`/`unserialize`, `__clone`, `__serialize`/`__unserialize`/`__sleep`/`__wakeup`, or how a value crosses the `spawn`/`spawn worker`/`spawn script` boundary | [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md). Holds the only copy of the rule that `clone` stays PHP's shallow, same-heap, single-level copy while `serialize()`/`unserialize()` share one recursive graph-copy operation with the isolate boundary; none of the four magic hooks exist, and `unserialize()` accepts only MWL's own closed format. |
| The built-in HTTP server, live cache invalidation, picking up an edited `.mwl` file without a restart | [ADR 0017](0017-hot-reload-without-restart.md). Holds the only copy of the path-pointer-swap mechanism, why it needs no filesystem watcher, and why a request-serving core is never blocked on a recompile. |
| The on-disk compiled-artifact cache — its file layout, header format, why a bad/tampered/wrong-target file is a cache miss rather than a crash, or its eviction policy | [ADR 0042](0042-on-disk-artifact-cache-format.md). Holds the only copy of the content-addressed file-per-unit layout, the mmap-verify-then-mprotect read path, the atomic-rename write path, why a payload checksum defends against corruption but never against a hostile cache directory, and the probabilistic eviction sweep. |
| A portable single-file executable, `mwl build --compile`, bundling a CLI app's source into one runnable file, "rebundling" | [ADR 0048](0048-portable-single-file-executables.md). Holds the only copy of the append-to-host-binary payload format, why it ships source rather than precompiled artifacts, the reused ADR 0025 static-`require`-graph rule, and why bundling a web-serving deployment is explicitly out of scope. |
| Types, `uint`, `array<T>`, unions, `mixed`, conversions, array keys | [ADR 0007](0007-explicit-type-system.md). Holds the only copy of the type grammar, the conversion table, the arithmetic result types and the list of deliberate divergences from PHP. |
| `decimal`, money, `bcmath`, `gmp`, big integers, why floats aren't used for currency | [ADR 0054](0054-decimal-scalar-type.md). Holds the only copy of `decimal`'s layout, its suffix-free target-typed literal form and the `as T` placing rule, its rows in ADR 0007's conversion and arithmetic tables, its division policy, and what replaces `bcmath`/`gmp`. |
| `foreach` over an object, `Iterator`/`IteratorAggregate`/`ArrayAccess`/`Countable`, generators, `yield`, lazy streaming | [ADR 0053](0053-iteration-and-generators.md). Holds the only copy of the two surviving interfaces, the rejection of `ArrayAccess`/`Countable`, and the rule that generators lower to a state machine rather than to a coroutine. |
| `var`, local type inference, why `$x = "foo";` doesn't need its type spelled out | [ADR 0037](0037-var-local-type-inference.md). Holds the only copy of the rule that `var $name = expr;` infers a local's type from its initializer and fixes it forever, exactly as if written by hand — and the one initializer shape it refuses, a bare array literal. |
| `lateinit`, deferring a property's first assignment past the constructor, DI/setter injection, "why can't this non-nullable property just be set later" | [ADR 0038](0038-lateinit-property-modifier.md). Holds the only copy of the rule that `lateinit` is restricted to non-nullable class/interface-typed properties, throws on read-before-write by reusing ADR 0022's existing mechanism, and is refused on `?T`, a promoted parameter, or alongside `readonly`. |
| PHP's `(int)$x`/`(string)$x` legacy cast syntax, why it doesn't parse | [ADR 0034](0034-legacy-cast-syntax-rejected.md). Holds the only copy of the rule that `as` is the sole conversion spelling — the legacy cast keywords are diagnosed at parse time naming the equivalent `as` expression, with no alias kept. |
| A conversion that shouldn't throw, `as ?int`, `tryParse`/`toIntOrNull`, validating untrusted input without `try`/`catch`, or why `Core\Validate` has no `isInteger` | [ADR 0066](0066-nullable-conversion-operator.md). Holds the only copy of the rule that `as ?T` yields `null` where `as T` throws, the null-in/null-out rule, the table of which conversions admit the form (and the three kinds that are a diagnostic instead), and why the three numeric `Validate` predicates were removed rather than kept. |
| Whether an `if`/`while`/`for`/`?:`/`&&`/`\|\|`/`!` condition needs an explicit `as bool`, PHP truthiness | [ADR 0035](0035-truthy-boolean-context.md). Holds the only copy of the truthy table and the exact six syntax positions it applies to; every other `bool` position (a parameter, property, `==`/`===`) is untouched and still needs `as bool`. |
| PHP's `and`/`or`/`xor` keyword operators, why they don't parse, "why does PHP have two ways to write AND/OR" | [ADR 0045](0045-and-or-xor-keyword-operators-rejected.md). Holds the only copy of the rule that `&&`/`\|\|` are the sole logical connectives — `and`/`or` are diagnosed naming that replacement, `xor` is diagnosed with no one-token replacement at all. |
| `<?php` as an open tag, PHP's `die` keyword, "why does MWL keep only one exit keyword"/"only one open tag" | [ADR 0049](0049-single-open-tag-and-single-exit-keyword.md). Holds the only copy of the rule that `<?mwl` is the sole code-mode open tag and `exit` is the sole process-termination keyword — `<?php` and `die` are each diagnosed at parse time naming the survivor, since neither ever differed in behavior from the spelling kept. |
| `list($a, $b) = $pair;`, PHP's `list()` destructuring spelling, "why doesn't `list` parse" | [ADR 0050](0050-list-destructuring-spelling-rejected.md). Holds the only copy of the rule that `[...]` is the sole destructuring spelling — `list(...)` is parsed in full so the diagnostic can span it, then discarded as `StmtKind::Error`; the element grammar the two shared is unchanged. |
| Restricting a parameter/property to one of a fixed set of values (PhpStorm's `#[ExpectedValues]`), `"a"\|"b"\|"c"` literal types, a subset of a class's constants, or a subset of an enum's cases (`Mode::A\|Mode::B`) | [ADR 0047](0047-literal-and-enum-case-types.md). Holds the only copy of the rule that a `string`/`int` literal and a named enum case are each their own type, unioned to declare a closed set; a class constant folds to its own literal type in that position, but an enum case never folds to its backing value — and why the wildcard/glob spelling (`Foo::TYPE_*`) was rejected outright rather than deferred. |
| `#[Attribute]`-style metadata, PHP doc-comment-as-config, annotations, `Core\Attributes`, why there's no attribute base class to declare | [ADR 0046](0046-attributes-shape-literal-metadata.md). Holds the only copy of the `#[Name(...)]`/`#[{...}]` shape-literal attribute syntax, the compile-time-constant-only payload rule, and the `Core\Attributes::get<T>`/`::all<T>` structural retrieval API — deliberately not part of `Core\Reflect`. |
| Regex, `preg_*`, `Core\Regex`, ReDoS, backreferences, lookaround, why a pattern can be refused at compile time | [ADR 0056](0056-regex-engine-policy.md). Holds the only copy of the two-tier engine rule, the throwing step budget, the compile-time tiering, and the rule that the *pattern* is a sink while the subject is not. |
| Why a literal regex/URI/format string is checked by `mwl check`, compile-time preparation, the intrinsic `Core` list | [ADR 0057](0057-intrinsic-literal-folding.md). Holds the only copy of the closed intrinsic list, the validate-always/prepare-where-possible split, and the rule that preparation can never change behaviour. |
| `string` vs `bytes`, the UTF-8 guarantee, text/binary conversion | [ADR 0009](0009-string-and-bytes.md) — **Proposed**, not yet Accepted: the default length/indexing granularity awaits a cost measurement (see its *Revisiting*). Holds the only copy of the `string`/`bytes` split and the conversion rule between them. |
| `enum`, enum cases, backing type, anything enum-shaped | [ADR 0010](0010-enums-are-a-value-type.md). Holds the only copy of enum semantics — a closed, named integer type like C#'s, not PHP's class-like construct; PHP's enum design is deliberately disregarded in full. |
| `static`, `global`, scoping, closure capture, where state may live at all | [ADR 0008](0008-static-and-global.md). Holds the only copy of the list of storage classes, and the one place `static`'s five PHP meanings are sorted into kept and rejected. |
| Free functions, global constants, the `Core` namespace, where a built-in lives | [ADR 0011](0011-functions-and-constants-are-class-members.md). Holds the only copy of the rule that every callable and every constant is a class member, and the `Core` domain-class shape built-ins are organised into. |
| `callable`, first-class callable syntax (`Foo::bar(...)`), taking a reference to a method/function, `__invoke` | [ADR 0027](0027-callable-is-closures-only.md). Holds the only copy of the rule that `callable` accepts exactly one kind of value and nothing else — PHP's string/array callable spellings are rejected, since first-class callable syntax already gives a statically resolvable reference, and `__invoke`/calling an object with `()` does not exist at all. |
| Anonymous functions/closures, `fn`, arrow functions, closure capture, `use (...)`, `Closure` vs `callable`, recursive closures | [ADR 0031](0031-callable-is-the-only-closure-type.md). Holds the only copy of the rule that `fn` (with or without a body) is the sole closure literal, closures have no `use` clause at all, and `callable` is the sole surviving type name — `Closure` does not exist as a separate spelling. |
| `__toString`/`Stringable`, `__destruct`/destructors, `__isset`/`__unset`, `unset()` on an object property, `__debugInfo`, `var_dump`/`print_r` customization, `__set_state`, `__autoload`, or "what happened to PHP magic method X" generally | [ADR 0028](0028-closing-the-remaining-magic-methods.md). Holds the only copy of the disposition of every remaining PHP magic method — a table indexing the ones already closed elsewhere (0014, 0023, 0027) plus the full reasoning for the ones it closes itself: `Stringable` replaces `__toString`, MWL has no destructors of any kind, `unset()` on a declared object property is always a diagnostic, and `__debugInfo`/`__set_state` are rejected with no replacement. |
| `stdClass`, an anonymous object literal `{a: 1, b: 2}`, the `object` type, or an inline `{name: T, ...}` shape type | [ADR 0036](0036-anonymous-object-shapes.md). Holds the only copy of the rule that `object` is the opaque top of every class type, an anonymous literal is a methodless compiler-synthesized instance, and a shape type is MWL's one deliberate, tightly scoped exception to fully nominal typing. |
| Naming conventions, `PascalCase`/`camelCase`/`SCREAMING_SNAKE_CASE`, identifier casing | [ADR 0029](0029-identifier-casing-is-checked.md). Holds the only copy of the per-category casing table and why the check is a hard compiler error with no suppression mechanism — not a lint. |
| Leading underscores in identifiers, whether `__construct`/`constructor` is a casing exception | [ADR 0030](0030-no-leading-underscores-constructor-spelling.md). Holds the only copy of the rule that no identifier — including properties, parameters and locals — may ever start with `_`, and that MWL's constructor is spelled `constructor`, not PHP's `__construct`, which is why ADR 0029 needs no exception for it at all. |
| Acronym spelling in names, whether `HTTPClient`/`parseXMLPayload`-style all-caps acronyms are allowed | [ADR 0032](0032-acronym-casing-rule-revoked.md). Holds the only copy of the rule that an identifier's casing check inspects only its first character — ADR 0029 § 1's "acronyms are one word, never kept all-caps" rule is revoked, so an all-caps acronym anywhere in a name is accepted. |
| `$_SERVER`, `$_GET`/`$_POST`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$_REQUEST`, `$argv`, or anything else PHP populates ambiently | [ADR 0012](0012-no-superglobals.md). Holds the only copy of the rule that no variable is ever host-populated — each becomes a `Core\Server`/`Core\Request`/`Core\Session`/`Core\Env`/`Core\Cli`/`Core\Script` call, and `$GLOBALS`/`$_REQUEST` have no replacement at all. |
| Comparing two objects with `<`/`>`/`<=`/`>=`/`<=>`, operator overloading, `Comparable`, `compareTo` | [ADR 0013](0013-comparable-interface.md). Holds the only copy of the rule that ordering two objects requires implementing `Comparable`; PHP's ambient property-walk fallback is rejected outright, and there is no cross-class overload. |
| Property hooks, `__get`/`__set`, `PropertyObserver`, undefined properties, `__call`/`__callStatic` | [ADR 0014](0014-property-observer.md). Holds the only copy of the rule that a property access runs its own hook first and a declared `PropertyObserver` second; accessing an undeclared property is always a hard error, and `__call`/`__callStatic` are not implemented at all. |
| `class_alias`, `use … as …`, or a `type` alias | [ADR 0015](0015-no-name-aliasing.md). Holds the only copy of the rule that nothing gets a second runtime-reachable name — `class_alias` does not exist and import renaming is rejected — while a `type` alias is kept as a distinct, compile-time-only synonym for a type expression, never for a single bare class. |
| `trait`, horizontal code reuse, mixins, `insteadof`, why a shared-behavior interface has method bodies, or how a PHP trait migrates | [ADR 0043](0043-interface-default-methods-and-delegation-replace-traits.md). Holds the only copy of the rule that `trait` does not exist at all — an `interface` method may carry a `public` (default, inherited/overridable) or `private` (internal-helper) body for shared behavior, and `implements Interface by $field;` delegates shared state to an ordinary property — with one conflict rule (an explicit override, always; there is no `insteadof`) covering both, and the full `mwl convert` migration path for every PHP trait shape. |
| Code coverage, call tracing, the deterministic per-call profiler, `Core\Debug`, or the `[debug]` `mwl.toml` section | [ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md). Holds the only copy of the probe mechanism and why it is safepoint-shaped rather than a second compiled tier; step debugging (`mwl dap`) and the sampling flamegraph profiler stay in [ADR 0016](0016-ide-integration.md) instead. |
| A scrollable timeline/flame-chart view combining calls with GC pauses and isolate-spawn boundaries, the `kind` field on a trace event, or exporting a trace/profile to speedscope | [ADR 0041](0041-timeline-export-and-gc-spawn-trace-events.md). Holds the only copy of the `call`/`gc`/`spawn` event taxonomy, why the GC/spawn instrumentation costs nothing on the per-statement/per-call hot path ADR 0018 already measures, and the speedscope-evented export alongside ADR 0018's Clover/lcov/Callgrind/NDJSON output. |
| `Core\Reflect`, `ReflectionClass`-equivalents, `Core\Ast`, or any runtime introspection/parsing surface | [ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md). Holds the only copy of the rule that both are built-in `Core` features, not extensions — reflective access enforces the same visibility/hook checks as ordinary code (no `setAccessible(true)`), and a parsed AST is inert typed data with no path back into execution. |
| Uncaught exceptions, memory/CPU-limit fatals, internal panics, compile-error reporting, `Core\Fatal`, `Core\Log`, or anything about what gets logged when a handler itself fails | [ADR 0020](0020-error-escalation-ladder.md). Holds the only copy of the four-tier escalation ladder, why a resource-limit report is not a `Throwable`, and why every tier is zero-retry. |
| XSS, SQL injection, command/header/path injection, taint tracking, `tainted string`/`bytes`, `Core\Html\Markup`, or why a `Core\Request` value can't reach a sink unescaped | [ADR 0024](0024-taint-tracking-for-injection-sinks.md). Holds the only copy of the `tainted` qualifier, how it propagates and is laundered, the sinks that refuse it, and the HTML auto-escape default. |
| SSRF, fetching a user-supplied URL, `Core\Http\Client`, the `net.connect` address policy, DNS rebinding, redirects | [ADR 0058](0058-outbound-request-policy.md). Holds the only copy of the two-layer rule — a `tainted` URL laundered by `Core\Http::allowUrl` into an address-pinned `Target`, and a capability-level address policy re-checked per redirect hop. |
| `exec`/`system`/`passthru`/`shell_exec`/backticks/`proc_open`, running another program, shell command injection, or `Core\Process` | [ADR 0044](0044-core-process-argv-only-no-shell.md). Holds the only copy of the `Core\Process::run()`/`::spawn()` API, why there is no shell-string form at all, the Windows batch/PowerShell-target refusal, and the `process.exec` capability. |
| Passwords, API keys, credentials, `secret string`/`bytes`, why a value can't be echoed/logged/dumped, or how `secret` differs from and composes with `tainted` | [ADR 0033](0033-secret-qualifier-for-confidential-values.md). Holds the only copy of the `secret` qualifier, why it has no ambient source the way `tainted` does, the sinks that refuse it (HTML output, `Core\Log`, debug dumps, `Throwable` messages, serialize/isolate-crossing), and `Core\Secret::reveal()`. |
| JWT, CSRF tokens, TOTP, signed/encrypted cookies, or whether OAuth/WebAuthn/SAML belong in `Core` | [ADR 0060](0060-application-security-protocols.md). Holds the only copy of the closed four-entry roster, the three-part admission test, the stateless-token-vs-flow boundary that keeps it closed, and the correct-by-construction constraints (the algorithm comes from the key, never the token). |
| A wasm32 browser target, running MWL client-side in a tab, `Core\Browser`, or why `spawn`/coroutine suspension/`.mwlx` extensions don't reach that target | [ADR 0025](0025-wasm-browser-target.md). Holds the only copy of the per-target capability matrix and why the language itself doesn't grow a browser-specific dialect. |
| The VS Code extension, the PhpStorm plugin, `mwl-lsp`/`mwl-fmt` client wiring, syntax highlighting, or what "IDE integration" does and doesn't cover yet | [ADR 0016](0016-ide-integration.md). Holds the only copy of the rule that language smarts and formatting live exactly once, in `mwl-lsp`/`mwl-fmt`, with a thin client per editor — PhpStorm's LSP-bridge-before-native phasing and the deferred debugger-UI wiring are both decided there, not left to be inferred from M10's task list. |
| `mwl fmt`'s formatting rules — indentation, brace placement, quoting, trailing commas, modifier/import order, or why it never reflows a wrapped expression | [ADR 0039](0039-canonical-code-formatting.md). Holds the only copy of the PER-based style, the no-reflow (gofmt, not Prettier) model, and the rule that formatting is unconfigurable and never enforced by the compiler — `mwl fmt --check` is opt-in, not a diagnostic. |
| The VS Code extension's deep feature catalog (inspections, refactorings, Test Explorer/coverage, AST panel, profiler view, debugger UI wiring), why a minimal `mwl-lsp` ships in milestone M4B instead of M10, or `mwl-syntax`'s resilient/error-recovering parse mode | [ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md). Holds the only copy of which VS Code feature is staged behind which milestone/ADR, and the rule that `mwl-syntax` keeps its strict whole-file parse for `mwl check`/`mwl run` while exposing a second, lossless, error-tolerant entry point used only by editor tooling. PhpStorm is untouched — it stays exactly at [ADR 0016](0016-ide-integration.md)'s scope. |
| Third-party licenses, attribution, `THIRD-PARTY-LICENSES.txt`, what `mwl info`/`mwl -i` prints, or whether a new dependency's license may ship | [ADR 0065](0065-third-party-attribution-and-mwl-info.md). Holds the only copy of the generate-commit-embed rule, the fail-closed properties of `tools/gen-attribution.py`, the dual-license preference order, and the `mwl info` surface. [deny.toml](../../deny.toml) stays authoritative for what may be *linked*; this decides what must be *shipped*. |
| A decision with no ADR — thread-per-core, value layout, safepoints, the unit cache, shared-nothing requests | [docs/adr/README.md](README.md) § *Decisions taken at project start* for **why**; the plan's § *Architecture* for the **mechanics**. That split is deliberate. |
| Any measured number, or checking whether an architecture assumption still holds | the guard tests in [benches/abi-probe/](../../benches/abi-probe/). The tests are the source of truth; docs quote them and can lag. |
| Cross-machine/OS performance history, callgrind instruction counts, the perf dashboard, or why CI regression guards use wall-clock ratios instead of that history | [ADR 0026](0026-performance-measurement-methodology.md). Holds the only copy of the split between per-PR wall-clock regression guards (unchanged) and the merge-to-`main` callgrind-based historical dashboard, and why each metric was chosen. |
| Concrete *spelling* an ADR left open — file modes and `<?=`, the declaration-slot grammar (typed locals, `foreach` bindings, destructuring), `as` as the final conversion operator, why there is no `bytes` literal | [docs/spec/00-overview.md](../spec/00-overview.md). The ADR owns the semantics, this file owns the syntax; where they disagree the ADR wins. |
| What the language should *do* beyond those two files | unwritten. `docs/spec/` holds `00-overview.md` and `01-core-library.md` and nothing else — say so rather than inferring semantics. |

**Adding a decision.** Touch exactly these, in order — nothing else should ever need its own copy:

1. Write the ADR file, following the shape every other one has: metadata block, **In short**, then
   `## Context` / `## Investigation` / `## Alternatives rejected` / `## Decision` / `## Consequences` as
   needed.
2. Add its row to the table below: number, one-line decision, status.
3. If it changes a prior ADR's decision, add **Amends** / **Amended by** lines linking the two, both
   directions.
4. If the topic is one an agent will search for by keyword, add one row to
   [AGENTS.md](../../AGENTS.md)'s "Where to look" table, and — only if it is a hard invariant — one bullet
   (a sentence, not a paragraph) to its "Ground rules enforced elsewhere".
5. If it changes what a milestone builds, update that milestone's paragraph in
   [the plan](../implementation-plan.md) with a link and a headline, not a restatement.

`tools/brief.py` needs no update for any of this: it slices this file's tables and the plan's status block
live, so a new row is picked up automatically. Two things to get right in a new row, both for the reader
rather than for a checker — nothing enforces either: keep the **Decision** cell to one sentence (~160 bytes
is the length that reads well in the digest), and write any `|` inside a cell as `\|`, or the row renders
broken on GitHub and `brief.py` says so.

| # | Decision | Status |
|---|---|---|
| [0002](0002-error-propagation.md) | Exceptions propagate by checked return, not by unwinding | Accepted |
| [0003](0003-extension-system.md) | Extensions are sandboxed WebAssembly components, not native shared libraries | Accepted |
| [0004](0004-memory-for-simplicity.md) | Memory is spent for security, speed and simplicity, in that order | Accepted |
| [0005](0005-config-changeability.md) | `mwl.toml` states defaults, not ceilings | Accepted |
| [0006](0006-isolated-script-execution.md) | Running another script is an in-process isolate, not a subprocess | Accepted |
| [0007](0007-explicit-type-system.md) | Types are declared, checked, and never change by themselves | Accepted |
| [0008](0008-static-and-global.md) | `static` marks a class member; there are no function statics and no `global` | Accepted |
| [0009](0009-string-and-bytes.md) | `string` is text; binary data is a distinct `bytes` type | Proposed |
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
| [0027](0027-callable-is-closures-only.md) | `callable` means a `Closure`; PHP's string/array callable spellings and `__invoke` are both rejected | Accepted |
| [0028](0028-closing-the-remaining-magic-methods.md) | `Stringable` replaces `__toString`; no `__destruct`, `__debugInfo`, or `__set_state`; `unset()` is refused on an object property | Accepted |
| [0029](0029-identifier-casing-is-checked.md) | Identifier casing is a hard compiler error — `PascalCase` types, `camelCase` members, `SCREAMING_SNAKE_CASE` constants, no suppression | Accepted |
| [0030](0030-no-leading-underscores-constructor-spelling.md) | No leading underscores anywhere; the constructor is spelled `constructor`, not `__construct` | Accepted |
| [0031](0031-callable-is-the-only-closure-type.md) | `fn` is the only closure literal, with no `use` clause; `callable` absorbs `Closure` as the one surviving type name | Accepted |
| [0032](0032-acronym-casing-rule-revoked.md) | The acronym-as-one-word casing rule is revoked; only an identifier's leading character is checked | Accepted |
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
[`benches/abi-probe`](../../benches/abi-probe/), which hold the current figure for a suspend/resume round
trip through live JIT frames. The decisive property is the absence of *function colouring*: any MWL
function may perform I/O and yield without being marked `async`, so converted PHP call chains become
concurrent with no rewriting. The cost is a stack per in-flight task (default 64 KiB, configurable, grown lazily) and a small
audited unsafe core for stack switching, taken as a dependency (`corosensei`) rather than hand-rolled. The
memory is paid deliberately, under [0004](0004-memory-for-simplicity.md).

**Isolated workers for CPU parallelism.** Work dispatched to another core gets its own heap; values
crossing the boundary are deep-copied, or moved when the refcount is 1. Data races are impossible by
construction rather than by discipline, which is what lets the refcounts stay non-atomic. The copy is
another instance of [0004](0004-memory-for-simplicity.md). The same rules govern the script-level boundary
in [0006](0006-isolated-script-execution.md), deliberately: one set of value-crossing rules, not two — and
[0023](0023-clone-serialize-and-cross-boundary-copy.md) gives that one rule its formal definition, shared
with `serialize()`/`unserialize()`.

**Strict shared-nothing requests.** Only compiled code survives a request. The consequence — reconnecting
to the database every request — is accepted for v1; `mwl-host` reserves an unused `PersistentRegistry` seam
so pooling can be added later without redesign. The same isolation is reachable from inside the language:
`spawn script` runs another `.mwl` file as a child isolate of the request tree
([0006](0006-isolated-script-execution.md)), and an inbound request is simply the root isolate of its tree,
so both paths are one implementation.

**Safepoints emitted from the first backend commit.** A poll at every loop back-edge and function entry is
the single mechanism behind CPU-time limits, client-disconnect cancellation, the cycle collector, the
profiler, debugger breakpoints and later deoptimisation. Retrofitting it would mean rewriting codegen, so
it is not deferrable.

**Server-level configuration, not per-project.** `mwl.toml` is root-owned, TOML
([0064](0064-configuration-file-format.md)), and per-app
capability blocks live in the *root* config so an application can never grant itself rights. What a script
may change about its own configuration at runtime is per-directive and is argued in
[0005](0005-config-changeability.md).

**Pure-Rust dependencies by default.** A memory-safe runtime cannot contain arbitrary C. Deviations are
explicit, argued and few — currently only SQLite (`rusqlite`), where no credible pure-Rust implementation
exists.

**Checked-return call sites go through one code path.** See [0002](0002-error-propagation.md): a missing
status check would silently swallow an exception, so no caller constructs a raw `call` instruction.

**`unsafe` is confined to named crates, each declaring its own policy.** The workspace sets
`unsafe_code = "forbid"`; crates that genuinely need it opt down to `deny` and allow individual blocks with
a stated reason. Currently that is `mwl-runtime` and `mwl-codegen` (planned: the coroutine stack switcher,
the request arena, JIT page mapping) plus `benches/abi-probe`, which must call JIT-compiled code to
measure it. The probe is `publish = false` and is not a dependency of anything shipped, so it does not
widen the runtime's unsafe surface.

**Architecture assumptions are tested, not remembered.** Several decisions here rest on how Cranelift,
`corosensei` and Wasmtime behave rather than on our own code, and a dependency bump can invalidate them
silently. `benches/abi-probe/` checks them on every CI run, including the *premise* of
[0002](0002-error-propagation.md) — that native unwinding through JIT frames is unavailable — so if that
ever changes we are told rather than left paying for a workaround that is no longer needed. It also guards a
premise about the *platform we are replacing*: that an OS process costs orders of magnitude more than a task,
which is the whole cost argument for [0006](0006-isolated-script-execution.md).
