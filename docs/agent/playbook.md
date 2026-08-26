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

- **`.agent-tmp/` is shared between concurrent writers, so a fixed scratch filename hands you their
  file.** `git commit -F .agent-tmp/msg.txt` picked up a *stale* message another session had left
  there and committed this tree's handoff under "the handoff says the benchmark material is
  committed, because it is" — no error, no warning, and the only notice was `git log -1`. Name a
  scratch file for the thing it holds (`wrap-msg-handoff.txt`), never `msg.txt`/`patch.txt`, and read
  `git log --oneline -1` after any `-F` commit. The same applies to a `--patch` file handed to
  `splice.py`. Better still, let `python tools/session.py --wrap` write the commits: it takes the
  message inline and never touches a shared path.
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
- **A Python `str.index` anchor that is not unique cuts the wrong region and *duplicates* the file, with
  no error.** A one-off script that spliced a block out of `lower/stmt.rs` with `s[:a] + s[b:]` found an
  earlier `other => panic!(` for `b`, so `b < a`, the "removed" block measured zero characters, and the
  file came back 118 lines longer than it went in. Nothing failed; the only notice was
  `git diff --stat` reporting three times the expected insertions. Two rules follow: **use the Edit tool
  to remove a block** (it refuses a non-unique `old_string`, which is the whole point), and if a script
  really must cut, `assert a < b` and print the slice length. Recovery is `git checkout -- <file>` plus
  re-applying the edits, which costs less than reading the damage.
- **The Bash tool eats a backslash inside a heredoc**, and an em dash or apostrophe in one can defeat an
  exact-match splice — a `\\` written in a `python - <<'PY'` heredoc arrives as `\`, and a `"\n"` arrives
  as a real newline, so a block containing either will silently fail to match. Use the Write/Edit tools, or
  `python tools/splice.py <target> --patch <file>` with the patch **written by the Write tool**
  ([conventions.md](conventions.md) has the format). This is the rule sessions break most: reaching for
  `cat > f <<'EOF'` to save a call is how the mangling gets in. `splice.py` matches the anchor **exactly**,
  trailing newline included — the Write tool ends a file with one, so strip it when splicing
  mid-paragraph, and use `--dry-run` if you are unsure the anchor is still current. A Rust string holding
  a `Core\Name` label needs `r"..."`, or the backslash is an unknown escape.
- **A file edited by a script comes back into your context whole.** The harness notices the on-disk
  change it did not make and re-prints the file as a "changed on disk" reminder — an 900-line module is
  about 10k of context, twice the cost of the edit itself, and the Edit tool never triggers it. So the
  heredoc/`splice.py` route is for an edit Edit genuinely cannot express (a non-unique anchor, a
  whole-field rewrite), not a shortcut for one it can.
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. Whole suite:
  `python tools/loop.py --leg-only` (background it; minutes) — every fixture, both suites and the
  valgrind sweep against a Linux build, and it needs no script because it drives `wsl.exe` for you.
  One fixture: `tools/leak-check.sh <paths>` — a file passed by path, for the reason above — and it
  takes `.mwl` files only, so a `.mwlt` passed to it reports a failure that is not a leak.
- **Another agent may be editing this repo at the same time.** Check the ADR directory for the next free
  number immediately before writing one, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it. `tools/brief.py` prints a loud
  banner when `.loop/running` exists — if it does, stop and tell the user rather than editing alongside it.
- **A second writer can replace a file you created *this session*, and the only notice is a "changed on
  disk" line.** A `Core` slice run twice concurrently produced two whole implementations of the same new
  module and two conformance cases for it; the second Write silently won. Do not revert — the tree is only
  internally consistent with the newer one — but *re-read the file before every later edit*, delete the
  duplicate case rather than shipping both (a padded conformance count is worse than a missing one), and
  say in the commit message that it carries two authors. The `git status --short` you run before staging is
  where a duplicate shows up, and it is the only place.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- **A `Core\Name` inside a `python - <<'PY'` heredoc is a Python escape error, not just a Bash one.**
  The heredoc bullet above is about the Bash tool eating a backslash; this is the second half of the
  same trap and it fails differently — `"""… Core\Uri …"""` reaches Python intact and *Python* then
  rejects `\U` as a truncated `\UXXXXXXXX`, so a patch script that quotes any `Core\U…`/`Core\N…`
  path dies at parse time with nothing about the real edit in the message. Use the Edit tool for a
  targeted replacement, or `splice.py` with a Write-tool patch file.
- **A test failing in a file your slice never touched is probably inherited, not caused.** A red
  `-p mwl-stdlib --lib` arrived this session from the previous one's `validate.rs`, and the first
  instinct — "my registry rows broke something" — costs a bisect. `git status --short` showing that
  file unmodified is the whole diagnosis. Fix it, but in **its own commit**, so `git log` does not
  read as though the feature slice touched it.
- **Never `plan.py --get "Open now"` just to check an edit landed.** That field is one logical line of
  about 10,000 tokens, and `--get` unwraps and prints the whole of it — five per cent of a session's
  ceiling to confirm a sentence you already wrote. `grep -n` the phrase in `docs/implementation-plan.md`
  instead; the surrounding `> `-prefixed lines are the same fact for a hundredth of the cost.
- **A plan status field is *one* logical line, and three renderings of it disagree.** `plan.py --get`
  hands back the unwrapped line, `orient.py` re-wraps it again, and on disk it is a `> `-prefixed
  blockquote wrapped at ~100 columns. So an anchor copied out of either *rendering* never matches the
  file: to change one sentence with the Edit tool, `grep -o` the phrase in `docs/implementation-plan.md`
  and copy the `> `-prefixed lines around it. `plan.py --set FIELD --from <file>` rewrites the whole
  field from a file written with the Write tool, which is the other way and the only one for a field-wide
  change.
- **A `**Bold phrase:**` inside a field body silently becomes an eighth status field.** `plan.py` finds
  fields with `^> \*\*([^*:]+):\*\*`, and it re-wraps what you `--set`, so a bolded lead that ends in a
  colon and happens to land at the start of a wrapped line is parsed as a new field name on the next read —
  which AGENTS.md's fixed field set forbids. Write `**Bold phrase** —` instead; the colon outside the
  asterisks is invisible to the regex. The symptom is `python tools/plan.py` listing eight fields, and the
  fix is `git checkout -- docs/implementation-plan.md` followed by the `--set` again.
- **Making a type `pub` owes it a `#[derive(Debug)]`.** The workspace denies `missing_debug_implementations`,
  so promoting a private struct to the public API compiles and then fails at clippy — step 3 of four, after
  the tests have already run. Add the derive in the same edit as the `pub`, not after `verify.py` says so.
- **`/tmp` is not the same directory to Bash and to Python here.** A file written by `>` in the Bash tool is
  invisible to a `python -` heredoc in the same call, which resolves `/tmp` to `%TEMP%`. Stage a scratch
  file under `.agent-tmp/` — both halves agree on a repo-relative path.
- **`mwl-ir` cannot name `mwl_hir::QName`** — `mwl-hir` is a *dev*-dependency there, on purpose, so a
  lowering helper that wants one in its signature does not compile even though `mwl_types::Ty::Enum`
  hands it a `&QName` to pattern-match. Destructure it at the call site and pass what the callee actually
  needs (an `&EnumInfo`, or the name already rendered with `to_string`); `mwl_types` re-exports the enum
  and layout tables but not `QName`, and adding the dependency to get one is the wrong direction.
- **Teaching `mwl-ir` a new receiver or value shape is two edits, and the second one panics somewhere
  else.** Recording a new `ExprInfo` variant gets the *access* lowering; what still fails is
  `erase_checked_ty`, which owns the `mwl_types::ty::Ty` → `mwl_ir::Ty` representation map, and whose
  panic names "a resolved call's parameter or return type" — so the message points at a call boundary,
  a `var` binding or a `foreach` element rather than at the feature you just built. `Ty::Shape` needed
  exactly that: `ExprInfo::ShapeProperty` plus one arm erasing a shape to `Ty::Object`.
- **A class synthesized while lowering an *expression* has four exits, not one.** `Lowering` builds
  one function, so a `crate::ir::Class` an expression invents (a closure's environment, a shape
  literal's) has nowhere to go until `lower_file` — it rides out of `lower_method`/`lower_hook`/
  `lower_script` at each of their three identical `std::mem::take(&mut low.closures)` sites, *and* out
  of `lower_closure`'s own recursion, or a body nested one level deeper silently contributes no class
  and codegen fails much later on a `New` naming a label the table has no entry for. Grep the take
  sites, not the struct field.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>`; a renamed
  test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two minutes.
- **One call reads many places: `python tools/peek.py A.rs:120-160 B.rs:@sym C.md:"## 4"`.** Locators are
  `120-160`, `120+30`, `@symbol`, `re:pattern` (`re:pattern:3` for context lines), `"## Heading"`, or
  nothing for a whole file under 400 lines; the path may be a glob, so one target can sweep a crate.
  `--locate <symbol> ...` answers with `file:line` and no bodies, which is what a handoff's anchors are
  made of. Reach for it instead of a `grep` and then a `sed` and then another `grep`: measured over a full
  run, sessions issued 3,647 tool calls and put two in one message **zero** times, including runs of 52 and
  57 consecutive reads, so the batching rule in `AGENTS.md` has never once been collected by hand.
- **Prefer `re:pattern` to `/pattern/` in a `peek.py` target on Windows.** Git Bash rewrites any argument
  that *starts* with a slash into a Win32 path before the process sees it, so `file.rs:/fn foo/` arrives as
  `file.rs;C:/Program Files/Git/fn foo/` and the tool reports no such file. Quoting does not help — the
  conversion happens in the shell's argv handling, not its parser. The same trap catches any tool argument
  spelled as a leading-slash path.
- **The end of a session is one call: `python tools/session.py --wrap <file>`.** Write one markdown file
  whose `## ` headings are instructions — `## plan: <Field>`, `## playbook: <Heading>`, `## handoff`,
  `## commit: <paths>` once per slice, `## status` — and it applies all of them in a fixed order, or
  validates one of them as broken and writes *nothing*. `--check` first says what the tree still owes,
  including any plan field whose prose names a conformance or differential count the tree contradicts.
  Doing this by hand measured 33 of a session's 98 calls, and because context peaks by then, 42% of its
  whole token bill.
- **A `[context] playbook` entry may name one bullet, not just a whole section.** `"Tooling > A whole ADR"`
  prints that bullet; `"Tooling"` still prints all 11 KB of the section. The playbook grew 61% during one
  run and its four selected sections were 35% of the entire orientation pack, so a goal that needs three
  traps should name three traps. `python tools/orient.py --audit` prices the difference.
- **A commit message carries no attribution trailer, and two gates enforce it.**
  `tools/session.py` strips `Co-Authored-By`, `Signed-off-by`, `Generated-with` and the prose
  `🤖 Generated with [tool](url)` out of any message it commits; `tools/git-hooks/commit-msg` rejects one
  arriving by `-m`, `-F`, an editor or a merge. Enable the hook once per clone with
  `git config core.hooksPath tools/git-hooks` — `verify.py` says so when it is not set. The rule and its
  reasoning are conventions.md § *A commit message*.
- **Git runs a hook with `LC_CTYPE=C.UTF-8`, and gawk's `[^a-z]` does not match an emoji under it.** A
  pattern anchored as `^[^a-z]*(generated|created)…` therefore passed every test run from the Bash tool —
  which has no locale set — and silently failed to match `🤖 Generated with [...]` when git itself ran it.
  Two lessons, both expensive: run a hook's test the way *git* invokes it, and prefer locating a phrase
  with `match()` and asking a pure-ASCII question about the text before it over any character class that
  has to step across multibyte input.
- **An awk fatal inside `$(…)` leaves the variable empty, and a hook that only checks the variable then
  fails OPEN.** `\[` in a *dynamic* awk regex (one built from a string) is consumed by the string literal
  first, so awk sees a bare `[`, dies with `invalid regexp`, and the surrounding `[ -z "$offenders" ] &&
  exit 0` cheerfully allows the commit. Write it `[[]`, and always capture `$?` from the awk itself and
  refuse on a non-zero status — a gate must fail closed.
- **A plan field can disagree with `loop-goal.toml`, and the toml wins.** `Open now` opened with
  "Catch-up is done … every `stage = "0 catch-up"` check in `loop-goal.toml` passes" while **six** of
  the tests those checks name did not exist in any crate — a session that believed it would have
  opened a Stage 3 `Core` slice behind a gate `loop.py` short-circuits before reaching. The toml
  names each test literally, so one `grep -rn "<test_name>" --include=*.rs` over the block settles
  the question for one call; `handoff.md` § *State* was the half that was right, and the prose was
  corrected rather than the toml.
- **An untracked *directory* under `benches/` breaks the whole cargo workspace, and the first symptom
  is a stale binary.** The root manifest globs `members = ["crates/*", "benches/*"]`, so
  `benches/userland/` — MWL and PHP benchmark *sources*, left untracked by an earlier session — made
  every `cargo` invocation die with `failed to read benches/userland/Cargo.toml` before a single crate
  was read. Piped through `| tail -3` the error scrolls past, `$?` belongs to `tail`, and the run that
  follows uses whatever `target/debug/mwl.exe` was built last, so the session diagnoses a phantom
  language bug instead. `cargo metadata --no-deps >/dev/null; echo $?` is the one-call check, and the
  fix is an `exclude = [...]` line beside the glob. The same hazard waits for any new non-crate
  directory under a globbed member path.
- **A `grep … | head` that finds nothing is not the same as a grep that finds nothing, and a handoff
  that reports one as the other sends the next session in with the wrong scope.** This session's item
  opened with "`Uri::isValid` is not written yet (`grep isValid` finds only `Encoding` and `Json`), so
  the work is the roster, not a deletion." Both roster classes declared `isValid`; the previous
  session's grep had a `| head` on it and the ten lines it kept ended one line before `uri.rs`'s rows
  began. The slice was therefore twice the size the handoff sized it at, and the wrong half —
  deleting a member touches the spec table, the ratchet, two conformance cases and the ADR that
  claimed the two spellings were one predicate. Two rules: **never `| head` a grep whose result you
  are about to assert is empty** (use `-c`, or `-l`, or no pipe at all), and treat "X is not written
  yet" in a handoff as a claim to re-check in one call before scoping around it.
- **A `loop-goal.toml` check's *comment* is not the specification, and it can contradict a settled
  ADR.** Item 13's comment read "`==` normalizes per RFC 3986 § 6.2.2 and then compares components",
  which ADR 0090 § 4 forbids outright — `==` on two objects is identity, there is no `__equals`, no
  `Equatable`, and that ADR names `$a->compareTo($b) == 0` as *the* spelling for content equality. The
  toml wins over a **plan field** (the bullet above), because both are status; it does not win over an
  ADR, because only one of those is a decision. One `peek.py <adr>:"## 4"` before writing the member
  settles it, and the comment is the thing to fix. Implementing what the comment said would have put a
  per-class equality hook in `mwl_runtime::identity` and re-opened an ADR from inside the loop.
- **`verify.py` can be red on a tree you did not touch, and `fmt` is where it happens.** A docs-only
  session hit `cargo fmt --check` failing on committed code — a four-line `mwl_array_get_index` signature
  rustfmt wanted on one line — which means the slice that added it was committed without step 3 ever being
  green. Do not treat that as "my change broke it" and do not skip the fix: it is one hunk, it goes in its
  own commit named for what it is, and the session's own slices stay clean. Check the blast radius first
  with `grep -c "^Diff in" .agent-tmp/verify-fmt.log` — one file means fix it here, a dozen means say so in
  the handoff instead of reformatting the workspace inside an unrelated slice.
- **Widening an operand's *representation* in `mwl-ir` moves a refcount decision you did not edit.**
  `lower_array_key` returned `(ValueId, bool)` where the `bool` meant "aliases storage someone else
  owns", and its four call sites read the `false` case as "a fresh buffer this frame owes a release
  for" — two different facts that happened to coincide while every key was a `Ty::Str`. The moment an
  `int` subscript could travel unrendered, `false` still arrived but there was nothing to release, so
  the *unchanged* `if !key_aliasing { emit_release }` line became a release of a plain integer. The
  fix is to return the operand's `Ty` alongside and guard on `ty.is_refcounted()`, and the general
  rule is that a widened operand's every consumer has to be re-read for a decision phrased as the
  *negation* of the old invariant. Nothing catches this: it builds, and the IR snapshots are the only
  place it shows.
- **`cargo insta accept` accepts *every* pending snapshot in the tree, including a previous
  session's.** Item 19 renamed three lowering tests and left their `.pending-snap` files behind, so
  one `cargo insta accept --workspace` after adding two new tests materialized five `.snap` files —
  three of them orphans naming tests that no longer exist, which nothing then fails on. `git status
  --short` immediately after is the whole diagnosis: any `??` snapshot whose name you did not just
  write is one to delete.
- **`INSTA_FORCE_UPDATE=1` rewrites every snapshot in the crate, not the failing ones.** Two
  lowering snapshots needed new content this session; the run came back with **86** modified files,
  because every other `.snap` still carried `source: crates/mwl-ir/src/lower.rs` from before that
  module was carved into `lower/mod.rs` and the forced update refreshed that header too. Nothing
  failed and nothing was wrong — it is just 84 files of churn inside a feature commit. Use plain
  `INSTA_UPDATE=always cargo test -p <crate>` (no `FORCE`), or sort it out afterwards with
  `git diff --numstat` per file and `git checkout --` the ones whose whole diff is two lines.
- **An acceptance check in `loop-goal.toml` can be red on a *name* rather than on a claim, and the
  difference costs a session to tell apart.** Item 20's three tests and item 21's two were all listed
  under names predicted before anyone wrote them; the implementations had landed sessions earlier
  under names that state the narrower, true claim
  (`appending_into_spare_capacity_allocates_nothing`, not
  `appending_to_a_uniquely_owned_string_does_not_reallocate` — spare capacity is what makes an append
  free, and a uniquely owned string with no room *does* reallocate). So when the handoff says a check
  names "three tests that do not exist", **grep the crate's test names for the claim before writing
  anything**: `grep -n "    fn " crates/<crate>/src/<file>.rs` costs one call and can turn a slice
  into a two-line edit. Correct the *list* when the tree's name is truer, and say so in the toml
  comment; a predicted name is status, not a decision, and the playbook's neighbouring bullet about a
  check's *comment* is the same rule one field over.
- **A `goal check:` line that repeats verbatim across sessions *is* the work, whatever the handoff
  says.** The acceptance test short-circuits at its first failure, so everything after that check is
  not red — it is unmeasured, which reads identically from the ledger. Sessions 0014–0030 of the
  2026-08-26 run all reported `mwl-runtime (string capacity): test '…' did not run`, and for those
  seventeen sessions Stage 4's two counts, Stage 5's guards, the WSL leg and the valgrind sweep
  never ran: the case counts the ledger quotes were session prose rather than the gate, and the run
  could not have stopped even had the goal been reached. That one was a stale *spec* — `loop.py`
  read `loop-goal.toml` once at start-up, so the rename the bullet above records never reached the
  running driver, and `drive()` now re-reads the list before every check. A repeat can just as
  easily be a real red check nobody has opened. Either way `python tools/loop.py --goal-only`
  answers it in one call, and it is worth one call the moment the same line lands twice.
- **`python tools/verify.py` does not run a single `.mwlt` case.** Its 74 suites are cargo's; the
  conformance tree is executed by `mwl test`, so a case that fails to compile or whose `--EXPECT--`
  is one byte off leaves verify green and fails the *driver's* acceptance check instead, one
  session later. `./target/release/mwl.exe test <path>` takes a single case and answers in under a
  second — run it while writing, and `mwl test tests/conformance` once before the wrap, which is
  also where the count the plan's fields quote comes from. Use the **release** binary:
  `target/debug/mwl.exe` is whatever the last `cargo test` left behind and can predate your change
  by a whole session.

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
  until the WSL valgrind leg catches it. `try { … } catch (Throwable $e) { … $e->message … }` **does**
  lower at file scope now, which is what `tests/conformance/core/time-datetime-is-a-civil-time-in-a-zone.mwlt`
  and both `Date`/`TimeOfDay` cases write; the class-method shape
  `tests/conformance/lang/a-lossy-conversion-throws.mwlt` uses is no longer needed for that.
- **A leak whose "definitely lost" size is `16 + strlen(a literal in the probe)` is a temporary abandoned
  on a throwing edge, and the allocating stack carrying no `mwl_` frame at all is the confirmation.** That
  is the whole recognition test, and it is worth knowing because bisecting to it costs an hour. Most of
  those are now closed: a call's arguments and receiver, and the operands of `.`, an interpolation and an
  `echo`, all go on `mwl_ir::lower::Lowering`'s owned-temporaries stack and are released on both edges. A
  probe that `catch`es a throw from a `Core` member taking a `string` is therefore *green* now and is a
  fair leak check. What is still open is narrower and named in that field's own doc comment (an argument
  being **transferred** when a later one throws) plus the producers that still release inline — a
  normalized subscript key, a `match` subject.
- **A field or element read off a *temporary* is a fresh producer, not an aliasing read.**
  `$h->peek()->name` and `$m->rows()["0"]` each used to leak one value per run; neither does now,
  because `lower_property_access`/`lower_index` retain what they read and release the base, and
  `Lowering::aliasing_read` therefore recurses into both a property access's and an index's own base
  and answers `false` for these shapes. So a consumer must not retain such a read a second time —
  every retain decision in `mwl-ir` already goes through `aliasing_read`, and a new one that reaches
  for the syntactic `is_aliasing_read` instead is how the double-retain gets back in.
- **`verify.py` never executes a `.mwlt` case**, so editing one is invisible to it. `cargo test`'s
  `conformance_coverage.rs` asserts only that a case *exists* naming each registry member — nothing
  in the cargo suite runs the case body, so a rewritten case can leave all four verify steps green
  and fail at `loop.py` one stage later. Any session that touches a `.mwlt` owes a
  `cargo run -p mwl-cli -- test <the cases>` of its own beside `verify.py`, and
  `mwl test tests/` to confirm the suite's pass/fail split has not moved. On Windows the baseline
  is **519 passed / 6 failed**, those six being the PHP-on-Windows oracle set.
- **A before/after measurement is worth a `git stash`, and the base half is what makes it an
  A/B rather than two readings** — stash, `cargo build --release -p mwl-cli`, `bench.py <cases>
  --reps 9`, pop, rebuild. Item 22's base run reproduced the ledger's own sweep to within 0.02×
  on every row, which is the check that the pair is comparable; without it a 0.72× → 0.91× move
  is indistinguishable from a quiet machine. Two costs to budget for: the two release rebuilds
  are about two minutes each, and the harness re-prints **every stashed file it has seen** into
  the session as an on-disk change, twice — that was about 16k of context for four files. A
  `git worktree` avoids the re-print and pays a full cold build instead, which is worse.

- **`bench.py` says when the machine was busy, and it means it.** A sweep whose min and median
  differ by more than 25% on `00-baseline` prints a note, and since the baseline is subtracted
  from every row, that run's ratios are all shifted — one such sweep read `20-method-dispatch`
  at 0.67× and the clean re-run put it back at 0.71×. Re-run before quoting, and do not reason
  about a 5% row move from a sweep carrying that note.
- **A `#[global_allocator]` in a test target must be `#[cfg(debug_assertions)]`.** `mwl-runtime`
  installs its pooled allocator under `all(not(test), not(debug_assertions))`, so a second one in a
  `mwl-stdlib` test binary links fine under `cargo test` and fails to link under
  `cargo test --release` with *"cannot define multiple global allocators"*. `verify.py` runs the
  debug profile, so the guard runs; the release profile compiles it out.
  `crates/mwl-stdlib/tests/allocation_policy.rs` is the worked example, and counting allocations is
  worth the setup — it turned "I think this allocates once" into a test.

- **A release build relinking the runtime moves a bench row by about ±6%, with no code change.**
  `06-string-split-join`'s work figure read 87.3 ms and 93.3 ms across two builds whose `join` was
  byte-identical, and `04-string-format` moved 82.4 to 86.9 with nothing of its own touched. So an
  A/B on a single row is only worth reading when the delta is well past that, and the *median* is
  the statistic to quote. Re-run the base binary once before believing a small regression.
- **`python tools/verify.py` is not deterministic, and the one test that makes it so is a real
  use-after-free rather than a flake to re-run past.** `-p mwl-codegen --test throwing`'s
  `an_uncaught_throw_leaves_the_status_and_the_message_on_the_context` fails about 7% of runs inside
  `verify.py` and 22 of 40 run on its own, either as a wrong `ctx.pending()` message or as a bare
  *"misaligned pointer dereference"* panic in `crates/mwl-runtime/src/object.rs:1220` with nothing
  naming the throw. So a red `verify.py` in a session that touched no Rust is worth **one** re-run to
  identify — and if that is the test, it is inherited: say so and leave it to the slice that owns it,
  because a second green run does not mean the tree is clean.
- **A `Ctx` that outlives the `Unit` whose code it ran reads freed class descriptors, and the crash lands
  nowhere near the cause.** Compiled code bakes each `ClassDesc`'s *address* in as a constant, so an
  exception object left on the context points into the `Rc<ClassTable>` the `Unit` owns and nothing else
  keeps alive. Drop the unit first and `ctx.pending()` reads freed memory — intermittently a wrong message,
  intermittently a *misaligned pointer dereference* inside `mwl_runtime::object::drop_one`, which is the
  release walking garbage slot counts. This is what made `verify.py` non-deterministic for several
  sessions (28 of 40 runs of one `mwl-codegen` test, 7% inside `verify.py`), and the reason it looked like
  a flake is that `mwl run` never hits it — `mwl-cli` installs the table. **`mwl_codegen::Unit::install_in`
  is now the one spelling and its doc comment is the rule**: call it before running any of a unit's code,
  whether or not you care about `catch`. A harness that builds a `Ctx`, runs a unit and then reads anything
  off the context is the shape to watch for.

- **`tools/leak-check.sh` used to report a fixture's own non-zero exit as a leak.** `examples/uncaught.mwl`
  ends in an uncaught throw and so exits 1 by design, which under valgrind's `--error-exitcode=1` was
  indistinguishable from a definite leak — the `definitely lost: 0 bytes in 0 blocks` line printed right
  beside the "failure" was the only tell. It uses 97 now, a status no MWL program produces, so a throwing
  fixture is a fair leak subject.
- **`target/release/mwl.exe` is whatever the *last* session built, and a `.mwlt` case it fails may
  simply predate it.** A session that adds nothing but cases still owes a
  `cargo build --release -p mwl-cli` — about two minutes — before it believes a red run: this one's
  binary was two hours and four commits old, so `mwl test tests/conformance` reported
  `str-replace-and-pad-are-the-identity-at-their-own-bound.mwlt` failing on a `Core\Str::padStart` line
  nothing in the session had touched. `git status --short` showing that file unmodified says the failure
  was not *caused* here, which is the neighbouring bullet's rule; only the rebuild says it is not real.
  The same tree, rebuilt, is 478 passed / 0 failed.
- **A scratch `.mwl` still needs its `<?mwl` tag, and without one the panic names a construct you did
  not write.** A file under `.agent-tmp/` that opens straight into `echo` lowers as a single
  `InlineHtml(0:0..139)` statement and dies in `mwl-ir`'s control-flow slice listing every statement it
  *does* lower — which reads as "`echo` is unsupported" rather than "this file is all text". The
  `.mwlt` harness supplies the tag for you inside `--FILE--`, so the omission only ever bites on a
  scratch run, which is exactly where a session is trying to find out whether a shape lowers.

## Adding a `Core` member

- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `mwl_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.mwlt` case that calls it — `crates/mwl-stdlib/tests/conformance_coverage.rs` fails
  `cargo test -p mwl-stdlib` without one. An instance member is covered by a case writing `->name(`.
  [conventions.md](conventions.md) writes all four out; `python tools/brief.py`'s *anchors* block
  resolves each spelling to a file and line.
- **A spec §§ 1-12 member owes a *fifth* thing: striking its line from
  `crates/mwl-stdlib/tests/spec-members-outstanding.txt`.** That file is the outstanding-member ratchet
  `crates/mwl-stdlib/tests/spec_registry_coverage.rs` reads, and the test fails on a **stale** line — one naming a member
  that is registered now — exactly as loudly as on an unregistered member the file does not list. So the
  failure you see after landing a member is not a regression; it is the list telling you it did not
  shrink. Its keys are `§<section> <the spec's own Member-cell spelling>`, which is why `§1 chunk` and
  `§2 chunk` are two different lines.
- **A `Core` instance's slots hold only values MWL already holds, so a member wanting native mutable
  state has to accumulate instead** — there is no destructor to free a `sha2::Sha256` context with, and
  `digest 0.10` cannot serialize one into a slot. The COW-correct read/write of a slot that holds an
  array is `identity_store::borrow`/`edit`/`replace`, which are generic over `(receiver, index, class,
  member)` despite that module being named for § 9's store; `instance::set_slot` is the raw write and
  `instance::slot` the borrowed read. `Core\Hash\Stream` is the worked example.
- **A new domain module is `mod`, not `pub mod`, so its `CLASS`/`NAME` are `pub(crate)`.** The
  workspace warns `unreachable_pub`, and half of `mwl-stdlib`'s modules are `pub mod` while the newer
  half is not — copying `uuid.rs`'s `pub const NAME` into a privately-declared module is a warning at
  build time, before `verify.py` says anything. `objmap.rs:36` is the shape to copy.
- **A `Core` symbol that is not a member breaks `every_registered_member_has_an_implementation_address`.**
  `mwl_stdlib::symbols()` used to be exactly one entry per `CLASSES` member, and that test asserts the
  count — so a constructor symbol from `registry::CONSTRUCTORS`, or anything else chained in beside the
  members, has to be added to the sum on the test's right-hand side in the same edit. The failure is a
  bare `left: 213, right: 211` in `-p mwl-stdlib --lib`, with nothing naming the symbol.
- **A `CoreTy::Array(&CoreTy::Uint)` parameter receives `Tag::Int` elements**, so a helper that reads
  each one through `as_uint` alone answers the member's most obvious call site with a fatal. A written
  `[97, 98]` type-checks against `array<uint>` and stays int-tagged all the way into the helper — a
  scalar `uint` parameter does not have this problem, because the call site materializes the literal at
  the declared type. Read both tags (`str.rs`'s `code_point`), and probe the literal spelling in a
  scratch `.mwl` before writing the case.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes. **An instance member's
  receiver is argument slot 0 and is not in `params`**, so `plus(Duration)` is `args: [2]`. A mismatch is
  an index-out-of-bounds panic at the first call.
- **A member on `registry::WRITTEN_CLASS_MEMBERS` takes one argument its row does not declare** — the
  class its call site wrote, in slot 0 — so its helper's `args: [N]` is `params` + 1 (+ the options bag's
  flattening). `crates/mwl-stdlib/tests/conformance_coverage.rs` looks for such a member spelled `Class::name<`, not
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
- **A registry rule quoted by test name may not be that rule, or may not exist.** A handoff opened
  ADR 0069's slice with "`registry`'s own `a_union_is_only_ever_a_parameter` says a union cannot be a
  return type", which would have forced `array<mixed>` on four members. There is no test by that name —
  the real one is `a_union_option_excludes_null`, which restricts an **option's** type and
  nothing else, and `CoreTy::Union`'s doc says "legal in **either** direction" outright. One
  `grep -n 'fn [a-z_]*(' registry.rs` over the test names costs one call and settles it; designing around
  a constraint that is not there costs a member's whole surface.
- **Adding a row to `mwl_hir::errors::TREE` fails a test in `mwl-ir`, and the message names neither
  the tree nor the class you added.** Spec § 10's exception tree is restated as a hard-coded label
  list in `lower/mod.rs`'s `a_file_with_no_class_still_carries_every_compiler_declared_class`, so
  `RecursionError` arrived as a bare `left: [... 12 names ...]` / `right: [... 11 names ...]` diff in
  `-p mwl-ir --lib` with nothing pointing back at the one-line `errors.rs` edit that caused it. The
  full roster a new § 10 class owes is: the `TREE` row, `mwl_runtime::ThrownClass`'s variant, its
  `name()` arm, its `ALL` entry, that assertion, and the spec's own tree drawing. Nothing else
  restates it — `mwl_types::error_lib` seeds whatever `TREE` holds.
- **A second *read* is not cheaper than the `memcpy` it saves, at `Core\Str` sizes.** Both obvious
  ways to make a result's length exact before writing it measured as losses, and each cost a full
  release build plus a bench sweep to find out: counting `Core\Str::replace`'s matches with a second
  `find_from` pass took `05-string-replace` from 0.92× to **0.74×**, and walking a cycle of
  `padEnd`'s padding to measure what a second walk then wrote took `07-string-normalize` from 0.50×
  to **0.45×**. Both are arithmetic now and both rows are far above where they started. The rule to
  carry: reach for arithmetic or for a good capacity guess, never for "measure it first" — and if a
  member's length genuinely costs a data-structure walk to learn, as `Core\Str::join`'s does, leave
  it alone. `crates/mwl-stdlib/src/str.rs` § *A result is written once* holds all of it.
- **`Value::as_str_bytes` and `Value::as_text` make the *same* tag check, so a `from_utf8` after the
  first can never catch anything.** Both go through `Value::str_ptr`, which answers for `Tag::Str`
  alone — and since `Tag::Bytes` became its own row over the shared allocation, a `bytes` argument
  reaches neither. Sixteen sites in `mwl-stdlib` carried the pair anyway, and `json.rs`'s carried a
  doc comment saying it was where "the caller passed binary data into a text format" got caught,
  which had quietly stopped being true. A defence a later ADR made unreachable reads exactly like a
  live one, and grepping for the *tag* it claims to catch is the cheap way to tell them apart.
  `crates/mwl-stdlib/tests/allocation_policy.rs`'s `no_member_revalidates_a_string_argument` is the
  source scan that keeps the pair out now.

## Writing a test case

- **`var` and a written type are two different declarations, and `var T $x = …` is neither.**
  `var $x = …` infers; a declared type is `T $x = …;` with no `var` at all — so
  `var Core\Regex\Pattern $p = …` is four diagnostics (`E0101` three times and then `E0301` for a name
  that was never declared), none of which says "drop the `var`". The trap is that the *inferring*
  spelling is the one every case reaches for, so the typed one looks like it should take a keyword too.
- **An array literal written *directly* as a `Core` argument infers `array<mixed>` and is refused.**
  `Core\Arr::replaceRange($a, 1, 2, ["X"])` is `E0401: expected array<string>, found array<mixed>` — the
  parameter's type is not pushed into the literal, whether the parameter is `array<T>` or concrete. Declare
  a typed local one line above and pass it, which is what every existing case already does; a scratch probe
  written the obvious way fails at the checker before it ever reaches the member you are testing.
- **A case cannot index into an `array<mixed>`'s elements, and `Core\Json::encode` is the way round
  it.** `$q["b"] as array<string>` panics `mwl-ir` outright — *"got `Tagged as Array`"*, ADR 0007 § 2's
  `array<T> as array<U>` row being the one still missing — so a member answering a nested shape has no
  spelling that reaches past the first level. `Core\Json::encode($q)` renders the whole structure in one
  line and it is byte-identical to PHP's `json_encode` over `parse_str`'s array, which makes it the
  strongest assertion available as well as the only one. `$q["a"] as string` on a top-level scalar does
  lower.
- **A named class does not satisfy a shape type**, whatever its properties are called: `View::y(new
  Point(3, 4))` against a `{y: int}` parameter is `E0401: expected {y: int}, found Point`. ADR 0036 § 3's
  width subtyping is shape-to-shape only (`mwl_types::expr::assign`), so the *one* way a shape receiver's
  static layout differs from the value's own is a narrower shape — which is the only widening a case
  testing § 4's name-keyed read can write. Several doc comments claimed the class direction worked; they
  were wrong and are fixed, so do not design a case around it.
- **A `?array<T>` cannot be indexed even after a `!= null` guard** — `mwl-ir` panics outright at
  `crates/mwl-ir/src/lower/expr.rs:2952`, *"has no resolved element type recorded … its base erased to
  `mixed`"*. The narrowing itself works for a `?string`, so the hole is specifically that a narrowed
  nullable **array** loses its element type, and `Core\Arr::first`/`last` over an `array<array<string>>`
  is where a case meets it. Three spellings do lower and are the way round it: `$rows["0"]["name"]`
  (nested indexing, no nullable in the path), `foreach ($rows as array<string> $row)` — the binding's
  declared type is what re-supplies the element type — and binding `var $row = $rows["0"];` first.
- **Registering a `Core` member and writing its conformance case are one slice, not two.**
  `crates/mwl-stdlib/tests/conformance_coverage.rs` fails the moment a registry row has no `.mwlt` case
  calling it, so a plan that lands the rows in one session and the cases in another leaves the tree red
  in between — and `verify.py` reports it as a `-p mwl-stdlib` test failure with nothing about the
  member in the message. A class constant counts too: `Core\Path::SEPARATOR` needs a case that writes it.
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
- **`"…" as bytes` is how a case writes a `bytes` it can read, and `Core\Encoding::fromHex("…")` is how
  it writes one it cannot.** There is no `bytes` literal at all (`00-overview` § 5), so those are the two
  spellings; `as bytes` is total and free and only reaches octets that are valid UTF-8, which is why an
  arbitrary buffer — a lone `ff`, a truncated sequence — still has to come from `fromHex`. Assert the
  result with `toHex` either way, since `echo` has no `bytes` row: ADR 0009 § 3 makes `bytes as string`
  *checked*, and an implicit render is not that check.
- **`emit_binop`'s ordering rows are `Int | Uint | Bool` only, so `<`/`<=`/`>`/`>=` over two `string`s
  does not lower** — and the checker does not stop you, because `operators.rs`'s result table models only
  `int`/`uint`/`float` operands and falls back to `mixed` for everything else. A `.mwlt` case that
  compares two strings for order therefore compiles and then fails at run time. Assert a fixed slice with
  `==`, or a shape with `Core\Regex::matches`, and pin the ordering in the crate's own `#[test]`, which
  can also sleep. Watch the neighbouring trap too: a `Core` member answering `uint`
  (`Core\Str::length`) in `$int + …` is `E0407`, not a widening.
- **`mwl-codegen` has no `BinOp` row for `Ty::Enum` at all**, matched pair or not: `emit_binop`'s
  `integral` set is `Int | Uint | Bool`, so an `Eq` a lowering emits over two enum values fails with
  *"a `Eq` over representation Enum(Int)"*. Compare one representation down — `InstKind::Reinterpret` to
  the backing integer is free, and it is the row `$m as int` already uses.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — CI runs that suite on all three hosted
  runners and none of them has PHP, so an oracle section makes the runner *skip the whole case* there,
  subtracting from the very count Stage 4 measures. Verify against PHP while authoring — `php -r '…'` is
  enough to settle a semantics question, and it is on `PATH` under Windows *and* inside the WSL distro
  ([docs/setup.md](../setup.md)) — then drop the section or put the case in
  `tests/differential/`, which is where an oracle belongs. The `.mwlt` format
  is `crates/mwl-test`'s module doc; a `--EXPECTF-ERROR--` block must reproduce the diagnostic's own
  indentation, which widens with the line number. A trailing space before a `\n` is unreliable in an
  `--EXPECT--` block — echo a sentinel character after it.
- **The `php` on Windows `PATH` has no `mbstring`**, so every `mb_*` oracle a `Core\Str` slice reaches
  for — `mb_convert_case`, `mb_strtolower`, `mb_str_split` — dies with *"Call to undefined function"*
  rather than answering. The byte-wise half (`strcmp`, `strnatcmp`, `substr_count`, `str_replace`) is
  all there, so a member replacing both spellings can still be checked on its ASCII rows. For the
  Unicode rows, cite the UCD table the member implements (folding is UAX #44's `C`+`F` mappings) and
  say so in the case's comment; do not silently assert whatever the implementation printed. WSL's
  `php` may have the extension — worth one `php -m | grep mbstring` before writing the rows off.
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
- **A property's declared default runs now, and the constant is checked — but only a literal or `[]`
  is one.** `public int $n = 4;` reaches the slot of every fresh instance, inherited defaults
  included, because `mwl_runtime::MwlObj::new` writes a per-class image the descriptor carries; there
  is no IR instruction for it and nothing between `new`'s allocation and its constructor call. What
  is *refused* is everything else: `= null`, an enum case, a `decimal`, a non-empty array literal and
  a `Class::CONST` are all `E0472`, so a case reaching for one gets a diagnostic rather than a wrong
  value. A `static` property is skipped entirely — it occupies no instance slot.
- **Two `foreach` headers in one file may reuse a binding name only at the same type.** A binding is
  function-scoped, so `foreach ($names as string $n)` followed later by `foreach ($heap as int $n)` is
  `E0406: `$n` is already declared` pointing at the *second* header — while a second `string $n` walk is
  fine, which is why the rule looks like it is not there until the third loop. A case walking two
  differently-typed collections needs two names.
- **`live_bytes()` cannot see an allocation that is freed again inside the call under test**, so a
  guard named "…allocates no key" written as a `live_bytes` delta passes whether or not the key was
  built. `mwl_array_set` renders an `MwlStr`, hands it to the packed arm, which has no use for it,
  and drops it before returning — live delta zero, exactly like the index-taking pair that never
  allocated at all. `counting_alloc::allocated_bytes()` is the monotone total that tells the two
  apart, and any measured claim about a *transient* cost needs it rather than `live_bytes`. The
  control matters as much as the claim: assert the old spelling **does** allocate in the same test,
  or a broken counter reads as a passing guard.
- **A compiled function called with an empty argument slice faults.** `mwl_runtime::call(f, &mut
  ctx, &[])` on a *method* looked like the obvious way to observe a return value, and it is an
  access violation (`0xc0000005`, a bare `STATUS_ACCESS_VIOLATION` from `cargo test` with no test
  name attached) — the callee reads its argument slot whether or not it declared one, and `&[]`
  hands it a dangling pointer. Give the fixture's method one parameter it ignores and pass
  `Value::int(0)`; `mwl-codegen`'s `stack_limit.rs` fixtures all declare one, which is why nothing
  had hit this. `run_with`/`output_of` are unaffected — the script frame is entered the same way but
  never faulted, so the crash arrives only when a test reaches for `unit.function("Class::member")`.
- **An allocation guard over a member that *builds* something measures the result's own storage
  first.** A `counting_alloc::allocated_bytes` delta over one `map`-shaped walk into a fresh
  `MwlArray` read 448 bytes with no key rendered at all: the output's own `Vec` doubling on the way
  to 16 entries is an allocation the guard cannot tell from the one it exists to catch. Walk twice
  and measure the *second* pass, where every write lands at a position that already exists — then
  the only thing left that can allocate is the thing under test.
  `a_callback_that_does_not_want_a_key_synthesizes_none` is the shape.
- **A `cargo test` that dies with a bare `STATUS_ACCESS_VIOLATION` may not reproduce**, so run it
  again before bisecting. One arrived in `-p mwl-codegen --test throwing` on the first run after a
  relink and never returned in four subsequent runs, including the full workspace sweep; the
  `.loop` logs hold an identical one-off in `--test strings`. The deterministic cause below (an
  empty argument slice) reproduces every time, which is how the two are told apart.
- **A test binary outside `mwl-runtime` cannot reach `counting_alloc`** — that module is `#[cfg(test)]` and
  its `allocated_bytes` is `pub(crate)`, so a `mwl-codegen` or `mwl-stdlib` integration test that wants to
  measure allocations installs **its own** `#[global_allocator]` in the test file, gated
  `#[cfg(debug_assertions)]`. The gate is not decoration: `mwl-runtime` registers its pooled allocator in
  every `not(test)` *optimized* build, so an ungated one makes `cargo test --release` fail to link rather
  than fail a test. `crates/mwl-stdlib/tests/allocation_policy.rs:135` and
  `crates/mwl-codegen/tests/arrays.rs`'s `Counting` are the two copies of the shape.

- **Measure compiled code's allocations as a difference between two run lengths, never as an absolute
  zero.** A run allocates its array, its locals and its output buffer once whatever the loop count is, so
  "this script allocated nothing" is not a claim that can hold; "400 passes allocated exactly what 4 passes
  allocated" is, and it is the same claim, because a per-access cost is O(accesses) and a setup cost cancels
  out of the difference. Pair it with a control arm that *does* allocate per access — the same accesses
  through a rendered key — or a counter stuck at zero passes the test for you.
- **A `-p mwl-stdlib` test can hand a `Core` member a real `callable` without a compiler in front of
  it.** `mwl_runtime::call_closure` reads exactly two things off a closure value — slot
  `CLOSURE_ARITY_SLOT`, and the `CLOSURE_INVOKE` method's address in its class — so a
  `ClassTable::define` + `set_methods` pair with a plain `unsafe extern "C" fn` is a whole closure, and
  everything else in `lower_closure`'s representation is captured state a native callback does not
  have. `crates/mwl-stdlib/tests/allocation_policy.rs`'s `closure_of` is the shape; leak the table,
  because a descriptor's address is its identity. The callee owes the exit sweep — release the
  receiver and each parameter, which `call_closure` retained on the way in — or the case leaks one
  reference per element and the WSL valgrind leg catches it much later.

- **Measuring "no allocation per element" is a *difference*, not a zero, whenever a closure is
  called.** `call_closure` allocates the argument slice it retains, once per call, whatever the
  callback declares — so a key-free `Core\Arr::map` over 64 entries spends 70 allocations, not 6, and
  an assertion of zero fails for the wrong reason. Assert the gap between two arities instead
  (`keyed >= quiet + entries`), and where there is no callback at all — `sort($list)` — assert the
  absolute bound, because nothing is left that could scale with the entry count.
- **An array literal is `array<mixed>`, and a `var` is the function's, not the block's.** Two
  spellings a depth case reaches for and neither is scoped the way it reads. `foreach (["a", "b"]
  as string $t)` is `E0401: expected string, found mixed` — the literal carries no element type,
  so a table sweep declares `array<string> $rows = [...]` on the line above and iterates *that*.
  And a second `for` loop reusing the first loop's `var $t` is `E0406: already declared`, because
  a `var` binding is function-scoped exactly as the `catch` binding the loop goal's standing
  decisions name; give the second loop its own name. Separate `try` statements each binding `$e`
  at file scope are fine — four in one case compile — so it is only `var` that bites.
- **A `catch` binding declared inside a loop body still belongs to the function, so a later file-scope
  `catch` cannot reuse its name.** Several file-scope `try`s each binding `$e` compile — that much is
  already true — but the moment one of them is inside a `for` or `foreach`, every later clause reporting
  `E0406: `$e` is already declared` points at the *loop's* clause as the first declaration. Give each
  `catch` in a case its own name (`$capped`, `$zero`, `$beyond`), which is what the older `Core\Json` case
  already does. In the same family: `Core\Str::repeat` takes a `uint`, so a `for` counter declared `int`
  needs `Core\Str::repeat("[", $d as uint)` and not `$d` — the diagnostic is `E0401`, at the argument.
- **A `.mwlt` case's own helper has to be a `public static function` inside a class.** A case is top-level
  statements, so the instinct when two steps want the same rendering is a plain `function render(...)` at
  file scope — which is `E0215: a function must be a method` (ADR 0011 § 1), caught only when the case is
  run. Wrap it in a `final class` and call it `Render::pairs($m)`; a compiler-owned generic type
  (`Core\ObjectMap<Tag, int>`) is accepted in that method's parameter list, so the helper can take the
  collection the case is about.
- **A case that sweeps a table cannot factor the sweep into a closure.** Two separate walls, one call
  apart: `function (…) { … }` is `E0222` outright (`fn (…) => …` or `fn (…) => { … }` is the one closure
  literal), and the `fn` form then *lowers nowhere* — calling a closure through the variable holding it
  panics `mwl-ir`'s control-flow slice with a bare `got Call { callee: Variable(…) }`, which reads as a
  parser gap rather than as the missing lowering it is. The shape that works is a typed array plus
  `foreach`, with the `try`/`catch` inline in the loop body: `array<string> $rows = ["…", …];` — **no
  `var`**, because an array literal is `array<mixed>` and `var` refuses to infer an element type
  (`E0414`), and `foreach ($rows as string $row)` over the literal directly is `E0401` for the same
  reason. Counting agreements into an `int` declared above the loop is how the sweep is then asserted,
  since there is no compound assignment either.
- **Deepening a `.mwlt` case in place does not move Stage 4's count.** The gate counts case *files*, so a
  depth slice that rewrites an existing thin case makes real progress the acceptance test cannot see —
  `mwl test tests/conformance` read 478 both before and after two sessions' worth of work. Land the new
  claim as its **own file**, named for the claim, and leave the thin case where it is with a one-line
  comment pointing at the deep one. Splitting after the fact is free; noticing after the run is not.
- **An `--ORACLE--` helper must not be named after a PHP built-in, and the failure does not say so.**
  `pos` is an alias of `current()`, so a case whose oracle declared `function pos(int|false $f)` failed
  with `--ORACLE--: PHP exited 255` and **`php stderr: <empty>`** — PHP writes *Cannot redeclare
  function* to stdout, which the runner is comparing rather than reporting. The MWL half compiles and
  runs, so the failure reads as a broken PHP install. Name an oracle helper for what it renders
  (`render`, `show`) and check `php -r 'var_dump(function_exists("<name>"));'` if in doubt; `key`,
  `next`, `end`, `reset`, `current` and `compact` are the other easy collisions.
- **A differential case checks itself, so write the rows and run it rather than pricing PHP's answer
  by hand first.** An `--ORACLE--` case's failure output prints both columns side by side, which is
  the whole comparison in one call; three `php -r '…'` calls spent pre-computing what a matching case
  was going to assert told this session nothing the first `mwl test <case>` did not. Reach for
  `php -r` for the *divergence* half instead, where the frozen `--EXPECT--` is MWL's own output and
  PHP's answer only appears in the case's prose — that is the one place the runner cannot check the
  sentence you wrote.
- **An `int` literal does not reach an `array<float>`'s element type**, so a differential case about MWL's *one numeric domain* has to declare the subject `array<int|float>`. `Core\Arr::contains($floats, 1)` against an `array<float>` is `E0401: expected float, found int` at the argument — the needle is typed `T`, and the widening `1 == 1.0` gets in an expression is not one an argument position performs. Declaring `array<int|float> $numeric = [1.0, 2.5];` makes `T` the union, the literal fits, and `contains($numeric, 1)` then answers `true` — which is the ADR 0090 § 3 row worth pinning, since `in_array(1, [1.0], true)` is `false` and the loose `in_array(1, [1.0])` is `true`, so MWL matches neither of PHP's two modes.
- **A `Core` member's refusal is a `FATAL:` line on standard error, not a `Throwable`, so `try`/`catch` cannot pin it.** `Fault::fatal` is what `key_bytes` and every argument-shape guard in `mwl-stdlib` raise, and it unwinds past `catch (Throwable $e)` untouched: a case wrapping `Core\Arr::countBy($floats)` in a `try` prints nothing from its handler and exits 1. Pin it with `--EXPECT-ERROR--` instead, whose presence is also what tells the runner this case's run is *meant* to fail — the stdout before the fatal still has to match `--EXPECT--`, so the agreeing rows can sit in the same case. Get the message by running the scratch under `2>` and `cat -A`: it is one line, `FATAL: ` then the member's own text, and it can carry an internal detail (`got tag 4`) that no other section would let you assert.
- **A `Core` member's *ordering* refusal is the other kind of `Fault` and a `catch` does reach it.**
  `Fault::thrown` — what `mwl_stdlib::ordering::compare_values` raises for a pair with no natural
  order, and so what `Core\Arr::min`/`max`/`sort` raise over a mixed-type subject — unwinds as an
  ordinary `Throwable`, so `try { … } catch (Throwable $e) { echo "refused\n"; }` at file scope
  prints and the case carries on. That is the opposite of the neighbouring bullet's `Fault::fatal`,
  which no handler sees, and it is what lets a divergence case render its refusals inline beside its
  agreeing rows instead of ending at an `--EXPECT-ERROR--`. Which one a member raises is decided in
  the helper, not by the member, so check the `Fault::` constructor at the site rather than assuming.

- **A `?bool` cannot be tested for truth, so a member answering one has no `yn` rendering at all.**
  `if ($found as bool)` on a `?bool` parameter panics `mwl-ir`'s truthy-condition slice at
  `lower/expr.rs:1036` with *"got Tagged"* — the `as bool` does not narrow the binding out of
  `Ty::Tagged`, and the guarded-branch conversion that works for `?int` and `?string`
  (`return $found as string;`) has no counterpart here because the condition is what fails. The
  `yn` helper in `arr-any-and-all-match-array_any-and-array_all` takes a plain `bool` for this
  reason. Keep a `bool`-valued subject out of a case about a `?T`-answering member, or render it
  through a member that answers `string`.
- **Neither leg's PHP has `gmp`**, so `Core\Math::gcd` and `::lcm` have no callable twin: `gmp_gcd`/`gmp_lcm`
  die with *"Call to undefined function"* on the Windows `php` and inside WSL alike, exactly as `mb_*` does
  on Windows. One `php -r 'echo function_exists("gmp_gcd");'` before designing the case is the check; when
  the twin is missing, either compute the expectation with an explicit loop in the `--ORACLE--` block — PHP
  still computes it, so the case is a real differential — or leave the member for a conformance case and say
  so in its comment. `bcmath` *is* present on both legs.

- **An `--ORACLE--` case must never `echo` a `NAN`.** PHP 8.4 and later emit *"Warning: unexpected NAN value
  was coerced to string"* onto the same stream as the output, so the oracle's expectation carries a warning
  MWL's side has no way to print and the case fails on a row that actually agrees. `INF` is fine. Render the
  value through a guard instead — `$v == $v` is false for exactly one `float`, on both sides — and echo a
  sentinel, which is what `math-int-div-and-mod-match-intdiv-and-fmod`'s `Show::real` does.

- **A `CoreTy::Var("T")` signature binds `T` to the first argument, so a mixed-type pair does not reach the
  runtime at all.** `Core\Math::min`, `max` and `clamp` declare every parameter and their return as one
  `Var("T")`, so `Core\Math::min(0, "a")` and even `Core\Math::min(2, 1.5)` are `E0401` at the *second*
  argument — which reads as "this member rejects the pair" when what is actually wanted is
  `mwl_stdlib::ordering::compare_values`'s throw. Declare the union on the bindings (`int|string $zero = 0;`)
  and the same call becomes the runtime refusal the case is trying to pin. In the same family: a `float`
  parameter does not widen an `int` literal, so `Core\Math::mod(7, 2.0)` is `E0401` and `7 as float` or a
  `float` binding is the spelling.
- **A PHP notice or deprecation lands on the oracle's *stdout*, so an `--ORACLE--` case that trips one
  can never match.** PHP's base conversions are the sharpest instance — `hexdec("beefy")` and
  `base_convert("-255", 10, 16)` each emit `Deprecated: Invalid characters passed for attempted
  conversion, these have been ignored` before answering — but the rule is general: any input a twin
  *repairs* rather than refuses is a candidate. That is usually the signal to split the case, putting
  the repaired inputs in an `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--` and leaving only the
  quiet rows under `--ORACLE--`. Check by running the oracle body through `php -r` while authoring: a
  notice is visible there and invisible in the `.mwlt` diff, which reports only that the two outputs
  differ.
- **A frozen `--EXPECT--` cannot hold a decomposed grapheme cluster, and nothing warns you.** `"cafe\u{0301}"` sliced at its last cluster renders as `é` — byte-identical in a terminal to the precomposed `é` a keyboard types into the expectation block, and a different string. The case reads as passing-looking and fails with an "expected"/"actual" pair whose two halves are visually the same, which is a long minute to diagnose. Echo `Core\Encoding::toHex($s as bytes)` for any cell whose content is not plainly ASCII; the hex is also the thing a reader of a grapheme-versus-byte case wants to see. The neighbouring rule about a trailing space before a `\n` is the same class of trap.

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
- **Adding a dependency costs a whole `target/` generation, and cargo never collects the old one.** A
  crate's artifacts are named `<name>-<metadata-hash>` and that hash covers the dependency graph, so a
  `Cargo.toml` or `Cargo.lock` edit orphans the previous set for every crate downstream — permanently, on
  stable. Editing *source* is free: a source-only rebuild reuses every hash and adds nothing. On a
  milestone that adds a crate most sessions, that is a generation most sessions, and it is how this tree
  reached 20 GB and zero free disk on 2026-08-25. You do not need to do anything about it in a session —
  the driver refuses to *start* a run under 10 GB free and says what to run — but if you are the one who
  hits it, `python tools/disk.py --clean` is the answer, not `cargo clean`.
- **A `#[global_allocator]` declared in a *library* crate is only picked up by a binary that
  actually links that crate.** `mwl_runtime::alloc::Pooled` is registered from `mwl-runtime`'s
  own `lib.rs`, so every binary in the workspace gets it — except one whose sources never name
  `mwl_runtime`, because rustc links an `--extern` crate lazily and an unlinked crate is not in
  the graph the allocator is chosen from. `benches/abi-probe/tests/perf_guards.rs` happens to
  name it; a *new* test binary measuring allocation might not, and would then silently measure
  the platform heap. `an_allocation_round_trip_stays_in_the_pooled_cost_class` is written to fail
  loudly in exactly that case — its two sides become the same code, so the ratio goes to 1.
- **A named `const` holding a `Cell` is `clippy::declare_interior_mutable_const`, which is denied
  here.** The obvious way to build a `thread_local!` array — `const EMPTY: Class = …;` then
  `[EMPTY; N]` — is refused, because a constant is *copied* at each use rather than referenced.
  The spelling that works is an inline const block in the repeat, `[const { … }; N]`, which is
  also a const-repeat of a non-`Copy` type and so still `const`-initializes the thread local.
- **A block-bodied closure must write its return type, and `fn () => { … }` is `E0450`.** The
  spelling is `fn (): void => { echo "x"; }` — an *expression*-bodied closure infers its type from
  the expression, a block-bodied one cannot, and every `Core` member taking a `callable` whose
  callback does work rather than computing a value meets this. `examples/collect.mwl`'s
  `Core\Out::capture` line was written the short way and sat there uncompiled for several
  sessions, which is the next bullet's fault as much as this one's.

- **One compile error hides every later one, so "the first red fixture line" moves *backwards* as
  you fix it.** `examples/collect.mwl` was recorded in three places as failing at line 47 on a
  missing `Core\Out::capture`; the truth was a **parse** error at 47 that suppressed name
  resolution entirely, and behind it sat a `Core\Uuid::isValid` at line 29 that the spec says does
  not exist and an `Arr::first` subscript at 45 that panics `mwl-ir`. Budget a fixture as "run it
  again after every fix until it exits 0", not as "one report, one slice" — and do not trust a
  handoff's claim about which line a fixture stops at without running it.
- **`var` takes no type annotation, and writing one costs four diagnostics a line.** `var string $s = …`
  is not a declaration with a redundant type: the parser reads `var`, expects a name, finds `string`, and
  emits `E0101` twice, `E0102`, a third `E0101` and then an `E0406` claiming `$` is already declared — per
  line, so a six-line scratch file came back with 24 errors and none of them said "a `var` has no type".
  The two spellings are `var $s = …` (inferred, and what every `.mwlt` case writes) and `string $s = …`
  (declared). The sentence to look for in the wall of output is the second `E0101`'s, "`var` infers its
  type from the initializer".
- **`int`'s own minimum has no literal spelling, so a case pinning a 64-bit field's lower bound
  cannot write it.** `-9223372036854775808` is a unary minus applied to a literal that is already
  too large for `int`, so it is `E0429` (*"only legal where a `uint` is expected"*) rather than
  `int::MIN`. That matters wherever a bound is being asserted on both sides: for `Core\Bytes::pack`'s
  `J`/`P` the accepted range is the union of `int`'s and `uint`'s, so **neither** refusal has an
  argument that can be written, and the honest row asserts the reach — `-9223372036854775807` and
  `18446744073709551615` both landing in the field — instead of inventing a refusal that cannot
  exist. Narrower widths (`C`, `n`, `v`, `N`, `V`) have both sides spellable and should assert them.
- **The first-class callable spelling `Class::method(...)` panics `mwl-ir` outright** — *"a static call
  has no resolved target recorded in the typed-expression table"*, which reads like a checker/lowering
  mismatch rather than a missing feature. It is the same hole as `mwl-ir` gap 1: a case cannot name one
  callback and hand it to several members, so every callback in a sweep is written inline at its call
  site. A `public static function` in the case file is still callable *directly*; it is only the
  reference-to-it that does not exist.
- **`bool as string` renders `false` as the empty string**, so a line built out of `as string` over
  predicates silently loses its false columns and still looks like a shorter tally. `bool as int` does not
  lower at all, so the way to *show* a predicate's answer is a two-line helper that branches and returns a
  character.
- **A `Core` member answering a union cannot be handed straight back to a parameter declared `T`.**
  `Core\Arr::append($a, Core\Arr::sum($empty))` is `E0401: expected int, found int|float|decimal`,
  because `sum`/`product`/`average` answer spec § 2's whole union whatever their subject's element type
  was, and `array<int>`'s `T` is `int`. There is no narrowing spelling for it either — `as int` over a
  union does not lower. So a case that wants to feed a fold's answer back into the array writes the
  literal (`0`, `1`) and asserts *separately* that the member answers it, which is two claims where one
  was wanted but is the only pair available. Rendering the union is fine: `echo` takes it, and so does
  `as string`.
- **An array literal written straight into an `array<array<mixed>>` element reads as `array<mixed>`,
  and then does not satisfy an `array<array<T>>` parameter.** `Core\Arr::flatten([$s, $s])` inside a
  `array<array<mixed>> $answers = [...]` literal is `E0401: expected array<array<mixed>>, found
  array<mixed>` pointing at the *inner* literal — the outer literal's expected element type is what
  the inner one is checked against, so the nesting the argument needs is one level short. Bind it
  first (`array<array<string>> $pair = [$s, $s];`) and pass the binding; the same literal in a
  `var`-free typed binding infers exactly what its declaration says.
- **An option bag's value may be a variable, and a `uint` parameter accepts an integer literal at
  the call site.** Both were unknowns worth one scratch run: `Core\Arr::from($c, {limit: $limit})`
  lowers with `$limit` a `uint` parameter, so a swept bound does not need one call site per value,
  and `Drive::at(0)` against `public static function at(uint $limit)` needs no `as uint`. The
  brace literal is an expression like any other — only its *keys* are fixed by the member's row.
- **A `== null` guard does not narrow a `?T` binding — `as T` is what states the narrowing.** The
  shape every `?T` differential case needs is a helper that renders absence beside PHP's `false`, and
  the obvious spelling does not compile: `if ($found == null) { return "none"; } return $found;` is
  `E0403: this method declares `string` but returns `string|null``, and inverting it to
  `if ($found != null) { return $found; }` fails identically at the same line, so the guard is not
  flow-narrowing at all. `return $found as string;` compiles, and it is total in that branch and a
  throw in no other. A `?uint` needs the same cast for a different reason (there is no `uint` row in
  `echo`), so one `Show::render(?T $found): string` covers both and is worth copying between cases.
- **A `--release` acceptance check costs a thin-LTO relink of every test binary in the package, not
  just the one holding the guard.** Touching `crates/mwl-runtime/src/lib.rs` and rebuilding
  `mwl-abi-probe` measured 200s for the package and 133s for `--test perf_guards` alone: five binaries
  at ~17s of link each, over ~116s of compiling six crates at `codegen-units = 1`. So a `--release`
  check in `loop-goal.toml` names its test *file*, and anything in the package that is not a cost
  guard gets a second, debug check — 29s for all of `mwl-abi-probe`, with the guards skipping
  themselves through `#[cfg_attr(debug_assertions, ignore)]`. Do not reach for a cheaper profile
  instead: the cost class is a claim about the profile MWL ships, so `lto`/`codegen-units` are the
  measurement and not overhead on it. What is left after narrowing is a build, and a build overlaps:
  `loop.py` starts it before the native build and runs the check last, which took a cold sweep from
  326s to 183s. Two cargos on one `target/` do not block each other — only the registry's package
  cache is briefly contended, and the debug half finishes in its usual time.
- **Measure a build with nothing else touching `target/`.** The same narrowed release build timed
  133s alone and 260s with a `du -sh target` walking the tree beside it. On a link-heavy build the
  disk is the contended resource, so a second reader of the same 1.7 GB doubles it — a timing run
  that disagrees with an earlier one by 2x is usually this and not the change under test.
