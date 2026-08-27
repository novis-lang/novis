# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-27. Current: **M4**, language completeness. M3 is done, spec §§ 1-12 is
> registered whole, the previous loop's Stage 0 catch-up list is closed, and **M4S Part I stands at
> 552 of its 600 conformance cases with both named guards green** — it is now the floor and the
> corpus gate of a new goal rather than the frontier. **The loop is aimed at M4**: every shape that
> compiles in the front end and then refuses below it, closed, before M4B's LSP is written against
> the surface. [docs/agent/loop-goal.md](agent/loop-goal.md) holds the 43 items, grouped by file
> set, and `python tools/holes.py` is the live worklist behind them — 45 refusal sites in `mwl-ir`
> and `mwl-codegen`, attributed to the item that names their function.
> `crates/mwl-stdlib/tests/spec-members-outstanding.txt` holds no keys: the ratchet
> `spec_registry_coverage.rs` reads is empty, which is this project's own definition of Part I being
> *registered*, and all seven Stage 3 fixtures produce their frozen output. **Every representation
> the library needed is built and recorded in the crate that owns it** — `mixed`/`?T`/every union is
> `mwl_ir::Ty::Tagged`, strict identity is `mwl_runtime::identity`, `decimal` is
> `mwl_runtime::decimal`, a `Core`-owned instance is an ordinary MWL object
> (`mwl_stdlib::instance`), a variadic tail is one `array<T>` argument, and a sink's carrier is a
> `Core` instance whose one slot `mwl_runtime::value_to_string` renders (ADR 0088 § 5). **ADR 0087's
> lexer half is built** — `mwl_syntax::bidi` is the one predicate, `E0008` at the lexer, and a
> substitution at both sinks at M7/M8. **ADR 0090 is built whole**, §§ 1, 2, 3 and 5 alike, and
> **ADR 0069's `+`/`+=` refusal** is `E0467` naming `Core\Arr::underlay`. **ADR 0071's first
> compiler-recognized attribute is built** — `#[Json\Derive]`, matched nominally in
> `mwl_types::derive`, decoder included for a scalar-fielded class — and `ParseError` carries § 10's
> `issues`. **ADR 0070's duration literal lexes, types and runs.** What M4 still owes is stated as
> items rather than prose: the operator table (promotion, integer `/`, overflow, the bitwise rows,
> `**`, `<=>`, `++`/`--`), control flow (`do`/`while`, `break 2`, a two-representation ternary,
> `finally` through a throwing `catch`, an abandoned generator's `finally`), calls (a closure
> through the variable holding it, named and spread arguments, `&$x` anywhere), targets (a static
> property, a nested element write), `mixed` dispatch, ADR 0007 § 2's last conversion rows, and the
> five declared features with no slice at all — `PropertyObserver`, ADR 0043's `by`-delegation, ADR
> 0046's retrieval, ADR 0092's `Core\Debug::dump`, ADR 0079's `#[Test]`. Dependencies: `regex` +
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
> **On disk:** the workspace, CI on three platforms with miri, asan and fuzz legs beside it,
> lint/deny/fmt/notice policy, `mwl-diagnostics`, `mwl-syntax` (+ `duration`, ADR 0070's one
> grammar, and `bidi`, ADR 0087's one predicate), `mwl-hir`, `mwl-types` (+ `layout`, `core_lib`,
> `error_lib`, `iter_lib`, `generics`, `conformance`, `defaults`, `derive`), `mwl-ir`, `mwl-runtime`
> (+ `alloc`, MWL's own `#[global_allocator]` in an optimized build and the `sanitizer` feature that
> takes it back out from under ASAN, `object`, `array`, `throwable`, `closure`, `identity`,
> `decimal`, and `ctx`'s capture stack and carrier roster), `mwl-stdlib` (`Arr` × 36, `Str` × 39,
> `Math` × 38, `format`, `granularity`, `ordering`, `cldr`, `instance`, `issue`, `Order`,
> `NormalForm`, `RoundMode`, `Unit` and `Weekday`, `Regex` × 6 plus `Regex\Match` × 4 over
> `regex`/`fancy-regex`, `Time` × 7 plus `Time\Instant` × 9, `Time\DateTime` × 14, `Time\Duration` ×
> 19 and `Time\Zone` × 4 (+ `UTC`) over `jiff`, `Json` × 4 over `serde_json`, `Path` × 9 (+
> `SEPARATOR`) over nothing at all, `Random` × 6 over `rand`, `Uuid` × 4 (+ `toString`) over `uuid`,
> `Uri` × 4 over nothing at all, `Encoding` × 2 over nothing at all, `Hash` × 4 plus `Hash\Stream` ×
> 2 over the RustCrypto family, `crc32fast` and `subtle`, `Csv` × 2 over `csv-core`, `Out` × 1
> answering the `Cli\Text` carrier, `ObjectMap` × 9, `ObjectSet` × 9 and `Heap` × 5 over
> `identity_store`, all three iterable through `cursor`, and the conformance-coverage gate),
> `mwl-codegen`, `mwl-cli` (`ast`, `check`, `run`, `test`, `info`), `mwl-test` (+ `case`, `expect`,
> `run`), `tests/conformance` × 558 (in `array`, `class`, `core`, `enum`, `error`, `iter`, `lang`
> and `reject`) and `tests/differential` × 162, `fuzz/`, `tools/`, `benches/abi-probe`.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** checker-side rules for ADRs 0007, 0010, 0013, 0014, 0015, 0021, 0022, 0024,
> 0027, 0028, 0029/0030, 0033, 0036, 0037, 0038, 0054, 0062, and 0043's syntax +
> default/private-method slice; end-to-end for 0007 §§ 2 and 4's `%` row, 0010, 0013, 0014 § 1, 0023
> § 1, 0035 § 4, 0031 §§ 1-2, 0065, 0036 § 2's anonymous literal in every position it writes and
> § 4 **whole** — the shape-field read and write, the plain-`object` receiver's read and write, and
> the missing-name throw all name-keyed and all right through a widened view, with every named
> class's slots carrying the declared tag that write is checked
> against, **0029/0030** (the casing checker existed but no pipeline called
> it), **0053 and 0009 in full**, **0054 §§ 1–4 in full** (`Core\Decimal`'s own roster is M8's, and
> that ADR's *Verification* says so), and **0066 §§ 1–3 for the checked numeric targets** — a
> nullable binding, parameter, property and return, `null` itself, `??` and now `as
> ?int`/`?uint`/`?float`/`?decimal` all run, leaving that ADR's enum target and its
> cannot-fail/no-conversion *refusals* owed; its **class** refusal is built and is now absolute
> (`E0473` for every class target, the withdrawn parse roster included), with § 3a's `tryParse` the
> member that answers a parse instead, and **0056 §§ 1, 2 and 5** — both engines are bound, the tier is chosen by the pattern and
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
> **Open now:** **The frontier is M4's language holes, and there are 43 of them**, ordered and
> grouped by file set in [docs/agent/loop-goal.md](agent/loop-goal.md). A hole is a shape that
> compiles in the front end and then refuses below it, and it is closed when it either runs with a
> fixture or a `.mwlt` case pinning what it prints, or is refused by a **diagnostic that names the
> rule** — never by a panic. **`python tools/holes.py` is the worklist and no session re-derives
> it**: it reads the refusal sites out of `mwl-ir` and `mwl-codegen` live, attributes each to the
> item whose prose names its function, and prints what no item claims (45 sites, 13 items, 2
> unattributed today). `--item N` is one item in full. `python tools/loop.py --list` prints the 32
> named `.mwlt` cases each stage owes and which are still to write. **Stage 0 is the operator table,
> and its last two items are open**: ADR 0007 § 4's promotion rows run (`1 + 1.5`, `$n < $f`, and §
> 2's implicit widening at a binding), integer `/` answers PHP's `int|float`, `+`/`-`/`*` and unary
> `-` throw `ArithmeticError` on overflow instead of wrapping, all six bitwise operators lower with
> PHP's own shift rules, `**` answers every numeric row and throws where that row has no integer,
> and `<=>` answers `-1`/`0`/`1` for every scalar `<` already orders — so what is left of Stage 0 is
> the increment/decrement pair and `==` over two enum values, plus the four named cargo guards items
> 1 and 2 never wrote. Even so, `tools/loop.py` runs it before the program legs, because every
> fixture and every case in every stage below is written against those rules. **Unbuilt in the
> library**, none of it a registration gap: `Core\Json::decodeAs<T>`'s wider codec-reachable set and
> its two default-bearing rows (`mwl_stdlib::json` gaps), ADR 0088's qualifier classification
> (`mwl_stdlib::hash`'s module doc), and ADR 0086 § 1's substitution table (M8,
> `crates/mwl-stdlib/src/cli.rs` gap 1). **Decided and unbuilt, and out of this goal's scope** —
> ADRs 0091, 0092 § 2's log levels, 0093, 0097 and 0100 § 3; their work is M6, M7, M8 and M10. **M4S
> Part I is the floor, not the frontier**: conformance is at 558 of the goal's new 750 and
> differential at 162 of 165, `python tools/gaps.py` still ranks the thin classes, and a `Core`
> depth slice is a legitimate slice when a group is blocked — never a reason to leave a language
> item unfinished. `docs/spec/02-php-migration.md` is 31% classified (`python
> tools/check-migration.py`).
>
> **Blocking:** Nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in `docs/agent/loop-goal.md` § *Standing decisions*, including the ones
> this goal added: an abandoned generator's `finally` runs through a resume-to-unwind entry point
> and **not** a destructor (so ADR 0028 § 2 stands, with one sentence folded into it), `int / int`
> widens at the binding rather than at the operator, a nullsafe assignment target and an element
> write through a hooked property are both compile errors because PHP refuses both, `&value` as an
> array-literal element does not exist (ADR 0031 § 2 left it no owner), and a `Core` class
> stringifies exactly where the spec gives it a `toString`. The representation questions the
> previous goal settled are settled still — `?T`/`mixed` (`mwl_ir::Ty::Tagged`), object identity
> (`mwl_runtime::identity`), element-covariant array reads (`mwl_types::expr::is_assignable`), a
> `catch` binding scoped to its own handler — and picking every dependency but the two the user
> named remains pre-authorized. Stages 1 and 8's floor passes whole on both legs with no
> intermittently red test left. **The one thing this goal forbids outright** is growing the
> allowlist in `every_refusal_is_a_diagnostic_or_decided` to make a run go green: every entry on it
> is a bullet in § *Standing decisions*, and a session that needs a new one takes the decision
> there, in the same session, with the reason.

**How the plan relates to the ADRs.** The plan is the record of *what* gets built, in what order, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR the plan links to it instead of restating
it — follow the link rather than expecting the argument here. For decisions with no ADR of their own, the
*why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the *mechanics*
are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

| Milestone | What it builds | Loop-days |
|---|---|---|
| [M0](plan/m0.md) | Project setup (~3 days) — **done** | 0.3 |
| [M1](plan/m1.md) | Front end (~3 weeks) | 0.7 |
| [M2](plan/m2.md) | HIR, types, IR (~4 weeks) | 1.5 |
| [M3](plan/m3.md) | Baseline Cranelift backend → **Hello World** (~3 weeks) | 0.5 |
| [M4](plan/m4.md) | Language completeness — a usable CLI language (~10 weeks) | ~3 |
| [M4S](plan/m4s.md) | The `Core` API contract and its pure half (~5 weeks) | ~1.5 |
| [M4B](plan/m4b.md) | Minimal `mwl-lsp` and the VS Code extension (~3 weeks) | ~1.5 |
| [M5](plan/m5.md) | Concurrency and script isolates (~5 weeks) | ~3.5 |
| [M6](plan/m6.md) | Config, limits, capabilities, disk cache (~3 weeks) | ~1 |
| [M7](plan/m7.md) | Built-in HTTP server (~4 weeks) | ~2 |
| [M8](plan/m8.md) | Stdlib and databases (~16 weeks) | ~6.5 |
| [M9](plan/m9.md) | Extension system (~6 weeks) | ~2.5 |
| [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined) | ~8 |
| [M11](plan/m11.md) | PHP transpiler (~10 weeks) | ~3 |
| [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |
| [M14](plan/m14.md) | Optional wasm32 browser target | not estimated |
| [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks; scheduled after M6) | ~3 + a calendar floor |
| [M16](plan/m16.md) | `mwl/web`, `mwl new`, and the framework (~12 weeks; scheduled after M7 and M8) | ~4 |

Each row is a file under [docs/plan/](plan/). `python tools/plan.py --show M8` prints one
without you needing to know that, and `--show M8:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).

**The two columns are not the same unit.** The parenthesised weeks are the original estimate, written for
a human team before any code existed; **Loop-days** is what this project's unattended loop actually spends,
elapsed and continuous — M0–M3 are measured, the rest projected. The measured conversion is ~23×, it is
not applied uniformly, and it carries a rework tax and four ways it breaks:
[docs/plan/velocity.md](plan/velocity.md) is the one home for all of that. Summed, the milestones left
come to **~7 weeks** against the ~99 the original column still shows. Neither figure gates anything.
