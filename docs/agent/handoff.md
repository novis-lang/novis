# Next session prompt

## State

**Spec § 4 runs whole but its two component types.** `Core\Time\DateTime` (fourteen members over
`jiff::Zoned`), `Core\Unit` and `Core\Weekday`, `Core\Time::parse`/`at`, `$instant->in($zone)` and the
`Core\Time\Zone::UTC` constant all landed, over a new CLDR pattern module. `examples/dates.mwl` produces
its frozen output, so Stage 3 is **five** of its seven fixtures; `json.mwl` is now the first that fails.

- **`crates/mwl-stdlib/src/cldr.rs` is the pattern grammar** `DateTime::format` and `Time::parse` share —
  its own docs own the closed letter subset, the English root locale (there is no `setlocale`), CLDR
  quoting, and why `icu` was not taken. A letter outside the subset is a diagnostic naming itself.
- **`crates/mwl-stdlib/src/time.rs` still owns all of § 4.** A `DateTime` is three slots — an `Instant`'s
  two plus a `Zone`'s one — and `DATETIME`'s own docs own why that beats seven civil fields. Its gap list
  is what § 4 still owes.
- **A `Core` class constant may now be an *instance*.** `registry::Const::Built` names the member that
  builds one and its constant arguments; `mwl_types::ConstArg::Built` carries it, and
  `mwl_ir::lower::emit_const_arg` inlines the `InstKind::CoreCall` at the use site (and releases the
  arguments, which is where the one leak this session found was).
- Two semantic calls, both recorded where they live: `difference` counts in the **receiver's** zone (spec
  § 4's row now says so), and a *parse* pattern refuses a zone-naming field because `Time::parse` takes
  the zone as its own third argument.
- Verified: `cargo build`/`test`/`clippy`/`fmt` green, 401 `.mwlt` cases pass, and
  `tools/leak-check.sh` is clean over `examples/dates.mwl` plus a fifty-iteration fixture exercising every
  new refcount edge. `python tools/loop.py --goal-only` reaches `examples/json.mwl`.

## Next

**Spec § 6, `Core\Json`** — `examples/json.mwl` is the frozen check and wants `encode`, `decode`,
`decodeAs<T>` and a validity predicate, with a decode failure reporting **every** bad field at once
([ADR 0071](../adr/0071-derived-codecs.md) § 5). Pick the crate under
[ADR 0051](../adr/0051-standard-library-tiers.md) § 4 and record the pick in the module's own docs; a new
dependency owes the three things `AGENTS.md` names. The `?T`/`mixed` representation a decoded value needs
already exists (`mwl_ir::Ty::Tagged`).

## Backlog

- **`Core\Time\Date`, `Core\Time\TimeOfDay`, `Core\Month`, and `DateTime::date`/`timeOfDay`/`withTime`** —
  `time.rs`'s gap 1; the machinery all exists now, so each is a registry row and a body.
- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all four
  declare the variadic that exists, so each is a registry row and a body.
- **`Arr::diff`/`intersect`** — want a `Core\SetOn { Values, Keys, Both }` in `registry::ENUMS` and an
  `{on?, by?, comparator?}` bag; `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **`==` over two enum operands does not lower** (`mwl-codegen`: "does not lower a `Eq` over
  representation Enum(Int)"), so a case comparison is written `($a as int) == ($b as int)` today.
- **`.` and `as string` over a `Stringable` object** (`mwl-ir` gap 12), and **`Core\Regex::compile`/
  `replaceWith`** (`regex.rs`'s gap 1).
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of a throwing `Core\Time::parse("nope", …)` inside a `try` lose 100
  blocks, two per string literal argument.

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
  label needs `r"..."`, or the backslash is an unknown escape. In a Python heredoc, an `r"""…"""` literal
  is what keeps a `\M`/`\T` in MWL prose from being a unicode-escape error.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes — so
  `format(string, mixed ...)` is `args: [2]`. **An instance member's receiver is argument slot 0 and is
  not in `params`**, so `plus(Duration)` is `args: [2]`. A mismatch is an index-out-of-bounds panic at
  the first call.
- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `mwl_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.mwlt` case that calls it — `tests/conformance_coverage.rs` fails
  `cargo test -p mwl-stdlib` without one. An instance member is covered by a case writing `->name(`.
- **Registering a `Core` class narrows `Core`'s blanket trust for that name.** An unregistered
  `Core\X::y()` is waved through by `mwl_hir::members`; once `X` is in `registry::CLASSES`, an unknown
  member on it is a diagnostic. So adding a class can turn a fixture that "compiled" into one that
  reports — which is the point, but check the fixtures that name it.
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
  A `Core\Time` case has the same hazard in a different place: never assert `Zone::system()`'s answer, and
  never assert a wall-clock value.
- **The traps that cost the most time are not gaps**: a `"%1$s"` template must be written in **single**
  quotes or the `$s` interpolates; `as` binds tighter than every binary operator *and* than unary minus,
  so write `($a > $b) as string` and `(0 - 3) as ?uint`; a duration literal used as a receiver needs
  parentheses (`(30s)->toSeconds()`); a `for` header takes *expressions* only, so the
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
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. Whole suite:
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
