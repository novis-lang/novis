# Next session prompt

## State

**The `Core`-owned instance is built, and `Core\Regex\Match` is the first one.** A `Core` class may now
carry an `instance` member roster and a `slots` layout (`mwl_stdlib::registry::CoreClass`), a
`CoreTy::Instance` names one as a type, and `crates/mwl-stdlib/src/instance.rs` is the value behind it —
that module's own doc comment is the home for the two decisions it records (a `Core` instance is an
ordinary MWL object; the descriptors are one leaked `ClassTable` per core) and what each spends. The
checker seeds an instance member with `is_static: false` and `mwl-ir`'s `MethodCall` arm lowers it to the
same `InstKind::CoreCall` a static `Core` call uses, with the receiver in argument slot 0.

§ 5 is now six of eight members: `match` (`?Match`) and `matchAll` (`array<Match>`) joined the four scalar
ones, and `Match` answers `group`/`groups`/`offset`/`text`. `crates/mwl-stdlib/src/regex.rs` owns what a
`Match` holds and why it is materialized eagerly; spec § 5 now states the four signatures.

Two holes this closed on the way, each with a reject case: `Core\Regex\Match::text()` written as a static
call used to reach the helper with an empty argument slice and abort the process (**E0458**), and `->` on a
nullable receiver used to panic `mwl-ir` (**E0459**).

Verified: `cargo build`/`test`/`clippy`/`fmt` green, 300 conformance + 85 differential cases, the whole WSL
leg including its valgrind sweep, and `tools/leak-check.sh` clean over a fixture that loops fifty times
through every new refcount edge — including a freshly built receiver, which the borrow rule makes this
frame's to release.

## Next

**`Core\Str::wrap`/`reverse`/`format`/`indexOf`/`before`, then `!== null` narrowing.** Those five members
are the rest of `examples/text.mwl`, and none needs a new signature shape —
`docs/spec/01-core-library.md` § 1 has the rows. The fixture then still fails on one language hole: it
writes `if ($found !== null) { $found->group(1); }`, and nothing narrows a local's type through a
condition, so E0459 fires where `?->` would work. That narrowing is the smaller half of the two and it is
what every migrated PHP program writes; `mwl_types::locals` is where a local's type lives.

## Backlog

- **`Core\Str::slice` and `Arr::diff`/`intersect`** — the last of §§ 1–2 that need no new shape;
  `diff`/`intersect` want a `Core\SetOn { Values, Keys, Both }` enum in `registry::ENUMS` and an
  `{on?, by?, comparator?}` bag. `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **`Core\Regex::compile`/`replaceWith`** — § 5's last two, both stated on `Pattern`, which is a pattern
  plus four compilation flags reaching the cache key. `crates/mwl-stdlib/src/regex.rs`'s gap 1.
- **A variadic parameter** is the one signature shape left — `mwl-stdlib`'s gap 3 owns the member list,
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
  `args: [3]`. **An instance member's receiver is argument slot 0 and is not in `params`**, so
  `group(int|string)` is `args: [2]`. A mismatch is an index-out-of-bounds panic at the first call.
- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `mwl_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.mwlt` case that calls it — `tests/conformance_coverage.rs` fails
  `cargo test -p mwl-stdlib` without one. An instance member is covered by a case writing `->name(`.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
  makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
  against PHP while authoring (a `.php` twin under `.agent-tmp/`, `php` is on the Windows `PATH`), then
  drop the section or put the case in `tests/differential/`. The `.mwlt` format is `crates/mwl-test`'s
  module doc; a `--EXPECTF-ERROR--` block must reproduce the diagnostic's own indentation, which widens
  with the line number. A trailing space before a `\n` is unreliable in an `--EXPECT--` block — echo a
  sentinel character after it.
- **`MwlStr::from_raw`/`MwlArray::from_raw` return an *owning* handle.** Reading a refcount through one in
  a unit test releases a reference when it drops — wrap it in `std::mem::ManuallyDrop`, or the test ends in
  a heap corruption rather than an assertion failure. `crate::arr::borrowed` is that wrapper for an
  argument, and `crate::instance::slot` is the borrowed read of an object's slot.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
- **The traps that cost the most time are not gaps**: `as` binds tighter than every binary operator *and*
  than unary minus, so write `($a > $b) as string` and `(0 - 3) as ?uint`; a `for` header takes
  *expressions* only, so the loop variable is declared on the line above it (`foreach (Core\Arr::range(…))`
  is usually shorter); `Core\Str::length` answers `uint`, so a running total it feeds must be one too;
  `bool as string` is PHP's `""`/`"1"`; a bare array literal in a `foreach` head or a call argument types as
  `mixed`, and `var` refuses one outright — so `$m?->groups() ?? []` is `array<T>|array<mixed>` and
  `foreach` refuses it; a `foreach` key binding must be declared `string` even over a list; a union interns
  sorted by type id, so `?string` describes as `string|null`; there is no int-to-float widening, so
  `Math::sqrt(2)` is a diagnostic and `2.0` is what a `float` parameter takes; a `catch` binding is
  function-scoped **until this loop re-scopes it** (pre-authorized); `Exception` is spelled `Core\Error`, a
  caught value's text is `$e->message` and not a getter, and a typed `catch` on a `Core` class does not
  lower yet — catch `Throwable` instead.
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
