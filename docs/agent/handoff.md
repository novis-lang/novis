# Next session prompt

## State

**The keystone is built.** `mixed`, `?T` and every other union now share one representation —
`mwl_ir::Ty::Tagged`, a 16-byte `mwl_runtime::Value` in a register pair — with `Tag`/`Untag`/`IsNull` to
widen into it, narrow out of it and test it, and `Lowering::coerce` as the one place the first two are
emitted. That variant's own doc comment owns the decision and what it spends; `mwl-runtime`'s module doc
owns the heap half (which is: nothing is allocated). `??` and the literal `null` landed with it, and
`mwl-ir`'s gap 5 closed behind it, so a short-circuit composes in **any** nested position — `lower_expr`
and `lower_expr_top` are one function now.

Both legs are green, valgrind included: Stage 1 of [`loop-goal.toml`](loop-goal.toml) passes on Windows and
under WSL, 263 conformance and 82 differential cases pass, and `tools/leak-check.sh` reports zero definite
losses over two fixtures written for the new refcount edges. Stage 2 now stops on
`` `Core\Arr` has no member named `first` `` — a missing member, not a missing representation.

## Next

**Teach `mwl_stdlib::registry`'s `CoreTy` the three shapes a Part I signature needs, then land the members
that were waiting on them.** A `?T` return has a representation now but no way to be *written down*: that
enum has no nullable, no variadic parameter and no union return (its own module doc's *Known gap* names the
first). Each is a variant here plus a lowering arm in `mwl_types::core_lib` — `Nullable` interns as
`Union([Null, T])`, which is exactly what `mwl_ir::lower_checked_ty` already maps to `Ty::Tagged`.

With them: `Core\Arr::first`, `last`, `keyOf`, `firstKey`, `lastKey`, `find`, `findKey`, `min`, `max`,
`sum` and ADR 0069's `overlay`/`underlay`/`appendAll` — the set `examples/nullable.mwl` and
`examples/collect.mwl` gate. **Every new `Core` member owes a `.mwlt` case in the same session**;
`every_part_one_member_has_a_conformance_case` fails naming it otherwise.

## Backlog

- **ADR 0066's `as ?T` operator does not lower** — it needs `mwl-ir`'s gap 4, the *checked* conversion
  rows, in a non-throwing form. `examples/nullable.mwl` uses `"4x" as ?int`.
- **`?->` does not lower on either side** — `mwl-ir` gap 6. It is one `IsNull` over the receiver plus the
  branch `lower_coalesce` already builds.
- **A ternary whose branches lower to two different representations panics** — `mwl-ir` gap 5, all that is
  left in that slot. Closing it is an `ExprInfo` entry like `Coalesce`'s, plus `coerce` on each arm.
- **`private`/`protected` is not enforced at all**, and the reserved `Comparable`/`Stringable` interfaces
  carry no member signatures — `mwl-types`' own gap list. `Core\Heap` needs the first, `Duration` the
  second.
- **`for`/`switch`/`match`, compound assignment and `decimal`'s IR** — `mwl-ir` gaps 1, 16 and 15, all
  in scope per [`loop-goal.md`](loop-goal.md). `examples/match.mwl` needs the first two.
- **An abandoned generator skips the `finally` it is suspended inside** — `mwl-ir` gap 18, the one PHP
  divergence the corpus has found and not closed.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of the kind AGENTS.md forbids —
  the one doc genuinely owed a trim, but never from inside the loop.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an em dash or apostrophe in one can defeat an
  exact-match splice. Use the Write tool, or `python tools/splice.py <target> <old> <new>` with both blocks
  written to `.agent-tmp/`. A throwaway `.agent-tmp/*.py` script run with `python` is fine for a bulk edit.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. Keep a PHP twin beside it.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
  makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
  against PHP while authoring, then drop the section or put the case in `tests/differential/`. The `.mwlt`
  format is `crates/mwl-test`'s module doc.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
- **The traps that cost the most time are not gaps**: `as` binds tighter than every binary operator, so
  write `($a > $b) as string`; `bool as string` is PHP's `""`/`"1"`, not `"false"`/`"true"`; a bare array
  literal in a `foreach` head types as `mixed`; a `foreach` key binding must be declared `string` even over
  a list; `Core\Str::length` and `Core\Arr::count` return `uint`; a `catch` binding is function-scoped
  **until this loop re-scopes it** (pre-authorized); `Exception` is spelled `Core\Error` and a typed
  `catch` on a `Core` class does not lower yet.
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
