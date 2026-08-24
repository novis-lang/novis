# Next session prompt

## State

**Strict identity is defined, and the five members that compare are registered.**
`mwl_runtime::value_identical` is the one comparison `contains`, `keyOf`, `unique`, `diff`, `intersect`
and every future `ObjectSet` ask; `mwl_runtime::value_hash` is the hash that agrees with it, so a set
member indexes instead of scanning. What identity *means* — an object is itself and nothing else, `int`
and `uint` are one integer domain, an array is compared entry by entry in order, `NaN` matches nothing —
is `crates/mwl-runtime/src/identity.rs`'s own module doc, which is the home
[`loop-goal.md`](loop-goal.md) § *Standing decisions* names for that call. `Core\Arr` gained `contains`,
`keyOf`, `unique`, `min` and `max`, each with three conformance cases under `tests/conformance/core/`.

Verification passed: `cargo build`/`test`/`clippy`/`fmt` green, 350 cases through `mwl test tests/`, and
`tools/leak-check.sh` reports zero definite losses over a fixture that exercises every new retain edge
inside a loop body. Stage 2 of [`loop-goal.toml`](loop-goal.toml) now stops one line further into
`examples/nullable.mwl`, on `"42" as ?int`.

## Next

**Lower ADR 0066's `as ?T`** — `mwl-ir`'s gap 4, the *checked* conversion rows (`Str as Int` and its
siblings) in a non-throwing form that yields `null` where the throwing one would throw. It is what
`examples/nullable.mwl` stops on, so it is the whole of Stage 2's remaining distance, and the
representation it lands in already exists: `mwl_ir::Ty::Tagged`, with `Tag`/`IsNull` to build and test the
answer. [ADR 0066](../adr/0066-nullable-conversion-operator.md) owns the semantics.

## Backlog

- **`Core\Arr::diff`/`intersect`** — the last two set members; they need a `Core\SetOn { Values, Keys,
  Both }` enum in `registry::ENUMS` and an `{on?, by?, comparator?}` bag, both shapes the registry can
  already state. `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **A `?T` parameter defaulting to `null` is untried** — `Core\Str::slice`'s `?int $length = null`, the
  spec's most common optional shape. `mwl-stdlib`'s gap 3 says every piece is in place.
- **A variadic parameter, and `CoreTy::Decimal`** — the two signature shapes still missing, blocking ADR
  0069's combination members and `Arr::sum`/`product`/`average`. `mwl-stdlib`'s gap 3 names both sets.
- **`?->` does not lower on either side** — `mwl-ir` gap 6, also in `examples/nullable.mwl`. It is one
  `IsNull` over the receiver plus the branch `lower_coalesce` already builds.
- **`for`/`switch`/`match`, compound assignment and `decimal`'s IR** — `mwl-ir` gaps 1, 16 and 15, all
  in scope per [`loop-goal.md`](loop-goal.md). `examples/match.mwl` needs the first two.
- **`private`/`protected` is not enforced at all**, and `Comparable`/`Stringable` carry no member
  signatures — `mwl-types`' own gap list. `Core\Heap` needs the first, `Duration` the second.
- **An abandoned generator skips the `finally` it is suspended inside** — `mwl-ir` gap 18, the one PHP
  divergence the corpus has found and not closed.

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
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
  makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
  against PHP while authoring, then drop the section or put the case in `tests/differential/`. The `.mwlt`
  format is `crates/mwl-test`'s module doc.
- **`MwlStr::from_raw`/`MwlArray::from_raw` return an *owning* handle.** Reading a refcount through one in
  a unit test releases a reference when it drops — wrap it in `std::mem::ManuallyDrop`, or the test ends in
  a heap corruption rather than an assertion failure.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
- **The traps that cost the most time are not gaps**: `as` binds tighter than every binary operator, so
  write `($a > $b) as string`; `bool as string` is PHP's `""`/`"1"`, not `"false"`/`"true"`; a bare array
  literal in a `foreach` head or a call argument types as `mixed`; a `foreach` key binding must be declared
  `string` even over a list; `Core\Str::length` and `Core\Arr::count` return `uint`; `Core\Str::join` takes
  an `array<string>`, so an `array<int>` needs a `map` first; a union interns sorted by type id, so
  `?string` describes as `string|null`; a `catch` binding is function-scoped **until this loop re-scopes
  it** (pre-authorized); `Exception` is spelled `Core\Error` and a typed `catch` on a `Core` class does not
  lower yet.
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
