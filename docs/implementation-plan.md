# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-26. **M3 is done, and Stage 0 has re-opened with a second catch-up batch.**
> All seventeen original items of [docs/agent/loop-goal.md](agent/loop-goal.md) § *Stage 0* are done
> — 12 landed **twice**, its `$s as ?Uri` parse roster withdrawn by ADR 0066 § 3 in favour of
> `Core\Uri::tryParse`/`Core\Uuid::tryParse`, so `as` now targets no class at all — and every one of
> the 37 tests `loop-goal.toml`'s `stage = "0 catch-up"` block named for them exists and passes.
> **Items 18–22 join them from a bench review**, so `python tools/loop.py --goal-only`
> short-circuits at Stage 0 again and Stage 3 waits behind them; `Open now` says what they are and
> [docs/perf/userland-gap.md](perf/userland-gap.md) holds every number. Behind them is Stage 3's
> last fixture, `examples/collect.mwl`, itself past its parser hole: `new Core\ObjectSet<Tag>()`
> parses, because `ExprKind::New` carries a `type_args` list read by the same checkpointed trial
> parse a call site's own `<...>` goes through, and both lists now *bind*:
> `registry::GENERIC_CLASSES` is the roster of `Core`-owned generic classes, so lines 13 and 19
> resolve and the fixture's first report is `Core\Encoding`/`Core\Hash` at line 25 — spec § 7 is the
> frontier, with § 9's three collections owning no members yet. Stage 4's own counts (conformance
> 436 of 600, differential 90 of 150) are the wall after that. **ADR 0087's lexer half is built** —
> `mwl_syntax::bidi` is the one predicate an unterminated directional control is rejected by,
> `E0008` at the lexer and, at M7/M8, a substitution at both sinks. **ADR 0090 §§ 1 and 2 are
> built**: `==`/`!=` are the only equality spellings, `===`/`!==` are `E0232` at the lexer (consumed
> whole, reported, then lexed as the two-character operator so a file still reports its other
> problems), `BinaryOp::Identical`/`NotIdentical` are gone from the AST, and two statically disjoint
> operands are `E0466` — at `==`/`!=`, and at the `switch` label and `match` arm § 6 points at the
> same rule. **What that ADR still owes** is § 3's string, array and object rows, one runtime helper
> each (M3/M4). **ADR 0069's `+`/`+=` refusal is built too**, `E0467` naming `Core\Arr::underlay`.
> **The goal behind the catch-up is unchanged: M4S Part I in full — spec §§ 1-12 — plus the M4
> surface it cannot be written without.** Every representation that blocked a section is built and
> recorded in the crate that owns it: `mixed`/`?T`/every union is `mwl_ir::Ty::Tagged`, strict
> identity is `mwl_runtime::identity`, `decimal` is `mwl_runtime::decimal`, a `Core`-owned instance
> is an ordinary MWL object (`mwl_stdlib::instance`), and a variadic tail is one `array<T>` argument
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
> `conformance`, `defaults`, `derive`), `mwl-ir`, `mwl-runtime` (+ `alloc`, MWL's own
> `#[global_allocator]` in an optimized build, `object`, `array`, `throwable`, `closure`,
> `identity`, `decimal`), `mwl-stdlib` (`Arr` × 36, `Str` × 39, `Math` × 38, `format`,
> `granularity`, `ordering`, `cldr`, `instance`, `issue`, `Order`, `NormalForm`, `RoundMode`, `Unit`
> and `Weekday`, `Regex` × 6 plus `Regex\Match` × 4 over `regex`/`fancy-regex`, `Time` × 7 plus
> `Time\Instant` × 9, `Time\DateTime` × 14, `Time\Duration` × 19 and `Time\Zone` × 4 (+ `UTC`) over
> `jiff`, `Json` × 4 over `serde_json`, `Path` × 9 (+ `SEPARATOR`) over nothing at all, `Random` × 6
> over `rand`, `Uuid` × 4 (+ `toString`) over `uuid`, `Uri` × 4 over nothing at all, `Encoding` × 2
> over nothing at all, `ObjectMap` × 9, `ObjectSet` × 9 and `Heap` × 5 over `identity_store`, all
> three iterable through `cursor`, and the conformance-coverage gate), `mwl-codegen`, `mwl-cli`
> (`ast`, `check`, `run`, `test`, `info`), `mwl-test` (+ `case`, `expect`, `run`),
> `tests/conformance` × 436 (in `array`, `class`, `core`, `enum`, `error`, `iter`, `lang` and
> `reject`) and `tests/differential` × 90, `fuzz/`, `tools/`, `benches/abi-probe`.
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
> **Open now:** **Stage 0 holds one item, and two thirds of it are landed.** `python tools/bench.py`
> puts MWL's median at **0.69×** PHP 8.5.9 with its JIT on, and docs/perf/userland-gap.md is the
> ledger behind that number — the suite case by case, what one operation costs, and which item moves
> it. Items **18**, **19**, **20** and **21** are **done**: MWL owns its allocator in every
> optimized build and the test build's byte counters wrap it, an `int` subscript travels to
> `mwl_array_get_index`/`mwl_array_set_index` unrendered — **6.3 ns against the rendered path's 28.5
> ns** — a `string` carries a capacity so `$out .= $piece` appends into its own buffer,
> `mwl_ir::ir::InstKind::Concat` is **n-ary**, a **string literal no longer allocates at all** — a
> whole `StrHeader` with a pinned refcount goes into the compiled unit's data section — and, item
> 21, **no `Core\Arr` callback member synthesizes a key nothing observes**: `map`, `filter`,
> `reduce` and `sort` each read `mwl_runtime::closure_arity` once before their loop, and
> `mwl_runtime::SlotKey` carries a preserved key in the shape the subject already holds it rather
> than as a rendered decimal. The first three took `03-string-concat` from **0.03× to above 1.00×**
> and `04-string-format` from **100.0 ms of work to 87.2 ms**; the fourth took `12-array-map-filter`
> from **0.36× to 0.51×**, `10-array-sort` from **0.69× to 0.96×** and `11-array-sort-by-field` from
> **3.98× to 5.12×**. The median went 0.31× to **0.69×** over the four. §§ B, C and D of the ledger
> record them, what the third header word spends is stated in `crates/mwl-runtime/src/string.rs`'s
> own module doc, why a pinned refcount keeps that module's plain `Cell` sound is its § *An immortal
> string*, and the key rule is `crates/mwl-stdlib/src/arr.rs`'s § *A callback that does not want a
> key is never handed one*. What item 19 still owes is the append-only `docs/perf/history.ndjson`
> entry item 15 asked for, which does not exist yet. Item **22** — a `Core\Str` member writes its
> result once, § E of the ledger — is two of its three slices in. Reading a `string` argument is a
> **tag check**: the unchecked read sits once behind `mwl_runtime::MwlStr::text_of`,
> `mwl_runtime::Value::as_text` is the safe caller that discharges it, and a debug build
> re-validates inside that one reader, so the O(n) `from_utf8` that ran at 56 call sites in `str.rs`
> is gone. `crate::granularity`'s ASCII fast-path test is one branchless fold where it was an
> `is_ascii` scan plus a separate search for `\r`, so `Core\Str::length` makes one pass and then
> `len`. Measured as an A/B against the commit before it: `05-string-replace` **0.72× → 0.91×**,
> `06-string-split-join` **0.78× → 0.96×**, `07-string-normalize` **0.42× → 0.51×**,
> `04-string-format`'s own work 92.9 ms → 82.0 ms. The median held at **0.69×** because those four
> rows crossed *over* it rather than lifting it. Left from here is § E's other half: `produced`
> allocates twice, and where the result length is known — `replace`, `padStart`/`padEnd`, `join` —
> the member can write straight into one `MwlStr`; the same helper is in `bytes.rs`, `path.rs` and
> `regex.rs`. It is not a JIT optimisation and does not belong to M12; it is the pattern every
> `Core` member written after it would copy. **What has already landed is not restated here** — `git
> log` holds the session-by-session history and the crate's own module doc holds its per-file gaps,
> which is this field's contract in AGENTS.md § *Keep each slice small*. What follows is what is
> **not** built. **Spec §§ 1-12, by section** — § 1 is **whole**, `normalize` having landed with
> `Core\NormalForm` and a named binding to `unicode-normalization`; § 2 is **whole**, `from` having
> landed over `registry::CoreTy::Iterated` — ADR 0069's four combination members, the
> `diff`/`intersect` set half with `Core\SetOn`, the positional rows, the callback rows and both
> sorts are all registered, which the ratchet below is the machine-readable statement of; § 4 is
> **whole**, `withTime` having landed beside `Core\Time\Date`, `Core\Time\TimeOfDay` and the two
> views that answer with them, and there is **no** `Core\Month` — § 4 writes no member that takes or
> answers with one (`mwl_stdlib::time` gap 1); § 5 is **whole**, `replaceWith` having landed beside
> `Core\Regex\Pattern` and `compile`, with the callback taking one `Match` rather than PHP's
> positional array and the six pattern-taking rows all reading the `Pattern|string` the spec writes;
> § 6 owes `decodeAs<T>` (`json` gap 2, which waited on a written type argument at a call site and
> no longer does); § 7 is **whole** — `Core\Bytes`'s twelve members and the whole of
> `Core\Encoding`, `pack`/`unpack` sharing one closed code table that
> `crates/mwl-stdlib/src/bytes.rs`'s own module doc states; § 9 is **whole**, its three collections
> each answering a `foreach` — a `Core` receiver reaches ADR 0053's protocol through its
> descriptor's own method table (`mwl_stdlib::cursor`, `mwl_stdlib::instance`'s dispatch roster),
> and the spec's `Heap` row is amended to declare `Iterable` because a heap whose contents can only
> be reached by emptying it is the PHP behaviour § 9 replaces; § 10 is **whole**, an `issues` entry
> being readable now that a shape field resolves to a name the IR fetches by name
> (`ExprInfo::ShapeProperty`, `InstKind::SlotGet`) — the constructor takes `{previous: $e}` and
> `$e->location` is pinned as the throw site, both stated by `mwl_types::error_lib`'s own module
> doc; § 11 is **whole**, `Random::bytes` and `Hash::stream` having landed; § 12 owes `Out::capture`
> alone, `Uri`, `Csv` and `Validate` being whole. §§ 3 and 8 are whole, and every signature shape
> the spec writes can be stated (`registry::CoreTy`'s `Variadic`, `Instance`, `Union`, `Decimal` and
> `Iterated`, plus `WRITTEN_CLASS_MEMBERS`), so a section that is not built is only unwritten. **The
> runtime hole that sat under four of those is closed** — `mwl_runtime::Tag` has a `Bytes` row of
> its own over the existing `MwlStr` allocation, so a fresh `bytes` value now constructs, refcounts,
> releases and round-trips through codegen; `mwl-runtime`'s module doc § *`bytes` is a tag, not a
> second heap shape* owns that decision and what it spends. `Core\Hash\Stream` is the first
> *mutable* `Core` instance and the honest return type of `Uri`'s two decoders is still unwritten
> rather than blocked. **Stage 4's counts are their own work rather than a side effect of member
> slices** — conformance is 436 of the 600 that gate requires, differential is 90 of 150, and
> `every_part_one_spec_member_is_registered` — the loop's own definition of done, which reads the
> spec's member rows and checks each against the registry — exists now, in
> `crates/mwl-stdlib/tests/spec_registry_coverage.rs`. It is a **ratchet against
> `tests/spec-members-outstanding.txt`** rather than a permanently red assertion, for the reason its
> own module doc states, and that file's **one remaining key — `§12 Out::capture` — is the whole
> machine-readable work list for §§ 1-12**: when it holds none, Part I is registered whole. **ADR
> 0088 opens one registry-wide item**: `mwl-stdlib`'s member rows carry no qualifier classification,
> so `Core\Str::format`'s template is not yet the sink that ADR makes it, and neither the
> fail-closed default for an unclassified `string`/`bytes` parameter nor the test that refuses an
> unclassified member exists; it lands with M4S's remaining sections. **ADR 0071 is built end to end
> for a scalar-fielded class**; its gaps are no enum/`decimal`/`Instant`/`array`/nested-class field
> decode and no optional key from a parameter default. **Four ADRs are decided and unbuilt but are
> not catch-up** — 0091 (the `development`/`production` run mode: a `[mode]` block, a `System`
> ceiling, five governed directives plus § 3a's three startup defaults, `Core\Env::mode`), 0092 (one
> diagnostic record rendered as plaintext, JSON or HTML by the sink in force — `Core\Log`'s
> `Log\Level`, `Core\Debug::dump`, throwables, test results and compiler diagnostics), 0093 (`mwl
> service`) and 0097 (the server's scope and its `[server]` block: mounts expanded from globs at
> boot, the trusted-proxy walk, four idle waits, `max_in_flight`; and the removal of the inbound TLS
> listener, h2c and the FastCGI transport, which deleted M13). None invalidates built behaviour or a
> written fixture; their work is M4, M6, M7, M8 and M10 and lands with those milestones. **Still
> open beside the library** — a property's **declared default runs now** and is type-checked
> (`E0472`), a per-class image on `mwl_runtime::ClassDesc` that `MwlObj::new` writes, so
> `NewDynamic` and ADR 0071's native decoder reach it too; what a default may be is still a literal
> or `[]`, with `= null`, an enum case and a `decimal` refused for the reasons `mwl_types::defaults`
> states. `do`/`while` is the one M4 control-flow statement that does not lower; an abandoned
> generator never runs the `finally` it is suspended inside (`mwl-ir` gap 18, a PHP divergence); ADR
> 0043's `by`-delegation is off path; and `docs/spec/02-php-migration.md` is 31% classified, one
> pass per PHP domain, reported by `python tools/check-migration.py`.
>
> **Blocking:** nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in `docs/agent/loop-goal.md` § *Standing decisions*, including the
> `?T`/`mixed` representation (now settled and recorded in `mwl_ir::Ty::Tagged`), re-scoping a
> `catch` binding to its own handler block, widening `array<T>` to element-covariant-on-read (now
> settled and recorded in `mwl_types::expr::is_assignable`), object identity (now settled and
> recorded in `mwl_runtime::identity`), and picking every dependency but the two the user named.
> Stages 1 and 2 pass whole on both legs, so any failure below Stage 3 is a regression rather than
> unfinished work. **Behind Stage 0's five reopened items** the loop resumes **Stage 3**, `Core`
> Part I across all twelve spec sections: six of
> its seven fixtures produce their frozen output — `examples/core.mwl`, `report.mwl`, `numbers.mwl`,
> `text.mwl`, `dates.mwl` and `json.mwl` — and `collect.mwl` is the first that does not. That one
> fixture names spec §§ 7, 8, 9, 11 and 12 at once, so it is several slices rather than one: § 8's
> `Core\Path`, § 11's `Core\Uuid` and § 12's `Core\Uri`, `Core\Csv` and `Core\Validate` are all
> built and whole. **Every § 12 dependency is now picked** — `Uri`'s grammar half took `fluent-uri`
> for RFC 3986 over `url`'s WHATWG reading, `Csv` took `csv-core` for its reader,
> hex needed none, `Encoding`'s base64 and base32 pairs took `base64` and
> `data-encoding`, its text trio `encoding_rs`, and `Hash` the RustCrypto family, while `Validate`
> argued none at all; each module's own doc holds its reasoning. So the
> fixture's first report has moved past all of them to `Core\Out::capture` at
> `collect.mwl:47`, which lands with M4S's sink work rather than before it.

**How the plan relates to the ADRs.** The plan is the record of *what* gets built, in what order, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR the plan links to it instead of restating
it — follow the link rather than expecting the argument here. For decisions with no ADR of their own, the
*why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the *mechanics*
are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

| Milestone | What it builds |
|---|---|
| [M0](plan/m0.md) | Project setup (~3 days) — **done** |
| [M1](plan/m1.md) | Front end (~3 weeks) |
| [M2](plan/m2.md) | HIR, types, IR (~4 weeks) |
| [M3](plan/m3.md) | Baseline Cranelift backend → **Hello World** (~3 weeks) |
| [M4](plan/m4.md) | Language completeness — a usable CLI language (~10 weeks) |
| [M4S](plan/m4s.md) | The `Core` API contract and its pure half (~5 weeks) |
| [M4B](plan/m4b.md) | Minimal `mwl-lsp` and the VS Code extension (~3 weeks) |
| [M5](plan/m5.md) | Concurrency and script isolates (~5 weeks) |
| [M6](plan/m6.md) | Config, limits, capabilities, disk cache (~3 weeks) |
| [M7](plan/m7.md) | Built-in HTTP server (~4 weeks) |
| [M8](plan/m8.md) | Stdlib and databases (~16 weeks) |
| [M9](plan/m9.md) | Extension system (~6 weeks) |
| [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined) |
| [M11](plan/m11.md) | PHP transpiler (~10 weeks) |
| [M12](plan/m12.md) | Optimising JIT tier (ongoing) |
| [M14](plan/m14.md) | Optional wasm32 browser target |
| [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks; scheduled after M6) |
| [M16](plan/m16.md) | `mwl/web`, `mwl new`, and the framework (~12 weeks; scheduled after M7 and M8) |

Each row is a file under [docs/plan/](plan/). `python tools/plan.py --show M8` prints one
without you needing to know that, and `--show M8:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).
