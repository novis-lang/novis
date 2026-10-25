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
two ever disagree, the rule is right and this file is the bug.

## Before you start: which regime are we in?

Read the version in [Cargo.toml](../../Cargo.toml)'s `[workspace.package]`.

- **Below 0.1.0 — the current state.** Prototyping. Update anything, fix what breaks, commit. Do steps 1, 2,
  4 and 7 below and skip the rest: there is no classification, no ladder, no sign-off, and no release
  consequence (`rule:packaging/the-version-contract-starts-at-0-1-0`). Do not apply the contract rules "early" — that is how a prototype acquires a
  compatibility surface nobody agreed to.
- **0.1.0 or above.** The whole file applies.

## 1. Orient

```sh
looper orient --full                      # where the run and the work stand
git status                                # start clean; a sweep never rides on top of unrelated work
bun nv deps                               # every pin beside its latest stable release, the ones behind first
cargo update --dry-run                    # what would move inside the declared ranges
cargo tree --duplicates                   # multiple-versions warnings that a bump might fix or create
```

**`bun nv deps` is the sweep's list.** Each `behind` row is moved or held, each `expired` row is retried,
and the pass is over when `bun nv deps --check` exits 0. `tools/nv/cmd/deps.ts`'s head comment is the home
of what it reads and how it judges a pin. It reads these places, and `cargo update` sees only the first:

| Pin | File | A hold |
|---|---|---|
| Crate ranges | [Cargo.toml](../../Cargo.toml) `[workspace.dependencies]`, and each `Cargo.toml` under `extensions/`, `fuzz/`, `tools/`, `benches/` and the `nvs ext new` template | `# HOLD (revisit <date>): <reason>` directly above the row |
| npm packages, and Bun as `engines.bun` | `package.json` at the root, in `editors/vscode/` and in `website/` | `"holds": { "<package>": "HOLD (revisit <date>): <reason>" }`, because JSON has no comments |
| The Rust toolchain, components and targets | [rust-toolchain.toml](../../rust-toolchain.toml) | the comment form |
| CI actions, by SHA with the release in a comment, and `cargo install --version` tools | [.github/workflows/](../../.github/workflows/) | the comment form |
| Container images | `docker/Dockerfile`, the Dockerfiles and compose files under `benches/`, and `tests/db/compose.yaml` | the comment form |
| wasi-sdk | CI's `WASI_SDK_VERSION`, and [extensions/image/libwebp/SOURCE.json](../../extensions/image/libwebp/SOURCE.json), which `bun nv webp-lib` rebuilds from | the comment form, in the workflow |
| Everything else that has a version — PHP oracle, `cargo-fuzz`, valgrind, MSVC/SDK | [setup.md](../setup.md) § *Versions no manifest pins*, which is their only home | not read by `bun nv deps` |

Any `HOLD` whose date has passed is part of this sweep: retry it, and either drop the hold or give it a new
date and a reason that is still true.

**Latest stable means a final release**: never an `-rc`, `-alpha`, `-beta` or `-dev`, which is what
`bun nv deps` compares against. A server image tracks its vendor's LTS line, and the newer line gets a hold
naming it. **A workspace row no member manifest names is removed, not bumped.**

## 2. Move one thing at a time

**One dependency, one commit** (`rule:packaging/one-bump-one-commit`) — or one group that genuinely cannot move separately, such as
the four `cranelift-*` crates, which must also stay compatible with the pinned `wasmtime`
(`rule:packaging/an-extension-is-a-sandboxed-wasm-component`).

```sh
cargo update -p <crate>                   # inside the declared range
# for a semver-incompatible bump, edit the version in [workspace.dependencies] first, then:
cargo update -p <crate> --precise <version>
```

Each bump gets a narrow check, and is committed as soon as it passes:

```sh
cargo check --all-targets                 # the whole tree compiles against the moved crate
bun nv verify -p <crate>                  # each workspace crate that names the moved dependency
```

**The full gate runs once per group of bumps, never once per bump.** A moved `Cargo.lock` reaches every
build, so each `looper verify` is the whole suite, and the per-bump commits already give `git bisect` one
bump per step. Take every bump of the group you can this session, then run the gate once on the tip: `looper verify`
in the loop, `bun nv affected --run` anywhere else, and the § 3 legs the group reaches.
A red gate is fixed before the session ends: the failing test usually names the crate, and when it does
not, `git bisect` walks the bump commits. This is the one place a commit lands before the full gate, and
the user chose it for the sweep's speed.

## 3. The battery

Run **A** once per group of bumps, on the tip. Add the conditional legs the moved crate actually touches — over-running costs
minutes, under-running costs a silently retracted premise.

**A. Always.**

```sh
bun nv affected --run                     # a moved Cargo.lock reaches every build; looper verify in the loop
cargo deny check                          # cargo install cargo-deny --locked, if absent
bun nv gen-attribution                    # regenerate; commit the diff in the same commit
```

**B. If `cranelift-*`, `wasmtime`, `corosensei` or anything the runtime links moved** — the premise guards.
Takes over two minutes, so start it with `run_in_background` and end your turn to wait for its report:

```sh
cargo test --release -p nvs-abi-probe
cargo test --release -p nvs-abi-probe --features wasm-probe    # wasmtime moved
```

**C. If codegen, the runtime or the stdlib moved** — the end-to-end legs, both platforms plus the leak check.
CI runs every test on Linux at the push, and its nightly `valgrind` job runs every `examples/` program
under memcheck. Before the push, `bun nv linux-test` runs one filtered test on Linux, and
`tools/leak-check.sh` runs the programs you name under memcheck.

**D. If a lexer/parser-adjacent crate moved:**

```sh
wsl.exe -- bash -lc "cd /mnt/<drive>/<repo> && cargo +nightly fuzz run lex -- -max_total_time=300"
wsl.exe -- bash -lc "cd /mnt/<drive>/<repo> && cargo +nightly fuzz run parse -- -max_total_time=300"
```

**E. If the JIT backend moved,** refresh the callgrind instruction counts per
[AGENTS.md](../../AGENTS.md) § *Fuzzing and callgrind on Windows* and record the delta in the report. A
double-digit percentage move is a finding, not a footnote.

**F. If the PHP oracle version changed,** run `bun nv bench --check`. Every case's Novis and PHP twins must
print the same output, byte for byte, before a single PHP figure is trusted.

**Two rules that are not yours to bend** (`rule:packaging/a-dependency-move-proves-two-things`): a failing guard test in `benches/abi-probe/` is never
fixed by editing its threshold — the rule named in the test's own comment is what gets revisited, and the
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
5. **Overwrite the live goal's handoff** in the shape [conventions.md](conventions.md) § *A handoff* gives — unless a loop is mid-goal, in which case leave that record to the loop —
   then show the user the report below.
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
- `wsl.exe` prefers a **script file**; a long inline `bash -lc "…"` mangles. From Bash, put
  `MSYS_NO_PATHCONV=1` in front of a `wsl.exe` call that passes a POSIX path such as `/mnt/…`.
- `cargo test --release -p nvs-abi-probe` takes over two minutes: start it with `run_in_background`.
- Another agent may be editing this repo concurrently: stage your own paths explicitly and check
  `git show --stat` after committing.
