# Next session prompt

## State

**Stage 2 passes whole and the loop is on Stage 3 — `Core` Part I across spec §§ 1–12.** Stage 3's
seven fixtures stand at two passing (`examples/core.mwl`, `examples/report.mwl`); the other five each
name a class that has no registry entry at all — `Regex` (text), `Math` (numbers), `Time` (dates),
`Json` (json), `ObjectSet`/`ObjectMap` (collect).

**A tagged value now renders.** `.`, `echo` and `as string` over a `mixed`, a `?T` or any other union
all reach one `mwl_ir::ir::Helper::TaggedToString`, which picks ADR 0007 § 2's row from the tag out of
line in `mwl_runtime::value_to_string` and throws where no row exists (array, object, closure,
resource). That unblocks every `Core` member whose spec signature returns a union — `Math::abs`,
`Arr::sum` and the rest — because a fixture could not previously print one. `mwl_ir::ty::Ty::Tagged`'s
own doc comment is the home for what compiled code may and may not do with a tag.

Verification passed: `cargo build`/`test`/`clippy`/`fmt` green, 366 cases through `mwl test tests/`
(281 conformance + 85 differential), and `tools/leak-check.sh` clean over a fixture exercising all four
new refcount edges (a retained string payload handed back as the result, a fresh tagged operand
released after the helper read it, an aliasing one that must not be, and the throwing row's error path).

## Next

**Register `Core\Math`** — spec § 3, `docs/spec/01-core-library.md:331`. It is the natural next class:
pure arithmetic, no outside dependency, and `examples/numbers.mwl` is its fixture. Two of its rows need
work that is not a registry line: the `PI`/`TAU`/`E`/`INT_MAX`/… **constants** have no mechanism at all
(`registry::CoreClass` holds `methods` and nothing else), and `round`'s `{mode?: RoundMode}` needs a
`Core\RoundMode` entry in `registry::ENUMS`. Everything `decimal` appears in — `abs`/`ceil`/`floor`/
`truncate`/`round`/`format`'s widest overload — registers at `int|float` today and widens when
`CoreTy::Decimal` lands; say so in `mwl-stdlib`'s gap 3 rather than leaving the row out.

## Backlog

- **`Core\Arr::diff`/`intersect`** — the last two set members; they need a `Core\SetOn { Values, Keys,
  Both }` enum in `registry::ENUMS` and an `{on?, by?, comparator?}` bag, both shapes the registry can
  already state. `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **Arithmetic, ADR 0035's truthy table and an array access over a `Ty::Tagged` operand still panic** —
  each closes the way rendering just did, with a `Helper` variant dispatching on the tag. That variant's
  own doc comment lists them.
- **`do`/`while` and `$i++`/`$i--` do not lower** — the first is `lower_while` with the branch moved
  below the body, the second is `lower_compound_assignment` with a synthesized `1`, but `mwl_types`
  types an inc/dec as its operand and checks no target, so that half is owed first.
- **The bitwise operators have no `ir::BinOp` variant**, so `&`/`|`/`^`/`<<`/`>>`/`**` and their
  compound forms all panic in lowering — `mwl-ir`'s gap 16. PHP throws `ArithmeticError` on a
  negative shift.
- **ADR 0066 § 3's refusals are `mwl_types`' half and are not built** — a conversion that cannot fail
  (`$i as ?string`) and one that does not exist (`$arr as ?int`) both reach `mwl-ir` and panic naming
  that ADR where a diagnostic belongs.
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
  binding must be declared `string` even over a list; a union interns sorted by type id, so `?string`
  describes as `string|null`; a `catch` binding is function-scoped **until this loop re-scopes it**
  (pre-authorized); `Exception` is spelled `Core\Error` and a typed `catch` on a `Core` class does not
  lower yet — catch `Throwable` instead.
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
