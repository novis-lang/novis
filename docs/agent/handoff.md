# Next session prompt

## State

**Stage 3 — `Core` Part I across spec §§ 1–12 — has spec § 3 whole and § 2's aggregations, and
`decimal` is a real type end to end.** ADR 0054's scalar is `mwl_ir::ty::Ty::Decimal` over
`mwl_runtime::decimal`; that module's own doc comment is the home for the layout, for why one
`Value` shape carries it (`Tag::Decimal`, mantissa spending the padding bytes) rather than a second
16-byte shape, and for what it spends. `Tag`/`Untag` are the **identity** on one, so a `mixed` or a
`?decimal` holds it with nothing to rebuild. Every § 3 operator and § 4 conversion is an
`ir::Helper`; three comparison helpers cover all six operators, which is what gives a `NaN` on the
`float` side PHP's answer to each.

Two things landed with it. `array<T>` is now **element-covariant on read**
(`mwl_types::expr::is_assignable` owns the rule and why copy-on-write makes it sound) — without it
no `array<int>` satisfies the `array<int|float|decimal>` the spec writes. And the checker now
**records** a numeric literal it placed at `decimal` (`mwl_types::expr::record_decimal_placement`),
which lowering reads back: `Ty::Array` erases the element type, so `array<decimal> $p = [19.99];`
would otherwise have stored a `float` where the checker said `decimal`.

Stage 3's seven fixtures stand at **three passing** (`core.mwl`, `report.mwl`, `numbers.mwl`).
`text.mwl` is the first that fails, on `Core\Regex`.

Verified: `cargo build`/`test`/`clippy`/`fmt` green, 293 conformance + 85 differential cases, and
`tools/leak-check.sh examples/numbers.mwl` clean — a `decimal` is not refcounted, so no new
refcount edge, but the fixture allocates a string per rendered value.

## Next

**`Core\Regex`, spec § 5** — `docs/spec/01-core-library.md` § 5 has the rows, and
[ADR 0056](../adr/0056-regex-engine-policy.md) settles the engine: `regex` as the linear-time
default *and* `fancy-regex` as the budgeted backtracking tier, both named by the user in
`loop-goal.md` § *Standing decisions*, so both land together and close that ADR rather than half of
it. `examples/text.mwl` is the fixture that unblocks; `Regex::split` is the member it reaches
first. A new dependency owes three things (AGENTS.md § *Commands*).

## Backlog

- **`Core\Str::slice` and `Arr::diff`/`intersect`** — the last of §§ 1–2 that need no new signature
  shape; `diff`/`intersect` want a `Core\SetOn { Values, Keys, Both }` enum in `registry::ENUMS` and
  an `{on?, by?, comparator?}` bag. `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **A variadic parameter** is the one signature shape left — `mwl-stdlib`'s gap 3 owns the member
  list, `mwl-ir`'s gap 8 the lowering half.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*; `Core\Str::padStart("ab", 10, "")` in a loop reproduces it under `tools/leak-check.sh`.
- **Arithmetic, ADR 0035's truthy table and an array access over a `Ty::Tagged` operand still
  panic** — each closes the way rendering did, with a `Helper` variant dispatching on the tag.
  `Core\Math::abs($x) < 1.0` is the shortest repro.
- **`do`/`while` and `$i++`/`$i--` do not lower**, and `<=>` lowers for no scalar operand at all
  (`mwl-ir` gaps 1, 15 and 16).
- **The bitwise operators have no `ir::BinOp` variant**, so `&`/`|`/`^`/`<<`/`>>`/`**` and their
  compound forms panic in lowering — `mwl-ir`'s gap 16.
- **A `decimal` parameter default is refused** — `mwl_types::defaults` says why, and now needs only
  a `ConstArg` variant carrying `InstKind::ConstDecimal`'s three parts.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an em dash or apostrophe in one can defeat an
  exact-match splice — a `\\` written in a `python - <<'PY'` heredoc arrives as `\`, and a `"\n"` arrives
  as a real newline, so a block containing either will silently fail to match. Use the Write tool, or
  `python tools/splice.py <target> <old> <new>` with both blocks written to `.agent-tmp/`.
  `splice.py` matches the anchor **exactly**, trailing newline included — the Write tool ends a file with
  one, so strip it from both blocks when splicing mid-paragraph. A Rust string holding a `Core\Name`
  label needs `r"..."`, or the backslash is an unknown escape.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A mismatch is an index-out-of-bounds panic at the first call, not a build error.
- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `mwl_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.mwlt` case that calls it — `tests/conformance_coverage.rs` fails
  `cargo test -p mwl-stdlib` without one.
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
  describes as `string|null`; there is no int-to-float widening, so `Math::sqrt(2)` is a diagnostic and
  `2.0` is what a `float` parameter takes; a `catch` binding is function-scoped **until this loop re-scopes
  it** (pre-authorized); `Exception` is spelled `Core\Error`, a caught value's text is `$e->message` and
  not a getter, and a typed `catch` on a `Core` class does not lower yet — catch `Throwable` instead.
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
