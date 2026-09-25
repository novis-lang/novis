# Dependency update prompt

Reusable prompt for the sweep that keeps Novis current — crates, the Rust toolchain, CI actions, the developer
tooling and the PHP oracle. **Paste this whole file as the prompt** when you want a pass. Update it in place
when a pass finds a step that is wrong or missing.

**A human fires this, always.** No agent, loop, cron job or CI workflow may start this pass on its own, and
an agent that notices a stale dependency mid-session says so and carries on with its own work
(`rule:packaging/the-sweep-is-fired-by-a-human`). Bumping a dependency is the
one class of change whose entire value is that a person weighed it.

**The policy is `rule:packaging/a-dependency-break-is-absorbed-never-forwarded`; this file is only
the procedure.** Every judgement call below routes to a section number there rather than restating it. If the
two ever disagree, the ADR is right and this file is the bug.

## Before you start: which regime are we in?

Read the version in [Cargo.toml](../../Cargo.toml)'s `[workspace.package]`.

- **Below 0.1.0 — the current state.** Prototyping. Update anything, fix what breaks, commit. Do steps 1, 2,
  4 and 7 below and skip the rest: there is no classification, no ladder, no sign-off, and no release
  consequence (`rule:packaging/the-version-contract-starts-at-0-1-0`). Do not apply the contract rules "early" — that is how a prototype acquires a
  compatibility surface nobody agreed to.
- **0.1.0 or above.** The whole file applies.

## 1. Orient

```sh
bun nv orient --full                      # where the run and the work stand, one line per module
git status                                # start clean; a sweep never rides on top of unrelated work
cargo update --dry-run                    # what would move inside the declared ranges
cargo tree --duplicates                   # multiple-versions warnings that a bump might fix or create
```

Then look at the four places a pin lives, because `cargo update` sees only the first:

| Pin | File |
|---|---|
| Crate ranges, and any `# HOLD (revisit …)` records | [Cargo.toml](../../Cargo.toml) `[workspace.dependencies]` |
| The Rust toolchain, components and targets | [rust-toolchain.toml](../../rust-toolchain.toml) |
| CI action majors (`actions/checkout@v4`, `Swatinem/rust-cache@v2`, …) | [.github/workflows/ci.yml](../../.github/workflows/ci.yml) |
| Everything else that has a version — PHP oracle, `cargo-fuzz`, valgrind, MSVC/SDK | [the plan](../implementation-plan.md)'s status block § *Toolchain*, which is that field's only home |

Any `HOLD` whose date has passed is part of this sweep: retry it, and either drop the hold or give it a new
date and a reason that is still true.

## 2. Move one thing at a time

**One dependency, one commit** (`rule:packaging/one-bump-one-commit`) — or one group that genuinely cannot move separately, such as
the four `cranelift-*` crates, which must also stay compatible with the pinned `wasmtime`
(`rule:packaging/an-extension-is-a-sandboxed-wasm-component`).

```sh
cargo update -p <crate>                   # inside the declared range
# for a semver-incompatible bump, edit the version in [workspace.dependencies] first, then:
cargo update -p <crate> --precise <version>
```

Run the battery after each one. Batching is what makes a failure take an hour to attribute.

## 3. The battery

Run **A** for every bump. Add the conditional legs the moved crate actually touches — over-running costs
minutes, under-running costs a silently retracted premise.

**A. Always.**

```sh
cargo build --workspace --all-targets
cargo test --workspace --all-targets
cargo test --workspace --doc
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check                          # cargo install cargo-deny --locked, if absent
bun nv gen-attribution                    # regenerate; commit the diff in the same commit
```

**B. If `cranelift-*`, `wasmtime`, `corosensei` or anything the runtime links moved** — the premise guards.
Takes over two minutes; start it in the background:

```sh
cargo test --release -p nvs-abi-probe
cargo test --release -p nvs-abi-probe --features wasm-probe    # wasmtime moved
```

**C. If codegen, the runtime or the stdlib moved** — the end-to-end legs, both platforms plus the leak sweep:

```sh
bun nv loop --goal-only                   # --list shows what it would run
```

The Linux leg and the leak sweep have no `bun nv` command yet: `tools/nv/driver/accept.ts`'s "Not here
yet" list names them.

**D. If a lexer/parser-adjacent crate moved:**

```sh
wsl.exe -- bash -lc "cd /mnt/<drive>/<repo> && cargo +nightly fuzz run lex -- -max_total_time=300"
wsl.exe -- bash -lc "cd /mnt/<drive>/<repo> && cargo +nightly fuzz run parse -- -max_total_time=300"
```

**E. If the JIT backend moved,** refresh the callgrind instruction counts per
[AGENTS.md](../../AGENTS.md) § *Fuzzing and callgrind on Windows* and record the delta in the report. A
double-digit percentage move is a finding, not a footnote.

**F. If the PHP oracle version changed,** re-run the differential conformance comparison before trusting a
single one of its results.

**Two rules that are not yours to bend** (`rule:packaging/a-dependency-move-proves-two-things`): a failing guard test in `benches/abi-probe/` is never
fixed by editing its threshold — the ADR named in the test's own comment is what gets revisited, and the
sweep stops there — and a graph change always commits the regenerated `THIRD-PARTY-LICENSES.txt` alongside
it.

## 4. When something breaks

Ask one question: **can an Novis program author see it?** (`rule:packaging/who-can-see-it-decides-the-release-slot`.)

- **No — it is our insides.** Fix it: adapt the call sites, add an adapter, done. No approval, no version
  event, patch-level. This is the common case and it needs no ceremony.
- **Yes.** Work down the absorption ladder in `rule:packaging/a-dependency-break-is-absorbed-never-forwarded` and stop at the first step that holds — adapt,
  adapt behind our own adapter, compensate at the boundary, hold with a dated record, fork/vendor, replace,
  and only then ship the break. Record which step you stopped at in the report.

**Stop and ask the user before** any of: shipping a break an Novis program can see (§ 5 step 7); forking or
vendoring (step 5); replacing a dependency (step 6); admitting a crate whose licence is outside
[deny.toml](../../deny.toml)'s allow list, or a C dependency, which is
`rule:packaging/a-c-dependency-answers-two-questions`'s two questions; or a guard-test failure that looks like
a real premise change rather than noise.

**A live advisory outranks everything,** including whatever milestone is in flight (`rule:packaging/the-sweep-is-fired-by-a-human`). A hold may
never cover an advisory that actually applies.

## 5. Classify each bump

Give every commit its § 4 row from `rule:packaging/a-dependency-break-is-absorbed-never-forwarded` — `patch`, `minor` or `major` — in the commit message. Nothing
checks this; the accumulated set is what decides the next release's number, so an unclassified bump is a
number nobody can compute later.

## 6. Deprecations

If a break did reach the surface and a cycle is possible, land the warning **now**, in a minor: the old
spelling keeps working and the compiler names its replacement (`rule:packaging/a-forced-break-is-announced-before-it-lands`). Removal waits for the major.
Write the migration note in the same commit as the warning, not at release time.

## 7. Close the pass

1. **Write the versions down where they live.** A changed toolchain, PHP oracle or developer-tool version
   goes in the plan's status block § *Toolchain* — overwrite the field, never append. Crate versions live in
   `Cargo.toml` and nowhere else; do not copy a version number into prose.
2. **Every hold has a date and a reason** on its own line in `[workspace.dependencies]`, or a dated entry in
   `deny.toml`'s `advisories.ignore`.
3. **`bun nv links`** if any doc moved.
4. **Commit** — one per bump, classification in the message.
5. **Overwrite `docs/agent/handoff.md`** per [AGENTS.md](../../AGENTS.md) § *Keep work small, commit your
   work*, then show the user the report below.
6. **Stop.** No second `cargo` pass after the commit.

## The report

Show the user exactly this, filled in:

```
Dependency sweep — <date>, regime: <prototyping | contract>

Moved
  <crate>  <old> → <new>   <patch|minor|major>   <one line: what changed for us>

Held
  <crate>  <version>  revisit <date>  — <reason>

Battery
  A always        <pass/fail>
  B guards        <pass/fail/skipped — why>
  C end-to-end    <pass/fail/skipped — why>
  D fuzz          <pass/fail/skipped — why>
  E callgrind     <delta, or skipped — why>
  F oracle        <pass/fail/skipped — why>

Absorbed
  <crate> — stopped at ladder step <n>: <what we did instead of forwarding the break>

Release consequence
  <the highest classification above, and what it means for the next version number>

Needs you
  <decisions waiting on the user, or "nothing">
```

## Tooling notes that cost a session to rediscover

- `cargo test` does not always relink `target/debug/nvs.exe` — run `cargo build` before testing a
  fixture by hand.
- `wsl.exe` needs PowerShell and prefers a **script file**; a long inline `bash -lc "…"` mangles.
- `cargo test --release -p nvs-abi-probe` takes over two minutes — background it.
- `python`, not `python3`.
- Another agent may be editing this repo concurrently: stage your own paths explicitly and check
  `git show --stat` after committing.
