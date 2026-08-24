# Next session prompt

## State

**Stage 2 of the acceptance list passes whole.** `switch` and `match` lower
(`mwl_ir::lower::Lowering::lower_switch`/`lower_match`, each with its own doc comment), so
`examples/match.mwl` produces its frozen output and the loop is on **Stage 3 — `Core` Part I across
spec §§ 1–12**. `mwl-ir`'s gap 1 is now `do`/`while` alone.

Both constructs are an equality chain of `Terminator::Branch`es rather than `Terminator::Switch`,
which selects on an integer while a label is any expression of the subject's type. `switch` pushes a
`LoopFrame` whose new `continue_target` is `None` — it owns `break` and nothing else — so a bare
`continue` inside one continues the **enclosing loop**, PHP's `continue 2`. That divergence is
recorded in `docs/adr/README.md` § *Decisions taken at project start* and pinned by
`tests/differential/lang/a-switch-and-a-match-agree-with-php.mwlt`.

Verification passed: `cargo build`/`test`/`clippy`/`fmt` green, 364 cases through `mwl test tests/`
(280 conformance + 84 differential), six new IR snapshots, and `tools/leak-check.sh` clean over four
fixtures covering every new refcount edge (a retained `string` subject, a case-body local released on
a fall-through, a `break` and a `return` out of a case body, a fresh vs. aliasing `match` subject).

## Next

**Stage 3: `Core` §§ 3–12 have no registry class at all.** Read `docs/agent/loop-goal.toml`'s Stage 3
block for the fixture per domain and its frozen output, and `docs/spec/01-core-library.md` for the
member rows. The two signature shapes still missing gate part of it: a **variadic** parameter (ADR
0069's `overlay`/`underlay`/`appendAll`, `Arr::append`/`prepend`, `Path::join`) and `CoreTy::Decimal`
(`Arr::sum`/`product`/`average`) — `mwl-stdlib`'s gap 3 names both sets. Picking whichever § 3–12
class the next fixture needs and registering it end-to-end is the smallest slice that moves Stage 3.

## Backlog

- **`Core\Arr::diff`/`intersect`** — the last two set members; they need a `Core\SetOn { Values, Keys,
  Both }` enum in `registry::ENUMS` and an `{on?, by?, comparator?}` bag, both shapes the registry can
  already state. `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **`do`/`while` and `$i++`/`$i--` do not lower** — the first is `lower_while` with the branch moved
  below the body, the second is `lower_compound_assignment` with a synthesized `1`, but `mwl_types`
  types an inc/dec as its operand and checks no target, so that half is owed first.
- **The bitwise operators have no `ir::BinOp` variant**, so `&`/`|`/`^`/`<<`/`>>`/`**` and their
  compound forms all panic in lowering — `mwl-ir`'s gap 16. PHP throws `ArithmeticError` on a
  negative shift.
- **ADR 0066 § 3's refusals are `mwl_types`' half and are not built** — a conversion that cannot fail
  (`$i as ?string`) and one that does not exist (`$arr as ?int`) both reach `mwl-ir` and panic naming
  that ADR where a diagnostic belongs.
- **A `?T` parameter defaulting to `null` is untried** — `Core\Str::slice`'s `?int $length = null`,
  the spec's most common optional shape. `mwl-stdlib`'s gap 3 says every piece is in place.
- **`private`/`protected` is not enforced at all**, and `Comparable`/`Stringable` carry no member
  signatures — `mwl-types`' own gap list. `Core\Heap` needs the first, `Duration` the second.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of the kind AGENTS.md
  forbids — the one doc in the repo genuinely owed a trim.

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
  `splice.py` matches the anchor **exactly**, trailing newline included — the Write tool ends a file with
  one, so strip it from both blocks when splicing mid-paragraph.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
  makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
  against PHP while authoring (a `.php` twin under `.agent-tmp/`, `php` is on the Windows `PATH`), then
  drop the section or put the case in `tests/differential/`. The `.mwlt` format is `crates/mwl-test`'s
  module doc; a `--EXPECTF-ERROR--` block must reproduce the diagnostic's own indentation, which widens
  with the line number. A trailing space before a `\n` is unreliable in an `--EXPECT--` block — echo a
  sentinel character after it.
- **`MwlStr::from_raw`/`MwlArray::from_raw` return an *owning* handle.** Reading a refcount through one in
  a unit test releases a reference when it drops — wrap it in `std::mem::ManuallyDrop`, or the test ends in
  a heap corruption rather than an assertion failure.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
- **The traps that cost the most time are not gaps**: `as` binds tighter than every binary operator *and*
  than unary minus, so write `($a > $b) as string` and `(0 - 3) as ?uint`; a `for` header takes
  *expressions* only, so the loop variable is declared on the line above it; `bool as string` is PHP's
  `""`/`"1"`; a bare array literal in a `foreach` head or a call argument types as `mixed`; a `foreach` key
  binding must be declared `string` even over a list; `Core\Str::length` and `Core\Arr::count` return
  `uint`, and `$u ?? -1` therefore unions to a `mixed` neither `.` nor `echo` can render — give a sentinel
  the target's own type; `Core\Str::join` takes an `array<string>`; a union interns sorted by type id, so
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
