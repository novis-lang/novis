# Next session prompt

## State

**Stage 3 — `Core` Part I across spec §§ 1–12 — now has its first section outside §§ 1–3: `Core\Regex`.**
[ADR 0056](../adr/0056-regex-engine-policy.md)'s two tiers are both bound, in `crates/mwl-stdlib/src/regex.rs`
— that module's own doc comment is the home for why `regex` and `fancy-regex` rather than a C engine, for
the thread-local compiled-pattern cache and what it spends, and for the three gaps § 5 still has. The tier
is chosen by the pattern and never by the caller; exhausting the backtracking budget **throws**, which is
the whole point of that ADR and is held by
`tests/conformance/core/regex-tiers-and-the-backtracking-budget.mwlt`.

Four of § 5's eight members are registered: `matches`, `replace`, `split`, `quote`. The other four —
`compile`, `match`, `matchAll`, `replaceWith` — are all stated in terms of a **`Core`-owned instance**
(`Pattern`, `Match`), and `registry::CoreClass` has no representation for a value of a `Core` class and no
dispatch for a method called on one. That single missing capability is now `mwl-stdlib`'s gap 3 and covers
§ 4's four time types, § 9's three collections and § 12's `Uri` as well.

Stage 3's seven fixtures still stand at **three passing** (`core.mwl`, `report.mwl`, `numbers.mwl`).
`examples/text.mwl` now fails on seven names rather than on the engine: `Regex::match`/`matchAll`, and
`Core\Str::wrap`/`reverse`/`format`/`indexOf`/`before`.

Verified: `cargo build`/`test`/`clippy`/`fmt` green, 296 conformance + 85 differential cases,
`cargo deny check` and `python tools/gen-attribution.py` re-run for the new dependency, and
`tools/leak-check.sh` clean over a fixture calling all four members.

## Next

**The `Core`-owned instance** — one capability, four spec sections. `mwl_types::error_lib` already seeds a
`Core`-owned class the checker resolves properties and methods on, so the missing half is the *value*: a
`registry::CoreTy` variant naming a `Core` class, what a native helper returns for one, and how
`$match->group(1)` reaches native code. `Core\Regex`'s `Match` is the smallest first subject — four
accessors over data the engine already produced — and it unblocks `match`/`matchAll` and half of
`examples/text.mwl`. `crates/mwl-stdlib/src/regex.rs`'s gap 1 states what those four members need.

## Backlog

- **`Core\Str::wrap`/`reverse`/`format`/`indexOf`/`before`** — the other half of `examples/text.mwl`, and
  none needs a new signature shape. `docs/spec/01-core-library.md` § 1 has the rows.
- **`Core\Str::slice` and `Arr::diff`/`intersect`** — the last of §§ 1–2 that need no new shape;
  `diff`/`intersect` want a `Core\SetOn { Values, Keys, Both }` enum in `registry::ENUMS` and an
  `{on?, by?, comparator?}` bag. `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **A variadic parameter** is the other signature shape left — `mwl-stdlib`'s gap 3 owns the member list,
  `mwl-ir`'s gap 8 the lowering half.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*; `Core\Str::padStart("ab", 10, "")` in a loop reproduces it under `tools/leak-check.sh`.
- **Arithmetic, ADR 0035's truthy table and an array access over a `Ty::Tagged` operand still panic** —
  each closes the way rendering did, with a `Helper` variant dispatching on the tag.
  `Core\Math::abs($x) < 1.0` is the shortest repro.
- **`do`/`while`, `$i++`/`$i--` and every bitwise operator do not lower** (`mwl-ir` gaps 1, 15, 16), and
  `<=>` lowers for no scalar operand at all.

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
  `""`/`"1"`; a bare array literal in a `foreach` head or a call argument types as `mixed`, and `var` refuses
  one outright; a `foreach` key binding must be declared `string` even over a list; a union interns sorted by
  type id, so `?string` describes as `string|null`; there is no int-to-float widening, so `Math::sqrt(2)` is
  a diagnostic and `2.0` is what a `float` parameter takes; a `catch` binding is function-scoped **until this
  loop re-scopes it** (pre-authorized); `Exception` is spelled `Core\Error`, a caught value's text is
  `$e->message` and not a getter, and a typed `catch` on a `Core` class does not lower yet — catch
  `Throwable` instead.
- **Another agent may be editing this repo at the same time.** Check the ADR directory for the next free
  number immediately before writing one, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it. `tools/brief.py` prints a loud
  banner when `.loop/running` exists — if it does, stop and tell the user rather than editing alongside it.
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed). A
  license identifier new to the tree must be added to **both** `deny.toml`'s allow list and
  `tools/gen-attribution.py`'s `PREFERENCE`, in the same commit: that script fails if the two disagree.
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
