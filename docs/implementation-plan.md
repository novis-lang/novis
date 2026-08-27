# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-27. Current milestone **M4**, language completeness: every shape that compiles
> in the front end and then refuses below it, closed, before M4B's LSP is written against the
> surface. M0–M3 are done and M4S Part I is the corpus floor rather than the frontier.
> [docs/agent/loop-goal.md](agent/loop-goal.md) holds the goal's items grouped by file set, and
> `python tools/holes.py` is the live count behind them. Dependencies: `regex` + `fancy-regex` and
> `jiff` are named by the user; the rest the loop picks under ADR 0051 § 4.
>
> **Done:** M0 (setup) and M1 (front end) whole, M2 (HIR, types, IR) and M3 (baseline Cranelift
> backend) whole, M4S Part I registered — `crates/mwl-stdlib/tests/spec-members-outstanding.txt`
> holds no keys, which is this project's definition of *registered*. M1's own section lists the one
> grammar addition still owed (`autoload`, ADR 0061). Each milestone file under
> [docs/plan/](plan/) states its own acceptance.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan and fuzz legs), and the
> nine crates — `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-runtime`,
> `mwl-stdlib`, `mwl-codegen`, `mwl-cli` — plus `mwl-test`, `fuzz/`, `tools/`, `benches/abi-probe`,
> and the two case trees `tests/conformance` and `tests/differential`. **Each crate's own module doc
> is the authority on what it holds and what it still owes**; `python tools/brief.py` prints one map
> line each, and `python tools/disk.py` the live counts.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** **each ADR's own *Verification* section is the authority on what its slice
> covers, and this field never restates one** — `python tools/brief.py --where <keyword>` routes to
> the ADR that owns a topic, and `python tools/adr.py --stats` shapes the whole set. What a crate
> still owes is its own module doc's `# Known gaps`. What landed in which session is in `git log`.
>
> **Open now:** **M4's language holes are the frontier**, ordered and grouped by file set in
> [docs/agent/loop-goal.md](agent/loop-goal.md). A hole is a shape that compiles in the front end
> and then refuses below it; it is closed when it either runs with a fixture or a `.mwlt` case
> pinning what it prints, or is refused by a **diagnostic that names the rule** — never by a panic.
> The statement dispatch has no shape left that the checker accepts: ADR 0007 § 3.3's `[int $a,
> string $b] = $pair;` lowers as the subscripts it is spelled out of and refuses what one refuses
> (`E0482`/`E0401`/`E0483`), inline HTML lowers over its raw span, a nested
> `class`/`interface`/`enum` is `E0233`, an increment takes its write target's own
> `E0479`/`E0478`/`E0480`, `unset()` is narrowed to an array element of a named holder and refuses
> every other operand (`E0234`, plus `E0413` for a static property), an element write whose root is
> only a temporary is `E0700` — the first code of the `E07xx` band the full `E04xx` one continues in
> — an increment's own target passes the parser's `E0105` gate like every other write spelling, the
> read-modify-write rewrite's own assertion has no reachable target left — its doc comment carries
> the proof, and `mwl_types`' two write-target refusals are two thirds of it — an element write
> evaluates the receiver under its root holder exactly once, PHP 8.5.9's own count, and `int $x;`
> and `;` both lower. Three live tools **are** the worklist and no session re-derives one: `python
> tools/holes.py` reads the refusal sites out of `mwl-ir` and `mwl-codegen` and attributes each to
> its item (`--item N` for one in full), `python tools/loop.py --list` prints the named `.mwlt`
> cases each stage still owes, and `python tools/check-migration.py` scores
> `docs/spec/02-php-migration.md`.
>
> **Blocking:** Nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in [docs/agent/loop-goal.md](agent/loop-goal.md) § *Standing decisions*,
> which is where a new one is taken, in the session that needs it, with its reason. Picking every
> dependency but the two the user named is pre-authorized under ADR 0051 § 4.

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
| [M4B](plan/m4b.md) | Minimal `mwl-lsp`, syntax highlighting and the VS Code extension (~3 weeks) | ~1.5 |
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
