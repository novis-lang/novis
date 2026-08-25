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

- **`orient.py` prints less than `brief.py` on purpose, and the gap is a bug in the goal, not in the
  tool.** If it did not print a module, an ADR section, a convention shape or a playbook section you turned
  out to need, do not conclude the orientation is broken and re-run `brief.py` for everything — fetch the
  one thing, and say in the handoff which `[context]` field in `loop-goal.toml` was missing its selector.
  A manifest that nobody corrects becomes a manifest every session works around, which costs more than the
  wide orientation it replaced.
- **A whole ADR is about 7,000 tokens; one of its `###` sections is about 1,000.** `sed -n` between the
  heading and the next one, not `cat`. The same goes for a 1,100-line module: `grep -n` for the anchor
  first. This is the largest single line item in `loop-stats.py --attribute` every time it is measured.
- **Another session may be writing this tree right now, and `ls` will not tell you.** Two sessions once
  reached for the same ADR number on the same day: an ADR referenced `0084` and `0085` by name before those
  files existed, and the second author only noticed because `git status` showed them untracked. **Claim a
  number with `git status --short docs/adr/` and not with `ls` or `brief.py` alone**, immediately before
  creating the file. The same applies to committing: `git commit -a` sweeps in whatever the other session
  has in flight, which is not wrong — the tree is only internally consistent with all of it — but the
  commit message then describes half of what it contains, so say so in the message rather than letting
  `git log` imply one author.
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
- **A plan status field is *one line* on disk, however `orient.py` wrapped it.** `plan.py --get` hands back
  that single line; an anchor copied out of the orientation's re-wrapped rendering will never match, and
  `--set` then silently rewrites the field unchanged. Edit what `--get` produced, in place.
- **Making a type `pub` owes it a `#[derive(Debug)]`.** The workspace denies `missing_debug_implementations`,
  so promoting a private struct to the public API compiles and then fails at clippy — step 3 of four, after
  the tests have already run. Add the derive in the same edit as the `pub`, not after `verify.py` says so.
- **`/tmp` is not the same directory to Bash and to Python here.** A file written by `>` in the Bash tool is
  invisible to a `python -` heredoc in the same call, which resolves `/tmp` to `%TEMP%`. Stage a scratch
  file under `.agent-tmp/` — both halves agree on a repo-relative path.
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
- **`mwl run` printing the right output and exiting **127** is a heap corruption at teardown**, not a
  missing command: Windows reports a double release that way, with nothing on stderr. So check `$?` on a
  scratch run rather than reading the output and moving on — a refcount bug is otherwise completely silent
  until the WSL valgrind leg catches it. `catch (Core\Throwable $e)` and `$e->message` do not lower at
  file scope, so a scratch file probing a throw needs the class-method shape
  `tests/conformance/lang/a-lossy-conversion-throws.mwlt` uses.

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

- **A multi-file `.mwlt` case works now — but only the entry file's statements run.**
  `--FILE <relative/path>--` repeats and writes another file into the case's working directory
  (`crates/mwl-test`'s module doc), and `mwl-cli`'s `front_end` resolves, checks and lowers the whole
  `require`/`autoload` graph, so a class declared in a second file is reachable from `mwl run`. What a
  second file contributes is its *declarations*: a bare `echo` at its file scope compiles and prints
  nothing (`mwl-ir` gap 22). So a case pins the second file by *using* what it declares, never by what it
  echoes on its own.
- **A `--EXPECTF-ERROR--` case must not also *use* what the broken declaration would have provided.**
  Diagnostics are ordered by phase, not by file, so an `E0303` from the entry point's reference is printed
  *before* the resolution error the case exists to pin, and the block no longer matches at its first line.
  A compile-error case's entry file should do the least that reaches the diagnostic — often a bare
  `require` and nothing else. Between two diagnostics `%A` covers the span, notes included.
- **A rule added to `mwl_syntax::check_declarations` reaches far less of the corpus than a grep
  suggests.** Only `mwl-cli` and `mwl_hir::requires` call that walk, so every `mwl-types` fixture, every
  parser test and every `mwl-codegen` fixture goes straight past it — ADR 0094's estimated "sixty inline
  snippets to rewrite" turned out to be eleven, all in `casing.rs`'s own tests. Grep for the *callers*
  before budgeting a corpus rewrite; a `<?mwl` snippet in a Rust string is not automatically subject to
  everything the compiler enforces.
- **A row the checker accepts is not a row that runs.** `mwl-codegen` refuses a binary operator over two
  representations with *"does not lower a binary operator over mismatched representations"*. Equality is
  out of that hole — `$n == $f` is `Helper::NumericEq` now — but `$n + $f` and `$n < $f` still type-check
  and still fail there, so a conformance case written straight off an ADR's compiling rows can fail at run
  time. Run the rows in a scratch `.agent-tmp/*.mwl` before writing the case; if one does not lower, pin it
  in the crate's own `tests/` and say in the case comment why it is not here.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — CI runs that suite on all three hosted
  runners and none of them has PHP, so an oracle section makes the runner *skip the whole case* there,
  subtracting from the very count Stage 4 measures. Verify against PHP while authoring — `php -r '…'` is
  enough to settle a semantics question, and it is on `PATH` under Windows *and* inside the WSL distro
  ([docs/setup.md](../setup.md)) — then drop the section or put the case in
  `tests/differential/`, which is where an oracle belongs. The `.mwlt` format
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

## Splitting a file that got too big

- **The mechanism is two lines, and it is a pure move.** Rust lets one inherent `impl` and one set of free
  functions live in several modules of the same crate: each child starts with `use super::*;` (which
  reaches the parent's private imports *and* its siblings' names once `mod.rs` globs them back), and every
  item that crosses a seam becomes `pub(super)` — the reach it had as a private item of one file. A child
  can also see the parent's private items, so plumbing stays private in `mod.rs`.
- **A `pub(crate)` item needs an explicit `pub(crate) use` in `mod.rs`** or `crate::thing::name` stops
  resolving for the rest of the crate. A glob `use self::child::*;` covers the in-directory names; the
  re-export list covers the crate-facing ones, and the two coexist.
- **Cut by *entity*, never by line number, and check the count afterwards.** A range that starts one line
  late leaves a `#[test]` attached to the previous item — which is a *silent* lost test, not an error,
  unless the function happens to take arguments. `grep -c '#\[test\]'` before and after, and the test count
  in `verify.py`'s output, are the two checks that catch it.
- **A moved test module also *renames* its snapshots**, on top of the *Tooling* bullet above: the file
  name is the test's module path, so `parser::tests::foo` becoming `parser::tests::stmt::foo` needs the
  `.snap` moved *and* its `source:` line updated. Do that by hand instead of accepting the `.new`, and the
  diff stays a rename rather than a delete plus an unreviewable add.
- **A big file hides doc comments attached to the wrong item.** Two of `mwl-types`' were 120 lines from the
  function they described, invisible in a 3.5k-line file and obvious the moment it became eight. When a
  carve leaves a doc block stranded above an unrelated item, that is a bug the split found, not one it
  made.
- **Header prose splits with the code.** A module doc that grew a paragraph per ADR slice *is* the split
  plan: each paragraph already names the rule it belongs to. What is left in `mod.rs` afterwards is its
  charter — see AGENTS.md's length-target table for why the charter is the part that matters.

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
(`ParseError`, `LogicError`, …) now does. **Inside a `namespace X;` every name resolves relative to it**,
PHP's rule exactly, so `Core\Str::upper` in a namespaced file is `X\Core\Str` and takes `E0303` plus a
knock-on `E0403` on the method's declared return; write `\Core\Str`, which does resolve, and reach a
sibling in the same namespace unqualified.
