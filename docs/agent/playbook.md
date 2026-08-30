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
something you are about to write — a commit message, a `.nvst` case, a `Core` member, an ADR — is
[conventions.md](conventions.md). What is left — "this looks like it should work and does not, and here
is why" — is this file.

## Tooling

- **`cargo test -p <crate>` straight after a workspace `cargo test` recompiles the crate, and the
  workspace run then recompiles it back.** A package selected alone unifies its dependencies' features
  differently from the whole workspace, so its `-p` artifact is a second one that every source edit
  stales. Measured: `touch crates/nvs-types/src/lib.rs`, `cargo test --no-run` (11s), then
  `cargo test -p nvs-types --no-run` — `Compiling nvs-types`, 5s, on a tree cargo had just built. The
  loop's acceptance check paid this seven times a session, 28s of rebuilds in front of 25s of tests,
  which is why `Goal.crate_tests` in `tools/loop.py` runs a crate's test executables off one workspace
  build instead. When you scope by hand, scope `verify.py -p` and every `cargo` call the same way for
  the whole session, or budget the rebuild each time you switch.
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
- **`cargo test` does not always relink `target/debug/nvs.exe`** — `cargo build -p nvs-cli` before running
  a fixture or a `.nvst` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. Whole suite:
  `python tools/loop.py --leg-only` (background it; minutes) — every fixture, both suites and the
  valgrind sweep against a Linux build, and it needs no script because it drives `wsl.exe` for you.
  One fixture: `tools/leak-check.sh <paths>` — a file passed by path, for the reason above — and it
  takes `.nvs` files only, so a `.nvst` passed to it reports a failure that is not a leak.
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
  `-p nvs-stdlib --lib` arrived this session from the previous one's `validate.rs`, and the first
  instinct — "my registry rows broke something" — costs a bisect. `git status --short` showing that
  file unmodified is the whole diagnosis. Fix it, but in **its own commit**, so `git log` does not
  read as though the feature slice touched it.
- **`plan.py --get <Field>` prints a field whole, so weigh it against what the field costs.** It was
  once flatly not worth it: `Open now` had grown to 44 KB — one logical line of about 11,000 tokens,
  five per cent of a session's ceiling to confirm a sentence you already wrote. That field is 5 KB
  now (its record of landed work is playbook bullets), so `--get` is the right call when you need
  the field's exact wording, which is what a `## plan-edit:` fragment has to quote. To confirm an
  edit *landed*, still `grep -n` the phrase in `docs/implementation-plan.md` — the surrounding
  `> `-prefixed lines are the same fact for a fraction of the cost, and `--wrap` already told you.
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
- **`nvs-ir` cannot name `nvs_hir::QName`** — `nvs-hir` is a *dev*-dependency there, on purpose, so a
  lowering helper that wants one in its signature does not compile even though `nvs_types::Ty::Enum`
  hands it a `&QName` to pattern-match. Destructure it at the call site and pass what the callee actually
  needs (an `&EnumInfo`, or the name already rendered with `to_string`); `nvs_types` re-exports the enum
  and layout tables but not `QName`, and adding the dependency to get one is the wrong direction.
- **Teaching `nvs-ir` a new receiver or value shape is two edits, and the second one panics somewhere
  else.** Recording a new `ExprInfo` variant gets the *access* lowering; what still fails is
  `erase_checked_ty`, which owns the `nvs_types::ty::Ty` → `nvs_ir::Ty` representation map, and whose
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
  test needs its old `.snap` deleted. `cargo test --release -p nvs-abi-probe` takes over two minutes.
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
  `benches/userland/` — Novis and PHP benchmark *sources*, left untracked by an earlier session — made
  every `cargo` invocation die with `failed to read benches/userland/Cargo.toml` before a single crate
  was read. Piped through `| tail -3` the error scrolls past, `$?` belongs to `tail`, and the run that
  follows uses whatever `target/debug/nvs.exe` was built last, so the session diagnoses a phantom
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
  per-class equality hook in `nvs_runtime::identity` and re-opened an ADR from inside the loop.
- **`verify.py` can be red on a tree you did not touch, and `fmt` is where it happens.** A docs-only
  session hit `cargo fmt --check` failing on committed code — a four-line `nvs_array_get_index` signature
  rustfmt wanted on one line — which means the slice that added it was committed without step 3 ever being
  green. Do not treat that as "my change broke it" and do not skip the fix: it is one hunk, it goes in its
  own commit named for what it is, and the session's own slices stay clean. Check the blast radius first
  with `grep -c "^Diff in" .agent-tmp/verify-fmt.log` — one file means fix it here, a dozen means say so in
  the handoff instead of reformatting the workspace inside an unrelated slice.
- **Widening an operand's *representation* in `nvs-ir` moves a refcount decision you did not edit.**
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
  because every other `.snap` still carried `source: crates/nvs-ir/src/lower.rs` from before that
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
  2026-08-26 run all reported `nvs-runtime (string capacity): test '…' did not run`, and for those
  seventeen sessions Stage 4's two counts, Stage 5's guards, the WSL leg and the valgrind sweep
  never ran: the case counts the ledger quotes were session prose rather than the gate, and the run
  could not have stopped even had the goal been reached. That one was a stale *spec* — `loop.py`
  read `loop-goal.toml` once at start-up, so the rename the bullet above records never reached the
  running driver, and `drive()` now re-reads the list before every check. A repeat can just as
  easily be a real red check nobody has opened. Either way `python tools/loop.py --goal-only`
  answers it in one call, and it is worth one call the moment the same line lands twice.
- **`gaps.py --errors` matches a site by the literal run of its message *before the first
  format hole*, so closing one member can silence siblings that are still unasserted.** A case
  echoing `Core\Time\DateTime::format(): …` contains the stem `Core\Time\DateTime::`, which is
  also the whole stem of every message spelled `Core\Time\DateTime::{member}(…)` — so the
  session that closed the three `format` refusals took **twelve** rows off the list for
  **seven** real closures, five of them (`time.rs:1968` and four `fatal` at `:1950`-`:1973`)
  merely hidden. The list is a worklist, not a ledger: when the drop is larger than the number
  of sites you asserted, diff it against the tree with your new cases moved aside and name the
  hidden ones in the handoff, or the next session inherits a shorter worklist than the tree has.
- **Never write a `## plan: Open now` section into a wrap file.** That field is 40 KB, `--wrap`
  replaces a field whole, and retyping it is both a fifth of a session's ceiling and a chance to
  silently drop a paragraph. Edit the one sentence in place in `docs/implementation-plan.md` with
  the Edit tool instead — `grep -n` the phrase, which the neighbouring bullet already recommends
  over `--get`. The one rule a hand edit has to respect: **a bare `>` line is the field separator**,
  so a blank line added for readability inside a field splits it in two and `plan.py --check`
  reports six fields where there were seven. Keep the paragraph continuous, and run
  `python tools/plan.py --check` after — it prints the field count and each field's size.

- **A plan field is patched, not retyped: `## plan-edit: <Field>` with `--- old` / `--- new`.**
  `## plan: <Field>` still exists and still replaces the field whole, but a replacement over 1.5 KB
  whose 8-word runs are 70% already on disk is refused as a retype and names the patch form. That
  is not pedantry about bytes: over one 21-session run `## plan:` sections were 160,388 B of wrap
  payload, 45% of everything those sessions wrote into a wrap file, one of them 33 KB in a single
  Write that cost 189 seconds — and re-emitting a field by hand is exactly where a paragraph gets
  silently dropped. Quote the `--- old` fragment as the field *reads*, which is one single-spaced
  paragraph however it is wrapped on disk (`python tools/plan.py --get "Open now"` prints it); it
  must match exactly once, and the refusal says whether it matched none or several. A deliberate
  *trim* is exempt — cutting a field to under 60% of its size is all-verbatim by definition and
  goes through as a `## plan:` — and so is a real rewrite, which overlaps less than 70%.
- **`gaps.py --differential` reads a `Core\X::member` spelling *inside a comment* as a call**, so a
  case whose prose names a neighbouring member silences that member's own row. `called_members`
  (`tools/gaps.py:159`) is one regex over the whole case text and never looks for a `(`. One
  sentence of comment in a new `Core\Path::basename` case — "only `Core\Path::normalize` resolves
  them" — took the list from 8 members to 5, which reads exactly like a session that closed three.
  Name a neighbour without its class (`normalize`), or cite the case *file* that owns it, and re-run
  `--differential` after writing a case so the drop you see is the one you earned.
- **`gaps.py --differential` matches text, so a member can leave the list without a case of its
  own.** The `Core\Json::isValid` oracle's fallback calls `json_decode` — `json_validate` is PHP
  8.3+ and the other leg's version is unknown — and that one call dropped **`Core\Json::decode`**
  from the list too, in the same run, before anything pinned it. The list is a worklist and not a
  ledger (the plan's *Open now* says so for the `--errors` half as well): when the drop is larger
  than the number of members the session actually asked about, the extra one is still owed a case.
- **A fact restated across ADRs goes stale, and the two shapes that always do it are a running
  count and an ordinal.** ADR 0071 § 1 said its closed attribute list "opens with exactly those four
  names", 0077 § 1 said it "now holds five", and 0085/0086/0096 each added entries without touching
  either — while the typed-`callable` deferral had a *forcing case* ordinal chained across four ADRs
  (0031 "second", 0061 "second", 0072 "third", 0077 "fourth"), so withdrawing one link renumbered
  nothing and left a wrong count in two places. Before adding to any roster, grep for the total:
  `grep -rn "holds .* names\|forcing case\|opens with exactly" docs/adr/`. The fix that sticks is a
  **table in the owning ADR** plus a rule that amending ADRs say "joins the list" and state no
  number — 0071 § 1 is now that table, and it names `nvs_types::derive::ATTRIBUTES` as the registry
  it must agree with.
- **A registry-reading tool can be blind to most of the registry and say so in a confident total.**
  `gaps.py`'s `CLASS_RE` matched only the inline spelling `CoreClass { name: r"Core\Arr"`, and the
  majority of the tree names its class through a file-level const instead (`name: NAME`, with
  `pub(crate) const NAME: &str = r"Core\Csv";` above it). So the tool saw **7 of 27 classes and 164 of
  318 members**, and every list it printed was silently that short — `--differential` reported the
  oracle gap **closed** when 9 members of `Core\Time` and `Core\Encoding` had never been looked at, and
  that empty list was copied into the plan and the handoff as a met gate. Nothing was wrong with the
  count it printed; it was a true count of what it could see. When a tool reads Rust with a regex,
  check what it found against the thing it is reading — `registry()` against `grep -c CoreClass` — before
  believing a total, and give an unresolvable name an empty owner rather than letting its members fall
  to the class above it.
- **`gaps.py --coverage` attributes a case to a class only when the case *text* names that class,
  so a class whose values are never spelled out reads as zero and its "no case calls" list is a
  false alarm.** `Core\Time\Instant` prints `0.00  0  9 … no case calls compareTo, in, minus` while
  three cases exercise all nine members — because an `Instant` is obtained (`Core\Time::now()`,
  `->toInstant()`) and never written, so the `re.escape(owner)` filter at `tools/gaps.py:301` keeps
  none of them, and the `->member(` scan only runs over the cases that filter kept.
  `Core\Time\DateTime` reads 0.06 for the same reason. The number is trustworthy for a class a
  program has to name — `Core\Time\Duration::ofSeconds`, `Core\Uri::parse`, `Core\Validate` — and
  is a floor, never a count, for the rest. Check with `python tools/gaps.py --member compareTo`
  before writing a case the ranking says is missing.
- **An ADR index **Decision** cell is derived from the ADR's own title, not written.** `adr.py --check` reports the whole index table stale when a cell says anything else, and the message names `--index` without saying why the row you just added is the one it dislikes. `python tools/adr.py --index | grep NNNN` prints the row it wants; paste that. Writing a richer sentence there and letting the title stay short is the natural move and it fails every time.
- **`alloc::Pooled` recycles a freed block, so a memory checker over it cannot see a
  use-after-free — and exactly one leg of four is affected.** ASAN and valgrind both work on memory
  that reaches `free`: one poisons it and quarantines it, the other unmaps it. A block Novis frees
  goes onto the per-thread size-class cache instead, so a read through a dangling pointer lands in
  live, mapped memory and neither tool says a word. The leg that matters is **`cargo test -p
  nvs-runtime`**, where `cfg(test)` installs `counting_alloc` over `Pooled` — the crate holding
  most of this tree's `unsafe` was the one whose own tests checked the least. `--features
  nvs-runtime/sanitizer` swaps the backing allocator for the platform heap, and the `asan` CI job
  passes it. **The easy mistake is extending that conclusion to the other three, which are all
  fine**: `tools/loop.py`'s valgrind sweep runs `target/debug/nvs` and a debug build is deliberately
  left on the platform heap, and `nvs-codegen`/`nvs-stdlib` link the runtime with `cfg(test)` off,
  so every one of those sees every free already. Before changing any of it, read `counting_alloc`'s
  module doc, which is the home of the reasoning — the wrong version of this bullet costs a session
  either way round.
- **Making a previously infallible instruction fallible moves two guards that name neither it
  nor the operator.** ADR 0007 § 4's overflow throw gave `+`/`-`/`*` and unary `-` an
  `Inst::on_error` edge, and the build then failed twice a long way from the change.
  `benches/abi-probe/tests/perf_guards.rs`'s `a_typed_arithmetic_loop_contains_no_call` counts
  machine-code `call`s against IR probe/safepoint sites, and each new edge adds *two* — the
  cold block's `nvs_raise_new` and the landing block's `nvs_trace_push` — so its accounting
  needs a term per category, not a bumped number. And `nvs-ir`'s
  `a_hook_body_reaching_its_own_property_touches_the_slot_directly` asserted "the body does not
  contain this function's name", which a landing block's `propagate "Box::$n::get() at …"` frame
  label now satisfies without any recursion existing; read for a `call` naming it instead. Both
  are string-shaped assertions over rendered IR, so `grep` for the *edge* (`! bb`, `propagate`)
  rather than for the operator when a change widens the fallible set.
- **A `python - <<'PY'` heredoc mangles non-ASCII in the *matched* string, so a `str.replace`
  whose `old` contains a `§` or an em dash silently finds nothing.** Writing one out is fine —
  the same script wrote `§` and `—` into three files correctly in the same session — but an
  `assert s.count(old) == 1` over a fragment quoted from a doc comment fails with no clue why,
  and the obvious next move is to re-`grep` the file and confirm the text *is* there, which
  costs two more calls. That is AGENTS.md rule 1 collecting its price: use Edit for anything
  whose anchor is prose. A heredoc is still fine when every matched byte is ASCII, which is
  what the `Helper::X => "symbol"` table edits in this session were.
- **`nvs-ir` cannot synthesize a local**, so "evaluate the base into a temporary and rewrite the
  target over that" is not a move a lowering has. `ExprKind::Variable` holds a `Span` and
  `lower_expr` reads the name back out of `self.src`, so a name no source file spells has no
  representation at all — and `ExprKind::Int` is a span too, which is the same wall an increment's
  implicit `1` hits. What replaces it is `Lowering::staged_targets`: lower the sub-expression once,
  record `(span, value, ty)`, and let `lower_expr` answer from that table *before* it looks at the
  expression's kind. Two halves make it safe and neither is optional — `aliasing_read` must answer
  `true` for a staged span, or the second read releases a base the first one still needs, and the
  stager must `own_temporary` a refcounted staged value, because it is a fresh producer precisely
  when it could not be re-read.
- **`InstKind::ArrayGet` answers a missing key with `Value::default()`, so any lowering that
  descends through one and then treats the result as a pointer aborts the process.** The
  symptom is not a null-deref: it is *"an Novis array pointer is never null"* followed by
  *"panic in a function that cannot unwind"* and exit 127, from inside `nvs_array_set` —
  which reads as a runtime bug in the array module rather than as the missing feature it is,
  because the frame that produced the null is three instructions upstream and long gone.
  `nvs_array_get`/`nvs_array_get_index` both end in `.unwrap_or_default()`; that is the whole
  recognition test. The general shape of the fix is a helper whose ownership answer is the
  *same* in the present and the absent case — `Helper::ArrayRowForWrite` retains what it found
  or allocates what it did not, so the caller emits no retain and needs no branch — and the
  general rule is that a borrowing read is not a building block for a write path, however
  well it reads.
- **A heredoc through the shell doubles a backslash, and a Rust `\`-continuation survives
  the doubling as a literal `\n` that still compiles.** AGENTS.md rule 1 — "a shell never
  carries file content into the tree" — is protecting you from exactly this, and the
  failure is silent: a rewritten `panic!` string came back as one physical line with `\n`
  printed as two characters, `cargo build` was green, and only reading the region back
  showed it. Three calls to undo. Use Write/Edit or `python tools/splice.py`; if an edit
  genuinely has to be scripted, build the backslash as `chr(92)` rather than writing one.
- **`grep -rn <pattern> .` from the repo root walks `target/` and times out.** It is minutes of
  I/O over build artifacts for an answer the source tree gives in under a second, and the shell
  call comes back with nothing at all. Name the roots (`grep -rn <pat> crates docs tests`), or use
  the harness `Grep` tool, which respects the ignore file. `python tools/peek.py
  "crates/**/*.rs:re:pattern"` is the version that answers several such questions in one call.
- **`holes.py --item N` groups a refusal site by the *file* an item anchors, so an item's own
  site living in a file another item names is silently filed under that other item.** Item 19
  printed three sites and had four: the closure-capture panic is in `lower/expr.rs`, which item
  17 anchors, so it is listed there and the item reads as closed when its headline half is not
  written. The tool's totals are right; only the attribution is a guess. When an item's prose
  names a shape, grep the shape (`grep -rn '&\$x' crates/nvs-ir/src`) before believing the site
  list is the whole item.
- **`holes.py` counts a site by the *words* in its message, so an assert that has become an
  internal-consistency check still reads as a hole until it stops saying "does not lower".** The
  regex is `does not (yet )?lower|only lowers|no lowering for` (`tools/holes.py`, `REFUSAL`), and
  nothing else marks a site as decided — there is no allowlist and no attribute. So the last edit
  of a slice that answers a hole *at the checker* is to reword the assert the way item 19's three
  were reworded: state what reached lowering and name the code that refuses it ("… reached
  lowering: … refuses this where it is written, as `E0494`"). Skip it and the worklist you quote in
  the plan is one higher than the tree.
- **`Lowering::untag_receiver` is unchecked, so a *new* erased receiver may not go through it.**
  Every tagged receiver that reached a member used to arrive with a tag `nvs_types` had already
  proved — a narrowed `?T`, a `?->`'s non-`null` arm — so the untag compares nothing and
  `open_nullsafe` does it for its caller without being asked. A `mixed` receiver is the first with
  no such proof, and the failure mode is not a panic you would see in a test log: `Untag` over an
  `int` payload is a pointer the next instruction dereferences. `ReceiverProof` in
  `crates/nvs-ir/src/lower/expr.rs` is the switch, and the check belongs in the runtime helper that
  already checks the *name* rather than in a fallible untag of its own — a throwing untag would
  have to have the receiver staged as an owned temporary before it, which is an ordering nothing
  else in that file has.

- **A panic's own message names one route to it, and there is usually a second.**
  `lower_property_access`'s catch-all said "erased to a plain `object` or to a shape that does not
  name this field", and the worklist recorded the one live route as a `mixed` receiver — but
  `int $i = 5; echo $i->name;` reached the identical line, because *every* receiver
  `class_qname_of` cannot resolve falls through the same hole. Three `nvs run` calls on scratch
  files, one per receiver family, cost less than the fix did and are what turned a one-answer slice
  into two: a lowering for the case that must be deferred, a diagnostic for the ones a declared
  type already answers. Do that sweep before deciding what a refusal site owes.
- **Only the *outermost* annotation has a recorded checked type, so `lower_decl_type` over a
  nested `Type` node silently answers from the AST instead.** `nvs_types::lower::lower_type`
  calls `record_type` once, at its own entry point, and recurses through `lower_type_at_depth`
  without recording — so a lowering that reaches *inside* an annotation (`?T`'s target, a
  union's member, an `array<T>`'s element) gets `lower_decl_type`'s `match &ty.kind` fallback,
  where every name-shaped atom is `Ty::Object` whether it names a class or an enum. `$m as ?Mode`
  died on `Tagged as ?Object` for exactly that reason, and the message points at a missing
  conversion row rather than at the missing table entry it actually is. Read the **whole**
  annotation's `declared_ty` and take the piece you want off the checked type — `?T` interns as
  `T|null`, so its target is that union minus `CheckedTy::Null`.
- **A refusal for a *name-shaped* expression fires on every `Foo::bar()` in the program unless
  the class side is taken off the value walk first.** `Class::method()`, `Class::CONST`,
  `Class::$prop`, `Class::class` and `$x instanceof Class` all carry the class as an ordinary
  `Expr` whose kind is `ExprKind::ConstFetch` — and `self`/`static`/`parent` are the same three
  shapes by a second route — so both walkers that see one, `nvs_hir::members::walk_expr` and
  `nvs_types::expr::check_expr`, recurse into it and an arm added for "a bare name is not a
  value" reports there too. `nvs_hir::members::walk_class_side` is the fix on the resolver side,
  one helper over five call sites; `nvs_types` has the identical five
  (`expr/mod.rs:264`, `:292`, `expr/calls.rs:143`, `expr/members.rs:85`, `:211`) and has not
  needed it only because its arms answer `mixed` in silence. Nothing catches this: it builds, and
  the first sign is a conformance case that used to pass reporting an extra error.
- **A value handed to an ADR 0014 § 1 `set` hook is *transferred*, so there is no "afterwards" in
  which to retain it.** Every other assignment target leaves the target itself owning what was
  stored — a local's slot, an `inout` pointee, a field, an array entry — so a lowering that wants a
  second owner of the stored value can retain once the store has run, and four of the five arms of
  `Lowering::lower_store` are safe that way. A `set` hook is a **call**, and `nvs-ir`'s argument
  convention gives the callee the reference: a retain emitted after that call can read a value the
  hook already released, and nothing catches it — it builds, it runs, and only a valgrind fixture
  whose hook *discards* its argument shows anything. That is why `lower_store` takes an
  `extra_owner` flag instead of handing the value back for its caller to retain: the retain has to
  be emitted where each arm still holds a reference, which for that one arm is *before* the call.
- **A `splice.py` patch cannot carry a patch, so an edit to prose *about* the patch format is one
  the `Edit` tool has to make.** A block runs from `<<<<<<< OLD` to the first `=======` after it,
  and a block whose own text quotes those markers — a doc showing the format, a bullet like this
  one — ends in the middle of itself. The failure does not look like a parse error: it looks like
  a stale anchor in whatever file the truncated block landed on, which is a confusing thing to
  read when the file was correct a second ago. Everything else about the tool is worth reaching
  for from three edits up, across as many files as the edit spans; this is its one edge.
- **The `E04xx` type-diagnostic band is full: `E0499` is the last number in it.** The next
  type diagnostic is a band decision, not a `brief.py` lookup — `E05xx` is IR and codegen
  and `E03xx` is name resolution, so neither absorbs it. `brief.py`'s "next free" line will
  happily print `E0500`, which belongs to another phase; do not take it. Widening the band
  (`E04xxx`, or a second types band) is a decision the session that needs one takes, in
  `docs/adr/README.md` § *Decisions taken at project start*, with the reason.
- **A `## Next group` item's rationale is a hypothesis, not a specification, and one `php -r`
  settles it.** The `exit` item said "`finally` must still run, which is what makes this an
  unwind rather than a `return`" — and PHP runs no `finally` on an `exit` at all:
  `php -r 'try { exit(3); } finally { echo "f"; }'` prints nothing and exits 3. So the
  expensive framing (a fourth unwind kind, a `finally` ladder that has to distinguish it) was
  the wrong one, and the cheap shape — a helper whose success is a non-`OK` status, riding the
  status check that already exists — was PHP-exact. Priority 2 decides these and PHP is on
  `PATH`: run the twin *before* costing the design. Same rule the neighbouring
  `loop-goal.toml` bullet states for a check's comment, one file over.
- **`holes.py`'s per-item refusal sites are the file's *catch-all* panics, not the item's own
  hole**, so an item can read "N sites still standing" long after the feature runs. Items 1, 4,
  6, 7 and 25 all did this session: `$x++`, `--$x`, the bitwise five, `<=>` over two `int`s and
  `object` as a declared type each run end to end today, and the sites attributed to them are
  `lower_expr`'s and `emit_binop`'s "got {other:?}" arms, which will still be there when the last
  hole closes. The tool ranks *candidates*; the ground truth is four lines in a scratch
  `.agent-tmp/*.nvs` and one `nvs run`, and that is what a session should spend before it picks
  an item off the list. `holes.py --cases` is the half that does not lie — a named case either
  exists on disk or does not.
- **A fact `nvs-ir` and `nvs-runtime` both need lives in one of them and is held to the other by a
  test in `nvs-codegen`.** Neither crate names the other — `nvs-ir` depends on `nvs-syntax`/
  `nvs-types`/`nvs-diagnostics` and nothing below, and `nvs-runtime` depends on neither — so a
  shared constant has no crate to live in that both can see. `nvs-codegen` sees both, which is why
  `FN_INVOKE`/`CLOSURE_INVOKE` and `FN_ARITY`/`CLOSURE_ARITY_SLOT` are each a pair with a codegen
  test between them rather than one definition. A whole *table* travels the same way: the closure
  parameter-tag nibbles are `nvs_ir::lower::param_tag_nibble` on the writing side and plain
  `nvs_runtime::Tag` discriminants on the reading side, held together by
  `param_tag_nibbles_are_the_runtime_tag_bytes` in `crates/nvs-codegen/src/ty.rs`. Reaching for a
  new dependency edge to avoid the pair is the wrong direction; a `pub fn` in `nvs-ir` plus a
  `#[test]` in `nvs-codegen` is the shape that already exists.

- **A closure object's slot layout has a third party, and it is a test in `nvs-stdlib`.**
  `closure_of` in `crates/nvs-stdlib/tests/allocation_policy.rs` hand-builds a closure — a class
  with the reserved slots and a Rust `invoke` — so a `Core` member can be handed a `callable` with
  no compiler in front of it. Adding a reserved slot in `nvs-ir` therefore breaks it, and the
  failure arrives as `field slot N is out of range for a class with N slots` from
  `nvs_runtime::object`, three crates from the edit. `grep -rn CLOSURE_ARITY_SLOT --include=*.rs
  crates/` finds every builder in one call; do that before moving the layout, not after.
- **`nvs_ir::Ty` is `#[non_exhaustive]`, so a `match` on it outside `nvs-ir` cannot be
  exhaustive** — the "a new representation is a decision, not a default" guard can only live in
  `nvs-ir` itself, and `nvs_ir::lower::param_tag_nibble` already *is* that guard. `nvs-codegen`'s
  `ty.rs` cannot hold a second copy: both `clif_ty` and `tag_of` end in a `_ =>` arm because the
  compiler requires one there.
- **A hole item names one spelling, and the panic can be under a different one.** Item 29 says
  `$a?->b = v` panics; it has been `E0479` since `c5a8761`, and what still panicked was
  `$a?->b++` — `check_write_target` was called from the plain and compound assignment arms and
  not from `PreIncDec`/`PostIncDec`. Three scratch files (`= v`, `+= v`, `++`) run against
  `target/debug/nvs.exe` in one call told the whole story, where editing the site the item named
  would have closed nothing. Ask every spelling that reaches the same lowering before you believe
  the item's, and `git log -S` on the code the item quotes says whether its half already landed.
- **A `splice.py` anchor copied out of `peek.py`'s output can carry a line break `peek` added.**
  `peek` wraps a long prose line for display, so a two-line anchor taken from a `.md` file may be
  one line on disk — the refusal then reads *"the anchor matches for its first 100 character(s) …
  the file has: ' pass', the anchor wants: ''"*, which is that wrap and nothing else. Keep a prose
  anchor inside one displayed line, or take it from `grep -n`.
- **`holes.py` keys on the refusal *phrase*, not on the macro.** A site reworded from `panic!` to
  `unreachable!` still counts as a language hole while its message says "only lowers", "does not
  lower" or "no lowering for" (`tools/holes.py`'s `REFUSAL`) — so a session that proves a site dead
  has to word the proof without those, or the tool keeps handing the site to the next session.
- **A `?T` row that can carry a fault has to be emitted with `emit_fallible`, or a `catch` wrapped
  straight round the conversion never sees the throw.** `Lowering::convert_or_null` emitted every
  row with a plain `emit` on the strength of "the helper answers `null` where the throwing row
  would throw, so it cannot fail" — true of the numeric rows, and false the moment `as ?string`
  could run the operand's own `toString()`. The failure looks nothing like a missing landing pad:
  the program prints `Uncaught Exception` and exits 1 *with* a `try`/`catch (Throwable $e)` around
  it, because a non-fallible `HelperCall` discards the status word rather than branching on it. So
  when a new row makes a uniform emit site fallible, the scratch file to write is one that throws
  from inside the new row and catches it — not one that checks the row's own value.
- **A `Core` class can render as text without a `toString` row, and the second rule is
  `nvs_runtime::is_carrier`.** A refusal written off the registry's member rosters alone
  looks right, compiles, and then turns four green `Core\Out::capture` cases red at the
  full verify: `Core\Cli\Text` is a **sink carrier**, so ADR 0088 § 5 renders it as the
  bytes it already holds — `value_to_string`'s own `Tag::Object` arm does it, asking for
  no member at all — and `Core\Html\Markup` is the other one. So "which `Core` classes
  render" is two rosters, not one; `nvs_stdlib::registry::class_renders` is where they
  are joined, and `nvs-types` has no `nvs-runtime` dependency to reach the second
  directly. The general shape: a rule stated over `registry.rs`'s rows is not the whole
  rule wherever `nvs_runtime` answers for a class on its own.
- **A diagnostic constant named in a doc comment may not exist**, so a `grep` for the name is not
  evidence that the rule behind it is future work. `reject_unrelated_class_conversion`'s own comment
  named `E_MARKUP_NOT_LITERAL`; the code is `E_MARKUP_REQUIRES_LITERAL`, it lives in
  `nvs_types::expr::quals`, and ADR 0024 § 5's `"lit" as Core\Html\Markup` is a **checked row
  today** — even though `Core\Html` declares no class and the conversion still panics one crate
  down, which is what makes it look unimplemented from `nvs run`. Grep
  `crates/nvs-diagnostics/src/lib.rs` for the band's `Code::new` rows, or the crate's own `tests/`
  for the behaviour: a name in prose is a pointer somebody wrote, not a fact the tree holds.
- **A `loop-goal.toml` check runs before the program legs only if its `stage` string starts with `0`.**
  `tools/loop.py:830` builds its catch-up class with `str(c.get("stage", "")).startswith("0")` — nothing
  else about the block matters, not its position in the file and not what the prose calls it. A stage
  named `"S inout"` sitting physically above the Stage 0 blocks still ran after the whole Stage 1 floor.
  Name a stage that must be reported first `0`-something (`"0a inout"`), and say in the block's comment
  that the digit is load-bearing, or the next person renames it back. `python tools/loop.py --list` prints
  the real run order and is the only way to see this.
- **`cargo insta` is not installed in this environment**, so a snapshot the playbook's
  `cargo insta accept` bullet assumes you can accept has to be accepted by hand: `cargo test`
  writes each new one beside its `.snap` as `.snap.new`, and accepting it is replacing the `.snap`
  with that file minus the `assertion_line:` header field, whose `source:` line also differs (the
  new one names the test file, the old one names the module). Doing that for the six pending at
  once is a five-line script; doing it by hand is two calls per snapshot. `cargo install
  cargo-insta` would fix it once for every future session and nobody has run it.
- **A Python rename script must open with `newline=""` at both ends, or it rewrites every line of
  every file it touches.** `Path.read_text()` / `write_text()` default to `newline=None`, which is
  universal-newlines on the way in and `os.linesep` on the way out — so on Windows an LF file comes
  back CRLF and `git diff --numstat` reads the whole file as changed, burying the six lines you
  meant. `git status` says *"CRLF will be replaced by LF the next time Git touches it"* and that
  warning is the whole diagnosis. `tools/splice.py` already gets this right; a one-off script beside
  it does not inherit that.
- **`cargo insta` is not installed in this tree**, so `cargo insta accept` and `cargo
  insta review` both fail with *"a command with a similar name exists: `init`"* rather
  than with anything about snapshots. Accepting one by hand is two steps and the second
  is the one that is easy to miss: `mv x.snap.new x.snap`, then
  `sed -i '/^assertion_line: /d' x.snap`, because insta writes that header into a
  *pending* file and a committed snapshot must not carry it — leave it in and the file
  is a diff away from every other snapshot in the tree. The neighbouring bullet about
  accepting *every* pending snapshot still applies whichever way you accept: run
  `git status --short` first, and any `.snap.new` you did not just produce is one to
  delete rather than to accept.
- **`cargo insta accept` does not exist on this box** — `cargo-insta` is not installed, and the
  failure reads as a mistyped cargo subcommand rather than as a missing tool. The pending
  snapshots are still written, as `*.snap.new` beside the `*.snap` (not `.pending-snap`), and
  accepting one by hand is copying it over its neighbour minus the `assertion_line:` header line
  insta strips. `diff` each pair first: that is the review the tool would have shown you, and the
  neighbouring bullet's warning about accepting a *previous* session's leftovers applies to a
  by-hand sweep exactly as much.
- **A new `nvs_ir::Helper` variant is four edits and the fourth one is not a `match`.** Three are
  exhaustive matches the compiler makes you write — the variant itself, `nvs-ir`'s `print.rs` name and
  `nvs-codegen`'s symbol name — and the fourth is a row in `nvs_runtime::helpers::symbols()`, a hand-kept
  `Vec` nothing checks. Miss it and the workspace builds clean, every unit test passes, and the first
  program that reaches the helper dies inside `cranelift-jit` with *"can't resolve symbol
  nvs_your_helper"*, which reads like a linker problem rather than a missing line. Grep the symbol name
  you just added and expect **three** hits outside the runtime's own definition.
- **A `gaps.py` "no case calls X" is a claim about the cases it *attributed* to that class,
  and attribution is not the same thing as coverage.** It used to be "the case spells the
  class name", which misses every value reached through a factory on another class — the
  whole time family, where `var $d = Core\Time::fromIso($t)->in($z);` exercises three
  classes and spells one. That ranked `Core\Time\DateTime` at one case over 17 members
  with two dedicated cases on disk, and named `date`, `dayOfYear` and `difference`
  uncalled while one of those cases called all three. `coverage`'s attribution follows
  produced instance types now (`gaps.py`'s `producers`), so the residue is narrower but
  real: a member exercised only through a closure, or through a helper class the walk
  cannot follow, still reads as uncalled. `python tools/gaps.py --member <name>` prints
  the cases that already ask about one, and it reads the whole corpus rather than one
  class's share — one call, before writing a case the tree already has.
- **`gaps.py --coverage`'s ranking was class *size*, not depth, and a whole named group was
  written off it that was already on disk.** The column divided a class's case-file count by its
  member count, so `Core\Math` led the table at 1.00 (38 files, 38 members) while every one of its
  members carried three to five cases and the class carried 31 dedicated files — a case names five
  or ten members at once, so a big class can never reach a high quotient however deeply it is
  asked. All three slices of the handoff's `Core\Math` group ("the four roundings agree", "the two
  base members round-trip and refuse the same bounds", "the inverse members answer their domain
  edge") were already pinned by `math-the-four-rounding-members-agree-wherever-there-is-no-fraction`,
  `math-both-base-members-stop-at-the-same-two-bases` and
  `math-inverse-members-answer-nan-outside-their-own-domain`. DEPTH is now the **median cases per
  member**, with `FLOOR` (its worst member) and the three thinnest members named with anchors on
  every row — so a group is taken from *members*, not from a class. Whatever the tool says, one
  `python tools/gaps.py --member 'Core\X::member'` before writing is what tells a claim that is
  missing from one that is already frozen.
- **Triage a stale guard name by grepping `fn <name>` over the crate's `src` *and* `tests`, then by
  looking for a `.nvst` case of the same name — and expect the answer to be cause 2 more often than
  the debt file's own "likely" hint suggests.** Of twelve names triaged in one session, seven had
  landed as conformance cases and three of those were *already listed* in `loop-goal.toml`'s
  `nvs-suite` `cases`, so the fix was deleting the `cargo-named` entry, not renaming it — a rename
  would have invented a Rust test that was never going to exist. Two more traps in the same call:
  `crates/nvs-ir/tests/` does not exist (its guard tests are unit tests in `src/lower/tests.rs`, so
  a `grep -r crates/nvs-ir/tests` fails silently and reads as "no such test"), and a name whose
  claim differs from the landed test's — `a_nested_element_write_separates_only_the_inner_array`
  against `writing_through_a_nested_subscript_separates_every_level` — is two different assertions,
  not a rename. `cargo test -p X -- --list` is the authority but costs a build; the grep answers the
  same question for nothing, and `python tools/loop.py --list` re-parses the toml afterwards.
- **A guard name that looks like a rename is often a `.nvst` case instead, and the roster that
  settles it is two greps, not `cargo test -- --list`.** `grep -rhoE "fn [a-z_]+"` over a crate's
  `src` and `tests` gives the cargo half in one call, and `ls tests/conformance/*/ | grep -iE
  "topic1|topic2|…"` gives the case half for a whole stage at once — thirteen names triaged in four
  probes rather than thirteen. Two traps inside that: a name matching a *private function* is not a
  match (`reject_arguments_to_implicit_constructor` is `expr/calls.rs`'s, and nothing calls it from a
  test), and a case whose file name shares the topic still has to be read — Stage 5's ambiguous
  attribute retrieval is `E0728` inside `an-attribute-retrieval-is-refused-where-it-cannot-be-folded`,
  which its name does not say.
- **A `loop-goal.toml` comment can be stale about the *tree*, not just about an ADR — check the
  crate before believing "none of that surface exists yet."** The Stage 7 comment on
  `a-test-attribute-builds-a-table-the-runner-reports.nvst` said `#[Test]` was not on
  `nvs_types::derive::ATTRIBUTES`, `Core\Test` had no row in `nvs_stdlib::registry` and there was no
  table to read back; all three had been false for some time — `derive.rs:77` lists `TEST`,
  `FIXTURE` and `TEST_WITH`, `crate::test::CLASS` is in the registry's `CLASSES`, and nineteen
  `.nvst` cases already exercise the assertion surface and all three of § 22's renderings. The
  handoff had copied the comment forward, so the item arrived predicting cause 3 where every name
  was cause 2. One `grep -rn` per claim the comment makes is the whole check, and it costs less than
  writing the wrong triage down.
- **A snapshot diff that is only *added releases in a landing block* can still be a double
  release, and accepting all nineteen of them by hand is how it lands.** The forget that takes a
  transferred argument off `Lowering::owned_temporaries` has to run **before**
  `emit_fallible`, not after: `emit_fallible` builds the call's own fault edge out of whatever is
  on the stack at that moment, and a callee releases its parameters on its *throwing* edge as much
  as on its normal one. Placed after the call, every one of the nineteen pending snapshots grew a
  release that read as "the fix working" and was in fact the double drop. The tell is free and it
  is the only one: ask which instruction the changed `bbN` is the `! bb` of. If it is the call
  that consumed the value, the release does not belong there. Moving it earlier made all nineteen
  diffs vanish — a correct fix here changes no fixture that has nothing fallible *after* a staged
  argument.
- **`tools/holes.py` reads 160 raw bytes back from a literal for the words
  `CodegenError::Unsupported`, and a *comment* counts.** So a catch-all reclassified to
  `CodegenError::Internal` whose comment explains "this is an `internal` rather than a
  `CodegenError::Unsupported`" is still on the worklist afterwards, and the count moves by
  two when you closed three. The recognizer cannot tell a comment from code — it is a
  window over the raw text, deliberately, so that a constructor wrapped by rustfmt is still
  found. Write the justification as "rather than an `Unsupported`", or put it further than
  160 bytes away; `python tools/holes.py` immediately after the edit is the whole check.
- **The driver's acceptance verdict now rides in the pack, and a red one outranks your item.**
  `orient.py`'s RUN section prints the last `goal check:` line out of `.loop/log.md`. Before it
  did, the driver was the only thing that saw a failing check — it writes the verdict to the
  ledger and starts the next session, whose item comes from the handoff — so `abi-probe` stayed
  red across sessions 0055, 0056 and 0057 while each of them worked on something else. A check
  naming a test that "did not run" is an item still open and is this goal's ordinary state; any
  other failure is a regression, and the run cannot end until it is green.

- **Do not write files through the shell — it is measured now, and it was 205 calls in one run.**
  `python tools/loop-stats.py` counts every heredoc, `>` redirect and `sed -i`, and 44 of 57
  sessions of the 20260828-112939 run used one where AGENTS.md rule 1 asks for Write/Edit or
  `python tools/splice.py --patch`. It works right up until an apostrophe in a doc comment closes
  the quote early. It also used to hide the session that did it: a heredoc is not an `Edit`, so
  session 0053 read as 52 orientation calls and no work at all until `MUTATORS` learned about it.
- **`holes.py`'s count is not the number of `panic!`s, and a session widening its recognizer will
  want it to be.** "Match the construct, not the wording" is the right instinct and half the answer:
  89 panic-family sites sit in `nvs-ir` and `nvs-codegen` and **62 are engine invariants** —
  "`foreach` lost the `Env` binding `{name}` it walks" — which no program reaches and which will
  still be there when the last hole closes, so counting them puts the gate's own end state out of
  reach permanently. Wording cannot separate the two either, in *either* direction: an invariant
  names the front-end guarantee it trusts, and so do two real holes (both of ADR 0027's `(...)`
  panics say "has no resolved target recorded in the typed-expression table"). The recognizer that
  works takes the construct **and** the claim's shape — "only lowers X", "lowers X only through Y",
  "has no arm for" — and the mechanical fix, whenever a session wants one, is to make the *source*
  declare which kind it is, as `CodegenError` already does with `Internal` versus `Unsupported`.
  Budget an audit of all 89 before touching that regex; the classification is the work, not the
  pattern.
- **`verify.py`'s test step can be red before you have touched anything, and the failure names
  `nvs-ir` rather than the goal switch that caused it.** `crates/nvs-ir/tests/refusals.rs` attributes
  every `nvs-ir` lowering refusal to an open item in `docs/agent/loop-goal.md`, so installing a new
  goal orphans every site the old goal's items claimed — 17 of them the day the parity chain's goal 1
  went in, in a crate the session that first hit it never opened. `python tools/holes.py
  --unattributed` says whether it is yours: if every site sits in a crate your diff does not touch, it
  is not. The gate stops at the first failure, so this one hides the `.nvst` trees and clippy behind
  it; run those by hand until it is closed. That test refuses its own allowlist as the fix, and it is
  right to.
- **A fixture named in `loop-goal.toml`'s `files` but not yet on disk aborts the *whole* acceptance
  check, before the first build.** `LoopGoal.begin` (`tools/loop.py:1589`) walks `files` and returns
  early on the first missing one, so the ledger reads `0s over 1 check(s)` and not one of the 61
  stage-1 floor checks runs — a goal that adds fixtures for unwritten features therefore has **no
  regression coverage at all** until every one of them exists. Two consecutive sessions ran that way
  after the goal switch. The fix is to write the fixture the moment the goal names it, red or not:
  `loop-goal.toml`'s own header says a fixture's source is not frozen precisely because "its author
  had to write it without being able to compile it".
- **A `loop-goal.toml` check can name a test in a crate that cannot host it, and its `args` is the
  half that is wrong.** Stage 0's `-p nvs-hir` check listed
  `an_implementor_without_a_no_argument_constructor_is_named`, but `E0744` is reported by
  `nvs-types`' checking pass: `nvs_hir::implementors` answers over the class graph and never sees a
  signature, so `required() > 0` is a question no `nvs-hir` test can ask. Move the name to the check
  whose crate owns the diagnostic rather than inventing a test where it was filed, and then copy
  `docs/agent/loop-goal.toml` over `docs/agent/goals/<goal>.toml` — they are byte-identical by
  construction, and the chain's next `goal-switch.py` restores the goal file over the live one.
- **`holes.py` only sees a numbered item whose bold title fits on one line, and a wrapped one fails
  *silently and backwards*.** `ITEM` is `^(\d+)\. \*\*(.+?)\*\*` with no `re.DOTALL`, so an item written
  `15. **a title that wraps\n    before its closing stars**` matches nothing — and because an item's body
  runs to the *next* item mark, all of its anchors are absorbed by the item above it. The observable
  result is the opposite of a failure: `--unattributed` drops to 0 and the summary attributes the sites
  to some earlier number, so the gate goes green while the ownership it reports is a fiction. `python
  tools/holes.py --item N` is the check — it prints "no item N" for the item you just wrote.

- **`verify.py` short-circuits, so the session that turns a red step green inherits every failure the
  steps after it were hiding.** Closing the refusal gate at step 3 let step 5 (`cargo doc`, `-D
  warnings` with `rustdoc::broken_intra_doc_links`) run for the first time in some while, and it failed
  on two intra-doc links in `crates/nvs-stdlib/src/router.rs` that no session that iteration had
  written. Budget for it: a gate that has been red is not a gate that has been passing up to that
  point, and "did I cause this" is answered by the line number, not by the timing.
- **A `loop-goal.toml` stage's *comment header* can carry a rule the orientation pack never prints, and
  stage 2's forbids the obvious fix.** The route-table check names nine tests, and six of the behaviours
  were already asserted in `crates/nvs-types/tests/routes.rs` under better names — so renaming those to the
  check's spellings reads as the cheap close, and the four-line comment above the `[[check]]` block says
  exactly the opposite: *"Every test named below must EXIST and pass. Most do not exist yet; writing one is
  how the loop finishes its item. Do not rename one to something already green."* `orient.py` prints the
  item, the standing decisions and the ADR sections; it does not print the TOML's own comments. When an
  acceptance failure names a test that "did not run", `sed -n` the twenty lines around its `[[check]]`
  block before deciding what the failure means — that block, comment included, is the specification.
- **Two slices over the same file cannot be split into two commits, so commit the first before
  starting the second.** `session.py --wrap` stages a `## commit:` by *pathspec*, so if slice 1 and
  slice 2 both edit `routes.rs`, the first section sweeps both slices' changes and the second
  commits nothing. The handoff's "next group" is a group precisely because its slices share a file
  set, so this is the normal case rather than the odd one. Either accept one commit with two
  clauses — house style already allows it — or `git commit` slice 1 by hand once its own crate's
  tests are green, and let the wrap commit the rest. What does not work is writing two
  `## commit:` sections over the same path and hoping git splits them.
- **A `loop-goal.toml` acceptance check can fail on the *driver*, not on the tree — and a check whose
  failure detail is a bare `True` is that.** Stage 4's `nvs build --openapi emits 3.1` was reported
  failing after four sessions in a row while passing by hand at every one of those commits, and two
  handoffs in a row wrote it off as "the driver built the binary before the commit landed". It was not:
  `tools/loop.py`'s `cargo_check` read `ordered_in`'s **boolean** as a description of what was missing
  (`missing = ordered_in(...); if missing: return f"{label}: {missing}"`), so a `kind = "command"` check
  failed exactly when it passed and printed the word `True` as its reason. No command check had ever
  passed since the kind was introduced in `058c1f0`. When an acceptance failure's detail is not a
  sentence about your code — `True`, an empty string, a bare number — read the branch in `loop.py` that
  produced it before touching the tree, and check the other call sites of whatever helper it names.
- **`orient.py`'s "THE DRIVER'S LAST ACCEPTANCE CHECK FAILED" can be a record from the *previous* run,
  already repaired.** The pack opened this session with stage 4's `nvs build --openapi emits 3.1` failing
  and the bare-`True` detail the bullet above explains — but the three `True` records in `.loop/log.md` all
  sit in the run that started 05:29, `ba38b77` fixed `loop.py` at 06:31:36, and the run that is holding the
  tree now started 06:33:25 with no acceptance line yet. The ledger keeps the last *recorded* result across
  the `## run started` boundary, so a failure repaired at the end of a run is what the next run's first
  session reads. Two calls settle it: `git log --format=%ad --date=iso <the fixing commit>` against the
  `started:` line the pack prints, and running the check's own `argv` by hand.
- **`python tools/gaps.py --coverage` does not list every registered class, and the classes it drops
  are the ones a floor gate then finds.** Its table printed 30 classes; `nvs_stdlib::registry::CLASSES`
  holds more, and `Core\Attributes` and `Core\Program` were in neither the ranking nor the "thinnest
  members" column while both had a member below three cases. The cause is that `gaps.py` reads class
  names out of the Rust source with `CLASS_RE`/`NAME_CONST_RE` rather than out of the registry, so a
  class whose `name:` const it cannot resolve vanishes silently — its members are not ranked, not
  counted and not reported as uncovered. The same read explains smaller drifts in the numbers: it put
  `Core\Test::assertEquals` at 3 where the registry-driven count says 2. Treat the tool as a *ranking*
  over most of the tree, not as the roster; anything that has to be true of **every** member reads
  `registry::CLASSES` directly.

- **The `regex` crate supports no look-around, and this tree carries `fancy-regex` for exactly that.**
  A pattern with `(?!...)` — the obvious way to write an identifier boundary — compiles fine and then
  panics at run time with `error: look-around ... is not supported`, naming the caret position and not
  the crate. Both crates are `nvs-stdlib` dependencies, so the import that fails and the import that
  works differ by one word. For a right-hand boundary specifically, neither is needed: a
  `match_indices` walk plus one `chars().next()` check is what `conformance_coverage.rs`'s `mentions`
  already does.
- **`gaps.py --errors` reads past a `Fault::` call into the next item's doc comment, so one of its
  rows is a phantom.** `test.rs`'s `Fault::thrown_as(ThrownClass::TestFailure, text)` carries no
  literal at all — its message is `format!`ed six lines above it — and the tool's 700-byte window
  finds the *following* function's `///` prose instead, listing the site as
  `a test that asserts nothing /// fails`, a stem no case can ever contain. The gate in
  `crates/nvs-stdlib/tests/conformance_coverage.rs` stops its window at a line-leading `///` for that
  reason, so it reads 71 sites where the tool lists 72 and the difference is that one row. A
  `Fault::` whose message is built above the call is outside both, the same way a message opening on
  its own format hole is: neither can be matched against a case by its stem.
- **`DECLARATION_WINDOW` is 8 lines counted from the `Fault::` line, not from the guard's first
  line, so a nine-line comment sits silently outside it.** `conformance_coverage.rs` slices
  `lines[fault - 9 .. fault - 1]`, and a `Fault::thrown(format!(` two lines below `let x = …` leaves
  only **seven** lines for the comment above it. A declaration written as a doc comment on the
  enclosing `fn` is always outside the window when the body does anything at all before the guard —
  put it in the body, immediately above the `let`, and keep it to seven lines. The failure names the
  right site and reads like the phrase is missing, which is the misleading part: it was present and
  one line too high.
- **A `<<'PY'` heredoc is not literal enough to carry a Rust string escape, and it fails as a
  mismatch rather than as a corruption.** AGENTS.md rule 1 already says a shell never carries file
  content into the tree; the reason worth knowing is that the *quoted* heredoc — the construct you
  reach for precisely because it is supposed to pass bytes through — still halved every backslash
  run on the way to Python, so a script matching `("Core\\Encoding", "encodeText")` was really
  matching a one-backslash string and found zero of twenty-two rows. It costs three calls to
  rediscover because the script asserts and changes nothing, which reads like the file having moved
  under you. Anything holding a `\\`, a backtick or an apostrophe goes through Write/Edit or
  `python tools/splice.py --patch`.
- **`python tools/check-migration.py --seed` is a candidate list, not rows to paste.** It attributes
  every backticked PHP name on a line to whatever that line is *about*, so a member's own prose drags
  its neighbours in: it emits `acos` → `Core\Math::asin`, `acosh` → `Core\Math::sinh` and `array_map`
  → `Core\Str::fromCodePoints`, three wrong answers from three lines where the name appears only as an
  aside. All 341 seeded rows would move the coverage figure by nine points on guesses, which is what
  02-php-migration.md's own header ("nothing here is guessed to make a number move") forbids. Fill a
  domain the other way round: read 01-core-library.md's table for the class once, then write one row
  per name `--report` still lists. The checker catches a member spelling 01 does not have, so a wrong
  *name* fails immediately — but a right name against the wrong PHP function passes silently, and that
  is exactly what `--seed` produces.
- **`gaps.py`'s per-member numbers are cases already written, not cases owed — and a handoff can
  turn them into an item that is already on disk.** The *Next group* this session opened with named
  "`Core\Random::float`'s bounds (`random.rs:285`, 3 cases)" and "`::int`'s inclusive pair
  (`random.rs:263`, 4 cases)", both reading as work to do; the `3` and the `4` are that member's
  current case count in `gaps.py`'s *thinnest members* column, and both shapes had landed long
  before — `random-float-s-unit-interval-is-bounded-at-both-ends.nvst` (c299254) and
  `random-int-s-closed-bound-reaches-the-ends-of-int-itself.nvst` (3b89685), 217 and 331 commits
  back. The check is one call and it is not `gaps.py`: `for f in tests/conformance/core/<class>-*.nvst;
  do sed -n '2p' $f; done` prints every sibling case's one-line claim, which is what says whether
  the shape exists. Do it before writing a line, and again before believing a case is new — the
  overlap the second pass found here was a two-line one inside a case whose own title did not
  mention it.
- **`cargo deny check` is not installed on this machine**, so the standing decision a new
  Rust dependency owes cannot be run locally — `cargo` answers *no such command: `deny`*
  and installing it is a multi-minute build. The substance of the check is doable by hand
  in one call: `python tools/gen-attribution.py` rewrites `THIRD-PARTY-LICENSES.txt` with a
  resolved-license column per crate, so `grep -n -E "(crate|names)" THIRD-PARTY-LICENSES.txt`
  against `deny.toml`'s `[licenses] allow` list settles the license half, and the advisory
  half is CI's. Say in the handoff which crates were cleared that way.
- **`gaps.py --coverage` counts cases per member, not depth, so its *thinnest members* column can
  name work that is already finished.** A handoff group built off it named `Core\Math`'s `lcm`,
  `hypot` and `atan2` at three cases each; all three were already pinned to depth —
  `math-the-division-identity-and-the-gcd-lcm-identity-hold-on-every-row-that-has-one.nvst` §§ 4-5
  carries `gcd*lcm == |a*b|` over 100 pairs with the sign convention, the zero rows and both
  members' bounds, and `hypot` and `atan2` each have a case of their own. One
  `grep -rn 'Class::member' tests/conformance` per named member, before believing the column, is
  the whole check and costs one call. A member with three *deep* cases needs a new boundary, not a
  fourth table — and the check belongs in the session that *writes* the next group, since it is the
  one holding the context.
- **A goal switch orphans whatever a carried check's green depended on, and `cargo test -p nvs-ir
  --test refusals` is where you find out.** `goal-switch.py` carries the outgoing goal's `[[check]]`
  blocks forward as the next goal's floor and its unclosed *items* not at all, so `nvs-ir (no refusal
  left)` arrives without the item list that attributed its seventeen sites — a full-tree red at the
  first `verify.py` of the new goal, in a crate the session never touched. It has now happened at two
  switches; the second time the fix stopped being a paragraph pasted into the new goal by hand.
  `python tools/holes.py` reads `docs/agent/carried-refusals.md` as a second item source, numbered
  from 900 so `--item 901` names the entry the document shows, and a goal's own `.md` must not
  restate it. If this test goes red on you, check whether the sites are *new* before writing
  anything: `python tools/holes.py --unattributed` says, and `CEILING` in `refusals.rs:66` is what
  catches a genuinely new one even when attribution claims its file.
- **A `cargo-named` `loop-goal.toml` check names *test names*, so a test that pins the same
  behaviour under a different name does not close it.** Stage `0 containment` asked for
  `a_panic_in_a_worker_task_is_contained_at_the_task`; the behaviour was fully pinned on disk as
  `a_panic_with_no_helper_beneath_it_is_contained_at_the_task_root`, and the driver reported "did
  not run" every iteration. `grep` the check's `tests = [...]` before writing anything, and
  **rename** the existing test rather than adding a second copy of it — the check is the contract
  for the name, and two tests asserting one thing is how the next session loses an hour deciding
  which is authoritative.
- `python tools/adr.py --index` **prints** the regenerated index table; it does not write it. The
  `index table is stale` finding stays until you paste the new row into `README.md` yourself, and the
  two *other* index findings (`no bullet in ground-rules.md`, `no row in § Where to look`) are three
  separate edits in two files, not one. Budget four edits per new ADR, then `--check` for `exit=0`.
  Every other check the tool runs is a report, and so is this one — the name is the only thing that
  suggests a fix.
- **A `[dev-dependencies]` addition owes `deny.toml` an answer but owes `THIRD-PARTY-LICENSES.txt`
  nothing, and the two are checked in opposite directions.** `tools/gen-attribution.py` walks normal
  and build dependencies only — its own docstring is the home of that — so `--check` stays green
  after one and there is no regeneration to commit. `cargo deny` is the other way round: its
  `[graph] all-features = true` license-checks every crate that reaches `Cargo.lock`, including the
  optional ones no target builds, and `cargo deny` is not installed here to tell you so before CI
  does. Read the new licences yourself in one call — `cargo metadata --format-version 1
  --all-features` — against `deny.toml`'s `allow` list. Adding `rustls` and `rcgen` on the `ring`
  provider put twenty crates in the lock file across `Apache-2.0 AND ISC`, `ISC` and
  `MIT OR Apache-2.0`, every one of them already allowed; `ring` also builds clean on
  windows-msvc from the pregenerated assembly it ships, with no nasm and no perl on `PATH`.
- **A `python - <<'PY'` heredoc eats a backslash, so a patch script cannot carry Rust or
  Novis escapes.** `<<'PY'` is quoted and the *shell* expands nothing, but something between
  it and Python still collapses `\\x` to `\x` and `\\n` to a newline — a `sed`-style Python
  patch containing either silently matches nothing (no `assert`, no error) or dies with
  `SyntaxError: truncated \xXX escape`. This is AGENTS.md rule 1 with a second face: it is
  not only `>` and `sed -i` that mangle content, it is any content at all crossing the shell.
  Write the new text with Write to a file under `.agent-tmp/`, then have the shell only
  *name* both files — `python -c "open(dst,'a').write(open(src).read())"` — or use Edit,
  which never goes near a shell.
- **An edit made after `verify.py --start` invalidates the run, and `fmt` is where you find out.**
  The loop's own step 3 says to start the verification and write the wrap while it runs — which is
  right, because prose cannot fail — but a source edit in that window is compiled at whatever moment
  each step happens to read the file. One session added a module-doc paragraph and ran `cargo fmt`
  after `--start`, and the background run failed at `fmt` (step 2 of 7) against the *pre-format*
  text: a red verdict describing a tree that no longer existed. There is nothing to debug and
  nothing to fix — `--start` again once the tree is final, and only then write prose. The rule is
  simply that `--start` marks the end of editing, not the start of the tail.
- **`Path.write_text` turns every `\n` into `\r\n` on Windows, and a test that reads the tree sees it.** A one-off script that rewrites `.rs` files must open them with `newline=""` (or write bytes): `.gitattributes` says `eol=lf`, git normalizes on commit so `git diff` looks fine, but `crates/nvs-stdlib/tests/conformance_coverage.rs` scans the working copy and reports every multi-line message as "neither asserted nor declared unreachable" with `\r\n` inside the quoted text. That is the signature; the fix is a byte-level `\r\n` → `\n` pass over the files the script touched, and it cost one full `verify.py` run.
- **Two sessions in one tree: a file both edit is committed by whichever stages it first, with the other's hunks inside.** ADR 0117's enum-and-constant amendment first landed inside a commit about ADR 0063, because the other session staged the whole file while this one still held it dirty — that session then redid its commit without the foreign hunks, which is the right repair but cost both sessions a turn. `git status --short` a file before editing it; if it is already dirty and the hunks are not yours, either wait for that session's commit or stage your own hunks alone — `git show HEAD:<path>` plus your change through `git hash-object -w --stdin` and `git update-index --cacheinfo 100644,<blob>,<path>` stages a version the working tree never holds, which is also how two slices that touch one file get one commit each.
- **`splice.py` writes LF, so splicing a CRLF working copy leaves the file mixed — and a gate that
  reads source *bytes* then fails somewhere you did not touch.** Putting a `names:` line on
  `crates/nvs-stdlib/src/str.rs`'s registry rows (line ~160) made
  `every_error_path_is_asserted_or_declared_unreachable` fail on `Core\Str::wrap`'s message at line
  2136, 2,000 lines away: `conformance_coverage.rs`'s `fault_sites` reads the message stem out of the
  source, a `\`-continued string literal then carried a `\r` the `.nvst` corpus does not have, and the
  stem stopped matching. The recognition test is that the reported stem contains a literal `\r\n`.
  `python -c "b=open(p,'rb').read(); print(b.count(b'\r\n'), b.count(b'\n')-b.count(b'\r\n'))"` says
  whether a file is mixed; the repository stores LF, so normalizing the whole file is the fix and not a
  reversion. Do **not** reach for `git stash` to bisect this — the loop driver may hold the tree, and a
  stash sweeps its in-flight work into yours.
- **`rand_core` 0.10 renamed the core trait and inverted which half you implement**, so the shape
  every guide writes fails twice here, one call apart. `RngCore` is gone (a deprecated stub);
  `rand::Rng` is now the infallible core trait, `rand::TryRng` the fallible one, and `Rng` is a
  **blanket** impl over `TryRng<Error = Infallible>`. So a hand-written `impl rand::Rng for T` is
  `E0119` against that blanket, while writing only `impl rand::Rng` without a `TryRng` half is
  `E0277` saying `T: TryRng` is unsatisfied — two errors for one mistake. Implement `TryRng`'s
  three `try_*` methods and take `Rng` and `rand::RngExt`'s drawing methods for free. Second trap
  in the same corner: `RngExt`'s methods are declared on a `Sized` receiver, so a `&mut dyn
  rand::Rng` parameter compiles and then offers `next_u64` and nothing else —
  `crates/nvs-stdlib/src/random.rs`'s `Generator` newtype is the way round it, and its doc says
  why a generic parameter is not.
- **`gaps.py`'s depth column is cases per *member*, so a class with two members looks thin however
  deeply each one is tested — and the corpus is now dense enough that the ranking's top rows are
  mostly this artefact.** Every one of its first eight rows was checked this way and `Core\Uuid`,
  `Core\Hash\Stream`, `Core\Heap`, `Core\Math`, `Core\Arr`'s floor-3 trio and four of `Core\Csv`'s
  five claims already had the case the item asked for — the *agreement* shape included, which the
  handoff named as "the shape with the most room left". Four sessions' worth of items have been
  written off the same ranking, which is how it went stale without anyone noticing. **What still
  finds room is the opposite direction: read a module's `//!` doc and its `MethodDoc` cards for a
  claim stated in prose, then `grep -rln` `tests/conformance/core/` for a case that asserts it.**
  Three of the four claims `crates/nvs-stdlib/src/csv.rs`'s module doc makes had no case at all —
  quoting *minimality*, the `{header: …}` round trip it names as a composition, and the dialect rule
  being one rule both members are asked — and each was one case. A claim a module doc bothers to
  argue for is a claim someone thought could go wrong; that is a better worklist than a median.

- **A variable lowers inside a `Core` options shape.** `{separator: $sep, quote: $quote}` and
  `{header: $names}` both compile, so a dialect or a header sweep can be driven from a table rather
  than from one literal per row — which is what makes an agreement case over nine dialects one loop
  instead of eighteen pasted calls. Probed with `nvs run` before the three `Core\Csv` cases were
  written.
- **`peek.py --locate` is a mode, not a flag you can add to a read.** A call written as
  `python tools/peek.py "docs/adr/0088-*.md:### 2" file.rs:79-120 --locate report_mismatch` prints the
  anchors and *silently drops both read targets* — `--locate` takes the rest of argv, so the questions you
  batched with it are never answered, and a symbol it cannot find exits 1 on top of that. Ask for anchors
  in their own call, and keep the reads in another.
- **A missing entry in `loop-goal.toml`'s `files` aborts the whole acceptance check, not one fixture.**
  `tools/loop.py:1779`'s `begin` walks that list before anything is built and returns on the first path
  that is not on disk, so `check()` never reaches the build, the stages or a single `[[check]]`. The
  driver reports one line — `examples/config.nvs is missing` — which reads like one fixture failing and
  is in fact the run producing **no measurement at all**; it stayed that way for ten sessions, each of
  which saw a green tree and an acceptance line about a file it had no reason to open. A goal whose
  `files` names a fixture a later stage will write needs that file to exist from the first session, even
  as a program that cannot compile yet: the source is not frozen and only the expected output is, so an
  early stand-in costs nothing and buys back every earlier stage's check. `python tools/loop.py --list`
  does not show this, because the list is checked before the plan is.
- **The handoff's own next-group item can contradict a settled ADR, and the ADR still wins — the same
  rule the `loop-goal.toml` bullet above states, arriving through the other artifact.** An item read
  "the unknown-key refusal is a new `E06xx` (next free E0605) naming the block", and ADR 0064 § 3 says
  in as many words that an unknown key is refused "with the existing `E0601`/`E_BAD_DIRECTIVE`
  diagnostic naming the line". Both artifacts are written by a session; only one of them is a
  decision. Claiming E0605 for it would have put two codes on one refusal and left `file.rs`'s
  `code_for` disagreeing with the ADR that named it. What the item was *reaching* for was real and
  costs six lines: `serde` cannot name the block, because by rejection time the deserializer knows
  the struct and not what the file called it, so `nvs_config::file::block_at` scans back to the
  nearest header and adds it as a note. The general shape — an item that asks for a new number is
  asking a question the ADR has usually already answered, and one `peek.py <adr>:"## 3"` settles it.
- **A `Files`-style trait in a crate is the seam a new filesystem question goes through, and its
  test fakes are where the question actually gets asked.** ADR 0104 § 1 needs a canonical path
  *without* § 6's trust check, so `nvs_config::resolve::Files` gained `canonical` beside `trust`
  and `Disk` routes both to `trust.rs`. The cost that is easy to miss: the two existing fakes in
  `crates/nvs-config/tests/resolve.rs` and `crates/nvs-config/tests/secret.rs` stop compiling until
  each grows the method, and a fake
  that answers it *lexically* silently makes every case a statement about paths no symlink was
  involved in — `crates/nvs-config/tests/resolve.rs`'s `Fake::exists` had to follow links too,
  because the real
  `Path::exists` `stat`s rather than `lstat`s and an include naming a symlink was otherwise
  `E0605`. Grep `impl <Trait> for` before adding a method, and give the fake the resolving
  behaviour rather than the identity one.
- **`peek.py` takes every target *before* any flag, and a target after one is `unrecognized
  arguments`.** `peek.py A.rs:re:pat --window 8 B.rs:1-16` fails on `B.rs:1-16` — argparse stops
  collecting the positional list at the first optional and will not resume — while the same call with
  both targets first works. The failure names only the trailing target, so it reads as a bad target
  spelling rather than as an ordering rule, and the natural fix (drop the target) is the wrong one.
- **`Edit` strips a trailing space from `new_string`, so a `replace_all` that narrows a keyword eats
  the space after it.** `pub const ` → `pub(crate) const ` arrived as `pub(crate) constMAGIC`, and
  the same edit re-applied to repair it is refused as "old and new are identical", because the tool
  compares the stripped strings. Include the following identifier in both halves, or write the run
  as one `python tools/splice.py --patch` file — which is the rule for three or more edits anyway,
  and which was the fix here. A new module in a bin crate needs that run: with no caller yet it
  trips `unreachable_pub` on every item under `-D warnings`, and then `dead_code` on all of them
  again once they are `pub(crate)`.
- **`INSTA_FORCE_UPDATE=1 cargo test` rewrites every snapshot in the crate, not the ones that
  failed.** A lowering change that moved 19 of `nvs-ir`'s 158 snapshots came back as 134 files
  modified, because insta also refreshes each file's `source:` header — and those headers have been
  stale since the tests moved to `lower/tests.rs`. `cargo insta review` is interactive and so is
  unusable here; the way back is to revert the header-only files by diff shape
  (`git diff -U0 -- <f>` showing exactly two changed lines, both `source:`), which leaves the slice's
  own 19. Check `git status` after any forced snapshot update rather than staging the directory.
- **A `Resolved`/`Snapshot` field is not enough to make a value reach a reader: `Snapshot::retype`
  rebuilds the typed tree from the merged *table*, so anything the resolver put on
  `Resolved::config` and nowhere else is dropped at the snapshot boundary.** That is the whole of
  finding U11 — `secret::materialize` had been filling `config.db[…].password` since the day it was
  written, and every reader still saw `None`, because the table it deserializes from holds
  `password_file` and no content. A reload retypes a second time, so a value put back once, at
  build, would have been dropped again on the next `Boot` carry. The fix shape that survives both:
  carry the value beside the table and re-apply it *inside* `retype`, which is the one place every
  deserialization goes through. Before believing a config value is lost in the resolver, check
  whether it is in `Snapshot::table` — `nvs config dump --toml` prints exactly that table.
- **The handoff's own `## Next group` can be stale about the tree, exactly as a `loop-goal.toml`
  comment can — and the check that costs nothing is `nvs.exe test` on the case the goal already
  names.** Item 32's group listed P7 `throw "x"` and P8 `clone $a` as panics to refuse; both were
  already `E0780`/`E0781` with green cases, and one `nvs run` over a four-line scratch showed P2 and
  P3 landed too, so four findings were ticked in `docs/reference/findings.md` without writing a line
  of Rust. A `loop-goal.toml` `[[check]]` block's `cases` list is the cheapest probe there is: run the
  ones ahead of the failing case, because the acceptance check names only the *first* thing missing
  and says nothing about what the rest of the list already proves.
- **`INSTA_FORCE_UPDATE=1` rewrites all 118 of `nvs-ir`'s snapshots, not the three your change
  moved — use `INSTA_UPDATE=always` alone.** The extra variable makes insta rewrite every snapshot
  it *passes* as well, and their `source:` headers still say `crates/nvs-ir/src/lower.rs` from
  before that file was split, so 115 files come back modified with a one-line header diff that has
  nothing to do with the slice and buries the three that matter. `INSTA_UPDATE=always cargo test -p
  nvs-ir --lib` touches only what actually differs. If it already happened, `git status --porcelain
  <snapshot dir>` piped through a `grep -v` of the ones you meant, then `git checkout --`, puts the
  rest back; do it before the wrap, because `session.py --wrap` stages the paths you name and
  sweeps everything else into the last commit.

## Running things

- **Verification is one call:** `python tools/verify.py` — build, fmt, test, the two `.nvst` trees and
  clippy in order, stopping at
  the first failure, ~10 lines when green. `-p <crate>` scopes it, `--fast` drops clippy and fmt for a
  mid-work check, and every step's full output lands in `.agent-tmp/verify-<step>.log` either way.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **One case, quickly:** `nvs test tests/conformance/core/str-case-members.nvst`, or
  `nvs test tests/ --filter str-` over the tree.
- **A scratch `.nvs` under `.agent-tmp/` run with `nvs run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.nvs` — there is no `Main::main` entry point, and a `for` header takes
  *expressions* only, so the loop variable is declared on the line above it.
- **`nvs run` printing the right output and exiting **127** is a heap corruption at teardown**, not a
  missing command: Windows reports a double release that way, with nothing on stderr. So check `$?` on a
  scratch run rather than reading the output and moving on — a refcount bug is otherwise completely silent
  until the WSL valgrind leg catches it. `try { … } catch (Throwable $e) { … $e->message … }` **does**
  lower at file scope now, which is what `tests/conformance/core/time-datetime-is-a-civil-time-in-a-zone.nvst`
  and both `Date`/`TimeOfDay` cases write; the class-method shape
  `tests/conformance/lang/a-lossy-conversion-throws.nvst` uses is no longer needed for that.
- **A leak whose "definitely lost" size is `16 + strlen(a literal in the probe)` is a temporary abandoned
  on a throwing edge, and the allocating stack carrying no `nvs_` frame at all is the confirmation.** That
  is the whole recognition test, and it is worth knowing because bisecting to it costs an hour. Most of
  those are now closed: a call's arguments and receiver, and the operands of `.`, an interpolation and an
  `echo`, all go on `nvs_ir::lower::Lowering`'s owned-temporaries stack and are released on both edges. A
  probe that `catch`es a throw from a `Core` member taking a `string` is therefore *green* now and is a
  fair leak check. What is still open is narrower and named in that field's own doc comment (an argument
  being **transferred** when a later one throws) plus the producers that still release inline — a
  normalized subscript key, a `match` subject.
- **A field or element read off a *temporary* is a fresh producer, not an aliasing read.**
  `$h->peek()->name` and `$m->rows()["0"]` each used to leak one value per run; neither does now,
  because `lower_property_access`/`lower_index` retain what they read and release the base, and
  `Lowering::aliasing_read` therefore recurses into both a property access's and an index's own base
  and answers `false` for these shapes. So a consumer must not retain such a read a second time —
  every retain decision in `nvs-ir` already goes through `aliasing_read`, and a new one that reaches
  for the syntactic `is_aliasing_read` instead is how the double-retain gets back in.
- **`verify.py` executes the `.nvst` trees, so nothing else needs running before the wrap.** Its
  `conformance` and `differential` steps are `target/debug/nvs test tests/<tree>` — the very
  command `tools/loop.py`'s acceptance check judges a session by — and their two lines are the
  counts the plan's fields quote. Fourteen seconds for both. So after a green `verify.py` there is
  no `nvs test tests/conformance` to run, no `nvs test tests/`, and above all **no
  `cargo build --release -p nvs-cli`**: that is 125s for a less faithful answer, and one measured
  run spent 8% of its entire wall clock on it across nine sessions. This bullet used to say the
  opposite — `cargo test`'s `conformance_coverage.rs` asserts only that a case *exists* naming each
  registry member, so a rewritten case body could leave every verify step green and fail at
  `loop.py` a stage later. That hole is what the two steps close. While *writing* a case, one at a
  time is still fastest: `./target/debug/nvs.exe test <path>`, under a second.
- **A before/after measurement is worth a `git stash`, and the base half is what makes it an
  A/B rather than two readings** — stash, `cargo build --release -p nvs-cli`, `bench.py <cases>
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
- **A `#[global_allocator]` in a test target must be `#[cfg(debug_assertions)]`.** `nvs-runtime`
  installs its pooled allocator under `all(not(test), not(debug_assertions))`, so a second one in a
  `nvs-stdlib` test binary links fine under `cargo test` and fails to link under
  `cargo test --release` with *"cannot define multiple global allocators"*. `verify.py` runs the
  debug profile, so the guard runs; the release profile compiles it out.
  `crates/nvs-stdlib/tests/allocation_policy.rs` is the worked example, and counting allocations is
  worth the setup — it turned "I think this allocates once" into a test.

- **A release build relinking the runtime moves a bench row by about ±6%, with no code change.**
  `06-string-split-join`'s work figure read 87.3 ms and 93.3 ms across two builds whose `join` was
  byte-identical, and `04-string-format` moved 82.4 to 86.9 with nothing of its own touched. So an
  A/B on a single row is only worth reading when the delta is well past that, and the *median* is
  the statistic to quote. Re-run the base binary once before believing a small regression.
- **`python tools/verify.py` is not deterministic, and the one test that makes it so is a real
  use-after-free rather than a flake to re-run past.** `-p nvs-codegen --test throwing`'s
  `an_uncaught_throw_leaves_the_status_and_the_message_on_the_context` fails about 7% of runs inside
  `verify.py` and 22 of 40 run on its own, either as a wrong `ctx.pending()` message or as a bare
  *"misaligned pointer dereference"* panic in `crates/nvs-runtime/src/object.rs:1220` with nothing
  naming the throw. So a red `verify.py` in a session that touched no Rust is worth **one** re-run to
  identify — and if that is the test, it is inherited: say so and leave it to the slice that owns it,
  because a second green run does not mean the tree is clean.
- **A `Ctx` that outlives the `Unit` whose code it ran reads freed class descriptors, and the crash lands
  nowhere near the cause.** Compiled code bakes each `ClassDesc`'s *address* in as a constant, so an
  exception object left on the context points into the `Rc<ClassTable>` the `Unit` owns and nothing else
  keeps alive. Drop the unit first and `ctx.pending()` reads freed memory — intermittently a wrong message,
  intermittently a *misaligned pointer dereference* inside `nvs_runtime::object::drop_one`, which is the
  release walking garbage slot counts. This is what made `verify.py` non-deterministic for several
  sessions (28 of 40 runs of one `nvs-codegen` test, 7% inside `verify.py`), and the reason it looked like
  a flake is that `nvs run` never hits it — `nvs-cli` installs the table. **`nvs_codegen::Unit::install_in`
  is now the one spelling and its doc comment is the rule**: call it before running any of a unit's code,
  whether or not you care about `catch`. A harness that builds a `Ctx`, runs a unit and then reads anything
  off the context is the shape to watch for.

- **`tools/leak-check.sh` used to report a fixture's own non-zero exit as a leak.** `examples/uncaught.nvs`
  ends in an uncaught throw and so exits 1 by design, which under valgrind's `--error-exitcode=1` was
  indistinguishable from a definite leak — the `definitely lost: 0 bytes in 0 blocks` line printed right
  beside the "failure" was the only tell. It uses 97 now, a status no Novis program produces, so a throwing
  fixture is a fair leak subject.
- **`target/release/nvs.exe` is whatever the *last* session built, and rebuilding it costs two
  minutes for a verdict the debug binary already gives.** A `.nvst` case a stale binary fails may
  simply predate it — one session's was two hours and four commits old and reported
  `str-replace-and-pad-are-the-identity-at-their-own-bound.nvst` failing on a `Core\Str::padStart`
  line nothing in the session had touched. The fix is **not** the release rebuild it used to be:
  `cargo build --release -p nvs-cli` is 125s here (thin LTO at `codegen-units = 1` relinks the world
  for a one-line edit), nine sessions of one run paid it, and that was 8% of the whole run's clock.
  Build `cargo build -p nvs-cli` and run `target/debug/nvs.exe` instead — 2s once `verify.py` has
  built, and it is the *same* binary `tools/loop.py`'s acceptance check judges you by, so it is the
  more faithful answer as well as the cheap one. `git status --short` showing the case unmodified
  says the failure was not *caused* here, which is the neighbouring bullet's rule; only a current
  binary says it is not real.
- **A scratch `.nvs` still needs its `<?nvs` tag, and without one the panic names a construct you did
  not write.** A file under `.agent-tmp/` that opens straight into `echo` lowers as a single
  `InlineHtml(0:0..139)` statement and dies in `nvs-ir`'s control-flow slice listing every statement it
  *does* lower — which reads as "`echo` is unsupported" rather than "this file is all text". The
  `.nvst` harness supplies the tag for you inside `--FILE--`, so the omission only ever bites on a
  scratch run, which is exactly where a session is trying to find out whether a shape lowers.
- **`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh …` fails from the Bash tool and works from
  PowerShell.** Git Bash rewrites any argument that looks like a POSIX path before `wsl.exe` ever
  sees it, so the documented command arrives as `bash: C:/Program Files/Git/mnt/<drive>/<repo>/tools/
  leak-check.sh: No such file or directory` — a path no document mentions, which reads as a missing
  script rather than as the MSYS path translation it is. The script's own header shows the
  invocation and cannot show this, because it is the *caller's* shell that mangles it. Run the
  identical line through the **PowerShell** tool, where the argument is passed through verbatim;
  `MSYS_NO_PATHCONV=1` in front of it is the other way and is one more thing to remember. The same
  rewrite hits any `/mnt/...` or `/tmp/...` argument handed to `wsl.exe` from Bash. `commands.md`
  § *Fuzzing and callgrind on Windows* spells the command itself; this is only what the shell in
  front of it does to the argument. **Five sessions wrote this bullet, one each.** That is what
  `python tools/playbook.py --dupes` now exists to catch: an append-mostly file cannot notice that
  it already knows something, and every copy is charged to every session afterwards.
- **A `holes.py` site guarded by a predicate over the same list it matches is an
  internal-consistency check, and the reachable holes are its *neighbours*.**
  `closed_literal_set`'s `other => panic!` was on the worklist and owed a
  diagnostic-or-lowering decision; the `closed` predicate three lines above it had already
  asserted every atom was one of the three the map handles, so no program could reach it and
  what it owed was `collect::<Option<_>>`. The item's own three spellings each hit a
  *different* site one call apart: `$x as 1|2.5` is `E0120` in the parser, `$x as true`
  aborted in `erase_checked_ty` (`lower/mod.rs`, naming a representation rather than the
  feature), and `$x as ?"a"` in `convert_or_null`. Four scratch files under `.agent-tmp/`
  cost less than reading either function, so run the item's spellings **before** designing
  anything — the panic a worklist item names is often not the one that fires.
- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `nvs_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.nvst` case that calls it — `crates/nvs-stdlib/tests/conformance_coverage.rs` fails
  `cargo test -p nvs-stdlib` without one. An instance member is covered by a case writing `->name(`.
  [conventions.md](conventions.md) writes all four out; `python tools/brief.py`'s *anchors* block
  resolves each spelling to a file and line.
- **A spec §§ 1-12 member owes a *fifth* thing: striking its line from
  `crates/nvs-stdlib/tests/spec-members-outstanding.txt`.** That file is the outstanding-member ratchet
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs` reads, and the test fails on a **stale** line — one naming a member
  that is registered now — exactly as loudly as on an unregistered member the file does not list. So the
  failure you see after landing a member is not a regression; it is the list telling you it did not
  shrink. Its keys are `§<section> <the spec's own Member-cell spelling>`, which is why `§1 chunk` and
  `§2 chunk` are two different lines.
- **A `Core` instance's slots hold only values Novis already holds, so a member wanting native mutable
  state has to accumulate instead** — there is no destructor to free a `sha2::Sha256` context with, and
  `digest 0.10` cannot serialize one into a slot. The COW-correct read/write of a slot that holds an
  array is `identity_store::borrow`/`edit`/`replace`, which are generic over `(receiver, index, class,
  member)` despite that module being named for § 9's store; `instance::set_slot` is the raw write and
  `instance::slot` the borrowed read. `Core\Hash\Stream` is the worked example.
- **A new domain module is `mod`, not `pub mod`, so its `CLASS`/`NAME` are `pub(crate)`.** The
  workspace warns `unreachable_pub`, and half of `nvs-stdlib`'s modules are `pub mod` while the newer
  half is not — copying `uuid.rs`'s `pub const NAME` into a privately-declared module is a warning at
  build time, before `verify.py` says anything. `objmap.rs:36` is the shape to copy.
- **A `Core` symbol that is not a member breaks `every_registered_member_has_an_implementation_address`.**
  `nvs_stdlib::symbols()` used to be exactly one entry per `CLASSES` member, and that test asserts the
  count — so a constructor symbol from `registry::CONSTRUCTORS`, or anything else chained in beside the
  members, has to be added to the sum on the test's right-hand side in the same edit. The failure is a
  bare `left: 213, right: 211` in `-p nvs-stdlib --lib`, with nothing naming the symbol.
- **A `CoreTy::Array(&CoreTy::Uint)` parameter receives `Tag::Int` elements**, so a helper that reads
  each one through `as_uint` alone answers the member's most obvious call site with a fatal. A written
  `[97, 98]` type-checks against `array<uint>` and stays int-tagged all the way into the helper — a
  scalar `uint` parameter does not have this problem, because the call site materializes the literal at
  the declared type. Read both tags (`str.rs`'s `code_point`), and probe the literal spelling in a
  scratch `.nvs` before writing the case.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes. **An instance member's
  receiver is argument slot 0 and is not in `params`**, so `plus(Duration)` is `args: [2]`. A mismatch is
  an index-out-of-bounds panic at the first call.
- **A member on `registry::WRITTEN_CLASS_MEMBERS` takes one argument its row does not declare** — the
  class its call site wrote, in slot 0 — so its helper's `args: [N]` is `params` + 1 (+ the options bag's
  flattening). `crates/nvs-stdlib/tests/conformance_coverage.rs` looks for such a member spelled `Class::name<`, not
  `Class::name(`, because that is what every call site writes.
- **Registering a `Core` class narrows `Core`'s blanket trust for that name.** An unregistered
  `Core\X::y()` is waved through by `nvs_hir::members`; once `X` is in `registry::CLASSES`, an unknown
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
- **Adding a row to `nvs_hir::errors::TREE` fails a test in `nvs-ir`, and the message names neither
  the tree nor the class you added.** Spec § 10's exception tree is restated as a hard-coded label
  list in `lower/mod.rs`'s `a_file_with_no_class_still_carries_every_compiler_declared_class`, so
  `RecursionError` arrived as a bare `left: [... 12 names ...]` / `right: [... 11 names ...]` diff in
  `-p nvs-ir --lib` with nothing pointing back at the one-line `errors.rs` edit that caused it. The
  full roster a new § 10 class owes is: the `TREE` row, `nvs_runtime::ThrownClass`'s variant, its
  `name()` arm, its `ALL` entry, that assertion, and the spec's own tree drawing. Nothing else
  restates it — `nvs_types::error_lib` seeds whatever `TREE` holds.
- **A second *read* is not cheaper than the `memcpy` it saves, at `Core\Str` sizes.** Both obvious
  ways to make a result's length exact before writing it measured as losses, and each cost a full
  release build plus a bench sweep to find out: counting `Core\Str::replace`'s matches with a second
  `find_from` pass took `05-string-replace` from 0.92× to **0.74×**, and walking a cycle of
  `padEnd`'s padding to measure what a second walk then wrote took `07-string-normalize` from 0.50×
  to **0.45×**. Both are arithmetic now and both rows are far above where they started. The rule to
  carry: reach for arithmetic or for a good capacity guess, never for "measure it first" — and if a
  member's length genuinely costs a data-structure walk to learn, as `Core\Str::join`'s does, leave
  it alone. `crates/nvs-stdlib/src/str.rs` § *A result is written once* holds all of it.
- **`Value::as_str_bytes` and `Value::as_text` make the *same* tag check, so a `from_utf8` after the
  first can never catch anything.** Both go through `Value::str_ptr`, which answers for `Tag::Str`
  alone — and since `Tag::Bytes` became its own row over the shared allocation, a `bytes` argument
  reaches neither. Sixteen sites in `nvs-stdlib` carried the pair anyway, and `json.rs`'s carried a
  doc comment saying it was where "the caller passed binary data into a text format" got caught,
  which had quietly stopped being true. A defence a later ADR made unreachable reads exactly like a
  live one, and grepping for the *tag* it claims to catch is the cheap way to tell them apart.
  `crates/nvs-stdlib/tests/allocation_policy.rs`'s `no_member_revalidates_a_string_argument` is the
  source scan that keeps the pair out now.
- **A roster probe binds its subject; it never `echo`s it, and it gets one panic per run.**
  Sweeping "which `BinaryOp` reaches this catch-all" over a scratch file, an `echo $subject`
  line goes through `concat_operand` first — so an `array<int>` operand reported *that*
  function's missing row and not the operator's, which reads as the probe having found its
  answer. `mixed $x = $a & $a;` asks the question the probe meant. The two phases also answer
  at different rates: the checker reports every diagnostic in the file at once, so one run
  enumerates all of the *refused* shapes, while lowering panics on the first shape that gets
  that far, so each panicking shape costs its own edit-and-run. Put the shapes you expect to
  be refused in one file and the ones you expect to lower in another.
- **`return $local;` retains nothing — it hands the binding's own reference out and tells
  `release_all_locals` to skip that name.** So any binding `release_all_locals` was never going to
  release anyway silently loses the retain: an `inout` parameter is a `Ty::Ref` cell, not
  refcounted, and `return $s;` inside `function grow(inout string $s)` handed the caller a value
  with no owner at
  all, which the caller's own discard then freed while the staged slot still pointed at it. `nvs
  run` printed the right answer and exited **127**. Two things are worth keeping from the hour it
  cost: an exit 127 is worth `git stash`-ing *before* you assume it is yours — this one predated
  the session that found it by a long way — and a `return`/`release_all_locals` exemption keyed on
  a **name** has to be re-read whenever a new binding *representation* enters `Env`, because the
  exemption is only sound for a binding that would otherwise have been released.
- **A non-UTF-8 file in the PHP corpus failed `corpus_parse.rs` with no message at all**, and a real-world
  corpus has them — Symfony ships a class named with the latin-1 byte `0xA9` and a deliberately binary
  string fixture. `SourceMap::load` reads UTF-8 only, and the `unwrap_or_else(|err| panic!(…))` that used
  to read each file sat *inside* the `panic::set_hook(Box::new(|_| {}))` the test installs to silence
  per-file parser panics, so its message was swallowed: a bare `FAILED`, no summary line, no filename to
  chase. Such a file is now counted and named as `unreadable`. The durable trap is the shape rather than
  the file — **any `panic!` between that `set_hook` and its matching `set_hook(prev_hook)` reports
  nothing**, so a new failure path in that loop must return a value the summary can print.
- **A `static` method's slot 0 is the *called class*, not an empty receiver.** Calling one from Rust
  (`nvs_runtime::abi::call` on a `Unit::function("Class::method")` address) with a `null` first slot
  segfaults inside the callee rather than faulting anywhere a message could be printed: ADR 0008's late
  static binding puts a `ClassDesc` there, `nvs_ir::lower` seeds it as `Param(0)` at `Ty::ClassDesc`, and
  `Value::class_desc` is the encoding a compiled call site uses. An instance method's slot 0 is the
  receiver as expected, so the trap only shows up the first time native code calls a `static` one.
- **A scratch `.nvs` file needs its `<?nvs` opener, and without one it "runs" and exits 0.**
  Everything before the opening tag is *inline HTML*, which lowers to an `echo` of the raw
  span — so a scratch file written without it prints its own source back and reports
  success, which reads as "the shape lowered" when nothing was compiled at all. Check that
  the output is the program's answer and not the program, or copy the first line from
  `examples/targets.nvs`. A `.nvst` case's `--FILE--` section has the same requirement and
  is harder to get wrong, every case in the tree carrying it.
- **A `FATAL` cannot be triggered mid-run by the safepoint, because the flag can only be set before
  the run and the script frame's own entry poll fires first** — which is why
  `a_fatal_is_never_caught` sees empty output. The stack limit is no better: `arm_stack_limit`'s
  soft tier answers a catchable `Recursion` long before the floor, so a runaway never reaches the
  fatal tier. What *is* reachable from source, after locals are already live, is a
  `Fault::fatal` from `nvs-stdlib` — `Core\Arr::countBy` over an `array<float>` is one
  (`FATAL: Core\Arr::countBy expected an `int|string` key, got tag 4`). Reach for that when a test
  needs a fatal to happen at a chosen point in a program rather than at its first instruction.
- **Windows compiles none of a crate's `#[cfg(unix)]` half, so `verify.py` on this host is silent
  about it** — a Unix-only type can be green here and not compile at all. The check is one call,
  `wsl.exe -- bash -lc 'cd /mnt/<drive>/<repo> && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo test -p
  <crate>'`, about two seconds warm on that target directory, and it is worth a second one for
  `cargo clippy -p <crate> --all-targets` because the lints are just as unrun. What it caught while
  `NvsUnix` was being written: `mio::net::UnixStream::peer_addr` returns
  `std::os::unix::net::SocketAddr` and **`mio::net` re-exports no address type at all**, so the
  symmetric-looking `mio::net::SocketAddr` is `E0425` — invisible to every Windows leg, including
  the acceptance check.
- **A thread-local whose `Drop` joins threads deadlocks on Windows, and the symptom is a test that
  runs its whole body and then never reports.** `blocking.rs`'s pool is reached from a free function
  through a `thread_local!`, exactly as the reactor is, so its `Drop` runs from a TLS destructor —
  and Windows runs those under the loader lock, which the thread being joined needs in order to run
  its own destructors and exit. The test printed its last line, `run_until_idle` returned a correct
  `RunReport`, and `cargo test` sat there until it was killed; two `nvs_host-*.exe` processes still
  running is what says the binary hung rather than the build. Read the *last* line the test body
  produced before suspecting the code under test — everything after it is teardown. The fix is to
  detach (drop the `JoinHandle`s) and let the threads see a shutdown flag, which is what a pool that
  may be torn down from anywhere has to do anyway.
- **A `loop-goal.toml` acceptance check reports the *first* diagnostic, not the tree's whole
  distance from passing.** Stage 6's check on `examples/isolate.nvs` has said
  `E0703 — 'spawn script' is not compiled yet` for several sessions, which reads as one construct
  away. It is three: `./target/debug/nvs.exe check examples/isolate.nvs` also reports `E0319`
  (`await` is not a constant that exists) and `E0101` on the very next line, because `await` is
  not a keyword — it is nowhere in `nvs-syntax`'s AST, so the example does not even parse past it.
  One `nvs check` of a failing `exact` check's own file, before planning the group that closes it,
  is the difference between a group and a milestone.
- **A valgrind sweep that goes red on *every* fixture at once is one allocation on the startup path,
  and the stack names it in one call.** After the run that landed the script-resolver seam, all 33
  targets reported the same `56 bytes in 1 blocks are definitely lost`; a single
  `valgrind --leak-check=full -q <binary> run examples/hello.nvs` printed
  `nvs::script::Compiler::leaked (script.rs:66)` at the top of the allocating stack, and that was the
  whole diagnosis. The trap is the *shrug*: the leak was deliberate, documented in the module doc, one
  per process and inside ADR 0004's bound, so it reads like something to accept — but the sweep is
  all-or-nothing and a gate with one known-red fixture is a gate nobody reads. A `Box::leak` that
  exists only to widen a borrow to `&'static` has a scoped form that costs nothing
  (`nvs_runtime::script::scoped`); reach for that before reaching for a suppression.
- **`Wake::current()` decides which of two isolate boundaries you just measured**, and a
  `#[test]` or a criterion `b.iter` has no scheduler under it, so it takes the inline one:
  `nvs_host::Isolate::run` outside a task runs the child on the caller's stack and reads about
  **80 ns**, against **0.44 us** for the real path with a stack of its own. Both figures are true and
  only the second is about the boundary a program crosses. The shape that fixes it is
  `benches/abi-probe/shared/isolate.rs`: build the scheduler and the task outside the clock, run the
  timing loop *inside* the task body, and hand criterion a batch through `iter_custom`.
- **A refcount cycle that is still live at exit is a `definitely lost` under valgrind and always will
  be — read the fixture before you read the runtime.** `examples/serialize.nvs` builds a `Ring` whose
  `$self` points at itself, and does it twice (the original and its decoded copy), so the sweep
  reported 48 bytes per ring with `graph::Reader::object` and `nvs_object_new` at the top of the two
  stacks. Nothing in `graph.rs` was wrong: Novis refcounts and has no cycle collector
  (`crates/nvs-runtime/src/object.rs` § *Decision: no cycle collector*, and ADR 0116's *Consequences*
  says the same of an isolate's teardown drain), so a self-referential object's last reference is its
  own field and dropping the local frees nothing. The fixture now breaks both rings by hand before it
  ends. A leak stack whose top frame is an object allocation is the shape to suspect — grep the `.nvs`
  for a cycle before opening the Rust.
- **`<repo>` grants `Authenticated Users` modify, so this repository's own `nvs.toml` fails ADR 0103
  § 6.** The check is right and the drive is what is unusual: a non-system Windows drive's root carries
  that ACE by default and everything under it inherits it, which is the hole § 6 closes. Nothing reads
  the tree through `Files::trust` yet — no crate depends on `nvs-config` — so nothing refuses today, but
  the session that wires the snapshot into `nvs run` will find every run in this checkout stopped by
  `E0607`. The fix is on the machine, not in the code: `icacls <path> /inheritance:d` then
  `icacls <path> /remove:g "<the account>"` for `<repo>\nvs.toml` **and** for `<repo>` itself, since the
  containing directory carries the same rule. Account names are localized — `icacls <path>` prints the
  spelling this machine uses. Ask the user before changing a machine's ACLs; a scratch tree under
  `%TEMP%` passes the check as it is, which is where `crates/nvs-config/tests/trust.rs` works.
- **`tools/bench.py --warm-start` measures `target/release/nvs.exe`, and nothing builds it.** Stage 5's
  acceptance check runs the bench with no `--nvs`, so on a machine that has only ever built debug it
  fails with `no Novis binary at …` rather than with a number — `cargo build --release -p nvs-cli`
  once is the fix, and the header of `bench.py` says why the harness refuses to build anything itself.
  A release binary older than `crates/` still measures: the staleness warning goes to stderr and the
  check stays green, so a start-up regression can hide behind a binary nobody rebuilt.
- **`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh <fixture>` run through a Git Bash shell does
  nothing and reports success.** Git Bash rewrites the `/mnt/...` argument before `wsl.exe` sees it,
  so bash answers `C:/Program Files/Git/mnt/<drive>/<repo>/tools/leak-check.sh: No such file or directory` —
  and the call still exits 0, which reads as a clean leak check on a fixture that was never run. Send
  that command through a PowerShell call instead, which is the spelling the script's own header
  already carries. The same rewrite applies to any WSL-side absolute path passed as an argument.

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
  it.** `$q["b"] as array<string>` panics `nvs-ir` outright — *"got `Tagged as Array`"*, ADR 0007 § 2's
  `array<T> as array<U>` row being the one still missing — so a member answering a nested shape has no
  spelling that reaches past the first level. `Core\Json::encode($q)` renders the whole structure in one
  line and it is byte-identical to PHP's `json_encode` over `parse_str`'s array, which makes it the
  strongest assertion available as well as the only one. `$q["a"] as string` on a top-level scalar does
  lower.
- **A named class does not satisfy a shape type**, whatever its properties are called: `View::y(new
  Point(3, 4))` against a `{y: int}` parameter is `E0401: expected {y: int}, found Point`. ADR 0036 § 3's
  width subtyping is shape-to-shape only (`nvs_types::expr::assign`), so the *one* way a shape receiver's
  static layout differs from the value's own is a narrower shape — which is the only widening a case
  testing § 4's name-keyed read can write. Several doc comments claimed the class direction worked; they
  were wrong and are fixed, so do not design a case around it.
- **A `?array<T>` *is* indexable after a `!= null` guard, and so is every other `?T`.** This bullet
  used to say the opposite and is kept because the shape it warns about moved rather than went away:
  `narrow` drops `null` whatever the residue now, and the read is untagged once at the variable
  (`Lowering::untag_narrowed`), so a subscript, a `foreach`, an element write and an argument all work
  inside the guard. What is still `E0482` is the **untested** one — `Core\Arr::first`/`last` over an
  `array<array<string>>` answers a `?array<string>` and indexing that answer *directly* has no test to
  narrow, so bind it (`?array<string> $row = Core\Arr::first($rows); if ($row != null) { … }`) rather
  than reaching for the old workarounds. **Under a `??` even the untested one is fine**, and that is the
  one exception: every level of a subscript chain below a `??` is guarded, so `$a["nope"]["j"] ?? "d"`
  and `$m["k"] ?? "d"` over a plain `?array<string> $m = null;` both answer `"d"` with no test at all.
- **Registering a `Core` member and writing its conformance case are one slice, not two.**
  `crates/nvs-stdlib/tests/conformance_coverage.rs` fails the moment a registry row has no `.nvst` case
  calling it, so a plan that lands the rows in one session and the cases in another leaves the tree red
  in between — and `verify.py` reports it as a `-p nvs-stdlib` test failure with nothing about the
  member in the message. A class constant counts too: `Core\Path::SEPARATOR` needs a case that writes it.
- **A multi-file `.nvst` case runs every file's statements, each in its own variable scope.**
  `--FILE <relative/path>--` repeats and writes another file into the case's working directory
  (`crates/nvs-test`'s module doc), and `nvs-cli`'s `front_end` resolves, checks and lowers the whole
  `require`/`autoload` graph, so a class declared in a second file is reachable from `nvs run` *and* a bare
  `echo` at its file scope prints, where the `require` is written and once per time that statement is
  reached. Two things a case still cannot assume: a required file's `$x` is not the caller's (ADR 0021
  § *Decision* — declarations cross, variables do not), and an *autoloaded* file is reached by no
  statement at all, so only its declarations ever run. This bullet used to say the opposite half of the
  first sentence.
- **A `--EXPECTF-ERROR--` case must not also *use* what the broken declaration would have provided.**
  Diagnostics are ordered by phase, not by file, so an `E0303` from the entry point's reference is printed
  *before* the resolution error the case exists to pin, and the block no longer matches at its first line.
  A compile-error case's entry file should do the least that reaches the diagnostic — often a bare
  `require` and nothing else. Between two diagnostics `%A` covers the span, notes included.
- **A rule added to `nvs_syntax::check_declarations` reaches far less of the corpus than a grep
  suggests.** Only `nvs-cli` and `nvs_hir::requires` call that walk, so every `nvs-types` fixture, every
  parser test and every `nvs-codegen` fixture goes straight past it — ADR 0094's estimated "sixty inline
  snippets to rewrite" turned out to be eleven, all in `casing.rs`'s own tests. Grep for the *callers*
  before budgeting a corpus rewrite; a `<?nvs` snippet in a Rust string is not automatically subject to
  everything the compiler enforces.
- **A row the checker accepts is not a row that runs.** `nvs-codegen` refuses a binary operator over two
  representations with *"does not lower a binary operator over mismatched representations"*. Equality is
  out of that hole, and so is ADR 0007 § 4's whole promotion table: `$n + $f`, `$n * $f`, `$n ** $f`,
  `$n < $f`, `$n <=> $f` and `$u + $f` all run today, `nvs-ir` having widened the narrower operand before
  the instruction is emitted. The ordering half of the hole is closed at the *checker* now (`E0715`, the
  neighbouring bullet), so what still reaches this refusal is a mismatched pair no widening exists for and
  no diagnostic names — a `Ty::Tagged` operand under an operator, mostly. So a conformance case
  written straight off an ADR's compiling rows can still fail at run time: run the rows in a scratch
  `.agent-tmp/*.nvs` before writing the case, and if one does not lower, pin it in the crate's own
  `tests/` and say in the case comment why it is not here.
- **`"…" as bytes` is how a case writes a `bytes` it can read, and `Core\Encoding::fromHex("…")` is how
  it writes one it cannot.** There is no `bytes` literal at all (`00-overview` § 5), so those are the two
  spellings; `as bytes` is total and free and only reaches octets that are valid UTF-8, which is why an
  arbitrary buffer — a lone `ff`, a truncated sequence — still has to come from `fromHex`. Assert the
  result with `toHex` either way, since `echo` has no `bytes` row: ADR 0009 § 3 makes `bytes as string`
  *checked*, and an implicit render is not that check.
- **`<`/`<=`/`>`/`>=`/`<=>` over two `string`s is `E0715` where it is written**, and this bullet used to
  say it compiled and then failed at run time — it does not any anymore, and the diagnostic names the
  member that does say what was meant. `Core\Str::compare` is the ordering two strings have; the same
  code refuses a `bytes`, an `array<T>`, a `callable`, an enum case (order `as int` instead) and `null`,
  while the object family keeps `E0411` however the receiver was spelled. So a case that wants to assert
  text ordering asserts `Core\Str::compare`, and one that wants a fixed slice still uses `==`. Watch the
  neighbouring trap too: a `Core` member answering `uint` (`Core\Str::length`) in `$int + …` is `E0407`,
  not a widening.
- **`emit_binop`'s `integral` set is `Int | Uint | Bool`, so an enum operand needs the reinterpretation
  first.** `==` over two enum values lowers today because `nvs-ir` compares one representation down —
  `InstKind::Reinterpret` to the backing integer is free, and it is the row `$m as int` already uses — so
  a new lowering that emits a `BinOp` over `Ty::Enum` directly still fails with *"a `Eq` over
  representation Enum(Int)"*.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — CI runs that suite on all three hosted
  runners and none of them has PHP, so an oracle section makes the runner *skip the whole case* there,
  subtracting from the very count Stage 4 measures. Verify against PHP while authoring — `php -r '…'` is
  enough to settle a semantics question, and it is on `PATH` under Windows *and* inside the WSL distro
  ([docs/setup.md](../setup.md)) — then drop the section or put the case in
  `tests/differential/`, which is where an oracle belongs. The `.nvst` format
  is `crates/nvs-test`'s module doc; a `--EXPECTF-ERROR--` block must reproduce the diagnostic's own
  indentation, which widens with the line number. A trailing space before a `\n` is unreliable in an
  `--EXPECT--` block — echo a sentinel character after it.
- **The `php` on Windows `PATH` has no `mbstring`**, so every `mb_*` oracle a `Core\Str` slice reaches
  for — `mb_convert_case`, `mb_strtolower`, `mb_str_split` — dies with *"Call to undefined function"*
  rather than answering. The byte-wise half (`strcmp`, `strnatcmp`, `substr_count`, `str_replace`) is
  all there, so a member replacing both spellings can still be checked on its ASCII rows. For the
  Unicode rows, cite the UCD table the member implements (folding is UAX #44's `C`+`F` mappings) and
  say so in the case's comment; do not silently assert whatever the implementation printed. WSL's
  `php` may have the extension — worth one `php -m | grep mbstring` before writing the rows off.
- **A `nvs-types` test that asserts an interned type's `describe` string is fragile.** A union orders its
  members by type id, so registering a member anywhere can flip `T|null` to `null|T`. Compare against
  `interner.make_union([...])` instead.
- **`NvsStr::from_raw`/`NvsArray::from_raw` return an *owning* handle.** Reading a refcount through one in
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
  included, because `nvs_runtime::NvsObj::new` writes a per-class image the descriptor carries; there
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
  built. `nvs_array_set` renders an `NvsStr`, hands it to the packed arm, which has no use for it,
  and drops it before returning — live delta zero, exactly like the index-taking pair that never
  allocated at all. `counting_alloc::allocated_bytes()` is the monotone total that tells the two
  apart, and any measured claim about a *transient* cost needs it rather than `live_bytes`. The
  control matters as much as the claim: assert the old spelling **does** allocate in the same test,
  or a broken counter reads as a passing guard.
- **A compiled function called with an empty argument slice faults.** `nvs_runtime::call(f, &mut
  ctx, &[])` on a *method* looked like the obvious way to observe a return value, and it is an
  access violation (`0xc0000005`, a bare `STATUS_ACCESS_VIOLATION` from `cargo test` with no test
  name attached) — the callee reads its argument slot whether or not it declared one, and `&[]`
  hands it a dangling pointer. Give the fixture's method one parameter it ignores and pass
  `Value::int(0)`; `nvs-codegen`'s `stack_limit.rs` fixtures all declare one, which is why nothing
  had hit this. `run_with`/`output_of` are unaffected — the script frame is entered the same way but
  never faulted, so the crash arrives only when a test reaches for `unit.function("Class::member")`.
- **An allocation guard over a member that *builds* something measures the result's own storage
  first.** A `counting_alloc::allocated_bytes` delta over one `map`-shaped walk into a fresh
  `NvsArray` read 448 bytes with no key rendered at all: the output's own `Vec` doubling on the way
  to 16 entries is an allocation the guard cannot tell from the one it exists to catch. Walk twice
  and measure the *second* pass, where every write lands at a position that already exists — then
  the only thing left that can allocate is the thing under test.
  `a_callback_that_does_not_want_a_key_synthesizes_none` is the shape.
- **A `cargo test` that dies with a bare `STATUS_ACCESS_VIOLATION` may not reproduce**, so run it
  again before bisecting. One arrived in `-p nvs-codegen --test throwing` on the first run after a
  relink and never returned in four subsequent runs, including the full workspace sweep; the
  `.loop` logs hold an identical one-off in `--test strings`. The deterministic cause below (an
  empty argument slice) reproduces every time, which is how the two are told apart.
- **A test binary outside `nvs-runtime` measures allocations through `nvs_runtime::budget`, and may not
  install a `#[global_allocator]` of its own.** `budget::live_bytes`, `::allocated_bytes` and
  `::allocations` are `pub` and maintained in *every* profile, because the memory limit is read off the
  same counters — so there is nothing left to re-implement, and re-implementing it now fails the build
  rather than duplicating it: `nvs-runtime` registers `budget::Accounting` in every `not(test)` build,
  debug included, and a second global allocator in the test file is
  `error: the #[global_allocator] in this crate conflicts with global allocator in: nvs_runtime`.
  `crates/nvs-codegen/tests/arrays.rs`, `crates/nvs-codegen/tests/closures.rs`,
  `crates/nvs-codegen/tests/throwing.rs` and
  `crates/nvs-stdlib/tests/allocation_policy.rs` all read the shared counters now; they keep their
  `#[cfg(debug_assertions)]` gates, but only because their numbers are pinned against an unoptimized
  build's inlining, never because a counter is missing.

- **Measure compiled code's allocations as a difference between two run lengths, never as an absolute
  zero.** A run allocates its array, its locals and its output buffer once whatever the loop count is, so
  "this script allocated nothing" is not a claim that can hold; "400 passes allocated exactly what 4 passes
  allocated" is, and it is the same claim, because a per-access cost is O(accesses) and a setup cost cancels
  out of the difference. Pair it with a control arm that *does* allocate per access — the same accesses
  through a rendered key — or a counter stuck at zero passes the test for you.
- **A `-p nvs-stdlib` test can hand a `Core` member a real `callable` without a compiler in front of
  it.** `nvs_runtime::call_closure` reads exactly two things off a closure value — slot
  `CLOSURE_ARITY_SLOT`, and the `CLOSURE_INVOKE` method's address in its class — so a
  `ClassTable::define` + `set_methods` pair with a plain `unsafe extern "C" fn` is a whole closure, and
  everything else in `lower_closure`'s representation is captured state a native callback does not
  have. `crates/nvs-stdlib/tests/allocation_policy.rs`'s `closure_of` is the shape; leak the table,
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
- **A `.nvst` case's own helper has to be a `public static function` inside a class.** A case is top-level
  statements, so the instinct when two steps want the same rendering is a plain `function render(...)` at
  file scope — which is `E0215: a function must be a method` (ADR 0011 § 1), caught only when the case is
  run. Wrap it in a `final class` and call it `Render::pairs($m)`; a compiler-owned generic type
  (`Core\ObjectMap<Tag, int>`) is accepted in that method's parameter list, so the helper can take the
  collection the case is about.
- **A case that sweeps a table can factor the sweep into a closure now, but `function (…) { … }` is
  still `E0222` outright** — `fn (…) => …` or `fn (…) => { … }` is the one closure literal (ADR 0031).
  Calling one through the variable holding it lowers (it used to panic), and so does `$f(...$args)`;
  what a call through a `callable` answers is `mixed`, so the result takes an `as T` wherever a
  narrower type is declared. The array-plus-`foreach` shape below is still the one to reach for when
  the sweep's rows are *data* rather than behaviour: a typed array plus
  `foreach`, with the `try`/`catch` inline in the loop body: `array<string> $rows = ["…", …];` — **no
  `var`**, because an array literal is `array<mixed>` and `var` refuses to infer an element type
  (`E0414`), and `foreach ($rows as string $row)` over the literal directly is `E0401` for the same
  reason. Counting agreements into an `int` declared above the loop is how the sweep is then asserted,
  since there is no compound assignment either.
- **Deepening a `.nvst` case in place does not move Stage 4's count.** The gate counts case *files*, so a
  depth slice that rewrites an existing thin case makes real progress the acceptance test cannot see —
  `nvs test tests/conformance` read 478 both before and after two sessions' worth of work. Land the new
  claim as its **own file**, named for the claim, and leave the thin case where it is with a one-line
  comment pointing at the deep one. Splitting after the fact is free; noticing after the run is not.
- **An `--ORACLE--` helper must not be named after a PHP built-in, and the failure does not say so.**
  `pos` is an alias of `current()`, so a case whose oracle declared `function pos(int|false $f)` failed
  with `--ORACLE--: PHP exited 255` and **`php stderr: <empty>`** — PHP writes *Cannot redeclare
  function* to stdout, which the runner is comparing rather than reporting. The Novis half compiles and
  runs, so the failure reads as a broken PHP install. Name an oracle helper for what it renders
  (`render`, `show`) and check `php -r 'var_dump(function_exists("<name>"));'` if in doubt; `key`,
  `next`, `end`, `reset`, `current` and `compact` are the other easy collisions.
- **A differential case checks itself, so write the rows and run it rather than pricing PHP's answer
  by hand first.** An `--ORACLE--` case's failure output prints both columns side by side, which is
  the whole comparison in one call; three `php -r '…'` calls spent pre-computing what a matching case
  was going to assert told this session nothing the first `nvs test <case>` did not. Reach for
  `php -r` for the *divergence* half instead, where the frozen `--EXPECT--` is Novis's own output and
  PHP's answer only appears in the case's prose — that is the one place the runner cannot check the
  sentence you wrote.
- **An `int` literal does not reach an `array<float>`'s element type**, so a differential case about Novis's *one numeric domain* has to declare the subject `array<int|float>`. `Core\Arr::contains($floats, 1)` against an `array<float>` is `E0401: expected float, found int` at the argument — the needle is typed `T`, and the widening `1 == 1.0` gets in an expression is not one an argument position performs. Declaring `array<int|float> $numeric = [1.0, 2.5];` makes `T` the union, the literal fits, and `contains($numeric, 1)` then answers `true` — which is the ADR 0090 § 3 row worth pinning, since `in_array(1, [1.0], true)` is `false` and the loose `in_array(1, [1.0])` is `true`, so Novis matches neither of PHP's two modes.
- **A `Core` member's refusal is a `FATAL:` line on standard error, not a `Throwable`, so `try`/`catch` cannot pin it.** `Fault::fatal` is what `key_bytes` and every argument-shape guard in `nvs-stdlib` raise, and it unwinds past `catch (Throwable $e)` untouched: a case wrapping `Core\Arr::countBy($floats)` in a `try` prints nothing from its handler and exits 1. Pin it with `--EXPECT-ERROR--` instead, whose presence is also what tells the runner this case's run is *meant* to fail — the stdout before the fatal still has to match `--EXPECT--`, so the agreeing rows can sit in the same case. Get the message by running the scratch under `2>` and `cat -A`: it is one line, `FATAL: ` then the member's own text, and it can carry an internal detail (`got tag 4`) that no other section would let you assert.
- **A `Core` member's *ordering* refusal is the other kind of `Fault` and a `catch` does reach it.**
  `Fault::thrown` — what `nvs_stdlib::ordering::compare_values` raises for a pair with no natural
  order, and so what `Core\Arr::min`/`max`/`sort` raise over a mixed-type subject — unwinds as an
  ordinary `Throwable`, so `try { … } catch (Throwable $e) { echo "refused\n"; }` at file scope
  prints and the case carries on. That is the opposite of the neighbouring bullet's `Fault::fatal`,
  which no handler sees, and it is what lets a divergence case render its refusals inline beside its
  agreeing rows instead of ending at an `--EXPECT-ERROR--`. Which one a member raises is decided in
  the helper, not by the member, so check the `Fault::` constructor at the site rather than assuming.

- **A `?bool` cannot be tested for truth, so a member answering one has no `yn` rendering at all.**
  `if ($found as bool)` on a `?bool` parameter panics `nvs-ir`'s truthy-condition slice at
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
  Novis's side has no way to print and the case fails on a row that actually agrees. `INF` is fine. Render the
  value through a guard instead — `$v == $v` is false for exactly one `float`, on both sides — and echo a
  sentinel, which is what `math-int-div-and-mod-match-intdiv-and-fmod`'s `Show::real` does.

- **A `CoreTy::Var("T")` signature binds `T` to the first argument, so a mixed-type pair does not reach the
  runtime at all.** `Core\Math::min`, `max` and `clamp` declare every parameter and their return as one
  `Var("T")`, so `Core\Math::min(0, "a")` and even `Core\Math::min(2, 1.5)` are `E0401` at the *second*
  argument — which reads as "this member rejects the pair" when what is actually wanted is
  `nvs_stdlib::ordering::compare_values`'s throw. Declare the union on the bindings (`int|string $zero = 0;`)
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
  notice is visible there and invisible in the `.nvst` diff, which reports only that the two outputs
  differ.
- **A frozen `--EXPECT--` cannot hold a decomposed grapheme cluster, and nothing warns you.** `"cafe\u{0301}"` sliced at its last cluster renders as `é` — byte-identical in a terminal to the precomposed `é` a keyboard types into the expectation block, and a different string. The case reads as passing-looking and fails with an "expected"/"actual" pair whose two halves are visually the same, which is a long minute to diagnose. Echo `Core\Encoding::toHex($s as bytes)` for any cell whose content is not plainly ASCII; the hex is also the thing a reader of a grapheme-versus-byte case wants to see. The neighbouring rule about a trailing space before a `\n` is the same class of trap.
- **A `?string` does not narrow into a `string` return position, and the fix is `as` rather than a
  different `if`.** A case rendering a nullable through a helper writes the obvious
  `if ($v == null) { return "<none>"; } return $v;` and gets `E0403: this method declares `string` but
  returns `string|null`` — and inverting the test to `if ($v != null) { return $v; }` reports the same
  thing one line up, so the second attempt looks like the narrowing is simply absent. It is not: the
  spelling that checks is `return $v as string;` after the null test, which is what
  `str-last-index-of-matches-strrpos` already does for a `?uint` through `$found as string`. The same
  `as` re-supplies the type inside an expression too — `Core\Str::join([$head as string, $tail as
  string], $cut)` and `Core\Str::slice($s, ($at as uint) as int)` both check, so a nullable answer can
  be fed straight into the next member without a local.
- **`preg_split("//u", $s, -1, PREG_SPLIT_NO_EMPTY)` is the mbstring-free code point splitter**, and it works on the Windows `php` where every `mb_*` the neighbouring bullet names does not — PCRE carries its own UTF-8 support, so a `Core\Str` slice that needs PHP to count *code points* has a real oracle rather than a frozen `--EXPECT--`. What it cannot give you is a code point's *number* (`mb_ord`) or a grapheme (`grapheme_strlen`: no `intl` either), so an oracle needing those still decodes UTF-8 by hand in the `--ORACLE--` block or freezes the rows and cites the UCD table.
- **A PHP *notice* lands in the oracle leg's stdout, so a deprecated twin cannot be swept in an
  `--ORACLE--` file.** On 8.5.9 `chr()` deprecates an argument outside `0..255` and prints
  `Deprecated: chr(): Providing a value not in-between 0 and 255 …` for **every** such call, so a
  sweep over code points past 255 fails on text neither side computed rather than on the answers.
  One `php -r '…'` before freezing a sweep is what catches it — and when the twin is deprecated at
  exactly the rows a case wants, that is usually the same boundary the divergence already sat on,
  so the rows belong in an `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--` anyway.

- **A typed declaration *inside* a loop body is fine; it is the second loop that collides.**
  `uint $want = $cp as uint;` and `array<uint> $points = …;` in a `for` body run 128 times without
  an `E0406` — the binding is declared once and assigned each iteration, which is what the
  neighbouring function-scope bullet means and not what it looks like it means. What collides is a
  *later* loop reusing the name. In the same family: `foreach (Core\Str::codePoints($s) as uint
  $point)` lowers, where the same shape over an array literal is `E0401`, because a member's
  declared `array<uint>` return carries the element type a literal does not.
- **`Core\Json::encode` refuses a `bytes` value**, so it is the way round an `array<mixed>` only
  while every element is a scalar or a string. `Core\Bytes::unpack`'s answer is exactly where that
  bites: a format holding an `a`, `A` or `Z` field yields a buffer element, and encoding the list
  throws *"Core\Json::encode(): tag 11 has no JSON encoding"* — which reads as a bug in the case
  rather than as the missing row it is. Render such a case with `Core\Arr::count` for the shape and
  a separate `Core\Encoding::toHex` for each buffer, and keep `Json::encode` for the numeric
  formats, where it prints the whole list in one line.
- **A `.nvst` helper cannot take a `Core` enum parameter, and the diagnostic names the same type
  twice.** `public static function m(string $s, Core\Charset $c)` called with `Core\Charset::Ascii`
  is `E0401`, reading *expected `Core\Charset`, found `Core\Charset`* — a source-declared annotation does
  not unify with the registry's own `CoreTy::Enum`, and a `mixed` parameter is refused just as hard
  at the `Core` call inside. `var $c = Core\Charset::Ascii;` *does* infer correctly and passes, so
  the hole is specifically the declared type. A case sweeping several charsets therefore writes the
  enum at each call site with an inline `try`/`catch` per row — each `catch` taking its own binding
  name — rather than folding the rendering into the `final class` helper the neighbouring cases use.

- **A frozen `--EXPECT--` must not render an invisible byte either, and a refusal that quotes its
  operand back will.** `Core\Encoding::encodeText`'s message spells the offending character through
  Rust's `Debug`, so U+0080 arrives printable as `'\u{80}'` — but the same message quotes the
  *subject* back unescaped, which puts a raw C1 control in the expectation where no reader can see
  it and no editor shows it. Assert the printable half with
  `Core\Str::contains($e->message, "(U+0080) at offset 0 of")` and echo a `yes`, exactly as the
  neighbouring decomposed-cluster bullet does for the other kind of invisible.
- **A `gaps.py --errors` row marked `thrown_as` carries a *class*, and asserting which one is the
  whole point of the row.** `Fault::thrown_as(ThrownClass::Logic, …)` and `…::Parse` are ordinary
  `Throwable`s, so a `catch (Throwable $e)` reaches both and says nothing about either; the clause
  that discriminates is spelled with the bare class name — `catch (LogicError $e)`, `catch
  (ParseError $e)` — and two such clauses on one `try` compile at file scope, each needing its own
  binding name like any other. The roster of names is `nvs_runtime::throwable::ThrownClass`'s own
  doc comment, one `///` line per class, which is cheaper to read than spec § 10's tree. What makes
  this worth a case rather than a row is that one member throwing two classes is a *partition*:
  `Core\Time::parse` answers `LogicError` for its pattern argument and `ParseError` for its text,
  and counting each table into the other's class is what a member throwing one class for everything
  fails.
- **A case that has to name a `Core\Class::member` inside a *string* writes the literal single-quoted.**
  `'Core\Time::fromIso(): '` is exactly those characters — the lexer treats only `\\` and `\'` as
  escapes in a single-quoted string, PHP's rule, so every other backslash stays literal. This matters
  past taste whenever the member's message ends in a *dependency's* wording: `gaps.py --errors` drops a
  site only when the literal run before the message's first format hole appears somewhere in the suite,
  so a case that asserts `jiff`'s or `regex`'s sentence with `Core\Str::startsWith($e->message, '…')`
  has to spell the prefix out, and cannot instead echo the whole message and freeze a tail the next
  dependency bump will rewrite.
- **`echo "label=", <a call that may throw>` prints the label before it throws**, so a `try` body
  written the obvious way leaks its own prefix into stdout on exactly the rows the case exists to
  refuse — `not refused=Core\Json::encode(): …` on one line, with the refusal's own text welded to
  it. Bind the call first (`var $written = Core\Json::encode($v);`) and echo on the line after, so
  the refusing row prints nothing at all and the `--EXPECT--` block stays a list of the rows that
  answered. In the same family, a local declared in a case obeys `E0112`: `var $written_nan` is
  refused and `$writtenNan` is the spelling, which bites when a sweep needs one `var` per row and
  reaches for snake_case to keep them apart.
- **Check a `gaps.py --errors` site's own parameter types before taking it as catchable.** A
  `Fault::thrown` guarding an *element* of a typed parameter may have no program that can reach
  it: `Core\Csv::format`'s "column N holds a value that is not a `string`" is guarded by an
  `array<array<string>>` parameter and an `array<string>` `{header:}`, and all four ways round it
  fail — `array<array<mixed>>` is `E0401` at the argument, a `mixed` cell is `E0401` inside the
  literal, a whole `mixed` argument is `E0401` at the parameter, and the cast that would launder
  it panics `nvs-ir` at `crates/nvs-ir/src/lower/expr.rs:877` (`array<T> as array<U>` is the
  conversion row still missing). The tool's own header says an entry is a candidate rather than a
  plan; this is the cheapest way to judge one, and it is four `nvs run` calls on a scratch file
  rather than a written case that will not compile.
- **`Core\Bytes::join`'s allocation refusal cannot be reached from source, and the obvious probe
  reports the wrong member.** Its size is the sum of parts that must already be in memory, so a case
  reaching for it by handing it a huge separator — `Core\Bytes::join($parts, Core\Bytes::fill(1e12,
  44))` — is refused by `fill` while it builds the argument, and the message names `Core\Bytes::fill`.
  The row looks like a `join` assertion and pins `fill` twice. The reachable count-shaped refusals are
  `Core\Str::repeat`, `Core\Str::padStart`/`padEnd`, `Core\Bytes::repeat`, `Core\Bytes::fill` and
  `Core\Random`'s two; a member whose size is a *product* reaches both sentences (past `isize::MAX` is
  `nvs_runtime::affordable`'s, below it the allocator's), while one whose size is its own `uint`
  argument — `fill` — can only ever reach the second.
- **An agreement case gets its sweep from a `mixed`-taking helper and `Core\Json::encode`, not from
  a closure.** The shape with the most room left is the one that asks one question of many members
  and asserts they *agree*, and the obstacle is that each member answers a different type — a
  `uint` from `count`, a `bool` from `isList`, a `?string` from `keyOf`, a nested `array` from
  `chunk`. `Core\Json::encode` renders every one of them, including `null` and a float, so a
  `public static function same(string $name, mixed $a, mixed $b, mixed $c): int` inside a
  `final class` compares the three renderings, echoes a `DISAGREE` line naming the member when they
  differ, and returns 0 or 1. The case then reads `$ok = $ok + Sweep::same("count", …);` per member
  — one line each, no closure stored anywhere — and asserts `$ok` against a literal total, which is
  the counting the shape asks for. `mixed` accepts every `Core` return type tried, nullable
  included, and a multi-line call with a trailing comma parses.

- **Do not put an append past `9223372036854775806` in a case: it kills the run.** A key of
  `9223372036854775807` followed by `$a[] = v` trips `Table::append`'s
  `"the append counter never names a live key"` `debug_assert` (`crates/nvs-runtime/src/array.rs:395`)
  and `nvs run` dies mid-file, so every later row of the case is lost and the failure reads as the
  harness rather than as the bound being probed. The last *accepted* append is the one after a key
  of `9223372036854775806`, which lands at `9223372036854775807`; the plan's `Open now` owns why the
  first refused one is a crash rather than PHP's `Error`.
- **`php -m` before designing an oracle around an extension's function.** The `php` on the Windows
  `PATH` is 8.5.9 with `json` and **no `gmp`**, so `Core\Math::gcd`/`lcm` — whose spec **Replaces**
  column names `gmp_gcd`/`gmp_lcm` — cannot be asked their twin at all, and a case calling one would
  die rather than disagree. The way through is the shape `Core\Path::normalize` already uses: write
  the oracle as a **second implementation in PHP**, which for these two is four lines of Euclid and
  is not the thing under test. That still closes the gap, because `gaps.py --differential` looks for
  a call to the **Novis member** in `tests/differential/`, not for the twin's name in the oracle —
  so the judgement about whether the twin is reachable is entirely the session's. The same check is
  what the `mbstring` bullet above is a second instance of.
- **A counter declared `uint` cannot be incremented by a literal**: `uint $n = 0; $n = $n + 1;` is
  `E0407: int and uint have no representable common type in arithmetic`, because the literal is an
  `int`, followed by an `E0401` on the same line reporting the result as `mixed`. The spelling that
  compiles is `$n = $n + (1 as uint);` — parenthesised, since `as` binds looser than `+`. A `uint`
  counter is worth the trouble whenever the total is compared against `Core\Arr::count`, which
  answers `uint`; the alternative is an `int` counter and `Core\Arr::count($rows) as int` at every
  use.
- **An `--ORACLE-DIVERGES--` block is *one line*, however long the prose is.** The harness answers
  `not a valid case: line 3: `--ORACLE-DIVERGES--` is one line` and refuses the whole file, so a
  divergence written as the four paragraphs it wants to be has to be folded back into a single
  paragraph before it will run. Write it as one line from the start and use a capitalised lead-in
  (`THE INTEGER BAND:`) where a `**bold**` heading would otherwise have earned a paragraph break —
  `json-decode-refuses-the-number-band-json_decode-degrades.nvst` is the worked shape. The `--TEST--`
  line has the same rule and always did; this is the block that looks like it does not.
- **`Core\Str::length` counts characters, and a CRLF is one of them** — so it is the wrong ruler for
  a round trip. A `Core\Csv` probe measuring `"a\r\nb"` read 3 on both sides and the obvious reading
  was "the reader normalized the CRLF away"; it had not, and
  `Core\Encoding::toHex($s as bytes)` says `610d0a62` before and after. Any claim about *which bytes*
  survived a member is written with `toHex`, and `length` is kept for what it answers, a count of
  characters.
- **A case that asserts two float computations agree has to pick rows whose *intermediate* is exactly
  representable, or it is pinning one libm rather than a property.** `Core\Math::hypot($x, $y)` against
  `Core\Math::sqrt($x * $x + $y * $y)` agreed on all 21 rows of a first table here, including `[0.3, 0.4]`
  and `[123.456, 789.012]` — but only because MSVC's `hypot` happens to round those the same way, and
  nothing says glibc's does on the WSL leg. Restricting the table to Pythagorean triples, zeros and
  dyadic fractions makes `$x * $x + $y * $y` exact, so both members are computing the correctly rounded
  square root of the *same* double and IEEE 754 requires them to agree — a property of the table, not of
  the host. The same test applies to any "these two spellings answer the same thing" float case.
- **A `Core` member accepts a `tainted` argument wherever its row's mark says so, and what a case has to
  get right is the declared type of the *answer*.** ADR 0088 § 2's classification reaches the checker
  (`MethodSig::param_quals`), so a `Contagious`, `Neutral` or `Launder` parameter takes a tainted
  argument and only a `Sink` refuses one. A contagious call's answer then carries the qualifier through
  the atom, an array's element and every member of a union: `Core\Str::after($t, ",")` is
  `?tainted string` and `Core\Str::split($t, ",")` is `array<tainted string>`, and the plain declared
  type is `E0401`. The spelling bites — `tainted ?string` is not a type this parser has, and it fails as
  `E0102: expected an expression` on the line *before* the mismatch, which reads like a broken case and
  is not one. Two refusals that are also not bugs in the case: `secret` is refused at every one of those
  positions, and a tainted argument is refused wherever the answer has nowhere to carry it —
  `Core\Uri::parse`, `Core\Json::decode`, `Core\Bytes::unpack`. `tests/conformance/reject/` has one case
  per row of this.
- **`Core\Math::atanh` is not exactly odd, and the two legs disagree about which rows it fails on.**
  Every other member of the family is: `sin`, `tan`, `sinh`, `tanh`, `asin`, `atan`, `asinh`, `cbrt`
  and `sign` satisfy `f(-$x) == 0.0 - f($x)` bit for bit on every row of a swept table, and `cos`/`cosh`
  satisfy the even form, because the host computes `f(-$x)` by the same steps it computes `f($x)` by.
  `atanh` does not — it goes through a logarithm of an expression that is not itself symmetric — so
  MSVC misses on 4 of 7 magnitudes in `[0, 1]` and glibc on 2 of the same 7, and not the same 2. A
  parity sweep that includes it fails on whichever leg it was not authored on. Assert a *relative*
  agreement for that one member instead (`abs($there + $back) <= abs($there) * 1.0e-15`, with the
  infinite row taken by exact equality, since infinities negate exactly), which is host-independent
  and still says the member is odd. This is the neighbouring `hypot` bullet's rule met from the other
  side: there the two spellings were different computations, here they are the same computation on
  arguments that are exact — and it is still not enough when the member's own formula is asymmetric.

- **`int as float` is checked and refuses past 2^53, not past `int`'s own range.**
  `nvs_runtime`'s `int_to_float` is `value.unsigned_abs() <= F64_EXACT_INT_LIMIT`, so
  `Core\Math::INT_MIN as float` throws `cannot convert `int` -9223372036854775808 to `float`` even
  though -2^63 is exactly representable as a double. A case sweeping a `float`-typed member over an
  `int` table is therefore bounded at ±9007199254740992, and reaching `int`'s own extremes on the
  float side needs a `float` *literal* — `0.0 - 9223372036854775808.0` — rather than a conversion.
- **`Core\Math::cbrt` is not one of IEEE 754's correctly rounded operations and the two legs
  disagree, so an *irrational* cube root's round trip cannot be frozen either way.**
  `Core\Math::cbrt(2.0)` cubed is exactly `2.0` under glibc on the WSL leg and
  `-1.1102230246252E-15` short of it under MSVC natively — measured, in the session that wrote
  `math-a-root-and-a-logarithm-undo-themselves-only-where-the-answer-is-exact.nvst`. `sqrt` is the
  only root member the standard requires to be correctly rounded, which makes it the only one whose
  inexact rows are the same on every conforming platform: `sqrt(2.0)` squared is
  `4.4408920985006E-16` over 2 everywhere. A *perfect* cube does round trip on both legs — 18 rows
  of both signs, including the dyadic ones — because the answer is representable and every libm's
  final refinement lands on it; that half is safe to assert, the irrational half is not. Same
  reading as the `hypot` and `atanh` bullets beside this one: pick the rows, not the member.

- **Float `<`, `>`, `&&` and `||` all lower, and `0.0 / 0.0` answers `NAN` rather than throwing** —
  which together are what let a case assert "near, not equal" without a member. The neighbouring
  bullet about `E0715` is about *strings* and the other unordered domains; two `float`s compare for
  order fine. So the tolerance spelling a float agreement case wants is
  `float $tol = 0.000000000001 * ($mag + 1.0);` then `if (($d < $tol) && ($d > 0.0 - $tol))`, with
  the magnitude taken by hand (`if ($mag < 0.0) { $mag = 0.0 - $mag; }`) because
  `Core\Math::abs` answers the `int|float` union and not a `float`. `-0.0` echoes as `-0` and is
  read through `1.0 / $x` when it has to be told from `0.0`, as the parity case does.
- **A float landmark is exact on both legs, and `php` inside WSL says so without a Linux build of
  `nvs`.** PHP calls the same libm Rust's `f64` methods do, so
  `wsl.exe -- bash -lc "php /mnt/<drive>/<repo>/.agent-tmp/rows.php"` answers the "does glibc round this the
  same way MSVC does" question in one call, against the two minutes a cross-build costs. Measured
  that way and safe to assert as *equalities*, on both legs: `acos(-1.0) == PI`, `asin(1.0) == PI /
  2.0`, `acos(0.0) == PI / 2.0`, `atan(1.0) == PI / 4.0`, `cos(PI) == -1.0`, `sin(PI / 2.0) == 1.0`,
  `exp(1.0) == E` and `log(E) == 1.0` — every one a domain endpoint or a halving, which is the same
  "pick the rows, not the member" reading as the `hypot` and `cbrt` bullets. The *interior* of the
  circle is not: `sin($x) * sin($x) + cos($x) * cos($x)` needs the tolerance spelling above even
  though it agreed on all 20 rows of one table.
- **A float round trip has a well-conditioned direction and an ill-conditioned one, and the case
  picks the direction rather than loosening the tolerance.** Composing a member with its inverse
  amplifies the inner answer's last digits wherever the outer member is steep, so the same claim is
  host-independent one way round and a measurement of the host's libm the other. Measured here, all
  on both legs through `php`: `asinh(sinh($x))` and `acosh(cosh($x))` are contractions and agree to
  `1.0E-13` relative on every row of a table out to ±5 — `acosh` recovers `|$x|`, since `cosh` is
  even — while `atanh(tanh($x))` is not: `atanh` amplifies by `1 / (1 - $y * $y)`, which is 5500× at
  `$x = 5`, so that round trip is written `tanh(atanh($y))` over a table of `$y` inside `(-1, 1)`
  instead. Same reading for the circular three: `asin(sin($x))` and `acos(cos($x))` need every row a
  quarter radian clear of a turning point (`±PI/2` for `asin`, `0` and `PI` for `acos`), where
  `atan(tan($x))` is well conditioned even beside the pole because `tan`'s blow-up and `atan`'s
  contraction cancel. This is the `hypot` and `atanh` bullets' "pick the rows, not the member" met
  from the composition side.

- **The hyperbolic and inverse-circular rows that are exact on both legs, so a case can assert them
  as equalities.** Measured through `php` natively and inside WSL, alongside the list in the
  neighbouring *A float landmark is exact on both legs* bullet: `sinh(0.0) == 0.0`,
  `cosh(0.0) == 1.0`, `tanh(0.0) == 0.0`, `asinh(0.0) == 0.0`, `acosh(1.0) == 0.0`,
  `atanh(0.0) == 0.0`, `asin(sin(0.0)) == 0.0`, `acos(cos(0.0)) == 0.0`, `atan(tan(0.0)) == 0.0`,
  `acos(cos(PI)) == PI` and `asin(sin(PI / 2.0)) == PI / 2.0`. Two saturation rows are exact as
  well and are properties rather than digits: `tanh(20.0) == 1.0` — past about 19 the two
  exponentials are a double's whole precision apart, so the ratio *is* one — while `cosh(20.0)` is
  still finite, and at 1000 both `cosh` and `sinh` answer `INFINITY` rather than refusing, a
  hyperbolic member having no domain to leave on that side.
- **A spread argument lowers now, so a case that composes a variadic member composes it.**
  `Core\Path::join(...Core\Path::split($p))` is how `path.rs`'s own doc comment writes the round
  trip, and it used to panic `nvs-ir`; it does not any more. The fold this bullet used to prescribe
  — a `public static function` helper walking the `array<string>` — is still what a case reaches for
  when the *pieces* are the subject, and two spellings inside it are still worth knowing: an array is
  indexed by the *string* of the offset (`$parts["0"]`, `$parts[$i as string]` inside a loop), and
  `Core\Arr::count($parts) as int` is what an `int` counter may be compared against.

- **A count that compares a library-built path against its input string is leg-dependent, and length
  is the way round it.** `Core\Path`'s members re-render with `Core\Path::SEPARATOR`, so "the rebuilt
  path differs from the input" counted 16 of 20 rows on the Windows leg and would count 4 on the WSL
  one — a non-vacuity counter that means two different things. `Core\Str::length($rebuilt) !=
  Core\Str::length($p)` counts only what was genuinely dropped (a repeated or trailing separator),
  which is 4 on both. The goal's *Path and the two legs* decision covers what a case may *print*;
  this is the same trap one step earlier, in what it may *count*.

- **`Core\Path::split` keeps `.` and `..` as elements**, so `join` of what it returned is the path
  with its separators respelled and **not** its normal form — resolving them is `normalize`'s job
  alone. The round trip is therefore an identity *through* the normal form (`normalize($rebuilt) ==
  normalize($p)` on every row) and exact only on a path that is already normal. A case asserting
  `join(split($p)) == normalize($p)` fails on 8 of 20 ordinary rows.
- **A `!= null` guard does not re-type a nullable local for an *argument* position; `as string` inside
  the guarded branch is what does.** ADR 0066's narrowing is what lets `->` reach a member of a `?Foo`,
  and `tests/conformance/lang/a-null-test-narrows-a-nullable-local.nvst` only ever pinned that shape —
  so `Core\Str::replace($r, …)` inside `if ($r != null)`, where `$r` came from a `?string` member like
  `Core\Path::relativeTo`, is `E0401: expected string, found string|null`, and declaring the local
  `?string` instead of `var` changes nothing. Two spellings do work and both are worth preferring to a
  narrowing that would silently start working later: `$r as string` inside the branch, a checked
  conversion (ADR 0007 § 2) that would throw rather than lie if the guard above it were wrong, and
  `$r ?? "<null>"` where the value is only being echoed — which is also how a case prints the refusal
  itself, since `echo` has no `null` row.
- **A green conformance case can be pinning the bug you are about to fix.**
  `reading-an-absent-array-key-throws.nvst` asserted `$maybe["gone"] ?? "stored-null"` *throws* —
  it was written to pin ADR 0007 § 7 row 11 and reached for `??` as a convenient way to spell
  the read, freezing the exact divergence from PHP that row was not claiming. So when a case
  goes red under a fix, check its expectation against PHP (`php -r '…'`) before adjusting
  either side: a `--EXPECT--` block is only as authoritative as the session that wrote it, and
  a case using a construct incidentally is where a wrong one hides.
- **`python tools/loop.py --list` names the exact `.nvst` *filenames* each stage owes, and a case
  written under a different name does not count toward them.** `python tools/holes.py --item N`
  prints the same names under "cases that may belong to it", which is the cheapest place to see them
  — one call, before writing the case rather than after. Two cases went in as
  `a-named-argument-fills-its-own-parameters-slot` and `a-spread-argument-unpacks-like-phps` when the
  stage was owed `a-named-argument-binds-by-name-and-a-spread-by-position` and
  `a-named-and-spread-argument-match-phps`; renaming afterwards is a `git mv` plus a re-run, but the
  owed name is also a *specification* — it said "and a spread by position", which is a row the first
  draft did not have.
- **`echo` writes its arguments one at a time, so a throwing call inside an `echo` list prints
  everything to its left first.** A `try { echo "did not throw, ", $f(1) as string, "\n"; }` that is
  *meant* to throw leaves `did not throw, ` on stdout ahead of the `catch`'s own line, and the
  `--EXPECT--` you then freeze pins that as correct. Bind the call first — `string $s = $f(1) as
  string; echo "did not throw, ", $s, "\n";` — so the negative branch prints nothing at all when the
  positive one is what happens.
- **`echo` writes its arguments one at a time, so a throwing call inside the list prints the
  prefix first.** `echo "did not throw ", Core\Arr::map($edge, $half), "\n";` inside a `try`
  prints `did not throw ` and *then* lands in the `catch`, so the expected output grows a
  fragment that reads like the case failing open. Bind the call above the `echo` —
  `var $past = Core\Arr::map($edge, $half); echo "did not throw ", ...;` — which is why the
  `.nvst` cases beside it do.
- **Two `catch` bindings of the same name at file scope are fine only while they name the same
  class.** `catch (LogicError $e)` twice compiles, and so do fourteen of them — which is what makes
  a counted refusal sweep possible at all, since a closure cannot be called through the variable
  holding it and the sweep cannot be factored. Add one `catch (Throwable $e)` after them and it is
  `E0406: $e is already declared`, pointing at the first: a `catch` binding is the function's, and
  the class it names is part of the declaration that clashes. Give a differently-typed handler its
  own name.
- **Two file-scope `catch`es may share a binding name only while they name the same class.** The
  neighbouring bullet says several file-scope `try`s each binding `$e` compile, and that holds — but
  `catch (ArithmeticError $e)` followed by `catch (LogicError $e)` is `E0406: `$e` is already declared`,
  pointing at the first clause, with no loop anywhere in the file. ADR 0007 § 1's declare-once rule is
  about the *binding*, so a case that catches two different classes needs two names (`$a` for every
  `ArithmeticError` clause, `$l` for every `LogicError` one) and can keep reusing each within its class.
- **A property default is a scalar literal or `[]`, and a *hooked* property takes none at
  all.** `public array<string> $rows = ["a"];` is `E0472`: a default is written into every
  fresh instance's slot at compile time, so a case that needs a seeded `array<T>` property
  seeds it in `constructor` — `array<int> $seed = ["0" => 1]; $this->counts = $seed;`, which
  is also the one spelling that gets past an array literal's own `array<mixed>` type. Putting
  a `{ get => …; set { … } }` block after a default is then a *parse* error that cascades into
  six more, so the hook block reads as broken syntax rather than as the illegal default it
  follows. Copy the shape from
  `tests/conformance/lang/every-write-spelling-agrees-on-a-refused-element-target.nvst`.
- **A parser refusal that yields `ExprKind::Error` doubles its own `--EXPECTF-ERROR--` block.**
  `Error` types as `mixed`, so every binding fed by one reports an `E0401` right beside the refusal
  that caused it — `@$n * 2` printed `E0236` *and* "expected `int`, found `mixed`" at the same span,
  once per site, turning a four-site case into eight blocks of expectation that say nothing. Handing
  the *operand* back in place of the whole prefix is the fix at the source and the better recovery
  besides; the legacy cast in `nvs-syntax/src/parser/expr.rs`'s `parse_unary` keeps `Error` only
  because `(int)$x` names a target type it cannot honestly produce a value of. Decide which of the
  two a new refusal wants *before* writing the expected block, not after pasting it.
- **`lower_first_method` lowers `T`'s *first* method, so a lowering fixture puts the method
  under test first and its helpers after it.** The instinct is to declare the callee at the
  top the way a `.nvst` case does, and the snapshot that comes back is then the callee's own
  three-line body — which looks like a lowering that produced nothing rather than like the
  wrong function, because a `static function m(): void { }` lowers to exactly a `safepoint`, a
  `param` per declaration and a `return`. Recognizing it costs one `cargo insta` cycle;
  reordering the two members is the whole fix.
- **A named case a stage owes may be pinning a hole rather than a landed feature, and the
  failure mode is a *missing* line rather than a wrong one.**
  `a-finally-runs-when-its-catch-body-throws` was handed over as the group's easy third
  slice — no Rust — and the shapes ran, exited 0 and printed something plausible: the
  `finally;` marker was simply absent from every line. Freezing what `nvs run` printed
  would have pinned the divergence as the expectation. So run a new case's shapes in a
  scratch `.agent-tmp/*.nvs`, write the same program as `.php`, and **diff the two** before
  filling in `--EXPECT--` — a conformance case takes no `--ORACLE--`, which is exactly why
  its expectation is the one nothing else checks against PHP.
- **An array literal's key arrow is `=>`, not the shape literal's `:`.** ADR 0036's
  anonymous object is `{x: 1}` and it is easy to carry that colon into the array
  form, where `["a": 1, "b": 2]` is not a near-miss but a parse failure that
  reports `E0102`/`E0101` three times over one line and hides whatever else the
  case was actually asserting. `examples/arrays.nvs:12` is the spelling —
  `["alpha" => 1, "beta" => 2]` — and PHP's own arrow is the one Novis kept.
- **An `--EXPECTF-ERROR--` section is matched whole, not as a prefix**, so a sweep that
  ends at its last `error[...]` line fails against a compiler that then prints
  `error: aborting due to N errors`. End the section with `%A` and that line — the count
  included, which is also a second assertion that no *extra* diagnostic crept in. The
  existing `a-void-call-is-not-an-operand.nvst` has the shape; a case written from the
  conventions' skeleton alone does not.
- **An inline-HTML run is assertable as a *value*, which is what lets a case about one count
  rather than read.** `Core\Out::capture(fn (): void => { ?>text<?nvs })` lowers — a run is a
  statement, and a closure's block body is a statement list, so the `?>` inside an array literal
  of `array<callable>` parses fine — and the `Core\Cli\Text` it answers takes `as string`, so
  `Core\Str::compare($t as string, "text")` is the comparison. That is the only way to put a raw
  span and an `echo` of the same literal side by side, since a run writes straight to the output
  and has no other spelling.
- **The coverage gate matches a member's *fully qualified* call spelling, so a case written
  entirely through `use Core\Test;` leaves every member it calls uncovered.**
  `crates/nvs-stdlib/tests/conformance_coverage.rs` greps the `--FILE--` sections for
  `Class::member(` with the class's whole name in it, so `Test::assertTrue(` after a `use`
  answers for nothing and `cargo test` fails with "N registered `Core` member(s) are never
  used by a conformance case" long after the case itself is green. Write each new member
  once in the `Core\…` spelling somewhere in the case — which is worth a line of its own
  anyway, since the two spellings being one member is a fact about `use`.
- **A `--ORACLE-DIVERGES--` section is one line**, and the runner refuses the case
  before it runs anything otherwise (`line N: `--ORACLE-DIVERGES--` is one line`). The
  standing cases read as paragraphs because they are one very long line that a viewer
  wraps, so a divergence written as prose with blank lines between its paragraphs looks
  exactly like them on disk and fails at the parse. Write it as one line from the start.
  The neighbouring fact, worth knowing before choosing the twin: the `php` on this box
  has **no `mbstring`** (the playbook bullet above owns that) but **does** have `iconv`,
  and `strptime` was removed from PHP outright, so `DateTime::createFromFormat` is the
  twin a `Core\Time::parse` oracle is written against.
- **A `Core` enum case cannot be held in a binding of its own type**, so a case that sweeps a
  table of them writes each row out rather than looping. `Core\Charset $c = Core\Charset::Ascii;`
  and `array<Core\Charset> $sets = [Core\Charset::Ascii, …]` are both `E0401` reading "expected
  `Core\Charset`, found `Core\Charset`" — the assignability check between a `Core` enum's *case*
  type and that enum's own type fails where the identical shape over a user-declared `enum Mode`
  passes. The way round is to write the case inline at every call site, which is what the three
  `encoding-*` differential cases do and why their sweeps are eight `if` lines rather than a
  `foreach`. It is a checker hole rather than a rule, and the handoff's `## Backlog` carries it.
- **A `for` header's initialiser cannot declare a typed local.** `for (int $i = 0; $i <
  200000; $i = $i + 1)` is twelve diagnostics in one line — `E0102`/`E0101` at the type
  keyword, then `E0301` and `E0401` on the `$i` that never got declared — because the
  header's first clause is an *expression*, not a statement, and ADR 0007 § 1 wants every
  binding declared. Declare it above the loop and leave the clause empty, or write the
  `while` the loop already is: `int $i = 0; while ($i < 200000) { … $i = $i + 1; }`. The
  first error's span points at the type keyword, which reads like the type is unknown
  rather than unexpected, so it is worth knowing before the eleven that follow it.
- **A `Core` enum cannot be a parameter's declared type**, so a sweep over `Core\Unit` or
  `Core\Weekday` cannot be factored into a closure or a helper that takes the case. `fn
  (Core\Unit $u): string => …` compiles, and the *call* is then `E0401: expected
  `Core\Unit`, found `Core\Unit`` — a written annotation for a registry-owned enum interns
  to a different type than the one `CoreTy::Enum` seeds, and `$u as int` inside such a body
  is `E0708` for the same reason, where `$d->weekday() as int` off the member's own return
  type runs. A user-declared `enum Local: int` is fine both ways, so it is `Core`-specific.
  The way round is to factor the *comparison* instead: pass the two formatted strings, or
  two `DateTime`s and an `int`, and write the enum case at each call site.
- **A `$` inside a double-quoted string interpolates, and the escape is `\$`.** A case
  about a character class writes the class out — RFC 5322's `atext` is
  `!#$%&'*+-/=?^_` plus a backtick and `{|}~` — and the bare `$%` in the middle of it is
  read as a variable rather than as two bytes. `"!#\$%&'*+-/=?^_`{|}~"` is the spelling
  that compiles; a backtick and an apostrophe need nothing inside double quotes, and
  `'q\'s'` is the single-quoted form. One scratch `nvs run` settles which of these a
  version of the lexer takes.
- **`echo` writes its operands one at a time, so a throwing call in the middle of one
  leaves half a line on stdout.** `try { echo "[", $t, "] port=", Show::port($t), "\n"; }
  catch (Throwable $e) { echo "[", $t, "] refused\n"; }` prints
  `[x] port=[x] refused` — the first three operands were already written when the fourth
  threw, and the catch starts a second line's worth of text on the same line. It reads as
  a case whose expectation is subtly wrong rather than as a case written in the wrong
  order. Evaluate the throwing call into a local *inside* the `try` and echo the whole
  line after it: `string $shown = Show::port($t); echo "[", $t, "] port=", $shown, "\n";`.
- **A `Core` enum reaches a `Core` member only as the case written at the call site.** A
  `Core\Unit` is `E0401` the moment it travels through anything the program declares: an
  `array<Core\Unit>` literal refuses every element, and a helper's own
  `public static function f(Core\Unit $u)` refuses both its uses and its call sites — with
  the diagnostic reading *"expected `Core\Unit`, found `Core\Unit`"*, which is why it looks
  like a compiler bug rather than a missing conversion. So a sweep over a `Core` enum's
  cases cannot be factored at all: write the rows out (a generator script into
  `.agent-tmp/` is the cheap way to author 22 of them) and let counters carry the
  assertion, exactly as the *invariance over a sweep* shape asks. `Core\Weekday` behaves
  the same; the case-to-parameter direction at a *`Core` member's own* row is fine, which
  is what makes the restriction easy to miss.
- **`Core\ObjectSet`'s `union`/`intersect`/`diff` answer an *unparameterized*
  `Core\ObjectSet`, so an algebra result can only be consumed by chaining off it.**
  `objset.rs:89` spells the return type `CoreTy::Instance(NAME)`, which carries no type
  argument, so `Core\ObjectSet<Tag> $u = $s->union($s);` and passing that result to a
  helper are both `E0401: expected `Core\ObjectSet<Tag>`, found `Core\ObjectSet`` — a
  diagnostic whose two sides read as the same class. Widening the helper's parameter to
  the bare `Core\ObjectSet` does not rescue it either: that spelling is `E0442` in a
  *declared* position. So "are these two sets the same" is written inline as
  `$a->diff($b)->count() == 0 && $b->diff($a)->count() == 0` plus the counts, and a case
  that wants a set-equality helper takes its two operands as the parameterized type and
  never as an algebra result. `Core\ObjectMap`'s members have the same shape.
- **A counting sweep adds `$b ? 1 : 0`, never `$b as int`.** `bool` converts to `string` and to
  `bool` alone (`E0708`, ADR 0007 § 2), so the *invariance over a sweep* shape — the one that
  asserts a property by counting agreements rather than reading them off a line — spells its
  counter `$n = $n + (Core\Str::contains(…) ? 1 : 0);`. The diagnostic's own help says so, but it
  costs a compile to find out. The rest of the shape does work: an
  `array<Core\Time\Duration> $each = [0s, 1ns, …]` of `Core` instances iterates with a typed
  `foreach` binding, `continue` skips the row a law does not apply to, and a bare
  `Core\X::member($arg);` is a legal statement when the case only wants the throw.
- **A handoff's named group can be already landed, and `gaps.py`'s depth number will not
  say so.** The `Core\Math` group named three slices — `abs`/`sign`/`intDiv`/`mod` at the
  ends of `int`, `clamp`/`min`/`max` agreeing on one ordering, `toBase`/`fromBase` at the
  radix ends — and all three claims were on disk verbatim
  (`math-int-div-and-mod-at-the-edges-of-their-arms`, `math-magnitude-sign-and-the-two-
  extremes`, `math-clamp-agrees-with-the-min-max-composition-until-the-range-is-empty`,
  `math-both-base-members-stop-at-the-same-two-bases`). Depth is cases *per member*, so a
  class can rank thinnest while every claim anyone would think of is already made. **One
  `ls tests/conformance/core/ | grep -i <class>` plus a `head -8` of the two nearest names
  costs one call and is the first thing to spend it on** — before reading the member's
  implementation, not after. When they are all taken, do not write a fourth row of the
  same shape: find the claim no case makes (here, that `Core\Math` splits into an
  `int`-parameter family that reaches both ends of `int` and a `float`-parameter family
  that stops at 2^53), and say in the handoff that the group's premise was stale.
- **A top-level element of an `array<mixed>` reads fine as a `mixed` argument, which is how a
  sweep over heterogeneous rows is written.** The neighbouring bullet — a case cannot index
  into an `array<mixed>`'s *elements* — is about the second level only: `$lefts[$i as string]`
  handed straight to a `mixed` parameter lowers, so a table whose rows are an `int`, a `float`,
  an array and an object is three parallel arrays (`array<string> $labels`, `array<mixed>
  $lefts`, `array<mixed> $rights`) plus a `while` counter and a `public static function` that
  asks the members about one row. In the same family, and cheap to trip over in a scratch
  probe: a file-scope `catch (Throwable $e1)` collides with a *later* `array<mixed> $e1 = …`
  (`E0406`), because the catch binding is the function's — name each catch for what it caught.
- **A closure is an object of a compiler-synthesized class, so `shown`'s `a closure` arm is
  unreachable from source.** `Core\Test::assertSame($f, $g)` over two `callable`s prints
  ``a `Script$fn0` `` and ``a `Script$fn1` `` — ADR 0031's closure lowering makes a literal an
  object with one field per capture, so the value carries `Tag::Object` and the class name it
  reports is the synthesized label. Keep a `callable` out of a case about how a value renders:
  the row pins a name no ADR owns. The same goes for `Tag::Resource` and `Tag::Unset`, which no
  file-scope expression produces at all.
- **A `Core` enum case will not go in an array or through a parameter, but a `var` binding takes
  one — and a `match` on an index is how a case sweeps a whole roster.** A source-written
  `Core\Charset` does not unify with the registry's own enum type, so both
  `array<Core\Charset> $sets = [...]` and a `Core\Charset` parameter are refused with `E0401`
  ("expected `Core\Charset`, found `Core\Charset`"), and an array literal of cases under `var` is
  `E0414` on top of that. What does work is `var $cs = Core\Charset::Utf8;`, and therefore
  `var $cs = match ($i) { 0 => Core\Charset::Utf8, …, default => … };` inside a `for` over the
  case count — which is what let the two encoding cases cross 41 charsets with a table of texts.
  Both `encoding-text-round-trips-through-every-charset-the-registry-names.nvst` and
  `encoding-isvalidtext-is-decodetext-s-verdict-over-the-whole-roster.nvst` carry the spelling.
- **A `Core` member's numeric parameter is often `uint`, and a helper factoring a sweep has to
  declare it that way.** A literal `64` places as `uint` at the call site, so
  `Core\Str::repeat("36", 64)` compiles and hides the rule — but the moment the count comes
  through a helper parameter typed `int`, `E0401` says "expected `uint`, found `int`" at every
  call. Type the parameter `uint`; the arithmetic on it (`$block - 5`) stays `uint` and is fine.
  The mirror of the same rule on the way out: `Core\Str::length` *returns* `uint`, so a loop
  counter fed from it wants `as int` or a `uint` of its own.
- **A `Core` member whose parameter is an enum-case union will not take the whole enum.**
  `Core\Hash::hmac`'s third parameter is ADR 0047 § 3's
  `Core\Digest::Sha256|Core\Digest::Sha384|Core\Digest::Sha512`, not `Core\Digest`, so a helper
  that forwards a digest to it must declare that union verbatim — a parameter typed
  `Core\Digest` is refused at the forward even though every value reaching it is one of the
  three. The union spelling parses in a parameter position and widens to `Core\Digest` for the
  members that take the whole enum, so one helper can forward to both `hmac` and
  `Core\Hash::stream`.
- **A member's *case count* does not say which shapes those cases already assert.**
  `gaps.py --coverage` ranks by cases-per-member, so a member can sit at 2 and already have
  both halves of its bound pinned — `Core\Bytes::at` did, in
  `bytes-reads-name-the-octet-they-stop-at.nvst` (last index each way, first refused each way,
  the empty receiver, and the refusal's own message) *and* in
  `bytes-indexes-and-orders-by-byte-offset.nvst`. Read the member's existing cases before
  writing the shape a handoff line asks for: what was actually left there was the index
  *type*'s ends, `i64::MAX` and `i64::MIN`, where `addressed`'s count-back-from-the-end could
  wrap into range, and that is two echo lines appended to the case that already owns the
  boundary rather than a new file duplicating it.
- **An enum declares its cases without the `case` keyword, and a map literal uses `=>`.**
  `enum Level: int { Off = 0, On = 1, }` — writing PHP's `case Off = 0;` is `E0220` plus an
  `E0101` per case, eight diagnostics for a four-line declaration, and none of them names the
  spelling that works (`tests/conformance/enum/an-enum-carries-negative-and-zero-cases.nvst` is
  the shortest example). `["a" => "1"]` is the map literal; `["a": "1"]` is the shape-literal
  syntax and dies at the colon. Both are two seconds in a scratch file and ten minutes if a
  written case is where you find out. In the same family, and the one worth knowing on its own:
  **an enum case behind a `mixed` reads *falsy* in a condition when its backing integer is `0`**,
  because ADR 0047 § 5 spends no representation on hiding it and the runtime table dispatches on
  the tag — while the same case behind its declared type is truthy, which is what ADR 0035 § 4
  actually decided. Do not assert the erased row as if it were the ADR's answer.
- **A `!= null` narrowing does not survive into a loop body, so a nullable receiver is
  nullable again inside a `foreach`.** `var $found = Core\Regex::match(…); if ($found !=
  null) { $found->groups() … }` narrows fine, and `$found->group($key)` one line further in,
  *inside a `foreach` over those groups*, is `E0459` — "this receiver is nullable" — with the
  narrowing test still four lines above it. The same thing hits an argument: passing `$found`
  to a `public static function` declaring `Core\Regex\Match` is `E0401: found
  null|Core\Regex\Match` from inside the loop and accepted outside it. So a case whose sweep
  is over a `?T`-answering member declares its helper's parameter `?Core\Regex\Match` and
  reads it with `?->` plus a `??`, rather than narrowing once at the top and trusting it. In
  the same family and cheaper to hit: `foreach ($m?->groups() as …)` is `E0443` outright, the
  nullsafe chain's own `null` being part of the iterated type.
- **How a case counts what a callback did: ADR 0031 § 2's `Counter` object, at file scope, in a
  block-bodied `fn`.** A closure captures by value and there is no `use (&$n)`, so "was this callback
  called, and how often" looks unaskable from a `.nvst` case — but a captured *object* is still shared,
  and the whole shape lowers today: `final class Seen { public int $calls = 0; }`, `var $t = new
  Seen();`, then `fn (Core\Regex\Match $m): string => { $t->calls = $t->calls + 1; return "X"; }` handed
  straight to `Core\Regex::replaceWith`. The counter is readable after the member returns, which is what
  turns "the answer looks right" into "the callback ran exactly `min(limit, matches)` times". Three
  spellings to get right on the way: a block-bodied `fn` must declare its return type, there is no `++`
  or `+=` so it is `$t->n = $t->n + 1`, and the closure may be declared fresh inside a `foreach` body
  (the binding is the function's, but re-executing its declaration is not a second declaration).
- **The handoff's named group may already be on disk, under a filename that does not say so.**
  Two of this group's three items — the four encoders' round-trip sweep and `with` agreeing with
  the seven readers — were already written, as
  `uri-percent-coders-are-two-inverse-pairs-over-a-byte-sweep.nvst` and
  `uri-seven-readers-with-and-tostring-are-one-parse.nvst`, neither of which reads like the item
  that named it. `gaps.py` counts *calls* per member, so a member three cases mention in passing
  still ranks thin while the property you were about to assert is already pinned. The check is two
  seconds and belongs before the first read of the implementation: `ls tests/conformance/core/ |
  grep -i <class>` and then `sed -n '2p'` over every hit — the `--TEST--` line is written to be
  exactly this index. Take the ranking off `gaps.py`, but take *which property is still open* off
  those lines.
- **`echo` writes its earlier operands before a later one throws**, so
  `try { echo "accepted [", Core\Uri::decodeComponent($bad), "]\n"; } catch ...` prints the
  `accepted [` prefix of *every* refused row and the expected output grows a run of them —
  which reads as the member having accepted the row. Bind the call on the line above
  (`var $read = ...;`) and echo only after it returned. This is the throwing-row twin of the
  `--EXPECT--` block's trailing-space rule: what a case prints before a refusal is part of
  what it pins.

- **A `Core` class with a `compareTo` is still not `Comparable` to the checker.**
  `Core\Uri` has the member, its doc comment names ADR 0013, and `$a < $b` over two of them is
  nonetheless `E0411: does not implement Comparable` — `nvs_types::expr::operators` asks
  `nvs_hir::implements_interface`, and no `Core` class is on that graph as implementing one.
  Two consequences for a case: the ordering operators, `Core\Arr::min`/`max`/`sort` and the
  spaceship all refuse a `Core` object, so `$x->compareTo($y)` written out is the only order
  such a pair has; and a `{comparator: ...}` closure over them must declare its parameters as
  the class itself, because `mixed as Core\Uri` is `E0711` ("this target names no class to
  test the value against") — ADR 0007 § 2 tabulates no conversion into an object.
- **A multi-argument `echo` prints its arguments as it evaluates them, so a `try` whose
  `echo` mixes a label with the call being tested prints the label and then throws**,
  leaving a half-written line above the `catch`'s own output that no amount of reading
  the `--EXPECT--` block explains. `echo $row, " -> ", Core\Uri::parse($row)->resolve("g")`
  is the shape that bites. A refusals case therefore routes the whole verdict through one
  helper that returns a `string` — `"= " . …` on the answering path, `"! " . $e->message`
  in the `catch` — and echoes that; which is also what lets the verdicts be *compared* to
  each other rather than read off the block, since the same message pinned once can then
  be counted over a whole corpus.
- **`gaps.py --coverage` counts cases per member, and a member sitting at 2 may already be
  at its bound.** A handoff derived `Core\Bytes::at`'s slice from that count — "the last
  in-range index and the first out-of-range one, at both ends and over an empty receiver" —
  and every clause of it was already in
  `bytes-reads-name-the-octet-they-stop-at.nvst`, both ends of `int` itself included, so
  there was nothing to write. The count is a proxy for depth and a case that asks five
  things about one member scores as one. `grep -rn "Class::member" tests/conformance/` over
  the item before writing costs one call; a duplicate case costs a session and then has to
  be told apart from the real one forever after. If the item is already answered, say so in
  the handoff and take the next one.
- **A `for` header takes expressions only, so a swept counter is a `while` with the
  counter declared above it** — `for (uint $w = 0; $w < 9; $w = $w + 1)` is six parse
  errors pointing at the `uint`. Declared once at file scope it is the function's, so a
  nested sweep resets it (`$w = 0;`) at the top of the outer body rather than redeclaring.
  In the same family: incrementing a `uint` needs a `uint` to add, so the case declares
  `uint $one = 1;` and writes `$w = $w + $one` — a bare `1` is an `int` with no
  representable common type.
- **A `--EXPECT--` block written by hand gets *precomposed* accented letters and the case
  emits *decomposed* ones**, which render identically in every terminal and in the diff
  the runner prints — so the failure reads as "expected X, actual X" and looks like a
  line-ending bug. Any case whose subject is ADR 0009 § 2's grapheme unit has combining
  marks in its output; build the block from the binary's own bytes rather than by typing
  it.
- **`Core\Str::slice`'s third argument is a *length*, not an end offset, and getting it
  wrong still counts plausibly.** A sweep walking an alphabet one symbol at a time reaches
  for `Core\Str::slice($alphabet, $i, $i + 1)` — which is "from `$i`, take `$i + 1`
  characters", so row 16 hands the decoder a seventeen-character operand. The bug is
  invisible in a *counted* assertion: the `4 of 64` a canonical-bits bound expects still
  came out `4`, and only the accepted symbols echoed beside it (`AIJKLMNOPQ…` where `AQgw`
  was meant) said anything was wrong. So a counting sweep should echo the *set* it counted
  as well as the count, and the spelling is `Core\Str::slice($s, $i, 1)`. The existing
  `Core\Str::slice("YWJjZAYW", 0, $k)` reads as either semantics, which is why it is not
  the place to check.
- **`gaps.py` ranks a class by *cases per member*, and a sweep case is one case however many
  members it asserts.** A handoff group derived from that rank can name work that is already
  done: `Core\Random` ranked thin at 7 cases over 7 members, but
  `random-every-draw-is-swept-for-its-invariants` already counted `float`'s half-open `[0, 1)`
  over 120 draws and `shuffle`'s multiset over `[1, 1, 2, 3, 3]`, and
  `random-draws-over-an-array` already pinned `pick` on both degenerate subjects. So before
  writing a ranked member's case, read the *bodies* of the class's existing cases and not only
  their `--TEST--` lines — a sweep names the members it covers nowhere else. What was actually
  left in that class was found by reading the member doc comments for a rule with no case:
  a divergence from PHP that every existing case's *list* subject cannot observe.
- **A nesting sweep is spelled `array<array<array<mixed>>>`, and `flatten` cannot be
  iterated in a loop.** `Core\Arr::flatten($x)` over an `array<mixed> $x` is `E0401`
  — its parameter is `array<array<T>>`, so the flat answer it converges on no longer
  satisfies it and the fixed point is a *type* boundary rather than a catchable
  throw. A "keep flattening until nothing changes" loop therefore has no spelling at
  all, and the invariance has to be written as one step over many shapes instead:
  `array<array<array<mixed>>> $rows = [...]` plus `foreach ($rows as array<array<mixed>>
  $row)` lets rows of *different* nestings be data, since `mixed` holds a scalar and an
  array alike and covariance carries an `array<array<string>>` into the same parameter.
  Two more from the same corner: `$pair["0"]` indexes such a row (an array is indexed by
  the string of the offset), and `Core\Str::countOf($json, "[") == 1` is how a case
  asserts an answer holds no nested array at all. `key_bytes`'s refusal, meanwhile, is
  `Fault::fatal` — so `Core\Arr::column`'s "an `indexBy` cell that is not an `int|string`"
  bullet is owed no case, no handler reaching it.
- **A `for` header cannot *declare* its counter**, so `for (int $i = 0; $i < $n; $i = $i + 1)` is
  twelve errors starting with `E0102: expected an expression` pointing at the `int`, and the later
  ones (`E0301` on `$i`, `E0401: expected int, found mixed`) read as if the counter were the
  problem rather than its declaration. Declare it on the line above and use a `while`:
  `int $i = 0; while ($i < $n) { … $i = $i + 1; }` — which is what an index-walked sweep over two
  parallel `array<string>`s wants anyway, since the arrays are still indexed by the *string* of the
  offset (`$hays[$i as string]`).
- **Two strings that look identical in a failed `--EXPECT--` diff can differ by a
  normalization form, and a `Core\Str` case is where that happens.** Echoing
  `Core\Str::reverse("cafe\u{301}")` prints `éfac`, the expectation written by hand in the
  editor is the *composed* `é` (`c3 a9`), and the runner's diff shows two lines that are
  character-for-character the same while the case fails. `od -c` on the actual output
  against the `--EXPECT--` block is the only way to see it. The fix is not to widen the
  expectation but to stop echoing the cluster: assert it against a source-escaped literal
  (`Core\Str::reverse("cafe\u{301}") == "e\u{301}fac" ? "1" : "0"`) and echo the composed
  spelling of the same row instead, so every byte in the expect block is one you typed.
- **`--EXPECT-ERROR--`/`--EXPECTF-ERROR--` means "this run must fail", so a program that exits 0
  has no way to state what it wrote to standard error.** `nvs_test::Case::expects_failure` is
  literally `self.expect_error.is_some()` (`crates/nvs-test/src/case.rs:127`), and
  `crates/nvs-test/src/run.rs:145` reports `expected the run to fail, and it succeeded` before it
  ever compares the stderr you wrote. This bites exactly the member that needs it —
  `Core\Debug::dump` writes nowhere but the diagnostic channel. The way through, and what
  `a-dump-is-its-arguments-rendered-and-a-capture-never-swallows-one.nvst` does, is to end the
  program with a deliberate `throw new RuntimeError(...)`, match the dumps byte for byte and
  absorb the fatal report with a trailing `%A` under `--EXPECTF-ERROR--`. 163 cases carry an error
  section and many of them also carry `--EXPECT--`, so there is no discriminator to relax the rule
  with; a section meaning "stderr of a run that succeeded" would be a new one.
- **A `.nvst` case cannot hold `Core` instances in an `array<mixed>`** — `$one as Core\Uri`
  on an element is `E0711` ("ADR 0007 § 2 tabulates no conversion into an object"), so a
  sweep that builds many objects and then asks one question of each has to collect the
  *answers*, not the objects: `$built[] = $u->toString();` and assert over the text. Two
  other spellings cost time in the same session and are worth having together: a `for`
  header cannot declare a typed local (`for (uint $i = 0; …)` is `E0102` at the type name,
  so declare it above and use a `while`), and `Core\Uri::with` takes one options bag, so
  it is `$u->with({scheme: "ftp"})` — `$u->with(scheme: "ftp")` names no parameter of it
  and is `E0486`, the bag itself being callable only as `options:` (ADR 0063 R2).
- **A `--EXPECT--` block cannot tell a composed `é` from a decomposed one, and the failure
  prints as two identical-looking blocks.** A case over `Core\Str` that echoes a subject built
  with `\u{301}` (or any combining mark) will fail against an expectation typed as the composed
  character, and the runner's *expected* and *actual* render byte-for-byte alike in the terminal,
  so the diff says nothing. Assert a decomposed cluster by **counting** instead — the sum of
  `Core\Str::length` over the pieces against the subject's, which is the cluster property the case
  wanted anyway — and keep the eyeball line on a subject whose characters have one spelling. The
  neighbouring bullet about a trailing space before a `\n` is the same family of invisible
  mismatch.
- **A sweep over `Core\Json::decodeAs<T>` needs one helper per `T`, not one helper.** The class is
  written at the call site (ADR 0063 R4, and `WRITTEN_CLASS_MEMBERS` hands the helper a `ClassDesc`
  ahead of the declared parameters), so there is no parameter a probe could carry the class in and
  no way to factor six codec types into one `public static function`. Write the six, each a
  `try { … return true; } catch (Throwable $bad) { return false; }` over the same document builder,
  and sweep the *documents* instead — that is what
  `json-decode-as-admits-exactly-its-declared-type.nvst` does. In the same family: `Core\Str` has
  `padEnd`/`padStart` and no `padRight`, and no `concat` at all — `.` is the concatenation.
- **`Core\ObjectSet`'s `union`, `intersect` and `diff` answer a set no declared type
  accepts, so a derived set can only be *chained*.** Their rows are
  `return_ty: CoreTy::Instance(NAME)` with no type argument
  (`crates/nvs-stdlib/src/objset.rs:89`), so `Core\ObjectSet<Tag> $u = $a->union($b);`
  is `E0401: expected Core\ObjectSet<Tag>, found Core\ObjectSet` and the bare spelling
  `Core\ObjectSet $u` is `E0442: takes 1 type argument(s), not 0` — there is no third
  spelling, and passing the answer to a case's own `public static function` helper fails
  the same `E0401`. What *does* take it is a `Core` member's own parameter, whose type is
  the same bare `Instance`, so every law over a derived set is written as one chain
  (`$a->diff($b)->union($a->intersect($b))->diff($a)->count() == 0`), and a "these two
  sets are equal" check is that chain in both directions rather than a named helper.
- **`Core\Uri::parse` refuses a stray `%`, so `normalized`'s malformed-escape branch is
  owed no case.** The module doc says "two references that both wrote the same stray `%`
  are still the same reference", which reads as behaviour a `compareTo` case can pin —
  but `parse` throws on `http://h/50%` at § 4.1 ("a `%` that does not begin a `%XX`
  escape"), so no program can build such a `Uri` at all and the branch is defensive
  against a `Uri` built some other way. Same family as the `csv.rs:512` bullet: check
  what `parse` admits before writing a row about what a normalizer keeps. In the same
  member, the seven component readers are **methods** (`$uri->path()`), and only five
  of them can hold a percent-escape — a scheme is `ALPHA *( ALPHA / DIGIT / "+" / "-" /
  "." )` and a port is digits — which is what a denominator in a swept `Core\Uri` case
  has to say out loud.
- **`gaps.py`'s depth number counts cases per member, not questions per member, and at the top of
  its table those have come apart.** A handoff item derived from that ranking can name an "unasked
  half" that is already a landed case: this session's own group opened with `Core\Bytes::startsWith`
  / `endsWith` and *"the empty and the over-long needle, counted over a sweep"*, which is
  `bytes-both-predicates-hold-the-same-bound-on-the-needles-length.nvst` verbatim, landed 47 commits
  earlier and still showing as "3 cases". Checking cost one `grep -rl 'Core\\X::member' tests/` plus
  one `cat`, and the same check then disqualified the thinnest members of `Core\Test`, `Core\Random`,
  `Core\Time` and `Core\Regex` in turn — `quote` at 2 cases already has a full escape-set sweep *and*
  a `preg_quote` differential. So read the candidate member's existing case *bodies* before writing,
  and prefer a question no existing case's `--TEST--` line states over a member with a low count.
- **A mechanical sweep over the `.nvst` corpus has to treat each `--SECTION--` as its own program.**
  ADR 0109's migration moved 55 `for` counters into their headers across 29 files, and the one rule
  that decides a case correctly is "is this counter read outside its own loop" — which is a question
  about *one section*. A whole-file answer keeps a differential case's Novis half in the old spelling
  because its **PHP** `--ORACLE--` twin mentions `$i`, and it would equally license an edit in one
  section on the strength of a use in another. Split on `^--[A-Z]` and scope both the backward search
  for the declaration and the "used elsewhere" check to the enclosing section. Two more things the same
  sweep turned up: a counter declared in a *stacked run* of declarations (`int $start = 0; int $len =
  0;` above two nested loops) is the commonest shape and is missed by a "previous line only" rule, and
  a nested header's declaration is re-entered once per outer iteration with no complaint, ADR 0109 § 2
  making it function-scoped either way. And `core.autocrlf=true` here means every rewritten file draws
  a loud `CRLF will be replaced by LF` warning from git that says nothing about your edit — read
  `git diff --stat`, not the warnings.
- **An interface method needs `public` in a `.nvst` case but not in a `nvs-types` unit fixture**, so
  a shape checked green by `crates/nvs-types/tests/common`'s `check_src` can still fail the case
  runner with `E0122` ("a method must declare `public`, `protected` or `private`"). `check_src`
  runs `parse` → `resolve` → `check_program` and nothing else; the casing/visibility pass
  `nvs-cli`'s `front_end` runs is not in it. So `interface Labelled { function label(): string; }`
  is a fine unit fixture and a broken case file, and the fix is one keyword rather than a hunt.
- **A nested `instanceof` guard *replaces* the residue rather than intersecting with it.** There is no
  intersection type, so inside `if ($v instanceof Labelled) { if ($v instanceof Counted) { ... } }` the
  subject is a `Counted` and nothing else, and `$v->label()` there is `E0405: \`Counted\` has no method
  named \`label\``, pointing at the *inner* interface for a member the outer guard proved. Read what the
  outer guard bought into a local before writing the second test —
  `tests/conformance/lang/an-instanceof-guard-narrows-to-an-interface.nvst` writes it that way and says
  so in the case.
- **Two sort keys that look different usually agree, and a case that does not separate them pins
  nothing.** ADR 0061 § 3's `implementors` sorts by `QName::segments()`, and the obvious
  counter-example to a rendered-string sort — a deeper name against a shallower sibling, `App\Sub\A`
  against `App\Beta` — orders the same way under *both* keys, because the first byte that differs
  falls inside a segment either way. The two part only where one segment is a proper **prefix** of
  the other and the longer one's next byte is below `\` (0x5C), which every upper-case letter and
  every digit is: `App\Sub\A` against `App\SubA` is the shortest such pair. Work the divergence out
  on paper before writing the fixture, because a case that does not contain one passes against
  either implementation and reads exactly like a case that does.
- **`use Core;` does not place a bare `#[Command]`, and the failure reads as if the roster edit did
  not land.** A `use` aliases one *name*, so the import that lets a file write `#[Command]` is
  `use Core\Command;` exactly as `#[Test]`'s is `use Core\Test;`; with only `use Core;` in scope the
  attribute resolves to `\Command` and the answer is `E0303: `Command` is not declared` — the
  ordinary undeclared-name refusal, which looks nothing like "this is not on the recognized roster"
  and points at the wrong file. Fully qualified (`#[\Core\Command(...)]`) needs no import and is the
  spelling to reach for when a case is about the match rather than about the import.
- **A new compile-time refusal can break a green `.nvst` written for the *runtime* half of the
  same ADR sentence, and the case will read as if it were always wrong.** ADR 0102 § 6 says two
  things in one breath — a non-capture `$params` key becomes the link's query string, and a key
  that is neither a capture nor a declared `#[Query]` parameter is a compile error. The previous
  session landed the first half and wrote
  `tests/conformance/core/router-url-turns-a-leftover-params-key-into-a-query-string.nvst` for it,
  whose whole point is `$params` keys the handler does not declare; landing the second half turned
  every one of its seven lines into `E0759` at the full verify, long after `-p nvs-types` was
  green. The fix was to declare the keys the case was already relying on, not to weaken the rule.
  Before adding a refusal, `grep -rl` the corpus for the *feature* it refuses — a case written for
  the permissive half of a two-half rule is invisible to the crate's own test run.
- **A new compiler rule that every existing fixture violates is one test-helper edit and a handful of
  line numbers, not N rewrites.** ADR 0096 § 1's "a `#[Route]` without an `#[Access]` does not compile"
  turned 19 of `crates/nvs-types/tests/routes.rs`' tests red at once, and the fixtures are all built by
  one `route_src` helper — so the decision is supplied there (`with_access`, which inserts the attribute
  ahead of every `public function`), and only the handful of tests that build a source inline had to be
  wrapped. What that leaves is the two things a helper cannot reach: a `.nvst` `--EXPECTF-ERROR--`
  section quotes **source line numbers**, so inserting a line into `--FILE--` shifts every one of them
  below it and the failure reads as a diagnostic that moved rather than as a case that grew a line; and
  a `tests/conformance/reject/` case must still fail for its *own* reason, which means the new rule has
  to be satisfied there rather than asserted. Count the inline fixtures before budgeting the rewrite —
  the helper is usually 25 of the sites and the manual ones are 5.
- **A `printf` fixture written in a Novis `"…"` string loses `%1$s` to interpolation, and the
  diagnostics blame the *variable*.** `Core\Str::format("%1$s %2$d", …)` in a double-quoted
  Novis literal is three `E0301`s about undeclared `$s` and `$d` — the template never
  reaches the checker as written, so a fixture pinning positional placeholders reads as a
  bug in the pass it is testing. Single-quote it: `'%1$s %2$d'` folds to the same
  `ConstArg::Str` and interpolates nothing. This is a real collision between two grammars
  rather than a fixture quirk, so it is also what a *program* writing positional
  placeholders has to do; `crates/nvs-types/tests/intrinsics.rs` says so at the one case
  that needs it.

- **Making a member an ADR 0057 intrinsic breaks the conformance case that pinned its runtime
  throw, and the failure names the *case*.** `str-format-refuses-every-mismatch.nvst` caught
  four `Core\Str::format` mismatches out of literal templates; the moment the checker read
  those literals, all four became compile errors and the case reported `standard output does
  not match / actual: <empty>` with four `E0769`/`E0770`s above it. The fix is § 2's own
  division rather than deleting the case: read the template out of a `string $t` variable and
  the runtime path is back, unchanged. Expect one such case per grammar as the remaining rows
  land — `grep` the throw's own message text in `tests/conformance/` before writing the arm.
- **Putting a member on ADR 0057 § 1's list breaks every conformance case that made it throw from a
  *literal*.** Landing the `Grammar::DateFormat`, `Regex` and `Uri` arms turned six green cases red at the
  full verify — `time-three-format-members-share-one-pattern-compiler`, `time-cldr-patterns-render-and-read`,
  `time-parse-refuses-a-pattern-and-a-text-in-different-classes`, both `regex-compile-*` and
  `error/a-core-member-throws-a-named-class` — each of which wrote its malformed pattern inline and caught
  the throw. They are not wrong; they are on the other side of § 2's division, so the repair is one line
  per site: bind the pattern to a `string $name = "…";` and pass that. Grep for the member's spelling in
  `tests/conformance/` **before** adding the arm and the whole set is visible in one call — the checker
  reports at most one file at a time, so discovering them from the failure log costs a verify run each.
  The same reasoning applies in reverse to a case that must *stay* literal: `Core\Time\Date::format` is not
  on the list, so its inline malformed pattern still throws, and a case can assert both halves side by side.
- **A case can *look* like it exercises ADR 0057's fold and exercise nothing, because § 1's list names the
  member that reads the pattern and not its siblings.** `examples/intrinsics.nvs` demonstrated the regex row
  with `Core\Regex::matches("order-4711", "^order-\\d+$")` under a comment naming `Core\Regex::compile`;
  `matches` takes a `Pattern|string` and is *not* on the list, so the literal was never read and the line
  asserted only that two runtime calls agree. The spelling that folds is `Core\Regex::compile("…")`, whose
  handle `matches` then takes. Check such a line against the `INTRINSICS` table
  (`crates/nvs-types/src/intrinsics.rs:107`) rather than against the argument's shape, and prove it the cheap
  way: the same text with a deliberate error is an `E0769` from `nvs check`, or the row is not being read.
- **A fixture that needs a route table does not need an `autoload` root.** The table is collected over
  every declaration the program checks, so a class declared in the entry file itself lands in it —
  `crates/nvs-cli/tests/fixtures/api/base.nvs` is one file with a class and an `echo`, and
  `nvs build --openapi` emits both its operations. `examples/routes.nvs` splits across a root because it
  is demonstrating ADR 0061 § 5's scan, not because an emitter fixture has to. Three near-identical
  fixtures are then three files rather than six, and they read as a diff of each other.
- **A test helper that names a file in `CARGO_TARGET_TMPDIR` after its *input* races the other
  tests that ask for the same input.** `crates/nvs-cli/tests/openapi.rs`'s `document(stem)` wrote
  `{stem}.json`, three of its tests ask for `base`, and `cargo test` runs them on their own threads:
  a reader that catches another thread's `fs::write` half-done sees a truncated document, `nvs api
  diff` reports no change, and `an_api_diff_classifies_a_removed_route_as_breaking` fails with a
  message that reads like a diff bug. It reproduces roughly one run in three and never under
  `cargo test -- <one test name>`, which is the trap: the obvious triage — stash the change, run the
  named test, watch it pass — points at your own diff. Run the **whole** test binary on the stashed
  tree before believing that. The fix is a per-call counter in the file name, not a lock.
- **The exception tree has no `Core\Error`, and naming one in a `catch` is an ICE rather than a
  diagnostic.** `nvs_hir::errors::TREE`'s roots are `Throwable`, `LogicError` and `RuntimeError` —
  `errors.rs`' own module doc says there is deliberately no `Error` and no `Exception` — but
  `Core\Error` *is* a `nvs_stdlib::registry` class (ADR 0063's *Amends* line adds it beside
  `Core\Path`, `Core\Out` and `Core\Bytes`), so `catch (Core\Error $e) { … $e->message … }` resolves
  the class, finds no member, and panics in `nvs-ir`: *"an instance method call at 0:228..241 has no
  resolved target recorded in the typed-expression table"*. The spelling a case wants is
  `catch (RuntimeError $e)`, and **`message` is a property, not a method** — `$e->message()` is
  `E0405` with a help line naming all four (`message`, `previous`, `backtrace`, `location`). Two
  wrong guesses in a row cost two runs; the diagnostic for the second one is excellent and there is
  none at all for the first.
- **An `array` key that is `bytes` panics in `nvs-ir` instead of being diagnosed**, the same
  shape as the `catch (Core\Error $e)` bullet above: `bytes $k = "a" as bytes; $pairs[$k] = "z";`
  reaches `lower/expr.rs:1972`'s *"an array key lowered to Bytes — nvs_types::check_program is
  trusted to have already rejected a float/bool/null key (ADR 0007 § 5)"*. The assertion names
  the pass that should have refused it, so the fix is a `nvs-types` diagnostic beside the
  float/bool/null one. Worth knowing when a probe asks "can this key be invalid UTF-8": the
  answer is no, but the reason is an ICE rather than a refusal.
- **The `unreachable from source` phrase must be within 8 lines of the `Fault::` line *and* the
  gate counts from the line the `Fault::` sits on, not from the statement it belongs to.** Two
  declarations written this session were refused after they were written: a seven-line comment above
  `uri.rs`'s `scalar_text` guard put the phrase on the line one *outside*
  `DECLARATION_WINDOW`, and a five-line one above `hash.rs`'s `finish` chunk guard was pushed out by
  the four-line `.as_bytes().or_else(…).ok_or_else(|| {` chain between the comment and the
  `Fault::fatal`. The gate's own message says "within the 8 lines above the site" and is exactly
  right; what is easy to miss is that a *multi-line* comment only counts through the line the phrase
  is on. Put the phrase on the comment's **last** couple of lines when the guard is a builder chain,
  or keep the whole comment to five lines — and re-run
  `cargo test -p nvs-stdlib --test conformance_coverage every_error_path` after writing a batch,
  which is where both were caught.
- **A `Fault::fatal` that a case can reach is pinned with `--EXPECT-ERROR--`, and the program's
  own `try`/`catch` around it is worth writing anyway.** `FATAL:` goes to standard error and the
  process stops there, so the case's `--EXPECT--` holds only what was printed *before* it and
  every row of the case has to come first — a fatal in the middle silently truncates the rows
  under it and the diff reads as "the member stopped answering". Writing the fatal call inside a
  `try` with an `echo "unreachable"` in the `catch` is what makes the *uncatchable* half an
  assertion rather than a claim, since a fatal that ever became catchable would print that line.
  `tests/conformance/core/test-an-object-comparison-names-its-depth-bound-and-refuses-a-compare-to-that-answers-no-int.nvst`
  is the shape, and `tests/differential/core/arr-count-by-refuses-what-array_count_values-warns-and-skips.nvst`
  was the only other case in either suite using the section.

- **A stem that has to reach `conformance_coverage.rs`'s corpus needs `Core\Test` spelled with
  one backslash, which a Novis string literal will not give you.** The gate reads every case
  file as raw text and looks for the site's message stem, so the stem has to appear literally.
  An `--EXPECT--` line is plain text and carries it; a `"Core\\Test::…"` written in the program
  would land as two backslashes and match nothing. Echo the message (or `Core\Str::slice` of its
  opening, when the message carries a 64-segment path) and let the expectation hold the stem.
- **`Core\Attributes` retrieval reads a *recognized* attribute's payload, so `#[Core\Command]` and
  `#[Core\Option]` are readable from a program even though the table itself is nominal.** ADR 0046
  §§ 4-5's retrieval is structural and does not care that a name is on `nvs_types::derive::ATTRIBUTES`:
  `Core\Attributes::get<{name: string, about: string}>(Deploy::deploy(...))` answers a `#[Core\Command]`'s
  payload, and `get<{about: string}>(Deploy::deploy(...), "dryRun")` answers that parameter's
  `#[Core\Option]`. Two limits come with it, found the same way. `get<T>` answers only the **first** of
  two `#[Command]`s on one method, so an alias is read with `all<T>` and indexed (`$rows[1]->name`
  works). And a payload-*less* `#[Option]` is indistinguishable from a parameter carrying no attribute
  at all — there is nothing for a shape to match — which is exactly why ADR 0086 § 6's table is built
  off the nominal roster rather than out of this pass.
  `tests/conformance/core/a-command-table-answers-its-own-help.nvst` is the worked example.

- **A new case can fail a test in a file you never opened, and the message is the fix.** Asking a member
  a second question can take it to item 10's floor of three, and
  `every_core_class_has_a_conformance_floor_of_three` refuses a `BELOW_THE_FLOOR` line that has been
  reached — "delete these lines" naming the member. It cost a whole `verify.py` cycle to learn, and the
  cheap habit is to check the case's members against `crates/nvs-stdlib/tests/conformance_coverage.rs`'s
  two rosters *before* the run rather than after it. The ratchet is working when this happens, not
  broken.
- **An `--ORACLE--` helper named after a PHP built-in is a fatal, and the runner reports it as a
  *case* failure.** A `function pos($v)` in the oracle half — the obvious name for "render a
  `?uint` position" — is `Cannot redeclare function pos()`, because `pos()` is `current()`'s alias
  and PHP has ~1,900 of these in the global namespace. The runner prints `PHP exited 255` plus the
  stderr, which is legible once you read it and reads like a case bug for the first few seconds.
  The cheap habit is to prefix every oracle helper with `php` (`phpAfter`, `phpLines`, `phpSort`) as
  the existing `str-before-and-after` and `arr-*` cases already do, and to keep the un-prefixed
  spellings for the Novis side, where a `Show::`/`Render::` class can never collide with anything.

- **A bare array literal in argument position is `array<mixed>` and does not narrow, so a `Core\Arr`
  member that takes a second array needs a typed local.** `Core\Arr::diff($a, [4, 2])` is
  `E0401: expected array<int>, found array<mixed>` at the literal, and so are `replaceRange`'s
  replacement, `appendAll`'s operand and `fromKeysAndValues`' two arguments — five sites in one file,
  all reported at once. `array<int> $against = [4, 2];` on the line above fixes each. The subject
  argument never has this problem because it comes from a declared variable, which is why the shape
  only bites on the members taking *two* arrays.
- **A `foreach` binding is a declaration in the enclosing scope, so one `.nvst` case cannot sweep two
  differently-typed corpora under the same variable name.** `foreach ($reals as float $n)` followed
  later by `foreach ($whole as int $n)` is `E0406: '$n' is already declared`, pointing at the first
  loop's header — which reads like the loop leaked its binding and is instead the ordinary
  one-declaration-per-name rule applied to a header. Repeating `foreach ($reals as float $n)` with the
  *same* type is fine, so a roster case that sweeps floats and ints alike wants one name per type
  (`$n` for the float rows, `$i` for the integer ones) rather than one per loop. The same bites twice
  in one file when two blocks both destructure a pair into `$a`/`$b`.
- A parameter default at a **literal or union** declared type is `E0451`, so a route case writing
  `#[\Core\Query] "asc"|"desc" $order = "asc"` does not compile — `nvs_types::defaults::literal_default`
  decodes a default against the *declared* type and has no arm for `Ty::StringLiteral` or `Ty::Union`. A
  `#[Query]` with no default is simply required, and a link is not obliged to supply one, so write the
  parameter without a default until that hole is closed.
- **A member `gaps.py` calls thin can already have its obvious bound pinned, and the depth number
  does not say which *shape* is missing.** `Core\Validate::isPrintable` stood at 4 cases with
  `validate-ascii-and-printable-name-their-own-bounds.nvst` already naming both ends of both `Cc`
  runs, so a handoff item reading "the range is named on both sides" was asking for a case that
  existed. `ls tests/conformance/core/ | grep <member>` before designing is the whole check, and it
  is one call. What was genuinely missing there was the *category* the range is — `U+2028` and a
  bidi override are printable, `NEL` and `TAB` are not — which is a different case and the one that
  landed. Read the member's existing case before choosing the shape, not after writing one.
- **Widening a union of enum-case types rewrites every `E0401` that names it, and a
  `--EXPECTF-ERROR--` case has the whole list frozen in it.** `Core\StrongDigest` went from
  three cases to ten and `tests/conformance/core/hash-hmac-refuses-a-weak-digest.nvst`
  went red, because its expected first line is
  `expected \`Core\Digest::Sha256|Core\Digest::Sha384|Core\Digest::Sha512\`, found
  \`Core\Digest\`` — the accepted set is generated from the parameter's own type, so the
  refusal case is a *roster* case as much as the roster table is. `%A` covers the span and
  the caret line but never the message itself. Before adding a case to a `CoreTy::EnumCase`
  union in `nvs-stdlib`, `grep -rn "<the union's first case>" tests/conformance/` and expect
  to update every hit; the `.nvst` suite is the only leg that catches it, and it catches it
  as a diff of two very long lines.
- **`Core\Test::assertSame`'s two parameters are one `CoreTy::Var("T")` bound to argument 1, so the
  symmetric spelling `assertSame(null, $x)` does not exist.** Writing the literal first binds `T` to
  `null` and the subject is then `E0401: expected 'null', found 'mixed'` at the *second* argument, which
  reads as "this member refuses a null comparison" and is only the signature. `mixed $nothing = null;`
  then `assertSame($nothing, $subject)` is the other direction, and it is the only way to ask identity's
  symmetry through this member at all. Same cause as the `Core\Math::min` bullet above, different
  consequence: there the pair is unreachable, here the *argument order* is.
- **An inline array literal in a `foreach` is `array<mixed>`, so a typed value binding refuses.**
  `foreach ([1, 2, 3] as string $k => int $n)` is `E0401: expected 'int', found 'mixed'` pointing at the
  binding rather than at the literal, because element types are not inferred into the literal's own type;
  `array<int> $steps = [1, 2, 3];` and then looping over `$steps` compiles unchanged. The key binding is a
  separate and well-diagnosed question — always `string`, `E0723` with ADR 0007 § 5 in the help line.
- **Taking a member to the floor of three is two edits, and the second one is a
  `BELOW_THE_FLOOR` line you did not write.** `crates/nvs-stdlib/tests/conformance_coverage.rs`
  carries an explicit worklist of the members still under the floor, and
  `every_core_class_has_a_conformance_floor_of_three` fails just as loudly for a *stale* entry
  ("1 member(s) listed in `BELOW_THE_FLOOR` have reached the floor of 3") as for a member that
  is genuinely short. Nothing in the orientation pack names the list, `gaps.py` does not print
  it, and the case itself passes on its own — so it surfaces only at the full `verify.py`, one
  rebuild after the work looked finished. Grep that constant for the member's name *before*
  starting a floor-closing slice: the list is also the cheapest confirmation that the member
  you picked is the one the tree is actually waiting on, and its remaining entries are the next
  session's group.
- **A `.nvst` case's bindings are function-scoped, not block-scoped, so a second sweep cannot reuse the
  first one's names.** A `for`/`foreach` body that declares `string $subject` makes a later, entirely
  separate loop's `string $subject` an `E0406: '$subject' is already declared`, with the two lines
  pointed at as if one statement were a duplicate of the other — which reads as a copy-paste slip rather
  than as the scoping rule it is. Renaming the second sweep's bindings is the whole fix, and it is worth
  doing by hand: a blind textual `$c` → `$meta` also rewrites `$codes` and `$cells`, and the case then
  fails on names that never existed.
- **`target/debug/nvs test <one-case.nvst>` runs a single case and prints its actual stdout on a
  mismatch**, so an `--EXPECT--` is frozen by writing the case with an *empty* one, running it once and
  pasting back what it printed. That is cheaper than keeping a parallel `.nvs` scratch under
  `.agent-tmp/`, which costs writing the body twice. Two cautions: the report indents the actual output
  by four spaces, so the paste is de-indented by hand; and a value that can be empty or end in a space
  needs a delimiter around it in the `echo` (`"[", $x, "]"`), because a trailing space is invisible in
  the report and exact in the comparison.
- **A `.nvst` case's bindings are script-scoped, and that includes a `catch`'s.** A case is top-level
  statements, so `int $left = 12;` inside the second sweep collides with the `var $left` the first
  sweep declared — `E0406: '$left' is already declared`, pointing at a line eighty above — and two
  `try`/`catch` blocks both binding `$e` fail the same way even though neither binding outlives its
  block. There is no block scope to hide behind: give every binding in the file its own name. The
  second failure mode is worse than the first, because a shadowed name that *does* typecheck reads as
  a working case: `$left <=> $right` over two still-in-scope `TimeOfDay` values is
  `E0411: does not implement Comparable`, which reads as a fact about the class rather than as the
  name collision it is.
- **A `Core\Time\TimeOfDay`'s four slots are not readable as properties, and the bottom of every one
  of its fields is a *checker* refusal rather than a runtime one.** `$t->hour` is
  `E0405: `Core\Time\TimeOfDay` has no property named `hour`` even though `slots` lists all four
  (`crates/nvs-stdlib/src/time.rs:1462`), so a case that wants a field as a number reads it back out
  of `format` — `$t->format("HH") as int`, and `format("SSSSSSSSS")` for the nanosecond one, which
  `crates/nvs-stdlib/src/cldr.rs`'s `Field::Fraction` pads to nine digits. The same shape decides how
  much of a "refuses one past either end" case can exist at all: `at`'s two parameters and both its
  options are `uint`, so `at(-1, 0)` is `E0401: expected `uint`, found `int`` and never reaches the
  member. A bound named on both sides has a runtime half at the top only, and the floor is worth one
  sentence of comment saying where it is enforced instead.
- **A `.nvst` case's locals are function-scoped, not block-scoped, so a second `for` loop cannot reuse
  the first one's names.** `string $index = $row as string;` in one sweep and the same spelling in the
  next is `E0406: '$index' is already declared`, pointing at both lines — which is a good diagnostic and
  still costs a run, because a case that sweeps two tables is the ordinary shape here and every sweep
  wants to call its cursor the same thing. Name the second sweep's locals for what that sweep is
  counting (`$slot`, `$point`, `$whole`), or hoist the body into a `public static function` that owns
  the names, which is what the four-way sweeps in `time-datetime-*` do.

- **The calendar interval a `Core\Time\Date` spans is wider at both ends than the one a `DateTime`
  reaches, so a case sweeping the range ends must build the two halves differently.**
  `Core\Time\Date::at(9999, 12, 31)` and `at(-9999, 1, 1)` are both accepted, while
  `Core\Time::at(9999, 12, 31, $utc)` and `at(-9999, 1, 2, $utc)` both throw — a `DateTime` must be
  convertible to the instant its zone puts it at, and jiff's timestamp range stops inside the
  calendar's last day and its first two (`-377705023201..=253402207200` seconds). The refusal names
  `Core\Time::at()` and says the conversion overflowed, which reads like a bound on the *year* and is
  not one. Pinned by
  `tests/conformance/core/time-datetime-leap-year-is-februarys-last-day-and-the-years-length.nvst`,
  which is why that case reads the year's length from March rather than from December.
- **`as` binds tighter than arithmetic, and `false as string` is the empty string.** Two `.nvst`
  spellings that cost a run each. `$which[$i - 1 as string]` parses as `$which[$i - (1 as string)]`
  and fails `E0716: '-' has no meaning for 'string'` — an index that computes needs its own
  parentheses, `($i - 1) as string`. And a `bool` printed with `as string` renders `1` for true and
  **nothing** for false, so a column of booleans in `--EXPECT--` silently changes width and reads as
  a missing field rather than as a `false`; `$b ? "y" : "n"` is the spelling that keeps such a row
  legible. Both are invisible until the case runs, and the second one passes review.
- **A `.nvst` case configures the run it makes through `--FILE <path>--`, and a scratch probe at the
  repo root does not.** `Core\Router::urlAbsolute` reads ADR 0102 § 6's origin out of `[app] origin`
  in `./nvs.toml` — the *working directory's*, resolved by `boot_snapshot` at
  `crates/nvs-cli/src/main.rs:610` — and both `router.rs`' gap 2 and the one case that existed read
  as though that made the answering half unreachable off the command line. It is not:
  `crates/nvs-test/src/run.rs` runs every case in a private temporary directory and writes each
  `--FILE <path>--` section into it before the case, so `--FILE nvs.toml--` is the mount a CLI run
  otherwise has none of. What makes it easy to miss is that the neighbouring sections refuse with a
  sentence — `--INI--` says `nvs.toml` is not read until M6 and `--ENV--` says the environment is
  unreachable until M8 — so the format looks like it configures nothing. The trap on the other side
  is worse, because it passes: **the repo's own `nvs.toml` already sets
  `origin = "https://example.test"`**, so `target/debug/nvs run scratch.nvs` from the repo root
  answers an absolute link and proves nothing about the case, which will run without one. Probe from
  a scratch directory carrying the `nvs.toml` the case will carry.
- **`BELOW_THE_FLOOR` in `crates/nvs-stdlib/tests/conformance_coverage.rs` fails the build when you
  *fix* one of its members, and nothing points at it from the case you just wrote.** The list is a
  ratchet asserted in both directions — a member below the floor and not listed fails, and a listed
  member that has reached it fails too — so the last case of a group that lifts a member to three
  turns `cargo test -p nvs-stdlib` red with `1 member(s) listed in BELOW_THE_FLOOR have reached the
  floor of 3`. The fix is to delete the line, which is the point; the cost is a whole `verify.py`
  cycle if you meet it at the end of the group rather than at the start. `grep -n <member>
  crates/nvs-stdlib/tests/conformance_coverage.rs` before writing the cases is the cheap check, and
  a group aimed at a member `gaps.py --coverage` shows below 3 should assume it is listed.
- **An array subscript whose key is a `string|int` union is an ICE, not a diagnostic, on both the
  read and the write — and it is the type `foreach` binds over `array<string|int>`.** ADR 0007 § 5
  makes `int|string` the key type and `nvs_types::check_array_key_type` accepts the union, so
  `foreach ($keys as string|int $k) { $seen[$k] = true; }` compiles and then panics in `nvs-ir`:
  *"an array key lowered to Tagged — nvs_types::check_program is trusted to have already rejected a
  float/bool/null key"* (`crates/nvs-ir/src/lower/expr.rs:1972`, whose `lower_array_key` has arms
  for `Ty::Str`, `Ty::Int` and `Ty::Uint` and nothing for a tagged one). `$seen[$k as string]` is
  the spelling that works, and it is not a compromise: `as string` renders exactly the decimal
  spelling array normalization maps an integer key onto, so a hand-built key table agrees with the
  member's entry for entry. Same family as the `bytes` array key ICE. What makes it expensive is
  that `Core\Arr::hasKey($seen, $k)` — the *member* taking the same union — accepts it happily, so
  the failing half of a two-line idiom is the half that looks unremarkable.
- **A `--EXPECT--` is byte-exact, and a loop that echoes its separator *after* each item leaves a
  trailing space nothing shows you.** `echo $a, "/", $b, " ";` inside a `foreach` costs a run: the
  expected block cannot carry a trailing space (an editor or a hook strips one, and the diff prints
  identically on both sides), so the row loop collects into an `array<string>` and the line is
  `Core\Str::join($rows, " ")` instead. The value a fold member answers — `Core\Arr::sum`'s
  `int|float|decimal` — concatenates with `.` and `echo`es fine even where `as string` on that union
  does not, so building the row string is available wherever echoing it is.
- **A zero-width match that starts where the previous match ended is dropped, and all four of
  `Core\Regex`'s iterating members drop it together.** `replace`, `replaceWith`, `matchAll` and
  `split` are each built on the `regex` crate's `captures_iter`/`split`, whose documented rule is
  that an empty match directly following a non-empty one is not reported; PCRE reports it. So
  `Core\Regex::replace("ab", "b*", "-")` is `-a-` where `preg_replace('/b*/','-','ab')` is `-a--`,
  and `matchAll("baaac", "a*")` finds three matches where `preg_match_all` finds four. The members
  agree with *each other*, so every conformance case passes and only an oracle case sees it — which
  is why it survived eighteen `.nvst` cases over the class. Until it is fixed, a differential row
  over § 5 must not put a zero-width match after a wide one; the three cases that name this
  divergence in their comments are the ones to update when it is.
- **A bare array literal in a `foreach` header is `mixed`, and the binding's type annotation is
  what reports it.** `foreach ([0, 1, 2] as int $n)` is `E0401: expected 'int', found 'mixed'`
  pointing at `int $n` rather than at the literal, which reads as if the binding were wrong. The
  literal has no element type until something declares one, so the spelling is a typed binding on
  the line above — `array<int> $starts = [0, 1, 2];` — and then `foreach ($starts as int $n)`.
  Cost two compiles in one case; every `foreach` in the corpus goes over a named variable for
  exactly this reason.
- **`Ctx::take_pending` answers an empty string for a *throw* in a hand-built context**, so a test
  outside the compiler pipeline cannot assert on a thrown message. `nvs_runtime::Ctx::buffered()`
  has no runtime error class installed, `Thrown::message()` returns `String::new()` when its
  object is null, and the failure looks like the member said nothing rather than like the harness
  is missing a class. Assert the ADR 0002 *status* — `THROWN` against `FATAL` — which is the half
  a bare harness can see and is usually the claim anyway;
  `benches/abi-probe/tests/invariants.rs`'s `decode_on_this_stack` is the worked shape.
- **A concurrency test that is the *first* one to run wide is worth writing even where the behaviour
  is already landed, and `a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count` is the
  worked example.** Every earlier `nvs-host` test drove one or two parked tasks; that one drives
  `2 × cpus + 2` concurrent blocking calls, and it failed at 11 of 34 finished because
  `Reactor::turn` reported `0` woken and `run_until_idle` reads `0` as idle. The cause is structural
  rather than flaky: one drain takes the queued ids of *several* pokes at once, so the surplus pokes
  come back ready over an empty queue, which is the common case exactly when the pool is saturated.
  When a `run_until_idle` returns with tasks still parked, suspect the turn's "nothing woke, so
  nothing can" exit before suspecting the wake that did not arrive — and read the two conditions
  together, because the entry test and the retry test have to be the same one.

- **Two adjacent `Instant::now()` calls can return the same instant, and `Timers::publish` turns a
  deadline equal to its base into one a nanosecond later** (`timer.rs:@publish` — the `.max(NOTHING +
  1)` that keeps a real deadline out of the "nothing filed" sentinel's way). So a test that arms at
  `let start = Instant::now()` on a `Timers` built the line before, and then sweeps at exactly `start
  + margin`, is one nanosecond short of overdue and reports nothing.
  `watchdog::tests::a_second_wedge_on_a_later_deadline_reports_again` failed that way — only under a
  loaded machine, and only inside the full suite, which is the expensive kind of flake. Arm strictly
  after the base (`Instant::now() + Duration::from_millis(1)`) whenever a sweep instant is derived
  from the armed one.
- **A frozen `--EXPECTF-ERROR--` block's line numbers are read off the runner, not counted by hand.**
  `--FILE--`'s first line — the `<?nvs` — is `case.nvs:1`, so the number is the case file's own line
  minus the header, and miscounting the prose comment above the code by one is the whole failure mode.
  `target/debug/nvs test tests/conformance/core --filter <slug>` prints expected against actual with
  the real `case.nvs:NN:CC` in the actual half, so writing the block with any plausible numbers and
  running it once is cheaper and more reliable than counting. The columns are already right if the
  scratch probe under `.agent-tmp/` used the same indentation, since a column does not shift with the
  header.
- **A `nvs-host` test makes a context *fail* with `Ctx::set_pending("message")`, not with `Thrown::new`.**
  That constructor is `unsafe fn new(class: *const ClassDesc, message: &str)` and needs a descriptor you
  would have to build first; `set_pending` takes a bare message and `take_thrown` promotes it through the
  error class the context carries — so a test that wants a *named* failure installs one first
  (`ClassTable::define("Throwable", &["message", "previous", "backtrace", "location"], &[])` plus
  `set_runtime_error_class`), and without one the name is gone and `take_thrown` answers a null `Thrown`.
  The neighbouring shape is a value `nvs_runtime::graph`'s walk **refuses**: a closure there is not
  `closure::` anything, it is a class carrying the `CLOSURE_INVOKE` method row, so
  `ClassTable::define` + `set_methods` + `NvsObj::new` is the whole fixture — leak the table, because a
  descriptor's address is its identity. `crates/nvs-runtime/src/graph.rs:925` and
  `crates/nvs-host/src/isolate.rs`'s `closure_value` are the two copies of it.
- **`await 5` does not parse, so a fixture that wants a badly-typed operand binds one first.**
  `await` is contextual (`docs/spec/00-overview.md` § 2) and is read as the operator only before
  what the production needs, so a bare literal after it leaves `await` read as an identifier and the
  parse fails with `E0101 expected ';'` pointing past the literal — not with anything naming
  `await`. `int $n = 5;` then `await $n` is the spelling that reaches the type checker, and
  `crates/nvs-types/tests/isolates.rs`'s `await_refuses_an_operand_no_spawn_produced` is the worked
  example. The same applies to `spawn script` before anything that is not a string expression.
- **A `.nvst` case runs in a temp working directory, so a relative path inside one resolves against
  *that*, not against the repository root.** A case that needs a second file writes it with a
  `--FILE <name>--` section — `tests/conformance/isolate/a-handle-is-collected-once-and-the-second-await-throws.nvst`
  is the worked example, and `crates/nvs-test`'s `RESERVED_NAMES` is the list of names it may not
  claim. The trap is that a case asserting a path *fails* passes either way: the first version of
  the sibling case spawned `"examples/isolate/capture.nvs"` and was green only because the file was
  missing there too.
- **`await $handle;` as a bare statement does not parse as an `await`** — the contextual keyword in
  statement position reads as a type name, so the case reports "`$handle` is already declared" at
  the second mention. Bind it: `var $ignored = await $handle;`. `spawn script …;` as a statement is
  fine, since nothing follows the construct that could start a declaration.
- **A `.nvst` case that spawns a child script writes the child into the case**, as a `--FILE
  child.nvs--` section beside `--FILE--`. The runner puts every auxiliary file into the workdir it
  then runs the case from (`crates/nvs-test/src/run.rs:99`), and `spawn script` resolves a relative
  path against the working directory (`crates/nvs-cli/src/script.rs`'s module doc), so `spawn script
  "child.nvs"` finds it — no fixture directory beside the cases, and no `../../examples/` path out
  of the suite. `--FILE <path>--` may appear any number of times and takes a forward-slash relative
  path, so a child that needs its own `require` graph is the same mechanism.
- **A new `Core` member owes `conformance_coverage.rs` three things, and `verify.py` reports them
  one gate at a time only after `cargo test` is otherwise green** — so budget for the round trips.
  A member needs (1) a case naming it, (2) **three** cases naming it, because
  `every_core_class_has_a_conformance_floor_of_three` is a floor per member and not per class, and
  (3) every `Fault::` message stem in the corpus or declared `unreachable from source` within 8
  lines of the site. The one that bites is a member whose *accepted* path a `.nvst` cannot reach —
  `Core\Test::advance` needs a `#[Test(at: ...)]` isolate, so all three of its cases ask about the
  refusal, and conventions.md's four shapes are what keeps them three different questions rather
  than one written three times. For an error path reachable only from a runner, give both refusals
  one `format!("Core\\Member(): {why}")` prefix: the gate keys a site on the literal stem before
  its first hole, so one case discharges both — and check the judgement is *true* before writing
  `unreachable from source`, which here it was only for `Core\Time::now`'s fixed-clock guard,
  both writers of that field validating before they store.
- **An object does not cross an isolate boundary today, even when both files declare the identical
  class.** A child that `return`s an instance comes back as `ok=false` with `` `Node` on the
  receiving side is a different class, so an instance of this one has no meaning there`` — two
  identical declarations in two compilation units are two classes. This is the plan's `Live::admit`
  gap (`crates/nvs-runtime/src/graph.rs` § *Known gaps*) seen from the return-value side, and it cost
  a whole case its second half: `a-graph-copy-round-trips-a-cyclic-value.nvst` was written with the
  arena-to-arena carrier alongside the byte one and had to be rewritten around bytes alone. Until it
  is closed, a cross-boundary case carries a scalar or an array, never an instance.
- **Three front-end spellings each cost a run while writing `.nvst` cases, and none of them is what
  PHP would have you write.** The constructor is `constructor`, not `__construct` (E0114, and the
  follow-on E0409 for every property it would have assigned). `var $x = [1, 2, 3];` is E0414 — an
  array literal has no target type to infer from, so it is `array<int> $x = [1, 2, 3];`. Two `catch`
  clauses in one scope may share a variable name while their classes match and are E0406 the moment
  they differ, so a case with a `TimeoutError` arm and a `LogicError` arm needs two names.
- **A `Qual` classification is enforced on the `tainted` axis only, so a `secret` claim still has to be
  probed before a case is written against it.** `nvs_types::expr::quals::admits_tainted_argument` reads
  the row's mark and lets a tainted argument through a `Contagious`, `Neutral` or `Launder` parameter;
  nothing reads it for `secret`, which is refused at all four marks exactly as an unclassified parameter
  refuses it, and that refusal is deliberate rather than pending. The probe is one file and one call —
  a scratch `.nvs` under `.agent-tmp/`, run with `./target/debug/nvs.exe run <path>`, which prints every
  diagnostic the case would otherwise have had to predict, and prints them for *every* line at once so
  one probe answers a whole group of members.
- **`as array<SomeClass>` is refused with `E0711`, and the diagnostic's own help is the fix: convert to
  `array<mixed>` and convert each element where it is read.** ADR 0007 § 2's `array<T> as array<U>` row
  checks every element against `U` as it walks, and what checks one element is its runtime tag — which a
  class is not decided by. `as array<int>` is fine for exactly that reason, so the refusal only shows up
  once a case rounds a container of *objects* through something answering `mixed`, which
  `Core\Serialize::decode` is. The spellings that compile are `... as array<mixed>` and then `$copy[0] as
  Cell`, and a nested one parenthesizes: `($outer[0] as array<mixed>)[0] as Cell`.
- **A `.nvst` case can carry its own `nvs.toml`, so a change to a configuration spelling breaks tests
  that no `grep` over `crates/` will show you.** `--FILE nvs.toml--` writes one into the case's working
  directory, and four `router-url-absolute-*.nvst` cases do exactly that — one of them pinning the
  diagnostic text that names the key, in `--EXPECT--`. Renaming `[app]` to `[[app]]` was green in
  `cargo build` and `cargo test` and cost a whole `verify.py` cycle at the `conformance` step. When you
  move a spelling that appears in a config file or in a diagnostic, `grep -rl` over `tests/` and
  `examples/` in the same call as `crates/`, and remember the message text is pinned in two places: the
  `Fault` in the crate and the `--EXPECT--` of the case that catches it.
- **A relative path in an *included* configuration file resolves against that file's own directory**
  (ADR 0103 § 5), and an `[[app]] root` is the one where it does not look like a path at all. A case
  writing `root = "srv/www/shop"` into `conf.d/shop.toml` is keyed on `conf.d/srv/www/shop`, and the
  refusal it gets is `E0605 cannot read` — which reads as a broken fixture rather than as § 5 doing
  exactly what it says. Write the `..` the operator would have to write.
- **A `--FILE nvs.toml--` section is now read by the real configuration reader, so an invalid one
  fails the whole case before the program starts.** The bullet above is still true about *where* it
  is mounted; what changed is who parses it. Three router-origin cases had been written against the
  line scanner that preceded `boot_snapshot` and held `[[app]]` blocks with no `root` or `entry`, a
  root-level `origin`, an `[server] origin` and an `[app.dev]` sub-table — every one of which ADR
  0064 § 3's typed tree refuses outright. The failure is `E0601`/`E0609` on stderr and an *empty*
  stdout, so it reads as the program having produced nothing rather than as a fixture problem. Two
  further rules a fixture has to keep: every `[[app]]` needs `root` or `entry` (§ 1 keys a block on
  an entry file path), and two blocks may not carry the same key (§ 2 has no order between them, so
  it refuses rather than picking). `entry = "nvs.toml"` is a legal narrower key that matches no
  program, which is the shape a "this block must not apply" decoy wants.
- **A path rule cannot be tested against a real symlink on Windows, and `nvs-config`'s `Files` trait
  is the seam that makes that a non-problem.** Creating one needs a privilege CI does not have, so
  `a_path_reaching_a_granted_root_through_dotdot_or_a_symlink_does_not_match` splits: the `..` half
  runs against `resolve::Disk` and the real filesystem, because a canonicalizer resolving `..`
  textually rather than by asking the OS is the exact bug the rule prevents and a fake would hide
  it, and the symlink half runs against a seven-method fake whose `canonical` maps one path to
  another. The fake asserts the same thing the symlink would — that the comparison is against the
  canonicalizer's *answer*. `crates/nvs-stdlib/tests/capability.rs`'s `Fake` is the shape; six of
  its methods are `unreachable!()` with a sentence saying why that call would be a bug.
- **A `.nvst` case can carry its own `nvs.toml`, and once a capability guards a construct it has
  to.** ADR 0103 § 1 step 2 finds the configuration at `./nvs.toml` in the working directory, and
  the multi-file form writes files into the case's own directory — so `--FILE nvs.toml--` with
  `[capabilities.script] spawn = true` is how a case that spawns keeps working under ADR 0118's
  deny-by-default. Putting the check inside `nvs_runtime::script::resolve` broke four cases at once
  (three under `tests/conformance/isolate/`, one under `task/`) and one `-p nvs-cli` unit test, and
  every one of them reported the *denial* rather than anything about the case, which reads as
  "spawn is broken". The inverse is what makes a denial assertable: a case with **no** `nvs.toml`
  grants nothing, which is what `tests/conformance/core/file-*.nvst` rests on, so the harness must
  never grant anything by default.
- **`python tools/try.py` does not reproduce a multi-file case's working directory**, so it reports
  a spurious failure for one that reads a sibling file — it copies the `--FILE--` body to
  `.agent-tmp/` and runs that alone. Check a multi-file case with `target/debug/nvs test
  tests/conformance/<tree>` instead, which is what `verify.py` runs; `try.py` is for the
  single-file shape it was built for.
- **A `.nvst` case can ship its own `nvs.toml`, and `Core\Config::restore` inside one can end the
  case with a `FATAL`.** `--FILE nvs.toml--` writes the file `nvs run` discovers in the working
  directory — with no `--config`, step 2 is `./nvs.toml` — so a case can state `[limits]` and
  `[limits.hard]` and then move its own ceiling, which is the only way a case reaches the runtime's
  cached memory limit at all. Two edges cost a run each. `restore` puts the file's smaller value back
  **while what the raised ceiling permitted is still live**, and the next helper call breaches it — so
  a case that raises, allocates, then restores has to drop the ballast first, and that ordering is the
  feature rather than the fixture (a request cannot un-allocate by lowering its own limit). And a case
  may carry `--EXPECT--` *and* `--EXPECTF-ERROR--` together: `crates/nvs-test/src/run.rs:147` asks only
  that a case stating an error expectation exits nonzero, so one case can pin the stdout printed before
  a `FATAL` and the `FATAL` line itself — which is what makes a refused ceiling provable in the runtime
  and not only in `Core\Config::get`.
- **A closure captures by value, so a `.nvst` case cannot count anything by incrementing a captured
  variable.** `var $seen = 0; var $f = fn (): void => { $seen = $seen + 1; };` compiles, runs, and
  leaves `$seen` at `0` however many times `$f()` is called — the case fails on a line that looks
  like a bug in the member under test rather than in the fixture. Whatever the closure has to report,
  it has to `echo` from inside itself, or the case has to count on the outside. The counting shape
  conventions.md recommends still works; it just cannot run its counter through a capture.
- **A `loop-goal.toml` check naming a crate with no `tests/` directory is not the wrong-`args` trap
  next to this one.** Stage 4's `-p nvs-host` block names seven tests and `crates/nvs-host` had no
  integration-test directory at all, which reads like the check was filed against the wrong crate —
  it was not. A package's ordinary `[dependencies]` are on the extern list of its test targets too,
  so a new `crates/nvs-host/tests/limits.rs` can `use nvs_runtime::{Ctx, nvs_safepoint}` and drive
  the ABI compiled code calls with no compiler and no `dev-dependencies` edit. The question that
  separates the two cases is *can this crate reach the thing the test asks about*, never *does a
  test like it already live here*: `nvs-hir` genuinely could not ask `required() > 0`, and `nvs-host`
  can reach every seam a request's limits are enforced at.
- **A `[limits]` reader answers off `Snapshot::table`, the raw `toml::Table`, and not off the typed
  `Config` beside it.** `Ctx::configured_memory_limit` and `::configured_cpu_time` go through
  `nvs_config::Request::get`, which reads `self.base.table` — so a case that builds
  `Snapshot { config: Config { limits: Some(..), .. }, ..Default::default() }` compiles, runs, and
  reports *no ceiling*, because the half it filled in is a different reader's. Build both halves from
  one TOML string (`written.parse::<toml::Table>()`, then `table.clone().try_into()`), which is what
  `crates/nvs-runtime/tests/configured_limits.rs` does; that also needs `toml` as a dev-dependency of
  the crate under test.
- **A `static` a test handler records into is shared by every test in that binary, and cargo runs
  them on threads of their own.** `crates/nvs-host/tests/limits.rs`'s `SEEN_LIMIT` was documented as
  "one test uses it, so nothing here has to survive another running beside it" — which stopped being
  true the moment a second case registered a handler writing to it, and the failure was a `None`
  where `Some("max_script_depth")` was expected, in a case that passed on its own under
  `--test-threads=1`. A `take()` from either reader empties it for the other, so the fix is a slot
  and a recorder **per test**, not a lock held longer: compiled code is called through a bare
  `extern "C"` pointer, which captures nothing, so "the same handler aimed at another slot" is not
  expressible and the recorder has to be copied. The counter at the top of that file is read as a
  *difference* for exactly this reason, and that note is the one that should have been read first.
- **A capability is invisible through the request overlay, so a case that "narrows one" through
  `Core\Config::set` is pinning nothing.** `nvs_runtime::capability::refusal` asks
  `config.snapshot().config.capabilities` — the typed tree — and never
  `nvs_config::Request`'s overlay, and `Request::set` refuses the `capabilities` block from either
  direction anyway: the row is `Class::RuntimeTighten` and a grant is a list, which
  `value::within_ceiling` answers `Ok(None)` for, so there is nothing to narrow *by*. A request
  therefore cannot narrow a capability at all; the narrowing in `a_child_cannot_widen_a_capability_its_parent_narrowed`
  is the operator's, in the snapshot the parent already holds, and what the case asks is the
  child's side of it. The general shape: a directive that is `RuntimeTighten` in the registry is
  not thereby settable — the value has to have a quantity for the comparison to mean anything.
- **A `cfg(unix)`-only test cannot satisfy a `cargo-named` acceptance check**, because the driver
  reads "did not run" as a failure and this loop runs on Windows. `crates/nvs-config/tests/trust.rs`
  gates its two negative cases on Unix and is right to — nothing names them — but stage 5's
  `a_world_writable_cache_directory_is_refused` *is* named, so it has to construct a world-writable
  directory on both platforms. Windows has one, and it is one line:
  `icacls <dir> /grant *S-1-1-0:(OI)(CI)(M)` grants `Everyone` modify rights, and
  `nvs_config::trust::check` sees it through `GetEffectiveRightsFromAclW`. Spell the principal as
  the SID rather than as `Everyone`: `icacls` is localized, and this machine's prints German. Then
  assert on `Untrusted::Breach` and on the directory's own last component — not on the canonical
  path, because `fs::canonicalize` yields `\\?\C:\…` on Windows where the refusal's message does
  not.
- **A path helper that normalizes `..` defangs the very escape the case was written to assert.** The
  `p()` in `crates/nvs-config/tests/request.rs` resolves `.` and `..` away so an ADR's `/a/b` spells
  itself host-natively, and reaching for the same helper in a case about a `..` escape hands
  `Capabilities::allows` a path that has already escaped — the assertion then passes on the
  sibling-root rule and never on ADR 0118 § 4's canonicalise-then-prefix. It fails no test and looks
  right in review. `crates/nvs-config/tests/capability.rs` keeps the two apart: `raw()` is what the
  caller wrote, `lexical()` is what the filesystem answers, and only the fake `Files` may turn one
  into the other.
- **An array literal is `mixed`, so `foreach (['a', 'b'] as string $path)` does not compile** — the
  binding's declared type is checked against the element type the literal *has*, and E0401 points at
  the binding rather than at the literal that produced it. Two `try` blocks written out cost nothing
  and read the same; a case that genuinely wants the loop has to name the array's type where it is
  built, not where it is walked.
- **A `--RUN-- test` case resolves no configuration tree, so `Core\Config` answers as the empty
  configuration inside every `#[Test]`.** The runner does give each test method an isolate of its own —
  which is the only thing in the CLI that runs two requests in one process — but
  `crates/nvs-cli/src/runner.rs:245` builds a bare `nvs_runtime::Ctx::stdout()`, and only `nvs run`
  resolves `./nvs.toml`. So a `--FILE nvs.toml--` beside a `--RUN-- test` case is written to disk and
  read by nobody, and `Core\Config::get` is `null` rather than the file's value — the stdlib module doc's
  "a context nobody configured answers as an empty configuration", reached from an unexpected direction.
  Two sequential `spawn script` children are the shape that does work, and they *do* see the
  configuration: `tests/conformance/config/config-set-is-invisible-to-the-next-request.nvst`.
- **A memory-limit breach is observed at the first `run_helper` member call after it, and an
  `$a[] = …` is not one.** A fixture that grows an array past `[limits] memory` and expects the
  `FATAL` there gets it at whatever `Core` member or `echo` runs *next* instead — which is how one
  session spent six probes reading a clean `FATAL` and concluding a reported abort was already
  fixed. `nvs_runtime::run_helper`'s own comment owns the rule ("the breach is therefore observed
  at the first member call after it happens"). Two consequences for authoring: put the member call
  where you want the breach reported, and expect nothing printed before it, since `run_helper`
  asks its question ahead of the body. A handler registered with `Core\Fatal::onLimit` only runs
  when the reading is still under `[limits] memory` *plus* the reserve, so a fixture that overshoots
  the ceiling by 2× sees no handler at all.
- **A diagnostic's help text is pinned byte for byte by an `--EXPECTF-ERROR--` case in a tree that
  never names the crate you edited.** Rewording `E0319`'s help in `crates/nvs-hir/src/members.rs`
  failed `tests/conformance/lang/a-bare-global-name-is-a-compile-error.nvst`, and `cargo test` does
  not run the `.nvst` trees — so the failure waits for `verify.py` or an explicit
  `nvs test tests/conformance/`, long after the edit looks finished. `grep -rn` a distinctive phrase
  of the help under `tests/` before changing it; the pin is one line of the case's expectation and
  updating it is the whole fix.
- **A `reject` case pins the *first* diagnostic, and the recovery type behind it writes the second
  one.** `check_read` answers `mixed` after reporting, so a `static` method declared `: int` whose
  body reads `$this->size` reports the refusal *and* `E0403 declares int but returns mixed`, and
  `%A` does not cover the trailing `aborting due to 2 errors` line. Two ways out, and the choice is
  about what the case is for: declare the surrounding position `mixed` so the recovery satisfies it
  (what `this-is-not-read-in-a-static-method.nvst` does, in a comment saying why), or pin both
  errors deliberately. Writing the case against one error and discovering the second costs a build.
- **A `--FILE--` body starts at byte 0 of the file the runner writes, with no leading newline — which
  is the only reason an offset-0 case can be written at all.** `crates/nvs-test/src/case.rs`'s section
  reader takes the lines *after* the header as the body verbatim, so
  `a-shebang-line-opens-code.nvst` really does hand the lexer `#!` at offset 0. Worth knowing before
  writing any case whose subject is the first bytes of a file: if the harness had kept the separator's
  newline, the case would have passed while testing nothing, because the rule it pins (ADR 0100 § 3)
  is exact about the offset and every other position is ordinary text.
- **A `namespace` in Novis is a statement and not a block, so a case that needs two of them needs two
  files.** `namespace App { … }` is `E0243`, *"write `namespace X;` once, before any declaration, and
  put a second namespace in a second file"* — which turns a one-file draft into a `--FILE app.nvs--`
  plus a `require './app.nvs';` at the top of the root file. Declarations cross a `require` (ADR 0021),
  so the classes are reachable and only the root file's statements print, which is what makes this the
  cheap shape for "the same name means two different things in two scopes". `try.py` cannot run the
  result — the neighbouring bullet says why — but `target/debug/nvs test <path>.nvst` takes a **single
  case path**, not only a tree, and reproduces its working directory.

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
- **A big file hides doc comments attached to the wrong item.** Two of `nvs-types`' were 120 lines from the
  function they described, invisible in a 3.5k-line file and obvious the moment it became eight. When a
  carve leaves a doc block stranded above an unrelated item, that is a bug the split found, not one it
  made.
- **Header prose splits with the code.** A module doc that grew a paragraph per ADR slice *is* the split
  plan: each paragraph already names the rule it belongs to. What is left in `mod.rs` afterwards is its
  charter — see AGENTS.md's length-target table for why the charter is the part that matters.

## Writing Novis itself

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
  actually links that crate.** `nvs_runtime::alloc::Pooled` is registered from `nvs-runtime`'s
  own `lib.rs`, so every binary in the workspace gets it — except one whose sources never name
  `nvs_runtime`, because rustc links an `--extern` crate lazily and an unlinked crate is not in
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
  callback does work rather than computing a value meets this. `examples/collect.nvs`'s
  `Core\Out::capture` line was written the short way and sat there uncompiled for several
  sessions, which is the next bullet's fault as much as this one's.

- **One compile error hides every later one, so "the first red fixture line" moves *backwards* as
  you fix it.** `examples/collect.nvs` was recorded in three places as failing at line 47 on a
  missing `Core\Out::capture`; the truth was a **parse** error at 47 that suppressed name
  resolution entirely, and behind it sat a `Core\Uuid::isValid` at line 29 that the spec says does
  not exist and an `Arr::first` subscript at 45 that panics `nvs-ir`. Budget a fixture as "run it
  again after every fix until it exits 0", not as "one report, one slice" — and do not trust a
  handoff's claim about which line a fixture stops at without running it.
- **`var` takes no type annotation, and writing one costs four diagnostics a line.** `var string $s = …`
  is not a declaration with a redundant type: the parser reads `var`, expects a name, finds `string`, and
  emits `E0101` twice, `E0102`, a third `E0101` and then an `E0406` claiming `$` is already declared — per
  line, so a six-line scratch file came back with 24 errors and none of them said "a `var` has no type".
  The two spellings are `var $s = …` (inferred, and what every `.nvst` case writes) and `string $s = …`
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
- **The first-class callable spelling `Class::method(...)` panics `nvs-ir` outright** — *"a static call
  has no resolved target recorded in the typed-expression table"*, which reads like a checker/lowering
  mismatch rather than a missing feature. It is the same hole as `nvs-ir` gap 1: a case cannot name one
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
  just the one holding the guard.** Touching `crates/nvs-runtime/src/lib.rs` and rebuilding
  `nvs-abi-probe` measured 200s for the package and 133s for `--test perf_guards` alone: five binaries
  at ~17s of link each, over ~116s of compiling six crates at `codegen-units = 1`. So a `--release`
  check in `loop-goal.toml` names its test *file*, and anything in the package that is not a cost
  guard gets a second, debug check — 29s for all of `nvs-abi-probe`, with the guards skipping
  themselves through `#[cfg_attr(debug_assertions, ignore)]`. Do not reach for a cheaper profile
  instead: the cost class is a claim about the profile Novis ships, so `lto`/`codegen-units` are the
  measurement and not overhead on it. What is left after narrowing is a build, and a build overlaps:
  `loop.py` starts it before the native build and runs the check last, which took a cold sweep from
  326s to 183s. Two cargos on one `target/` do not block each other — only the registry's package
  cache is briefly contended, and the debug half finishes in its usual time.
- **Measure a build with nothing else touching `target/`.** The same narrowed release build timed
  133s alone and 260s with a `du -sh target` walking the tree beside it. On a link-heavy build the
  disk is the contended resource, so a second reader of the same 1.7 GB doubles it — a timing run
  that disagrees with an earlier one by 2x is usually this and not the change under test.
- **A `?Instance` *does* narrow, at file scope, and a case reaching for a `?Match` needs no
  helper class.** `var $found = Core\Regex::match($s, $p); if ($found == null) { … } else {
  $found->groups() … }` compiles and runs, and inside the `else` the receiver is the class type,
  so `foreach ($found->groups() as string $key => ?string $value)` and `$found->groups()["0"] ==
  $found->text()` both lower. The neighbouring trap — a `?array<T>` that cannot be indexed even
  after a `!= null` guard — is specifically about a narrowed nullable *array* losing its element
  type, not about narrowing, so do not generalise it into wrapping every nullable in a
  `public static function`. Narrowing a `?string` to compare it with `""` works in the same
  place, which is what lets a case tell an absent group from one that captured nothing.
- **`int`'s own low end is not a writable literal, so a case pinning a 64-bit bound spells it
  `-9223372036854775807 - 1`.** `-9223372036854775808` is `E0429: this integer literal is too large
  for `int`; it is only legal where a `uint` is expected` — the minus is an operator applied to a
  literal that has already overflowed, exactly as in PHP and Rust, and the diagnostic names `uint`
  rather than the negation, which reads as though a `uint` would have helped. The top of the
  unsigned range needs no such trick: `18446744073709551615 as uint` is accepted. A sweep that
  pairs each code with its own bounds carries them in a parallel `array<int>` and reads it by key —
  `foreach ($codes as string $k => string $c)` binds both, and `$high[$k]` indexes the sibling
  array — since there is no arithmetic on a format string to build one from.
- **A closure held in an array can be handed to a `Core` member's `callable` option**, and
  calling one through the variable holding it lowers too (it used to panic). `array<callable>
  $filters = [fn (Core\Cli\Text $c): int => 7, ...];` then `foreach ($filters as callable
  $filter) { ... Core\Out::capture($body, {through: $filter}) ... }` lowers and runs, because the
  call is the runtime's (`nvs_runtime::call_closure`) and not a lowered `Call` — which is what
  turns a sweep over eight closures into a sweep rather than eight copies of one block. Two
  spellings to get right on the way: a **block-bodied** `fn` must declare its return type
  (`E0450`: `fn (): void => { ... }`), an expression-bodied one takes the expression's; and a
  `mixed` is **not** implicitly assignable to a narrower type, so `array<string> $row = ["a",
  $cell];` over a `mixed $cell` is `E0401` at the element.
- **An integer literal past `int` lowers in a `uint` *argument* and panics `nvs-ir` inside an
  `array<uint>` literal.** `Core\Random::bytes(9223372036854775808)` is fine — the parameter's
  declared type is what decides how the literal lowers — but `array<uint> $counts = [1,
  9223372036854775808];` dies with *"nvs-ir: integer literal `9223372036854775808` doesn't fit an
  `int`"*, an element position carrying no such expectation. So a sweep table whose rows run past
  `i64::MAX` has to *compute* them rather than write them, and the multiplier is the second half of
  the trap: `$n * 2` over a `uint` is `E0407` and then `E0401`, the literal `2` being an `int` with
  no representable common type, so the case declares `uint $two = 2;` and multiplies by that.
- **One `RuntimeSig` may name the signature two different runtime symbols are declared under, and
  changing one symbol's Rust declaration then miscompiles the other in silence.** `nvs_array_unset`
  was emitted through `RuntimeSig::ArrayAppend` because both happened to be two pointers in and one
  pointer back; the moment `nvs_array_append` grew ADR 0002's `(ctx, …, out) -> status` shape, that
  reuse would have declared `nvs_array_unset` to Cranelift with four parameters and an `i32` return
  and nothing — not the Rust compiler, not `clippy`, not a codegen test — would have said so, because
  `Linkage::Import` never checks a declaration against the definition. So before editing an
  `extern "C"` in `nvs-runtime`, grep `crates/nvs-codegen/src/emit.rs` for its `RuntimeSig::` variant
  and check whether a *second* `runtime_ref` call names it; if one does, give that symbol its own
  entry in `Signatures` first. `nvs_str_concat`/`nvs_str_append`/`nvs_str_concat_n` and the
  `nvs_throwable_*` pair are the other shared entries, and their doc comments say the sharing is
  deliberate.
- **A closure passed to a `Core` member may call a static method in its body, and `"\u{0000}"` is
  how a case writes a NUL.** Two spellings the neighbouring bullets make one doubt, both measured in
  a scratch `.nvs` and both fine: the closure trap is only about calling a closure *through the
  variable holding it*, so `fn (int $a, int $b): int => Key::magnitude($a) - Key::magnitude($b)`
  handed to `new Core\Heap<int>(...)` — and the identical body handed to `Core\Arr::sort`'s `{by}` —
  lowers and runs, which is what lets one rule be written twice and the two members asked to agree
  about it. And the code-point escape is the only way to put a NUL in a case's subject (there is no
  `\0`); `Core\Str::length("a\u{0000}b")` reads **3**, so it is a character in the string rather
  than a terminator, which is the assertion a `Core\Validate` sweep over degenerate subjects rests
  on.
- **A case that sweeps code points writes `Core\Str::fromCodePoint($c as uint)`, and orders two strings
  with `Core\Str::compare`.** The obvious spellings both fail: `Core\Str::fromCodePoints` takes
  `array<uint>` while `Core\Arr::range` answers `array<int>`, and `array<int> as array<uint>` is the
  conversion ADR 0007 § 2 still owes — so the singular member, with the loop's `int` counter converted at
  the argument, is the way a sweep names a code point at all. For ordering, the neighbouring bullet's
  `<`/`>` hole over two `string`s is real but no longer the end of it: `Core\Str::compare($a, $b) <= 0`
  is an `int` comparison, it lowers, and over fixed-width zero-padded hex it *is* the numeric comparison —
  which is what lets a case assert that a `Core\Uuid::v7` sweep never goes backwards without leaving for
  the crate's own `#[test]`.
- **`Core\Math::INT_MAX as float` throws, so a case reaching for "a huge finite float" has to
  reach for `Core\Math::FLOAT_MAX` instead.** `int as float` is one of ADR 0007 § 2's *checked*
  conversions and 2^63-1 is not representable in an `f64`, so the row lands as
  `Uncaught Exception: cannot convert `int` 9223372036854775807 to `float`` at run time with
  nothing said at compile time. `Core\Math`'s roster already has the four floats a numeric table
  wants — `FLOAT_MAX`, `FLOAT_MIN` (the smallest positive *normal*, PHP's name, not `f64::MIN`),
  `INFINITY` and `NAN` — and a derived infinity is spelled `Core\Math::FLOAT_MAX * 10.0` or
  `Core\Math::log(0.0)`, whose second `{base: …}` argument may be omitted entirely.
- **A `Core` member declared `CoreTy::Union(NUMBER)` answers `int|float|decimal`, and that union is a
  *representation* no binary operator will meet a plain `int` or `float` across.** `Core\Math::abs` is
  the one a sweep reaches for first: `Core\Math::abs($n) == Core\Math::max($n, 0 - $n)` type-checks and
  then dies at run time with *"nvs-codegen does not lower a binary operator over mismatched
  representations"*, and passing it on to a member that wants one arm — `Core\Math::isNan(Core\Math::abs($x))`
  — is `E0401: expected float, found int|float|decimal` at compile time instead. Write
  `Core\Math::abs($x) as int` / `as float` at every use; the cast is free, and which arm to write is
  decided by the argument, since the member never crosses arms. `grep -n 'CoreTy::Union' <the module>`
  says in one call which members owe the cast.
- **A counting sweep has all the shapes it needs, and none of them is the one that bites.** Two nested
  `foreach`es over the same `array<string>` compile as long as the two bindings are named differently
  (`$leftText`/`$rightText`); a `public static function of(int $n): int` in a `final class` may
  `return -1;` — a unary minus on a literal lowers, so the `0 - 1` an older case writes is not required,
  though `0 - $x` still is for a *variable*; and a `bool $flag = false;` declared **inside** a loop body
  is fine, since the `E0406` that bites is a second declaration in the *source*, not a second execution.
  So the shape of an agreement case is: typed array literals above, counters as `int` above, one
  `if (…) { $n = $n + 1; }` per claim, and one `echo` of the counts against the sweep's own total.
- **A `Core`-owned enum cannot be written as a type at all — not as a parameter, not as an
  `array<T>` element — and the diagnostic prints the same name on both sides.** `array<Core\Unit>
  $units = [Core\Unit::Day]` and `public static function step(Core\Unit $u)` are each `E0401:
  expected `Core\Unit`, found `Core\Unit``, because the registry interns the name as an enum type
  (`crates/nvs-types/src/core_lib.rs:292`) and the source-written annotation resolves to something
  else. A **user-declared** `enum Mode: int { Fast = 0, }` works in both positions, so the hole is
  specific to `Core\Unit`, `Core\Order` and their siblings. What this costs a case is the sweep
  shape: a table of units cannot be iterated, so a `.nvst` that steps by several units writes the
  case literal at each `plus`/`minus` call site inside a loop over the *other* dimension, which is
  what both `Core\Time\Date` and `Core\Time\TimeOfDay`'s agreement cases do.
- **A local's name is checked, and the diagnostic is `E0112`.** `int $words_n = …` in a case is
  *"local variable names must be camelCase, e.g. `wordsN`"*, with the rename spelled out in the
  suggestion — so a snake_case counter or table name costs a whole run of the case to learn
  something the name itself could have avoided. Reach for `$wordsN`, `$firstOrdered`,
  `$mathMinRefused` from the first draft; a `.nvst` case tends to want several near-identical names
  at once (one `catch` binding per clause, since they are all function-scoped) and that is exactly
  where the underscore creeps in.
- **ADR 0007 § 2's one implicit conversion was not implemented at all, and it reads as a
  division problem until you probe a plain assignment.** `float $x = $n;` over an `int $n`
  was `E0401: expected float, found int` — `nvs_types::expr::assign::is_assignable` had no
  `int`/`uint` → `float` row and no rule for a *union* source against a non-union target, so
  ADR 0007 § 4's `int|float` quotient could not reach a declared `float` either. Both rows are
  there now, with `nvs_ir::lower::Lowering::coerce` performing the conversion — and that is
  the shape of the trap for anything similar: the widening **throws** above 2^53, so it could
  not live in `coerce` until `&Env` was threaded through all 17 of its call sites *and*
  `close_nullsafe`, which had none of its own. A conversion that can fail needs the frame's
  landing block, and `coerce` was written when none of its rows could fail.
- **A shift count carries its operand's signedness, so a `uint` shift needs a `uint` count.**
  `$u << 64` is `E0407: int and uint have no representable common type in arithmetic`, because
  `nvs_types::expr::operators::bitwise_result` refuses a mixed-signedness pair for all five
  binary bitwise rows and a count is just the right-hand operand. Declare `uint $width = 64;`
  and shift by that. On the `int` arm a *negative* count is the one refusal PHP has —
  `ArithmeticError: Bit shift by negative number` — and a count of 64 or more answers `0`
  (or all-sign for an arithmetic `>>`) rather than the masked shift the machine would do.
- **A `decimal` binding takes a plain decimal literal, not a `d` suffix.** `decimal $d = 1.25d;`
  is four errors deep — the lexer reads `1.25d` as a *duration* literal and reports `E0007:
  \`.\` has no meaning in a duration`, then the parser loses the statement and the checker
  reports the wreckage as `E0401: expected \`decimal\`, found \`mixed\``, none of which names
  the real problem. ADR 0054 § 2 is why there is no suffix at all: a numeric literal is untyped
  until placed, so `decimal $d = 1.25;` is the whole spelling and the binding's declared type is
  what makes it a `decimal`.
- **An enum is not spelled the way PHP spells it.** `enum Mode: int { case Read = 1; }` parses
  as a *class* and reports eight errors on four lines, none of which says "wrong enum syntax":
  the Novis shape is `enum Mode { Read = 1, Write = 2 }` — bare names, commas, no `case`
  keyword — and the backing type is `enum Mask: uint { … }`, which is the only way to reach
  `EnumRepr::Uint` since a case past `int` is `E0437` under the default backing.
  `public Rank $rank = Rank::Silver;` **does** compile — a property default takes ADR 0046 § 2's
  whole constant set, an enum case and another class's `const` included
  (`nvs_types::defaults::const_reference_default`) — but a *parameter* default still takes a
  literal only, so `function m(Rank $r = Rank::Silver)` is `E0451`.
- **A nullable property cannot default to `null`.** `public ?string $s = null;` is `E0472: a property
  default must be a `string|null` constant — not a constant of the declared type`, and the same for
  `?object`, so a class in a case or a fixture cannot open a nullable slot the obvious way. What
  works is declaring it non-nullable and filling it in `constructor`, which is what the help text
  already says for the shapes it does mean to refuse. A **local** `?object $m = null;` is fine — it
  is only the property-default folder that refuses the `null` literal, and it refuses it for every
  `?T`, not just for an object one.
- **A write through a property has *two* receivers to untag, and the second one fails in
  cranelift rather than panicking.** `$m->rows = [...]` through a narrowed `?T` local works
  because `lower_reassignment`'s property arm calls `untag_receiver`; `$m->rows["0"] = "w"`
  goes back through the property a second time, in `write_back_array`'s own arm, and that one
  had no such call — so the `FieldSet` stored through a 128-bit "pointer" and the whole
  function was rejected with *"invalid pointer width (got 128, expected 64)"*. That message,
  with `i128` named as the failing operand, **is** the signature of a missing
  `InstKind::Untag`: read it as "a tagged value reached an instruction that wanted an object",
  not as a codegen bug. Nothing above catches it, because the checker is happy and the lowering
  never panics.
- **A `...spread` array-literal element lowers now, and it is PHP-exact on keys** — an
  integer-looking key is renumbered under the destination's counter, every other key is
  preserved (ADR 0007 § 5). So `[...$xs, ...$ys]` concatenates two lists and
  `[...$m, "k" => "v"]` overrides by name, `[...Core\Arr::keys($m), "end"]` works with a
  call as the subject, and a keyless element beside a spread continues from what the
  spread contributed rather than from its own position. The one refusal left is the
  append's: a literal whose explicit key is already `i64::MAX` throws PHP's *"Cannot add
  element to the array as the next element is already occupied"*. A **call** argument
  spread (`f(...$a)`) lowers now too — into a variadic tail at a resolved target, and
  through `Helper::CallClosureArray` at a `callable`.
- **A local's slot is re-pointed in four places in `nvs_ir::lower`, and a rule hooked into
  `bind_local_value` catches three.** That function is the funnel for `$x = e` and every
  compound form; `write_back_holder` (an `inout` argument's copy-back) and `write_back_array`
  (`$x[0] = e`) `env.insert` directly and each needs the hook of its own. The fourth is a
  *lowering's own* bookkeeping insert, and it is the one that bites: `foreach (… as inout $v)`
  re-points the array binding by hand, so with three hooks in place a nested
  `foreach ($grid as … inout $row) { foreach ($row as … inout $cell) … }` updated the row and never
  told the grid — it builds, every single-level case passes, and the wrong answer is a
  silently un-updated outer array. `grep -n "env.insert(" crates/nvs-ir/src/lower/` is the
  whole check, and it is worth doing for any rule phrased as "whenever this name is
  rebound".
- **A `holes.py` site's panic message names one route, and the `assert!` four lines below it is
  often the bigger one.** `lower_instanceof` had two: the missing-`ExprInfo` panic the worklist item
  quoted (the dynamic `$x instanceof $name` form — genuinely a diagnostic) and, right after it,
  `assert!(matches!(ty, Ty::Object))`, which aborted `$m instanceof Box` over a `mixed` — the shape
  the operator exists for. Only a `panic!`/`todo!`/`unimplemented!` is *counted* as a site, so the
  item's prose ("almost certainly a diagnostic rather than a lowering") was written from an
  inventory that could not see the assert. Read the whole function, and spend one scratch
  `.agent-tmp/*.nvs` per operand shape before believing an item that predicts its own answer.
- **A `#[should_panic(expected = "known gaps")]` test is how a lowering hole is pinned, and closing the
  hole turns it red rather than green.** `verify.py`'s `test` step failed on
  `a_mixed_condition_still_panics_naming_the_gap` *after* the panic was replaced by a working lowering —
  the diagnosis reads like a regression and is the opposite. `grep -n "should_panic" crates/<crate>/src`
  for the shape you are about to build, before you build it: the test is the previous session's note that
  this was deliberate, and the right move is to rewrite it as the snapshot test asserting what now
  happens, not to delete it. `cargo insta test -p <crate> --lib` then writes the `.snap.new` files
  (plain `cargo test` stops at the first one), and `git status --short | grep pending-snap` immediately
  before `cargo insta accept` is what keeps a previous session's orphan out of the commit.
- **An increment in *value* position runs the write where its own branch runs, and `echo` prints
  operand by operand.** Both bit a case that read plausibly and printed something else. `if ($no &&
  ($k++ > 0))` leaves `$k` at `0`, `$absent ?? $s++` runs the increment only when the left side is
  `null`, and a `match` arm's increment runs only for the arm that matched — all PHP-identical, all
  easy to write an expectation against as though the operand were evaluated unconditionally. And
  `echo "made: ", Cell::make()->count++, "\n";` prints `made: ` *before* `make`, because `echo`
  writes each operand as it reaches it rather than evaluating the whole list first; a case whose
  operand has a side effect has to expect the interleaving.
- **A static property does not lower, in either direction.** `Reg::$current` as a *read*
  panics `lower_expr`'s dispatch catch-all with a bare `StaticPropertyAccess { … }`, and
  `Reg::$count = 1;` panics `lower_stmt`'s reassignment arm naming the same node — so a case
  reaching for a class-level counter or a class-level flag has no spelling at all, and what
  works instead is an instance property on a local object (`Counter $c = new Counter();`).
  The checker accepts both happily, which is why this reads as a checker/lowering mismatch
  rather than as the missing feature it is; nothing on `python tools/holes.py`'s worklist
  names it either.
- **`static::$prop` is late-bound in PHP and was not here, and only a redeclaring subclass
  shows it.** `static::$total` inside `Base` answered `Base`'s slot for `Sub::viaStatic()`
  where PHP answers `Sub`'s — the two agree on every class that does *not* redeclare the
  static, which is why a scratch run has to redeclare one to see it at all. It is `E0499`
  now rather than a silent difference. The general shape is worth keeping: a resolved-at-
  compile-time storage and a spelling PHP resolves at run time agree until the two classes
  disagree, so the scratch file that judges one has to make them disagree.
- **A closure's declared parameter types are checked by nobody, and a mismatch is an arbitrary
  dereference rather than a fault.** `Core\Arr::map($ints, fn (string $s): string => $s)` over an
  `array<int>` dies inside `nvs-runtime`'s `string.rs` on a misaligned pointer, and `$f(1)` on a
  `fn (string $s)` does the same now that a direct call lowers: ADR 0031 § 1 gives `callable` no
  parameter list, so nothing compares a call site against the body it reaches and the compiled
  `invoke` reads each slot at its own declared representation. So a case or a fixture that hands a
  closure to a `Core` member must spell the element type and the parameter type *the same*, and a
  crash with no Novis frame in it is this before it is anything else. `nvs_runtime::closure`'s module
  doc owns the hole and what closing it costs.
- **An integer literal in an array-literal element position keeps `int`, whatever the array's
  declared element type says.** `array<uint> $u = [7, 8];` compiles and its elements carry the
  **`int`** tag, while `array<uint> $u = [7 as uint, 8 as uint];` carries `uint` — so a sweep whose
  point is the `int`/`uint` split silently asks the wrong question on the bare-literal row and
  answers plausibly. ADR 0054 § 2's "a numeric literal is untyped until placed" is applied at a
  parameter and at a binding but not at an element, and the only place it is observable today is a
  `callable`'s parameter-tag check, `Core\Reflect::typeOf` not existing yet. Convert in the literal
  whenever the tag is the subject.
- **A promoted constructor property does not exist as a property.**
  `final class M { public function constructor(public string $name) {} }` compiles, and `$m->name`
  is then `E0405: `M` has no property named `name`` from anywhere, including inside the class — so
  a scratch file that reaches for one gets a diagnostic naming the *property* and reads as a typo
  rather than as the missing feature it is. Declare the field (`public string $name = "m";`) when
  the case only needs an object with a field; `tests/conformance/core/out-capture-refuses-a-through-that-answers-anything-but-the-carrier.nvst`'s
  `Impostor` has a promoted one and gets away with it only because it never reads it.
- **An Novis enum case carries no `case` keyword**, so PHP's `enum Colour: int { case Red = 1; }`
  is `enum Colour: int { Red = 1, }` here — commas, not semicolons. Getting it wrong does not
  say so: the parser reports `E0220` *"an enum declares only cases and an optional backing
  type"* pointing at the `{`, then `E0101` *"expected a class member"* at the `case`, then one
  more pair per line, which reads like the enum is in a position that does not accept a
  declaration rather than like a spelling error. Check the spelling against a case under
  `tests/conformance/lang/` before concluding the *position* is what failed.
- **A property default may not be an array with entries in it** — `public array<string> $rows =
  ["a" => "x"];` is `E0472` (*"a property default must be a `array<string>` literal"*, which reads
  as if the literal were mistyped), and `[]` is the only array a declaration may carry. So a scratch
  file or a `.nvst` case that wants a pre-filled array property fills it with element writes after
  the `new`, or in `constructor` — and the same rule bites a `public static` one, where there is no
  constructor to fall back on and the writes have to be top-level statements.
- **A panic's message names the shape it was written for, not the shape that reaches it.**
  `stmt.rs`'s property-write panic said "its receiver erased to a plain `object`, which ADR
  0036 § 4's erased half still does not lower", and the erased half had landed sessions
  earlier — `$o->name = "z"` through a plain `object`, a `mixed` and a shape all run, and
  both of § 4's write throws (missing name, wrong type for the field's *real* declared type)
  fire correctly. What actually reached it was a **computed member name** and an undeclared
  property on a class kind the checker excused. Four scratch `.nvs` files under `.agent-tmp/`
  found that in one call each; reading the message and believing it would have rebuilt a
  feature that was already there. Enumerate the arms of whatever *records* the table entry
  and probe one program per arm, before taking the panic's own account of itself.
- **A plan field's "no shape left the checker accepts" is a claim, not a proof, and the cheap way
  to judge one is the roster and the binary rather than the arms.** `StmtKind` has 31 variants and
  `lower_stmt` had arms for 18; four of the thirteen left — `autoload`, `namespace`, `use` and
  `type` — reached the catch-all from source the checker happily accepted, and one of them
  (`autoload` in the *entry* file) is spelled in a passing conformance case, just from a
  `require`d file whose statements never lower. Grep the enum's variant list, subtract the arms,
  then write one scratch `.nvs` per survivor and run it: six `nvs run` calls settled thirteen
  variants, where reading the arms would only have re-derived the claim. The same subtraction is
  what turns the residue into the doc comment the panic then carries.
- **Inside a `namespace`, a qualified name resolves *relative* to it — including `Core\`.** A file
  that opens `namespace App;` and then writes `Core\Str::length("abc")` is `E0303: App\Core\Str is
  not declared`, and so is `Core\Str::class`; the reserved namespace gets no exemption from the
  ordinary unqualified/qualified lookup. This reads as a bug in whatever you just changed if the
  same line works in a namespace-less scratch file, which is how it cost time. `use Core\Str;` and
  then `Str::length(...)`, or write the leading `\`. The same rule is why `App\User::class` inside
  `namespace App;` is `App\App\User` — PHP resolves both exactly this way, so it is a trap rather
  than a divergence.
- **Widening what the *checker* accepts for an integer literal opens a hole in `nvs-ir` one
  crate down.** `lower_int_literal` decides `ConstInt` versus `ConstUint` from the
  `expected: Option<Ty>` its *caller* threads, not from anything the checker recorded — so a
  literal the checker newly places at `uint` still lowers as an `int` and panics with
  *"nvs-ir: integer literal `…` doesn't fit an `int`"* wherever the position hands no `Ty`
  down. `lower_binary` passes `Some(lty)` to its right operand and only the whole
  expression's `expected` to its left, which is why the left-hand digit run was the half that
  fell over while `$u - 18446744073709551615` was already fine. The two crates have to make
  the same placement, and the checker's half alone is not the feature.
- **A new `nvs_ir::Helper` row needs a *fourth* edit, and the three obvious ones all build
  clean without it.** The variant, `print.rs`'s name and `emit.rs`'s `helper_symbol` string are
  what a session looks for; what nothing points at is `nvs_runtime::helpers`' symbol table,
  the `(name, address)` list the JIT resolves against. Miss it and the whole workspace
  compiles, every unit test passes, and the *first program that reaches the new row* dies
  inside cranelift with `can't resolve symbol nvs_<name>` and no Novis frame anywhere in the
  message. `grep -n "nvs_call_closure" crates/` names all four sites at once.
- **`inout ...$rest` does not parse, and spreading into a variadic `inout` tail is accepted in
  silence.** `Parser::parse_arg` tests for `...` *before* it eats `inout`, so the marked spelling
  eats the word and then fails on the ellipsis — `E0714` plus five lines of `E0101`/`E0102`
  cascade, which is not a shape to pin. The unmarked one is worse: `Adder::many(...$rest)` against
  `public static function many(inout int ...$xs)` compiles and runs with no diagnostic at all,
  though nothing is written back. So `E0714`'s "a spread's entries" half is reachable only through
  a *fixed* `inout` parameter, and a case that wants the variadic row has to wait for that hole.
- **A new `nvs_ir::Helper` needs a *fourth* edit, and the three obvious ones build without it.**
  The variant, `nvs_ir::print`'s name and `nvs_codegen::emit`'s `helper_symbol` row all compile
  happily; what fails is at run time, `cranelift-jit` panicking with `can't resolve symbol
  nvs_value_add` from inside `JITModule`. The missing edit is `nvs_runtime::helpers::symbols()`,
  the `(name, address)` table `nvs-codegen` registers with `JITBuilder::symbol` — a `#[no_mangle]`
  helper is *not* found by name in the host process, it is found in that vector. Grep it for a
  neighbouring helper rather than trusting the compiler to notice.
- **A refusal in `nvs_types` phrased "this operand is not one of the four rows"
  does not cover a `mixed` operand, and the hole opens one crate down.**
  `reject_unary_arith_operand` decides from `equality_domain`, which answers
  `None` for a type that names no domain at all — a `mixed`, a union — and the
  `!matches!(…, None | Some(Numeric))` reading takes `None` as *accepted*. That
  is right (the deferral is what `mixed` is for) and it is also why `-$m` walked
  past the checker into `nvs-codegen`'s representation catch-all with nothing
  between. The general rule: a checker refusal written as "the type is not one
  of these" leaves the erased operand to the *runtime*, so every such site owes
  a tagged answer one crate down or it owes a diagnostic that names `mixed`
  explicitly. `python tools/holes.py --item N` lists the sites; running the
  shape in a scratch `.agent-tmp/*.nvs` is what tells the two apart in one call.
- **`nvs-codegen` has *two* catch-alls under one operator, and the cheap-looking one is the
  dangerous one.** `emit_binop` reports "a `Sub` over representation `Str`" from a
  representation gate near the top and "the binary operator `Mod`" from the match at the
  bottom, so which message a hole prints tells you which gate it fell through, not how bad it
  is. The gate's condition is `matches!(ty, Ty::Int | Ty::Uint | Ty::Bool)` — **`Ty::Bool` is
  `integral`** — so an unrowed `bool` pair never reaches either message: `true + true` lowered
  to an `iadd` over the `i8` a `bool` is stored in and printed a *number*, where the same hole
  over two `string`s merely refused. So when closing an operator table at the checker, probe
  the `bool` row first and read its answer rather than its exit status; a refusal you can see
  is the good case.
- **A `catch` binding has no methods at all, and that is now a diagnostic rather than a panic.**
  `catch (Throwable $e) { echo $e->getMessage(); }` is `E0405` where it is written, with a help
  naming the property that answers the same question (`->message`), and so is every other PHP
  accessor — `nvs_types::expr::calls::report_exception_accessor` is that mapping's home. It used
  to panic `nvs-ir` with *"an instance method call has no resolved target recorded in the
  typed-expression table"*, a message blaming a `mixed`/union/scalar receiver, because the
  exception tree was exempt from the unknown-member refusal long after `nvs_types::error_lib`
  began seeding it. So a case that wants to show *what* was thrown reads `$e->message`, and the
  spellings a ported program reaches for are refused where they are written.
- **An exemption written for a class family that had no signatures outlives the seeding
  and reads as a checker/lowering mismatch.** The exception tree was skipped by
  `infer_method_call`'s unknown-member refusal (`!qname.is_reserved_global_class()`) long
  after `nvs_types::error_lib` began seeding its properties and constructor, so
  `$e->getMessage()` panicked `nvs-ir` while `$e->nope` refused cleanly one module over.
  The cheap diagnosis is to ask the *other* member kind the same question: a property half
  that refuses where the method half panics means the hole is an exemption in the checker,
  not a missing feature below it.
- **A promoted constructor parameter is not a property at all yet**, so `private Greets
  $inner` has to be written out in full in any fixture that reads it back. `public
  function constructor(private Greets $inner) {}` parses and checks the *declaration*,
  and then `$this->inner` is `E0313: `Outer` has no property named `inner``, because
  `nvs_types::layout::own_properties` collects declared `ClassMemberKind::Property`
  members and nothing else — a reader that gets past the checker (a synthesized ADR 0043
  § 4 forward did) fails one crate down with "nvs-codegen does not lower the property
  `Outer::inner`, which this unit declares no slot for yet". ADR 0043 § 4's own worked
  example writes the field out, and so should a case.
- **A new `nvs_runtime::Tag` discriminant collides with a nibble that was chosen as "one past the
  roster".** `FN_PARAM_TAG_ANY`/`CLOSURE_PARAM_TAG_ANY` — the closure-parameter nibble meaning "no
  argument can be wrong for this one" — was `12` because the tag roster ran to eleven, so adding
  `Tag::Unset = 12` made every `mixed` closure parameter demand a tag instead. Nothing about it is
  visible from the tag's own crate; what caught it is `nvs-codegen`'s
  `the_any_nibble_denotes_no_tag_at_all`, one test holding three crates' copies of one number
  together, and the five conformance cases that then failed all named closure arguments rather than
  the tag. Both constants are `15` now, parked at the top of the nibble on purpose.
- **A property access records the class the *receiver* was typed as, not the class that declared the
  property.** `nvs_types::expr::members::check_property_member` writes `ExprInfo::Property { class:
  qname }` from the receiver, which is right for `InstKind::FieldGet` (a slot index computed against
  a base class is valid for every subclass) and wrong for any per-property fact `nvs-ir` looks up by
  that label: an inherited `lateinit` read through a subclass answers `false` against the parent's
  own-only record and the guard is silently not emitted. Flatten such a table along the class graph
  where it is recorded, rather than expecting the declaring class at the site — and note the failure
  is quiet, because a method that never touches `$this` runs perfectly well on a null receiver.
- **A `nvs-ir` instruction's kind is often bound to a local first, so grepping `self.emit(` for a
  literal `InstKind::` misses sites.** Sweeping every status-returning instruction onto
  `emit_fallible` for item 36, a scanner that parsed the kind out of each `self.emit(...)` call
  found 18 of 20; the two it could not see were `let call = InstKind::HelperCall { … };` followed by
  `self.emit(cur, ty, call)` several lines down (`convert.rs`'s `as ?T` and `array<T> as array<U>`
  rows). What found them was turning `nvs_codegen::emit`'s tolerant `None` arm into an
  `internal(...)` refusal and running the crate's own tests: the backend already knows which
  instructions return a status, so make it say so and let the suite enumerate the producers rather
  than trusting a grep over the emitters.
- **`Class::CONST` is `mixed` at every expression site, whatever the constant declares.**
  `nvs_types::signatures`' own module doc names it as a known gap: the const table holds ADR 0047
  § 2's folded *values* and there is no table of a class constant's declared *type*, so `int $n =
  Limits::MAX;` is `E0401` and a payload field holding one satisfies no shape declaring a scalar.
  A `.nvst` that wants a constant in a typed position writes the literal, or an enum case, which
  does carry its type.
- **An attribute's name was already load-bearing for two other passes before ADR 0046 § 1 got to
  say what it means.** ADR 0071 § 1 matches `#[Json\Derive]`/`#[Json\Field]` *nominally* against a
  closed `Core`-owned roster, and ADR 0061 § 1 harvests every attribute name as a reference the
  autoloader then places by prefix. So a rule about what an attribute name may resolve to has to
  exempt the first and leave the second alone — and the tests that pin them are the ones that
  fail first: `tests/conformance/lang/a-class-named-only-by-an-attribute-is-autoloaded.nvst` and
  the four `json-derive-*` cases.
- **An expression-bodied `fn (): void => <a void call>` does not lower**, and its
  block-bodied twin does. `Core\Test::expectFailure(fn (): void => Test::assertSame(1, 2));`
  fails the whole compilation with *"nvs-codegen does not lower an operand used before it
  is defined"*, naming a compiler bug for a shape the checker accepted, while
  `fn (): void => { Test::assertSame(1, 2); }` runs — the two differ by nothing else, so
  write the braces whenever a closure's whole body is one `void` call. Bisecting to it
  costs a scratch run per candidate, because the message names neither the closure nor the
  call.
- **A size check is not a loop bound.** `Core\Bytes::repeat` and `Core\Str::repeat` both
  checked the *product* — `affordable` on `len * times`, then the allocator — and then ran
  `for _ in 0..times`. An empty subject makes that product zero for every count there is,
  so both checks pass and the loop then runs a caller-supplied `uint` of iterations
  appending nothing: `Core\Bytes::repeat($empty, 2000000000)` spun for 74 seconds to answer
  the empty buffer, and `uint`'s maximum is an unbounded spin on the request path with a
  caller's number as its only bound. Both short-circuit on an empty subject now. The
  general shape to look for is a loop whose iteration count is the *caller's count* rather
  than the *result's size*: the two size seams are both about how large the answer is, so
  neither can see it, and nothing else in the tree can either — it builds, it is correct,
  and it returns.
- **`nvs_runtime::affordable` is not the last check, and the gap is one header wide.** It
  accepts any size up to `isize::MAX` and knows nothing of the container header the
  allocation then prepends, so a `Core\Str` producer handed exactly `isize::MAX` cleared
  the check and reached `str_layout`, whose two `expect`s panicked — a FATAL no program
  can catch, out of the *fallible* `NvsStr::try_build`. `try_str_layout` closes it for
  every `built_fallibly` caller. The recognition test for the shape: a member whose size
  check and whose allocation are two different expressions — `padding_run` bounds the run
  it is about to add while `built_fallibly` allocates that run *plus the subject* — and
  the boundary count is the largest the first one accepts, not a round number. The
  `Core\Bytes` and `Core\Arr` families were already clean, their `Vec`-backed reserves
  being genuinely total.
- **An expression-bodied `fn (): void => Something();` panics `nvs-codegen`, and the block-bodied
  form of the same closure does not.** `Core\Out::capture(fn (): void => M::run())` dies with
  `nvs-codegen does not lower an operand used before it is defined`, while
  `Core\Out::capture(fn (): void => { M::run(); })` runs — and `fn (): int => M::n()` runs too, so
  it is the `void` return rather than the call or the arrow that is unlowerable. Every existing
  `Core\Out::capture` case in the corpus already uses the block body, which is why nothing had
  caught it. Write the braces; the panic names neither the closure nor its return type.
- **A refusal the parser writes takes an `E02xx` code, not the `E01xx` "next free parser code."** The
  bands are by *kind*, not by which crate reports them: `E01xx` is a malformed parse, `E02xx` is
  "rejected PHP constructs", and a PHP spelling Novis declines is the second one however early it is
  caught. `E_IMPORT_ALIAS_UNSUPPORTED` (`E0212`), `E_ENUM_MEMBER_UNSUPPORTED` (`E0220`) and
  `E_IMPORT_GROUP_UNSUPPORTED` (`E0238`) are all reported from `parser/decl.rs`. So a `loop-goal.toml`
  comment naming "the next free parser code" is naming the band `brief.py` prints for `E01xx`, which is
  the wrong half of the registry for a refusal — find the sibling refusal's code first and take the
  number next to it.
- **A refusal site's *message* can be about a different feature than the item that claims it**, because
  `holes.py` attributes by file. Item 16 was written as "a named argument and a spread argument lower",
  and both of those had landed; all three sites it still claimed were the `let CallArgs::List(list) = args
  else` arms, which only the first-class-callable sentinel `(...)` ever reaches. Read the site's *else*
  branch before believing the item's title, and settle reachability with four scratch runs rather than by
  reasoning: `Class::method(...)` and `$obj->method(...)` both died a whole file earlier at
  `expr.rs`'s "no resolved target recorded" panic, `$m->method(...)` was already a diagnostic, and the one
  shape that actually reached `lower_call_args` was `new C(...)` — which nothing in the item mentioned.
- **A function's disassembly holds more cold blocks than its IR does, so "skip the `Propagate`
  blocks" does not skip the cold path.** Counting `call` lines in a whole `--dump-asm` section
  prices an access for three other mechanisms: for `$c->n = $c->n + 1` the extra calls are an
  ArithmeticError *construction* — which lives in a block codegen invents for the `seto` check and
  that corresponds to no `nvs_ir` block at all — plus the landing block's `Release` and its
  `Propagate`. What actually separates hot from cold in the VCode text is one shape: `testX`
  immediately followed by `jnz labelA; j labelB`. That is how *every* status word is checked — a
  call's error return, an overflow's `seto`, the safepoint's pending-exception field, and the ADR
  0018 probe's null table pointer — so walking the `blockN:` graph from the entry and never
  following that `jnz` gives the path a run that throws nothing takes. Two consequences worth
  knowing before writing a guard: the probe calls are *conditional*, so a "subtract one call per
  `StmtMarker`" correction over a whole-function count is measuring nothing; and `Cell::plainOne`
  emits 23 calls of which 5 are on that path.
- **A fixture that needed a `mixed` value has probably reached for whatever was `mixed` that
  week, and closing a gap moves it.** `an_array_index_through_a_mixed_base_defers_to_the_tag`
  (`crates/nvs-types/src/expr_table.rs`) used `T::UNTYPED[0]` over an unannotated `const UNTYPED
  = 1;` purely because a class constant inferred `mixed`; the moment item 45 gave one a type it
  became an `int` subscript and the test failed on the *deferral* it was written to assert, not
  on anything about constants. The fix is to give such a fixture the erasure it actually means —
  a `mixed` parameter — rather than to hunt for the next thing that still infers `mixed`. So when
  a slice widens what the checker knows about a shape, `grep` the test tree for that shape used
  as a *source* and not as a subject.
- **`Core` is two rosters, not one, and the second is the exception tree.** Narrowing a `Core` name
  from the `is_core()` spelling test to `nvs_stdlib::registry` looks total — the registry's own docs
  call `CLASSES` the whole roster — and it refuses `new Core\Test\Failure(...)`, which lives in
  `nvs_hir::errors::TREE` instead. `QName::is_reserved_global_class`'s doc comment says so out loud
  ("its one namespaced row is trusted to exist through `is_core`"), on the predicate you are *not*
  editing, so it is only found by the conformance leg. Any check that reads `is_core()` as "in the
  registry" owes `errors::is_exception_class` beside it.
- **"ADR § X makes this return-only" is a rule with two enforcement sites, and a tree can have
  neither while looking like it has one.** ADR 0007 § 3's `void`/`never` were return-only in the
  grammar's prose and nowhere else: `never $p` panicked `nvs-ir`'s `lower_checked_ty`, and `void $p`
  type-checked, *lowered fine*, and died one crate further down with `internal error: reading a
  value of representation 'void'`. Both `docs/agent/loop-goal.md` item 25 and `type_atoms.rs`'s
  `KNOWN_ICE` comment recorded "`void` in a parameter is diagnosed" on the strength of the second
  one failing differently. Two minutes of `nvs run` on a two-line scratch file is what tells the
  three apart — a shape that panics, a shape that is refused, and a shape that reaches codegen and
  dies there is a *third* outcome `crates/nvs-ir/tests/type_atoms.rs` cannot see at all, because it stops at
  `lower_program`.
- **`nvs-ir` does not depend on `nvs-hir`, so an `ExprInfo` variant carrying a `QName` cannot be
  destructured by name there.** `nvs_types::expr_table::ExprInfo` names `nvs_hir::QName` freely —
  `ExprInfo::New`'s `class` is one — and `nvs-ir` gets away with it only because every site it reads
  calls `class.to_string()` without ever *writing* the type. A new variant whose lowering helper
  wants a `&[nvs_hir::QName]` parameter fails with `unresolved module or unlinked crate nvs_hir` at
  the signature, not at the use, which reads as a missing `use` and is not one. Convert to `String`
  at the `self.exprs.lookup(...)` site and let the helper take `&[String]`; adding the dependency to
  buy one type name would put the whole HIR in the lowering crate's graph for nothing.
- **Inside `namespace App;`, `Core\Str` means `App\Core\Str`.** A `Core` name in a namespaced file
  needs the leading `\` — `\Core\Str::upper`, `#[\Core\Route(...)]` — and without it the diagnostic
  is `E0303: 'App\Core\Command' is not declared`, which names the joined path rather than the missing
  backslash. The top-level fixtures never show this because they are in the global namespace, so it
  first bites on the *autoloaded* half of a two-file example.
- **A `Core` attribute name cannot be an ADR 0046 § 1 shape alias, because there is no `Core`-seeded
  alias table.** `nvs_hir::AliasTable` is collected from source `type` declarations and from nothing
  else (`crates/nvs-hir/src/aliases.rs:92`), so `Core\Command` reaches
  `nvs_types::attributes::resolve_shape_alias` with no entry, `qname.is_core()` says it is declared,
  and the answer is `E0726: … is not a `type` alias` — a refusal no stdlib edit can lift. Every
  `Core`-owned attribute is therefore a *nominal* match on `nvs_types::derive::ATTRIBUTES` and owes a
  pass that checks its payload, which is the one thing that keeps that closed list from admitting
  `#[Command(nmae: "x")]` in silence. A handoff item that says an attribute "becomes a shape-typed
  `type` alias" is naming the diagnostic, not the mechanism.
- **A `Class::CONST` whose class does not exist passes checking and panics `nvs-ir`.**
  `echo \Core\Http\Method::Get;` type-checks clean today and dies at
  `crates/nvs-ir/src/lower/expr.rs:274` with "a `Class::CONST` … with no value recorded in the
  typed-expression table". So a one-line probe that *compiles* is not evidence that the name it
  writes resolves to anything, and `nvs_types::check` reports no `E0405` for the class half of
  that spelling. The same hole is why a payload roster naming an enum the tree does not declare
  (`nvs_types::routes` gap 1) admits a case of the wrong enum rather than refusing it: nothing
  below the roster is asking whether the name exists.
- **`array<K, V>` is a spelling the docs write and the type system has no form for.**
  `nvs_types::ty::Ty::Array` carries one `TypeId`, and there is no `CoreTy` for a keyed
  array — a Novis array's keys are `int|string` by construction and are not part of its
  type. So `array<string, mixed>`, which spec § 15 and ADR 0077 § 4 both write for
  `Core\Router::url`'s `$params`, is declared as `CoreTy::Array(&CoreTy::Mixed)` and the
  key rule is enforced where it can be (§ 4 makes a literal key naming neither a capture
  nor a `#[Query]` parameter a compile error). Same family, one call earlier:
  `Core\Arr::append`'s second parameter is one *element*, so `Core\Arr::append($a, $b)`
  over two arrays is `E0401: expected int, found array<int>` rather than a concatenation
  — build the combined array with a `foreach` and one `append` per element.
- **A field added to `nvs_types::Env` builds clean and fails `--all-targets`.** There are three
  construction sites, and the third (`crates/nvs-types/src/lower.rs:668`) is inside a `#[cfg(test)]`
  module, so `cargo build -p nvs-types` is green while `cargo check -p nvs-types --all-targets` is the
  first thing that reports `E0063: missing field`. The two real sites are `check.rs`'s per-file loop
  and `signatures.rs`, which wants a scratch value for the same reason it passes a placeholder
  `ExprTypeTable`: its pass runs before anything fills the new table.
- **A `Core` implementation symbol that no `CoreMethod` row names is never handed to the JIT, and
  the failure is a `cranelift-jit` panic at run time rather than anything a build reports.**
  `nvs_stdlib::symbols()` (`crates/nvs-stdlib/src/lib.rs:264`) builds the roster it registers by
  walking `registry::CLASSES`' member rows, so adding an arm to a module's own `address()` — which
  reads like the whole registration, and is what `docs/agent/conventions.md` § *A `Core` member*
  calls "the one that bites" — resolves nothing: `address_of` is only ever asked about a symbol the
  roster already produced. The symptom is `can't resolve symbol nvs_core_…` from
  `cranelift-jit/src/backend.rs`, naming no Novis file. Any symbol lowering emits that is not a
  member's own — ADR 0077 § 4's two prepared link entry points, a constructor — owes a `.chain()`
  in that function beside `registry::CONSTRUCTORS`', **and** a term in
  `every_registered_member_has_an_implementation_address`' arithmetic, which is the same sum
  written out a second time and fails the moment the roster grows.
- **The handoff proposes; the ADR decides — and `nvs.toml`'s location is the worked case.** The handoff
  scoped `[app] origin` as "read `nvs.toml` beside the entry file", which ADR 0103 § 1 step 2 forbids
  outright: the root of the configuration tree is `./nvs.toml` in the **working directory**, exactly one
  directory and never a walk upward, with `--config` as the only other source. Following the handoff
  would have put the fixture in `examples/` where no `nvs run` from the repository root would ever read
  it. A handoff bullet is the previous session's *plan*, written before it read the ADR the slice lands
  inside; when the two disagree the ADR body wins and the handoff is the bug. ADR 0104 is the one to
  read next here — it makes `[[app]]` an array of tables keyed on `root`/`entry`, which ADR 0102 § 6's
  plain `[app] origin` does not know about yet.
- **A refusal over a *derived* fact fires on declarations that were already refused for something
  else, and the existing `--EXPECTF-ERROR--` case is what catches it.** ADR 0071 § 7's "an attribute
  with no effect is a mistake" reads as "refuse a `#[Json\Derive]` class whose field list came out
  empty" — but `reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst` declares two properties
  and has *both* refused, so its list is empty too and its frozen `aborting due to 2 errors` became
  three. The fix is a three-way outcome per property (kept / skipped in writing / refused) rather
  than an `Option`, so the empty-contract error fires for a class that chose an empty contract and
  never for one that was already told what is wrong. General shape: before adding a diagnostic
  whose condition is *the absence of a result*, grep the reject tree for a case that already makes
  that result absent.
- **Renaming a `pub` field breaks intra-doc links written in its *neighbours*, and the only step that
  says so is `verify.py`'s last one.** `Route::query` became `Route::params`; the sibling `access` field's
  doc comment said "exactly as [`Self::query`] does", which builds, tests, clippies and formats cleanly and
  then fails `cargo doc` with `-D rustdoc::broken_intra_doc_links` — a whole verify run spent on a
  four-character edit. Before renaming a `pub` item, `grep -n "Self::<oldname>\|\[\`<oldname>\`\]"` over the
  crate: an intra-doc link is invisible to every other tool in the gate.
- **A new `CoreTy` variant that *wraps* another type silently falsifies seven walkers, and ADR 0088
  § 2's classification is a leaf variant because of it.** Every recursive `match` over `CoreTy` in
  `crates/nvs-stdlib/src/registry.rs` — `collect_written`, and the six inside `mod tests` — ends
  `_ => {}` under a comment saying "a variant that carries no nested type carries no variable
  either". `CoreTy` is `#[non_exhaustive]`, so nothing outside errors either, and a
  `Classified(Qual, &'static CoreTy)` wrapper would have compiled clean while making `written()`,
  the nullable check and the enum-case check blind to whatever it wrapped. The landed spelling is
  `CoreTy::Text(Qual)` / `CoreTy::Blob(Qual)` — leaves, so the wildcard arms stay honest — and the
  only two sites that had to change are the ones that name `CoreTy::Str` specifically:
  `nvs_types::core_lib::lower` and one `matches!` in a registry test. **The data landed first and the
  enforcement followed**: `lower` still maps `Text(_)` to the same interned `string` as `Str`, and the
  mark travels beside the lowered type in `MethodSig::param_quals`, which is what the call check reads.
- **`OWED_A_CASE`'s declaration window is eight lines measured from the `Fault::` line, so a
  long comment with the phrase at the top is invisible to the gate.**
  `conformance_coverage.rs`'s scan walks *upward* from the site and stops at the first line
  containing `Fault::`, so a nine-line paragraph whose first line reads "Unreachable from
  source: …" declares nothing, and the failure re-prints the same worklist line with no hint
  that the comment exists. Put the phrase in the comment's **last** sentence when the
  reasoning needs more than four lines, and note that a helper with several guards needs one
  declaration per guard rather than one at the top of the function — `time.rs`'s `instant_of`
  reads three slots and each needed its own line, two of them a single sentence pointing at
  the first.
- **A grapheme count does not decompose into one correction per seam, and regional indicators
  are the whole of why.** Caching `Core\Str::length` in the string header makes a concatenation
  want `left + right - (a cluster spans the join)`, and that is right for every UAX #29 rule but
  GB12/GB13: those group a run of `Regional_Indicator`s into *pairs*, so a prefix ending in an odd
  number of them re-groups the entire run after the join. `"🇩" . "🇩🇪"` is **two** clusters where
  each side alone is one, and the seam is *not* a boundary — so the obvious arithmetic answers 1
  and a corpus test catches it two edits later. The parity that would fix it for two pieces is
  wrong for three (`"🇩" . "🇩" . "🇩"` is 2, and the per-seam parities say 3), because the parity is
  a property of the finished run rather than of any adjacency. `nvs_runtime::graphemes::seam_joins`
  refuses the seam outright when a regional indicator sits on both sides, and the caller leaves the
  count uncached; that is one range check each side and it costs flags a scan rather than an
  answer. The general shape: a cached aggregate over Unicode text may only be corrected locally for
  the rules that *are* local, and there is exactly one that is not.
- **A `Core` call that builds its argument vector by hand must call `account_for_arg` itself, and
  nothing but a valgrind run will tell you it did not.** `nvs_ir::lower::lower_route_link` is the
  only site that does not go through `lower_call_args`, and it lowered `$params` without staging
  it on `owned_temporaries` — so `release_temporaries_since` had nothing to release and every
  `Core\Router::url("…", ["id" => 7])` leaked one array header per call. It compiled, it ran, it
  printed the right link, and every test passed: a leak is invisible to the program that causes
  it, so the only leg that saw it was the driver's `examples/` valgrind sweep, three stages after
  the code landed. `wsl.exe -- bash tools/leak-check.sh <fixture>` is the one-fixture form — and
  from the Bash tool it needs `MSYS_NO_PATHCONV=1` in front, or Git Bash rewrites `/mnt/d/...`
  into `C:/Program Files/Git/mnt/d/...` and the script is simply not found. The general shape:
  wherever lowering hand-rolls what a shared helper normally does, the accounting is the half that
  gets dropped, and `crates/nvs-ir/src/lower/tests.rs`'s
  `a_resolved_route_link_releases_its_params_array` is the assertion shape that pins one — find
  the call's own argument `ValueId` and require a `Release` of *it*, never a count of releases.
- **Classifying a `Core` class can break that class's *own* structural unit test, and the ADR 0088
  ratchet says nothing about it.** `Core\Test`'s nine rows share one `MESSAGE: &[CoreOption]`, so
  marking the bag is a single edit — and `test::tests::the_only_option_is_a_message_that_defaults_to_absent`
  asserts `matches!(bag[0].ty, CoreTy::Str)` over every one of them, which
  `CoreTy::Text(Qual::Neutral)` is not. The registry-wide gate
  (`every_member_parameter_carries_a_qualifier_classification`) passes cleanly while that one fails,
  so the failure names a member you did not think you were editing. Grep the class's own `mod tests`
  for `CoreTy::Str` before classifying it; `Core\Path`, `Core\Time` and `Core\Uri` happened not to
  have one and `Core\Test` does.
- **A *prepared* `Core` member has two bodies, and the registry names the one that throws.** Reading
  `nvs_stdlib::registry`'s `symbol` for `Core\Router::url` lands on `nvs_core_router_url`, whose whole
  body is `Err(no_such_route(...))` — which reads as "this feature is not built", and the module's own
  *Known gaps* agreed. It is built: `nvs_types::links` folds a literal route name while checking,
  `nvs_ir::lower::expr::lower_route_link` swaps the written name for a *prepared path* and calls a
  **second symbol** (`nvs_core_router_link`), and that one does the real work. The registry row names
  the unfolded path because that is the one a computed argument takes. So for any member ADR 0057 § 3
  *prepares* rather than folds, `grep` the crate for a sibling symbol before believing the body the row
  points at — or just run it: `target/debug/nvs.exe run` on four lines settled in one call what reading
  three doc comments had got backwards.
- **A class is not generic at the `new` site.** `new Core\Task\Channel<int>(2)` is `E0441: this target
  takes no type arguments` — the `<T>` positions the language has are the built-in ones (`array<T>`,
  `Iterator<T>`, `Core\Program::implementing<T>()`), not a user or `Core` class's constructor. A
  container's element type is therefore carried by the `foreach` binding (`foreach ($chan as int $v)`)
  and by the declared type of what goes in, which is enough for the checker and is what
  `examples/channel.nvs` is written against.
- **Dropping a suspended `corosensei` coroutine unwinds its stack, and a `catch_unwind` in the way
  aborts the process.** `Coroutine::drop` raises a private `ForcedUnwind` marker down the coroutine's
  stack and expects to see it come back out; `nvs_runtime::run_task` — which sits under every task
  root — swallowed it, so corosensei's own `panic!("the ForcedUnwind panic was caught and not
  rethrown")` fired inside a `Drop` and double-panicked to `STATUS_STACK_BUFFER_OVERRUN`
  (`0xc0000409`, and on Linux a plain abort). The recognition test is that the crash names
  `corosensei-0.2.2/src/coroutine.rs` twice, the second time with *"cannot propagte coroutine panic
  with #![no_std]"* — that second message is the double-panic, not the cause. It is reached by an
  ordinary worker shutdown with one request still parked, which is why the fix is
  `nvs_runtime::Teardown` (a thread-local depth counter that makes `run_task` re-raise instead of
  contain) held across `Drop for Scheduler`, and not "drain the parked set first". Anything else that
  gains a `catch_unwind` between a coroutine's root and its suspension points owes the same guard.
- **A non-blocking `connect` cannot be completed by asking `peer_addr`, whatever `mio`'s own example
  says — on Windows that call answers `Ok(the target address)` for a socket whose connect has not
  succeeded and never will.** `take_error` is `Ok(None)` there too until the attempt actually ends
  (about two seconds for a refused loopback port), so a connect built on either question alone
  reports success and hands back a stream that is writable and dead. The pair that is sound on both
  platforms is `take_error` plus a **zero-length write**: `Ok(0)` on a connected socket having sent
  nothing, `NotConnected`/`WouldBlock` while the handshake is in flight, and on Linux the refusal
  itself. `crates/nvs-host/src/net.rs:155`'s `finish_connecting` is the worked shape and its doc is
  the home of the reasoning. Cost of finding this by hand: four probe builds across Windows and WSL.

- **A poll asked to wait a bounded time can come back early, so "0 woken" is not "nothing can wake
  these tasks".** A platform rounds a wait to its own timer granularity and returns a fraction of a
  millisecond before the deadline it was given; `run_until_idle` breaks out of its loop on
  `Some(0)`, so the first timer written straight into `Reactor::turn` abandoned a lone sleeping task
  and the test hung on an assert rather than on the clock. `turn` therefore retries a bounded wait
  until it has genuinely reached the earliest deadline (`crates/nvs-host/src/reactor.rs:332`). The
  giveaway that it is this and not a lost wake: the same code with a *second* runnable task passes,
  because the extra turn hides the early return.
- **A task cannot reach the `&mut Scheduler` that is resuming it, and every obvious design for the
  task tree dies on that.** `Scheduler::run` holds `&mut self` for the whole turn, so a running
  task can call neither `spawn` nor `cancel` nor anything else on the scheduler — which means a
  parent/child link stored on `Task`, or a cancel flag read out of the run queue, cannot be written
  from the one place that knows the answer. What works is splitting the scheduler in two: the run
  queue and the stack pool stay behind `&mut self`, and the part a task needs — the id counter, the
  parent links, the cancel flags, and a pending list of children asked for but not yet built — goes
  in an `Rc<RefCell<..>>` the scheduler publishes in a thread-local for the length of its turn
  (`scheduler.rs:211` and `:782`), which is the shape `crate::reactor` already uses. The second
  half of the same trap: never call `Coroutine::force_unwind` from a task's own stack. Teardown
  belongs on the scheduler's stack, which is why cancellation *marks* and the next turn unwinds.
- **A running task cannot wake a peer, and `Scheduler::wake` is not the route.** It takes
  `&mut Scheduler`, which is the frame currently resuming the task, so anything that unblocks
  *another* task — `nvs-host`'s channel, and every synchronisation primitive written after it — goes
  through `nvs_host::Wake` instead: a handle taken while the waiting task is running, which queues a
  `TaskId` on the task tree and is drained into the run queue by `Scheduler::run`. Two things about it
  are load-bearing and neither is the obvious implementation. It **holds the tree** rather than reading
  the `TREE` thread-local when it fires, because whoever wakes it is often outside any turn — a test
  driving `sched.run()` by hand, an accept loop's end of a channel — and because a `TaskId` is unique
  only within its own tree, so a thread-local read would cross wakes between two schedulers on one
  thread, which is a shape most of this crate's tests take. And the drain has to run **at the top of
  `run` as well as after every resume**, or a wake issued between turns lands in the queue and nothing
  ever delivers it, leaving the parked task asleep with no error anywhere.
- **One new row in `nvs_stdlib::registry::CLASSES` owes four gates, three of them outside the
  crate you are editing.** `every_registered_member_has_an_implementation_address`
  (`crates/nvs-stdlib/src/lib.rs`) wants a symbol with a real address;
  `every_part_one_member_has_a_conformance_case` wants a `.nvst` under `tests/conformance/`
  whose `--FILE--` writes `Core\X::y(`; `every_core_class_has_a_conformance_floor_of_three`
  wants three of them; and `BELOW_THE_FLOOR` only ever shrinks, so there is no exemption to
  add. That reads like a bar on registering a member whose *runtime* is a later slice, and it
  is not: the coverage gate greps the `--FILE--` section and never runs anything, so an
  `--EXPECTF-ERROR--` case discharges it, and three of them discharge the floor —
  `Core\Task::all` landed exactly that way, with its three cases pinning § 1's typing instead
  of its behaviour. The address gate takes `crate::attributes`' shape: a body that says at its
  own site why it was reached and stops. What you must *not* reach for is a placeholder
  `Fault::`, because `every_error_path_is_asserted_or_declared_unreachable` then wants either
  a case freezing that message or a declaration naming the diagnostic that refuses the call
  first, and a not-implemented-yet body has neither.
- **A bare-message failure with no runtime error class installed loses its message at
  `Ctx::take_thrown`, and answers a *null* `Thrown`.** `set_pending` files a
  `Pending::Message`, and the promotion into an object needs a class descriptor —
  `Thrown::new_as` returns `Thrown::none()` for a null one, whose `message()` is `""`. Every
  embedder installs one (`Unit::install_in`), so this only bites a Rust-side test that built a
  `Ctx::new(...)` by hand and then asserted on a thrown message: the assertion reads
  `left: ""` against the message you just set, and nothing points at the missing class.
  `crates/nvs-host/src/group.rs`'s `ctx_with_error_class` is the four-line fixture — a
  `ClassTable` with one `Throwable` carrying the four slots, handed over with
  `Ctx::set_runtime_error_class`. Worth knowing before writing the test, not after: the same
  promotion is what a compiled `catch` runs, so this is the existing semantics rather than a
  gap, and a test asserting on `matches!(.., Threw(_))` instead is a weaker test for no reason.
- **A `Core` member may not park, because a forced unwind cannot cross its
  `extern "C"` frame.** `nvs_host::Scheduler::tear_down` cancels a *parked* task
  with `corosensei`'s `force_unwind`, and a member that suspended — `Core\Time::sleep`
  through `nvs_runtime::host::Host::sleep` is the first that wants to — is a
  `nvs_helper!` frame on that stack. Two symptoms, in this order, and both are
  the same cause: `the ForcedUnwind panic was caught and not rethrown` from
  inside `corosensei`, because `run_helper`'s `catch_unwind` swallowed it; then,
  once `run_helper` re-raises it (it does now, on `Teardown::in_progress()`),
  `panic in a function that cannot unwind` naming the helper. `extern "C-unwind"`
  is not the fix — the frame *below* the helper is JIT code with no landing
  pads, so an unwind through it would leak every temporary it owns. The fix is
  the other route Novis already has for exactly this: a cancelled task with
  script frames dies by ADR 0002's return status at its next safepoint, which is
  what `nvs_safepoint` gives `SafepointFlags::CANCEL`.
- **A task that dies by ADR 0002's return status leaves a *pending* message on its context, and whatever collects that context must not read it as a throw.** The symptom is a program whose deadline works perfectly printing `uncaught in a cancelled sibling: the request was cancelled` and then an uncaught exception at the `Core\Task::map` call site — a cancelled child, collected by `nvs_host::group::Child::run`, whose `ctx.pending()` was the safepoint's own record of the teardown rather than anything the script threw. `Ctx::cancelled()` is the discriminator and it is asked *before* `pending()`; a cancelled child's slot stays empty, exactly as it does for one a forced unwind tore down. The same trap is waiting for every future collector of a child context — the request boundary under `nvs serve`, and whatever reports a `spawn script`.
- **A `CoreTy::Uint` parameter arrives tagged `Tag::Uint` (3), not `Tag::Int` (2), so `Value::as_int()`
  on one answers `None`.** `uint` is a tag of its own by ADR 0007 § 4, and the two are not interchangeable
  at the ABI however interchangeable they look in a signature. The failure is not a compile error and not
  a wrong number — it is the member's *own* "expected an int, got tag 3" fatal, which reads as a caller
  bug and is not one. `Value::as_uint()` is the reader, `Value::uint(…)` is what a `-p nvs-stdlib` test
  has to hand such a member, and `crates/nvs-stdlib/src/arr.rs:1792` is the shape to copy. Worth the
  bullet because the row and the body are written in the same minute and nothing between them says which
  tag a `CoreTy` lands as.
- **`Qual::Sink` needs no rule of its own in `nvs-types`, and that is not the gap it looks like.** A
  classified `CoreTy::Text(q)`/`Blob(q)` lowers to exactly what its unclassified spelling lowers to
  (`crates/nvs-types/src/core_lib.rs:286`), so the refusal a sink parameter gets is ordinary
  assignability: a `tainted bytes` argument does not satisfy a plain `bytes` parameter. The checker's
  only `Qual` rule is the *admission* in `expr/quals.rs`, which lets the other three marks through and
  leaves a sink to that plain refusal, so grepping for code that reads `Qual::Sink` still finds
  `crates/nvs-stdlib` and nothing else. What genuinely needs a checker rule is the *opposite* shape: a sink whose parameter is `mixed` (`Core\Debug::dump`,
  `Core\Serialize::encode`), where nothing below the call site can still see the qualifier, which is why
  those three live in `expr/quals.rs` as call-site walks over the written arguments.
- **The two `Core` coverage gates run in opposite directions, and a class with no members owes
  neither anything.** `conformance_coverage.rs` walks `registry::CLASSES` and asks the repository
  for a `.nvst` case per *member* (plus a floor of three per member); `spec_registry_coverage.rs`
  walks `docs/spec/01-core-library.md` §§ 1-12's `| Member | Signature |` *rows* and asks the
  registry for each. So registering `Core\Script\Handle` — three lines and a module — turned both
  green with no spec edit and no case, where the handoff had predicted a spec entry beside it.
  Adding a row to the spec for such a class is the actual trap: it would demand a member the class
  exists in order not to have. A class's *prose* home is still owed, and § 19's own last line says
  where the concurrency surface's is: `docs/spec/00-overview.md` § 2, not this file.
- **A `CoreCall` to a symbol with no registry row links only if `nvs_stdlib::symbols()` chains it in** —
  `address_of` is not enough, and the failure is a Cranelift panic at *run* time reading
  `can't resolve symbol nvs_core_script_spawn`, long after everything has compiled and every test
  in the crate has passed. `symbols()` walks `registry::CLASSES` and `CONSTRUCTORS`, so a member with
  a row is found for free; a rowless symbol — ADR 0077's two prepared link entry points, ADR 0006's
  `spawn script` and `await` — needs its own `.chain([...])` there beside `address`'s arm. Two
  registrations, not one, and the second has no compile-time gate at all.
- **The live graph carrier keeps the source object's descriptor, so ADR 0023 § 2's *unresolvable class* has no counterpart there until someone hands it a receiving table.** `decode` resolves a class by name and refuses one the program does not declare; `copy_graph` never resolved anything, because both sides of a `clone` are one program. At the isolate boundary they are not — `nvs-cli` compiles one unit per written path — so the rule had to be added rather than found: `copy_graph_into(value, Some(&resolve))` and `Live::admit`. Do not read a refusal in `graph.rs` as covering both carriers; the `Carrier` trait is the list of what they share.
- **A transferred call argument is released by the *landing block*, not by the normal edge.** `release_temporaries_since` skips a `TemporaryKind::Transferred` entry, so a lowering test asserting "the transferred value is never released" fails on the error path, where the frame still owes it: a callee that returned non-OK never took the reference. Assert per block — the call's own block for what the normal edge does, `inst.on_error`'s for what the throw does — and the pair reads as the bound it is.
- **`===` and `!==` do not exist**, and reaching for one in a `.nvst` is `E0232` on the operator
  rather than a type error you can read past: Novis keeps exactly one equality operator, `==`, which
  never converts either operand, so there is nothing for a second one to distinguish. A null test is
  `$x == null`.
- **A new `[limits]` key needs a row in `nvs_config::value::unit_of`, and without one the reader
  answers its *default* rather than failing.** `fatal_reserve_time` had a field on the typed
  `Limits` tree, a `DIRECTIVES` row and a `Ctx` reader, and a case asking for `"300ms"` still read
  50 ms. `Ctx` asks for a limit by its **bare** name, and `request::canonical` only prefixes
  `limits.` onto a name `unit_of` knows — so a key missing from that one `match` resolves as a
  top-level key, finds nothing, and every reader falls through to whatever it does when the
  directive is unstated. Nothing refuses: the tree accepted the value, the registry classed it, and
  only the number was wrong. The full roster a new `[limits]` key owes is the `Limits` field, the
  `DIRECTIVES` row, the `unit_of` arm and the `Ctx` reader; `unit_of`'s own doc calls itself the one
  table, which is the sentence to trust over the three files that look complete without it.
- **A new `[limits]` key needs four edits, and the one that is easy to miss makes the other three
  read as a silent default.** `max_script_depth` had its `Limits` field
  (`crates/nvs-config/src/tree.rs`), its `DIRECTIVES` row (`crates/nvs-config/src/directive.rs`) and
  its `Ctx` reader, and every written value still came back as the default: `Request::get` resolves a
  bare name to `limits.<name>` only when `nvs_config::value::unit_of` knows the leaf, so a key
  missing from that `match` has no block, never reaches `[limits]`, and the reader answers its
  default instead. The comment above `unit_of`'s `Duration` arm already says exactly this — it is
  worth reading before adding a key rather than after. The full roster is: the `Limits` field, the
  `DIRECTIVES` row, the `unit_of` arm, and the reader. Nothing fails to compile without the third.
- **ADR 0020 § 1's roster of resource limits is restated in four places, and three of them are
  prose no test reads.** Adding `max_output` as a limit whose breach reaches the tier-1 handler
  meant editing the ADR's own list (`docs/adr/0020-error-escalation-ladder.md` § 1, which also
  carried "call-stack depth is the *fifth* limit" — a running count of the kind
  `conventions.md` § *An ADR* forbids), `nvs_runtime::Limit`'s enum doc ("Three variants, because
  three limits are enforced... § 1 lists five"), and `Core\Fatal::onLimit`'s reference card `short`
  in `crates/nvs-stdlib/src/fatal.rs`, which enumerates them for the website. Only the enum's own
  `name()` arm is load-bearing, so nothing fails when the other three drift. `grep -rn "cpu_time"`
  over `docs/adr crates/nvs-stdlib/src` finds all of them in one call.
- **`unsafe_code` is `forbid` at the workspace root, so the first `unsafe` in a crate is a
  manifest edit before it is a code edit.** `-F unsafe-code` cannot be turned off by any
  attribute — `#[expect(unsafe_code)]` at the call site still fails with *usage of an unsafe
  block*, and the error's note names the command line rather than the lint table, which is the
  part that misleads. The fix is the shape `crates/nvs-config/Cargo.toml`
  already carries: replace that crate's `[lints] workspace = true` with `[lints.rust]` +
  `[lints.clippy]` copied verbatim from `Cargo.toml`'s `[workspace.lints.*]`, change the one
  line to `unsafe_code = "deny"`, and say in a comment above it which call needs it. Copy both
  tables or the crate silently loses every clippy lint the workspace sets. `nvs-cli` did this
  for ADR 0042 § 3's one `Mmap::map`.
- **A source path is read off the filesystem in more places than the front end, and the
  configuration snapshot is the one that bites.** ADR 0048's bundled executable resolves its entry
  and its whole `require` graph out of an appended payload, so the path handed to `nvs run` is
  synthetic — and `nvs_config::Snapshot::build` canonicalizes that same path through
  `trust::canonical` to key ADR 0104's `[[app]]` blocks, which refused the run with `E0605` long
  after the program had compiled cleanly. The fix is not to widen `trust::canonical`: a bundle is
  ADR 0048 § 1's single trust domain, and the only path an `[[app]]` block could legitimately key
  on there is the executable itself, which `run_run` now substitutes. Before changing a byte
  source, `grep -n 'canonicalize\|read_to_string' crates/` for the *other* readers — there were
  three, in three crates, and only one of them was in `nvs-hir`.
- **A `Known gap` paragraph in a module doc can outlive the gap, and it reads exactly like a
  decision rather than like a report.** `crates/nvs-types/src/defaults.rs`'s said a written
  `= null` stayed refused because "a type that admits both `null` and a `T` has no IR
  representation yet" — while `?int $absent = null` is a landed conformance case for a *local*,
  `?int $rank` is a parameter several `Core` cases already pass, and `?int $length = null` is a
  live `Core` default reaching `ConstArg::Null` through `core_lib`. The representation had landed
  underneath the paragraph, so the finding it produced (D25) arrived reading like a decision to
  overturn and was one missing match arm. One `grep` for the shape the paragraph says is
  impossible is the whole check — the same check the *a `loop-goal.toml` comment can be stale
  about the tree* bullet asks for one file up, because a sentence written as a reason is still a
  claim about the tree.
- **Making an operator throw is four edits, not one, and the codegen guard is the last of them.**
  Adding the zero-divisor guard to `emit_binop`'s float `/` built and then died at run time with
  `internal error: an arithmetic throw with no error edge`: `raise_arithmetic_error` leaves the block
  on `Inst::on_error`'s edge, and `nvs-ir`'s `lower/operator.rs` `fallible` match decides whether the
  instruction has one — its `BinOp::Div => ty == Ty::Tagged` arm names the *result* representation,
  so the float row had no edge to leave on. The full roster an operator that starts throwing owes is:
  the `fallible` arm in `lower/operator.rs`, the guard in `nvs-codegen`, the same rule in
  `nvs_runtime::helpers` for the tagged path a `mixed` operand takes (the two ends are separate
  implementations of one ADR sentence, and `float_arith`'s doc comment had written the old answer
  down as deliberate), and whatever `.nvst` cases used the old answer as an *instrument* rather than
  as a subject. That last one is the expensive half and no grep for the operator finds it: eight math
  cases read the sign of a zero with `1.0 / $z`, which is exactly the divisor that now throws, so they
  all had to move to `Core\Math::fdiv`. Grep for the *shape* — `1.0 /`, `/ 0.0` — not for the feature.
- **Refusing a construct that already parses breaks the tests that used it as a *fixture*, not as a
  subject.** `<>` was a row in `operators_longest_match_wins`, and `new class { … }` was the body two
  `casing.rs` fixtures used to prove the casing pass descends into a nested declaration; all three
  called a helper that asserts the parse was clean, so they failed with the new code rather than with
  anything about the shape they test. Before writing a refusal, grep the crate for the spelling — the
  fix is to move the fixture to the helper that collects both halves (`casing.rs`'s `parse_and_check`)
  or to hand the shape to the new test outright, never to weaken the new refusal.
- **A new refusal is a hypothesis until the whole conformance tree has run it, and the tree is where
  the counterexample lives.** `E0787`'s first shape — refuse *every* write to a property with a
  `get` hook and no `set` — read as obviously right, passed its own new case, and was refuted in one
  seven-second `./target/debug/nvs.exe test tests/conformance` by
  `lang/the-inout-marker-is-required-at-both-ends-and-replaces-every-ampersand.nvst`, whose class
  arms its own `get`-only property from its constructor. That case is the language's own answer to
  "how does such a property ever hold a value", and it turned the rule scope-shaped (only the
  declaring class writes it) rather than blanket. So: write the refusal, run the whole tree, *then*
  write the case that pins it — a case written first only pins the rule you already believed.
- **A qualifier written on an element of an array literal is gone before any call-site rule looks at
  it.** `check_array_literal` (`crates/nvs-types/src/expr/literals.rs:718`) joins nothing: with no
  expectation on it a literal infers `array<mixed>`, so `Core\Json::encode(["token" => $secret])`
  compiles while `Core\Json::encode($secret)` and a declared `array<secret string>` are both refused
  by the same rule. That is ADR 0033's unmodelled container axis, not a hole in the sink — the bit is
  lost at the literal, and a call-site walk could not recover it one variable later anyway. So a
  refusal written over the argument's *type* is right and it is not the whole of § 4's "anywhere in
  the value it walks": probe the container spelling with a scratch `.nvs` before writing the case
  that claims it, or the case pins a refusal that never fires.
- **A diagnostic reported at a *read* fires a second time on a declaration another diagnostic
  already refused, and the two cases it turns red look unrelated to what you wrote.** `E0792` — a
  class constant whose value folds to nothing — was written at the read rather than the
  declaration, deliberately, so that declaring an unfoldable constant and never naming it stays
  legal. It then reported over the top of `E0246` (`const LIMIT = 9;` has no type, so it folds to
  nothing *because* it was already refused) and over `E0727` in the secret-payload case (`const
  secret bytes BLOB = "b";` cannot fold because ADR 0009 § 1 gives the language no `bytes`
  literal). Neither failure named the constant folder. The rule that falls out: before reporting on
  a *use* of a declaration, ask which declared types can never have reached this point cleanly —
  `mixed` here is "already `E0246`" and `bytes` is "no literal exists to write" — and return early
  for each with the other diagnostic named, because a read is not where either mistake is worth
  saying twice.
- **A hand-built forwarding call must forward the whole calling convention, and the two parameter
  shapes that do not survive one abort inside `nvs-runtime` rather than reporting.** ADR 0027's
  `(...)` lowers to a thunk that passes its own parameters straight through, which is right for
  every ordinary parameter and wrong for exactly two: an `inout $x` wants an address where the
  thunk has an `int`, and a variadic tail wants the collected array where it has the first element.
  Both compile, both check, and the variadic one then dies as `misaligned pointer dereference` at
  `nvs-runtime/src/array.rs`, naming a slot dereference with nothing pointing back at the callable
  that built it. A scratch that prints the right answer for the ordinary shapes says nothing about
  these, so write one case per *declaration* shape the callee can have — not per call site — and
  read `nvs_types::signatures::MethodSig`'s own field list for what those shapes are.
- **Hand-built IR that needs a `Ty::Tagged` operand emits the bare constant and *then*
  `InstKind::Tag`; the `Ty` on the instruction is not a cast, and skipping the `Tag` fails in
  Cranelift naming nothing you wrote.** `low.emit(b, Ty::Tagged, InstKind::ConstNull)` compiles,
  lowers, and then aborts the run with `internal error: cranelift rejected the code generated for
  \`C::current\`: should be implemented in ISLE: inst = \`v25, v26 = isplit.i64 v46\`` — the
  `isplit` is the 128-bit tagged pair being taken apart, and neither the message nor the
  instruction it names appears in the lowering that caused it. `convert`'s `(_, Ty::Tagged)` arm is
  what produces one for every source-level widening, so a synthesized body is the only place the
  step has to be written by hand; every other `InstKind::ConstNull` under `lower/` is `Ty::Null`
  for exactly this reason. The first place it bit was a synthesized `new LogicError(…)`, whose
  `previous` parameter spec § 10 types `Throwable|null`.
- **A crate's own `# Known gaps` list can be stale about that crate's body, and the body is the
  rule.** `nvs_types::layout`'s module doc said "a promoted constructor parameter claims no slot
  yet", and `nvs_types::derive`'s gap 1 cited it as half the reason a promoted parameter could not
  be a `#[Json\Derive]` field — while `own_properties` (`crates/nvs-types/src/layout.rs:313`) had
  been giving one an ordinary slot, and `signatures::record_promoted_properties` had been recording
  it as a property, for some time. The whole of D8 turned out to be the derive walk's own
  `for member in &decl.members` skipping past the constructor; nothing in `layout` or `signatures`
  needed touching. This is the *doc* twin of the `loop-goal.toml` traps above: a `Known gaps`
  bullet is status, an ADR is a decision, and status goes stale silently. One scratch `.nvs` under
  `.agent-tmp/` proving the gap is still there costs one call and is the whole check.

## Divergences and refusals already pinned

What a session found when it pinned a `Core` member against its PHP twin, one bullet per finding,
in the order they were pinned. These lived in the plan's `Open now` field, which grew from 4,106 to
44,160 bytes carrying them and was shipped into every session's orientation pack whole — 48% of
that pack, for findings about members most sessions do not touch. They are here because this is the
file with a bullet-level `[context]` selector: a goal names the handful its file set needs (`python
tools/playbook.py --goal --min 3` picks them), and the rest stay one `grep` away instead of costing
every session. Nothing below was reworded on the way.

- **A member answering a value rather than an array has no key rule to diverge over at all**, which
  is why `min`/`max` and `reduce` match PHP over a map as readily as over a list; what parts them
  from their twins is PHP's loose comparison — `min([0, "a"])` is `0`, and two numeral strings
  compare numerically — and `array_reduce`'s callback taking exactly two arguments where Novis's
  takes the key third.
- **Four other twins sit outside the key rule**: `array_unique` renumbers nothing at all, so the
  divergence there is `SORT_STRING`'s comparison by *spelling*; `array_count_values` names a bucket
  through the same key normalization `countBy` uses, so that pair parts only where PHP
  warns-and-skips a value `countBy` refuses; `ksort` reads a numeral *key* as a number where ADR
  0007 § 5 makes every stored key a `string` compared bytewise, so `sortByKey` parts from it over a
  numeral or mixed-key subject and a `comparator` is the way back; and `array_fill` takes a start
  index `fill` drops, so every non-zero start is `Core\Arr::fillKeys` over the keys the caller
  wanted.
- **`Core\Str` and `Core\Regex` are closed, `Core\Math` is down to the `gmp` pair alone, and
  `Core\Path`'s three are now the largest block of the 8.** `Core\Math`'s settled pairs say what
  shape the rest take: nothing there has a key rule, so a member either agrees with its twin
  outright or parts over a *tie*, a *conversion*, a *guard* or a *repair*.
- **PHP's two-argument `min` answers its second argument on a tie and its `max` answers its
  first**, where `nvs_stdlib::math::pick` answers the first to both — visible wherever two equal
  values are distinguishable, `min(1000000000000000000, 1.0e18)` being the sharpest — and
  `f64::total_cmp` separates `-0.0` from `0.0` where PHP's `<` calls them equal.
- **`Core\Math::mod` throws on a zero divisor where `fmod` answers `NAN`**, spec § 3 making a
  division by zero a throw wherever it appears, while the IEEE *domain* rows — an infinite
  dividend, either operand a `NAN` — stay at IEEE's answer and agree; `intDiv` agrees with `intdiv`
  on every row including both refusals.
- **The transcendental pairs are closed and they agree with their twins outright**: `sqrt`, `exp`
  and `log` leave the *argument's* domain unguarded, so a negative root and a negative logarithm
  are `NaN`, a zero logarithm is `-INF`, `exp` overflows to `INF` and underflows to the smallest
  subnormal, and every one of those renders byte-identically on both sides; `hypot` and `atan2`
  agree on the four signed-zero quadrants, on the infinite ones, and on the magnitude where
  `sqrt($x * $x + $y * $y)` overflows and `hypot` does not.
- **The one guard this added is `log`'s *base***, which PHP has and Novis did not: a base not greater
  than zero is a `Fault::thrown` where PHP raises a `ValueError`, and base `1.0` is `NAN` on both
  sides rather than the infinity `ln($n) / ln(1.0)` would answer.
- **The base pair and the two predicates are closed too, and each parts from its twins over a
  repair rather than over arithmetic.** `toBase`/`fromBase` agree with
  `decbin`/`dechex`/`decoct`/`base_convert` and their from-halves on every non-negative number
  written in digits the base holds — the same lowercase alphabet out, either case in, and the same
  `ValueError`-versus-`Fault` refusal outside base 2 to 36 — and part at the four inputs PHP has no
  failure mode for: a negative `$n` is `-ff` where `dechex` writes the two's complement
  `ffffffffffffff01` and `base_convert` drops the sign as a character it has no digit for, a digit
  past the base is a throw where `hexdec("beefy")` is `48879`, an empty string is a throw where
  `bindec("")` is `0`, and an answer past `int` is a throw where `hexdec("ffffffffffffffff")`
  widens to a `float`. `isNan` and `isFinite` agree with `is_nan` and `is_finite` outright, and
  PHP's third predicate is the two of them folded, so `is_infinite` is neither — asserted as a
  *partition* over a table rather than row by row.
- **`toRadians` and `toDegrees` now compute PHP's own expression rather than the accurate one**,
  and that is the only place in `Core\Math` where a spelling was changed to match a twin:
  `($degrees / 180.0) * PI` replaces `f64::to_radians`'s multiply by the correctly rounded `PI /
  180.0`, and `($radians / PI) * 180.0` replaces `f64::to_degrees`. The std forms are the more
  accurate — over the 3,600 tenths of a degree in a turn they are closer to the true value 851
  times against 118 — and the difference is at most one ulp, invisible at both languages'
  precision-14 rendering. It is visible through `==`, which is exact over `float` (ADR 0090): under
  PHP's spelling a whole-degree round trip lands back on its angle for 19 of 22 sampled angles and
  under the std one for 13, so the ulp is what a ported program comparing a round trip actually
  sees. AGENTS.md's priority 2 — PHP-compatible *observable* behaviour — is what decides it, and
  the accuracy spent is stated in `nvs_stdlib::math`'s own doc comments at both members.
- **`Core\Math::gcd` and `::lcm` have no callable twin on either leg**: neither the Windows `php`
  nor WSL's has `gmp`, so `gmp_gcd`/`gmp_lcm` are undefined functions and an oracle case for those
  two has to compute its expectation with an explicit Euclidean loop in PHP or be left out of the
  count.
- **Novis's `float` rendering is PHP's**, precision 14 with trailing zeros trimmed — `sqrt(2.0)`
  prints `1.4142135623731`, `exp(-745.0)` prints `4.9406564584125E-324` and `0.1 + 0.2` prints
  `0.3` on both sides — so a `Core\Math` oracle case may echo a float directly and needs no
  formatting, but never a `NAN`, which PHP 8.4 and later warn about coercing to a string.
- **`Core\Str`'s twins part from PHP over a *unit* before they part over anything else**, and
  `slice`/`replaceRange` is the worked pair: `nvs_stdlib::granularity::DEFAULT` is
  `Unit::Grapheme`, so a member's `int $offset` and `?int $length` count clusters where `substr`
  counts bytes and `mb_substr` counts code points. Inside ASCII with no carriage return all three
  units coincide, which is why the whole offset/length sign table — 10 offsets × 9 lengths, both
  members, plus the `null` length, the empty subject and the deletion spelling — agrees outright as
  one `--ORACLE--` file, and the three-way split is a second `--ORACLE-DIVERGES--` file with a
  frozen `--EXPECT--`, because the `php` on the Windows `PATH` has no `mbstring` and `mb_substr` is
  not callable at all. Both members read one `window` helper, and putting a window's own slice back
  into it reproduces the subject on all 90 ASCII cells and all 72 multibyte ones — **PHP's pair
  holds that identity too**, which the `replaceRange` doc comment used to deny and no longer does.
- **A frozen `--EXPECT--` must never render a decomposed cluster**: `e`+U+0301 and U+00E9 are
  indistinguishable in an editor and are different strings, so a divergence case echoes
  `Core\Encoding::toHex($s as bytes)` for those cells instead.
- **`Core\Str::before`/`::after` and `::compare` are closed, and each parts from a *twin* rather
  than from a rule.** Both cut members exclude the needle where `strstr` keeps it, so the port of a
  program that wanted PHP's shape is `$needle . Str::after(...)`, and an absent needle is `null`
  where `strstr`, `stristr` and `strrchr` all answer `false` — a difference in how "no answer" is
  spelled, folded to one sentinel on both sides rather than a divergence.
- **`{last: true}` is `strrchr` only at a one-character needle**: `strrchr` reads the needle's
  first character and nothing else, so `strrchr("a::b::c", "::")` is `":c"` where the member
  answers `"c"`, and `strrchr("banana", "an")` answers `"a"` for a needle that never occurs at all;
  at the empty needle `strrchr` fails outright where `strrpos` answers the subject's length, which
  is the member's answer too. The honest twin past one character is `strrpos` plus a `substr`, and
  both halves rejoin through the needle to the subject on every cell where it occurs — counted, on
  both sides, under each occurrence rule.
- **`stristr` has no twin**, because the pair's one option is `{last}`; the port is
  `Core\Str::indexOf($s, $n, {caseInsensitive: true})` plus a `slice`, needle-inclusive.
- **`compare` folds four twins into two independent options and the fold is exact over ASCII** —
  all 144 ordered pairs of a table, in all four corners — but only after a *sign* normalization,
  because PHP's four disagree with each other on magnitude: 8.2 narrowed `strcmp`,
  `strnatcmp`/`strnatcasecmp` were always -1/0/1, and `strcasecmp` on 8.5.9 still returns the byte
  difference, in 84 of those 144 cells. `compare` is always one of three literals, and it is the
  only ordering two strings have at all, since `<` over two `string`s does not lower.
- **`reverse` and `chunk` are closed, and on those two the unit is the *whole* disagreement**,
  since neither takes an offset: `strrev` and `str_split` walk bytes, so their answer for any
  subject outside ASCII is not well-formed UTF-8 and therefore not a value Novis can hold at all —
  which is why neither member is offered a compatibility spelling back to its twin, there being
  nothing there a port could want. Over ASCII both agree with their twins outright, and what each
  agreeing file pins is the *properties* rather than the rows: `reverse` is its own involution over
  a table, preserves length, is the identity on exactly the palindromes, and turns a concatenation
  inside out on all 100 ordered pairs — PHP holding every one of those byte-wise. `chunk` folds
  three twins, and `chunk_split` is not a fourth member but `Core\Str::join(Core\Str::chunk($s,
  $n), $end) . $end`, which reproduces it on all 30 cells *including the empty subject*, where
  `chunk` gives no chunks and `chunk_split("")` is still the separator alone; a zero size is
  refused on both sides, PHP's `ValueError` against a catchable `Fault::thrown`.
- **The counted claim that survives a unit change is not the discriminating one**: pieces rejoined
  with nothing between them reproduce the subject under all three units, so what parts the members
  from their twins is the piece *count* — `Core\Str::chunk($s, 1)` answers `Core\Str::length($s)`
  pieces where `str_split($s, 1)` answers `strlen($s)` of them.
- **`replaceAll` and `wrap` are closed too, and each is a fold whose two twins disagree with *each
  other* rather than with Novis.** `replaceAll` folds `str_replace`'s array form and `strtr`, which
  read one table two ways: `strtr` scans the subject once and takes the longest needle matching at
  each position, never rescanning what it produced, where `str_replace` runs each pair over the
  whole subject in turn and feeds every earlier replacement to every later pair.
- **The member is `strtr`**, ADR 0063 R20 being why, and 19 cells of a 48-cell table of cascades,
  swaps, prefix pairs and a growth the next pair re-matches are where the two twins part. The
  cascading reading is not lost and is owed no option: `Core\Str::replace` applied pair by pair in
  a `foreach` reproduces `str_replace`'s array form on all 48, and the property that decides
  between the two readings is *counted* rather than read off a line — the same table written back
  to front answers the same thing on 48 of 48 cells one-pass and on 29 in sequence.
- **`replaceAll` is also the one `Core\Str` member the unit question does not reach**: it matches a
  needle as a byte sequence, and a valid UTF-8 needle can never begin inside a character, so it
  splits a decomposed cluster exactly as `strtr` does — `"e"` over `"e\u{301}ta\u{301}t"`, a
  subject `Core\Str::length` counts as 4 — and there is no divergence file to write. Its one
  boundary left to the conformance case is the *empty needle*, which both skip but PHP warns while
  skipping, and a warning on stdout is not a difference in the result.
- **`wrap` runs `wordwrap`'s own algorithm** with PHP's third and fourth arguments as `{breakWith,
  cutLongWords}`, so 8 subjects × 8 widths under both cutting readings, a multi-character break and
  the default newline agree outright, the rule that a break already present in the subject *resets
  the line* included; both refusals agree as well — an empty break and a zero width that must cut,
  `ValueError` against a catchable `Fault::thrown` — while a zero width *without* cutting is an
  answer on both sides and breaks at every space. What the cutting option is worth is counted: 352
  of 352 lines are within their width when long words may be cut, and 122 when they may not.
- **`wrap`'s width counts clusters where `wordwrap` counts bytes**, so its multibyte half is still
  owed as an `--ORACLE-DIVERGES--` file — the last `Core\Str` divergence file not written, and the
  one item of the closed section that is not counted.
- **`codePoints` and `fromCodePoint` close the section, and they are the one pair whose unit the
  *member* chose rather than one the class default imposed**: `codePoints` is `Unit::CodePoint` on
  purpose, `nvs_core_str_code_points`'s doc comment being the home of why, so it parts from
  `Core\Str::length` on exactly the subjects where a cluster holds more than one scalar value —
  four of seven sampled, `\r\n` among them, which is the one place inside ASCII the two units
  disagree. Inside ASCII the byte-wise half of PHP's pair answers the same question, so `str_split`
  plus `array_map("ord", …)` is `codePoints` and `chr` is `fromCodePoint` over the whole 0–127
  table: the 128 encodings render as bytes sixteen to a line, the round trip holds on all 128, each
  answer is exactly one byte, and both members are injective there — 128 code points giving 128
  distinct strings.
- **Past 127 the pair parts in both directions and the reason is the same one read from either
  end.** `ord` reads a byte and `str_split` cuts between them, so `array_map("ord",
  str_split("é"))` is `[195, 169]` where `codePoints("é")` is `[233]`; and `chr` constrains its
  argument with `% 256`, which makes `chr(55296)`, `chr(1114112)` and `chr(0)` one byte and the
  function not injective past 255 — with no diagnostic. `fromCodePoint` refuses instead: a
  surrogate in `55296..=57343` and anything past `1114111` are a catchable `Fault::thrown`
  (`nvs_stdlib::str::scalar_value`), never a substituted U+FFFD, ADR 0009 § 3's
  checked-not-repaired rule reaching a code point exactly as it reaches a buffer.
- **PHP is retreating from the wrap from its own end**: on 8.5.9 `chr()` *deprecates* an argument
  outside `0..255`, and the notice it prints to stdout is itself why that half cannot be an oracle
  leg. What survives the unit change is the round trip, which holds on every multibyte subject too,
  `fromCodePoints` being the fused spelling of `fromCodePoint` applied element by element.
- **`Core\Regex`'s two twins are closed, and each parts from PHP over a *shape* rather than over a
  result.** `quote` and `preg_quote` escape different sets outright — 18 characters here,
  `#$&()*+-.?[\]^{|}~`, against 22 there, `!#$()*+-./:<=>?[\]^{|}` — because `&` and `~` are meta
  to the Rust engine's character-class set operators and not to PCRE, `!:<=>` are meta to neither
  engine and PCRE's launderer escapes them anyway, and `/` is escaped only because a delimiter was
  handed in, which ADR 0056 § 5 removed. The outputs are therefore not comparable at all, and what
  is counted instead is the property both launderers exist for: over the whole 95 × 95
  printable-ASCII grid a quoted character matches itself and nothing else, the same holds over a 20
  × 39 grid of metacharacter-carrying literals against the strings their unlaundered reading would
  have reached, every quoted literal is still found *inside* a larger subject, and the one
  row-level agreement left is that neither launderer touches a word character.
- **`Core\Regex\Match::groups` is `preg_match`'s `$matches` under `PREG_UNMATCHED_AS_NULL` and not
  under PHP's default**: ADR 0063 R11 removed the `PREG_*` constants, so one of the two readings
  has to be the only one, and the default's trimming of *trailing* unmatched groups plus its `""`
  for the ones in the middle conflates "not declared", "declared and did not participate" and
  "participated and captured nothing" — the first two being exactly what `group`'s
  throw-versus-`null` split is built on. Over twelve rows the two readings part on six of them, six
  entries short in total, and the flagged one agrees with `groups()` on every key, every value and
  the order they arrive in, a name before its number.
- **`Core\Bytes::pack`'s refusals are closed, and they are the first block `--errors` has
  emptied**: all six argument sites and all three range sites now have a case asserting the
  *message* each names rather than only that something threw, which is what the older `pack` case
  reported. What the two cases add past their rows is counted. Every code accepts exactly one class
  of argument, so a 15 × 7 grid of code against argument kind partitions the roster into 3 buffer
  codes, 7 integer, 4 float and `x`, which reads no argument at all and therefore accepts none — a
  code that grew a conversion of its own, an integer field taking a numeral string or a float field
  taking an `int`, still looks right on its own line and fails there. An integer field's accepted
  range is the **union** of its width's signed and unsigned ranges, so `-1` and the unsigned top
  write the same octets and the signed bottom writes what its own negation writes, at every width,
  while the value one past either end is refused on both sides.
- **The 64-bit codes are the one width whose range refusal is unreachable**, and that is not a gap:
  `int` and `uint` together are exactly what `J` and `P` accept, so a value past the bound is
  `E0429` at the call and never becomes an argument — `-9223372036854775808` is not a writable
  literal at all, and the field's low end is spelled `-9223372036854775807 - 1`.
- **A 32-bit float field is the one place `pack` accepts a lossy write**: precision is what a
  caller chose when they wrote four octets, so `0.1` rounds silently, while a finite value that
  would round to an infinity throws and the same argument is an ordinary field under the 8-byte
  code.
- **`Z` differs from `a` by exactly the octet it reserves**, asserted as an agreement over five
  widths rather than as a row, and `a0` over the empty argument is the sharpest accepted cell
  against `Z0`, which has nowhere to put its NUL.
- **`bytes.rs`'s reachable list is empty now, and so is `Core\Encoding`'s decoder half.**
  `unpack`'s two bounds, `Core\Bytes::at` and `Core\Bytes::fill` each name the last octet they read
  beside the first they cannot, and each of `fromBase64`, `fromBase64Url`, `fromBase32` and
  `fromHex` says which of its reasons refused and at what offset — four reasons for base64 and for
  base32, two for hex, and which one a text gets is decided by the earliest thing wrong with it, so
  a space at offset 4 of eight base32 symbols is the alphabet's refusal while a space that also
  makes the text nine long is the group's. What the two cases add past their rows is counted: a
  code reads its whole width or none of it at every width in the table, an index and its negative
  name one octet, all 256 values a `fill` byte admits are one octet read back, and the offset a
  base64 refusal names is the offending position at each of eight in turn rather than a constant.
  Two boundaries are the compiler's rather than the member's — `fill`'s low end is `E0401` on a
  `uint` parameter, as `pack`'s 64-bit range is `E0429` at the call — and the operand a decoder
  quotes back is bounded at 32 characters, asserted on both sides of that bound.
- **`--errors` names no assertable site in `str.rs` or `encoding.rs` at all now.** Spec § 7's
  `encodeText` says which character the charset has no spelling for, its code point and the byte
  offset it sits at, and `decodeText` the offset the first unreadable sequence *starts* at — there
  being no character to quote when the whole point is that these octets spell none. Spec § 1's five
  say what they stopped at: `at`'s index against the length it is outside of, `chunk`'s size,
  `countOf`'s needle, and which of `wrap`'s two options, whose width pair is the sharper because
  the same width of 0 is an answer or a throw depending only on `cutLongWords`.
- **`Core\Str::at`'s message was the one in `nvs-stdlib` missing the `()` every sibling writes**
  and now has it. What the two cases add past their rows is counted. `Latin1` is the one charset
  with no refusal at all, so it reads every one of the 256 octets and each returns as the octet it
  went in as, while `Ascii` reads exactly half — the same bound stated as a partition rather than
  as two rows — and `isValidText` answers `false` at exactly the octets `decodeText` throws for on
  all 256, which is what makes one the other's question with somewhere to put the answer.
- **Both encoding members count their reported offset in bytes**, asserted over eight positions in
  turn on each side, which is the one thing the two messages have to agree about since a caller
  uses one to index what it handed the other; and `encodeText`'s quoted operand is bounded on both
  sides of 32 characters, ADR 0088's register being why it is bounded at all. `at`'s accepted set
  is exactly the 2n indices from -n to n-1 and nothing else, over sixteen spanning both bounds, and
  the length it reports agrees with `Core\Str::length` on six subjects whose byte and cluster
  counts diverge in different places.
- **Two spellings a case reaching for these cannot use**: a source-declared `Core\Charset`
  parameter does not unify with the registry's own enum type and fails `E0401` naming the same type
  twice, so the enum is written at each call site; and a message quoting a C1 control back cannot
  be a frozen `--EXPECT--`, so U+0080's row asserts the printable half. Both are playbook bullets.
- **`path.rs` holds no assertable site at all now, and neither does `Core\Time`'s rendering half.**
  Spec § 8's `withExtension` refuses four ways, and because two of a call's arguments can be wrong
  at once, what the case pins past the four sentences is the *order* they run in: the extension is
  checked before the path, and inside the extension the empty one beats the dot beats the separator
  — which is why `withExtension('/', './a')` reports a dot whose suggested replacement the next
  check would itself refuse. What it adds past its rows is counted once per argument. Twelve
  extensions against one path leave four accepted, the dot being refused only at position 0 — so
  `tar.gz` and `txt.` are usable extensions and `.txt` and `.` are not, the second of those being
  the sharpest cell because the extension it names instead is the empty one the *first* check
  refuses — and every refusal but the empty one quotes what it was handed. Twelve paths against one
  extension leave the nameless ones exactly the six roots, a *trailing* separator not being one,
  each naming the path back.
- **Spec § 4's three `format` members read one pattern compiler** (`nvs_stdlib::cldr`) and each
  wraps its refusal in its own name, which is the half of the sentence the neighbouring § 4 cases
  never asserted: an unknown letter, an unterminated quote and `V` at any count but two are one
  grammar spoken in three names. What the two zone-free halves do not share is the narrowing, which
  refuses a field rather than filling one in because every value they could invent — midnight, the
  process's own zone — is a wrong answer stated confidently, and it is asserted as a **partition**:
  over the 16 fields the subset carries, no field is accepted by both `Date` and `TimeOfDay`, and
  the three neither takes are exactly the zonal ones. Over the 52 ASCII letters exactly 15 are
  fields and 36 of the 37 refusals quote the letter they were handed, the odd one out being `V`, a
  pattern letter at the wrong count and saying so. The compile step runs before either narrowing,
  so an unreadable letter beats a field the value cannot carry.
- **The unknown-zone sentence is spelled three times, each behind the name of the member that would
  speak it, and only `Zone::of`'s is reachable** — the other two read an id this crate wrote
  itself. What that case adds past the sentence is counted twice. Over fourteen ids the six
  accepted are exactly the region names: a sign at position 0 is refused whichever way it leans,
  because that is `fixed`'s spelling and R20 leaves one way to build one value, while `Etc/GMT+5`
  is an ordinary name whose sign means the *opposite* of what a sign means in an offset — and every
  refusal equals the sentence rebuilt from the id it was **handed** rather than the canonical one,
  the lookup being case-insensitive, so `europe/berlin` is accepted and stored as `Europe/Berlin`.
  The second count is the premise the two unreachable siblings rest on: all seventeen `DateTime`
  members that read the zone slot answer for every zone, and each fixed offset rebuilt out of the
  `DateTime` it was stored in is worth what it went in as — which is where the round trip could
  actually break, a fixed zone's id being rendered rather than looked up.
- **`Core\Time::parse` throws two of § 10's classes and which one is decided by which argument was
  wrong**: a pattern is written by the call site, so a bad one is a `LogicError`, while text
  arrives from elsewhere, so text that does not match a well-formed pattern is a `ParseError`. The
  two tables are asserted as a *partition* — not one of seven bad patterns lands in `ParseError`,
  not one of six bad texts in `LogicError`, and every sentence on both sides is wrapped in the
  member's own name.
- **One refusal crosses that split, and it is the pattern that names a zone**: `format` accepts
  every zonal field and shares the compiler, so only the reader can refuse it, and a *pattern*
  mistake therefore arrives as a `ParseError`. The compile step runs first, so a call with both
  arguments wrong reports the pattern. A calendar refusal — an impossible date, an hour past 23 —
  is `jiff`'s own wording and is asserted by class and by the name in front of it rather than
  frozen.
- **`Core\Time::fromIso` refuses in `ParseError` too now, and not in the bare `RuntimeError` a
  plain `Fault::thrown` gives** — the one behaviour a session changed here rather than only
  asserting. That member's single argument is text that arrived from somewhere else, so the rule
  `parse` splits its two classes by decides this one outright, and § 10's own gloss on the class is
  "input did not match a format this code declared". The sentence past the member's name is
  `jiff`'s, because it says which component it stopped at, so it is asserted by its class and by
  the name in front of it rather than frozen — counted over fourteen texts, of which exactly the
  five carrying their own offset are accepted, all five at one epoch second, and all nine refusals
  wrapped in the member's own name. The bound is one character wide, the same civil time being
  refused without an offset and accepted with one; and the five accepted spellings share a *second*
  rather than a point, since a subsecond is read and kept and `toEpochSeconds` is what truncates
  it.
- **`--errors` now names no assertable site in `math.rs` or `time.rs` either, and `Core\Math`'s
  only guarded arguments are `log`'s base and `format`'s decimal count.** What the two cases add
  past those two sentences is counted. Over the eighteen members whose argument and whose answer
  are both a `float`, one `NaN` is answered by all eighteen and refused by none — the *argument's*
  domain is IEEE's throughout, which is what leaves `isNan` and `isFinite` something to ask about —
  while that same value is refused by `sign`, which would otherwise hand back a side of zero the
  caller goes on to branch on, and by `format`, which has no digit string for it. `log` is one of
  the eighteen and answers the `NaN` with the rest; its base arrives through an *option* rather
  than a position, which is why the sweep cannot see it, and the bound is named on both sides: zero
  is refused whichever sign it carries, the least positive `float` — one subnormal step above it —
  is accepted and answers a ratio of logs, and base `1.0` is inside the guard and is `NAN` rather
  than an infinity. `format`'s cap is stated the same way, a hundred decimals accepted against the
  hundred-and-first refused, and its sentence quotes the count it was *handed* rather than its own
  constant twice — asserted at 101, 200 and 1000 in turn — while the accepted count really is the
  fraction's width, one more decimal being exactly one more character over six subjects whose exact
  binary value terminates before either count.
- **`Core\Json`'s two `thrown_as` and `Core\Regex\Match::group` are closed, and two messages
  changed rather than only being asserted.** What `Core\Json::encode` refuses is a partition over
  the *tags* rather than a list: eleven values with a JSON spelling render — both array shapes
  among them, a list as an array and a string-keyed map as an object, and a `decimal` written as
  ADR 0054's exact number rather than through a `float` — while the five without one all throw in
  `LogicError`, the class saying the value was built by the program rather than handed to it by the
  world, and each is wrapped in the member's own name. The three sentences are frozen and two of
  them are new here: a non-finite `float` is quoted the way `echo` spells it, `NAN` and `INF`
  through `nvs_runtime::php_float_to_string` rather than Rust's `inf`, and a value whose tag has no
  spelling is named by `Tag::describe` — ``a `bytes` value has no JSON encoding`` — where it
  reported a tag *number* only this crate can read, that arm being where a `bytes` argument lands
  as `Encodable::text`'s own doc comment already said.
- **`decodeAs<T>`'s codec refusal is the same rule read from the other end, and it is asserted as
  an agreement**: ADR 0071 § 5 makes one attribute decide both directions of the wire, so over
  three classes declaring the same two fields the two members answer alike on all three and exactly
  one participates. Two claims sit past that — the codec is read *before* the document, so one
  unreadable text is a `LogicError` for a class with no codec and a `ParseError` for one with, and
  each refusal quotes back the class it was handed rather than a constant.
- **One wart is left there**: a class carrying the attribute and declaring *no* field has an empty
  codec, so both members refuse it with the sentence that says it does not carry the attribute at
  all — the message is wrong about why, and whether a fieldless class should encode as `{}` instead
  is the question behind it.
- **`Core\Regex\Match::group` splits three ways over a pattern's whole group set**, where PHP's
  absent array entry folds the refusal and two of the answers together: of the eight keys
  `groups()` reports, four are reached, two participate and capture nothing and two are declared
  but never reached, and `group` agrees with `groups()` on every one of them. Everything outside
  that set is a `Fault::thrown` and so a `RuntimeError` — where the call-site-argument rule that
  made `Core\Time::parse`'s bad pattern a `LogicError` argues for `Logic`, which the case asserts
  as it stands rather than as that rule implies — and the sentence is rebuilt from the key it was
  handed at three keys in turn. The bound is one number wide, group 4 being the last the pattern
  declares and group 5 the first it does not, and a name and its number reach one group at all
  three named ones, as do the `int` and `string` spellings of a number.
- **`Core\Arr::average`'s one catchable refusal and `Core\Out::capture`'s are closed, and the
  second of them changed the check rather than only asserting it.** `average` divides an exact
  total by the entry count as a `decimal`, so the only subject it has no answer for is one whose
  rounding carries past ADR 0054 § 1's 96-bit mantissa — a knife edge rather than a half-line,
  which is why the bound is named on both sides by the same division over the same count: seven
  tenths of `2^96 - 1`, read one place further out, lands on exactly that mantissa with a remainder
  that rounds away, while the total one tenth below has a digit to spare and answers at a wider
  scale and the one above rounds up and answers at a narrower one.
- **What is refused is a property of the *quotient* and not of the total**, so it is counted rather
  than read off a line: one total against fourteen counts — the entries past the first are zero, so
  every prefix carries the whole of it — and exactly one of the fourteen has no answer, while seven
  of the other thirteen divide inexactly and round without complaint, which is why "inexact" is not
  the rule here. `Core\Arr::sum` agrees on all fourteen, which puts the refusal in the division and
  not in the fold that feeds it, and the same subject in the `float` arm answers, only the exact
  arm having a range to run out of. The neighbouring `arr.rs:4098` `fatal` — more entries than a
  `uint` counts — is unreachable and owed nothing.
- **`Core\Out::capture`'s `through` now refuses by class rather than by objecthood.** A `callable`
  is opaque as to signature (ADR 0031), so nothing static stands between § 12's option and what its
  closure answers, and a check that asked only whether the answer was an *object* let a foreign one
  through under the member's declared `Core\Cli\Text` return — a claim about the value that was not
  true, and one that failed much later, wherever the carrier was next read. It compares the
  rendered class name now, and the message names what it got: a non-object by its type, where it
  used to report a tag *number* only this crate can read, and an object by its class. The claim is
  a partition and is counted: of the eight answers a `through` can give exactly the identity is
  accepted, all seven refusals are wrapped in the member's own name, and nothing the refusing seven
  captured reaches the program's output, the capture level being closed on both edges.
- **`--errors` names no catchable site a case can still assert anywhere in `nvs-stdlib`.**
  `random.rs:333` was reachable after all: `nvs_runtime::affordable` refuses only a size past
  `isize::MAX`, so every count at or below it that the allocator cannot serve reaches the *second*
  check, and the bound between the two is one count wide with a different sentence on each side of
  it.
- **`Core\Random::token` ran the same two checks and then drew infallibly**, which is the one
  behaviour a session changed here rather than only asserting: `vec![0; n]` and
  `String::with_capacity` abort the process when the allocator refuses, so a count the seam allowed
  and the machine could not serve killed the process — exit 127, nothing on stderr, nothing
  catchable, and every in-flight request with it. Both buffers are reserved through
  `try_reserve_exact` now and report `bytes`'s own sentence. The seam is asked about the *answer's*
  width rather than the draw's, the token being twice as long as its entropy, so `token`'s bound
  sits at exactly half of `bytes`'s — which is what the case counts rather than reads off a line:
  `token($n)` answers as `bytes(2 * $n)` does on all six rows of a table spanning served, the
  allocator's refusal and the seam's, and not one of those counts is past `bytes`'s seam where
  exactly two are past `token`'s.
- **The four count-shaped refusals agree** as well — each answers one step inside its bound, throws
  in `RuntimeError` one step outside it, and names both the member and the numbers it was handed,
  four sentences distinct over all sixteen ordered pairs.
- **The last site, `csv.rs:512`, is not reachable from source at all and is owed no case**:
  `Core\Csv::format` takes an `array<array<string>>` and its `{header:}` an `array<string>`, a
  `mixed` is not implicitly assignable to a narrower type, and ADR 0007 § 2's `array<T> as
  array<U>` does not lower — so no program can put a non-`string` in a cell, and the column that
  refusal names cannot be reached until that conversion row lands.
- **Every count-shaped producer in the library draws fallibly now, and the last of them needed a
  runtime seam rather than a call-site fix.** `nvs_runtime::affordable` refuses only a size past
  `isize::MAX`, so every count below it that the machine cannot serve used to reach an infallible
  `NvsStr::build`, `vec![…; n]`, `slice::repeat` or `Vec::push` and abort — exit 127, nothing on
  stderr, nothing catchable, every in-flight request with it. Measured, not deduced:
  `Core\Arr::fill(1000000000000, 0)` printed an allocator abort and now throws and is caught, as
  `Core\Str::repeat("x", 1000000000000)` already did.
- **The fallible half of the string seam is `NvsStr::try_build`**
  (`crates/nvs-runtime/src/string.rs:317`), which answers `None` where `NvsStr::build` aborts and
  is **exact-capacity only**: a writer past its capacity still grows through the aborting
  `StrWriter::grow`, which is why `built` stays the spelling for a member whose capacity is a bound
  on a subject already in memory rather than a count off its own call site. `Core\Str`'s three
  reach it through `built_fallibly` (`str.rs:707`) and `Core\Bytes`' three through
  `reserved`/`produced_fallibly` (`bytes.rs:459`, `:434`) — the `Vec` reserved with `try_reserve`,
  amortized because `join` reaches it once per part, and the copy out through `try_build` as well,
  since that second allocation is as able to fail as the first and both are live at that moment.
- **The array half is `NvsArray::try_reserve`** (`crates/nvs-runtime/src/array.rs:789`), which is
  the same bargain over the entry storage instead of over a payload: `Vec::try_reserve` in the
  packed form, both of the hash form's vectors otherwise, and what it makes infallible is the entry
  storage's growth and nothing else — the hash form's append also renders and allocates a key, so a
  caller that needs the guarantee appends into a list, which `Core\Arr::fill`, `::padStart` and
  `::padEnd` all do because each builds a fresh array. Those three reach it through `append_copies`
  (`arr.rs:1321`), which now takes the member's own qualified name: the `affordable` call there was
  hard-coded to `Core\Arr::fill`, so a padding member that refused reported the wrong one.
- **Two sentences, and which one a count gets is the bound**: the seam's own past `isize::MAX`, the
  allocator's — `the result is larger than any string`/`buffer`/`array` `this process could hold` —
  below it. Both are reachable for `Core\Str::repeat`, both padding pairs, `Core\Bytes::repeat` and
  `Core\Arr::fill`, whose sizes are products; `Core\Bytes::fill` reaches only the second, its
  length being the size; and `Core\Bytes::join`'s is not reachable from source at all, its parts
  having to be in memory already to sum past it.
- **The agreement is asserted now**
  (`tests/conformance/core/count-shaped-producers-refuse-alike.nvst`): eight members answer a size
  they can serve, the seven that can reach the policy seam speak one sentence there word for word,
  and the allocator's sentence partitions the eight by the noun each names — 3 `string`, 2
  `buffer`, 3 `array` — with every message checked to begin with its own member's name, so a member
  that grew its own wording or borrowed a sibling's noun fails the counts while still reading
  plausibly beside its own arguments. Both halves are asserted by construction as well: every
  helper catches `RuntimeError` and nothing wider, and a member that drew infallibly would leave
  the case with no output at all.
- **JSON numbers are where `Core\Json` and PHP part, in three places, and none is yet a case.**
  `1e999` is `INF` to `json_decode` and accepted by `json_validate`; both `Core\Json` readers refuse
  it, because `serde_json` will not produce an infinite `float`. `9223372036854775808` fits `uint`
  but not `int`, so it is refused here — *"the integer … is too large for `int`"* — where PHP widens
  it to a `float`; an integer past **every** integer type (`12345678901234567890123`) is not that
  row and does agree, both reading it as a `float`. And `-0` is the integer `0` to PHP where it is
  `-0.0` here. The writer belongs to the same family: PHP's `json_encode` prints a whole-numbered
  float without its point (`0.0` → `0`, `100.0` → `100`) and `Core\Json::encode` keeps it, so a
  differential case comparing decoded *values* by re-encoding has to leave every whole float out or
  it measures the two writers instead of the reader.
- **`Core\Json::decode` refuses a band of integers `json_decode` widens, and reads `-0` as a float
  where PHP reads an integer** (`json-decode-refuses-the-number-band-json_decode-degrades`). The
  refusal band is exactly `i64::MAX`+1 ..= `u64::MAX`: `9223372036854775807` is read exactly by both,
  `9223372036854775808` and `18446744073709551615` throw here and become floats there, and
  `18446744073709551616` is widened by *both* — that upper edge is `nvs_stdlib::json`'s gap 1 rather
  than a rule, since `serde_json` has already made an `f64` of a longer literal before the visitor
  sees it. There is no negative half of the band: `int`'s floor is read exactly and one below it is
  widened by both, because the only refusing visitor arm is the unsigned one. `1e999` refuses here at
  the read where PHP reads `INF` and then fails at the *write*. Separately, and not a reader question
  at all, `Core\Json::encode` keeps a whole-valued float's fractional part — `0.0`, `100.0`,
  `1e+308` — where `json_encode` writes `0`, `100`, `1.0e+308`, so a round-trip *text* comparison
  between the two languages is measuring the writers unless every row carries a fraction.

- **`Core\Arr::flattenDeep` agrees with `iterator_to_array` over a `RecursiveIteratorIterator`
  outright, keys and all** (`arr-flatten-deep-matches-iterator_to_array-over-a-recursive-array-iterator`)
  — the one `Core\Arr` member carrying a key rule that does *not* diverge, because the twin's second
  argument is `false` and that discards every key alike, which is ADR 0069 § 3's rule exactly. The
  twin's other setting is not a second answer to compare against: `true` re-keys each leaf by its own
  level's key, so `[[1,2],[3,4]]` collapses to `[3,4]` and entries are lost. Both sides also agree
  that an empty level contributes nothing and no hole, that `0`/`""`/`false`/`null` are leaves, and
  at 300 levels of nesting.
- **`0 - $x` at `int`'s smallest value wraps back to itself in silence** — no throw, no diagnostic, the
  answer is `-9223372036854775808` again. This is what makes `Core\Math::abs` a member rather than sugar
  for `Core\Math::max($x, 0 - $x)`: the composition is *total* at that row and quietly wrong, while `abs`
  refuses with "its magnitude is one past the largest" (ADR 0007's no-silent-promotion rule). A case
  asserting the divergence should assert that the derivation *answered* and `abs` *refused*, not what the
  arithmetic wrapped to — the overflow policy is a different member's question and pinning it here would
  make this case fail for the wrong reason. `math-abs-and-sign-are-the-ordering-trio-and-part-from-it-only-where-they-refuse.nvst`
  is the shape.
- **`Core\Uri`'s four percent-coders are inverse in three of their four cross-directions, and the
  fourth is the space.** `decodeFormValue` reads `%20` as well as `+`, so it undoes
  `encodeComponent` over the whole ASCII sweep; `decodeComponent` reads only `%20`, so
  `decodeComponent(encodeFormValue(" "))` is `"+"` and every other byte round-trips. The two
  encoders disagree on exactly two bytes — the space and `~` — and above `U+007F` they are one
  encoding, because no octet of a multi-byte sequence is in either unreserved set. A sweep is
  written with `Core\Str::fromCodePoint($b as uint)` over `0..127`; past that a *lone* octet is not
  a `string`, so `decodeComponent("%FF")` throws (`uri.rs`'s gap 2). In the same family, the one
  `Uri` for which the seven readers do not recompose to `toString()` is one whose port was written
  empty: `http://h:/p` reports `port()` of `null` because § 3.2.3's empty port is not a port, so a
  `with` that never mentions the port drops the `:` — `nvs_core_uri_with`'s own doc comment names
  this as the single place a round trip through `with` is not the identity. Both are pinned now.
- **Seven members share one total order, and it is asserted by counting agreements between them.**
  `Core\Arr::sort`/`min`/`max`, `Core\Math::min`/`max`/`clamp` and `Core\Heap` all reach
  `nvs_stdlib::ordering::compare_values`, so over one table the sorted pair's two ends are
  `Core\Math`'s two answers, `Core\Arr::min`/`max` name those same ends, `clamp` treats each value
  as inside the pair's own interval, and a heap's `peek` is the smaller whichever way round the two
  were pushed — 432 agreements over 36 ordered pairs, for a `string` table and an `int` one alike.
  A heap drained by `pop` is `Core\Arr::sort`'s sequence entry for entry, which is the same order
  applied n log n times. Pinned by
  `tests/conformance/core/ordering-is-one-total-order-shared-by-arr-math-and-heap.nvst`.
- **The ordering refusal is one throw in one wording, and a case only reaches it through `mixed`.**
  `Core\Math::min("a", 1)` never runs — `T` unifies at the first argument, so the second is
  `E0401: expected string, found int` — so both halves of an orderless pair have to be laundered
  through `mixed` locals, and the array through `array<mixed>`, before the pair reaches
  `compare_values` at all. What it then raises is the *same* `Fault::thrown` for every member,
  `<member> has no natural order for tag N against tag M: …`, differing only in the name in front,
  and a `catch (Throwable …)` reaches all of them. All seven agree about refusing string/int,
  null/int, bool/int, null/string, bool/string and array/int, and about accepting an ordered pair.
  Pinned by
  `tests/conformance/core/ordering-refuses-a-pair-with-no-order-once-for-all-seven-members.nvst`.
- **`continue 2` inside a `switch` is PHP's *idiomatic* spelling, not an exotic one, so counting
  `continue N` over loops alone silently breaks ported code.** Novis already reads a bare `continue`
  inside a `switch` as continuing the enclosing loop (docs/adr/README.md), and the obvious
  generalization — count loop frames and skip `switch` frames — compiles, passes every existing test,
  and then rejects `foreach { switch { case: continue 2; } }` outright with "names more than the 1
  enclosing loop". Worse, with two nested loops it silently retargets PHP's *inner* loop to the outer
  one. The rule that is PHP-identical everywhere PHP accepts the level: count **every** frame the way
  PHP does, then walk *outward* from the frame the level lands on to the nearest loop — level 1
  reduces to the bare-`continue` rule already decided, and nothing new is invented. One
  `.agent-tmp/*.nvs` scratch run beside `php` on the same file is what found it; the checker alone
  would have shipped the divergence.
- **Check every spelling against `php -r` before deciding a family is refused, because PHP does
  not always agree with itself.** `echo $a[];` is *"Cannot use [] for reading"* and `unset($a[])`
  is *"Cannot use [] for unsetting"*, both compile errors — but `$a[] .= "x"` **appends**, silently,
  with no notice even at `error_reporting=-1`, because the element that is not there yet reads as
  `""`. So refusing that third one is a divergence and needed a row of its own (ADR 0007 § 7 row
  10), not a sentence in a doc comment. Three `php -r` calls settled what an hour of reasoning from
  the first two would have got wrong.
- **Turning a panic into a diagnostic breaks the tests that pinned the panic, and they do not look
  like your change.** `nvs-ir`'s `#[should_panic(expected = "known gaps")]` guard failed with
  *"panic did not contain expected string"* while printing the new diagnostic, and
  `nvs-types`' `expr_table` fixture failed at its own `assert!(!diags.has_errors())` helper —
  neither names the feature. Delete the guard (the `.nvst` case is its replacement) and split the
  fixture helper so the one test whose point *is* the diagnostic gets the `Diagnostics` back.
- **A new refusal in `check_expr` fires before `check_write_target`, so it double-reports every
  receiver that already has a better code.** `$erased->rows["0"] = "z"` printed E0482 *and* E0480;
  `$maybe?->rows["0"] = "z"` printed E0482 *and* E0479. The ordering is fixed — `check_write_target`
  reads the `ExprInfo` the target's own check records — so the suppression has to be a lookup the
  early arm can already do: the chain root's recorded `HookedProperty`/`ShapeProperty`, or a
  syntactic `nullsafe: true`, gated on the level being an assignment target at all so a plain *read*
  through the same receiver keeps its only diagnostic.
- **A standing case can pin the spec § 10 *class* a refusal arrives in, so a member
  whose classification you change fails `verify.py`'s conformance leg and not its unit
  tests.** `Core\Time::parse`'s zonal-pattern refusal was a `ParseError` because the
  case that pinned it argued the check "cannot live in the compile step" — which is
  true and beside the point: `crate::cldr`'s `date_fields_only`/`time_fields_only` are
  already per-member *narrowings* of one shared compiler, so a third one is the shape
  the module has. Read the case's own reasoning before reclassifying, and if the
  reasoning is what is wrong, rewrite that paragraph rather than the expectation
  alone — the count below it is usually what decides whether the row moved sides.
- **`Core\Test::assertCount`'s two `Fault::fatal` sites are unreachable from source and are owed
  no case.** Its parameters are `array<T>` and `uint` (`crates/nvs-stdlib/src/registry.rs`'s row,
  `test.rs:365`), so every way to hand it a non-container or a non-`uint` count stops at the
  checker: a `mixed` subject is `E0401: expected array<mixed>, found mixed`, a `?array<string>`
  is `E0401` on the union, and a `mixed` count is `E0401` at the second argument. Same shape as
  `Core\Csv::format`'s column guard in the bullet above, and the same three `nvs run` probes on a
  scratch file settle it in one call rather than in a case that will not compile. Worth knowing
  because the assertion members read as if a bad subject were a runtime question; it is a
  signature question.
- **`Core\Arr::withoutFirst` keeps the keys it did not remove, so dropping entry `0` of a list leaves a
  map at key `1`.** PHP's `array_shift` reindexes; this does not, because ADR 0063 R3 makes the member
  answer a copy rather than mutate, and a copy that silently renumbers its own keys is the surprise the
  rule exists to avoid. `Core\Arr::values` is the reindexing left explicit. Pinned by
  `tests/conformance/core/arr-the-empty-array-is-what-every-reshaping-member-answers-it-with.nvst`.

- **An `Instant`'s epoch readings truncate toward zero, not toward minus infinity.** 1.5 seconds before
  the epoch reads as `-1` second, `-1500` millis, `-1500000` micros — so the coarse reading stays the
  fine one divided on both sides of the epoch, at the cost of the second reading not being the second
  the instant falls inside. Unix `time_t` convention would floor to `-2`. Pinned by
  `tests/conformance/core/time-reading-an-instant-in-a-zone-does-not-disturb-the-instant.nvst`.
- **CLDR's single-count `X` and `x` omit an offset's minutes when they are zero, and no PHP `date()`
  letter does.** `$dt->format("x")` at a zero offset is `+00` where `O` is `+0000`; the two-count and
  three-count forms (`xx` against `O`, `xxx` against `P`) agree outright, and `X` at a zero offset is
  `Z`, which is PHP's `p`. So the one-letter forms are the only rows of the pattern grammar whose twin
  has to be computed (`substr($m->format("O"), 0, 3)`), and they sit beside the other three that do:
  `D` is a one-based day of year against a zero-based `z`, and `K`/`k` are hour cycles PHP has no
  letter for at all. `tests/differential/lang/a-date-format-string-renders-as-phps-does.nvst` is where
  all four are written out.

- **`grep -r --include=<glob> .` through the Bash tool walks this tree and silently finds nothing**,
  returning exit 0 with no output, so it reads as a clean answer rather than a broken search. It found
  0 hits for a pattern that ripgrep found 172 of, across files it had just been pointed at by name.
  Use the **Grep tool** for any repo-wide question whose answer you are about to write down — a shell
  `grep -n` on *one named file* is still fine, and is how the discrepancy surfaced. This cost an ADR a
  wrong corpus count, stated as a measured fact and committed before the tests disagreed with it.
- **A name-spelling change has three corpora, not one, and the third only fails at the very end.**
  `.nvs`/`.nvst` fixtures hold the spelling literally; Rust test fixtures hold it **escaped**
  (`"#[\\Core\\Route]"`, two bytes per separator); and a handful hold it in a **raw string**
  (`r#"#[\Core\Route]"#`, one byte). A regex written for either of the first two matches nothing in the
  third, so the sweep looks complete, the workspace builds, and one test in one crate fails on a
  fixture nothing else touches. Sweep for the single-byte form *after* the escaped one and check the
  hits are only doc comments before believing you are done.
- **A refusal of a shape the language used to accept has five homes, and two of them are tables.**
  The code in `crates/nvs-diagnostics/src/lib.rs`, the report site, a `tests/conformance/reject/`
  case, a row in `docs/reference/tools/30-php-differences.md` **and** a row in
  `docs/adr/divergences.md` — then `python tools/reference.py`, which regenerates `docs/novis.md`
  (it carries both tables) and proves its 265 examples, so a chapter example written in the old
  spelling fails there rather than in `verify.py`. And a shape that *worked* has a test pinning the
  old rule: `parser::tests::stmt::try_multi_catch_and_finally` and
  `an_unannotated_class_constant_reads_at_its_values_type` were each the only thing that failed
  after the parser changed, and each had to be rewritten to the new rule rather than deleted.
