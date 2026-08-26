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

## Running things

- **Verification is one call:** `python tools/verify.py` — build, fmt, test, the two `.mwlt` trees and
  clippy in order, stopping at
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
- **`verify.py` executes the `.mwlt` trees, so nothing else needs running before the wrap.** Its
  `conformance` and `differential` steps are `target/debug/mwl test tests/<tree>` — the very
  command `tools/loop.py`'s acceptance check judges a session by — and their two lines are the
  counts the plan's fields quote. Fourteen seconds for both. So after a green `verify.py` there is
  no `mwl test tests/conformance` to run, no `mwl test tests/`, and above all **no
  `cargo build --release -p mwl-cli`**: that is 125s for a less faithful answer, and one measured
  run spent 8% of its entire wall clock on it across nine sessions. This bullet used to say the
  opposite — `cargo test`'s `conformance_coverage.rs` asserts only that a case *exists* naming each
  registry member, so a rewritten case body could leave every verify step green and fail at
  `loop.py` a stage later. That hole is what the two steps close. While *writing* a case, one at a
  time is still fastest: `./target/debug/mwl.exe test <path>`, under a second.
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
- **`target/release/mwl.exe` is whatever the *last* session built, and rebuilding it costs two
  minutes for a verdict the debug binary already gives.** A `.mwlt` case a stale binary fails may
  simply predate it — one session's was two hours and four commits old and reported
  `str-replace-and-pad-are-the-identity-at-their-own-bound.mwlt` failing on a `Core\Str::padStart`
  line nothing in the session had touched. The fix is **not** the release rebuild it used to be:
  `cargo build --release -p mwl-cli` is 125s here (thin LTO at `codegen-units = 1` relinks the world
  for a one-line edit), nine sessions of one run paid it, and that was 8% of the whole run's clock.
  Build `cargo build -p mwl-cli` and run `target/debug/mwl.exe` instead — 2s once `verify.py` has
  built, and it is the *same* binary `tools/loop.py`'s acceptance check judges you by, so it is the
  more faithful answer as well as the cheap one. `git status --short` showing the case unmodified
  says the failure was not *caused* here, which is the neighbouring bullet's rule; only a current
  binary says it is not real.
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
- **A `.mwlt` helper cannot take a `Core` enum parameter, and the diagnostic names the same type
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
  binding name like any other. The roster of names is `mwl_runtime::throwable::ThrownClass`'s own
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
  it panics `mwl-ir` at `crates/mwl-ir/src/lower/expr.rs:877` (`array<T> as array<U>` is the
  conversion row still missing). The tool's own header says an entry is a candidate rather than a
  plan; this is the cheapest way to judge one, and it is four `mwl run` calls on a scratch file
  rather than a written case that will not compile.
- **`Core\Bytes::join`'s allocation refusal cannot be reached from source, and the obvious probe
  reports the wrong member.** Its size is the sum of parts that must already be in memory, so a case
  reaching for it by handing it a huge separator — `Core\Bytes::join($parts, Core\Bytes::fill(1e12,
  44))` — is refused by `fill` while it builds the argument, and the message names `Core\Bytes::fill`.
  The row looks like a `join` assertion and pins `fill` twice. The reachable count-shaped refusals are
  `Core\Str::repeat`, `Core\Str::padStart`/`padEnd`, `Core\Bytes::repeat`, `Core\Bytes::fill` and
  `Core\Random`'s two; a member whose size is a *product* reaches both sentences (past `isize::MAX` is
  `mwl_runtime::affordable`'s, below it the allocator's), while one whose size is its own `uint`
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
  `"the append counter never names a live key"` `debug_assert` (`crates/mwl-runtime/src/array.rs:395`)
  and `mwl run` dies mid-file, so every later row of the case is lost and the failure reads as the
  harness rather than as the bound being probed. The last *accepted* append is the one after a key
  of `9223372036854775806`, which lands at `9223372036854775807`; the plan's `Open now` owns why the
  first refused one is a crash rather than PHP's `Error`.

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
- **A closure held in an array can be handed to a `Core` member's `callable` option**, even
  though calling one through the variable holding it panics `mwl-ir` outright. `array<callable>
  $filters = [fn (Core\Cli\Text $c): int => 7, ...];` then `foreach ($filters as callable
  $filter) { ... Core\Out::capture($body, {through: $filter}) ... }` lowers and runs, because the
  call is the runtime's (`mwl_runtime::call_closure`) and not a lowered `Call` — which is what
  turns a sweep over eight closures into a sweep rather than eight copies of one block. Two
  spellings to get right on the way: a **block-bodied** `fn` must declare its return type
  (`E0450`: `fn (): void => { ... }`), an expression-bodied one takes the expression's; and a
  `mixed` is **not** implicitly assignable to a narrower type, so `array<string> $row = ["a",
  $cell];` over a `mixed $cell` is `E0401` at the element.
- **An integer literal past `int` lowers in a `uint` *argument* and panics `mwl-ir` inside an
  `array<uint>` literal.** `Core\Random::bytes(9223372036854775808)` is fine — the parameter's
  declared type is what decides how the literal lowers — but `array<uint> $counts = [1,
  9223372036854775808];` dies with *"mwl-ir: integer literal `9223372036854775808` doesn't fit an
  `int`"*, an element position carrying no such expectation. So a sweep table whose rows run past
  `i64::MAX` has to *compute* them rather than write them, and the multiplier is the second half of
  the trap: `$n * 2` over a `uint` is `E0407` and then `E0401`, the literal `2` being an `int` with
  no representable common type, so the case declares `uint $two = 2;` and multiplies by that.

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
  compare numerically — and `array_reduce`'s callback taking exactly two arguments where MWL's
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
  first**, where `mwl_stdlib::math::pick` answers the first to both — visible wherever two equal
  values are distinguishable, `min(1000000000000000000, 1.0e18)` being the sharpest — and
  `f64::total_cmp` separates `-0.0` from `0.0` where PHP's `<` calls them equal.
- **A `CoreTy::Var("T")` signature binds `T` to the first argument**, so a crossed pair reaches the
  runtime refusal only when the union is declared on the bindings; written as two literals
  `Core\Math::min(0, "a")` is `E0401`, and the same holds of `Core\Math::mod(7, 2)`, whose two
  `float` parameters do not widen an `int` the way `fmod` does — the integer remainder is the `%`
  operator.
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
- **The one guard this added is `log`'s *base***, which PHP has and MWL did not: a base not greater
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
  the accuracy spent is stated in `mwl_stdlib::math`'s own doc comments at both members.
- **`Core\Math::gcd` and `::lcm` have no callable twin on either leg**: neither the Windows `php`
  nor WSL's has `gmp`, so `gmp_gcd`/`gmp_lcm` are undefined functions and an oracle case for those
  two has to compute its expectation with an explicit Euclidean loop in PHP or be left out of the
  count.
- **MWL's `float` rendering is PHP's**, precision 14 with trailing zeros trimmed — `sqrt(2.0)`
  prints `1.4142135623731`, `exp(-745.0)` prints `4.9406564584125E-324` and `0.1 + 0.2` prints
  `0.3` on both sides — so a `Core\Math` oracle case may echo a float directly and needs no
  formatting, but never a `NAN`, which PHP 8.4 and later warn about coercing to a string.
- **`Core\Str`'s twins part from PHP over a *unit* before they part over anything else**, and
  `slice`/`replaceRange` is the worked pair: `mwl_stdlib::granularity::DEFAULT` is
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
  subject outside ASCII is not well-formed UTF-8 and therefore not a value MWL can hold at all —
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
  other* rather than with MWL.** `replaceAll` folds `str_replace`'s array form and `strtr`, which
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
  purpose, `mwl_core_str_code_points`'s doc comment being the home of why, so it parts from
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
  (`mwl_stdlib::str::scalar_value`), never a substituted U+FFFD, ADR 0009 § 3's
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
- **`Core\Str::at`'s message was the one in `mwl-stdlib` missing the `()` every sibling writes**
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
- **Spec § 4's three `format` members read one pattern compiler** (`mwl_stdlib::cldr`) and each
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
  through `mwl_runtime::php_float_to_string` rather than Rust's `inf`, and a value whose tag has no
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
- **`--errors` names no catchable site a case can still assert anywhere in `mwl-stdlib`.**
  `random.rs:333` was reachable after all: `mwl_runtime::affordable` refuses only a size past
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
  runtime seam rather than a call-site fix.** `mwl_runtime::affordable` refuses only a size past
  `isize::MAX`, so every count below it that the machine cannot serve used to reach an infallible
  `MwlStr::build`, `vec![…; n]`, `slice::repeat` or `Vec::push` and abort — exit 127, nothing on
  stderr, nothing catchable, every in-flight request with it. Measured, not deduced:
  `Core\Arr::fill(1000000000000, 0)` printed an allocator abort and now throws and is caught, as
  `Core\Str::repeat("x", 1000000000000)` already did.
- **The fallible half of the string seam is `MwlStr::try_build`**
  (`crates/mwl-runtime/src/string.rs:317`), which answers `None` where `MwlStr::build` aborts and
  is **exact-capacity only**: a writer past its capacity still grows through the aborting
  `StrWriter::grow`, which is why `built` stays the spelling for a member whose capacity is a bound
  on a subject already in memory rather than a count off its own call site. `Core\Str`'s three
  reach it through `built_fallibly` (`str.rs:707`) and `Core\Bytes`' three through
  `reserved`/`produced_fallibly` (`bytes.rs:459`, `:434`) — the `Vec` reserved with `try_reserve`,
  amortized because `join` reaches it once per part, and the copy out through `try_build` as well,
  since that second allocation is as able to fail as the first and both are live at that moment.
- **The array half is `MwlArray::try_reserve`** (`crates/mwl-runtime/src/array.rs:789`), which is
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
  (`tests/conformance/core/count-shaped-producers-refuse-alike.mwlt`): eight members answer a size
  they can serve, the seven that can reach the policy seam speak one sentence there word for word,
  and the allocator's sentence partitions the eight by the noun each names — 3 `string`, 2
  `buffer`, 3 `array` — with every message checked to begin with its own member's name, so a member
  that grew its own wording or borrowed a sibling's noun fails the counts while still reading
  plausibly beside its own arguments. Both halves are asserted by construction as well: every
  helper catches `RuntimeError` and nothing wider, and a member that drew infallibly would leave
  the case with no output at all.
