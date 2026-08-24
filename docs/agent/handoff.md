# Next session prompt

## State

**A call site can now write its own type argument.** `Core\Json::decodeAs<User>($body)` parses, resolves
and binds; `docs/spec/01-core-library.md` § 6's fourth member is no longer blocked on the *language*, only
on ADR 0071's derive. Stage 3 is unchanged at five of seven fixtures.

- **`mwl_stdlib::registry::CoreTy::Written` marks a variable the call site supplies** rather than one
  inferred from an argument, and `CoreMethod::written` is the one place their order comes from. That
  variant's own docs hold why the two kinds are separate; `MethodSig::type_params` is the checker's half
  and `mwl_types::expr::check_written_type_args` the rule (`E0441`/`E0442`, reusing the type-position
  codes). **No member declares one yet**, so every written list is refused today — which is what the new
  `tests/conformance/reject/a-written-type-argument-needs-a-member-that-declares-one.mwlt` asserts.
- **The `<` ambiguity is a checkpointed trial parse** in `Parser::parse_call_type_args`: it commits only
  when the list parses with no diagnostic *and* a `(` follows, so `Foo::BAR < $c > $d` is untouched. ADR
  0007 § 3 is folded — "`array<T>` is parsed only in type position" was the claim this changes.
- Verified: `cargo build`/`test`/`clippy`/`fmt` green, 404 `.mwlt` cases pass. No runtime or codegen change,
  so no new refcount edge and no `valgrind` run.

## Next

**ADR 0071's `#[Json\Derive]`** — the pass that makes `decodeAs<T>` have something to decode *into*, and
with it `examples/json.mwl`. [ADR 0071](../adr/0071-derived-codecs.md) § 1 is the nominal-match rule (a
closed `Core`-owned attribute-name list, matched after resolution), § 2 the field list and its
constructor-parameter requirement, § 5 the accumulating decoder. Register `Core\Json::decodeAs` with
`CoreTy::Written("T")` in the same slice, and spec § 10's `issues: array<Core\Issue>` on `ParseError`
alongside it — § 5 makes that the one throw carrying every failed field.

## Backlog

- **`Core\Time\Date`, `Core\Time\TimeOfDay`, `Core\Month`, and `DateTime::date`/`timeOfDay`/`withTime`** —
  `time.rs`'s gap 1; the machinery all exists, so each is a registry row and a body.
- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all four
  declare the variadic that exists, so each is a registry row and a body.
- **`Arr::diff`/`intersect`** — want a `Core\SetOn { Values, Keys, Both }` in `registry::ENUMS` and an
  `{on?, by?, comparator?}` bag; `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **The rest of `Core` still throws `RuntimeError` for everything** — `Core\Math`'s overflow and
  zero-divisor rows are spec § 10's `ArithmeticError`, a `Core\Regex` budget exhaustion arguably
  `TimeoutError`. One pass per module, each a `thrown` → `thrown_as`.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of a throwing `Core\Json::decode("{oops}")` inside a `try` lose 50
  blocks, one per string literal argument.
- **`==` over two enum operands does not lower** (`mwl-codegen`), so a case comparison is written
  `($a as int) == ($b as int)` today.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **`D:` fills up.** `target/debug` reached 33 GB and `cargo test` failed as a wall of `link.exe` 1180/1318
  errors — the real message (`no space on device`) only appears without a `Select-String` filter. `cargo
  clean` frees it in seconds; the rebuild is a few minutes. Check `Get-PSDrive D` before diagnosing a
  linker failure.
- **The Bash tool eats a backslash inside a heredoc**, and an em dash or apostrophe in one can defeat an
  exact-match splice — a `\\` written in a `python - <<'PY'` heredoc arrives as `\`, and a `"\n"` arrives
  as a real newline, so a block containing either will silently fail to match. Use the Write/Edit tools, or
  `python tools/splice.py <target> <old> <new>` with both blocks written to `.agent-tmp/`.
  `splice.py` matches the anchor **exactly**, trailing newline included — the Write tool ends a file with
  one, so strip it from both blocks when splicing mid-paragraph. A Rust string holding a `Core\Name`
  label needs `r"..."`, or the backslash is an unknown escape. `python -c` with `chr(92)` for a backslash
  is the reliable way to patch a file that contains one.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point, and a `for` header takes
  *expressions* only, so the loop variable is declared on the line above it.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes. **An instance member's
  receiver is argument slot 0 and is not in `params`**, so `plus(Duration)` is `args: [2]`. A mismatch is
  an index-out-of-bounds panic at the first call.
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
- **The traps that cost the most time are not gaps**: a `"%1$s"` template must be written in **single**
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
- **Clippy refuses a float literal that approximates π or e**, and refuses `assert!` over two constants —
  a compile-time invariant belongs in `const _: () = assert!(…);`, not a `#[test]`.
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
  `tools/leak-check.sh <paths>` — it takes `.mwl` files only, so a `.mwlt` passed to it reports a failure
  that is not a leak.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>`; a renamed
  test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two minutes.
