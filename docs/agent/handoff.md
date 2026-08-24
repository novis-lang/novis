# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The live path is unchanged: `Core` breadth for Stage 3.** Six of seven fixtures produce their frozen
output; `examples/collect.mwl` is the first that does not. Nothing in the tree changed this session.

- **[ADR 0079](../adr/0079-testing-is-a-language-feature.md) is written and nothing is built.** It designs
  the testing capability MWL programs use — `#[Test]` compiling to a table, isolate-per-test, generic
  assertions, closure-shape doubles, `#[Fixture]`, `#[TestWith]`, property testing, inline snapshots,
  `#[Bench]`, `mwl test --mutate`. Its § 24 is the milestone table; the plan's M4/M5/M8/M10 sections now
  each name their slice of it. **Do not start it while Stage 3 is open** — the assertions land at the M4S
  tail, after `Core` breadth.
- It **amends two ADRs**, both folded into their bodies: [0018](../adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
  gains a *counting* mode on its existing probe sites (statements, calls, allocations, bytes, GC cycles —
  bit-identical across machines, which is what `#[Bench]` reports and what CI can gate on), and
  [0026](../adr/0026-performance-measurement-methodology.md)'s *Scope* now says user-program benchmarking
  is 0079's, not its.
- `.mwlt` is untouched and stays MWL's own conformance format ([0079](../adr/0079-testing-is-a-language-feature.md) § 23).
  `crates/mwl-test`'s module doc is still the one home for it.
- Verified: `python tools/check-links.py` clean across 92 files, `python tools/verify.py` green. The plan's
  status block was deliberately **not** edited — 0079 is scheduled, not in flight, and `Open now` is
  already 8× its size target.

## Next

**`Core\Path` — spec § 11.** Still the cheapest slice inside `examples/collect.mwl`: `join` (variadic,
which exists), `basename({withoutExtension})`, `extension(): ?string`, `SEPARATOR`, and no new
dependency. `docs/agent/loop-goal.md` § *Standing decisions* has the two-legs rule for `SEPARATOR` — a
case asserting a built path must normalize it.

## Backlog

- **`Core\Encoding`, `Core\Hash`, `Core\Uuid`, `Core\Csv`, `Core\Validate`, `Core\Random`, `Core\Out`,
  `Core\Uri::parseQuery`** — the rest of `examples/collect.mwl`; each needs a dependency picked under
  [ADR 0051](../adr/0051-standard-library-tiers.md) § 4 and its three obligations.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item that needs *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need.
- **`Core\Time\Date`, `Core\Time\TimeOfDay`, `Core\Month`, `DateTime::date`/`timeOfDay`/`withTime`** —
  `time.rs`'s gap 1; the machinery all exists, so each is a registry row and a body.
- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all
  declare the variadic that exists, so each is a registry row and a body.
- **A shape property read does not lower** — `mwl-ir`'s ADR 0036 § 4 gap: `$e->issues[0]->path` panics
  naming that ADR rather than reading slot 1, so an issue's own fields are unreadable from MWL.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of a throwing `Core\Json::decode("…")` inside a `try` lose 50 blocks.
- **[ADR 0079](../adr/0079-testing-is-a-language-feature.md)'s first slice, after Stage 3** — `#[Test]`
  parsing plus the compile-time table (§ 1) and the generic `Core\Test` assertion roster (§ 4). Its
  *Verification* section is the fixture list.
