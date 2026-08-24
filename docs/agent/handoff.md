# Next session prompt

## State

**A new loop goal is set, and its acceptance list fails by design.** The previous loop reached the old
list; that list was a threshold, not a milestone, and it stopped with `Core` at 39 of ~205 spec member
rows. The new goal is **M4S Part I in full — spec §§ 1–12 — plus the M4 surface it cannot be written
without**: [`loop-goal.md`](loop-goal.md) is the prose and the standing decisions,
[`loop-goal.toml`](loop-goal.toml) the checks. Read both before starting; every design call this loop
reaches is already pre-authorized there, so **`BLOCKED` should not be needed**.

**Stage 1 of the new list is the old list, unchanged, and passes today** — it is a non-regression floor,
never traded for anything above it. Stages 2 and 3 add seven new fixtures that do not compile yet:
`nullable.mwl`, `match.mwl`, `text.mwl`, `numbers.mwl`, `dates.mwl`, `json.mwl`, `collect.mwl`. Their
expected output is frozen; their *source* is a reading of the spec by someone who could not compile it, so
correcting a signature, an enum namespace or an options bag in one is a bug fix, not a decision.

**Nothing else changed.** No Rust was touched this session — the plan's status block, the two spec folds
(`Uri::parseQuery`, `levenshtein`) and the goal files are the whole diff.

## Next

**The keystone: `?T` and the `mixed` runtime tag, decided as one design.** `mwl-ir`'s gap 3 is the whole
blocker — no nullable representation, no `null`, and `mixed` erases but does not dispatch. Every
`?T`-returning member in all twelve spec sections waits on it, and so does ADR 0066's `as ?T`, `??` and
`?->`. `examples/nullable.mwl` is the gate that says it works end to end. Record the representation and
what it spends per value in `mwl-ir`'s module doc (IR half) and `mwl-runtime`'s (heap half) — a numbered
ADR is explicitly *not* wanted for it.

Then, in the order the fixtures gate them: a **variadic** parameter and a **union return** (`mwl-stdlib`'s
gap 3 and `CoreTy::Union`'s docs), **compound assignment** (`mwl-ir` gap 16 — a desugar), **`for`/`switch`/
`match`** (gap 1 — every terminator already exists), **`decimal`'s IR** (gap 15), and **ADR 0070's duration
literal**, which `Core\Time\Duration::parse` shares an implementation with, so build the literal first.

Only then the breadth: `Core\Str` and `Core\Arr` to their last member, then §§ 3–12. **Every new `Core`
member owes a `.mwlt` case in the same session** — `every_part_one_member_has_a_conformance_case` fails
naming it otherwise. Stage 4 also names a test that **does not exist yet**,
`every_part_one_spec_member_is_registered`: it must read the member rows out of
[the spec](../spec/01-core-library.md) §§ 1–12 and fail naming every one with no registry entry. That test,
not a case count, is what "Part I is complete" means.

## Backlog

- **`private`/`protected` is not enforced at all**, and the reserved `Comparable`/`Stringable` interfaces
  carry no member signatures — `mwl-types`' own gap list. `Core\Heap`'s ordering needs the first,
  `Duration`'s `Stringable` the second, so both are likely to stop being backlog mid-loop.
- **An abandoned generator skips the `finally` it is suspended inside** — `mwl-ir` gap 18, the one PHP
  divergence the corpus has found and not closed.
- **`new` on an `abstract` class is not refused, and `$n->foo()` on a scalar receiver panics `mwl-ir`** —
  both missing `mwl-types` diagnostics.
- **`autoload` (ADR 0061) and ADR 0047's checker row** are re-opened M1/M2 slices, off this loop's path
  unless a fixture needs them — see M1's own section in [the plan](../implementation-plan.md).
- **`docs/spec/02-php-migration.md` is 31% classified**, one pass per PHP domain remaining
  (`python tools/check-migration.py`). It becomes a tested claim as §§ 1–12 land.
- **ADR 0078 is decided but unbuilt** — scheduled into M6/M7/M9, needs nothing before then.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of the kind AGENTS.md forbids —
  the one doc genuinely owed a trim, but never from inside the loop.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an apostrophe-heavy one can fail to parse at
  all. Use the Write tool, or `python tools/splice.py <target> <old> <new>` with both blocks written to
  `.agent-tmp/`. A throwaway `.agent-tmp/*.py` script run with `python` is fine for a bulk edit.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. Keep a PHP twin beside it.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
  makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
  against PHP while authoring, then drop the section or put the case in `tests/differential/`. The `.mwlt`
  format is `crates/mwl-test`'s module doc.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
- **The traps that cost the most time are not gaps**: `as` binds tighter than every binary operator, so
  write `($a > $b) as string`; a bare array literal in a `foreach` head types as `mixed`; a `foreach` key
  binding must be declared `string` even over a list; `Core\Str::length` and `Core\Arr::count` return
  `uint`; a `catch` binding is function-scoped **until this loop re-scopes it** (pre-authorized).
- **Another agent may be editing this repo at the same time.** Check the ADR directory for the next free
  number immediately before writing one, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it. `tools/brief.py` prints a loud
  banner when `.loop/running` exists — if it does, stop and tell the user rather than editing alongside it.
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed).
  The dependency *sweep* is a pass the user fires by hand (ADR 0068); never start it as a side effect.
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh` (background it; minutes). One fixture:
  `tools/leak-check.sh <paths>`. PHP 8.5 is on `PATH` under Windows but **not** inside WSL, deliberately.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>`; a renamed
  test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two minutes.
