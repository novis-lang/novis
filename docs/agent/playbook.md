# Playbook — the traps this repository has actually sprung

Hard-won specifics: things that cost a real session real time, written down so the next one does not
pay again. **This file is append-mostly.** Add a bullet when something bites you; edit one when it stops
being true; delete one when the underlying gap is closed. Never rewrite it wholesale, and never reword a
bullet to say the same thing differently — the churn is the cost this file exists to avoid.

It lived inside `handoff.md` until it was two thirds of that file, which meant every session regenerated
about 2.4k tokens of stable lore as if it were state, and reworded it a little each time. `handoff.md` is
*state* and is overwritten every session; this is *knowledge* and outlives all of them.

**Scope.** A rule that binds every agent goes in [AGENTS.md](../../AGENTS.md). A decision with reasoning
goes in an ADR. How a subsystem works goes in that crate's own module doc comment. The *shape* of
something you are about to write — a commit message, a `.mwlt` case, a `Core` member, an ADR — is
[conventions.md](conventions.md). What is left — "this looks like it should work and does not, and here
is why" — is this file.

## Tooling

- **`D:` fills up.** `target/debug` reached 33 GB and `cargo test` failed as a wall of `link.exe` 1180/1318
  errors — the real message (`no space on device`) only appears without a `Select-String` filter. `cargo
  clean` frees it in seconds; the rebuild is a few minutes. Check `Get-PSDrive D` before diagnosing a
  linker failure.
- **The Bash tool eats a backslash inside a heredoc**, and an em dash or apostrophe in one can defeat an
  exact-match splice — a `\\` written in a `python - <<'PY'` heredoc arrives as `\`, and a `"\n"` arrives
  as a real newline, so a block containing either will silently fail to match. Use the Write/Edit tools, or
  `python tools/splice.py <target> --patch <file>` with the patch **written by the Write tool**
  ([conventions.md](conventions.md) has the format). This is the rule sessions break most: reaching for
  `cat > f <<'EOF'` to save a call is how the mangling gets in. `splice.py` matches the anchor **exactly**,
  trailing newline included — the Write tool ends a file with one, so strip it when splicing
  mid-paragraph, and use `--dry-run` if you are unsure the anchor is still current. A Rust string holding
  a `Core\Name` label needs `r"..."`, or the backslash is an unknown escape.
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh` (background it; minutes). One fixture:
  `tools/leak-check.sh <paths>` — it takes `.mwl` files only, so a `.mwlt` passed to it reports a failure
  that is not a leak.
- **Another agent may be editing this repo at the same time.** Check the ADR directory for the next free
  number immediately before writing one, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it. `tools/brief.py` prints a loud
  banner when `.loop/running` exists — if it does, stop and tell the user rather than editing alongside it.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>`; a renamed
  test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two minutes.

## Running things

- **Verification is one call:** `python tools/verify.py` — build, test, clippy and fmt in order, stopping at
  the first failure, ~10 lines when green. `-p <crate>` scopes it, `--fast` drops clippy and fmt for a
  mid-work check, and every step's full output lands in `.agent-tmp/verify-<step>.log` either way.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point, and a `for` header takes
  *expressions* only, so the loop variable is declared on the line above it.

## Adding a `Core` member

- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `mwl_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.mwlt` case that calls it — `tests/conformance_coverage.rs` fails
  `cargo test -p mwl-stdlib` without one. An instance member is covered by a case writing `->name(`.
  [conventions.md](conventions.md) writes all four out; `python tools/brief.py`'s *anchors* block
  resolves each spelling to a file and line.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes. **An instance member's
  receiver is argument slot 0 and is not in `params`**, so `plus(Duration)` is `args: [2]`. A mismatch is
  an index-out-of-bounds panic at the first call.
- **A member on `registry::WRITTEN_CLASS_MEMBERS` takes one argument its row does not declare** — the
  class its call site wrote, in slot 0 — so its helper's `args: [N]` is `params` + 1 (+ the options bag's
  flattening). `tests/conformance_coverage.rs` looks for such a member spelled `Class::name<`, not
  `Class::name(`, because that is what every call site writes.
- **Registering a `Core` class narrows `Core`'s blanket trust for that name.** An unregistered
  `Core\X::y()` is waved through by `mwl_hir::members`; once `X` is in `registry::CLASSES`, an unknown
  member on it is a diagnostic. So adding a class can turn a fixture that "compiled" into one that
  reports — which is the point, but check the fixtures that name it.
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed). A
  license identifier new to the tree must be added to **both** `deny.toml`'s allow list and
  `tools/gen-attribution.py`'s `PREFERENCE`, in the same commit: that script fails if the two disagree.
  The dependency *sweep* is a pass the user fires by hand (ADR 0068); never start it as a side effect.

## Writing a test case

- **Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
  makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
  against PHP while authoring (`php` is on the Windows `PATH`; `php -r '…'` is enough to settle a
  semantics question), then drop the section or put the case in `tests/differential/`. The `.mwlt` format
  is `crates/mwl-test`'s module doc; a `--EXPECTF-ERROR--` block must reproduce the diagnostic's own
  indentation, which widens with the line number. A trailing space before a `\n` is unreliable in an
  `--EXPECT--` block — echo a sentinel character after it.
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
- **Clippy refuses a float literal that approximates π or e**, and refuses `assert!` over two constants —
  a compile-time invariant belongs in `const _: () = assert!(…);`, not a `#[test]`.

## Writing MWL itself

**The traps that cost the most time are not gaps.** A `"%1$s"` template must be written in **single**
quotes or the `$s` interpolates; `as` binds tighter than every binary operator *and* than unary minus,
so write `($a > $b) as string` and `(0 - 3) as ?uint`; a duration literal used as a receiver needs
parentheses (`(30s)->toSeconds()`); a `foreach` binding declares a type (`as int $i`);
`Core\Str::length` answers `uint`, so a running total it feeds must be one too, and `?? 0` against a
`?uint` needs `?? 0 as uint` to stay one; `bool as string` is PHP's `""`/`"1"`; a bare array literal in
a `foreach` head types as `mixed`, and `var` refuses one outright; a `foreach` key binding must be
declared `string` even over a list; there is no int-to-float widening, so `Math::sqrt(2)` is a
diagnostic; a `catch` binding is function-scoped **until this loop re-scopes it** (pre-authorized), so
two clauses on one `try` need two different variable names; `Exception` is spelled `Core\Error` in the
spec's own prose but the tree's root is `Throwable`, a caught value's text is `$e->message` and not a
getter, and a typed `catch` on a `Core`-owned class does not lower yet — a `catch` on a spec § 10 class
(`ParseError`, `LogicError`, …) now does.
