# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-25. **M3 is done, and the second M4 + M4S loop now runs a catch-up stage
> before `Core` breadth.** Eleven ADRs (0080-0090) were accepted after the milestones that own their
> work were reported done; `docs/agent/loop-goal.md` § *Stage 0* is the ordered list, and
> `loop-goal.toml`'s `stage = "0 catch-up"` block runs before the program legs so an unfinished item
> is what the ledger names. **ADR 0087's lexer half is built** — `mwl_syntax::bidi` is the one
> predicate an unterminated directional control is rejected by, `E0008` at the lexer and, at M7/M8,
> a substitution at both sinks. **ADR 0090 §§ 1 and 2 are built**: `==`/`!=` are the only equality
> spellings, `===`/`!==` are `E0232` at the lexer (consumed whole, reported, then lexed as the
> two-character operator so a file still reports its other problems),
> `BinaryOp::Identical`/`NotIdentical` are gone from the AST, and two statically disjoint operands
> are `E0466` — at `==`/`!=`, and at the `switch` label and `match` arm § 6 points at the same rule.
> **What that ADR still owes** is § 3's string, array and object rows, one runtime helper each
> (M3/M4). **ADR 0069's `+`/`+=` refusal is built too**, `E0467` naming `Core\Arr::underlay`. **The
> goal behind the catch-up is unchanged: M4S Part I in full — spec §§ 1-12 — plus the M4 surface it
> cannot be written without.** Every representation that blocked a section is built and recorded in
> the crate that owns it: `mixed`/`?T`/every union is `mwl_ir::Ty::Tagged`, strict identity is
> `mwl_runtime::identity`, `decimal` is `mwl_runtime::decimal`, a `Core`-owned instance is an
> ordinary MWL object (`mwl_stdlib::instance`), and a variadic tail is one `array<T>` argument
> (`registry::CoreTy::Variadic`) — so **every signature shape the spec writes can now be stated**,
> and a section that is not built is only unwritten. **Every M4 control-flow statement lowers but
> `do`/`while`**, and ADR 0070's duration literal lexes, types and runs. Sections whole or nearly:
> **§ 3** (thirty-eight `Core\Math` rows plus eleven class constants), **§ 4** (over `jiff` and the
> CLDR pattern grammar in `mwl_stdlib::cldr`; `Date`/`TimeOfDay` are that module's gap 1), **§ 5**
> (both ADR 0056 tiers), and **§ 6** — `Core\Json::encode`/`decode`/`isValid` over `serde_json`,
> driven through a visitor so a document becomes `MwlArray`s directly; `decodeAs<T>` is
> `mwl_stdlib::json`'s gap 2. **A helper's failure names its own spec § 10 class**:
> `mwl_runtime::ThrownClass` is the closed roster, `Fault::thrown_as` is how a member picks one, and
> `catch (ParseError $e)` matches a bad JSON document or a bad `Core\Time::parse`. **A call site can
> write its own type argument** — `decodeAs<User>($b)` parses, and a member declares which variables
> are written rather than inferred. **ADR 0071's first compiler-recognized attribute is built**:
> `#[Json\Derive]` is matched nominally in `mwl_types::derive`, and the field list it reads reaches
> `Core\Json::encode` through `mwl_runtime::ClassDesc`, so a declared class encodes, and
> **`ParseError` carries spec § 10's `issues`** — the one property any class in the exception tree
> declares below the root, over ADR 0071 § 5's `Core\Issue` shape. Dependencies: `regex` +
> `fancy-regex` and `jiff` are named by the user; the rest the loop picks under ADR 0051 § 4.
>
> **Done:** M0 (setup); M1 (front end — lexer with dual mode, inline HTML, heredoc/nowdoc and
> interpolation, the full parser, and the M1-scoped grammar of ADRs
> 0024/0031/0033/0036/0037/0043/0046/0049/0050/0066; `crates/mwl-syntax/tests/corpus_parse.rs` parses the
> local `php-src` checkout and a 5-minute WSL `cargo fuzz run lex`/`parse` both find zero panics).
> **M1 was re-opened for four grammar additions**, each accepted after it was reported done and each
> blocking its ADR's already-scheduled M2 slice; **`decimal` (0054) is built, grammar and checker rows
> alike, and 0047's literal/enum-case type atoms are built as grammar** — their checker rows are still
> owed, and the atoms are refused by name until then. The **duration literal (0070)** is now built too,
> lexer through IR; `autoload` (0061) is the one left. M1's own section lists it.
>
> **On disk:** the workspace, CI on three platforms, lint/deny/fmt/notice policy, `mwl-diagnostics`,
> `mwl-syntax` (+ `duration`, ADR 0070's one grammar, and `bidi`, ADR 0087's one predicate),
> `mwl-hir`, `mwl-types` (+ `layout`, `core_lib`, `error_lib`, `iter_lib`, `generics`,
> `conformance`, `defaults`, `derive`), `mwl-ir`, `mwl-runtime` (+ `object`, `array`, `throwable`,
> `closure`, `identity`, `decimal`), `mwl-stdlib` (`Arr` × 36, `Str` × 28, `Math` × 38, `format`,
> `granularity`, `ordering`, `cldr`, `instance`, `issue`, `Order`, `RoundMode`, `Unit` and
> `Weekday`, `Regex` × 6 plus `Regex\Match` × 4 over `regex`/`fancy-regex`, `Time` × 7 plus
> `Time\Instant` × 9, `Time\DateTime` × 14, `Time\Duration` × 19 and `Time\Zone` × 4 (+ `UTC`) over
> `jiff`, `Json` × 4 over `serde_json`, and the conformance-coverage gate), `mwl-codegen`, `mwl-cli`
> (`ast`, `check`, `run`, `test`, `info`), `mwl-test` (+ `case`, `expect`, `run`),
> `tests/conformance` × 324 (in `array`, `class`, `core`, `enum`, `error`, `iter`, `lang` and
> `reject`) and `tests/differential` × 86, `fuzz/`, `tools/`, `benches/abi-probe`.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** checker-side rules for ADRs 0007, 0010, 0013, 0014, 0015, 0021, 0022, 0024,
> 0027, 0028, 0029/0030, 0033, 0036, 0037, 0038, 0054, 0062, and 0043's syntax +
> default/private-method slice; end-to-end for 0007 §§ 2 and 4's `%` row, 0010, 0013, 0014 § 1, 0023
> § 1, 0035 § 4, 0031 §§ 1-2, 0065, **0029/0030** (the casing checker existed but no pipeline called
> it), **0053 and 0009 in full**, **0054 §§ 1–4 in full** (`Core\Decimal`'s own roster is M8's, and
> that ADR's *Verification* says so), and **0066 §§ 1–3 for the checked numeric targets** — a
> nullable binding, parameter, property and return, `null` itself, `??` and now `as
> ?int`/`?uint`/`?float`/`?decimal` all run, leaving that ADR's enum target and every § 3 *refusal*
> owed, and **0056 §§ 1, 2 and 5** — both engines are bound, the tier is chosen by the pattern and
> the backtracking budget throws, leaving § 3's compile-time tiering (which waits on ADR 0057) and §
> 4's pattern sink (which waits on a qualifier the registry can state), and **0070 in full but its §
> 3 constant** — `30s` lexes as one token, types as `Core\Time\Duration` and runs, over the one
> grammar `mwl_syntax::duration` holds for the lexer, `Duration::parse` and M6's `mwl.toml`; what is
> owed is the *immortal* constant-pool value, which `mwl-runtime`'s gap 3 owes a string literal too,
> and **0071 §§ 1–3, § 5 and § 6 in full for a scalar-fielded class** — the nominal match, the field
> list, both per-field overrides, the three refusals, `ParseError::issues` and now the generated
> *decoder* behind `Core\Json::decodeAs<T>` all run: a decode is an ordinary `new`, every bad field
> is reported from one throw, and `?T` accepts a present `null`. What that decoder still owes is §
> 2's wider codec-reachable set and § 4's two default-bearing rows, both `mwl_stdlib::json`'s own
> gaps. and **0087's lexer half in full** — `mwl_syntax::bidi` is the one predicate, `E0008` is a
> hard error with no suppression over comments, string literals and inline HTML per line, and that
> ADR's two sink callers are M7's and M8's, and **0090 § 1 in full** — `===`/`!==` are `E0232` at
> the lexer, the two token kinds and the two `BinaryOp` variants are deleted, and the null test
> narrows and lowers through `Eq`/`NotEq`, and **0090 § 2 with 0069 § 2** — the two operand
> refusals, `E0466` for an equality between two statically disjoint types (which § 6 points a
> `switch` label and a `match` arm at as well) and `E0467` for `+`/`+=` with an array operand.
> Instance calls dispatch on the receiver's runtime class. Each ADR's own *Verification* section
> says what its slice covers, not this field.
>
> **Open now:** **Catch-up outranks `Core` breadth.** Eleven ADRs (0080-0090) landed after the
> milestones that own their work were reported done, and
> [docs/agent/loop-goal.md](agent/loop-goal.md) § *Stage 0* is the ordered list the loop works
> before opening another `Core` slice; `loop-goal.toml`'s `stage = "0 catch-up"` block is its
> machine half and runs before the program legs. **ADR 0094 is built** — a property, class constant
> or method with no `public`/`protected`/`private` is `E0122` from `mwl_syntax::check_declarations`
> (the walk that was `check_casing`), a bare `(set)` names the pair it is missing, a class-body
> `var` is redirected in the parser, and a plain constructor parameter stays exempt because
> visibility is what promotes it. What is left, in order: **ADR 0090 § 2 at run time**, since §§ 1,
> 3 and 5 are built (`===`/`!==` are `E0232` at the lexer, the whole 48-file corpus is rewritten,
> `E0466` refuses two statically disjoint operands — at `==`/`!=`, at a `switch` label and at a
> `match` arm alike — and § 3's non-scalar rows now reach `mwl_str_eq`, the new `mwl_array_eq`, an
> inline pointer compare and, for a `mixed` operand, `Helper::Identical` over
> `mwl_runtime::value_identical`, which answers `false` for a mismatched runtime pairing rather than
> throwing): the numeric types are one domain there and `mwl-types` accepts `$n == $f`, but
> `mwl-codegen` refuses two representations, so one side wants widening before the compare and the
> rows pinned in `crates/mwl-types/tests/equality.rs` go back into the conformance case; **ADR 0047
> § 4**'s literal and enum-case atoms, which parse and are refused by name;
> **`private`/`protected`**, which nothing enforces; **`Comparable`/`Stringable`**, which carry no
> member signatures, so `$s->toString()` is `E0405` and `instanceof Stringable` panics `mwl-ir`; and
> **ADR 0061**'s `autoload` grammar and its name-to-file fixpoint. **ADR 0069's refusal is built** —
> `+`/`+=` with an array operand is `E0467` naming `Core\Arr::underlay` — leaving that ADR's
> combination *members* to § 1's `Core` breadth below. **ADR 0087 is built** — `mwl_syntax::bidi` is
> the one predicate, the lexer reports `E0008` per line over comments, string literals and inline
> HTML, and seven `.mwlt` cases pin it; its `Core\Html` and `Core\Cli` sink halves are M7's and
> M8's. **ADR 0088 opens one registry-wide item**: `mwl-stdlib`'s member rows carry no qualifier
> classification, so `Core\Str::format`'s template is not yet the sink that ADR makes it, and
> neither the fail-closed default for an unclassified `string`/`bytes` parameter nor the test that
> refuses an unclassified member exists; it lands with M4S's remaining sections. **Then `Core`
> breadth, where Stage 3 stopped.** Spec §§ 3 and 6 are whole, § 2's aggregations are written, § 5
> is six of its eight members, and § 1 is missing its eleven text-shaping rows,
> `Arr::diff`/`intersect` and ADR 0069's combination members. Every signature shape the spec writes
> can now be stated: a variadic tail is `registry::CoreTy::Variadic`, a `Core`-owned instance is
> `CoreClass`'s `instance` roster over `mwl_stdlib::instance`, a class constant may be an instance
> through `registry::Const::Built`, and a member may be handed the class its call site wrote through
> `registry::WRITTEN_CLASS_MEMBERS`. § 4 runs whole but owes `Date`/`TimeOfDay`/`Core\Month` (that
> module's gap 1). **ADR 0071 is built end to end for a scalar-fielded class**; its gaps are no
> enum/`decimal`/`Instant`/`array`/nested-class field decode and no optional key from a parameter
> default. **Next on the path is `examples/collect.mwl`**, which needs §§ 7-9 and 11-12 at once:
> `Core\Path` is the cheapest slice, then `Encoding`/`Hash`/`Uuid`, then `ObjectSet`/`ObjectMap`,
> which additionally need `new Core\X<T>()` to parse. One PHP divergence stands unfixed — an
> abandoned generator never runs the `finally` it is suspended inside, `mwl-ir`'s gap 18. In docs,
> `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain remaining, reported by
> `python tools/check-migration.py`. **Three more ADRs are decided and unbuilt but are not
> catch-up** — 0091 (the `development`/`production` run mode: a `[mode]` block, a `System` ceiling,
> four governed directives, `Core\Env::mode`), 0092 (one diagnostic record rendered as plaintext,
> JSON or HTML by the sink in force — `Core\Log`'s `Log\Level`, `Core\Debug::dump`, throwables, test
> results and compiler diagnostics) and 0093 (`mwl service`). None invalidates built behaviour or a
> written fixture; their work is M4, M6, M7, M8 and M10 and lands with those milestones. Off path:
> ADR 0043's `by`-delegation.
>
> **Blocking:** nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in `docs/agent/loop-goal.md` § *Standing decisions*, including the
> `?T`/`mixed` representation (now settled and recorded in `mwl_ir::Ty::Tagged`), re-scoping a
> `catch` binding to its own handler block, widening `array<T>` to element-covariant-on-read (now
> settled and recorded in `mwl_types::expr::is_assignable`), object identity (now settled and
> recorded in `mwl_runtime::identity`), and picking every dependency but the two the user named.
> Stages 1 and 2 pass whole on both legs, so any failure below Stage 3 is a regression rather than
> unfinished work. The loop is on **Stage 3**, `Core` Part I across all twelve spec sections: six of
> its seven fixtures produce their frozen output — `examples/core.mwl`, `report.mwl`, `numbers.mwl`,
> `text.mwl`, `dates.mwl` and `json.mwl` — and `collect.mwl` is the first that does not. That one
> fixture names spec §§ 7, 8, 9, 11 and 12 at once, so it is several slices rather than one:
> `Core\Path` needs no new dependency, `Encoding`/`Hash`/`Uuid`/`Csv` each need one picked under ADR
> 0051 § 4, and `ObjectSet`/`ObjectMap` additionally need `new Core\X<T>()` to parse.

**How this document relates to the ADRs.** This is the plan of record: *what* gets built, in what order,
and how each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR this document links to it instead of
restating it — follow the link rather than expecting the argument here. For decisions with no ADR of their
own, the *why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the
*mechanics* are in this document's **Architecture** section.

## Context

This is the living implementation plan for **MWL**, the language this repository builds. It is the
single place where the plan of record lives; every decision it states is argued out in full in
[docs/adr/](adr/README.md). Keep the status block above in sync as milestones land.

The goal is a new programming language for web servers and CLI, written in Rust, that:

- takes PHP 8.5 syntax as its starting point so existing PHP projects can be migrated,
- compiles to native code just-in-time with no build step (edit file → run),
- has first-class in-language parallelism,
- can run another `.mwl` file as a fully isolated unit of work **inside the same process**, so a script
  never has to spawn an interpreter to get isolation ([ADR 0006](adr/0006-isolated-script-execution.md)),
- is memory-safe and hard to attack,
- serves HTTP from a **single process** handling unlimited concurrent, fully isolated requests while
  sharing one compiled-code cache across all of them,
- and is fast, safe and simple *first* — spending memory to stay that way rather than the reverse
  ([ADR 0004](adr/0004-memory-for-simplicity.md)).

**Who this is for, and what it claims** — [ADR 0080](adr/0080-the-audience-mwl-is-built-for.md) owns both
and this states only the headline. The first serious user is the **multi-tenant or regulated platform**: a
team whose process runs code, or holds data, at more than one trust level. MWL makes exactly three claims to
that user, and no incumbent language can add any of them later — **injection and secret leakage are compile
errors**; **a request, a job, a connection and an untrusted script are each a budgeted isolate in one
process**; and **suspension has no colour**. Raw speed against PHP is measured
([ADR 0026](adr/0026-performance-measurement-methodology.md)) and is not the pitch: persistent-worker PHP
runtimes and PHP 8's JIT have answered enough of that argument that it no longer justifies a rewrite on its
own. The PHP-shaped syntax is an **on-ramp, never a compatibility promise**, and ADR 0080 § 3 forbids any
document from implying otherwise.

Intended outcome: a self-hosted toolchain (`mwl` binary) that runs `.mwl` files on the CLI, serves them
over HTTP from one process, ships a working framework and a supply-chain-safe package system
([0082](adr/0082-the-first-party-framework.md), [0081](adr/0081-packages-are-digests-resolution-is-a-maximum.md)),
and can mechanically transpile an existing PHP codebase's *own* application code — including its `.phpt`
test suites — into MWL.

---

## Confirmed design decisions

Every row that names an ADR states only the headline — open that ADR for the mechanism, the exact
spellings rejected, and the reasoning. Do not restate that detail here when adding a row.

| Area | Decision |
|---|---|
| Implementation language | Rust (stable, pinned via `rust-toolchain.toml`) |
| Resource priorities | Security → semantics → latency → simplicity → memory footprint, in that order, within an enforced per-request cap ([ADR 0004](adr/0004-memory-for-simplicity.md)) |
| Execution | Cranelift JIT from day one, no interpreter tier; baseline codegen first, optimising tier later |
| Code cache | Content-addressed on-disk cache (BLAKE3) + in-process `Arc` sharing; hot-reloads on an edit via a per-path pointer swap, no watcher, no restart ([ADR 0017](adr/0017-hot-reload-without-restart.md)); the on-disk file format, its mmap-verify-then-execute read path and its eviction policy are [ADR 0042](adr/0042-on-disk-artifact-cache-format.md) |
| Parallelism | Hybrid: `async`/`await` for I/O inside a task (same heap, cooperative) + isolated workers on other cores for CPU work |
| Suspension | Stackful coroutines — no async colouring; any function may yield |
| Isolated execution | `spawn script 'file.mwl'` runs another file in-process as a child isolate, file-only, never a source string ([ADR 0006](adr/0006-isolated-script-execution.md)) |
| Type system | Static, mandatory, explicit; every binding's declared type never changes; `uint` alongside signed `int` ([ADR 0007](adr/0007-explicit-type-system.md)) |
| Enums | A closed, named integer type, C#-style; PHP's class-like enum design (`::cases()`, methods, `string` backing) is disregarded entirely ([ADR 0010](adr/0010-enums-are-a-value-type.md)) |
| Scoping and state | `static` is a class-member modifier only; no function-scope `static`, no `static fn`, no `global` ([ADR 0008](adr/0008-static-and-global.md)) |
| No superglobals | No variable is ever populated by the host; every PHP superglobal becomes a `Core` accessor class, and `$GLOBALS`/`$_REQUEST` have no replacement ([ADR 0012](adr/0012-no-superglobals.md)) |
| Object comparison | Ordering two objects requires the global `Comparable` interface; PHP's ambient property-walk fallback is rejected outright ([ADR 0013](adr/0013-comparable-interface.md)) |
| Property access | A property's own hook runs first, then a declared `PropertyObserver` second, always both, never a fallback for a missing property ([ADR 0014](adr/0014-property-observer.md)) |
| OOP-only: no free functions, no global constants | Every callable is a method, every constant a class constant; built-ins live under `Core` domain classes ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)) |
| Name aliasing | No `class_alias` or import `as`; a compile-time-only `type` alias for a type expression is the one exception ([ADR 0015](adr/0015-no-name-aliasing.md)) |
| Code reuse | No `trait`; shared behavior is a `public`/`private` interface method body, shared state is explicit `implements Interface by $field;` delegation, and any resulting name collision is always a compile error requiring an explicit override — there is no `insteadof` ([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md)) |
| PHP compatibility | Pragmatic superset of the syntax, not of the type discipline: PHP 8.5 syntax accepted, `strict_types` implicit, no `eval`/`$$var`/`goto`/`extract()`/`settype()`/pipe operator (`\|>`, deliberately unparsed — see `mwl-syntax`'s module docs). Existing PHP does not run unconverted — see *Consequences to accept* below, and each ADR above for its own divergence from PHP |
| Templating | `<?mwl … ?>` inline-HTML mode, `<?= ?>` short echo, `.mwl` extension. Explicit escaping (not auto) |
| Request state | Strict shared-nothing: only compiled code survives a request; no connection pooling in v1 (seam reserved). A request is the root isolate of a tree; `spawn script` adds children to it |
| Regex | Pure Rust two-tier: `regex` (linear-time) → `fancy-regex` (lookaround/backrefs) fallback |
| Security | Server-level `mwl.toml`, root-owned, TOML ([ADR 0064](adr/0064-configuration-file-format.md)), deny-by-default capabilities + hard per-request limits ([ADR 0005](adr/0005-config-changeability.md)) |
| Serving | Built-in HTTP/1.1 + h2c server. FastCGI deferred to optional transport. HTTP/3 out of scope |
| Text and binary | `string` is guaranteed-valid UTF-8 and counts extended grapheme clusters; binary data is the separate `bytes` primitive, counting bytes ([ADR 0009](adr/0009-string-and-bytes.md)) |
| Databases | One `Core\Db` API over MySQL, MariaDB (a driver of its own, not a MySQL version), PostgreSQL, SQLite and MS SQL Server: connections named in root-owned config, every statement prepared, a transaction is a closure ([ADR 0067](adr/0067-core-db.md)) |
| Tooling | LSP + formatter, test runner, debugger + profiler, package manager |
| Audience | Multi-tenant and regulated platforms first; the pitch is isolation and qualifiers, and PHP syntax is an on-ramp rather than a compatibility promise ([ADR 0080](adr/0080-the-audience-mwl-is-built-for.md)) |
| Packages | Content-addressed source archives from a first-party registry or (root-only) a git URL, resolved by minimal version selection, with no package code running before the program and capabilities granted per package ([ADR 0081](adr/0081-packages-are-digests-resolution-is-a-maximum.md)) |
| Framework | First-party and split by [ADR 0051](adr/0051-standard-library-tiers.md)'s six tests: privileged halves in `Core`, the opinionated layer as the `mwl/web` package; no ORM, no runtime container, the language is the view layer ([ADR 0082](adr/0082-the-first-party-framework.md)) |
| Real-time | WebSocket and SSE connections are root isolates opened the way a script is spawned; fan-out is a bounded `Core\Topic` ([ADR 0083](adr/0083-persistent-connections-are-isolates.md)) |
| Background work | A durable job is a row in a `Core\Db` table, enqueued inside the caller's transaction and run as an isolate ([ADR 0084](adr/0084-durable-background-jobs.md)) |
| API contracts | OpenAPI 3.1 generated while compiling from the route table and derived codecs, with `mwl api diff` as a breaking-change gate ([ADR 0085](adr/0085-openapi-is-generated-from-the-route-table.md)) |
| Testing | Hand-written suite is normative; `.phpt → .mwlt` transpiler imports PHP's corpus |
| Migration | `mwl convert` — real PHP→MWL transpiler |
| Extensions | Three tiers: built-in, sandboxed **WebAssembly components** (`.mwlx`), statically linked native. No `dlopen` ([ADR 0003](adr/0003-extension-system.md)) |
| Platforms | Windows x86_64, Linux x86_64, macOS (x86_64 + aarch64) |
| Licence | MIT |

### Rationale for the two calls left open

**Built-in HTTP server over FastCGI.** FastCGI's worst vulnerability class — the
`SCRIPT_FILENAME`/`PATH_INFO`/`cgi.fix_pathinfo` RCE family — exists *because* the decision of which file
to execute is split between web server and runtime. A native server keeps that decision in one place.
Throughput over loopback/UDS differs by single-digit microseconds per request, irrelevant beside script
execution. A bespoke FCGI record parser would be attack surface we own; `hyper` is memory-safe and among
the most-fuzzed HTTP stacks in existence. Isolation guarantees live in the host, not the protocol, so
transports sit behind a trait and FCGI can be added later for shared hosting/IIS.

**HTTP/1.1 + h2c, no HTTP/3.** nginx `proxy_pass` speaks HTTP/1.1 upstream only → h1 keep-alive is
mandatory. Caddy/Traefik/HAProxy/Envoy support cleartext h2 upstream, and `hyper` provides h1+h2 in one
crate → h2c is nearly free. No production proxy speaks HTTP/3 to an origin; the edge terminates QUIC and
talks h1/h2 upstream → h3 is pure cost.

### Consequences to accept

- **`Hello World` is ~3–5 weeks out, not day one.** With no interpreter tier, the first program requires
  the whole front end *plus* a working native backend. Mitigation: the backend ships as a *baseline* tier
  where every operation lowers to a call into a Rust runtime helper — mechanically close to an interpreter
  loop, so it is fast to get correct, and typed inlining layers on afterwards without redesign.
- **Database connections are pooled per core, so a connection reset is a security boundary.** Shared-nothing
  is a rule about *program* state; a connection is host state MWL code cannot observe, so pooling it costs
  the model nothing ([ADR 0067](adr/0067-core-db.md) § 13). What it does cost is a reset that must be
  provable rather than best-effort — a connection that cannot be proven clean is destroyed, because one
  tenant's session state arriving in another tenant's request is a leak, not a performance bug.
- **An existing PHP application's framework and packages do not come along.** No trait, no `__call`, no
  `ArrayAccess`, no runtime autoloader ([ADRs 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md),
  [0014](adr/0014-property-observer.md), [0053](adr/0053-iteration-and-generators.md),
  [0061](adr/0061-compile-time-autoload-and-program-discovery.md)) means the ecosystem built on those is
  unreachable at any price — so `mwl convert` (M11) ports an application's own code onto MWL's own
  framework, and never onto its old one. [ADR 0080](adr/0080-the-audience-mwl-is-built-for.md) § 4 records
  why this is survivable and what the alternative cost.
- **A JIT means a native-codegen component in the trusted core.** User programs stay fully memory-safe
  (all codegen is type-checked and bounds-checked); the codegen itself, the coroutine stack switcher and
  the arena are the audited unsafe surface. See "Unsafe policy".
- **Resident memory is higher than a footprint-tuned runtime's, and scales with in-flight concurrency
  rather than request rate.** 64 KiB of reserved stack per task, 16-byte values, copy-on-write clones and a
  per-request arena held to its peak are all deliberate purchases of safety, speed or simplicity. Sizing a
  deployment therefore means sizing for concurrency — tasks × stack, plus concurrent requests × their
  `memory` cap — and deployment docs must say so rather than quote a typical RSS.
- **Existing PHP does not run unconverted.** PHP has no syntax for the type of a `foreach` binding or a
  destructuring target ([ADR 0007](adr/0007-explicit-type-system.md)), and every one of its global
  functions and global constants needs a new home on a class before it type-checks at all
  ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)). A plain local has a type-eliding
  spelling now — [ADR 0037](adr/0037-var-local-type-inference.md)'s `var` — so `mwl convert` can emit that
  directly instead of inferring and writing an annotation, but it still has to write annotations and
  rewrite call sites for everything else, not just drop `.php` files into a document root. M11 stays
  mandatory rather than a convenience, and the imported `.phpt` pass rate is still structurally lower than
  a compatibility-first design's, not through bugs.
- **One new language construct that PHP has no equivalent of.** `spawn script` is a surface a developer
  has to learn and the spec has to define next to `require`, which they will confuse it with. Accepted:
  the requirement it answers — run another file, isolated, without a second process — has no other honest
  answer, and it reuses the request boundary and the worker value rules rather than adding either
  ([ADR 0006](adr/0006-isolated-script-execution.md)).
- **This is an 18–24 month effort at full-time pace for one experienced engineer**, front-loaded: M0–M4
  (a usable CLI language) is roughly 3–4 months; the stdlib and DB drivers are the long tail.

---

## Validated premises

The M0 spikes are now permanent guard tests in [`benches/abi-probe`](../benches/abi-probe/), because
several decisions here rest on how Cranelift, `corosensei` and Wasmtime behave rather than on MWL's own
code, and a dependency bump can invalidate them silently. **The tests own the numbers; this document does
not restate them.**

| Premise | Guarded by | Argued in |
|---|---|---|
| Native unwinding through JIT frames is unavailable on every platform — the premise the calling convention exists for | `tests/unwind_unavailable.rs` | [0002](adr/0002-error-propagation.md) |
| A throw propagates, and a runtime panic is *contained*, across native frames | `tests/invariants.rs` | [0002](adr/0002-error-propagation.md) |
| A checked-return frame stays cheap, and throwing costs no more than returning | `a_checked_return_frame_stays_cheap`, `throwing_costs_about_the_same_as_returning` | [0002](adr/0002-error-propagation.md) |
| A coroutine can suspend from beneath live JIT frames, cheaply | `a_helper_can_suspend_with_jit_frames_live_above_it`, `a_coroutine_round_trip_stays_cheap` | [adr/README.md](adr/README.md), *Stackful coroutines* |
| A wasm guest cannot read past the host heap or outlive its deadline, and the boundary is affordable | `tests/wasm_sandbox.rs`, `a_host_to_guest_call_stays_cheap`, `per_request_instantiation_stays_affordable` | [0003](adr/0003-extension-system.md) |
| An OS process still costs orders of magnitude more than a task — the whole cost case for in-process isolation | `an_os_process_costs_orders_of_magnitude_more_than_a_task` | [0006](adr/0006-isolated-script-execution.md) |

Those spikes forced one design change, and it is normative for everything below: **exceptions propagate by
checked return, not by unwinding**, and every runtime helper is `extern "C"` wrapping `catch_unwind`. The
signature, the measured cost and the reasoning are in [ADR 0002](adr/0002-error-propagation.md).

## Extension system

Three tiers, each the right answer for a different class of code rather than a compromise. The full
reasoning — the WIT interface, the handle-table value model, the measured boundary costs, the isolation and
loading rules, and the decisive rejection of `dlopen` — is in [ADR 0003](adr/0003-extension-system.md).

- **Tier 0 — built-in (`mwl-stdlib`).** Compiled into the binary, native, direct heap access, no boundary.
  Home of the fine-grained primitives whose total cost is comparable to a call.
- **Tier 1 — WebAssembly component extensions (`.mwlx`).** The default for third parties: one binary that
  runs on every platform, sandboxed by construction, authorable in any language `wit-bindgen` targets.
- **Tier 2 — statically linked native.** A Rust crate compiled into a custom `mwl` binary, for first-party
  subsystems needing raw sockets, TLS or direct heap access — `mwl-db`, `mwl-regex`, crypto. It requires
  building from source, which is the right friction for code that runs unsandboxed.

One sequencing constraint, which is why this section is in the plan at all: the `mwl:ext@1.0.0` WIT world
must be authored **during** the stdlib milestone (M8), from the same value-access design as the built-ins,
so that the Tier 0 internal interface and the Tier 1 guest interface are one design rather than two that
drift. Deferring it to M9 would mean retrofitting.

## Architecture

```
                    .mwl / .php source
                            │
    ┌───────────────────────▼───────────────────────┐
    │ mwl-syntax    lexer (dual mode, inline HTML)  │
    │               parser → AST + spans            │
    ├───────────────────────────────────────────────┤
    │ mwl-hir       name resolution, namespaces,    │
    │               class graph, symbol table       │
    ├───────────────────────────────────────────────┤
    │ mwl-types     declared types, unions, flow    │
    │               narrowing, no inference engine  │
    ├───────────────────────────────────────────────┤
    │ mwl-ir        CFG/SSA, safepoints, refcount   │
    │               ops, optimisation passes        │
    ├───────────────────────────────────────────────┤
    │ mwl-codegen   Cranelift → native code (W^X),  │
    │               status checks, helper calls     │
    └───────────────────────┬───────────────────────┘
                            │  Arc<CompiledUnit>  (immutable, shared)
    ┌───────────────────────▼───────────────────────┐
    │ mwl-host   unit cache (single-flight compile) │
    │            isolate = arena + globals + limits │
    │            (a request is one isolate's root)  │
    └───────┬───────────────────────────────┬───────┘
            │                               │
    ┌───────▼───────┐               ┌───────▼───────┐
    │ mwl-http      │               │ mwl-cli       │
    │ h1 + h2c      │               │ run / test    │
    └───────────────┘               └───────────────┘
            (mwl-fcgi later, same Transport trait)
```

### Thread-per-core, shared-nothing runtime

The decisive structural choice. One single-threaded Tokio runtime **pinned per CPU core**; connections are
load-balanced across cores; a request never migrates between cores.

- Value refcounts are **non-atomic** — no atomics on the hot path, because a heap is only ever touched by
  one thread.
- True multicore parallelism comes from N independent runtimes.
- `spawn worker` for CPU-bound work dispatches to another core; the value crossing the boundary is
  deep-copied (moved when refcount == 1). This is exactly the isolated-worker semantics already chosen, so
  the model is internally consistent rather than a compromise.
- Compiled units are immutable, so they are shared across all cores via `Arc` with zero copying.

### Value representation

16-byte tagged value: `{ tag: u8, _pad: [u8;7], bits: u64 }`. NaN-boxing is rejected because PHP semantics
require full-range `i64`. Tags: `null | bool | int(i64) | uint(u64) | float(f64) | string | array | object |
closure | resource | decimal`. `uint` is a tag, not a wider slot, so it costs nothing here; the type system
that demands it is [ADR 0007](adr/0007-explicit-type-system.md), which also owns the array element-type stamp
carried on the array header. `decimal` is the one tag whose value does not fit the payload alone — its 96-bit
mantissa spends the padding bytes too, so it is the whole sixteen — and `mwl_runtime::decimal`'s own module
doc is the home for that layout.

Memory: refcounting + copy-on-write arrays/strings (PHP semantics). Reference cycles are bounded by the
request lifetime — the whole request heap is dropped wholesale at request end, which makes cycle leaks
structurally impossible to accumulate in the server. Long-running CLI scripts additionally get an optional
mark-sweep cycle collector running at safepoints; whichever milestone implements its run routine also gives
it a `gc`-kind trace event, at no cost to the safepoint poll itself
([ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)).

Both choices here — 16 bytes per value instead of 8, and peak-not-average retention inside a request — cost
memory to buy correct PHP semantics and a collector that never runs on the request path. That is the
priority ordering in [ADR 0004](adr/0004-memory-for-simplicity.md), not an oversight to optimise away later.

### Safepoints — build these into codegen from the very first commit

Codegen emits a cheap poll (load a per-task flag, branch) at every loop back-edge and function entry. This
single mechanism delivers:

1. CPU-time limit enforcement and wall-clock timeouts,
2. cancellation when a client disconnects,
3. cycle-collector and profiler stop-the-world points,
4. debugger breakpoints,
5. deoptimisation/OSR points for the optimising tier.

Retrofitting safepoints later would mean rewriting the backend. They are not optional.

### Compiled-unit cache with single-flight compilation

```rust
enum CompileState {
    Compiling(broadcast::Receiver<Result<Arc<CompiledUnit>, Arc<Diagnostics>>>),
    Ready(Arc<CompiledUnit>),
    Failed(Arc<Diagnostics>),
}
// DashMap<UnitKey { path, content_hash, env_hash }, CompileState>
```

The first requester inserts `Compiling` and compiles on a **dedicated compile pool** (never on a
request-serving core, so compilation cannot stall request handling). Concurrent requesters await the same
broadcast — N simultaneous first-hits compile exactly once, and none of them block a core. Staleness:
`stat` (mtime+size) → BLAKE3 content hash → atomic swap. Governed by
`opcache.validate = never|mtime|hash`. Native pages are mapped `RX`, never `RWX` (W^X discipline).

Because the key above is not `{path}`, a `Ready` entry is write-once — two versions
of a file are two entries, never one overwritten. A small separate index, `path → current content_hash`,
sits in front of it and is the one thing a hot-reload actually swaps; [ADR 0017](adr/0017-hot-reload-without-restart.md)
holds the only copy of that mechanism, why revalidation needs no filesystem watcher, and how a file edited
under a live server reaches the next request with no restart while a request already running keeps the
version it started with. The third key field, `env_hash`, is the environment a unit was compiled in —
including the loaded extension set — so a config reload that changes that set turns every unit into an
ordinary cache miss rather than needing an invalidation pass
([ADR 0078](adr/0078-config-reload-and-control-socket.md)).

### Per-request isolation

Each request gets: its own heap arena with a hard byte cap; fresh backing state for the `Core\Request`/
`Core\Server`/`Core\Session` accessors ([ADR 0012](adr/0012-no-superglobals.md), replacing PHP's
superglobals); a copy-on-write overlay of the config; its own coroutine tree. At request end the arena is
released
wholesale. `catch_unwind` at the request boundary means a runtime panic kills one request, never the
process, which is why `panic = "unwind"` is load-bearing in every profile. Every capability check
consults the *request's* config snapshot, so a script cannot affect its neighbours.

### In-process isolated script execution

The provisional surface, the semantics, and what is/is not shared are decided and stated in full in
[ADR 0006](adr/0006-isolated-script-execution.md) — including the `spawn script … with(…)` example, how
values cross (the graph-copy operation [ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md) now
formally defines, shared with `serialize()`/`unserialize()`), how budgets are accounted (at the root of the
request tree, never per isolate), the `script.spawn` capability and its path resolution, and failure
arriving as a value rather than as an unwind. The spec pins the exact grammar down in M5. The three
invariants no optimisation may trade away are listed in [AGENTS.md](../AGENTS.md).

The structural consequence for this plan: `mwl-host` gains **one** `Isolate` type, and an inbound HTTP
request *is* the root isolate of its tree. The server path (M7) and the `spawn script` path (M5) therefore
share one arena setup, one teardown, one place limits are enforced — and one state-bleed test suite. That
is why isolates land in M5, before the server that depends on them.

### `mwl.toml` — server-level, root-owned

A directive registry in a root-owned TOML file, with per-app capability blocks living in the *root* config so
an application can never grant itself rights. `mwl.toml` states **defaults, not ceilings**: a directive is a
limit that cannot be exceeded only when it cannot be changed at runtime at all. Each directive carries one
of three changeability classes — `System`, `Runtime`, `RuntimeTighten`.

[ADR 0005](adr/0005-config-changeability.md) holds the only copy of the directive layout: the classes and
why each is argued per directive, the `[core]`, `[limits]`, `[limits.hard]`, `[capabilities]` and `[app]`
tables, and the `Core\Config::set`/`::get`/`::restore` overlay rules.
[ADR 0064](adr/0064-configuration-file-format.md) holds the only copy of why the file is TOML rather than
INI, and of the `Core\Config` signatures. Do not restate either here.

Two cross-cutting consequences the milestones below depend on:

- A widened limit lives on the request-local copy-on-write overlay, so it dies with the request that set it
  and is never visible to another. A set refused by a ceiling or a class returns `false` and leaves the
  value unchanged — it is **not** clamped.
- Every `[limits]` value is accounted against the **root of a request tree**, not per isolate, which is
  what keeps the process worst case independent of how many isolates a script creates
  ([ADR 0006](adr/0006-isolated-script-execution.md)).

---

## Repository layout

The layout, and which crates exist today versus which are deferred:
[README](../README.md#repository-layout). Crates for later milestones are created when their milestone
starts rather than sitting empty.

### Unsafe policy

`unsafe_code = "forbid"` workspace-wide. Crates that genuinely need it opt down to `deny` and allow
individual blocks with a stated reason: `mwl-runtime` and `mwl-codegen` (planned — the coroutine stack
switcher, the request arena, JIT page mapping), `mwl-stdlib`, whose every `Core` member is an
[ADR 0002](adr/0002-error-propagation.md) helper entry point and therefore an `extern "C"` function
decoding raw pointers, plus `benches/abi-probe`, which must call JIT-compiled code
to measure it and is `publish = false`, so it does not widen the runtime's unsafe surface. Those modules
carry `deny(unsafe_op_in_unsafe_fn)`, a safety-invariant doc comment per block, dedicated Miri/ASAN
coverage, and require an ADR to grow. Prefer `corosensei` (audited, handles Windows SEH and aarch64) over a
hand-rolled switcher. The policy is enforced in [Cargo.toml](../Cargo.toml).

---

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

### M0 — Project setup (~3 days) — **done**
Workspace scaffold, the CI matrix across the three platforms, `clippy -D warnings`, `rustfmt`,
`cargo-deny`, `cargo-fuzz`, the licence, and the first ADRs recording the decision table above. The
architecture spikes were promoted into `benches/abi-probe` as permanent guard tests rather than left in a
scratchpad.

**Verified:** `cargo test` / `cargo clippy` / `cargo deny check` green on all three platforms in CI.

### M1 — Front end (~3 weeks)
Lexer with dual mode (`<?mwl`, `<?php`, `<?=`), inline HTML, heredoc/nowdoc, string interpolation, all
PHP 8.5 tokens. Recursive-descent parser covering the pragmatic-superset grammar: classes, interfaces,
enums (cases and an optional backing type only — no methods, no `implements`, see
[ADR 0010](adr/0010-enums-are-a-value-type.md)), methods (a `function` declaration is only ever a class
member, static or instance — see [ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
attributes, `match`, `fn` closures — with or without a block body, and with an optional self-name for
recursion, but no anonymous `function(...) {...}` literal and no `use` clause at all
([ADR 0031](adr/0031-callable-is-the-only-closure-type.md)) — generators, named arguments, spread, nullsafe,
`readonly`, promoted constructor parameters, first-class callable syntax, property hooks (their pipeline
relative to the new `PropertyObserver` interface is [ADR 0014](adr/0014-property-observer.md)), asymmetric
visibility. Rejects, with a diagnostic naming the replacement, every construct an earlier ADR closes:
`eval`/`$$var`/`goto`/`global`/`extract`/`settype`/function-scope `static`/`static fn`
([ADR 0008](adr/0008-static-and-global.md)), anonymous `function(...) {...}`/`function(...) use (...) {...}`
and any `use` capture clause ([ADR 0031](adr/0031-callable-is-the-only-closure-type.md)), enum
methods/`implements`/`string` backing
([ADR 0010](adr/0010-enums-are-a-value-type.md)), a `function` or `const` outside a class body and a
`namespace` or class named `Core` ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
every superglobal spelling ([ADR 0012](adr/0012-no-superglobals.md)), and `use … as …`
([ADR 0015](adr/0015-no-name-aliasing.md)). Error recovery good enough for the LSP.

**M1 is re-opened twice more, once for a check that is now built and once for a spelling that is not.**

[ADR 0087](adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s predicate over every string
literal, comment and inline-HTML run is **built**: `mwl_syntax::bidi` is the one implementation of the
rule, the lexer reports `E0008` with no suppression, and each line of a multi-line token is its own span so
a heredoc cannot hide a scope across them. The predicate lives beside the lexer because its two other
callers, `Core\Html::escape` at M7 and `Core\Cli`'s sink at M8, import it rather than restate it.
Identifiers need nothing: they are already ASCII-only, which is what closes the homoglyph half of Trojan
Source structurally.

[ADR 0090](adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md) § 1 is **not** built and is
this milestone's open item: the lexer still produces `===` and `!==`, which that ADR deletes. Removing them
takes `BinaryOp::Identical`/`NotIdentical` out of the AST with them and rewrites every fixture and `.mwlt`
case that writes the rejected spelling; the corpus-parse test needs nothing, since it holds "the parser
does not panic" rather than "php-src parses cleanly". The rest of that ADR is M2's and M3/M4's, below.

Plus the type grammar of [ADR 0007](adr/0007-explicit-type-system.md), which is a parser problem before it
is a checker one: nested `array<T>`, DNF unions and intersections, `uint`, the conversion operator
including its nullable form `as ?T` ([ADR 0066](adr/0066-nullable-conversion-operator.md)), and the
declaration slots PHP has no syntax for — typed locals, `foreach` bindings and destructuring targets. Also
in the grammar, each *parsed* here and *enforced* in M2: the `tainted` and `secret` qualifiers on
`string`/`bytes`, composable only as `secret tainted T`
([ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md),
[ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)); `type Name = TypeExpr;`
([ADR 0015](adr/0015-no-name-aliasing.md)); the anonymous object literal `{a: 1}` and the inline shape type
`{name: T}`, whose two collisions with a block body and a block statement are resolved by requiring
parentheses, diagnosed by name at both sites ([ADR 0036](adr/0036-anonymous-object-shapes.md)); and
`implements Interface by $field` plus a body on an interface method, replacing the `trait`/`use
TraitName`/`insteadof` grammar that is now a parse-time `E_TRAIT_NOT_SUPPORTED`
([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md)).

**Still owed — four ADRs accepted after this milestone was reported done add grammar it owns.** Each is
parser-and-lexer work that M2's checker slice is already scheduled against, so each blocks its ADR rather
than being optional. Items 1-2 are built; items 3-4 are not:

1. **`decimal`** ([ADR 0054](adr/0054-decimal-scalar-type.md)) — **built**, and its M2 checker rows with
   it: the reserved keyword, the type atom, § 2's untyped-until-placed literal (including `as T` as a
   placing position), § 1's mantissa and scale bounds, and § 3's arithmetic table. A trailing `m` is a
   stray identifier the parser refuses, not a suffix. The runtime half is built too: `mwl_ir::ty::Ty::Decimal`
   is the representation and `mwl_runtime::decimal` the value behind it.
2. **Literal and enum-case type atoms** ([ADR 0047](adr/0047-literal-and-enum-case-types.md)) —
   **built**, grammar only: a `StringLiteral`/`IntLiteral` atom, unions of them, `?"a"` sugar, and a
   class-constant or enum-case reference in type position, which needed no production beyond the
   `ClassName`/`EnumName` ambiguity ADR 0010 § 4 already established. That ADR's M2 row — § 4's
   assignability and conversion table — is **not** built, and the checker refuses all three atoms by name
   until it is; that ADR's *Verification* section owns why, and names the one step the table's own wording
   understates.
3. **`autoload`** ([ADR 0061](adr/0061-compile-time-autoload-and-program-discovery.md)) — the two
   file-scope declaration forms whose grammar
   [`docs/spec/00-overview.md`](spec/00-overview.md) § 2 already fixes. M2's name-to-file fixpoint has
   nothing to resolve until this parses.
4. **The duration literal** ([ADR 0070](adr/0070-duration-literals.md)) — **built**, and its M2 and M4S
   halves with it: one `DurationLiteral` token over `( DEC_INT unit )+`, typed `Core\Time\Duration` with
   nothing placing it, folded to a nanosecond count and lowered to the one member that also serves a
   computed `Duration::nanoseconds($n)`. The grammar is `crates/mwl-syntax/src/duration.rs`, shared with
   `Duration::parse` and with M6's `mwl.toml` exactly as § 5 requires. It met item 1 at the lexer, and the
   ADR decided the boundary: `1.5s` is that grammar's own fractional refusal, so `19.99m` is refused there
   too rather than lexing as two tokens — a suffix *outside* the unit alphabet, like `19.99x`, still does.

**Verify:** `mwl ast file.mwl` dumps the AST; `insta` snapshot tests; `cargo fuzz` on the lexer and parser
finds no panic in a 5 minute run; parse the full local `php-src` folder for `.php` files without crashing
(they will not *check* — see M2 — but they must parse). A snapshot pins the one grammar wrinkle in
ADR 0007: `as` in a `foreach` header belongs to `foreach`, so a conversion of the subject needs
parentheses. The three additions above each carry their own ADR's M1 verification line.

### M2 — HIR, types, IR (~4 weeks)
Name resolution: namespace/`use` scoping, class hierarchy resolution — no trait flattening, since traits do
not exist; instead, default-method/private-method visibility and `by`-delegation type-matching
([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md); this replaces
`crates/mwl-hir`'s already-written `hierarchy.rs` trait-use/`insteadof` resolution, per that ADR's
*Consequences*) — statically resolved `require` with a dynamic fallback
([ADR 0021](adr/0021-single-file-inclusion-construct.md)), `autoload`-declared name-to-file resolution as a
fixpoint over that same require-graph worklist
([ADR 0061](adr/0061-compile-time-autoload-and-program-discovery.md)), every callable/constant resolved as a class
member with no bare-name fallback ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
`type`-alias substitution ([ADR 0015](adr/0015-no-name-aliasing.md)), and property-access resolution with
no `__get`/`__set` fallback ([ADR 0014](adr/0014-property-observer.md)).

Type checker ([ADR 0007](adr/0007-explicit-type-system.md)): declared types enforced, definite assignment
(locals, and per [ADR 0022](adr/0022-definite-property-initialization.md) every constructor-declared
property), flow-sensitive union narrowing, array element types checked at every depth, the arithmetic
result-type table. No inference engine and no `Unknown` type — that's the simplification the mandatory
declarations buy. Also enforced here: `Comparable`-gated object ordering
([ADR 0013](adr/0013-comparable-interface.md)); `tainted` poisoning/laundering
([ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md)) and, independently, `secret`
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)); `callable` accepted only from
first-class-callable syntax or an `fn` literal, no `__invoke`
([ADR 0027](adr/0027-callable-is-closures-only.md), [ADR 0031](adr/0031-callable-is-the-only-closure-type.md));
`Stringable`-gated string conversion and `unset()` refused on a declared property
([ADR 0028](adr/0028-closing-the-remaining-magic-methods.md)); identifier casing
([ADR 0029](adr/0029-identifier-casing-is-checked.md)/[0030](adr/0030-no-leading-underscores-constructor-spelling.md),
lands in `mwl-syntax` directly since it needs no name resolution); `object` subtyping and shape-type
structural checking ([ADR 0036](adr/0036-anonymous-object-shapes.md)); `foreach` accepted over exactly an
`array<T>`, an `Iterable<T>` or an `Iterator<T>`, with `$obj[$k]` on a non-array refused
([ADR 0053](adr/0053-iteration-and-generators.md)); and `decimal`'s conversion and arithmetic rows,
including `decimal + float` refused on the same grounds as `int + uint`
([ADR 0054](adr/0054-decimal-scalar-type.md), whose literals are target-typed with no suffix, so `as T`
must place one); and the nullable target form `as ?T`, which needs no parser work and reuses § 6's `?T`
narrowing, refusing the conversions that cannot fail or do not exist
([ADR 0066](adr/0066-nullable-conversion-operator.md)).

**Re-opened here by [ADR 0090](adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md):** an
equality whose two operand types are **disjoint** is a compile error, over that ADR § 2's table and
including § 6's `switch` labels and `match` arms — no equality-operand check exists for any type pair
today. Its § 3 also moves § 6's null-test narrowing from `=== null`/`!== null` to the one remaining
spelling. The lowering half — the null tag test and a helper each for the string, array and object rows —
is M3/M4's, and `mwl-ir`'s own gap list owns it.

Lowering to a CFG/SSA IR carrying explicit safepoints, refcount operations and runtime-helper calls, with a
stable per-statement/per-edge id reserved for
[ADR 0018](adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s probes — cheap now,
expensive to retrofit once M3 builds on the IR without it. The IR must also be able to represent a
**suspension point inside a loop body**, so that [ADR 0053](adr/0053-iteration-and-generators.md)'s
state-machine lowering of a generator can be added without reshaping it. The transform itself may land
later; foreclosing it here is the expensive mistake, exactly as with the probe ids.

**Verify:** `mwl check` on a curated corpus with one fixture per diagnostic named in each ADR listed
above's own *Verification* section, plus ADR 0007's own core entries (undeclared/redeclared local,
read-before-definite-assignment, `int + uint`, array element-type violations at depth, a missing vs.
present narrowing). Plus IR snapshot tests; no program in the corpus produces an `Unknown` type, since
the IR has none.

### M3 — Baseline Cranelift backend → **Hello World** (~3 weeks)
The checked-return calling convention from [ADR 0002](adr/0002-error-propagation.md), which is normative
for the signature, the `catch_unwind` helper wrapper and the status check emitted after every call. There
is no platform unwind-table registration to do — that is the point of that ADR. Plus the runtime helper
table, `echo`, string concat, arithmetic, comparison, control flow, function calls, safepoint polls, the
debug-flags probe checks at every statement boundary and call site, and W^X page management
([ADR 0018](adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) — same cost class as the
safepoint poll, landing with it rather than after it).

Because [ADR 0007](adr/0007-explicit-type-system.md) makes operand types known by construction, the
baseline tier emits a native instruction for any single-scalar static type and falls back to the generic
helper only for `mixed`, unions and dynamic calls — a chunk of what M12 was for, arriving with the first
backend.

**Verify:** every bullet below, machine-checked by the unattended loop against the frozen `examples/*.mwl`
fixtures on Windows and Linux — `docs/agent/loop-goal.md` holds the one copy of that acceptance list and is
authoritative for it. `mwl run` prints from natively compiled code. A throw crosses several JIT frames and
is caught; a helper panic terminates the script with a `FATAL` status and leaves the process able to run the
next one. An MWL-level backtrace names the right functions, resolved from MWL's own frame chain rather than
from the platform unwinder, and is readable from MWL as a rendered string — the `backtrace` member's array
form is a deliberate carry-over to M4, where arrays land. `mwl run --dump-asm` shows generated code. A
typed arithmetic loop lowers to native instructions rather than helper calls, committed as a figure in
`benches/` with a guard, so ADR 0007's claim that mandatory types pay for themselves on the request path is
tested rather than asserted.

### M4 — Language completeness — a usable CLI language (~10 weeks)
Full ordered-hash arrays with COW — including `Throwable`'s `backtrace` member, carried over from M3, which
lands its rendered string form only — `uint` arithmetic, and the conversion operator over every row of
[ADR 0007](adr/0007-explicit-type-system.md)'s conversion table; exceptions propagating correctly by
checked return across JIT frames ([ADR 0002](adr/0002-error-propagation.md)), closures that bind `$this`
only where the body uses it ([ADR 0008](adr/0008-static-and-global.md)),
inheritance/interfaces, including default/private interface method bodies and `by`-delegation
([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md)), object ordering through
`Comparable` ([ADR 0013](adr/0013-comparable-interface.md)), enums
([ADR 0010](adr/0010-enums-are-a-value-type.md)), `decimal` arithmetic
([ADR 0054](adr/0054-decimal-scalar-type.md)), generators and `foreach` over the two iteration interfaces
([ADR 0053](adr/0053-iteration-and-generators.md)),
references (`&$x`), instance members and static members including late static binding
(`static::`, `new static()`, `: static`), property hooks and `PropertyObserver`
([ADR 0014](adr/0014-property-observer.md)), `clone`
([ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md)), `Core\Debug::dump` and its plaintext
rendering over the one record model ([ADR 0092](adr/0092-one-diagnostic-record-three-renderings.md))
— with a `secret`-qualified property's value redacted
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)) — `Stringable` and the rest of
[ADR 0028](adr/0028-closing-the-remaining-magic-methods.md), and `#[...]` attribute syntax on every
declaration it attaches to, including the explicit `<T>` call-site type argument
`Core\Attributes::get<T>`/`::all<T>` need even though their retrieval body doesn't land until M8
([ADR 0046](adr/0046-attributes-shape-literal-metadata.md)).

Also in this milestone: `mwl test` and the `.mwlt` format — deliberately a **superset of `.phpt`'s
sections**, so the M11 importer is mechanical rather than a rewrite. Both are built;
[`crates/mwl-test`](../crates/mwl-test/src/lib.rs)'s module doc is the one home for every section, what
`--EXPECTF--`'s escapes match, and why a case runs in a subprocess. What is left here is the corpus, and
the **Verify** list below is what it has to cover.

**Distinct from that, and landing at this milestone's tail with M4S:** the testing capability MWL *programs*
use — `#[Test]` and the table the compiler builds from it, `#[Fixture]`, `#[TestWith]`, the generic
`Core\Test` assertion roster, the ledger behind a catchable failure, and the human/JUnit/JSON reporters.
[ADR 0079](adr/0079-testing-is-a-language-feature.md) owns all of it, including § 24's milestone table for
the pieces that land later; `.mwlt` and `#[Test]` answer different questions and are never unified (§ 23).

**Verify:** hand-written conformance suite ≥ 1000 `.mwlt` cases green, including `uint` at `0`, `i64::MAX`,
`i64::MAX + 1` and `2^64 − 1`; every conversion in ADR 0007 both succeeding and throwing; overflow throwing
rather than promoting to `float`; key order preserved across insert, delete, re-insert and every sort
function; `array_keys()` typed `array<string>`; `json_encode` output identical to PHP's for both lists and
maps. `new static()` through two levels of inheritance returns the called class, and a closure written in a
method without naming `$this` is unbound — `bindTo()` on it rebinds nothing, which is ADR 0008's single
divergence and gets its own case. Plus one fixture per rule in the *Verification* section of ADRs
[0014](adr/0014-property-observer.md), [0023](adr/0023-clone-serialize-and-cross-boundary-copy.md),
[0028](adr/0028-closing-the-remaining-magic-methods.md),
[0046](adr/0046-attributes-shape-literal-metadata.md) and
[0069](adr/0069-array-combination-is-key-type-independent.md) — each of those sections is the one home for
what its own rule requires, including the "a class that does not implement `PropertyObserver` shows no
measurable overhead" measurement and the diagnostic `$a + $b`/`$a += $b` over two arrays now owes. A non-trivial program runs correctly and leaks nothing under Valgrind — the
*argument-parsing, file-processing* half of that program moves to M8 with `Core\Cli` (§ 15) and `Core\IO`
(§ 14), since neither argv nor a file handle is reachable before capabilities exist at M6.

### M4S — The `Core` API contract and its pure half (~5 weeks)
The library the language has been compiling calls *against* since M2 without any of it existing. Its shape
is [ADR 0063](adr/0063-core-api-conventions.md) and its member list is
[docs/spec/01-core-library.md](spec/01-core-library.md), which is authoritative for every signature; this
milestone implements **§§ 1–12** of that file — `Core\Str`, `Arr`, `Math`, `Time`, `Json`, `Regex`,
`Encoding`, `Bytes`, `Path`, the three collection types, the exception types, `Random`, `Uuid`, `Hash`,
`Uri`, `Validate`, `Csv`, `Out`. Every one is pure: no capability, no reactor, no driver, no open handle, so
none of it is blocked on M5–M7. § 13's compiler-facing surfaces are pure too but each waits on something
outside `Core`; that file's own *Milestones* section says which, and is the one home for it. It is placed here rather than at M8 so that everything after it — the LSP's
completion data, M5's concurrency tests, M9's extension conformance fixtures, M11's converter mapping
table — is written against a real standard library instead of against fixtures that will need rewriting.
Part II of the spec file (anything capability-bearing) stays at M8 and merely conforms to the same
contract. `crates/mwl-stdlib` starts here — the Tier 0 crate
[ADR 0003](adr/0003-extension-system.md) § *Tier 0* already names, and the workspace manifest already
declares; `Core\Regex` binds the engine [ADR 0056](adr/0056-regex-engine-policy.md)
picks, and `Core\Time`'s `format`/`parse` (CLDR patterns), `Core\Time\Duration::parse` and
`Core\Str::format` land as [ADR 0057](adr/0057-intrinsic-literal-folding.md) intrinsics with the
compile-time half wired into `mwl-types` — and, per
[ADR 0088](adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 6, all three grammars are
`tainted` **sinks** alongside `Core\Regex`'s pattern. **`mwl-stdlib`'s member registry gains a per-parameter
qualifier classification here**, with an unclassified `string`/`bytes` parameter refusing `tainted` and that
crate's own test suite failing on any member that ships without one (§ 2 of the same ADR); the pass that
marks the existing rows is part of building §§ 1–12 rather than a separate slice. `Duration::parse` shares its grammar and its implementation with
M1's duration literal ([ADR 0070](adr/0070-duration-literals.md)), so build the literal first and this is
the same parser reached from a second entry point. `Core\Str`'s
unit is [ADR 0009](adr/0009-string-and-bytes.md) § 2's grapheme cluster, decided and seamed in
`mwl_stdlib::granularity`; the **lazily cached count** that ADR's *Consequences* names is still owed and
belongs here, in `mwl_runtime::MwlStr`'s header, alongside the O(1) boundary correction a concatenation
needs at the seam. `Core\Json` also brings the first **compiler-recognized**
attribute: [ADR 0071](adr/0071-derived-codecs.md)'s `#[Json\Derive]`, a `mwl-types`→`mwl-ir` pass that emits
a `Json\Codec` implementation per annotated class, plus the nominal-matching rule that gates it. `#[Db\Derive]`
is the same pass over a second format and lands with M8. The **second** compiler-recognized attribute lands
here as well: [ADR 0077](adr/0077-compile-time-routing.md)'s `#[Route]`, whose route table is built by
filtering [ADR 0061](adr/0061-compile-time-autoload-and-program-discovery.md) § 3's program enumeration and
whose three compile errors — a duplicate route, a `{param}` with no matching method parameter, an unknown
literal `url()` name — are the whole point of doing it here. `Core\Router::match` itself waits for M7.
The **third** rides the same pass: [ADR 0086](adr/0086-core-cli-terminal-is-a-sink.md) § 6's
`#[Command]`/`#[Option]`/`#[Argument]` command table, with its own three compile errors — a duplicate
command name, two options sharing a spelling, an `#[Option]` on a parameter with no conversion from
`string`. `Core\Command::run` and the rest of `Core\Cli` wait for M8, since neither argv nor a terminal is
reachable before capabilities exist at M6.

**Verify:** every member in the spec file has a conformance test, and a mechanical check over that file
enforces the rules that can be checked mechanically — [ADR 0063](adr/0063-core-api-conventions.md)'s
*Verification* section is the one home for that list. PHP 8.5 is the differential oracle wherever a member
claims PHP-compatible observable behaviour (`Core\Str`, `Core\Arr`, `Core\Math`, `Core\Regex`), and each
deliberate divergence is a named fixture rather than a failing comparison. A `tainted` value cannot reach a
sink and cannot be laundered except by the members the spec marks **launder**. `Core\Arr` mutates in place
when its argument's refcount is 1 — measured, since it is the whole cost argument for
[ADR 0063](adr/0063-core-api-conventions.md) R3 — and allocates a copy when it is not.
[ADR 0071](adr/0071-derived-codecs.md)'s own *Verification* section lists the derive's cases, including the
one that matters most: a decode with four bad fields throws exactly one error listing all four. The M4 CLI program
is rewritten against `Core` and gets shorter. `python tools/check-migration.py` reports full coverage of
every PHP name this milestone's classes replace, which is the point at which
[docs/spec/02-php-migration.md](spec/02-php-migration.md)'s string, array, number and date rows stop being
a plan and become a tested claim.

**Also here: the OpenAPI emitter** ([ADR 0085](adr/0085-openapi-is-generated-from-the-route-table.md)),
alongside the `#[Route]` and `#[Json\Derive]` passes it reads. `Core\Api` joins
[ADR 0071](adr/0071-derived-codecs.md) § 1's closed attribute list, the four contradiction cases become
compile errors, and `mwl build --openapi` writes a deterministic 3.1 document. `mwl api diff` is the same
slice — the classification is mechanical over two emitted documents, so it costs a comparison rather than a
design.

### M4B — Minimal `mwl-lsp` and the VS Code extension (~3 weeks)
Pulled ahead of M10 by [ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md) so real-world
testing in an editor starts the moment M4 makes MWL a usable CLI language, rather than after M5–M9.
`crates/mwl-syntax` gains a second, error-recovering parse entry point — a lossless tree, in the shape
rust-analyzer's `rowan` popularized, that keeps producing a usable structure around a syntax error instead
of aborting — used only by the pieces below; `mwl check`/`mwl run` keep the existing strict, all-or-nothing
parse unchanged. `crates/mwl-lsp` (`tower-lsp`) ships its first, minimal slice: diagnostics (via `mwl
check` run against the resilient tree), hover (declared types), go-to-definition, and keyword/member
completion — no workspace-wide symbol search or code actions yet, that's M10. `editors/vscode` ships
alongside it: `.mwl` registration, a TextMate grammar, `language-configuration.json`, `mwl lsp` process
spawning, a `LanguageStatusItem` for server health, `mwl run`/`mwl test` as VS Code Tasks, and an AST
explorer panel backed by the CLI's existing `mwl ast` command (no new language feature needed for that
one). No formatting support yet (`mwl fmt` doesn't exist until M10) and no PhpStorm work — PhpStorm stays
entirely at M10, per [ADR 0016](adr/0016-ide-integration.md).

**Verify:** typing an incomplete statement (unclosed brace, trailing `->`) does not stop
diagnostics/hover/completion from working on the well-formed code around it — the resilient-parse mode's
core claim. The VS Code extension activates on `.mwl`, shows TextMate colour immediately and semantic-token
colour once `mwl-lsp` responds, and diagnostics/hover/go-to-definition/completion round-trip through it
with no logic duplicated into the extension. The AST panel renders `mwl ast --json`'s tree for the active
file.

### M5 — Concurrency and script isolates (~5 weeks)
Per-core runtimes, coroutine scheduler, `spawn` / `await` and `Core\Task\Channel` with backpressure,
cross-core worker dispatch with deep-copy-or-move, structured concurrency (a task tree dies with its parent
— no orphans), async-native file I/O, sockets, timers and HTTP client. The earlier `all`/`race`/`timeout`/
`parallel_map` verb list is now [ADR 0072](adr/0072-core-task-structured-concurrency.md)'s `Core\Task`
roster — `::all` over a shape literal of `fn` literals binding each field's own type, `::map` (what
`parallel_map` becomes, subject-first), `{limit, deadline}` as the one options shape in place of a
`timeout` wrapper, and `race` deferred with a named future spelling. That ADR also fixes what cancellation
does: no user code runs, native teardown does, and no call returns with a child still running. The isolate
this milestone builds is also what makes `mwl test` parallel: from here every `#[Test]` runs in its own
isolate sharing nothing but compiled code, the runner owns the test's task tree, and `#[Test(at:, seed:)]`
puts the clock and the generator under the test's control
([ADR 0079](adr/0079-testing-is-a-language-feature.md) §§ 2, 12, 16).
`serialize()`/`unserialize()` share this milestone's deep-copy-or-move graph walk, externalized to MWL's own
closed byte format ([ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md)); `unserialize()` refuses
anything not in that format, with no `__serialize`/`__unserialize`/`__sleep`/`__wakeup` hook. That same graph
walk — both as `serialize()` and as the `spawn worker`/`spawn script` value-crossing operation below — refuses
a `secret`-qualified value outright unless it was first passed through `Core\Secret::reveal()`
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)).

**Also in this milestone: `spawn script`** ([ADR 0006](adr/0006-isolated-script-execution.md)) — the
`Isolate` type in `mwl-host` with its own arena, `Core` accessor backing state and config overlay; the
request tree and its shared budget; `Core\Script::args()` ([ADR 0012](adr/0012-no-superglobals.md)) and the
top-level `return` contract; the `ScriptResult` shape; the value-crossing
rules shared with worker dispatch (graph copy, refusal of closures, references and resources, refusal of an
unresolvable class); `output: capture|inherit`; `on: worker`; cancellation of a child at its next safepoint.
It belongs here rather than later because it is a task with a heap boundary, which is exactly what this
milestone builds — and doing it now means the HTTP server in M7 is written against the same `Isolate`
instead of growing a second isolation path that has to be unified afterwards. Enforcement of its *limits*
and of the `script.spawn` capability lands with the rest of the config work in M6; until then it runs under
compiled-in defaults. The three spawn-construct routines built here also each gain a `spawn`-kind trace
event with a parent/child overhead split, once ADR 0018's `TRACE`/`PROFILE` bits exist alongside them
([ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)).

**Verify:** stress tests with 100k concurrent tasks; a deliberate deadlock test proves cancellation
works; `Core\Task::map` shows near-linear speedup across cores on a CPU-bound benchmark; ThreadSanitizer
clean. [ADR 0072](adr/0072-core-task-structured-concurrency.md)'s own *Verification* section is the one home
for what `Core\Task` owes, including the two that are easy to skip: a `Task::all` field holding a `callable`
variable rather than an `fn` literal is a compile error, and a cancelled task's `catch` and cleanup blocks
do not execute while its arena is still released. For isolates: a child cannot read or write a parent
variable, global, static, or output buffer, and
a `Core\Request`/`Core\Server`/`Core\Session` call inside it throws rather than seeing the parent's request
([ADR 0012](adr/0012-no-superglobals.md)); a closure, reference or resource is refused at the boundary; a
cyclic argument crosses without
hanging; a child's uncaught throw, its limit breach and a contained panic inside it all leave the parent
running with `ok = false`; a cancelled parent leaves no orphan and no leaked arena; spawn-to-result for a
trivial child on a warm cache is single-digit microseconds, committed to `benches/isolation.rs` next to the
process baseline it replaces, with a guard test alongside
`an_os_process_costs_orders_of_magnitude_more_than_a_task`. `serialize()`/`unserialize()` round-trip a cyclic
value using the same graph-copy fixtures as the isolate-boundary tests; bytes that are not MWL's own format,
or that name a class whose declared properties no longer match, are refused rather than partially accepted
([ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md)).

### M6 — Config, limits, capabilities, disk cache (~3 weeks)
Directive registry with changeability classes, boot config parsing (TOML via `serde`, with duplicate and
unknown keys refused — [ADR 0064](adr/0064-configuration-file-format.md); § 2a there is the block list and
names the ADR that argues each block's directives, including this milestone's new `[deferred]`,
`[[schedule]]`, `[http.*]`, `[metrics]` and `[trace]`), per-request overlay,
`Core\Config::set` semantics, capability enforcement at every syscall-touching stdlib entry point, safepoint-driven limit
enforcement, content-addressed artifact cache with integrity verification and a refusal to use a
world-writable cache directory — the exact file layout, header format, mmap-verify-then-execute read path
and probabilistic eviction sweep are already decided in [ADR 0042](adr/0042-on-disk-artifact-cache-format.md);
this milestone builds exactly what that ADR specifies, not a fresh design. Isolates get their governance here: the `script.spawn` capability with
canonicalise-then-prefix path resolution, `max_script_depth`, per-tree accounting of every `[limits]` value,
spawn-site sub-caps, and derivation of a child's overlay from its parent's effective config. **Also here:**
the `fatal_reserve_memory`/`fatal_reserve_time` directives and `Core\Fatal::onLimit` registration
([ADR 0020](adr/0020-error-escalation-ladder.md)) — the reserved slice a resource-limit `FATAL`'s handler
runs with is carved out of the request's own budget at the same point these limits are set up.

**Also here: `mwl build --compile`**, a CLI-only "one portable executable" bundler — appends an entry file's
statically-resolved `require` graph to the host `mwl` binary as plain source, read back through this same
artifact cache with no new mechanism. Scope, the source-not-precompiled-artifacts trade, and why bundling a
web-serving deployment is explicitly out of scope are all in
[ADR 0048](adr/0048-portable-single-file-executables.md), the only copy of the reasoning.

**Also here: the boot-time validation the four new blocks owe**, each ADR's own *Verification* section
being the one home for its list — a `[[schedule]]` entry with no `scope`, a malformed `cron`, a `script`
outside `script.spawn`'s roots or `scope = "fleet"` with no shared store all refuse to boot
([ADR 0073](adr/0073-scheduled-work-is-config.md)); `origins = ["*"]` with `credentials = true`, and
`same_site = "None"` with `secure = false`, are refused at boot and by `Core\Config::set` alike
([ADR 0074](adr/0074-http-defaults-safe-and-finite.md)).

**Also here: the config snapshot and the registry's reloadability field.** The registry becomes an immutable
`Arc<Config>` a request clones at start and reads for its whole life, and every directive gains a
`Reload`/`Boot` field beside its changeability class — orthogonal to it, and now the only thing that makes a
directive boot-only ([ADR 0078](adr/0078-config-reload-and-control-socket.md) §§ 1-2). `env_hash` lands here
too, carried by both compiled-unit cache keys, which is what stops an artifact compiled against one
extension set from ever being reused against another. No socket yet: the client that drives a reload needs a
long-running server, so it arrives with M7.

**Verify:** adversarial suite — a script attempting to widen a capability or set a `System` directive
fails; `Core\Config::set('memory', '512M')` above the `[limits]` default succeeds and takes effect, above the
`[limits.hard]` ceiling returns `false` with the previous value intact, and is invisible to the next request
on the same core; memory/CPU caps terminate runaway scripts as a `FATAL`, reported to `Core\Fatal::onLimit`
if registered and never to an ordinary `catch` ([ADR 0020](adr/0020-error-escalation-ladder.md)); warm-cache
CLI startup under 10 ms; a tampered cache artifact is rejected. For isolates: `spawn script` without `script.spawn`
fails; a path outside the granted roots fails, including one reaching it through `..` or a symlink; a child
cannot widen a capability its parent narrowed; N concurrent isolates cannot together exceed the tree's
memory, CPU or output budget; a recursive spawn is stopped by `max_script_depth` and reported as that rather
than as an out-of-memory. For the bundler: a bundled executable runs identically to `mwl run` against the
same source, on all three platforms, per ADR 0048's own verification list. For the config snapshot: a request
that started before a swap reads the old value to completion while one started after reads the new; a
malformed file leaves the previous snapshot serving and names the offending line; a changed `Boot` key is
reported in the result and does not take effect.

### M7 — Built-in HTTP server (~4 weeks)
`mwl serve`: hyper h1 + h2c, per-core accept and dispatch, request → the root isolate of a request tree
(the same `Isolate` M5 built, not a second isolation path), the `Core\Request`/`Core\Server` accessor
classes populated from it (`Core\Request::query()`/`::post()`/`::cookie()`/`::file()`,
`Core\Server::meta()`/`::header()` — replacing `$_GET`/`$_POST`/`$_SERVER`/`$_COOKIE`/`$_FILES`, see
[ADR 0012](adr/0012-no-superglobals.md)), returning `tainted string`/`tainted bytes` per
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md), multipart and urlencoded body parsing with
limits, streaming responses, static-file serving, graceful shutdown and zero-downtime reload, structured
request logging, optional TLS via `rustls`. **`Core\Response`'s body surface is the five typed members**
`html`/`json`/`text`/`bytes`/`sendFile` rather than a single `write`, each setting its own `Content-Type`,
with `echo` the HTML-only sixth path and mixing the two a compile error
([ADR 0088](adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 4) — which is also where a JSON
body stops being an `echo` the auto-escape sink would corrupt. The **`echo` binding table** (§ 3 of that
ADR) is enforced from here: the HTML sink is attached by a request and by nothing else, so a scheduled
script's and an isolate's `echo` take the terminal sink's neutralization instead.

**Also here: the control socket and `mwl ctl`** — a local unix socket (named pipe on Windows), created
`0600` and refused if its directory is world-writable, speaking HTTP so that a network listener would later
be a second `bind` rather than a second protocol, with `mwl ctl reload` as its only operation and no control
port in either direction of configuration ([ADR 0078](adr/0078-config-reload-and-control-socket.md) §§ 3, 6).

**Also here: `mwl service`**, which makes this binary installable under the platform's own service manager
rather than under a third-party shim. On Windows that is SCM registration with the hosted argv encoded into
a quoted, absolute `ImagePath`, a per-service virtual account, `STOP_PENDING` driven by the graceful drain
above and `PARAMCHANGE` driven into the reload beside it; on Linux it is a printed, hardened systemd unit
with `Type=notify` and an `ExecReload` pointing at that same socket, written to disk only on an explicit
`--install`. The installer is an [ADR 0088](adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
sink and fails closed — a closed `serve`/`run` subcommand allowlist, no relative path, no install whose
output would go nowhere, no password on a command line, and a refusal to install from an
[ADR 0048](adr/0048-portable-single-file-executables.md) bundle. Scope, every refusal, the argv-encoding
rule and the verification list are
[ADR 0093](adr/0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md), the only copy.

**Four subsystems land on top of that server, each with its own ADR holding the only copy of its rules.**
The **response policy** — secure headers, closed CORS and `Secure; HttpOnly; SameSite=Lax` cookies applying
with nothing configured, every directive `Runtime` so a request may change it for itself
([ADR 0074](adr/0074-http-defaults-safe-and-finite.md) §§ 1–4). The **`[[schedule]]` ticker**, firing each
entry as a **root** isolate through the same `Isolate` M5 built, with the fleet lease over the shared store
([ADR 0073](adr/0073-scheduled-work-is-config.md)). **`Core\Task::afterResponse`**, whose tree stays alive
past the connection and is bounded by `[deferred] max_concurrent`
([ADR 0072](adr/0072-core-task-structured-concurrency.md) §§ 6–7). And the **observability export** —
`Core\Metrics`, the default series, W3C `traceparent` inbound, and spans derived from
[ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)'s existing event kinds with no probe
added to ADR 0018's measured path ([ADR 0076](adr/0076-observability-export.md)). **`Core\Router::match`**
lands here too, over the table M4S compiled. **Not yet named here:** raw/unparsed body access (a JSON
payload, a webhook body, an arbitrary content-type) — a gap [ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md)'s *Revisiting* flags for whoever designs `Core\Request`'s full surface at this milestone.

**Also in this milestone: hot-reload of the compiled-unit cache**, which is what makes "no restart to see an
edit" true of a running server rather than only of `mwl run`. [ADR 0017](adr/0017-hot-reload-without-restart.md)
holds the only copy of the mechanism — a per-path pointer over the content-addressed cache M5/M6 already
built, revalidated lazily and rate-capped, swapped without ever blocking a request-serving core. Every
request stays as isolated as a fresh subprocess regardless: compiled code is the only thing this milestone
ever lets one request share with another, and that sharing is exactly what M5's `Isolate` and M6's
`[limits]`/`[limits.hard]` already bound per request tree, not per file.

**Verify:** the core requirement demonstrated under load — 10k concurrent cold requests for the same file
compile it **exactly once** (assert via a compile counter) with no stalled requests; a state-bleed test
suite proves nothing leaks between requests, and the same suite runs across an isolate boundary, which the
shared `Isolate` makes a parameterisation rather than a second suite; a request whose isolates are still
running when the client disconnects leaves none of them behind; path traversal, header injection and
request-smuggling suites pass; `wrk`/`oha` throughput compared against PHP 8.5 + FPM + opcache and recorded
in `benches/`. For hot-reload specifically ([ADR 0017](adr/0017-hot-reload-without-restart.md)): editing a
file under concurrent load recompiles it exactly once no matter how many in-flight requests race to notice;
a request that resolved the old version runs it to completion while a newer version is already being served
to new requests; a revalidation that fails to compile fails only requests resolving it afterwards; a `stat`
storm against one hot, `mtime`-validated file is bounded by `revalidate_freq`, not by request rate.

**Also here: persistent connections** ([ADR 0083](adr/0083-persistent-connections-are-isolates.md)).
`Core\Socket::upgrade` and `Core\Sse::upgrade` reuse [ADR 0006](adr/0006-isolated-script-execution.md)'s
`with(...)` clause whole and hand the socket to a **root isolate** — the same `Isolate` M5 built, so this
milestone adds a lifetime, not an isolation path. `Core\Topic` is the cross-core publish/subscribe bus, with
a bounded per-subscriber queue that closes a slow subscriber rather than blocking a publisher. That ADR's
*Verification* is the fixture list; the state-bleed suite above gains connections as a third
parameterisation rather than a second suite.

### M8 — Stdlib and databases (~16 weeks)
**The roster this milestone builds is [ADR 0051](adr/0051-standard-library-tiers.md) § 3** — which class is
Core, which is a capability-gated native subsystem, which is an extension, and which of PHP's extensions
has no equivalent at all. That ADR is the one home for the list; this paragraph covers only what M8 must
decide beyond it. The capability-bearing half of testing lands with the capabilities it needs:
`#[Test(db:)]`'s rolled-back transaction, `Core\Test::request`'s in-process dispatch through the compiled
route table, `#[Test(server: true)]`'s ephemeral listener, and inline snapshots with their source updater
([ADR 0079](adr/0079-testing-is-a-language-feature.md) §§ 14, 17, 18).

**`Core\Cli` lands here in full** ([ADR 0086](adr/0086-core-cli-terminal-is-a-sink.md)): the terminal
output sink and its visible-substitution table, the `Cli\Text`/`Style`/`Color` value types, the once-per-
process tty/colour-depth/width resolution over `anstream`, the five prompts reading the controlling
terminal rather than stdin, the scoped `live`/`progress` regions, and `Core\Command::run` plus its
generated `--help` and shell completions over M4S's table. Two dependencies arrive with it — `crossterm`
for raw mode, key events and resize, `unicode-width` for UAX #11 columns — both pure Rust, both owing a
notice regeneration. The [ADR 0020](adr/0020-error-escalation-ladder.md) § 4 terminal-restoration
obligation is part of this slice, not a follow-up: a live region that survives a panic is the defect the
whole scoped shape exists to prevent.

Regex is two-tier, and the tiering is a rule rather than an implementation detail: a linear-time engine by
default, backtracking only for patterns it cannot express and only under a throwing step budget, with a
literal pattern's tier settled at compile time and the *pattern* argument refusing `tainted`
([ADR 0056](adr/0056-regex-engine-policy.md)). The compile-time half of that rides on
[ADR 0057](adr/0057-intrinsic-literal-folding.md)'s closed intrinsic list, which also lands here and
covers `Core\Uri`, the date-format strings and `Core\Str::format`'s placeholder checking.
`Core\Decimal`/`Core\BigInt`/`Core\BigDecimal` supply the method surface around the `decimal` scalar M2–M4
already built ([ADR 0054](adr/0054-decimal-scalar-type.md)) — including `divExact`, `divRound` and
`allocate`, since division is the one place a decimal result may be inexact. `Core\Cache`'s two tiers, the
copy-in/copy-out rule and the per-core memory cap are [ADR 0059](adr/0059-cross-request-state-is-explicit.md);
`Core\Http\Client`'s `tainted`-refusing URL parameter, the `Core\Http::allowUrl` launderer and the
`net.connect` address policy are [ADR 0058](adr/0058-outbound-request-policy.md), and its finiteness half —
no spelling for an unbounded wait, jittered opt-in retry under one covering deadline, and a `post` retried
without an `idempotencyKey` being a compile error — is
[ADR 0074](adr/0074-http-defaults-safe-and-finite.md) §§ 5–7; the closed
signed-cookie/CSRF/TOTP/JWT roster and its correct-by-construction constraints are
[ADR 0060](adr/0060-application-security-protocols.md). **`Core\RateLimit`** lands here too — GCRA over the
shared store as `consume`, per-core and approximate as `shed`, no configuration at all, and an unreachable
store throwing rather than deciding *allowed*
([ADR 0075](adr/0075-core-ratelimit.md)); the shared tier's atomic script is ours, since no distributed
rate limiter exists as a crate. `Core\Http\Client` also gains outbound `traceparent` propagation
([ADR 0076](adr/0076-observability-export.md) § 2), which is the point at which a trace crosses a service
boundary at all.

Also: JSON; hashing and crypto (RustCrypto: sha2, blake3, argon2,
bcrypt, aes-gcm), AEAD-only per ADR 0051 § 3; date/time with PHP-compatible formatting; filesystem and
stream abstractions — with no scheme dispatch anywhere in them
([ADR 0052](adr/0052-closed-doors.md) § 2); process
execution behind the `process.exec` capability gate — `Core\Process::run()`/`::spawn()`, argv-only with no
shell-string form at all, a Windows batch/PowerShell-target refusal, and coroutine-suspending waits, per
[ADR 0044](adr/0044-core-process-argv-only-no-shell.md) (which supersedes ADR 0024 §4's original
placeholder bullet); sessions, which may not be backed by `Core\Cache`'s local tier; and
`Core\Html::escape`/`Markup` plus the `Core\Taint` launderers, per
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md).

**The database half of this milestone is [ADR 0067](adr/0067-core-db.md)**, with its signatures in
[docs/spec/01-core-library.md](spec/01-core-library.md) § 18: one API replacing `PDO`, `mysqli`, `pgsql` and
`sqlite3`, over pure-Rust MySQL, MariaDB, PostgreSQL and MS SQL Server drivers plus SQLite (whose C
dependency is [ADR 0051 § 4](adr/0051-standard-library-tiers.md)'s audited exception). Connections are named
in root-owned config under a new `db.connect`/`db.open` capability pair, every statement is prepared, and a
transaction is a closure. Two things it needs from elsewhere in this milestone: `Core\Time`'s types for the
date columns, and `Core\Json` for the JSON ones, which are not auto-decoded. Also here: **`Core\Reflect` and `Core\Ast`**, built-in — not
extension-provided — structural reflection and a runtime door onto `mwl-syntax`'s own lexer/parser, per
[ADR 0019](adr/0019-reflection-and-ast-parsing-are-core-features.md); reflective access enforces the same
visibility/hook checks ordinary code does, and a parsed AST is typed, inert data with no path back into
execution. Also here: **`Core\Attributes`**, the narrow, statically-resolved `get<T>`/`all<T>` accessor onto
attribute literals attached in M4 — deliberately not part of `Core\Reflect`'s general-purpose walk, per
[ADR 0046](adr/0046-attributes-shape-literal-metadata.md). Also here: **`Core\Fatal` and `Core\Log`**, plus the operator-configured `.mwl` error-handler
script and the engine-native logging floor beneath it, per
[ADR 0020](adr/0020-error-escalation-ladder.md); `Core\Log`'s JSON-Lines writer is the same native
serialiser the engine floor calls directly, so the two never disagree on log shape. `mwl check` refuses a
`secret`-qualified operand at a `Core\Log::write()` call site's `fields` argument despite that parameter's
open `array<string, mixed>` type, and `Core\Secret::reveal()` plus the password-hashing helpers from this
milestone's crypto line item above are the two ways a value legitimately loses `secret` before reaching it
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)).

**Also in this milestone: author the `mwl:ext@1.0.0` WIT world.** It must be designed from the same
value-access model as the `Core` domain classes' static methods, so the Tier 0 internal interface and the Tier 1 guest
interface are one design rather than two that drift. Writing it later would mean retrofitting. **It must
also carry a qualifier axis** — a parameter that refuses `tainted`, and a return that is always `tainted` —
per [ADR 0055](adr/0055-extension-qualifier-declarations.md), which M9 then freezes; adding it afterwards
would be a breaking change to a published ABI, and without it any `.mwlx` launders untrusted data merely by
being called. The same
applies to the signatures themselves: the parametric array signatures
([ADR 0007](adr/0007-explicit-type-system.md) — `array_map(callable, array<T>): array<U>` and friends) are
written once for the built-ins and reused by the WIT world, where `uint` now maps to `u64` with no
conversion. Type variables stay available only to declarations the compiler owns; user-defined generics are
not part of this milestone.

**Also in this milestone: the framework's privileged half** ([ADR 0082](adr/0082-the-first-party-framework.md)
§ 2), each entry placed by [ADR 0051](adr/0051-standard-library-tiers.md)'s own six tests rather than by a
new rule — `Core\Validate` (the launderer, and the one entry that could never be a package),
`Core\Session`, `Core\Password`, `Core\Mail` transport against an operator-named SMTP endpoint,
`Core\Storage` over local disk, and `Core\Cldr::pluralCategory`. **`Core\Queue` lands here too**
([ADR 0084](adr/0084-durable-background-jobs.md)): the jobs and dead-letter tables, `mwl queue migrate`,
per-backend `SKIP LOCKED`-shaped claiming, the visibility timeout, bounded retries with jittered backoff,
and the transactional-enqueue property that is the reason for the whole design. **And the connection pool**
([ADR 0067](adr/0067-core-db.md) § 13) — per core, keyed as `connect`/`open` already key, with a per-backend
reset that is a security boundary: PostgreSQL's targeted reset that preserves the statement cache, MySQL's
and SQL Server's protocol resets that do not, and a failed reset destroying the connection rather than
returning it.

**Verify:** ADRs 0051 and 0054–0060 each carry their own M8 verification list — that is the one home for
them, and this paragraph does not restate it. Two are worth naming here because they are CI infrastructure
rather than fixtures: a check enumerating the default binary's C dependencies, failing on any addition not
recorded against [ADR 0051](adr/0051-standard-library-tiers.md) § 4's two questions, and a check that no
class outside Tier 0 registers a name beginning `Core\`. Beyond that: per-subsystem conformance suites, and
for the database half, the five-driver CI-container matrix [ADR 0067](adr/0067-core-db.md)'s own
*Verification* section specifies. `Core\Reflect`/`Core\Ast`
verified per [ADR 0019](adr/0019-reflection-and-ast-parsing-are-core-features.md)'s own M8 verification
list — a reflective call to a `private` method from outside its class fails like the equivalent ordinary
call; `Core\Ast::parse()` fuzzed with the same corpus as M1's lexer/parser target. `Core\Attributes` verified per
[ADR 0046](adr/0046-attributes-shape-literal-metadata.md) — `get<T>` on a site with zero matching attributes
compiles to a constant `null`, one match compiles to that constant value with no runtime lookup, and more
than one match is a compile-time diagnostic naming `all<T>`; a literal property/parameter name that names no
real member is a compile-time diagnostic, and a non-literal one is a runtime empty result instead.
`Core\Fatal`/`Core\Log`
verified per [ADR 0020](adr/0020-error-escalation-ladder.md)'s own M7/M8 list — `onUncaughtThrow` receives
the real `Throwable`; the configured handler script runs charged to the engine's own reserve and still
fires when the reporting request is at its own memory ceiling; application code and the engine floor
produce schema-identical log records for the same error. `Core\Process` verified per
[ADR 0044](adr/0044-core-process-argv-only-no-shell.md)'s own M8 list — a tainted `$path`/`$argv` element is
a compile-time diagnostic, a Windows batch/PowerShell target is refused, the `process.exec` capability is
deny-by-default, and a concurrent-spawn scheduler guard sits alongside `benches/abi-probe`'s existing
coroutine-suspension tests.

### M9 — Extension system (~6 weeks)
`mwl-ext`: `.mwlx` loading (wasm component + `mwl.manifest` custom section), manifest parsing and
registration into the compiler symbol table so extension calls are statically type-checked — including the
`tainted`/`secret` qualifier axis, applied at an extension call site by the same code path as a `Core` one,
with a manifest attempting the laundering form ADR 0055 § 3 says does not exist failing validation at load
time — the WIT host
implementation, per-call handle tables for value access, lazy per-request instantiation on the pooling
allocator, epoch-interruption wiring to the per-request CPU cap, `StoreLimits` wiring to the memory cap,
the capability bridge (no ambient authority; optional WASI world with preopens derived from `mwl.toml`
grants), hash pinning and signature verification, and compiled-module caching in the existing
content-addressed artifact cache. The set is **reloadable rather than boot-only**: `mwl ctl reload`
re-verifies every pin, refuses the swap whole if one does not match, and rides the `env_hash` M6 put in both
cache keys, so a changed set recompiles lazily through ADR 0017's existing machinery with no invalidation
pass ([ADR 0078](adr/0078-config-reload-and-control-socket.md) § 4). Duplicate class names across extensions
are refused at load, which is what makes that hash order-independent.

Tooling: `mwl ext new --lang rust|c|zig|go`, `mwl ext build` (one portable `.mwlx`), `mwl ext inspect`
(manifest and requested capabilities), `mwl ext test`, `mwl ext verify`.

**Verify:** the two first-party extensions [ADR 0051](adr/0051-standard-library-tiers.md) § 3 places at
Tier 1, end to end. The **image codec** is the one that makes the security claim legible — decoding an
attacker-supplied file in a sandbox with a memory cap and an epoch deadline — and it is built from Rust
*and* from a second language to prove the toolchain claim, running unmodified on all three platforms from
one binary. The **intl component** proves the two shapes that ADR's Ext placement depends on: CLDR data
carried in the component's own wasm data section, and a batch-shaped API where sorting 10,000 strings costs
one boundary crossing rather than one per comparison. Adversarial suite: an extension attempting filesystem or network access it was not granted fails;
a runaway extension is trapped by the request's CPU cap rather than hanging a core; a deliberately
memory-hungry extension hits the cap; an extension that stores state in a global cannot observe it on the
next request. Reload, per [ADR 0078](adr/0078-config-reload-and-control-socket.md)'s own list: an artifact
compiled under one extension set is never reused under another; an extension added by a reload is callable
from requests arriving after it with no restart; a reload whose pin does not match the file on disk is
refused whole, leaving the previous set live. Benchmark in-guest compute throughput against the equivalent native Tier 2 implementation and
**commit the numbers** — this is the one figure in ADR 0003 that is currently asserted rather than
measured.

### M10 — Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined)
`mwl fmt` (canonical, idempotent — the **only** formatting implementation; neither editor client below gets
its own; its PER-based, unconfigurable, no-reflow style and `--check`/`--diff` surface are
[ADR 0039](adr/0039-canonical-code-formatting.md)); `mwl-lsp` grows past M4B's minimal slice into full
workspace-wide symbol search, incremental reparse, rename, and code actions; `mwl dap` using safepoints for
breakpoints plus deopt-to-debug in codegen; a sampling profiler, emitting output in the open speedscope
format so it opens in existing viewers rather than a bespoke flamegraph renderer
([ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md)); `mwl pkg` with lockfile, semver
resolution and a registry. Also here: `Core\Debug`, the `[debug]` `mwl.toml` section and
`debug.trace`/`debug.profile` capabilities, and the Clover/lcov/Callgrind exporters wired to `mwl test
--coverage=…` and `mwl run --profile=…` — the developer-facing coverage/tracing/profiling feature whose
probe mechanism landed with M3
([ADR 0018](adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md), a deterministic per-call
profiler distinct from the sampling one above), plus a speedscope-evented export rendering that same
trace data — now tagged `call`/`gc`/`spawn` — as one scrollable timeline
([ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)). The same probe sites, in their
**counting** mode, are what `mwl test --bench` reports: statements, calls, allocations, bytes and GC cycles,
bit-identical across machines and OSes and therefore gateable in CI, with wall-clock printed as advisory
only ([ADR 0079](adr/0079-testing-is-a-language-feature.md) § 15). `mwl test --mutate` lands here too,
generating type-valid mutants from typed HIR and running each against only the tests the coverage data says
touch the mutated line (§ 21).

**Also in this milestone: the rest of the two editor clients.** M4B already shipped `mwl-lsp`'s minimal
slice and `editors/vscode`'s baseline; what lands here per
[ADR 0016](adr/0016-ide-integration.md)/[ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md) is
the deep half:

- **`editors/vscode`** (extending the M4B package, not a second one) — format-on-save and format commands
  wired to `mwl fmt`; inspections and quick fixes for every ADR-named diagnostic that has an obvious fix
  (casing, legacy casts, missing property init, `include`→`require`, `tainted`/`secret` laundering);
  workspace-wide rename, extract-to-method/variable, alias-free organize-imports; signature help and
  cross-file completion; inlay hints; a native Test Explorer wired to `mwl test`/`.mwlt` with coverage via
  VS Code's own `FileCoverage` API (no custom gutter UI); a "View Profile" command opening the
  speedscope-format sampling output; and — reversing ADR 0016 § 4 for VS Code specifically — a
  `DebugAdapterDescriptorFactory` and `launch.json` schema wiring `mwl dap` into VS Code's existing debugger
  UI. Full rationale and per-feature dependencies: [ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md).
- **`editors/phpstorm`** — unchanged from [ADR 0016](adr/0016-ide-integration.md): a Kotlin/Gradle plugin
  that registers `.mwl` as its own file type (distinct from PhpStorm's bundled PHP support, which must not
  claim it), bridges to the **same** `mwl lsp`/`mwl fmt` binaries through JetBrains' LSP client support (or
  LSP4IJ, per ADR 0016 *Revisiting*), and ships an equivalent TextMate-or-equivalent baseline grammar.
  PSI-level refactoring, structural search, a native Formatter/Code Style page, a Test Explorer, and
  debugger UI wiring are all still out of scope for PhpStorm — [ADR 0016](adr/0016-ide-integration.md)
  names the native-plugin path as a later decision, not a silent gap, and [ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md)
  does not touch PhpStorm at all.

**Verify:** `mwl fmt` is idempotent across the whole corpus, and neither editor extension contains its own
formatting logic. The VS Code extension's inspections/refactorings/rename round-trip as LSP code actions
and requests with no logic duplicated locally; format-on-save matches `mwl fmt --check` byte-for-byte; the
Test Explorer runs `.mwlt` cases and shows coverage sourced from the Clover/lcov exporters; a captured
profile opens correctly in a speedscope-compatible viewer; a breakpoint set in VS Code's UI hits in
JIT-compiled code with correct variable values through the wired-up `mwl dap` adapter, with no
MWL-authored debugger UI code. The PhpStorm plugin registers `.mwl` as its own file type (opening one does
not invoke PhpStorm's bundled PHP support) and gets the same completion/hover/diagnostics/rename/formatting
round trip through the identical `mwl lsp`/`mwl fmt` binaries as VS Code — evidenced by both editors
agreeing byte-for-byte on the same file's formatted output and diagnostics — with breakpoints verified via
`mwl dap` directly, since PhpStorm's debugger UI is still not expected to exist yet.

### M11 — PHP transpiler (~10 weeks)
`mwl convert`: PHP source → AST → rewrite passes → `.mwl` output, under the contract
[ADR 0089](adr/0089-convert-is-one-rule-table-with-two-modes.md) fixes — one rule table read through two
modes, an equivalence claim that a differential case against the PHP oracle discharges, byte-for-byte
determinism, nothing dropped, and `php-rs-parser` as the pinned front end behind that ADR's § 7 facade,
covering PHP 7.4 through 8.6.
**This milestone is now on the critical path for adoption rather than a convenience**, because PHP has no syntax for a `foreach` binding's
or a destructuring target's type and [ADR 0007](adr/0007-explicit-type-system.md) requires one for both: the
converter carries the type-inference engine MWL's compiler deliberately does not have for those two
positions, and writes the annotations into the output for a human to review. A plain local is now
mechanical instead — [ADR 0037](adr/0037-var-local-type-inference.md)'s `var $name = expr;` lets the
converter emit the initializer unchanged and let the compiler's own checker fix its type, no inference pass
needed. Where the remaining inference cannot decide, it emits `mixed` with a `TODO` naming the binding
rather than guessing — an honest `mixed` runs, and a wrong annotation would not. That inference pass is the
reason for the two extra weeks over the original estimate.

**The catalogue of individual rewrites is not restated here**, because it now has one home:
[ADR 0089](adr/0089-convert-is-one-rule-table-with-two-modes.md)'s rule table. Every construct MWL removed
keeps its destination in the ADR that removed it — each of those carries its own *M11* verification entry
naming what the converter owes — and each becomes one row with a tier: proven identical against the PHP
oracle, a mechanical destination that may differ, or no mechanical destination at all. The name half of the
table is generated from [docs/spec/02-php-migration.md](spec/02-php-migration.md), written and CI-checked
long before this milestone, so a name PHP has and MWL dropped produces that row's stated reason rather than
an unresolved call ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)).

`--check` mode emits a migration report without writing files. A `.phpt → .mwlt` converter
reuses the same pipeline to import PHP's test corpus as native MWL tests. A PHP project depending on a C
extension is reported as needing either a Tier 1 `.mwlx` replacement or a Tier 2 native one — the converter
cannot synthesise either, and says so rather than emitting code that fails at runtime.

**Verify:** convert a real open-source PHP project end to end and run its test suite under MWL, reporting
*annotations written* against *`TODO`s emitted*; imported `.phpt` cases run in `mwl test` with a tracked
pass rate and failures triaged as bug vs intentional divergence — the nine type-discipline divergences in
[ADR 0007](adr/0007-explicit-type-system.md) §7 are counted separately, or the structural gap reads as
regression.

### M12 — Optimising JIT tier (ongoing)
Profiling counters, inline caches for property and method access (monomorphic → polymorphic →
megamorphic), unboxed int/uint/float fast paths, inlining, refcount elision, escape analysis, deopt and OSR
at existing safepoints. Narrower than originally scoped: the declared types of
[ADR 0007](adr/0007-explicit-type-system.md) mean the baseline tier is already typed, so speculation only
has to cover `mixed`, unions and dynamic calls.

**Verify:** macro benchmarks show a multiple over the baseline tier and over PHP 8.5 with JIT — measured per
[ADR 0026](adr/0026-performance-measurement-methodology.md)'s same-host PHP-oracle ratio, not a raw
cross-machine wall-clock claim; no correctness regressions in the full conformance suite when the
optimising tier is forced on.

### M13 — Optional FastCGI transport
Only if a deployment target requires it (shared hosting, IIS, an existing nginx estate). Implements the
same `Transport` trait as `mwl-http`, with a fuzzed record parser and `SCRIPT_FILENAME` handling that
resolves within a configured root — closing the historical vulnerability class by construction.

### M14 — Optional wasm32 browser target
Only if an embedding wants MWL running client-side in a browser tab, per
[ADR 0025](adr/0025-wasm-browser-target.md). A second codegen backend consuming the same M2 IR —
instruction selection to wasm32 opcodes via a pure-Rust emitter, not Cranelift, which has no wasm32
output. `spawn worker`/`spawn script`, coroutine-based suspension, and `.mwlx` extension loading are all
unavailable in this target (a diagnostic naming the ADR, never a silent no-op); every other language and
stdlib feature is unchanged. Adds `Core\Browser` as a seventh [ADR 0012](adr/0012-no-superglobals.md)
accessor domain (method table undesigned until this milestone starts), and requires `require` to resolve
every path at build time — no dynamic fallback, since there is no filesystem at runtime.

**Verify:** a `.mwl` file using none of the three excluded features compiles to a `.wasm` module and runs
identically to the native target's output on the same input, in a headless-browser test harness; a `.mwl`
file using `spawn`, suspension-requiring `Core` I/O, or a `.mwlx` extension for this target produces the
ADR-named diagnostic rather than a miscompile or a silent downgrade; `benches/abi-probe` gains a
browser-target guard for whatever cost claim this milestone's spike validates.

### M15 — Packages, the registry and the supply chain (~8 weeks; scheduled after M6)

**Not optional and not last** — [ADR 0080](adr/0080-the-audience-mwl-is-built-for.md) § 5 ranks this and M16
above new `Core` breadth, because they are the two things a new user meets before any language feature. The
number is an identity, not a position: it runs after **M6**, which is the first point at which capabilities
exist to be granted per package.

[ADR 0081](adr/0081-packages-are-digests-resolution-is-a-maximum.md) is the whole design. The client half:
`package.toml`/`package.lock`, `mwl add`/`fetch`/`update`/`outdated`/`vendor`/`audit`/`publish`, minimal
version selection, the root-only git source, digest verification, transparency-log inclusion and consistency
checks, and the generated `vendor/packages.mwl` that reaches a fetched package through an ordinary
[ADR 0061](adr/0061-compile-time-autoload-and-program-discovery.md) `autoload` declaration — the piece that
keeps this out of the language. The compiler half is smaller than it looks: a file → package map, and a
capability check at every `Core` call site against the package's grants. **There is no name-resolution work
at all**, which is the design's own assertion and the first thing to verify.

The registry half is a service rather than a milestone deliverable: a static index, an artifact store, an
append-only Merkle log with signed checkpoints, an advisory feed, accounts with a second factor. It is
specified here and operated outside the repository.

**Verify:** [ADR 0081](adr/0081-packages-are-digests-resolution-is-a-maximum.md)'s *Verification* is the
list. The two that matter most: a package calling a `Core` member without a matching grant line fails to
compile naming the package, the capability and the fix; and a package containing a top-level statement with
an observable effect produces no effect from `fetch`, `build` or `vendor`.

### M16 — `mwl/web`, `mwl new`, and the framework (~12 weeks; scheduled after M7 and M8)

[ADR 0082](adr/0082-the-first-party-framework.md) is the split and the roster. The privileged half lands in
M8 with the rest of `Core`; **this milestone is the package** — `Web\Controller` and the middleware pipeline
over [ADR 0077](adr/0077-compile-time-routing.md)'s table, `Web\Response`, `Web\Auth`, `Web\Validation`,
`Web\Mail`, `Web\I18n`, `Web\Storage`, `Web\Pagination`, `Web\Job`, `Web\Api` — plus `mwl new`. Wiring is
constructor injection resolved through
[ADR 0061](adr/0061-compile-time-autoload-and-program-discovery.md) § 3's enumeration, so a missing binding
is a compile error and there is no runtime container. There is no ORM and no template engine, for the
reasons that ADR's § 4 gives.

**`Web\Migration` is blocked, deliberately.** [ADR 0082](adr/0082-the-first-party-framework.md) § 7 records
the open gap — ordering, transactional DDL, fleet locking, reversibility, safety against a live
multi-tenant database — and nothing in the package may ship a migration runner until an ADR closes it. That
ADR is the prerequisite for this milestone finishing, not a follow-up to it.

**Verify:** `mwl new` compiles, serves its routes, authenticates a session, reads a row and passes its
`#[Test]`, on all three platforms, as a first-class CI job. Its `package.lock` names `mwl/web` and nothing
else. Deleting the scaffold's validation call makes it fail to compile — the assertion that the qualifier
demonstration is real. And a fixture in which `mwl/web` tries to return an unqualified `string` derived from
a `tainted` one fails to compile, proving the framework is subject to
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md) § 3 like any other package.

---

## Overall verification strategy

- **Unit** — `cargo test` per crate; `insta` snapshots for AST/IR/codegen.
- **Conformance** — hand-written `.mwlt` suite as the normative definition of MWL; imported `.phpt`
  corpus tracked as a compatibility percentage.
- **Property/fuzz** — `proptest` for the array and string implementations; `cargo-fuzz` on lexer, parser,
  HTTP parser, multipart, regex and JSON, run continuously in CI.
- **Sanitisers** — Miri on the safe subset, ASAN/TSAN on the unsafe core and the scheduler.
- **Security** — the adversarial suites from M6/M7 (capability escape, resource exhaustion, cross-request
  *and* cross-isolate state bleed, execution-root escape, traversal, smuggling) plus `cargo deny` advisories
  on every build.
- **Performance** — `criterion` microbenchmarks and application-level macro benchmarks, always compared
  against the locally installed PHP 8.5.8, with results committed so regressions are visible in diffs.
