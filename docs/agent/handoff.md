# Next session prompt

## State

**Every signature shape the spec writes can now be stated.** The last missing one was a **variadic**
parameter, and it landed end to end this session as `registry::CoreTy::Variadic`:

- **One ABI argument, not one per written argument.** `mwl_ir::lower::lower_variadic_tail` collects every
  argument from that parameter's position into a fresh `array<T>`, so a variadic member is an ordinary
  `args: [N]` helper. That variant's own doc comment owns the shape, the rejected alternative and the
  ownership rule; `mwl-ir`'s gap 8 is now only *named*/`...spread` arguments.
- **`MethodSig::required()` stops one short for a variadic signature**, and `check_args_typed` keeps the
  lower arity bound while dropping the upper one — `expected at least 1 argument(s)`.
- **`Core\Str::format` is the first row to declare one.** The whole `printf` grammar is
  `crates/mwl-stdlib/src/format.rs`, which owns the closed conversion list, what it refuses and why, and
  the one thing still owed (ADR 0057's compile-time check of a *literal* template).

Verified: `cargo build`/`test`/`clippy`/`fmt` green, 304 conformance + 86 differential cases, and
`tools/leak-check.sh` clean over a fifty-iteration fixture that passes the same refcounted local twice
into one variadic call. Every expectation in `format.rs`'s unit tests and in the differential case came
out of PHP 8.5's own `vsprintf` before it was written down.

## Next

**Narrowing a local through `!== null`.** It is the *only* thing `examples/text.mwl` still needs —
`if ($found !== null) { $found->group(1); }` is E0459 where `?->` works — so it is what turns the loop's
Stage 3 from three fixtures to four. `mwl_types::locals` is where a local's type lives, and
`mwl_types::expr`'s `Binary` arm is where the condition is checked; the shape to build is a per-branch
narrowing of a `Union` that drops its `null` member inside the `then` block. ADR 0066's own body is the
rule; `docs/agent/loop-goal.md` § *The gaps that actually sit on the path* names it.

## Backlog

- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1, none needing a new shape except a Unicode
  normalization dependency for `normalize`. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all four
  declare the variadic that now exists, so each is a registry row and a body.
- **`Arr::diff`/`intersect`** — want a `Core\SetOn { Values, Keys, Both }` in `registry::ENUMS` and an
  `{on?, by?, comparator?}` bag; `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **`Core\Regex::compile`/`replaceWith`** — § 5's last two, both stated on `Pattern`.
  `crates/mwl-stdlib/src/regex.rs`'s gap 1.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of `Core\Str::padStart("ab", 10, "")` inside a `try` lose 100 blocks.
- **Arithmetic, ADR 0035's truthy table and an array access over a `Ty::Tagged` operand still panic**, and
  **`do`/`while`, `$i++`/`$i--` and every bitwise operator do not lower** (`mwl-ir` gaps 1, 15, 16).

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
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes — so
  `format(string, mixed ...)` is `args: [2]`. **An instance member's receiver is argument slot 0 and is
  not in `params`**, so `group(int|string)` is `args: [2]`. A mismatch is an index-out-of-bounds panic at
  the first call.
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
- **A `mwl-types` test that asserts an interned type's `describe` string is fragile.** A union orders its
  members by type id, so registering a member anywhere can flip `T|null` to `null|T`. Compare against
  `interner.make_union([...])` instead.
- **`MwlStr::from_raw`/`MwlArray::from_raw` return an *owning* handle.** Reading a refcount through one in
  a unit test releases a reference when it drops — wrap it in `std::mem::ManuallyDrop`, or the test ends in
  a heap corruption rather than an assertion failure. `crate::arr::borrowed` is that wrapper for an
  argument, and `crate::instance::slot` is the borrowed read of an object's slot.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
- **The traps that cost the most time are not gaps**: a `"%1$s"` template must be written in **single**
  quotes or the `$s` interpolates; `as` binds tighter than every binary operator *and* than unary minus,
  so write `($a > $b) as string` and `(0 - 3) as ?uint`; a `for` header takes *expressions* only, so the
  loop variable is declared on the line above it (`foreach (Core\Arr::range(…))` is usually shorter); a
  `foreach` binding declares a type (`as int $i`); `Core\Str::length` answers `uint`, so a running total
  it feeds must be one too, and `?? 0` against a `?uint` needs `?? 0 as uint` to stay one; `bool as
  string` is PHP's `""`/`"1"`; a bare array literal in a `foreach` head or a call argument types as
  `mixed`, and `var` refuses one outright — so `$m?->groups() ?? []` is `array<T>|array<mixed>` and
  `foreach` refuses it; a `foreach` key binding must be declared `string` even over a list; there is no
  int-to-float widening, so `Math::sqrt(2)` is a diagnostic and `2.0` is what a `float` parameter takes;
  a `catch` binding is function-scoped **until this loop re-scopes it** (pre-authorized); `Exception` is
  spelled `Core\Error`, a caught value's text is `$e->message` and not a getter, and a typed `catch` on a
  `Core` class does not lower yet — catch `Throwable` instead.
- **Clippy refuses a float literal that approximates π or e**, even in a test expectation derived from
  PHP — pick a different constant and re-derive the expected string rather than allowing the lint.
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
