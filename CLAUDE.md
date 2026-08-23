# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This is the only file you always need to read. It carries the rules that are easy to get wrong, plus a
table telling you the *one* other file to open for whatever you are doing.

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named below is authoritative and the other is a bug — fix it rather than reconciling it in your head.

## Where to look

Read this file, then **one** row below. Do not read the docs tree breadth-first: it is several hundred KB
and only grows as ADRs are added, and most of it is reasoning you only need when you are about to overturn
a decision.

**First, run `python .claude/brief.py`.** One call, well under 20 KB: the plan's status block, a one-line
map of every milestone plus the lead of the current one, the one-line title and status of every ADR, what
the guard tests actually hold with their thresholds, and what exists on disk. It stores no facts — it
slices the live files and names each source, so it cannot go stale, and it says so loudly if a slice comes
back empty. It deliberately does not print each ADR's full rule or a milestone's full text, so it stays
small as the repository grows — open the row below, or the plan at the line number it prints, for those.

Its size is held by one structural rule, not by trimming: **it may only print text whose length is bounded
by a count of entities, never by a length of prose.** Every section is one bounded line per milestone, per
ADR, per guard test, per named status field, and every section budget is *derived* from that count. So
adding an ADR, a milestone, or a paragraph of prose can never put it over budget. The one way to overrun is
writing a single over-long entity, and `python .claude/brief.py --check` reports exactly that — the file,
the line, and how many bytes to cut. **Run it after editing any doc it slices**; CI's `docs-budget` job runs
the same command. A hit is a thirty-second local edit, never a signal that the doc set needs a trim pass.

| Doing this | Open this |
|---|---|
| Deciding what to build next; scoping a milestone; checking what exists | [docs/implementation-plan.md](docs/implementation-plan.md) — the status block at the top, then your milestone. This is the plan of record. |
| Exceptions, the call ABI, helper signatures, panic containment | [ADR 0002](docs/adr/0002-error-propagation.md). It holds the only normative copy of the calling convention, and it *supersedes* any unwinding language you find elsewhere. |
| Extensions, wasm, WIT, `.mwlx` | [ADR 0003](docs/adr/0003-extension-system.md) |
| Whether a stdlib feature belongs in `Core`, in the default binary, in an extension or nowhere; which PHP extension maps to what; whether a C dependency is acceptable | [ADR 0051](docs/adr/0051-standard-library-tiers.md). Holds the only copy of the six ordered placement tests, the roster PHP's bundled extensions map onto, the rule that only Tier 0 may claim the `Core` prefix, and the two-question C-dependency test that replaces "argued individually". |
| `FFI`, `dl()`, native modules, stream wrappers, `php://`/`phar://`, `shmop`/`sysv*`/APCu, `eval`, `putenv`, `setlocale`, or "why can't userland do X at all" | [ADR 0052](docs/adr/0052-closed-doors.md). Holds the only copy of the four structural closures and the argument each rests on. |
| Whether a `.mwlx` can be an injection sink or source, `tainted`/`secret` at an extension call, what the manifest may declare | [ADR 0055](docs/adr/0055-extension-qualifier-declarations.md). Holds the only copy of the contagion-by-default rule, the two declarable deviations, the "an extension can never launder" rule, and the monotonicity property that makes the analysis independent of signature verification. |
| What a `Core` member looks like — argument order, options, failure signalling, naming, mutation, callbacks; whether a PHP built-in survives at all; why there is no procedural twin of a class API | [ADR 0063](docs/adr/0063-core-api-conventions.md). Holds the only copy of the twenty shape rules and the standing reasons a PHP built-in is dropped. The member list itself is [docs/spec/01-core-library.md](docs/spec/01-core-library.md), which is authoritative for every signature. |
| Weighing memory against safety, speed or simplicity | [ADR 0004](docs/adr/0004-memory-for-simplicity.md) |
| `mwl.ini`, `ini_set`, limits, capabilities | [ADR 0005](docs/adr/0005-config-changeability.md). Holds the only copy of the directive layout. |
| `spawn script`, isolates, the request boundary | [ADR 0006](docs/adr/0006-isolated-script-execution.md) |
| Caching between requests, APCu, `Core\Cache`, why a cached value is copied, what cache memory is charged to | [ADR 0059](docs/adr/0059-cross-request-state-is-explicit.md). Holds the only copy of the local/shared tier split, the copy-in/copy-out rule, and the O(cores × working set) cost statement. |
| `include`/`require`, loading another file into the current frame | [ADR 0021](docs/adr/0021-single-file-inclusion-construct.md). Holds the only copy of the rule that `require` is the one surviving spelling — `include`, `include_once` and `require_once` are all rejected with a diagnostic naming it. |
| Case sensitivity, whether `IF`/`TRUE` parse, whether `new httpclient()` resolves, or why a `require` that works on Windows must also work on Linux | [ADR 0062](docs/adr/0062-case-sensitivity-is-a-compiler-property.md). Holds the only copy of the three-axis rule — names resolve case-sensitively, reserved spellings are lower case and nothing else, and a `require`/`autoload` path is compared to the on-disk entry exactly — plus why a mis-cased keyword deliberately gets no diagnostic of its own. |
| Autoloading, `spl_autoload_register`, PSR-4, Composer's `vendor/autoload.php`, "do I have to `require` every file", or enumerating plugin/module classes nothing references by name | [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md). Holds the only copy of the `autoload` declaration's two forms and their relative-to-the-declaring-file rule, the one-declaration-per-autoloaded-file rule, `Core\Program::implementing<T>()`, and what each adds to the two caches' invalidation sets. |
| Uninitialized properties, `undefined`, why a typed property can't silently be `null`/zero/`undefined` | [ADR 0022](docs/adr/0022-definite-property-initialization.md). Holds the only copy of the rule that every constructor must definitely assign every property it declares, with no new type and no silent per-type default; the one residual runtime throw is scoped to `Core\Reflect`-bypassed construction. |
| `clone`, `serialize`/`unserialize`, `__clone`, `__serialize`/`__unserialize`/`__sleep`/`__wakeup`, or how a value crosses the `spawn`/`spawn worker`/`spawn script` boundary | [ADR 0023](docs/adr/0023-clone-serialize-and-cross-boundary-copy.md). Holds the only copy of the rule that `clone` stays PHP's shallow, same-heap, single-level copy while `serialize()`/`unserialize()` share one recursive graph-copy operation with the isolate boundary; none of the four magic hooks exist, and `unserialize()` accepts only MWL's own closed format. |
| The built-in HTTP server, live cache invalidation, picking up an edited `.mwl` file without a restart | [ADR 0017](docs/adr/0017-hot-reload-without-restart.md). Holds the only copy of the path-pointer-swap mechanism, why it needs no filesystem watcher, and why a request-serving core is never blocked on a recompile. |
| The on-disk compiled-artifact cache — its file layout, header format, why a bad/tampered/wrong-target file is a cache miss rather than a crash, or its eviction policy | [ADR 0042](docs/adr/0042-on-disk-artifact-cache-format.md). Holds the only copy of the content-addressed file-per-unit layout, the mmap-verify-then-mprotect read path, the atomic-rename write path, why a payload checksum defends against corruption but never against a hostile cache directory, and the probabilistic eviction sweep. |
| A portable single-file executable, `mwl build --compile`, bundling a CLI app's source into one runnable file, "rebundling" | [ADR 0048](docs/adr/0048-portable-single-file-executables.md). Holds the only copy of the append-to-host-binary payload format, why it ships source rather than precompiled artifacts, the reused ADR 0025 static-`require`-graph rule, and why bundling a web-serving deployment is explicitly out of scope. |
| Types, `uint`, `array<T>`, unions, `mixed`, conversions, array keys | [ADR 0007](docs/adr/0007-explicit-type-system.md). Holds the only copy of the type grammar, the conversion table, the arithmetic result types and the list of deliberate divergences from PHP. |
| `decimal`, money, `bcmath`, `gmp`, big integers, why floats aren't used for currency | [ADR 0054](docs/adr/0054-decimal-scalar-type.md). Holds the only copy of `decimal`'s layout, its literal form and `m` suffix, its rows in ADR 0007's conversion and arithmetic tables, its division policy, and what replaces `bcmath`/`gmp`. |
| `foreach` over an object, `Iterator`/`IteratorAggregate`/`ArrayAccess`/`Countable`, generators, `yield`, lazy streaming | [ADR 0053](docs/adr/0053-iteration-and-generators.md). Holds the only copy of the two surviving interfaces, the rejection of `ArrayAccess`/`Countable`, and the rule that generators lower to a state machine rather than to a coroutine. |
| `var`, local type inference, why `$x = "foo";` doesn't need its type spelled out | [ADR 0037](docs/adr/0037-var-local-type-inference.md). Holds the only copy of the rule that `var $name = expr;` infers a local's type from its initializer and fixes it forever, exactly as if written by hand — and the one initializer shape it refuses, a bare array literal. |
| `lateinit`, deferring a property's first assignment past the constructor, DI/setter injection, "why can't this non-nullable property just be set later" | [ADR 0038](docs/adr/0038-lateinit-property-modifier.md). Holds the only copy of the rule that `lateinit` is restricted to non-nullable class/interface-typed properties, throws on read-before-write by reusing ADR 0022's existing mechanism, and is refused on `?T`, a promoted parameter, or alongside `readonly`. |
| PHP's `(int)$x`/`(string)$x` legacy cast syntax, why it doesn't parse | [ADR 0034](docs/adr/0034-legacy-cast-syntax-rejected.md). Holds the only copy of the rule that `as` is the sole conversion spelling — the legacy cast keywords are diagnosed at parse time naming the equivalent `as` expression, with no alias kept. |
| Whether an `if`/`while`/`for`/`?:`/`&&`/`\|\|`/`!` condition needs an explicit `as bool`, PHP truthiness | [ADR 0035](docs/adr/0035-truthy-boolean-context.md). Holds the only copy of the truthy table and the exact six syntax positions it applies to; every other `bool` position (a parameter, property, `==`/`===`) is untouched and still needs `as bool`. |
| PHP's `and`/`or`/`xor` keyword operators, why they don't parse, "why does PHP have two ways to write AND/OR" | [ADR 0045](docs/adr/0045-and-or-xor-keyword-operators-rejected.md). Holds the only copy of the rule that `&&`/`\|\|` are the sole logical connectives — `and`/`or` are diagnosed naming that replacement, `xor` is diagnosed with no one-token replacement at all. |
| `<?php` as an open tag, PHP's `die` keyword, "why does MWL keep only one exit keyword"/"only one open tag" | [ADR 0049](docs/adr/0049-single-open-tag-and-single-exit-keyword.md). Holds the only copy of the rule that `<?mwl` is the sole code-mode open tag and `exit` is the sole process-termination keyword — `<?php` and `die` are each diagnosed at parse time naming the survivor, since neither ever differed in behavior from the spelling kept. |
| `list($a, $b) = $pair;`, PHP's `list()` destructuring spelling, "why doesn't `list` parse" | [ADR 0050](docs/adr/0050-list-destructuring-spelling-rejected.md). Holds the only copy of the rule that `[...]` is the sole destructuring spelling — `list(...)` is parsed in full so the diagnostic can span it, then discarded as `StmtKind::Error`; the element grammar the two shared is unchanged. |
| Restricting a parameter/property to one of a fixed set of values (PhpStorm's `#[ExpectedValues]`), `"a"\|"b"\|"c"` literal types, a subset of a class's constants, or a subset of an enum's cases (`Mode::A\|Mode::B`) | [ADR 0047](docs/adr/0047-literal-and-enum-case-types.md). Holds the only copy of the rule that a `string`/`int` literal and a named enum case are each their own type, unioned to declare a closed set; a class constant folds to its own literal type in that position, but an enum case never folds to its backing value — and why the wildcard/glob spelling (`Foo::TYPE_*`) was rejected outright rather than deferred. |
| `#[Attribute]`-style metadata, PHP doc-comment-as-config, annotations, `Core\Attributes`, why there's no attribute base class to declare | [ADR 0046](docs/adr/0046-attributes-shape-literal-metadata.md). Holds the only copy of the `#[Name(...)]`/`#[{...}]` shape-literal attribute syntax, the compile-time-constant-only payload rule, and the `Core\Attributes::get<T>`/`::all<T>` structural retrieval API — deliberately not part of `Core\Reflect`. |
| Regex, `preg_*`, `Core\Regex`, ReDoS, backreferences, lookaround, why a pattern can be refused at compile time | [ADR 0056](docs/adr/0056-regex-engine-policy.md). Holds the only copy of the two-tier engine rule, the throwing step budget, the compile-time tiering, and the rule that the *pattern* is a sink while the subject is not. |
| Why a literal regex/URI/format string is checked by `mwl check`, compile-time preparation, the intrinsic `Core` list | [ADR 0057](docs/adr/0057-intrinsic-literal-folding.md). Holds the only copy of the closed intrinsic list, the validate-always/prepare-where-possible split, and the rule that preparation can never change behaviour. |
| `string` vs `bytes`, the UTF-8 guarantee, text/binary conversion | [ADR 0009](docs/adr/0009-string-and-bytes.md) — **Proposed**, not yet Accepted: the default length/indexing granularity awaits a cost measurement (see its *Revisiting*). Holds the only copy of the `string`/`bytes` split and the conversion rule between them. |
| `enum`, enum cases, backing type, anything enum-shaped | [ADR 0010](docs/adr/0010-enums-are-a-value-type.md). Holds the only copy of enum semantics — a closed, named integer type like C#'s, not PHP's class-like construct; PHP's enum design is deliberately disregarded in full. |
| `static`, `global`, scoping, closure capture, where state may live at all | [ADR 0008](docs/adr/0008-static-and-global.md). Holds the only copy of the list of storage classes, and the one place `static`'s five PHP meanings are sorted into kept and rejected. |
| Free functions, global constants, the `Core` namespace, where a built-in lives | [ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md). Holds the only copy of the rule that every callable and every constant is a class member, and the `Core` domain-class shape built-ins are organised into. |
| `callable`, first-class callable syntax (`Foo::bar(...)`), taking a reference to a method/function, `__invoke` | [ADR 0027](docs/adr/0027-callable-is-closures-only.md). Holds the only copy of the rule that `callable` accepts exactly one kind of value and nothing else — PHP's string/array callable spellings are rejected, since first-class callable syntax already gives a statically resolvable reference, and `__invoke`/calling an object with `()` does not exist at all. |
| Anonymous functions/closures, `fn`, arrow functions, closure capture, `use (...)`, `Closure` vs `callable`, recursive closures | [ADR 0031](docs/adr/0031-callable-is-the-only-closure-type.md). Holds the only copy of the rule that `fn` (with or without a body) is the sole closure literal, closures have no `use` clause at all, and `callable` is the sole surviving type name — `Closure` does not exist as a separate spelling. |
| `__toString`/`Stringable`, `__destruct`/destructors, `__isset`/`__unset`, `unset()` on an object property, `__debugInfo`, `var_dump`/`print_r` customization, `__set_state`, `__autoload`, or "what happened to PHP magic method X" generally | [ADR 0028](docs/adr/0028-closing-the-remaining-magic-methods.md). Holds the only copy of the disposition of every remaining PHP magic method — a table indexing the ones already closed elsewhere (0014, 0023, 0027) plus the full reasoning for the ones it closes itself: `Stringable` replaces `__toString`, MWL has no destructors of any kind, `unset()` on a declared object property is always a diagnostic, and `__debugInfo`/`__set_state` are rejected with no replacement. |
| `stdClass`, an anonymous object literal `{a: 1, b: 2}`, the `object` type, or an inline `{name: T, ...}` shape type | [ADR 0036](docs/adr/0036-anonymous-object-shapes.md). Holds the only copy of the rule that `object` is the opaque top of every class type, an anonymous literal is a methodless compiler-synthesized instance, and a shape type is MWL's one deliberate, tightly scoped exception to fully nominal typing. |
| Naming conventions, `PascalCase`/`camelCase`/`SCREAMING_SNAKE_CASE`, identifier casing | [ADR 0029](docs/adr/0029-identifier-casing-is-checked.md). Holds the only copy of the per-category casing table and why the check is a hard compiler error with no suppression mechanism — not a lint. |
| Leading underscores in identifiers, whether `__construct`/`constructor` is a casing exception | [ADR 0030](docs/adr/0030-no-leading-underscores-constructor-spelling.md). Holds the only copy of the rule that no identifier — including properties, parameters and locals — may ever start with `_`, and that MWL's constructor is spelled `constructor`, not PHP's `__construct`, which is why ADR 0029 needs no exception for it at all. |
| Acronym spelling in names, whether `HTTPClient`/`parseXMLPayload`-style all-caps acronyms are allowed | [ADR 0032](docs/adr/0032-acronym-casing-rule-revoked.md). Holds the only copy of the rule that an identifier's casing check inspects only its first character — ADR 0029 § 1's "acronyms are one word, never kept all-caps" rule is revoked, so an all-caps acronym anywhere in a name is accepted. |
| `$_SERVER`, `$_GET`/`$_POST`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$_REQUEST`, `$argv`, or anything else PHP populates ambiently | [ADR 0012](docs/adr/0012-no-superglobals.md). Holds the only copy of the rule that no variable is ever host-populated — each becomes a `Core\Server`/`Core\Request`/`Core\Session`/`Core\Env`/`Core\Cli`/`Core\Script` call, and `$GLOBALS`/`$_REQUEST` have no replacement at all. |
| Comparing two objects with `<`/`>`/`<=`/`>=`/`<=>`, operator overloading, `Comparable`, `compareTo` | [ADR 0013](docs/adr/0013-comparable-interface.md). Holds the only copy of the rule that ordering two objects requires implementing `Comparable`; PHP's ambient property-walk fallback is rejected outright, and there is no cross-class overload. |
| Property hooks, `__get`/`__set`, `PropertyObserver`, undefined properties, `__call`/`__callStatic` | [ADR 0014](docs/adr/0014-property-observer.md). Holds the only copy of the rule that a property access runs its own hook first and a declared `PropertyObserver` second; accessing an undeclared property is always a hard error, and `__call`/`__callStatic` are not implemented at all. |
| `class_alias`, `use … as …`, or a `type` alias | [ADR 0015](docs/adr/0015-no-name-aliasing.md). Holds the only copy of the rule that nothing gets a second runtime-reachable name — `class_alias` does not exist and import renaming is rejected — while a `type` alias is kept as a distinct, compile-time-only synonym for a type expression, never for a single bare class. |
| `trait`, horizontal code reuse, mixins, `insteadof`, why a shared-behavior interface has method bodies, or how a PHP trait migrates | [ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md). Holds the only copy of the rule that `trait` does not exist at all — an `interface` method may carry a `public` (default, inherited/overridable) or `private` (internal-helper) body for shared behavior, and `implements Interface by $field;` delegates shared state to an ordinary property — with one conflict rule (an explicit override, always; there is no `insteadof`) covering both, and the full `mwl convert` migration path for every PHP trait shape. |
| Code coverage, call tracing, the deterministic per-call profiler, `Core\Debug`, or the `[debug]` `mwl.ini` section | [ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md). Holds the only copy of the probe mechanism and why it is safepoint-shaped rather than a second compiled tier; step debugging (`mwl dap`) and the sampling flamegraph profiler stay in [ADR 0016](docs/adr/0016-ide-integration.md) instead. |
| A scrollable timeline/flame-chart view combining calls with GC pauses and isolate-spawn boundaries, the `kind` field on a trace event, or exporting a trace/profile to speedscope | [ADR 0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md). Holds the only copy of the `call`/`gc`/`spawn` event taxonomy, why the GC/spawn instrumentation costs nothing on the per-statement/per-call hot path ADR 0018 already measures, and the speedscope-evented export alongside ADR 0018's Clover/lcov/Callgrind/NDJSON output. |
| `Core\Reflect`, `ReflectionClass`-equivalents, `Core\Ast`, or any runtime introspection/parsing surface | [ADR 0019](docs/adr/0019-reflection-and-ast-parsing-are-core-features.md). Holds the only copy of the rule that both are built-in `Core` features, not extensions — reflective access enforces the same visibility/hook checks as ordinary code (no `setAccessible(true)`), and a parsed AST is inert typed data with no path back into execution. |
| Uncaught exceptions, memory/CPU-limit fatals, internal panics, compile-error reporting, `Core\Fatal`, `Core\Log`, or anything about what gets logged when a handler itself fails | [ADR 0020](docs/adr/0020-error-escalation-ladder.md). Holds the only copy of the four-tier escalation ladder, why a resource-limit report is not a `Throwable`, and why every tier is zero-retry. |
| XSS, SQL injection, command/header/path injection, taint tracking, `tainted string`/`bytes`, `Core\Html\Markup`, or why a `Core\Request` value can't reach a sink unescaped | [ADR 0024](docs/adr/0024-taint-tracking-for-injection-sinks.md). Holds the only copy of the `tainted` qualifier, how it propagates and is laundered, the sinks that refuse it, and the HTML auto-escape default. |
| SSRF, fetching a user-supplied URL, `Core\Http\Client`, the `net.connect` address policy, DNS rebinding, redirects | [ADR 0058](docs/adr/0058-outbound-request-policy.md). Holds the only copy of the two-layer rule — a `tainted` URL laundered by `Core\Http::allowUrl` into an address-pinned `Target`, and a capability-level address policy re-checked per redirect hop. |
| `exec`/`system`/`passthru`/`shell_exec`/backticks/`proc_open`, running another program, shell command injection, or `Core\Process` | [ADR 0044](docs/adr/0044-core-process-argv-only-no-shell.md). Holds the only copy of the `Core\Process::run()`/`::spawn()` API, why there is no shell-string form at all, the Windows batch/PowerShell-target refusal, and the `process.exec` capability. |
| Passwords, API keys, credentials, `secret string`/`bytes`, why a value can't be echoed/logged/dumped, or how `secret` differs from and composes with `tainted` | [ADR 0033](docs/adr/0033-secret-qualifier-for-confidential-values.md). Holds the only copy of the `secret` qualifier, why it has no ambient source the way `tainted` does, the sinks that refuse it (HTML output, `Core\Log`, debug dumps, `Throwable` messages, serialize/isolate-crossing), and `Core\Secret::reveal()`. |
| JWT, CSRF tokens, TOTP, signed/encrypted cookies, or whether OAuth/WebAuthn/SAML belong in `Core` | [ADR 0060](docs/adr/0060-application-security-protocols.md). Holds the only copy of the closed four-entry roster, the three-part admission test, the stateless-token-vs-flow boundary that keeps it closed, and the correct-by-construction constraints (the algorithm comes from the key, never the token). |
| A wasm32 browser target, running MWL client-side in a tab, `Core\Browser`, or why `spawn`/coroutine suspension/`.mwlx` extensions don't reach that target | [ADR 0025](docs/adr/0025-wasm-browser-target.md). Holds the only copy of the per-target capability matrix and why the language itself doesn't grow a browser-specific dialect. |
| The VS Code extension, the PhpStorm plugin, `mwl-lsp`/`mwl-fmt` client wiring, syntax highlighting, or what "IDE integration" does and doesn't cover yet | [ADR 0016](docs/adr/0016-ide-integration.md). Holds the only copy of the rule that language smarts and formatting live exactly once, in `mwl-lsp`/`mwl-fmt`, with a thin client per editor — PhpStorm's LSP-bridge-before-native phasing and the deferred debugger-UI wiring are both decided there, not left to be inferred from M10's task list. |
| `mwl fmt`'s formatting rules — indentation, brace placement, quoting, trailing commas, modifier/import order, or why it never reflows a wrapped expression | [ADR 0039](docs/adr/0039-canonical-code-formatting.md). Holds the only copy of the PER-based style, the no-reflow (gofmt, not Prettier) model, and the rule that formatting is unconfigurable and never enforced by the compiler — `mwl fmt --check` is opt-in, not a diagnostic. |
| The VS Code extension's deep feature catalog (inspections, refactorings, Test Explorer/coverage, AST panel, profiler view, debugger UI wiring), why a minimal `mwl-lsp` ships in milestone M4B instead of M10, or `mwl-syntax`'s resilient/error-recovering parse mode | [ADR 0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md). Holds the only copy of which VS Code feature is staged behind which milestone/ADR, and the rule that `mwl-syntax` keeps its strict whole-file parse for `mwl check`/`mwl run` while exposing a second, lossless, error-tolerant entry point used only by editor tooling. PhpStorm is untouched — it stays exactly at [ADR 0016](docs/adr/0016-ide-integration.md)'s scope. |
| A decision with no ADR — thread-per-core, value layout, safepoints, the unit cache, shared-nothing requests | [docs/adr/README.md](docs/adr/README.md) § *Decisions taken at project start* for **why**; the plan's § *Architecture* for the **mechanics**. That split is deliberate. |
| Any measured number, or checking whether an architecture assumption still holds | the guard tests in [benches/abi-probe/](benches/abi-probe/). The tests are the source of truth; docs quote them and can lag. |
| Cross-machine/OS performance history, callgrind instruction counts, the perf dashboard, or why CI regression guards use wall-clock ratios instead of that history | [ADR 0026](docs/adr/0026-performance-measurement-methodology.md). Holds the only copy of the split between per-PR wall-clock regression guards (unchanged) and the merge-to-`main` callgrind-based historical dashboard, and why each metric was chosen. |
| Concrete *spelling* an ADR left open — file modes and `<?=`, the declaration-slot grammar (typed locals, `foreach` bindings, destructuring), `as` as the final conversion operator, why there is no `bytes` literal | [docs/spec/00-overview.md](docs/spec/00-overview.md). The ADR owns the semantics, this file owns the syntax; where they disagree the ADR wins. |
| What the language should *do* beyond those two files | unwritten. `docs/spec/` holds `00-overview.md` and `01-core-library.md` and nothing else — say so rather than inferring semantics. |

Each ADR opens with a metadata block and reaches `## Decision` within ~60 lines. Read those two. The
`## Context`, `## Investigation` and `## Alternatives rejected` sections are for when you intend to
*change* the decision — skip them otherwise.

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
  more observability than the operator's `mwl.ini` ceiling allows ([ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)).
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
  resolve relative to the file that declares it, so no manifest, walk-up search or `mwl.ini` directive is
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

Use a shell for what it is for — `cargo`, `git`, `python .claude/brief.py`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

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


## Keep work small, commit your work
- Always commit your work, when a step is done, you dont need to verify the history before, just commit everything that has changed
- Once you are done show me the next prompt i need to use for a new session to start right off where we left. Also store this next prompt into a seperate .md file, so i can use it whenever i want. Also update this file with the latest prompt, so we do not keep any content that already was in that file.
- The docs accumulate rationale bloat as ADRs are added. Periodically (the user does this manually, you never automatically) re-run the pass
  captured in [DOC_CLEANUP_PROMPT.md](DOC_CLEANUP_PROMPT.md) rather than re-deciding its rules from scratch.
- Everytime we decide to add new features, change feature or remove features, decide and ask what the tradeoffs are in performance, memory, usability and simplicity for developers using the langauge. If there are huge tradeoffs, notify the user and ask for agreement before proceeding. If there are only benefits, just go ahead.
- [docs/implementation-plan.md](docs/implementation-plan.md)'s leading status block has a **fixed field
  set** — `Status`, `Done`, `On disk`, `Toolchain`, `ADR slices landed`, `Open now`, `Blocking` — each
  capped at 400 bytes and enforced by `python .claude/brief.py --check`. Overwrite a field in place each
  session; never append a paragraph, and never add a field name (the check rejects an unrecognised one,
  which is what stops the block drifting back into free prose that grows a paragraph per milestone).
  Session-by-session history already lives in `git log`; per-file known-gap detail belongs in that crate's
  own module doc comment, not in the plan. A milestone's own section is free to be as long as it needs —
  the digest prints only its heading and opening paragraph — but still name the ADR it draws a rule from
  and stop, rather than re-deriving the rule inline; that's what
  [docs/adr/README.md](docs/adr/README.md) already owns.