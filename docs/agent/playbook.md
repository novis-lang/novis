# Playbook — the traps this repository has actually sprung

Hard-won specifics: things that cost a real session real time, written down so the next one does not
pay again. A bullet is a *trap* — "this looks like it should work and does not, and here is what to do
instead" — in three sentences, ending with the `[until: <kind> <arg>]` trailer that says what retires it
([conventions.md](conventions.md) § *A playbook bullet*; the kinds are `tools/playbook.py`'s module doc).
**It is not a changelog.** The session that hit the trap, the stage it was on and what it tried first
are `git log`'s to keep, and a bullet that carries them is charged to every session after it.

**Append, expire, never reword.** Add a bullet when a trap costs you time — `session.py --wrap`'s
`## playbook:` section is the one way in, and it refuses a bullet without a trailer or past 700 bytes.
Edit one when it stops being true. The wrap deletes every bullet whose condition holds, so a trap that
dies with a flag, a file or a test declares that and leaves on its own. Never reword a bullet to say the
same thing differently: the churn is the cost this file exists to avoid.

**Scope.** A rule that binds every agent goes in [AGENTS.md](../../AGENTS.md). A settled rule lives in
the rulebook under [docs/rules/](../rules/), with a decision record for its reasoning. How a subsystem
works goes in that crate's own module doc comment. The *shape* of something you are about to write is
[conventions.md](conventions.md). What is left — the trap — is this file, and `orient.py` hands a
session only the bullets its goal's `[context] playbook` selects and its item's own files promote.

## Tooling

- **`cargo test -p <crate>` straight after a workspace `cargo test` recompiles the crate, and the
  workspace run recompiles it back.** A package selected alone unifies its dependencies' features
  differently from the whole workspace, so its `-p` artifact is a second one that every source edit
  stales, which is why `Goal.crate_tests` in `tools/loop.py` runs a crate's test executables off one
  workspace build. When you scope by hand, scope `verify.py -p` and every `cargo` call the same way
  for the whole session, or budget the rebuild at each switch. [until: reviewed 2026-09-06]
- **`.agent-tmp/` is shared between concurrent writers, so a fixed scratch filename hands you their
  file.** `git commit -F .agent-tmp/msg.txt` can pick up a stale message another session left there,
  with no error and no warning, and the same applies to a `--patch` file handed to `splice.py`. Name
  a scratch file for the thing it holds (`wrap-msg-handoff.txt`), never `msg.txt`/`patch.txt`, read
  `git log --oneline -1` after any `-F` commit, or let `python tools/session.py --wrap` write the
  commits, which never touches a shared path. [until: reviewed 2026-09-06]
- **`orient.py` prints less than `brief.py` on purpose, and the gap is a bug in the goal, not in the
  tool.** If it did not print a module, a rule section, a convention shape or a playbook bullet you
  turned out to need, do not re-run `brief.py` for everything. Fetch the one thing, then edit that
  `[context]` field in `loop-goal.toml` yourself — it reloads every session and widening it breaks
  nothing, while a gap only written into the handoff is one every later session pays too. `modules`
  is the field you may leave: `tools/context-sync.py` sweeps it from your commits between sessions.
  [until: gone tools/context-sync.py:MAX_ADDED]
- **A goal's number is its position, so inserting one renumbers every goal after it — and that is
  fine only because nothing but the tool writes a number.** `python tools/chain.py --new <slug>
  --after N` renames the files it displaces and rewrites the two headers and the link targets that
  carry a number; prose names a goal by its slug and is untouched. Never rename a goal file by hand,
  and never write `goal 29` — nor a *range* of them, `goals 30–44`, which is the form that got past
  this and into the handoff — into a sentence; `chain.py --check` fails on it, because the next
  insert would silently make it name a different goal. [until: gone tools/chain.py:number_citations]
- **A whole decision record costs thousands of tokens to read; one of its `###` sections costs a
  fraction.** `python tools/peek.py <file>:"## 4"`, or `sed -n` between the heading and the next
  one, never `cat`. The same goes for a long module: `grep -n` for the anchor first.
  [until: reviewed 2026-09-06]
- **`D:` fills up.** `target/debug` grows to tens of GB and `cargo test` then fails as a wall of
  `link.exe` errors whose real message (`no space on device`) only appears without a `Select-String`
  filter. Check `Get-PSDrive D` before diagnosing a linker failure; `cargo clean` frees it in
  seconds and the rebuild is a few minutes. [until: reviewed 2026-09-06]
- **A Python `str.index` anchor that is not unique cuts the wrong region and *duplicates* the file,
  with no error.** A script splicing with `s[:a] + s[b:]` that finds an earlier match for `b` has `b
  < a`, removes nothing, and hands the file back longer than it went in — only `git diff --stat`
  notices. Use the Edit tool to remove a block, since it refuses a non-unique `old_string`; if a
  script really must cut, `assert a < b` and print the slice length, and recover with `git checkout
  -- <file>` plus re-applying the edits. [until: reviewed 2026-09-06]
- **The Bash tool mangles backslashes and escapes inside a heredoc, so a spliced block silently
  fails to match or lands corrupted.** A `\\` in a `python - <<'PY'` heredoc arrives as `\`, a
  `"\n"` arrives as a real newline, and a Rust `\`-continuation comes back as a literal `\n` that
  still compiles. Use Write/Edit, or `python tools/splice.py <target> --patch <file>` with the patch
  written by the Write tool; if an edit genuinely must be scripted, build a backslash as `chr(92)`,
  and give a Rust string holding a `Core\Name` label `r"..."`. [until: reviewed 2026-09-06]
- **A file edited by a script comes back into your context whole.** The harness re-prints an on-disk
  change it did not make as a "changed on disk" reminder — a long module is thousands of tokens,
  more than the edit cost — and the Edit tool never triggers it. Take the heredoc/`splice.py` route
  only for an edit Edit genuinely cannot express (a non-unique anchor, a whole-field rewrite).
  [until: reviewed 2026-09-06]
- **`cargo test` does not always relink `target/debug/nvs.exe`.** A stale binary reports a member
  you just registered as `mixed`, which reads as a registry bug. Run `cargo build` before
  running a fixture or a `.nvst` case by hand. [until: reviewed 2026-09-06]
- **`wsl.exe` needs PowerShell and a script file.** An inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. For the whole suite, background
  `python tools/loop.py --leg-only`, which drives `wsl.exe` for you; for one fixture,
  `tools/leak-check.sh <paths>` takes `.nvs` files only, so a `.nvst` passed to it reports a failure
  that is not a leak. [until: reviewed 2026-09-06]
- **Another agent — another session of the loop, or the user — may be editing this repo right now,
  and `ls` will not tell you.** `tools/brief.py` prints a loud banner when `.loop/running` exists;
  if it does, stop and tell the user rather than editing alongside it. Otherwise claim a new
  numbered file with `git status --short <dir>` immediately before creating it, stage your own paths
  explicitly rather than `git commit -a`, check `git show --stat` after committing, and re-read a
  shared doc immediately before rewriting it. [until: reviewed 2026-09-06]
- **A second writer can replace a file you created *this session*, and the only notice is a "changed
  on disk" line.** Two concurrent runs of one slice produce two implementations of the same new
  module and two conformance cases for it, and the second Write silently wins. Do not revert — the
  tree is only consistent with the newer one — but re-read the file before every later edit, delete
  the duplicate case rather than shipping both, and look for it in the `git status --short` you run
  before staging, which is the only place it shows. [until: reviewed 2026-09-06]
- **After touching either spec file, `python tools/check-migration.py`; after moving or renaming a
  doc, `python tools/check-links.py`.** The link check reports broken *and* mis-cased relative
  links. [until: reviewed 2026-09-06]
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own
  file. [until: reviewed 2026-09-06]
- **A `Core\Name` inside a `python - <<'PY'` heredoc is a Python escape error, not just a Bash
  one.** `"""… Core\Uri …"""` reaches Python intact and *Python* then rejects `\U` as a truncated
  `\UXXXXXXXX`, so a patch script quoting any `Core\U…`/`Core\N…` path dies at parse time with
  nothing about the real edit in the message. Use the Edit tool for a targeted replacement, or
  `splice.py` with a Write-tool patch file. [until: reviewed 2026-09-06]
- **`plan.py --get <Field>` prints a field whole, so weigh it against what the field costs.** It is
  the right call when you need the field's exact wording, which a `## plan-edit:` fragment has to
  quote. To confirm an edit *landed*, `grep -n` the phrase in `docs/implementation-plan.md` instead
  — the `> `-prefixed lines are the same fact for a fraction of the cost, and `--wrap` already told
  you. [until: reviewed 2026-09-06]
- **A plan status field is *one* logical line, and three renderings of it disagree.** `plan.py
  --get` hands back the unwrapped line, `orient.py` re-wraps it, and on disk it is a `> `-prefixed
  blockquote wrapped at ~100 columns, so an anchor copied out of either rendering never matches the
  file. To change one sentence with the Edit tool, `grep -o` the phrase in
  `docs/implementation-plan.md` and copy the `> `-prefixed lines around it; `plan.py --set FIELD
  --from <file>` is the only way to make a field-wide change. [until: reviewed 2026-09-06]
- **A `**Bold phrase:**` inside a field body silently becomes an eighth status field.** `plan.py`'s
  `FIELD_RE` is `^> \*\*([^*:]+):\*\*`, and `--set` re-wraps, so a bolded lead ending in a colon
  that lands at the start of a wrapped line is parsed as a new field on the next read, which
  AGENTS.md's fixed field set forbids. Write `**Bold phrase** —` instead; the symptom is `python
  tools/plan.py` listing eight fields, and the fix is `git checkout -- docs/implementation-plan.md`
  followed by the `--set` again. [until: gone tools/plan.py:FIELD_RE]
- **Making a type `pub` owes it a `#[derive(Debug)]`.** The workspace denies
  `missing_debug_implementations`, so promoting a private struct to the public API compiles and then
  fails at clippy, after the tests have already run. Add the derive in the same edit as the `pub`,
  not after `verify.py` says so. [until: gone Cargo.toml:missing_debug_implementations]
- **`/tmp` is not the same directory to Bash and to Python here.** A file written by `>` in the Bash
  tool is invisible to a `python -` heredoc in the same call, which resolves `/tmp` to `%TEMP%`.
  Stage a scratch file under `.agent-tmp/` — both halves agree on a repo-relative path.
  [until: reviewed 2026-09-06]
- **`nvs-ir` cannot name `nvs_hir::QName`.** `nvs-hir` is a *dev*-dependency there on purpose, so a
  lowering helper that wants one in its signature does not compile even though `nvs_types::Ty::Enum`
  hands it a `&QName` to pattern-match. Destructure it at the call site and pass what the callee
  needs (an `&EnumInfo`, or the name rendered with `to_string`); adding the dependency to get one is
  the wrong direction. [until: reviewed 2026-09-06]
- **Teaching `nvs-ir` a new receiver or value shape is two edits, and the second one panics
  somewhere else.** Recording a new `ExprInfo` variant gets the *access* lowering; what still fails
  is `erase_checked_ty`, which owns the `nvs_types::ty::Ty` → `nvs_ir::Ty` map and whose panic names
  "a resolved call's parameter or return type", so the message points at a call boundary or a
  `foreach` element rather than at the feature you built. Add the erasing arm with the variant:
  `Ty::Shape` needed `ExprInfo::ShapeProperty` plus one arm erasing a shape to `Ty::Object`.
  [until: reviewed 2026-09-06]
- **A class synthesized while lowering an *expression* has four exits, not one.** `Lowering` builds
  one function, so a `crate::ir::Class` an expression invents (a closure's environment, a shape
  literal's) rides out of `lower_method`/`lower_hook`/`lower_script` at their three identical
  `std::mem::take(&mut low.closures)` sites *and* out of `lower_closure`'s own recursion, or a body
  nested one level deeper contributes no class and codegen fails much later on a `New` naming a
  label the table lacks. Grep the take sites, not the struct field. [until: reviewed 2026-09-06]
- **`python`, not `python3`; `gen` is reserved in Rust 2024; a renamed snapshot test needs its old
  `.snap` deleted.** `cargo test --release -p nvs-abi-probe` takes over two minutes.
  [until: reviewed 2026-09-06]
- **One call reads many places: `python tools/peek.py A.rs:120-160 B.rs:@sym C.md:"## 4" D.rs:re:pat:3`.**
  Locators are `120-160`, `120+30`, `@symbol`, `re:pattern` — which is the matching line and nothing
  else, so `re:pattern:3`, or `--context 3` for the whole call, is what brings the block with it —
  `"## Heading"`, or nothing for a whole file under 400 lines, and the path may be a glob, so one target
  can sweep a crate. `--locate <symbol> ...` answers with `file:line` and no bodies, which is what a
  handoff's anchors are made of; reach for it instead of a `grep`, then a `sed`, then another
  `grep`. [until: reviewed 2026-09-06]
- **Prefer `re:pattern` to `/pattern/` in a `peek.py` target on Windows.** Git Bash rewrites any
  argument that *starts* with a slash into a Win32 path before the process sees it, so `file.rs:/fn
  foo/` arrives as `file.rs;C:/Program Files/Git/fn foo/` and the tool reports no such file; quoting
  does not help, because the conversion happens in argv handling. The same trap catches any tool
  argument spelled as a leading-slash path. [until: reviewed 2026-09-06]
- **The end of a session is one call: `python tools/session.py --wrap <file>`.** Write one markdown
  file whose `## ` headings are instructions — `## plan-edit: <Field>`, `## playbook: <Heading>`,
  `## handoff`, `## commit: <paths>` once per slice, `## status` — and it applies all of them in a
  fixed order, or refuses one as broken and writes *nothing*. `--check` first says what the tree
  still owes, including any plan field whose prose names a conformance or differential count the
  tree contradicts. [until: reviewed 2026-09-06]
- **A `[context] playbook` entry may name one bullet, not just a whole section.** "Tooling > a whole
  decision record" prints that bullet; "Tooling" prints the whole section, which is a large share of
  the orientation pack once several are named. A goal that needs three traps should name three
  traps; `python tools/orient.py --audit` prices the difference. [until: reviewed 2026-09-06]
- **A commit message carries no attribution trailer, and two gates enforce it.** `tools/session.py`
  strips `Co-Authored-By`, `Signed-off-by`, `Generated-with` and the prose `🤖 Generated with …` line
  out of any message it commits; `tools/git-hooks/commit-msg` rejects one arriving by `-m`, `-F`, an
  editor or a merge. The hook is off until `git config core.hooksPath tools/git-hooks` has been run
  once per clone — `verify.py` says so when it is not set — and conventions.md § *A commit message*
  is the rule's home. [until: reviewed 2026-09-06]
- **Git runs a hook with `LC_CTYPE=C.UTF-8`, and gawk's `[^a-z]` does not match an emoji under it.**
  A pattern anchored as `^[^a-z]*(generated|created)…` passes every test run from the Bash tool,
  which sets no locale, and silently fails to match `🤖 Generated with [...]` when git itself runs
  it. Run a hook's test the way *git* invokes it, and locate a phrase with `match()` and ask a
  pure-ASCII question about the text before it rather than stepping a character class across
  multibyte input. [until: reviewed 2026-09-06]
- **An awk fatal inside `$(…)` leaves the variable empty, and a hook that only checks the variable
  then fails OPEN.** `\[` in a *dynamic* awk regex (one built from a string) is consumed by the
  string literal first, so awk sees a bare `[`, dies with `invalid regexp`, and the surrounding `[
  -z "$offenders" ] && exit 0` allows the commit. Write it `[[]`, and always capture `$?` from the
  awk itself and refuse on a non-zero status — a gate must fail closed. [until: reviewed 2026-09-06]
- **A plan field can disagree with `loop-goal.toml`, and the toml wins.** A field claiming every
  catch-up check passes while tests those checks name exist in no crate sends a session behind a
  gate `loop.py` short-circuits before reaching. The toml names each test literally, so one `grep
  -rn "<test_name>" --include=*.rs` over the block settles it in one call; correct the prose, not
  the toml. [until: reviewed 2026-09-06]
- **An untracked non-crate *directory* under a globbed workspace member path breaks every `cargo`
  call.** The root manifest globs `members = ["crates/*", "benches/*"]`, so a directory without a
  `Cargo.toml` there kills cargo with `failed to read .../Cargo.toml` before any crate is read;
  through `| tail -3` the error scrolls past and the next run uses a stale `target/debug/nvs.exe`.
  `cargo metadata --no-deps >/dev/null; echo $?` is the one-call check, and the fix is the `exclude
  = [...]` line beside the glob. [until: reviewed 2026-09-06]
- **A `grep … | head` that finds nothing is not the same as a grep that finds nothing.** The lines
  `head` keeps can end one line before the rows you are asking about, so a handoff that reports "X
  is not written yet" on that evidence sizes the next slice at half its real scope. Never `| head` a
  grep whose result you are about to assert is empty (use `-c`, `-l`, or no pipe at all), and
  re-check a handoff's "not written yet" in one call before scoping around it.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check's *comment* is not the specification, and it can contradict a settled
  rule.** The toml wins over a plan field, because both are status; it does not win over a rule,
  because only one of those is a decision — a comment describing `==` as component comparison
  contradicts `rule:expressions/object-identity-equality`, under which `==` on objects is identity
  and `compareTo` is the spelling for content equality. One `peek.py` of the cited rule before
  writing the member settles it, and the comment is the thing to fix. [until: reviewed 2026-09-06]
- **`verify.py` can be red on a tree you did not touch, and a step that has been red hides every
  step after it.** `git status --short` showing the failing file unmodified is the whole diagnosis,
  and because the gate stops at the first failure, the steps behind a long-red one (the `.nvst`
  trees, clippy, `cargo doc`) have not been passing either. Fix it in its own commit named for what
  it is; for `fmt`, `grep -c "^Diff in" .agent-tmp/verify-fmt.log` is the blast radius — one file
  means fix it here, a dozen means say so in the handoff instead. [until: reviewed 2026-09-06]
- **Widening an operand's *representation* in `nvs-ir` moves a refcount decision you did not edit.**
  `lower_array_key`'s `bool` meant "aliases storage someone else owns" while its call sites read
  `false` as "a fresh buffer owed a release", which coincided only while every key was a `Ty::Str`;
  an unrendered `int` subscript turned the *unchanged* `if !key_aliasing { emit_release }` into a
  release of a plain integer, and only the IR snapshots showed it. Return the operand's `Ty` and
  guard on `ty.is_refcounted()`, and re-read every consumer of a widened operand for a decision
  phrased as the *negation* of the old invariant. [until: reviewed 2026-09-06]
- **An acceptance check in `loop-goal.toml` can be red on a *name* rather than on a claim.** A check
  can list tests under names predicted before anyone wrote them, while the implementation landed
  under a name stating the narrower, true claim (`appending_into_spare_capacity_allocates_nothing`,
  not `appending_to_a_uniquely_owned_string_does_not_reallocate`). Before writing anything, `grep -n
  "    fn " crates/<crate>/src/<file>.rs` for the claim; correct the *list* when the tree's name is
  truer and say so in the toml comment — a predicted name is status, not a decision.
  [until: reviewed 2026-09-06]
- **`gaps.py --errors` matches a site by its message *before the first format hole*, so closing one
  member can silence its siblings.** A case echoing `Core\Time\DateTime::format(): …` contains the
  stem `Core\Time\DateTime::`, which is the whole stem of every message spelled
  `Core\Time\DateTime::{member}(…)`. The list is a worklist, not a ledger: when the drop is larger
  than the number of sites you asserted, diff it against the tree with your new cases moved aside
  and name the hidden ones in the handoff. [until: gone tools/gaps.py:--errors]
- **A bare `>` line inside a plan field is the field separator, so a blank line added for
  readability splits the field in two.** `plan.py --check` then reports six fields where there were
  seven, and AGENTS.md's fixed field set forbids both the split and the eighth field. Keep the
  paragraph continuous when hand-editing `docs/implementation-plan.md`, and run `python
  tools/plan.py --check` after — it prints the field count and each field's size.
  [until: reviewed 2026-09-06]
- **A plan field is patched, not retyped: `## plan-edit: <Field>` with `--- old` / `--- new`.** `##
  plan: <Field>` still replaces a field whole, but a replacement over 1.5 KB whose text is mostly
  already on disk is refused as a retype, because re-emitting a field by hand is where a paragraph
  gets silently dropped. Quote the `--- old` fragment as the field *reads* — one single-spaced
  paragraph, however it is wrapped on disk — so it matches exactly once; a deliberate trim to under
  60% of the size, or a real rewrite, still goes through as a `## plan:`.
  [until: reviewed 2026-09-06]
- **One field gets one `## plan-edit:` section, however many fragments it moves in.** A second `##
  plan-edit: Open now` further down the wrap file is not merged with the first and its fragments are
  silently not applied, which looks like the size refusal repeating with the byte count unchanged
  after you added more cuts to buy the space. Repeat `--- old`/`--- new` *inside* the one section;
  if a rejection's numbers do not move after an edit, check that the section it names appears once.
  [until: reviewed 2026-09-06]
- **`gaps.py --differential` matches text, so a member can leave the list without a case of its
  own.** `called_members` in `tools/gaps.py` is one regex over the whole case text and never looks
  for a `(`, so a comment naming `Core\Path::normalize` inside a `basename` case silences
  `normalize`'s row, and an oracle's fallback call to a sibling member does the same. Name a
  neighbour without its class, re-run `--differential` after writing a case, and when the drop is
  larger than the members you asked about, the extra one still owes a case.
  [until: gone tools/gaps.py:called_members]
- **A fact restated across decision documents goes stale, and the two shapes that always do it are a
  running count and an ordinal.** A record saying a list "opens with exactly four names" is wrong
  the day another adds a fifth without touching it, and an ordinal chained across records renumbers
  nothing when a link is withdrawn. Before adding to any roster, grep for the total (`grep -rn
  "holds .* names\|opens with exactly" docs/`); the fix that sticks is a table in the owning rule
  that names the code registry it must agree with (`nvs_types::derive::ATTRIBUTES` for
  `rule:core-classes/derive-attribute`), and amendments that state no number.
  [until: reviewed 2026-09-06]
- **`alloc::Pooled` recycles a freed block, so a memory checker cannot see a use-after-free — and
  only one leg of four is affected.** A freed block goes onto the per-thread size-class cache
  instead of reaching `free`, so a dangling read lands in live memory and neither ASAN nor valgrind
  says a word; the leg that matters is `cargo test -p nvs-runtime`, where `cfg(test)` installs
  `counting_alloc` over `Pooled`, and `--features nvs-runtime/sanitizer` swaps in the platform heap.
  The other three legs see every free already; read `counting_alloc`'s module doc before changing
  any of it. [until: gone crates/nvs-runtime/src/counting_alloc.rs]
- **Making a previously infallible instruction fallible moves two guards that name neither it nor
  the operator.** Each new `Inst::on_error` edge adds *two* machine-code `call`s (`nvs_raise_new`,
  then `nvs_trace_push` in the landing block), so `perf_guards.rs`'s
  `a_typed_arithmetic_loop_contains_no_call` needs a term per category, and `nvs-ir`'s
  `a_hook_body_reaching_its_own_property_touches_the_slot_directly` finds the function's name in a
  landing block's `propagate` label with no recursion. Both assert over rendered IR, so `grep` for
  the *edge* (`! bb`, `propagate`), not the operator. [until: reviewed 2026-09-06]
- **A `python - <<'PY'` heredoc mangles non-ASCII in the *matched* string, so a `str.replace` on a
  `§` or an em dash finds nothing.** Writing one out is fine; an `assert s.count(old) == 1` over a
  fragment quoted from a doc comment fails with no clue why, and re-grepping the file only confirms
  the text *is* there. Use Edit for anything whose anchor is prose; a heredoc is safe only when
  every matched byte is ASCII. [until: reviewed 2026-09-06]
- **`nvs-ir` cannot synthesize a local, so "evaluate the base into a temporary and rewrite over it"
  is not a move a lowering has.** `ExprKind::Variable` holds a `Span` and `lower_expr` reads the
  name out of `self.src`, so a name no source file spells has no representation. Use
  `Lowering::staged_targets`: lower the sub-expression once, record `(span, value, ty)`, and let
  `lower_expr` answer from that table first — `aliasing_read` must answer `true` for a staged span
  and the stager must `own_temporary` a refcounted staged value; neither half is optional.
  [until: reviewed 2026-09-06]
- **`InstKind::ArrayGet` answers a missing key with `Value::default()`, so a lowering that treats
  the result as a pointer aborts.** The symptom is *"an Novis array pointer is never null"* then
  *"panic in a function that cannot unwind"* and exit 127 inside `nvs_array_set`, which reads as an
  array-module bug; `nvs_array_get`/`nvs_array_get_index` both end in `.unwrap_or_default()`, which
  is the recognition test. A borrowing read is no building block for a write path: use a helper
  whose ownership answer is the same in both cases, as `Helper::ArrayRowForWrite` retains what it
  found or allocates what it did not.
  [until: gone crates/nvs-runtime/src/array.rs:unwrap_or_default]
- **`grep -rn <pattern> .` from the repo root walks `target/` and times out.** It is minutes of I/O
  over build artifacts for an answer the source tree gives in under a second, and the shell call
  comes back with nothing at all. Name the roots (`grep -rn <pat> crates docs tests`), use the
  harness `Grep` tool, which respects the ignore file, or `python tools/peek.py
  "crates/**/*.rs:re:pattern"` for several such questions in one call. [until: reviewed 2026-09-06]
- **`holes.py --item N` groups a refusal site by the *file* an item anchors, so a site in a file
  another item names is filed there.** The tool's totals are right; only the attribution is a guess,
  so an item reads as closed while its headline half is unwritten. When an item's prose names a
  shape, grep the shape (`grep -rn '&\$x' crates/nvs-ir/src`) before believing the site list is the
  whole item. [until: gone tools/holes.py:--item]
- **`Lowering::untag_receiver` is unchecked, so a *new* erased receiver may not go through it.**
  Every tagged receiver that reaches a member arrives with a tag `nvs_types` already proved, so a
  `mixed` receiver is the first with no proof, and `Untag` over an `int` payload is a pointer the
  next instruction dereferences, not a panic a test log would show. `ReceiverProof` in
  `crates/nvs-ir/src/lower/expr.rs` is the switch; put the check in the runtime helper that already
  checks the *name*, not in a fallible untag, which would need the receiver staged as an owned
  temporary first. [until: gone crates/nvs-ir/src/lower/expr.rs:ReceiverProof]
- **Only the *outermost* annotation has a recorded checked type, so `lower_decl_type` over a nested
  `Type` node answers from the AST.** `nvs_types::lower::lower_type` calls `record_type` once at its
  entry and recurses through `lower_type_at_depth` without recording, so a lowering reaching
  *inside* an annotation (`?T`'s target, a union member, an `array<T>` element) gets the `match
  &ty.kind` fallback, where every name-shaped atom is `Ty::Object`. Read the whole annotation's
  `declared_ty` and take the piece off the checked type — `?T` interns as `T|null`, so its target is
  that union minus `CheckedTy::Null`. [until: reviewed 2026-09-06]
- **A refusal for a *name-shaped* expression fires on every `Foo::bar()` unless the class side is
  taken off the value walk first.** `Class::method()`, `Class::CONST`, `Class::$prop` and
  `Class::class` carry the class as an ordinary `Expr` of kind `ExprKind::ConstFetch`, so a walker
  that recurses into it reports "a bare name is not a value"
  there. `nvs_hir::members::walk_class_side` is the helper over those sites in `walk_expr` and
  `nvs_types::expr::check_expr`; nothing catches a miss but a conformance case reporting an extra
  error. [until: reviewed 2026-09-06]
- **A value handed to an `rule:classes/property-hooks` `set` hook is *transferred*, so there is no
  "afterwards" to retain it in.** Every other assignment target keeps owning what was stored, so
  four of the five arms of `Lowering::lower_store` can retain after the store; a `set` hook is a
  call that hands the callee the reference, so a retain after it can read a value the hook already
  released, and only a valgrind fixture whose hook *discards* its argument shows it. `lower_store`'s
  `extra_owner` flag exists for that: the retain is emitted where each arm still holds a reference,
  which for that arm is *before* the call. [until: reviewed 2026-09-06]
- **A `splice.py` patch cannot carry a patch, so an edit to prose *about* the patch format is one
  the `Edit` tool has to make.** A block runs from `<<<<<<< OLD` to the first `=======` after it, so
  a block whose own text quotes those markers ends in the middle of itself. The failure does not
  look like a parse error: it looks like a stale anchor in whatever file the truncated block landed
  on. [until: reviewed 2026-09-06]
- **A full diagnostic band's max-plus-one belongs to another phase, and `brief.py`'s "next free"
  line prints it anyway.** The types band has filled twice — at `E0499` and at `E0799` — and each
  time the printed next number (`E0500`, `E0800`) read as another phase's band or as none, so a new
  band was opened instead. Do not take the printed number when the band's last code says it is full;
  opening a band is a project-level decision recorded in `docs/adr/README.md` § *Decisions taken at
  project start*, and the band table atop `crates/nvs-diagnostics/src/lib.rs` is the current state.
  [until: reviewed 2026-09-06]
- **A `## Next group` item's rationale about PHP behaviour is a hypothesis, not a specification, and
  one `php -r` settles it.** An item claiming "`finally` must still run on `exit`, which makes this
  an unwind" framed a fourth unwind kind, while `php -r 'try { exit(3); } finally { echo "f"; }'`
  prints nothing and exits 3, so the cheap shape — a helper whose success is a non-`OK` status — was
  the PHP-exact one. Priority 2 decides these and PHP is on `PATH`: run the twin *before* costing
  the design. [until: reviewed 2026-09-06]
- **`holes.py`'s per-item refusal sites are the file's *catch-all* panics, so an item can read "N
  sites standing" after the feature runs.** `lower_expr`'s and `emit_binop`'s "got {other:?}" arms
  are attributed to whatever item anchors their file and will still be there when the last hole
  closes. The tool ranks *candidates*; the ground truth is four lines in a scratch
  `.agent-tmp/*.nvs` and one `nvs run`, and `holes.py --cases` is the half that does not lie — a
  named case either exists on disk or does not. [until: gone tools/holes.py:--cases]
- **A fact `nvs-ir` and `nvs-runtime` both need lives in one of them and is held to the other by a
  test in `nvs-codegen`.** Neither crate names the other, so a shared constant has no crate both can
  see; `nvs-codegen` sees both, which is why `FN_ARITY`/`CLOSURE_ARITY_SLOT` are a pair with a
  codegen test between them and why the closure parameter-tag nibbles
  (`nvs_ir::lower::param_tag_nibble` writing, `nvs_runtime::Tag` reading) are held by
  `param_tag_nibbles_are_the_runtime_tag_bytes` in `crates/nvs-codegen/src/ty.rs`. A `pub fn` in
  `nvs-ir` plus a `#[test]` in `nvs-codegen` is the shape, not a new dependency edge.
  [until: reviewed 2026-09-06]
- **A closure object's slot layout has a third party, and it is a test in `nvs-stdlib`.**
  `closure_of` in `crates/nvs-stdlib/tests/allocation_policy.rs` hand-builds a closure with the
  reserved slots and a Rust `invoke`, so adding a reserved slot in `nvs-ir` breaks it as `field slot
  N is out of range for a class with N slots` from `nvs_runtime::object`, three crates from the
  edit. `grep -rn CLOSURE_ARITY_SLOT --include=*.rs crates/` finds every builder in one call; do
  that before moving the layout, not after. [until: reviewed 2026-09-06]
- **`nvs_ir::Ty` is `#[non_exhaustive]`, so a `match` on it outside `nvs-ir` cannot be exhaustive.**
  The "a new representation is a decision, not a default" guard can only live in `nvs-ir` itself,
  and `nvs_ir::lower::param_tag_nibble` already *is* that guard. `nvs-codegen`'s `clif_ty` and
  `tag_of` end in a `_ =>` arm because the compiler requires one there, so do not try to hold a
  second copy in `ty.rs`. [until: gone crates/nvs-ir/src/ty.rs:non_exhaustive]
- **A hole item names one spelling, and the panic can be under a different one.** An item saying
  `$a?->b = v` panics can be long closed while `$a?->b++` still does, because `check_write_target`
  was reached from the assignment arms and not from `PreIncDec`/`PostIncDec`; likewise a catch-all
  naming one route (`mixed`) is reached by every receiver `class_qname_of` cannot resolve. Run one
  scratch file per spelling and per receiver family against `target/debug/nvs.exe` before editing
  the site the item names; `git log -S` on the code it quotes says whether its half already landed.
  [until: reviewed 2026-09-06]
- **A `splice.py` anchor copied out of `peek.py`'s output can carry a line break `peek` added.**
  `peek` wraps a long prose line for display, so a two-line anchor taken from a `.md` file may be
  one line on disk, and the refusal reads *"the anchor matches for its first 100 character(s) … the
  anchor wants: ''"*. Keep a prose anchor inside one displayed line or take it from `grep -n`;
  `splice.py` matches exactly, trailing newline included, so strip the one the Write tool ends a
  patch file with when splicing mid-paragraph. [until: reviewed 2026-09-06]
- **`holes.py` keys on the refusal *phrase*, not on the macro.** A site reworded from `panic!` to
  `unreachable!` still counts as a hole while its message matches `REFUSAL` in `tools/holes.py`
  ("does not lower", "only lowers", "no lowering for"); there is no allowlist and no attribute. The
  last edit of a slice that answers a hole at the checker is to reword the assert to state what
  reached lowering and name the code that refuses it ("… reached lowering: … refuses this where it
  is written, as `E0494`"), or the worklist you quote is one higher than the tree.
  [until: gone tools/holes.py:REFUSAL]
- **A `?T` row that can carry a fault must be emitted with `emit_fallible`, or a `catch` round the
  conversion never sees the throw.** `Lowering::convert_or_null` emitted every row with a plain
  `emit` because "the helper answers `null` where the throwing row would throw" — false once `as
  ?string` could run the operand's own `toString()` — and a non-fallible `HelperCall` discards the
  status word, so the program prints `Uncaught Exception` *with* a `catch (Throwable $e)` around it.
  When a new row makes a uniform emit site fallible, the scratch file to write throws from inside
  the new row and catches it. [until: reviewed 2026-09-06]
- **A `Core` class can render as text without a `toString` row, and the second rule is
  `nvs_runtime::is_carrier`.** `Core\Cli\Text` and `Core\Html\Markup` are sink carriers, so
  `rule:security/capture-answers-the-carrier` renders them as the bytes they hold —
  `value_to_string`'s `Tag::Object` arm does it, asking for no member — and a refusal written off
  the registry's member rosters alone turns `Core\Out::capture` cases red at the full verify.
  `nvs_stdlib::registry::class_renders` joins the two rosters; a rule stated over `registry.rs`'s
  rows is not the whole rule where `nvs_runtime` answers for a class itself.
  [until: reviewed 2026-09-06]
- **A diagnostic constant named in a doc comment may not exist, so a `grep` for the name does not
  prove the rule is future work.** `reject_unrelated_class_conversion`'s comment named
  `E_MARKUP_NOT_LITERAL` while the code is `E_MARKUP_REQUIRES_LITERAL` in `nvs_types::expr::quals`,
  and `rule:core-classes/html-auto-escape`'s `"lit" as Core\Html\Markup` was a checked row that
  still panicked one crate down, which made it look unimplemented from `nvs run`. Grep
  `crates/nvs-diagnostics/src/lib.rs` for the band's `Code::new` rows, or the crate's own `tests/`,
  for the behaviour. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check runs before the program legs only if its `stage` string starts with
  `0`.** `tools/loop.py` builds its catch-up class with `startswith("0")` (a stage naming `floor`
  also counts); position in the file and what the prose calls the stage do not matter, so a `"S
  inout"` block sitting above the Stage 0 blocks still runs after the whole floor. Name a stage that
  must be reported first `0`-something (`"0a inout"`), say in its comment that the digit is
  load-bearing, and read the real run order with `python tools/loop.py --list`.
  [until: gone tools/loop.py:startswith("0")]
- **A Python rename script must open with `newline=""` at both ends, or it rewrites every line of
  every file it touches.** `Path.read_text()`/`write_text()` default to universal newlines on the
  way in and `os.linesep` on the way out, so on Windows an LF file comes back CRLF and `git diff
  --numstat` reads the whole file as changed, burying the lines you meant. `git status` saying
  *"CRLF will be replaced by LF the next time Git touches it"* is the whole diagnosis;
  `tools/splice.py` gets this right, and a one-off script beside it does not inherit that.
  [until: reviewed 2026-09-06]
- **`cargo insta accept` does not exist on this box, and the failure reads as a mistyped cargo
  subcommand.** The error is *"a command with a similar name exists: `init`"*; `cargo test` still
  writes each pending snapshot as `*.snap.new` beside its `*.snap`, and accepting one by hand is
  copying it over its neighbour minus the `assertion_line:` header line. Run `git status --short`
  first and `diff` each pair: any `.snap.new` you did not just produce — a previous session's
  renamed test, say — is one to delete, not accept; `cargo install cargo-insta` fixes it for every
  future session. [until: reviewed 2026-09-06]
- **A new `nvs_ir::Helper` variant is four edits and the fourth one is not a `match`.** Three are
  exhaustive matches the compiler makes you write — the variant, `nvs-ir`'s `print.rs` name and
  `nvs-codegen`'s symbol name — and the fourth is a row in `nvs_runtime::helpers::symbols()`, a
  hand-kept `Vec` nothing checks; miss it and everything builds and passes until the first program
  reaching the helper dies inside `cranelift-jit` with *"can't resolve symbol nvs_your_helper"*.
  Grep the symbol name you just added and expect **three** hits outside the runtime's own
  definition. [until: gone crates/nvs-runtime/src/helpers.rs:fn symbols]
- **A `gaps.py` "no case calls X" is a claim about the cases it *attributed* to that class, and
  attribution is not coverage.** `coverage`'s attribution follows produced instance types
  (`gaps.py`'s `producers`), so a value reached through a factory on another class is counted, but a
  member exercised only through a closure or a helper class the walk cannot follow still reads as
  uncalled — the number is a floor, never a count. `python tools/gaps.py --member <name>` prints the
  cases that already ask about one: one call, before writing a case the tree already has.
  [until: gone tools/gaps.py:producers]
- **A guard name that looks like a rename is often a `.nvst` case instead, and two greps settle it,
  not `cargo test -- --list`.** `grep -rhoE "fn [a-z_]+"` over a crate's `src` and `tests` gives the
  cargo half and `ls tests/conformance/*/ | grep -iE "topic1|topic2"` the case half for a whole
  stage; a name already under the toml's `nvs-suite` `cases` wants its cargo entry deleted, not
  renamed into a Rust test that will never exist. A name matching a *private function* is not a
  match, a case sharing the topic still has to be read, and a name whose claim differs from the
  landed test's is not a rename. [until: reviewed 2026-09-06]
- **A snapshot diff of only *added releases in a landing block* can still be a double release.** The
  forget that takes a transferred argument off `Lowering::owned_temporaries` has to run **before**
  `emit_fallible`, which builds the call's fault edge out of whatever is on the stack at that
  moment, and a callee releases its parameters on its *throwing* edge as much as on its normal one.
  Ask which instruction the changed `bbN` is the `! bb` of: if it is the call that consumed the
  value, the release does not belong there. [until: reviewed 2026-09-06]
- **The driver's acceptance verdict rides in the pack, and a red one outranks your item.**
  `orient.py`'s RUN section prints the last `goal check:` line out of `.loop/log.md`, and because
  the check short-circuits at its first failure, everything after a red line is unmeasured rather
  than green — a line repeating verbatim across sessions *is* the work, whatever the handoff says. A
  check naming a test that "did not run" is an open item; any other failure is a regression the run
  cannot end under, and `python tools/loop.py --goal-only` re-runs the check in one call.
  [until: reviewed 2026-09-06]
- **`holes.py`'s count is not the number of `panic!`s, and widening its recognizer to the construct
  counts engine invariants as holes.** Most panic-family sites in `nvs-ir` and `nvs-codegen` are
  invariants no program reaches ("`foreach` lost the `Env` binding it walks") and will outlive the
  last hole, and wording cannot separate the two in either direction. The recognizer takes the
  construct **and** the claim's shape ("only lowers X", "has no arm for"); the fix is to make the
  *source* declare its kind, as `CodegenError` does with `Internal` versus `Unsupported`, and an
  audit of every site is the work, not the pattern. [until: gone tools/holes.py:REFUSAL]
- **`verify.py`'s test step can be red before you touch anything, and the failure names `nvs-ir`,
  not the goal switch that caused it.** `crates/nvs-ir/tests/refusals.rs` attributes every lowering
  refusal to an open item in `docs/agent/loop-goal.md`, so installing a new goal orphans every site
  the old goal's items claimed. `python tools/holes.py --unattributed` says whether it is yours; the
  gate stops there and hides the `.nvst` trees and clippy, so run those by hand until it is closed,
  and the test rightly refuses its own allowlist as the fix.
  [until: gone crates/nvs-ir/tests/refusals.rs]
- **A fixture named in `loop-goal.toml`'s `files` but not yet on disk aborts the *whole* acceptance
  check, before the first build.** `begin` in `tools/loop.py` walks `files` and returns on the first
  missing one, so the ledger reads `0s over 1 check(s)` and not one floor check runs — a goal that
  adds fixtures for unwritten features has no regression coverage at all until every one of them
  exists. Write the fixture the moment the goal names it, red or not: the toml's own header says a
  fixture's source is not frozen precisely because its author could not compile it.
  [until: gone tools/loop.py:is missing -- the acceptance fixtures are fixed]
- **A `loop-goal.toml` check can name a test in a crate that cannot host it, and its `args` is the
  half that is wrong.** A `-p nvs-hir` check listing
  `an_implementor_without_a_no_argument_constructor_is_named` asks a question only `nvs-types`'
  checking pass can answer, since `nvs_hir::implementors` never sees a signature. Move the name to
  the check whose crate owns the diagnostic rather than inventing a test, then copy
  `docs/agent/loop-goal.toml` over `docs/agent/goals/<goal>.toml` — they are byte-identical by
  construction, and the next `goal-switch.py` restores the goal file over the live one.
  [until: reviewed 2026-09-06]
- **`holes.py` only sees a numbered item whose bold title fits on one line, and a wrapped one fails
  *silently and backwards*.** `ITEM` is `^(\d+)\. \*\*(.+?)\*\*` without `re.DOTALL`, so a title
  that wraps before its closing stars matches nothing and, because an item's body runs to the *next*
  item mark, all of its anchors are absorbed by the item above — `--unattributed` drops to 0 and the
  gate goes green over a fiction. `python tools/holes.py --item N` is the check; it prints "no item
  N" for the item you just wrote. [until: gone tools/holes.py:(.+?)\*\*", re.MULTILINE]
- **A `loop-goal.toml` stage's *comment header* can carry a rule the orientation pack never
  prints.** `orient.py` prints the item, the standing decisions and the rule sections, not the
  TOML's own comments — and a `[[check]]` block's comment can forbid the obvious fix (*"Do not
  rename one to something already green"*). When an acceptance failure names a test that "did not
  run", `sed -n` the twenty lines around its `[[check]]` block before deciding what the failure
  means; that block, comment included, is the specification. [until: reviewed 2026-09-06]
- **Two slices over the same file cannot be split into two commits, so commit the first before
  starting the second.** `session.py --wrap` stages a `## commit:` by *pathspec*, so if two slices
  both edit `routes.rs` the first section sweeps both slices' changes and the second commits nothing
  — and a "next group" shares a file set by definition, so this is the normal case. Either accept
  one commit with two clauses, which house style allows, or `git commit` slice 1 by hand once its
  own crate's tests are green and let the wrap commit the rest. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` acceptance check can fail on the *driver*, not the tree, and a failure detail
  that is a bare `True` is that.** `tools/loop.py`'s `cargo_check` once read `ordered_in`'s boolean
  as a description of what was missing, so a `kind = "command"` check failed exactly when it passed
  and printed `True` as its reason, and successive handoffs wrote it off as a stale build. When a
  failure's detail is not a sentence about your code — `True`, an empty string, a bare number — read
  the branch in `loop.py` that produced it before touching the tree. [until: reviewed 2026-09-06]
- **`orient.py`'s "THE DRIVER'S LAST ACCEPTANCE CHECK FAILED" can be a record from the *previous*
  run, already repaired.** The ledger keeps the last *recorded* result in `.loop/log.md` across the
  `## run started` boundary, so a failure repaired at the end of a run is what the next run's first
  session reads. Two calls settle it: `git log --format=%ad --date=iso <the fixing commit>` against
  the `started:` line the pack prints, and running the check's own `argv` by hand.
  [until: gone tools/orient.py:LAST ACCEPTANCE CHECK FAILED]
- **`python tools/gaps.py --coverage` does not list every registered class, and the classes it drops
  are the ones a floor gate finds.** `gaps.py` reads class names out of the Rust source with
  `CLASS_RE`/`NAME_CONST_RE` rather than out of the registry, so a class whose `name:` const it
  cannot resolve vanishes silently — unranked, uncounted, unreported. Treat the tool as a *ranking*
  over most of the tree: check `registry()` against `grep -c CoreClass` before believing a total,
  and read `nvs_stdlib::registry::CLASSES` directly for anything that must hold of **every** member.
  [until: gone tools/gaps.py:NAME_CONST_RE]
- **The `regex` crate supports no look-around, and this tree carries `fancy-regex` for exactly
  that.** A `(?!...)` pattern compiles and then panics at run time with `error: look-around ... is
  not supported`, naming the caret position and not the crate; both are `nvs-stdlib` dependencies,
  so the failing import and the working one differ by one word. For a right-hand boundary alone
  neither is needed: a `match_indices` walk plus one `chars().next()` check, as `corpus::mentions`
  in `crates/nvs-stdlib/tests/corpus/mod.rs` does.
  [until: gone crates/nvs-stdlib/Cargo.toml:fancy-regex]
- **`gaps.py --errors` reads past a `Fault::` call into the next item's doc comment, so one of its
  rows is a phantom.** A `Fault::thrown_as(...)` whose message is `format!`ed above the call carries
  no literal, and the tool's window finds the *following* function's `///` prose instead, listing a
  stem no case can contain. `conformance_coverage.rs` stops its window at a line-leading `///`, so
  the two counts differ by that row; a message built above the call is outside both and cannot be
  matched by its stem. [until: gone tools/gaps.py:--errors]
- **`DECLARATION_WINDOW` is 8 lines counted from the `Fault::` line, not from the guard's first
  line, so a nine-line comment sits silently outside it.** `conformance_coverage.rs` slices the
  eight lines above the `Fault::` call, so a `Fault::thrown(format!(` two lines below its `let`
  leaves seven for the comment, and a doc comment on the enclosing `fn` is outside once the body
  does anything before the guard. Put the declaration in the body immediately above the `let`,
  within seven lines — the failure reads as the phrase missing when it was one line too high.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
- **`python tools/check-migration.py --seed` is a candidate list, not rows to paste.** It attributes
  every backticked PHP name on a line to whatever that line is *about*, so a member's own prose
  drags its neighbours in (`acos` → `Core\Math::asin`), and a right member name against the wrong
  PHP function passes the checker silently. Fill a domain the other way round: read
  `01-core-library.md`'s table for the class once, then write one row per name `--report` still
  lists. [until: gone tools/check-migration.py:--seed]
- **A goal switch orphans whatever a carried check's green depended on, and `cargo test -p nvs-ir
  --test refusals` is where you find out.** `goal-switch.py` carries the outgoing goal's `[[check]]`
  blocks forward and its unclosed items not at all, so an `nvs-ir (no refusal left)` check arrives
  without the item list that attributed its sites — a red in a crate the session never touched.
  `python tools/holes.py` reads `docs/agent/carried-refusals.md` as a second item source (`--item
  901` and up); before writing anything, `python tools/holes.py --unattributed` says whether the
  sites are new. [until: gone tools/holes.py:carried-refusals]
- **A `cargo-named` `loop-goal.toml` check names *test names*, so a test that pins the same
  behaviour under a different name does not close it.** The driver reports `did not run` every
  iteration while the assertion sits green on disk under its own spelling. `grep -rn` the drafted
  name across `crates/` before writing anything, then **rename** the existing test rather than
  adding a second copy — a `///` in a third file may already link the drafted name, and two tests
  asserting one thing is how the next session loses an hour. [until: reviewed 2026-09-06]
- **A `[dev-dependencies]` addition owes `deny.toml` an answer but owes `THIRD-PARTY-LICENSES.txt`
  nothing, and the two are checked in opposite directions.** `tools/gen-attribution.py` walks normal
  and build dependencies only, so `--check` stays green after one and there is nothing to
  regenerate; `cargo deny`'s `[graph] all-features = true` license-checks every crate that reaches
  `Cargo.lock`, optional ones no target builds included. Run `cargo deny check`, or read the new
  licences with `cargo metadata --format-version 1 --all-features` against `deny.toml`'s `allow`
  list, before CI does. [until: gone deny.toml:all-features = true]
- **A `python - <<'PY'` heredoc eats a backslash, so a patch or probe script cannot carry a Rust,
  Novis or `Core\X` escape.** The quoted heredoc is the construct you reach for because it is
  supposed to pass bytes through, yet `\\x` still arrives as `\x` and `Core\Xml` as `CoreXml`: the
  script matches nothing (no error), dies with `SyntaxError: truncated \xXX escape`, or reports
  every class missing. Write the script or the text with Write to a file under `.agent-tmp/` and
  have the shell only *name* it, or use Edit and `python tools/splice.py --patch`, which never cross
  a shell. [until: reviewed 2026-09-06]
- **An edit made after `verify.py --start` invalidates the run, and `fmt` is where you find out.**
  Each step reads the tree at whatever moment it runs, so a `cargo fmt` or a module-doc paragraph
  after `--start` gives a red verdict about a tree that no longer exists. There is nothing to debug:
  `--start` marks the end of editing, so `--start` again once the tree is final and only then write
  prose. [until: gone tools/verify.py:--start]
- **Two sessions in one tree: a file both edit is committed by whichever stages it first, with the
  other's hunks inside.** The loop may hold the tree while you do, and staging a whole file takes
  the foreign hunks along. `git status --short` a file before editing it; if it is already dirty
  with hunks that are not yours, wait for that session's commit or stage your own hunks alone — `git
  show HEAD:<path>` plus your change through `git hash-object -w --stdin` and `git update-index
  --cacheinfo 100644,<blob>,<path>` stages a version the working tree never holds.
  [until: reviewed 2026-09-06]
- **`rand_core` 0.10 renamed the core trait and inverted which half you implement.** `RngCore` is a
  deprecated stub; `rand::Rng` is the infallible trait and a *blanket* impl over `TryRng<Error =
  Infallible>`, so `impl rand::Rng for T` is `E0119` and leaving out the `TryRng` half is `E0277` —
  two errors for one mistake. Implement `TryRng`'s three `try_*` methods and take `Rng` and
  `rand::RngExt` for free; `RngExt` needs a `Sized` receiver, so `&mut dyn rand::Rng` offers only
  `next_u64`, which is what `crates/nvs-stdlib/src/random.rs`'s `Generator` newtype is for.
  [until: reviewed 2026-09-06]
- **`gaps.py`'s per-member numbers are cases already written, not cases owed, so its *thinnest
  members* ranking names finished work.** The floor is 3 and the count is per member, so a class
  whose every question has been asked keeps being nominated, and a handoff group built off the
  column reads as work to do. Before writing a case, `ls tests/conformance/core/<class>-*` and `sed
  -n 2p` each sibling's claim line; what still finds room is a claim a module's `//!` doc or
  `MethodDoc` argues for that no case under `tests/conformance/core/` asserts.
  [until: gone tools/gaps.py:thinnest]
- **`peek.py --locate` is a mode, not a flag you can add to a read: every `path:target` in the same
  call is silently dropped.** `--locate` takes the rest of argv as symbols, prints their anchors and
  nothing else, and a symbol it cannot find exits 1 — so the windows you batched read as files
  holding nothing. Ask for anchors in their own call, or use a `file.rs:re:fn name` target, which
  prints `file:line` and the matching line beside the other windows.
  [until: gone tools/peek.py:--locate]
- **A `Files`-style trait in a crate is the seam a filesystem question goes through, and its test
  fakes are where the question is actually asked.** Adding a method such as
  `nvs_config::resolve::Files::canonical` stops every fake compiling, and a fake that answers
  *lexically* silently makes every case a statement about paths no symlink was in — `Path::exists`
  `stat`s rather than `lstat`s, so a fake `exists` must follow links too. Grep `impl <Trait> for`
  before adding a method, and give the fake the resolving behaviour rather than the identity one.
  [until: reviewed 2026-09-06]
- **`Edit` strips a trailing space from `new_string`, so a `replace_all` that narrows a keyword eats
  the space after it.** `pub const ` → `pub(crate) const ` arrives as `pub(crate) constMAGIC`, and
  the same edit re-applied to repair it is refused as "old and new are identical" because the tool
  compares the stripped strings. Include the following identifier in both halves, or write the run
  as one `python tools/splice.py --patch` file. [until: reviewed 2026-09-06]
- **A `Resolved`/`Snapshot` field is not enough to make a value reach a reader: `Snapshot::retype`
  rebuilds the typed tree from the merged *table*.** Anything the resolver puts on
  `Resolved::config` and nowhere else is dropped at the snapshot boundary, and a reload retypes
  again, so a value put back once at build is lost on the next carry. Carry the value beside the
  table and re-apply it inside `retype`; before believing a value is lost in the resolver, check
  `Snapshot::table` — `nvs config dump --toml` prints exactly that.
  [until: gone crates/nvs-config/src/snapshot.rs:fn retype]
- **`INSTA_FORCE_UPDATE=1` rewrites every `nvs-ir` snapshot, not the ones your change moved — use
  `INSTA_UPDATE=always` alone.** The extra variable rewrites snapshots that *pass* as well, and
  stale `source:` headers come back as a one-line diff on each that buries the ones that matter.
  `INSTA_UPDATE=always python tools/verify.py -p nvs-ir` touches only what differs; if it already
  happened, `git checkout --` the rest before the wrap, because `session.py --wrap` sweeps
  everything unnamed into the last commit. [until: reviewed 2026-09-06]
- **A docs/reference/core/<Class>.md page is hand-written prose, not a render of the class's cards,
  so a finding that says "the card" means three files.** The fact can be right in the `MethodDoc`,
  right in the `docs/reference/lang/` chapter and wrong on the class page, and nothing checks them
  against each other. Check all three and let the binary break the tie — a probe under `.agent-tmp/`
  against `target/debug/nvs.exe` costs one call and is the only copy that cannot be out of date.
  [until: reviewed 2026-09-06]
- **A loop-goal.toml check can fail on a file no session wrote — the user edits this tree too.** An
  uncommitted hand edit to a spec or an inventory the check reads can be right and still fail it,
  and staging it to make the check pass takes the user's in-flight work into your commit. `git
  status --short` before diagnosing says whose change it is; a fix that lands in a *tool* —
  `check-migration.py`'s `AHEAD_OF_THE_BUILD` is the shape — is committable on its own without
  touching those files. [until: reviewed 2026-09-06]
- **A handoff item's `file.rs:NN` anchors are inlined by `orient.py`, so a stale one arrives as code
  that does not match the item's prose.** The line numbers were written before the previous
  session's own edits moved them, and the pack prints whatever now sits at the number under the
  heading "the code your item anchors". When a printed window does not match the item, do not read
  around the number: `python tools/peek.py --locate <symbol>` or a `:re:` target lands first time.
  [until: gone tools/orient.py:ANCHOR_RE]
- **An `Edit` anchored on a `fn` line lands *inside* that function's doc comment, and nothing
  reports it.** An item inserted before `fn name(` splits the `///` block above it: the head becomes
  the new function's documentation, the tail the old one's, and `cargo check` is green because both
  are well-formed. Rust has no marker for where a doc block starts, so anchor on the blank line
  after the previous function's closing brace, or on the function's own first `///` line, and put
  the new item before it. [until: reviewed 2026-09-06]
- **`refusals.rs`'s `CEILING` never rises, so a new `assert!`/`panic!` refusing a shape in `nvs-ir`
  or `nvs-codegen` turns `verify.py` red.** `crates/nvs-ir/tests/refusals.rs`'s message names a
  count against the ceiling and no file; `python tools/holes.py --sites` lists every site and the
  new one is obvious. Before writing the guard, check whether an existing site already refuses the
  shape — run the fixture and read the panic; an internal-consistency `panic!` is not counted, a
  guard against a language shape is. [until: gone crates/nvs-ir/tests/refusals.rs:CEILING]
- **A fixture that exits nonzero *by design* fails the valgrind sweep, and the failure reads as a
  leak.** `tools/loop.py` runs `valgrind --error-exitcode=1` and grades the fixture on its exit
  status, so a `FATAL` fixture reports as `valgrind <file>: exit 1` with its own stderr as the
  "error". Check whether the fixture's own `[[check]]` says `exit = "nonzero"` before looking for a
  leak, and name it under `[valgrind] skip` in both the live goal file and its `docs/agent/goals/`
  twin — that entry is load-bearing and a goal switch has lost it before.
  [until: gone tools/loop.py:error-exitcode]
- **A missing acceptance *fixture* aborts the whole acceptance run before a single check runs, and
  that is the ordinary state of a goal's first session.** `loop.py`'s `begin` walks the goal's
  `files` list and returns on the first path not on disk, so the ledger reads `0s over 1 check(s)`,
  every other check measures nothing, and the one-line report reads like one fixture failing. Write
  every file the `files` list names in one slice, as the program it is meant to be, and let each
  fail its own `exact` check with a real diagnostic. [until: gone tools/loop.py:def begin]
- **A loop-goal.toml check can name something that is not a test at all, and the tell is the *check
  count* rather than the message.** A `cargo-named` check listing an `examples/*.nvs` path beside
  real test names fails forever, since a program leg cannot appear in `cargo test`'s output, and the
  driver stops there — `over 4 check(s)` in `.loop/log.md` where a healthy iteration reads over a
  hundred. Check that a name *could* be a `#[test]` before writing one, and fix
  `docs/agent/goals/<goal>.toml` alongside the live copy, or the next `goal-switch.py` carries it
  back. [until: reviewed 2026-09-06]
- **A `Fault::` message whose format string *starts* with an interpolation is invisible to the
  error-path coverage gate, which then passes by accident.** `conformance_coverage.rs`'s
  `fault_sites` takes the stem before the first `{` and drops any under 14 characters, so
  `format!("{MEMBER} refused {arg:?}")` is never owed a case; and the match is
  `corpus.contains(&stem)` over raw `.nvst` text, so a stem holding `Core\IO::` needs a backslash no
  Novis string writes unescaped. Open a refusal with a backslash-free sentence naming the rule and
  put the member name in the interpolated tail.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:OWED_A_CASE]
- **`python tools/disk.py --clean` refuses while `.loop/running` exists, which is every loop session
  — so a full disk inside one is cleared by hand.** The failure does not read as a disk failure:
  `cargo test` reports `LNK1140`, a `STATUS_STACK_BUFFER_OVERRUN` out of `rustc`, and only at the
  end an `os error 112`. The refusal is about a *concurrent build*, not the driver: check
  `Get-Process cargo,rustc,link`, then `Remove-Item -Recurse -Force target/debug/incremental`, a
  pure cache costing one cold build — and run `python tools/disk.py` before a slice that adds a
  dependency. [until: gone tools/disk.py:RUNNING]
- **A `.rs` in the working copy with CRLF fails a gate that names something else entirely.** Write
  on Windows, `Path.write_text`, or `splice.py` writing LF into a CRLF file all produce one; git
  normalises on commit, but `every_error_path_is_asserted_or_declared_unreachable` reads `Fault::`
  literals off disk, a `\`-continued string keeps the CR, and the failure names a message far from
  your edit with `\r\n` in the printed stem. One `python -c` byte rewrite of `\r\n` to `\n` fixes it
  (`newline=""` in a script prevents it); never `git stash` to bisect, because the loop driver may
  hold the tree. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:fault_sites]
- **An item names one of its stage's checks, and the stage names the rest — including ones in
  another crate.** The orientation pack prints the item, not the stage, so a `cargo-named` check
  over a second crate whose test has never been written is the same slice's, and the driver stops at
  the first failure, holding the whole acceptance list at that stage. One `grep -n -i <topic>
  docs/agent/loop-goal.toml` before starting lists every check the slice owes.
  [until: reviewed 2026-09-06]
- **`peek.py` takes its options *after* its targets, and mixing them is "unrecognized arguments"
  naming a target that is perfectly well formed.** The positional list is `nargs="*"` and closes at
  the first flag, so `peek.py A.rs:re:x --window 4 B.rs:re:y` fails on `B.rs:re:y` and reads as a
  malformed target; re-quoting, escaping the `::` and dropping `re:` all fail the same way. Put
  every target first and every `--window`/`--in` last. [until: gone tools/peek.py:--window]
- **An acceptance check reading `E0405: Core\X has no member named y` can be a stage nobody has
  started, not a regression.** `nvs_hir::qname::is_reserved_global_class` knows the *name* while no
  module registers a member, so every member of an unwritten class fails with that sentence, and
  because the program legs stop at the first failure, a fixture going green moves the line to the
  next `exact` check, however far ahead. One `ls crates/nvs-stdlib/src/<x>.rs` separates "a row went
  missing" from "the stage is unstarted"; everything behind that fixture is dark until it lands.
  [until: reviewed 2026-09-06]
- **A `did not run` naming an *early* stage is the acceptance frontier having moved backwards past
  the handoff's next group.** The driver walks the stages in order and stops at the first failure,
  so closing a later check can expose a test an earlier stage named that never existed, and every
  session since had been reading a report that happened to name something later. The tell is the
  check count in `.loop/log.md` rising as earlier stages pass; grep the named test across
  `crates/**/*.rs` and read the check's `tests` list in `loop-goal.toml` before taking the handoff's
  group at all. [until: reviewed 2026-09-06]
- **A `"` inside a `peek.py` pattern is eaten before Python sees it, and the error names the *regex*
  rather than the quoting.** Windows re-quotes a native command's arguments after PowerShell has
  finished with them, so an embedded double quote opens a quoted region that swallows the next
  target too — `bad regex /name: [a-z] other:re:x/: multiple repeat`, a regex visibly two targets
  joined. Write the pattern without `"` (`name: .[a-z]` matches the same rows) rather than hunting
  for an escape that survives both layers. [until: reviewed 2026-09-06]
- **`docs/agent/loop-goal.toml` and `docs/agent/goals/<goal>.toml` drift, and the live one is the
  half that is right.** They are byte-identical by construction and the next `goal-switch.py`
  restores the goal file over the live one, so every session that improves a check and does not copy
  it back is queueing a silent revert of module paths, `[docker]` services or a fixture's
  expectations. `git diff docs/agent/goals/` right after the `cp` is the whole check, and what it
  prints is other sessions' work about to be lost. [until: reviewed 2026-09-06]
- **A loop-goal.toml acceptance test can name *two* bounds, and splitting it across two `#[test]`s
  fails the check.** A `cargo-named` check matches the test's *name*, so
  `the_engine_floor_rotates_and_rate_limits_itself` split into `..._rotates` and `..._rate_limits`
  reads better, passes `cargo test`, and leaves the driver reporting `did not run` forever. Read the
  `tests = [...]` list before deciding how many functions the work becomes, even when it is two
  slices and two mechanisms. [until: reviewed 2026-09-06]
- **`nvs-ir`'s refusal ceiling is a ratchet over what a panic's *message says*, so rewording one can
  turn `verify.py` red with no new site.** `tools/holes.py`'s `REFUSAL` regex matches `does not
  (yet) lower`, `only lowers`, `has no arm for` and their siblings inside any `panic!`/`assert!`
  literal, so adding "which this crate does not lower yet" to a consistency panic raises the count.
  The tell is a red `-p nvs-ir --test refusals` while `python tools/holes.py` reports `UNATTRIBUTED:
  0`; a claim of a lowering gap goes in the crate docs' known-gaps section, and the panic keeps what
  it said. [until: gone tools/holes.py:REFUSAL]
- **The handoff's own item text can contradict a settled rule, and the rule still wins — anything in
  the pack that is status can be stale about a decision.** An item asking for a new diagnostic code
  or a new capability row is usually asking a question the cited `rule:` already answers, and the
  handoff was written by a session that had not read the section yet. One `peek.py <rule>:"## N"` on
  every section the item cites, *before* writing the row, is the whole check.
  [until: reviewed 2026-09-06]
- **A `peek.py` `re:` target over a wide glob prints every matching file's hits, and
  `tests/conformance/**/*.nvst` is over a thousand of them.** One probe for a spelling across the
  corpus comes back as tens of kilobytes, because every matching file prints its hits with context.
  Name one file, or ask a question whose answer is a handful of lines; when the question really is
  "what is the spelling for X across the corpus", send a subagent, whose findings are three lines
  and whose reading is enormous. [until: reviewed 2026-09-06]
- **`tools/reference.py` reads the *built* binary, so a card edit it has not been rebuilt for is
  reported as `docs/novis.md unchanged`.** The generator asks `nvs meta` for the registry rather
  than parsing `registry.rs`, and `cargo test -p nvs-stdlib` does not rebuild the binary either, so
  a card fix that passes its own gate can still ship the old sentence. After fixing a `MethodDoc`,
  `cargo build` and *then* `reference.py`. [until: gone tools/reference.py:nvs meta]
- **nvs-stdlib's gates scan `io.rs`'s *prose*, not just its rows, and two of them read a doc comment
  as code.** `no_member_dispatches_on_a_uri_scheme` fails on the string `php://stdin` inside a
  reference card's `short`, because the scan is for the scheme spelling anywhere in the module, and
  a `-p nvs-stdlib --test capability` failure naming a doc-comment line reads like a code bug;
  `no_registry_card_cites_an_adr` is the same shape one file over. Name what PHP's spelling *did*
  ("the standard-input wrapper") rather than writing it. [until: reviewed 2026-09-06]
- **A session that died leaves its whole slice on disk, and the next pack hands you the item as if
  nothing had been done.** A session that exits without a commit and without `.loop/status.txt`
  leaves `orient.py` quoting the same handoff to the next one, whose item is the unverified work
  already in the tree; the tell is a `.loop/log.md` session line with `(no status written)`. `git
  status --short` **before the first edit** — then the owed work is `verify.py`, the commit
  messages, and the next slice, not a second implementation. [until: reviewed 2026-09-06]
- **`tools/try.py` runs the `--FILE--` body and the `--FILE nvs.toml--` body as one program**, so
  any `.nvst` case carrying a capability grant fails there with invented diagnostics —
  `error[E0319]: read is not a constant that exists` pointing at `read = ["."]`, which reads exactly
  like a name-resolution bug in the case you just wrote. Every `Core\IO` case has that section, so
  nothing is wrong with the case. `target/debug/nvs.exe test <path.nvst>` understands the format,
  takes a single file, and is the same harness `verify.py` uses; keep `try.py` for a snippet with no
  config block. [until: exists tools/try.py:nvs.toml]
- **`cargo metadata` reports a build dependency whether or not the feature that uses it is on, and
  `links` is not a C signal at all.** `python tools/gen-attribution.py --check-c-deps` over the
  resolved graph lists `blake3`'s `cc` build-dependency even under `features = ["pure"]`, and
  `defmt`'s `links` is Cargo's one-version token rather than a native library. An enumeration is a
  **signal** and the ledger entry is the finding: a `no-native-code` verdict resting on a feature
  names it in `C_DEPENDENCIES`'s `requires` column, and the crate's own `Cargo.toml` comment settles
  each in one look. [until: gone tools/gen-attribution.py:C_DEPENDENCIES]
- **A loop-goal.toml `nvs-suite` check can name `.nvst` paths in a directory layout the corpus never
  adopted.** The failure reads exactly like unwritten work; the corpus is flat directories with a
  subsystem prefix on the file name (`tests/conformance/core/io-…`), not `tests/conformance/io/…`,
  and a drafted name describes the *claim*, which is usually already pinned under the name the
  corpus took — sometimes as an `nvs-stdlib` `#[test]`, when it is a panic no `.nvst` can survive.
  `sed -n 2p` the plausible neighbours before writing a case to satisfy a name.
  [until: reviewed 2026-09-06]
- **`check-migration.py` validates a `Core\X::y` cell against `01-core-library.md`'s *backticked
  words*, not against the registry.** Its extraction takes a bare backticked word, a `name(` prefix
  or `$var->name`, so a member the spec spells only as `implementing<T>()` or `->readLine` fails
  with "which 01-core-library.md does not" — a spec-side gap that reads as a misspelling. Do not
  widen the regex: write the handle form (`$file->readLine`), name the class and leave the member in
  prose, or give 01 a machine-readable spelling; `core_surface()` via `importlib` answers a whole
  table up front. [until: gone tools/check-migration.py:core_surface]
- **`cargo deny check`'s advisories leg can fail on a crate no session added, and the failure
  arrives inside the diff of the one you just added.** A yank is a fact about crates.io that changes
  under a tree nobody touched, so this leg goes red on its own schedule, naming a transitive crate
  that has been in the lock file for months. Read the finding's own package before suspecting the
  dependency in your diff; `cargo update -p <crate>` to the next patch is the whole fix for a yank,
  in the same commit as the `Cargo.lock` you were already writing. [until: reviewed 2026-09-06]
- **A file a build step wrote — `Cargo.lock`, `docs/novis.md` — is dirty after `session.py --wrap`,
  and the wrap's sweep will not catch it.** The sweep covers files the *wrap* wrote, while `cargo`
  rewrites the lock during the work and `verify.py`'s reference step regenerates `docs/novis.md`
  from the registry's cards; the only place it shows is the tail's `uncommitted after the wrap`
  line, which reads as someone else's edit. Name `Cargo.lock` in the `## commit:` of the slice that
  touched a manifest, and `docs/novis.md` in the one that adds or rewords a `Core` member's card.
  [until: gone tools/session.py:uncommitted after the wrap]
- **`tools/splice.py` and `tools/session.py` take a patch in two different formats, and the wrap
  skeleton is the one you will have read most recently.** A splice patch is git conflict markers —
  `--- <path>`, then `<<<<<<< OLD` / `=======` / `>>>>>>> NEW` around each block — while a wrap
  file's `## plan-edit:` is `--- old` / `--- new`, and writing the wrap form into a splice patch is
  refused with a message that names the fix but not the shape. `python tools/splice.py --help`
  prints the invocation forms and the patch format. [until: gone tools/splice.py:<<<<<<< OLD]
- **A `[db.<name>]` field has a second home in the reference chapter, and `docs/novis.md` is
  generated from it.** `docs/reference/tools/20-config.md`'s block table lists every key each block
  accepts, and nothing checks it against `nvs_config::tree`: `deny_unknown_fields` refuses a key the
  struct lacks, no test refuses a struct field the table lacks, so a field added to the struct alone
  leaves the user-facing list wrong with nothing failing. Edit the chapter row in the same slice as
  the field, and let `verify.py` write `docs/novis.md`.
  [until: gone docs/reference/tools/20-config.md:password_file]
- **A handoff item can say the spec is silent about something the spec declares in a fenced block.**
  `docs/spec/01-core-library.md`'s members table cites a type (`->type(): ColumnType`) without
  defining it, and the definition sits one screen down under *Enums, settings and errors*, so a
  reader who greps the table region concludes there is none. One `grep -rn <Name> docs/` is the
  whole check, before deciding that a slice gets to invent the cases. [until: reviewed 2026-09-06]
- **`git commit -F .agent-tmp/<name>.txt` can silently commit a *previous* session's message.**
  `.agent-tmp/` is not cleaned between sessions and every session reaches for the same obvious file
  names, so a `-F` naming a file you did not write this session succeeds with someone else's subject
  line, and `git commit -F msg.txt 2>/dev/null || true` hides even the missing-file case. Write the
  message file in the same call sequence you commit it in, under a name new to this session, and
  never mask a `git commit`'s exit status; `session.py --wrap` is immune.
  [until: reviewed 2026-09-06]
- **A hand-rolled tool's `--help` can be wrong or run the tool instead, and `loop.py` reads a
  non-zero exit on it as a broken script.** The argparse tools print their flags for free, while a
  hand-rolled parser can omit a flag or ignore `--help` and run its full inventory, hiding what the
  script takes from any session that asks. `loop.py`'s `tools_still_load` probes every changed
  `tools/*.py` with `--help`, so a new hand-rolled tool must print its docstring's flag block and
  exit 0. [until: gone tools/loop.py:tools_still_load]
- **A `peek.py` window's first printed line is a bad `splice.py` anchor when it lands inside a doc
  comment.** The window starts at the line you asked for, and a wrapped `///` sentence almost always
  began on the line above, so the anchor you copy starts mid-sentence, reads perfectly, and
  `splice.py` refuses it with "the anchor matches for its first 7 character(s)" — the tiny prefix
  length against a verbatim-looking block is the tell. Ask for one line more than you think you need
  whenever the region is prose. [until: gone tools/splice.py:the anchor matches for its first]
- **`splice.py`'s `OLD` block wants the file's real indentation, and `peek.py`'s gutter is easy to
  mis-count.** `peek.py` prints `<line number><two spaces><the line>`, so a nested line reads one
  level deeper than it is, and the refusal ("the anchor matches for its first N character(s), up to
  target line M") can name a line far away, because a leading run of spaces matches elsewhere before
  the words do. Subtract two from the column `peek.py` shows, or measure it: `awk 'NR>=A && NR<=B {
  match($0, /[^ ]/); print NR": indent="RSTART-1 }' <file>`. [until: reviewed 2026-09-06]
- **The acceptance failure the orientation pack quotes is a line the ledger wrote at the time, so one
  written before `first_err_line` learned to skip warnings still names a warning rather than the
  failure.** That property now picks the failed test, then the panic, then the first `error` line,
  but nothing rewrites a line already in `.loop/log.md` — a `warning: use of deprecated method
  fetch_update` reached a session as the whole verdict on a leg whose real failure was a flaky test
  eleven lines above it. Grep `.loop/logs/<run>-console.log` for the check's name and read its whole
  block before believing the one line, because that log holds both streams and the ledger holds one
  sentence. [until: reviewed 2026-09-14]
- **`peek.py --locate` takes symbols only, and answers `NOT FOUND` for a `path:re:pattern` target
  instead of refusing it.** That is the same line it prints for a name that is nowhere in the tree,
  and a path in that list is likewise read as one more symbol to search the whole repository for,
  which can run past a 120-second timeout. Use `--locate <symbol> --in '<glob>'` for symbols, and an
  ordinary `peek.py` target or `grep -n` for a pattern. [until: gone tools/peek.py:NOT FOUND]
- **A loop-goal.toml check can name a test that no crate can host *yet*, and the two reasons look
  identical from the driver's report.** A test "did not run" either because it was filed against the
  wrong crate — a filing bug fixed in one edit — or because the feature under it does not exist, in
  which case staying open is correct. Read the *known gaps* of the crates the check names before
  assuming either. [until: reviewed 2026-09-06]
- **A [context] adrs gap does not close by naming the section in the handoff item.** `orient.py`
  slices the `[context] adrs` and `rules` lists and nothing else — it never reads an item's own
  `rule:` tokens — so a section asked for in the handoff is re-sliced by hand session after session
  while the list stays unchanged. Edit `docs/agent/loop-goal.toml` in the session that discovers the
  gap; `tools/loop.py` reloads it every iteration. [until: gone tools/orient.py:adrs]
- **`python tools/peek.py` reads inside this repository only, and a path outside it answers `NO SUCH
  FILE` rather than an error you can act on.** A glob that resolved fine in the shell a call earlier
  gets the same answer. Reading a dependency's own source
  (`~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/src/…`, found with `ls -d`) is the
  Read tool's job, and it is worth doing before writing by hand a wire field the crate already
  models. [until: gone tools/peek.py:NO SUCH FILE]
- **A `## plan-edit:` `--- old` fragment cannot span a run of two spaces in the field, and the
  refusal reads as if the words were wrong.** `session.py` normalizes its fragments with `"
  ".join(text.split())`, but `plan.py`'s `find_fields` joins the field's *lines* and leaves the
  spacing inside one, so a stray double space in the plan matches no normalized quote; the tell is a
  refusal quoting your fragment back word for word. Quote a shorter run on one side of the anomaly,
  or two runs as two pairs — `python tools/plan.py --get 'Open now'` prints the field with the
  double space invisible. [until: gone tools/session.py:def normalize]
- **A loop-goal.toml check can list a test under the wrong crate because the rule it cites lives one
  layer up.** A check citing a rule that only `nvs-stdlib` implements, filed under `-p nvs-db`,
  reads as open work forever. Read the cited rule before writing the test, and split the check
  between the crates rather than moving it whole. [until: reviewed 2026-09-06]
- **The handoff's own next-group item can already be on disk, and the tree, not the handoff, is what
  says so.** A re-scope carries an item forward without re-checking it, and a stale item reads
  exactly like an open one — down to a `loop-goal.toml` comment still saying the feature does not
  exist. Before writing a line: `git log --oneline -S '<the symbol>'`, `python tools/peek.py
  --locate <test name>` then `cargo test -p <crate> <name>`, or run the check's `cases` ahead of the
  failing one, since acceptance names only the *first* thing missing; then say "already landed" in
  the next handoff. [until: reviewed 2026-09-06]
- **A loop-goal.toml check name can be a *conjunction*, and then it reports one open item where
  there are two — one landed, one unwritable.** A conjunction reports `did not run` while *either*
  half is missing, so the landed half earns nothing and the report cannot say which one is the work.
  The tell is the `_and_` in the name: grep each half's claim separately, and split the entry into
  one name per half rather than waiting for a single test to become writable.
  [until: reviewed 2026-09-06]
- **A framed `crates/nvs-stdlib/tests/queue.rs` case runs under one filtered `cargo test`, but
  `NVS_DB_MATRIX_CA` must be an absolute path.** The six variables are the ones `tools/db-matrix.py`
  sets, their values are in `tests/db/compose.yaml`, and the anchor comes out with `docker compose
  -f tests/db/compose.yaml cp mysql:/certs/ca.crt <abs path>`. A *relative* path resolves against
  the crate directory, so the first case panics inside `open` with a bare `NotFound` and every later
  one reports `Once instance has previously been poisoned`, which reads as a broken fixture.
  [until: gone crates/nvs-db/src/matrix.rs:NVS_DB_MATRIX_CA]
- **Taking a C dependency fails `--check-c-deps` on more crates than you took, and one of them is
  never compiled.** `tools/gen-attribution.py`'s enumeration is deliberately host- and
  target-independent, so a `links =` crate reached only under `cfg(target_arch = "wasm32")`
  (`sqlite-wasm-rs` beside `libsqlite3-sys`) is reported exactly like the real one. Write it a
  `no-native-code` row naming the target gate rather than hunting for where it got linked in, and
  run `python tools/gen-attribution.py` after, not only `--check-c-deps`: `THIRD-PARTY-LICENSES.txt`
  is a second gate with a separate failure. [until: gone tools/gen-attribution.py:--check-c-deps]
- **The oracle PHP build will tell you an extension's real signatures, and a migration row written
  from memory instead is wrong in a way no test catches.** Which spelling is the deprecated alias of
  which and what a function returns are not derivable from `check-migration.py --report`'s name
  list. A six-line `ReflectionFunction` loop over `get_defined_functions()["internal"]`, run as `php
  .agent-tmp/<name>.php`, prints every parameter type, return type and `isDeprecated()` in one call;
  copy the `Core\X::y` spellings out of an already-green sibling section rather than inventing them.
  [until: reviewed 2026-09-06]
- **A `$` in a `wsl.exe -- bash -lc '…'` command string does not reach WSL, so `$?` reads 0 and a
  measured exit code is a fiction.** Single quotes do not protect it: `wsl.exe -- bash -lc 'false;
  echo "CODE=$?"'` prints `CODE=0`, and a `for f in a b; do … $f …; done` runs with `$f` empty every
  iteration, so a failing valgrind run reads as green. Use `&&`/`||` or `if cmd; then … fi`, never
  `$?`, and put the varying part in `xargs -I@`, which substitutes before any shell sees it.
  [until: reviewed 2026-09-06]
- **A goal document or a manifest comment can state a dependency's graph as fact, and only `cargo
  tree -i` knows.** A crate taken with `default-features = false` can still pull a dependency
  unconditionally for one type, and the only place this shows is the `Adding <crate>` line in the
  first `cargo check`'s resolution list. `cargo tree -i <dep> -e normal` names the parent and
  `~/.cargo/registry/src/*/<crate>-<version>/Cargo.toml` says whether that edge is `optional` or
  feature-gated — do both *before* writing the comment that claims the graph, because the comment is
  what the next reader trusts. [until: reviewed 2026-09-06]
- **The item `orient.py` hands you can already be on disk, uncommitted, from a session that died
  before its wrap.** The next pack opens on the same item as if nothing existed, and re-deriving it
  throws the work away and leaves two designs for one rule. Run `git status --short` and read the
  modified and untracked files your item names before writing a line of it; the tell is a `0
  commit(s) | (no status written)` line for the previous session in `.loop/log.md`.
  [until: reviewed 2026-09-06]
- **`splice.py` refuses an anchor that matches more than once, and a run of identical test call
  sites is exactly that.** Byte-identical argument lists give no unique anchor short of quoting out
  to each enclosing `#[test]` name. When an anchor is not unique, change the shape so the edit
  disappears — bundle the shared arguments into one struct a helper builds — rather than growing the
  anchor. [until: reviewed 2026-09-06]
- **A handoff item's "this does not exist yet" can be contradicted by the anchor window `orient.py`
  printed directly beneath it.** The item was written from the tree at drafting time and the anchor
  is resolved live, so when they disagree the anchor is right by construction. Read the window the
  pack already gave you before believing the sentence above it; a claim about the *tree* is answered
  by something already in the pack. [until: reviewed 2026-09-06]
- **A crate doc's `ADR NNNN § N` citation can be off by one, and the handoff will copy it forward
  rather than check it.** A whole block of citations can sit one section high while the group line
  that inherits them reads plausibly. `python tools/peek.py <record>:"re:^### "` prints every
  heading in a few hundred bytes; run it before writing a doc comment that cites two or more
  sections of one record. [until: reviewed 2026-09-06]
- **An anchor window — `orient.py`'s inlined code, or a `peek.py` line range — carries no `impl`
  header, so the receiver type in it is a guess.** Methods of an inner type read as the outer
  type's, the tell is `error[E0599]: no method named ...` after a full rebuild, and `peek.py
  --locate` answers the symbol, not its owner. One `grep -n '^impl ' <file>` filtered to the lines
  around the anchor says which type you are adding a call to. [until: reviewed 2026-09-06]
- **`python tools/peek.py --locate` and window targets in one call: only the locate prints.**
  `--locate` is a mode, not an extra question: every other argument is read as a symbol name, so
  windows are dropped without a word and a `file:re:pattern` target is echoed back verbatim with
  `NOT FOUND` after it, which reads like a real miss. Ask for anchors in one call and bodies in
  another, never both. [until: reviewed 2026-09-06]
- **A `cargo-named` check reports only the *first* of its missing names, so one "did not run" can be
  four tests of work.** The driver's line names one test; the check's `tests` list may hold several,
  all unwritten. Size the item from the check's whole list with one `grep -rn` over the names, which
  also separates a name the tree pins elsewhere from one nobody has written.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check whose comment says a slice is unlanded can be right, and one grep for
  the surface settles it either way.** The sibling triages assume a misfiled check; the other
  outcome is a real gap that several module docs each name as "not landed". One `grep -rn` for the
  surface's own name across the crates answers where the rule is named, where it is implemented, and
  which doc comments go stale when it lands — that list is also the edit list.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check can name a crate that *depends* on the surface and still cannot host
  the test; what settles it is what kind of test that crate's `tests/` already hold.** A manifest
  naming the dependency passes the sibling triage and says nothing: every case under
  `crates/nvs-types/tests/` compiles a source string and reads `Diagnostics` back, so a runtime fact
  — a media type declared on a response — cannot be observed there. Read the first twenty lines of
  any existing test in the named directory before the manifest. [until: reviewed 2026-09-06]
- **A failing acceptance check can name a feature with no foundation anywhere in the tree, and the
  cheapest triage is one `grep -rn` for the rule across `crates/*/src`.** The grep finds the surface
  *named* in a module doc's *Known gaps* rather than implemented or absent: not misfiled, not a
  regression, but a milestone-sized item reported as "did not run". Read the owning module's gap
  section before planning; it also says which of the names the check's crate cannot host.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check can name a *status* a rule forbids the named crate to send, and the
  sentence that settles it is in the section the check cites.**
  `rule:routing/matched-once-before-the-handler` forbids the server to send `404`/`405`; they are
  the program's decision over the table's answer. When a check name carries a wire-level effect (a
  status, a header, a close), find who may emit it before writing the test, and move the check to
  the crate owning the computation (`crates/nvs-runtime/src/routes.rs` here) with a comment saying
  why the name is not a claim about who sends it. [until: reviewed 2026-09-06]
- **`cargo fmt --all` mid-session re-prints every file you had already read.** The harness sees each
  file the formatter rewrote as changed on disk and pastes it back into context, so a formatting
  pass costs far more than its own output. `python tools/verify.py` formats as step 1 and its
  `formatted N file(s)` line is the tell; write formatted code and there is nothing to re-print, and
  never run a rewriting tool over open files except `session.py --wrap`, where the session ends.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check whose name is a conjunction can have its two halves in two *crates*,
  and the tell is that one half names a compile-time fact.** "as tainted" is a qualifier on a
  `nvs_stdlib::registry` row's signature; a crate that names neither `nvs-stdlib` nor `nvs-types`
  can carry the value but has nothing to write the qualifier with. Ask which crate can host each
  half of an "as …" or "and …" name separately, and split the check. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check can name a test for a feature no rule has decided yet, and then the
  first slice is the rule rather than the test.** Three tells: the crate has no module for the
  surface at all, the cited record defers rather than specifies, and its *Verification* names a
  later milestone than the check's stage. Do not move the check or write the test against nothing —
  decide the thing, which a `[context]`-listed goal pre-authorizes under its own § *Standing
  decisions*. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check can name a test its crate may not *write*, and `Cargo.toml` naming the
  right dependency does not settle it — read the `[lints]` block underneath.** A fixture that needs
  a leaked `ClassTable` carrying an `invoke` address (`allocation_policy.rs`'s `closure_of`) cannot
  compile under the workspace's `unsafe_code = "forbid"`, which `#[expect(unsafe_code)]` cannot
  open. `grep -rn unsafe_code crates/*/Cargo.toml` lists the crates that chose `deny`; then ask
  which crate holds the state the claim is about before moving the check.
  [until: reviewed 2026-09-06]
- **`python tools/verify.py` can fail on a *concurrent session's* half-written file, and the repair
  is never to format it.** The driver runs several sessions at once and the gate stops at the first
  failure, so the verdict says nothing about what this session wrote, and formatting another
  session's file lands their work under your commit. `git status --porcelain` is the triage; then
  run the same checks scoped to your crates (`cargo fmt -p A -- --check`, `cargo test -p A`, `cargo
  clippy -p A --all-targets`) and say in the handoff that the full gate was not reached.
  [until: reviewed 2026-09-06]
- **`peek.py --window` is a global flag, not a per-target one, so a second `--window` later in the
  same argv is an argparse error that discards the whole call.** `peek.py A.rs:re:x --window 40
  B.rs:re:y --window 8` exits 2 with `unrecognized arguments`, having read nothing. Pick the one
  window the widest target needs and let the narrow ones overshoot; splitting into two calls is the
  wrong reflex. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check can name a test for a feature the tree's own module doc says is
  *blocked*, and then the check is right and the doc is the specification.** The tell is a gap
  section that names its own repair in a sentence — a doc that precise has already done the design,
  so the check asks for the implementation, not a filing correction. Read the module doc's gap
  section before `Cargo.toml`: it answers "impossible", "unwritten" and "decided but unlanded" in
  one call, and only the third looks like a misfiling from outside. [until: reviewed 2026-09-06]
- **A `python - <<'PY'` regex over a whole source file is AGENTS.md rule 1's breach with a different
  spelling, and it fails the same way a `sed -i` does: silently, far from where you were looking.**
  A non-greedy pattern has no idea what a Rust item is and can rewrite a function signature hundreds
  of lines from the call sites it was aimed at. Use `Edit`'s `replace_all` when the string is
  literal and unique and `tools/splice.py --patch` when it is not; a script that must exist uses
  exact literals with an asserted count, never a regex with `.` in it. [until: reviewed 2026-09-06]
- **A chain-switch failure in `.loop/log.md` numbers its check in the *folded* goal, not in the goal
  file you will open — and the reason may already be fixed.** `loop.py`'s `install_next` validates
  the live copy after `goal-switch.py` has folded the previous goal's floor checks in above the
  entry's own, so the index is offset by the floor count `goal-switch.py` prints on its first line.
  Run `python tools/chain.py --check`, which validates every queued entry, before spending anything
  on what the message says. [until: reviewed 2026-09-06]
- **A test you are about to write may already have its name fixed by `loop-goal.toml`, and the
  handoff item will not carry it.** A gate written to the item alone lands under a fourth name and
  leaves the stage red with everything implemented; the fixed names are also a design, since three
  names say what is asserted separately. Before writing a new test for a goal item, `grep -n -A8
  'stage = "<the stage>"' docs/agent/loop-goal.toml` and take the names from the `tests = [` block.
  [until: reviewed 2026-09-06]
- **A goal's § *Standing decisions* can be right about the ruling and wrong about the reason it
  gives, and a frozen test name can carry the wrong reason with it.** A module doc outlives the goal
  file that seeded it, so a stated reason the code contradicts becomes a sentence that is simply
  false. Write the doc from what the code does (`capability.rs`'s `pin_host` is asked of a hostname,
  not `denied_by_default`'s address), keep the frozen test name, and say in the handoff that the
  wording was wrong rather than the decision. [until: reviewed 2026-09-06]
- **A module doc's known gap can give its reason as "`docs/agent/loop-goal.md` § *Standing
  decisions* keeps it out of scope", and that reason expires at the next goal switch.**
  `tools/goal-switch.py` carries neither item lists nor the sentences that referred to them, so the
  gap reads as a standing decision while being a stale one. One `grep -n` of `loop-goal.md` for the
  name settles it before you treat a gap note as a decision: a reason naming a goal file is true for
  one goal, unlike one naming a rule. [until: reviewed 2026-09-06]
- **A mid-goal edit to `docs/agent/goals/<n>-<slug>.md` is invisible to the loop until the same edit
  lands in `docs/agent/loop-goal.md`.** That file is `goal-switch.py`'s verbatim copy of the goal's
  prose and the one `orient.py` pipes into a session, and nothing re-syncs the two between switches —
  so a settled standing decision can sit in the source while every pack still prints the question.
  Edit both, and `diff` them before committing. [until: gone docs/agent/loop-goal.md]
- **A `loop-goal.toml` `cargo-named` check's *other* test names are the specification for the design
  question the rule left open.** A sibling name such as
  `a_cli_runs_record_is_still_level_and_msg_alone` bounds what the first test may assume — stamping
  `ts` unconditionally would have broken
  `application_code_and_the_engine_floor_produce_schema_identical_records`. Read a check's whole
  `tests` list as one sentence before writing the first of them; a sibling name is cheaper than
  re-deriving the bound from the rule. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check filed in a crate that cannot reach the surface has a *fourth* repair:
  leave the check where it is and make the surface a parameter the binary fills.** It applies when
  the named crate owns the *decision* and a crate it may not name owns only the *data* the decision
  reads — adding the dependency would put the test one layer above its rule.
  `crates/nvs-server/src/schedule.rs`'s `Fires` is the shape: a trait the crate declares and
  `nvs-cli`, which names both sides, implements. [until: reviewed 2026-09-06]
- **A `## Next group` anchor is a line number, and the session that wrote it usually went on editing
  that same file.** The windows `orient.py` inlines for a stale anchor are correct-looking code from
  the wrong place, with nothing in the pack to say so. Trust the symbol name in the item, not the
  code beside it — `python tools/peek.py --locate <sym> ...` re-derives every anchor in one call —
  and resolve the next group's anchors after the last commit, not before it.
  [until: reviewed 2026-09-06]
- **A `peek.py` `re:` target over `docs/agent/loop-goal.toml` sweeps every stage of a file thousands
  of lines long, and the word you searched for is prose in most of them.** A bare word matches
  `[context]`, unrelated stage comments and a dozen `tests = [...]` lists, costing many times the
  two blocks wanted. Anchor on the syntax around what you want — `re:stage = "3` for a stage's own
  blocks, `re:pub fn <name>` for a definition — or read the window once you know the line.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check can be filed in the crate whose *name* matches the subcommand, and the
  subcommand lives in `nvs-cli`.** `nvs serve` is a subcommand of the `nvs` binary
  (`crates/nvs-cli/src/serve.rs`); `nvs-server` is the accept-loop library it hands a bound socket
  to, resolves no configuration and boots nothing, and the manifest and `[lints]` triages both pass
  because `nvs-cli` depends on it. Triage a check naming a `nvs <verb>` behaviour by `grep -n
  'Command::' crates/nvs-cli/src/main.rs`, which lists every subcommand beside the module that
  answers it. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` acceptance list is not all under `crates/` and `tests/` —
  `benches/abi-probe/` hosts a whole leg of it.** `perf_guards.rs` and `invariants.rs` under
  `benches/abi-probe/tests/` are the ABI and cost-class guards, a workspace member the goal names
  like any other, and the only tests outside those two roots. Grep `fn <name>` from the repository
  root, or you will file a slice to write a test that already exists.
  [until: gone benches/abi-probe/tests/perf_guards.rs]
- **A goal's own new fixture blocks the *entire* acceptance sweep until it exists, and then masks
  every cargo check until it passes.** `tools/loop.py`'s `begin()` refuses if a `files` entry is not
  on disk, and `_check` runs all program checks before any cargo check, so a fixture written against
  a later stage's surface fails on `exit 1` and the `cargo-named` stages never run; the tell is a
  ledger line reading `over 1 check(s)`. Write the fixture anyway, and check whether the surface its
  `want` names exists before reading the earliest failure as this session's work.
  [until: reviewed 2026-09-06]
- **A new goal's TOML is validated by `python tools/chain.py --check` and by nothing else, so run it
  before you believe the file.** Two mistakes look identical to a reading eye: a `[[check]]` `kind`
  the driver does not know (the set is `cargo-named`, `command`, `contains`, `exact`, `min-bytes`,
  `nvs-suite`, `ordered` — there is no `nvst`), and a `kind = "command"` naming a tool a *later*
  goal builds. `--check` catches the first and cannot catch the second, so prefer a `cargo-named`
  check against a test that exists today over a `command` against a tool that does not.
  [until: reviewed 2026-09-06]
- **Writing a tree file through Python's text mode rewrites every line ending on this checkout.**
  `Path.read_text` maps `\r\n` to `\n` and `write_text` writes back what it was handed, so a
  one-word change reports every touched file modified with an empty `git diff`, buries the real
  change in `git status`, and makes a "restore the original bytes" rollback a lie. Open with
  `newline=""` in both directions. [until: reviewed 2026-09-06]
- **Never scan re-wrapped prose line by line for anything holding a space.** `orient.py` and
  `brief.py` fill a plan field at width 100, so a change anywhere earlier in the field reflows every
  line after it, and a `rule:` token, a `file.rs:NN` anchor, a `§ N` or a `Core\Foo::bar` can break
  at its space. Scan the whole text and let the pattern's own `\s+` decide; line-oriented reading is
  only safe for things a line *is* — a heading, a table row, an indent. [until: reviewed 2026-09-06]
- **A `loop-goal.toml` `command` check's `want` is an *ordered* list, so two names in it
  assert an order as much as a presence.** `nvs schema dump`'s wanted `nvs_jobs` before
  `nvs_dead_jobs`, which no server answers: every catalog query in `nvs_db::catalog` orders
  by table name, so the dump is alphabetical and the check read as unwritten work forever.
  Before writing output to satisfy a `want`, ask whether the order it names is one the data
  can have — and when it is not, fix the `want` and say in a comment which ordering is real.
  [until: reviewed 2026-09-06]
- **`splice.py` refuses a block that occurs more than once, which is exactly the shape of adding a
  payload to an AST variant: fifteen identical `kind: ExprKind::Error,` lines at five different
  indentations.** Giving each one unique surrounding context is a page of anchors that must be
  byte-perfect, and a single wrong space fails the whole patch. Use `Edit` with `replace_all` once
  per indentation level, with a leading newline inside `old_string` so a 12-space block cannot match
  inside a 16-space line, and keep `splice.py` for the blocks that really are one of a kind.
  [until: reviewed 2026-09-07]
- **A seed corpus cannot live under `fuzz/corpus/`: `.gitignore` ignores that whole directory.** It is
  where the nightly `fuzz-smoke` cache accumulates the corpus between runs, and a file inside an ignored
  *directory* cannot be brought back by a negation pattern. Put seeds in `fuzz/seeds/<target>/` and let the
  CI step before the run copy them in — it does that for any target that has one.
  [until: gone .gitignore:/fuzz/corpus]
- **A `cargo-named` acceptance check can report a test red that passes on every re-run, and the
  driver's own overlapped release prebuild is why.** `nvs-server`'s `serve` tests bind real loopback
  sockets and hold a connection to a memory ceiling, so they are timing-sensitive in a way the rest
  of the workspace is not, and a check run while `cargo build --release` saturates the machine can
  miss a frame that arrives every other time. Before treating a red *named* test as the regression
  that outranks your item, run that one test and then its whole crate suite — if both pass, it was
  load and the ledger's next line will not repeat it. [until: reviewed 2026-09-07]
- **`session.py --wrap` can refuse over a link no session touched, because a rule fragment's
  relative link is resolved from two different places.** A fragment's links are copied verbatim
  into the chapter one directory above it, so `../../../README.md` resolved from
  `docs/rules/packaging/the-banner-states-the-build.md` and escaped the repository from
  `docs/rules/packaging.md` — `git show HEAD:<file>` showed the line unchanged, so "this session's"
  was about which resolver ran rather than about the diff. Write a fragment's links to resolve from
  the *chapter*, `python tools/rules.py --render`, and confirm with `python tools/check-links.py`.
  [until: reviewed 2026-09-07]
- **A `splice.py` block that inserts text before an anchor repeats that anchor on both sides, so
  fixing a stale `--- old` copy alone silently rewrites that line.** The tool matches the OLD side
  and writes the NEW one verbatim, so an "anchor" is an edit too, and a landed doc comment changes
  under an insertion that never meant to touch it. Correct both copies, and read the anchor lines
  out of `git diff` after a splice whose blocks were insertions.
  [until: reviewed 2026-09-07]
- **An ADR cross-reference can name a section that says nothing about the thing citing it.** ADR 0099
  § 3's hover row sends a `Core` signature row's rendering to "as ADR 0088 § 5 writes it", and that
  section is `Core\Out::capture` answering the sink's carrier — 0088 decides no rendering anywhere, so
  the spelling was undecided rather than specified. Read the section a pointer names before treating it
  as the specification, and when it does not hold the answer, decide it in the module doc and say so
  rather than hunting for a record that does not exist.
  [until: gone docs/decisions/0099.md:signature row rendered as]
- **A reserved word has no accessor for its own spelling, and `Keyword`'s doc comment names one that
  does not exist.** `crates/nvs-syntax/src/token.rs`'s enum says "every variant's `name()` gives that
  one spelling" and there is no `name()`; `Keyword::from_lowercase` is the only table, and it maps the
  spelling *to* the variant rather than back. A list of spellings written anywhere else is therefore a
  copy — write it as `&[&str]` and guard it with a test that every entry round-trips through
  `from_lowercase`, which is what `nvs_lsp::completion`'s keyword lists do.
  [until: exists crates/nvs-syntax/src/token.rs:pub fn name]
- **A `cargo-named` check counts an `ignored` test as one that ran, so a guard copied from
  `perf_guards.rs` reports green while measuring nothing.** `tools/loop.py:2350` asks only whether
  the test's name is in the output and `test <name> ... ignored` names it, while `perf_guards.rs`
  gates each guard behind `#[cfg_attr(debug_assertions, ignore)]` and no check's `args` carries
  `--release`. Put the ceiling behind `#[cfg(debug_assertions)]` instead, as
  `crates/nvs-lsp/tests/latency.rs` does, so the guard runs in the build the gate builds.
  [until: gone tools/loop.py:name not in both]
- **`tools/verify.py` runs `npm run --silent test:headless` the moment
  `editors/vscode/package.json` exists**, so the manifest, the runner and a green suite land in one
  group or every session's gate goes red. `scripts/headless.mjs` prints a line only for a suite with
  compiled cases under `out/test/<name>/` and exits 1 when there are none, so naming a stage cannot
  fake it green and the loop's `want` list stays red until each suite is real. Add one by creating
  `test/<name>/*.test.ts` — nothing registers it elsewhere.
  [until: gone editors/vscode/scripts/headless.mjs:ORDER]
- **An anchor copied out of `peek.py`'s output carries its line-number gutter, so its indentation is
  wrong.** The gutter is the number right-aligned in a fixed width and then two spaces, which reads
  as part of the indent, and `splice.py` refused a block that wanted ten leading spaces where the file
  has eight — reporting a divergence eighteen lines above the anchor, because the first candidate it
  found was a shallower line with the same opening brace. Take the depth from a line whose nesting you
  already know, or read the exact bytes with `sed -n 'A,Bp' <file> | cat -A`, before writing the OLD
  block. [until: reviewed 2026-09-08]
- A TextMate `begin`/`end` rule whose only pattern is an include the registry cannot resolve is
  dropped whole, and the drop takes the rule that included it with it. `--ORACLE--` lost its header
  colour and its whole body that way, because it includes `source.php`, which no registry outside a
  real editor holds. Stub the missing scope in `editors/vscode/test/grammar/tokenize.ts` as
  `text.html.basic` already is, rather than hunting a regex that looks wrong.
  [until: gone editors/vscode/test/grammar/tokenize.ts:PHP_STUB]
- **`session.py --wrap` cannot commit a rename: a `## commit:` naming the path that went away is
  refused with "no such path", and naming only the new one leaves the deletion uncommitted.** Every
  named path is checked against the disk before a byte is written, and the commit it makes is
  pathspec-limited, so no spelling of the section carries a `git mv` — and `git add -- <old>` would exit
  128 anyway once the index has the rename. Commit a renaming slice yourself, `git add -A -- <dir>` then
  `git commit -F`, after `verify.py` is green, and leave the wrap the docs. [until: reviewed 2026-09-08]
- **A `rule:` citation wrapped across two comment lines fails `rules.py --check` from a file you never
  touched.** `docs/agent/goals/` had split
  `rule:ide/the-extension-refuses-a-binary-it-does-not-understand` over two `#` lines, so the checker
  read the first half as a rule id that does not exist and `rules.py --render` has exited 1 ever since —
  which lands on whoever next edits `docs/rules/`, because `session.py --wrap` runs both for them.
  Reword the sentence so the token sits on one line rather than breaking one across a wrap.
  [until: reviewed 2026-09-08]
- **`peek.py`'s `:@name` selector takes the symbol as it is *written*, so a `Type::method` spelling
  lands somewhere else or nowhere.** `object.rs:@ClassTable::define` printed `set_field_tags`, whose
  doc comment merely *mentions* `ClassTable::define`, and `@NvsObj::set_field` answered "no
  definition or mention" for a method three screens down — an inherent method is written
  `pub fn set_field`, and the `impl` block's name is not part of the token. Ask for the bare name
  (`@set_field`), or `--locate set_field`, and read the `impl` off the anchor.
  [until: reviewed 2026-09-08]
- **A `splice.py` OLD block typed out of a `peek.py` window can carry the wrong
  indentation, and the miss reports as a stale anchor.** `peek.py` prints a line-number
  gutter ahead of the source, so leading spaces counted off the screen include however many
  the gutter took — a block eight spaces deep in the file arrives twelve deep in the patch
  and matches nothing that is there. Anchor on the shortest unique run of lines rather than
  on a whole comment, or read the exact region back before typing one; the refusal names the
  first character that diverged, which is the tell that it is the indentation and not the
  words. [until: reviewed 2026-09-08]
- **A `loop-goal.toml` fixture check can name `want` lines only a *leg argument* can produce, and the
  missing half is the check's `args`, not the program.** A check whose comment says it "runs under
  `nvs run --request`" while passing no `--request` leaves the fixture printing a refusal forever,
  which reads exactly like unwritten work. `python tools/loop.py --list` prints each fixture's whole
  argv: read that before believing a `want` cannot be produced, and fix
  `docs/agent/goals/<goal>.toml` too. [until: reviewed 2026-09-08]
- **A `toml file=nvs.toml` fence attaches to the next *runnable* example, not to the next fence, so an
  `nvs skip` between them hands the config to a later program.** `tools/reference.py:618` skips a `skip`
  fence without clearing `pending`, so a configuration block written to illustrate an unrunnable example
  lands in the working directory of whatever runnable example comes next — on a reference page that ends
  with the house `LogicError` refusal, that is the one it reaches. Put the configuration in a comment
  inside the `skip` fence, or place the `file=` block directly before the runnable program it belongs to.
  [until: reviewed 2026-09-08]
- **The driver's end-of-session sweep commits every dirty path in the tree, not the ones its session
  touched.** A session that exits without wrapping has its leftovers swept into a `wip(loop)` commit,
  so a goal file the user is authoring alongside the run lands in a commit named for a session
  number, and `orient.py` then opens the next pack presenting it as the predecessor's unfinished
  slice. Read `git reflog` before continuing such a commit: a reset and a re-land by the user leaves
  the same paths dirty as work that is genuinely still owed, and the reflog is the only thing that
  tells the two apart. [until: reviewed 2026-09-08]
- **`peek.py --locate` takes symbol names only and answers one line per file, so a name two `impl`
  blocks in one file share resolves silently to the first one.** `--locate set_peer
  crates/nvs-runtime/src/ctx/inbound.rs` reports `Inbound::set_peer` at `:600` and never
  `InboundSpec::set_peer` at `:1227`, and it reads the path as a second *symbol* — printing
  `NOT FOUND` for it rather than scoping the search — while a `file.rs:@Type::method` target answers
  "no definition or mention" for a method that is right there. For a name a file carries twice, one
  `grep -n 'fn <name>' <file>` names both and is the only form that does.
  [until: reviewed 2026-09-08]
- **A `cargo-named` check matches its name as a *substring* of the run's output, so a longer test
  name satisfies a shorter one and a differently-worded one satisfies nothing.**
  `two_body_spellings_in_one_spec_are_refused` is green under `..._refused_naming_both`, so
  `did not run` can mean *named differently* rather than *not written*. `tools/loop.py:2358` is the
  matcher: grep the crate for what the name *describes*, then rename that test rather than landing a
  twin beside it. [until: reviewed 2026-09-08]
- **A goal stage's `[[check]]` block can be green before any of that stage's work exists, because it
  names gates that already pass.** Goal `test-request`'s stage 4 checks five `nvs-stdlib` roster gates that hold on
  every tree, so the driver's acceptance can report the goal green while that stage's deliverable — a
  rule fragment and a spec row — is unwritten. Where a stage's deliverable is documentation, read the
  stage's own prose in `loop-goal.md` before believing its checks: they are a floor, not the
  specification. [until: reviewed 2026-09-08]
- **A handoff item's anchor can name the wrong file, and `orient.py` inlines that window as if it
  were evidence.** This goal's stage-3 item pointed at `crates/nvs-ir/src/lower/call.rs:713` for the
  `WRITTEN_CLASS_MEMBERS` call site, so the pack printed `emit_const_shape` — a shape *literal*'s
  emitter — and the real lowering is `lower/expr.rs:3212` and `:3441` plus `lower/closure.rs:621`.
  One `grep -n` for the symbol the item names, before reading the window it inlined, is the whole
  fix. [until: reviewed 2026-09-06]
- **The live acceptance list and the goal's own `docs/agent/goals/<goal>.toml` drift apart, and a
  plain `diff` will not show you how far.** The live `docs/agent/loop-goal.toml` is written with CRLF
  while the source stays LF, so `diff` reports every line changed and reads as two unrelated files —
  and because `goal-switch.py` carries the *live* file's checks forward, a stale source is invisible
  until something reinstalls it. Use `diff --strip-trailing-cr` before believing either file, and put
  an amendment to a check into both in the same commit. [until: reviewed 2026-09-08]
- **A `loop-goal.toml` check naming a refusal can be asking for the refusal itself, not for one more
  name under it.** Stage 2's `a_program_declaring_its_own_interface_named_parses_is_refused` reads as a
  new case of a settled rule, but `nvs check` accepted `interface Stringable {}` and `class Throwable {}`
  alike — neither compiler-owned roster was refused anywhere, because every resolver short-circuits on
  those names before the symbol table and nothing ever looked at the declaration. Probe the precedent
  with `target/debug/nvs.exe check` on a two-line file before budgeting the slice as a one-line
  addition. [until: reviewed 2026-09-08]
- **A goal's prose can name a diagnostic code that was free when it was written and is taken now.**
  Goal `unix-sockets`'s stage 3 asked for `E0627` "beside `E0626`", but `E0627` had since been issued to
  `E_UNSPELLED_EXPORTER` and the band's next free number was `E0635` — which the orientation pack
  prints for every band, under *the next free number*. Take the number from that block and never from
  a goal, a rule or a check's comment, and amend the prose that named the stale one in the same
  session, or the next session derives it again. [until: reviewed 2026-09-09]
- **A new rule's `because` may name only records whose own `changes:` block names that rule, so
  citing an older record as background fails `python tools/records.py --check`.** A rule created by
  0162 and listing `["0162", "0051"]` because 0051 placed the class reads as good provenance and is
  refused with `0051.md:3  <rule>'s because names 0051, but its changes: does not name the rule` —
  the two halves are one relation and a frozen record cannot grow a new entry. Cite background
  records in the fragment's prose, and keep `because` to the records that created or amended the
  rule. [until: reviewed 2026-09-09]
- **A `cargo-named` check's *test name* can carry a member spelling a later ADR renamed, and
  writing that member is the wrong repair.** Goal `net-os-signal` asked for
  `..._memory_usage_..._all_answer` after ADR 0148 § 12 had moved held bytes to `Core\Budget`,
  leaving `Core\Os::residentBytes`. A check name is drafted before its stage runs, so when the
  spec row, the migration table and the outstanding file all disagree with it, amend the name.
  [until: reviewed 2026-09-09]
- **A `loop-goal.toml` check can name a test the tree already declares under a *different* name.**
  Stage 5 asked for `every_migration_member_is_registered`, which no crate declares, while
  `spec_registry_coverage.rs` has carried `every_migration_member_row_names_a_registered_member` —
  the same walk over the same fixture — throughout, so the check read "did not run" against a green
  tree. `grep` a check's test name against `crates/` first: a near miss on a real name is a
  shorthand to fix in the goal file, not a test to write. [until: reviewed 2026-09-09]
- **A plain `php tools/dump-php-builtins.php` writes a *smaller* inventory than the committed one, and
  growing it drops `check-migration.py` under its `--min 100` floor.** The Windows build ships
  `fileinfo` and `zip` as DLLs its `php.ini` does not enable, and every name a regeneration adds is
  `open` until `docs/spec/02-php-migration.md` rows it. Pass `-d extension=` per DLL, diff the
  `# extensions:` header, and row the new names in the same slice. [until: reviewed 2026-09-09]
- **A `loop-goal.toml` `exact` check over an example now pins the *line numbers* of that example's
  own producers, so a comment added above one turns the floor red.** A record's envelope carries
  `source` since goal `record-origin`'s stage 2 (`rule:errors/a-record-names-where-it-was-produced`), and
  `examples/logging.nvs`'s two `Core\Log::write` lines are in the check's `want` verbatim. Edit such
  an example only below its last producer, or run `target/debug/nvs.exe run <example>` afterwards and
  move the `want` with it — the check reports stdout line for line and says nothing about why a
  number moved. [until: reviewed 2026-09-09]
- **A `Core` member's `signature` in `nvs meta --json` carries its type parameters, so the leading
  token of a rendered line is not the member's name.** `Core\Json::decodeAs<T>(string $json): T` is
  the spec's own spelling, and a symbol read as "everything before the first `(`" keeps the `<T>` and
  then resolves to nothing — three of `nvs agent`'s cases went red on the ten `…As` members at once.
  Split on `['(', '<', ' ']` when reading a name back out of a rendered signature, and check
  `Core\Arr::shapeAs` and `Core\Request::queryAs` before believing a walk over the registry is
  complete. [until: reviewed 2026-09-09]
- A `<!-- primer -->` marker goes in `docs/reference/`, whatever else names `docs/spec/`.
  `tools/reference.py`'s `SOURCES` is `docs/reference`, and the one spec file it reads is
  `docs/spec/02-php-migration.md` for Part D's crosswalk table — so a marker written into
  `docs/spec/` is read by nothing and lifts nothing into `nvs agent primer`. Put it on the line
  above a section heading under `docs/reference/lang/` or `docs/reference/tools/`, and expect the
  same slice to owe `python tools/reference.py --no-examples` and `docs/novis.md` in its commit,
  because every chapter edit makes the generated file stale.
  [until: gone docs/reference/README.md]
- **Changing a capability denial's sentence is a corpus *and* reference edit, not a one-line one.**
  `grep -rln "which is not granted" tests/ docs/reference/` is the blast radius: the conformance
  cases, several of which compare `$e->message ==` inside program logic rather than echoing it
  (`tests/conformance/core/io-copy-names-two-grants-and-neither-one-alone-is-enough.nvst:23`) so an
  expectation bump never finds them, and the executed `output` fences in
  `docs/reference/lang/70-errors.md`, `docs/reference/lang/80-concurrency.md` and
  `docs/reference/tools/20-config.md`, which `verify.py`'s `reference` leg runs and
  which owe `python tools/reference.py --no-examples` for `docs/novis.md` in the same commit. Budget
  the pass as the slice itself, and price any design that leaves `$e->message` alone against it
  before choosing. [until: reviewed 2026-09-09]
- **A decision record's `changes:` block needs `modifies:` spelled out even when the record modifies
  nothing.** `conventions.md` § *A decision record* shows both keys under one example that uses both, so
  a record creating two rules and editing none reads complete without the second — and then
  `tools/records.py --check` answers `changes: has no modifies: list`. Write `modifies: []` in the same
  keystroke as `creates:`, and run `records.py --check` beside `rules.py --render` rather than after it.
  [until: reviewed 2026-09-09]
- **A goal's own `[[check]]` blocks can sit thousands of lines below the head of `loop-goal.toml`,
  under every carried floor check, so the file's top makes a live goal look like it has no
  acceptance of its own.** Goal `gap-owners`'s are at `loop-goal.toml:6730`, and they name flags the
  tool being written has to have — `--untagged-is-an-error`, `--reasons` — which is a specification,
  not a suggestion. `python tools/loop.py --list` prints every check with its stage in one call; run
  it before designing anything the goal's prose only describes.
  [until: reviewed 2026-09-10]
- **A `# Known gaps` item can name work that has since landed, and the attribution pass is where
  that surfaces.** Three of the five items in `crates/nvs-types/src/lib.rs` described checks the
  crate already makes — `inout`'s both-sides-agree obligation, named and spread argument
  positioning, and four of `rule:types/narrowing`'s five spellings — so tagging them would have put
  an owner on finished work and made the roster lie about what is owed. Read the code an item names
  before choosing its owner, since `peek.py --locate` over the function it describes settles it in
  one call, and delete a closed item rather than tagging it. [until: reviewed 2026-09-10]
- **An `— owner:` marker counts only as an item's *last* line, and settled prose trailing under
  `# Known gaps` swallows it.** `owners.py` folds every line after a bullet into that bullet until
  the next one starts, so the two closing statements under `crates/nvs-types/src/error_lib.rs`'s
  block left the marker mid-item and the item stayed on `--untagged` with nothing said about why.
  Move that trailing prose above the heading — under this goal's § *Standing decisions* it is a
  decision rather than a gap — instead of hunting the marker's indentation.
  [until: gone tools/owners.py:the owner tag is not the item's last line]
- **A `# Known gaps` block that enumerates nothing is *one* item to `owners.py`, whatever its
  paragraph count.** `--untagged` then names only the first paragraph's opening sentence, which reads
  as though the paragraphs under it were already tagged — they are not, they are the same item, and
  one `— owner:` line is all the block can carry. Enumerate the block into `*` bullets when its
  paragraphs need different owners, and put each tag on that item's own last line; `items_of` in
  `tools/owners.py` is the parser that decides it. [until: reviewed 2026-09-10]
- **`owners.py` counts a `# Known gaps` *heading*, and never a `**Known gaps:**` bold run.**
  `crates/nvs-hir/src/requires.rs:69` holds five real gap bullets under a bold label, and the roster
  has never seen one of them, so `--untagged-is-an-error` can go green over a module that names
  nobody for anything it owes. Before concluding a crate is clean, grep it for `Known gaps` rather
  than trusting `--untagged`, and treat a bold run as a hole in the tool rather than a doc to
  reshape. [until: reviewed 2026-09-10]
- **A heredoc, a `>` redirect or a `sed -i` that writes a file dies on the first apostrophe or
  backtick, and a doc comment is made of both.** The shell expands `` `rule:...` `` and eats the
  quoting before the tool it feeds ever runs, so it lands as mangled content on disk rather than as
  an error you can see. Write and Edit carry file content into the tree and `python
  tools/splice.py --patch <file>` does three or more edits in one call; `python
  tools/loop-stats.py` counts the sessions that reached for the shell instead.
  [until: reviewed 2026-09-10]
- **A goal slug can name a *retired* goal, and nothing in the goals directory says which those are.**
  `chain.py --check` accepts the tag because a retired goal keeps its `N-<slug>.md` and loses only its
  `.toml` and `.handoff.md`, so `owners.py` takes it and then reports it as an owner that went green
  without closing its gap — a finding a later session has to unpick. `ls docs/agent/goals/*.toml` is
  the live set; everything else is walked, which also means its milestone counts as *carried* and its
  leftover gaps are `unowned` rather than that milestone's. [until: reviewed 2026-09-10]
- **Moving an item out of a `# Known gaps` block renumbers every item after it, and other files cite
  those numbers by position.** `crates/nvs-stdlib/src/db/mod.rs`'s eight became four and twelve
  citations went stale at once, in six siblings, `queue.rs`, the register and goal `gap-zero`'s two
  files, because a gap has no name — only a position. Grep `gap [0-9]`, `crate::<module>` and the
  module's own path across `crates/` and `docs/` *before* the edit, and put the doc and the whole
  sweep in one `splice.py --patch`. [until: reviewed 2026-09-10]
- **A `# Known gaps` item's claims about the rest of the tree go stale, and an attribution pass
  reads them as current.** `crates/nvs-stdlib/src/uuid.rs`'s gap 1 waited on a
  `nvs_runtime::Tag::Bytes` variant that is live (`crates/nvs-runtime/src/value.rs:307`), and
  `docs/agent/playbook.md` spelled out `json.rs`'s gap *number*, which a moved item silently
  re-points. Grep the blocker a gap names, and the module's own path, before tagging or renumbering
  one. [until: reviewed 2026-09-10]

- **An `[until: gone <path>:<needle>]` needle is matched literally, so one that drops the source's
  backticks fires the moment it is written.** `session.py --wrap` retired a `carried-gaps.md` bullet
  in the same call that added it, because the needle read `bytes round trip` where
  `crates/nvs-stdlib/src/uuid.rs` writes `` `bytes` round trip ``. Copy the needle out of the file it
  watches, or point it at a plain-text state such as `owner: unowned` instead.
  [until: reviewed 2026-09-10]
- **`owners.py` reads a gap item as running to the end of the module doc, so a paragraph *after* the
  numbered list makes the owner tag "not the item's last line".** `crates/nvs-stdlib/src/process.rs`
  kept its `**What it spends:**` paragraph inside `# Known gaps`, so the tag appended to item 1 left
  the gate red with a message that reads like a formatting typo in the tag itself. Move the trailing
  paragraph above the `# Known gaps` heading — where the rest of the crate already has it — rather
  than moving the tag down past prose it does not belong to.
  [until: gone tools/owners.py:the owner tag is not the item's last line]
- **A `splice.py` block that deletes a whole line leaves the line behind as a blank one.** The empty
  NEW half replaces only what OLD matched, and the newline after the last matched character is not
  part of that match, so striking a row from `carried-gaps.md` § *Owned* left an empty line mid-table,
  which splits one rendered table into two. Put the *following* line inside both halves of the block —
  anchor on the row plus the line after it, and write that following line back alone.
  [until: reviewed 2026-09-10]
- **A `# Known gaps` item's own rule citation can be to a rule that does not say what the item
  claims, and that is what decides gap-versus-decision.** `crates/nvs-stdlib/src/cli.rs` gap 1 rested
  "empty is the answer that rule wants" on `rule:security/capability-check-at-the-door`, which is only
  about *where* a capability check lives, and `crates/nvs-stdlib/src/storage.rs` gap 1 rests on
  `rule:core-api/shape-rules` R7 and R20, which that chapter's table gives as *members are full words*
  and *no mutable/immutable twin types*. Open the cited rule before reading an item as settled — the
  citation is the claim being made, not the evidence for it. [until: reviewed 2026-09-10]
- **A milestone tag is a claim about that milestone's plan text, and the plan can refuse the gap
  rather than carry it.** `crates/nvs-stdlib/src/random.rs`'s "not reseeded on `fork`" pointed at
  M7's unbuilt service registration, but `docs/plan/m7.md:62` writes a `Type=notify` unit — the
  systemd type that does not daemonize — and `rule:core-classes/process-is-argv-only` makes every
  child an exec, so M7 is where that hazard is ruled out rather than scheduled. Read the plan
  paragraph a gap points at before tagging it with that milestone, and when the paragraph refuses
  the item's premise the item is a decision that moves above `# Known gaps`.
  [until: reviewed 2026-09-10]
- **`owners.py --reasons` is satisfied by a reason that merely *mentions* the file.**
  `unowned_paths` collects every `crates/**.rs` path written anywhere under `carried-gaps.md`
  § *Unowned*, so a file named inside another entry's prose — `crates/nvs-types/src/locals.rs` is,
  by the entry that owns `lib.rs`'s two flow checks — already passes the gate for a gap whose reason
  nobody wrote. Read the section for an entry that actually ends `<path> gap N.` before trusting a
  green `--reasons`. [until: reviewed 2026-09-10]
- **A `cargo-named` check's *test name* can prescribe a design the cited rules already decided
  against, and then the name is what moves.** `..._are_declared_shape_parameters_...` asked
  `Core\Queue::push` for two whole shape parameters, which `rule:core-api/options-bag`'s
  one-optional-tail and `rule:concurrency/queue-four-members` each forbid alone. A name specifies
  only where the rules left the question open, so rename the check — in `loop-goal.toml` and the
  goal's own copy — rather than bending the surface to it. [until: reviewed 2026-09-10]
- **`rules.py --check` reads `docs/agent/handoff.md`, so a `rule:` token invented in a *handoff* turns
  a floor check red for the next session rather than for the one that wrote it.** Session 0006's
  `## Next group` cited `rule:http-server/containment-does-not-end-at-the-helper` with its tail cut
  off, and the driver's earliest-stage failure became "the rulebook validates", with findings
  pointing at the handoff rather than at any rule. Paste a `rule:` token from `brief.py --where
  <topic>` rather than shortening one to the words you remember — **including in the bullet you write
  about the mistake**, since this file is scanned too and quoting the broken token here keeps the
  check red. [until: reviewed 2026-09-10]
- **`peek.py --locate <directive>` cannot find a directive's reader, because a directive is a string
  key and a reader is a symbol under some other name.** `[limits] max_output` read as "unread by
  anything in the tree" for a whole goal on that evidence, while `Ctx::output_limit` had been reading
  it through `configured_bytes("max_output")` since it landed. Locate a *symbol*; find a directive
  with `peek.py "crates/**/*.rs:re:<key>"`, which reads the string literals too.
  [until: reviewed 2026-09-10]
- **A goal can promise a converge of one `Safe` step and mandate an index in the same item.**
  `nvs_db::ddl::base_grade` grades every `AddKey` `Locking` — a build over every row already
  there, with no concurrent one in v1 — so a column-plus-index change is two steps and the second
  needs `--including-risky`. Read `base_grade` before writing an expectation from a stage's prose,
  keep the construct the *rule* mandates, and assert the two halves apart rather than dropping the
  index to make the sentence true.
  [until: gone crates/nvs-db/src/ddl.rs:An index is built over every existing row]
- **An `[until: gone <path>:<needle>]` needle that spans a wrapped line retires itself the first
  time `session.py --wrap` runs.** The needle is a plain substring of the file, so a sentence broken
  across a Rust string continuation or a `///` wrap holds it nowhere, and the bullet is deleted with
  a message stating the file no longer holds a sentence it plainly still does. Grep the needle
  before writing the trailer, and shorten it to the longest run that sits on one line.
  [until: gone tools/playbook.py:A needle is a plain substring]
- **`docs/agent/loop-goal.toml` is CRLF and its `docs/agent/goals/<goal>.toml` copy is LF, so a
  plain `diff` of the pair reports every line as changed and buries the drift that matters.**
  Compare them with `diff --strip-trailing-cr`: run that way this pair differed by exactly the two
  `[context] modules` lines a session had added to the live copy alone, which the next
  `goal-switch.py` would have dropped. Write each file in its own ending, and read `git diff
  --numstat` afterwards to see that you did. [until: reviewed 2026-09-10]
- **A `loop-goal.toml` stage's *whole* `tests` list can be drafted against a file the goal later
  chose not to write.** Goal `sqlite-queue` drafted `..._on_sqlite` names for
  `crates/nvs-stdlib/tests/queue.rs`, and stage 2 put this backend's cases in a new
  `queue_sqlite.rs` under its own `a_sqlite_…` convention, so two checks read "did not run" over
  claims already pinned. When *no* name in a `cargo-named` list resolves, grep the claims rather
  than the names and see whether an earlier stage's list was already re-pointed — that is the tell
  that the drafted names are the stale half. [until: reviewed 2026-09-10]
- **A `loop-goal.toml` check name can carry a *count* of the roster the goal itself grows, and the
  count is stale the day the goal lands.** `runs_answers_true_for_three_drivers_and_false_for_sql_server_alone`
  was drafted while `nvs_stdlib::queue::runs` answered true for three drivers, and goal
  `sqlite-queue` makes it four, so the drafted name could only be satisfied by a test whose name
  lies about what it asserts. Rename the check in both toml copies to the claim with the count
  taken out — in a name of that shape the claim is the half after the `and`, and the count is
  decoration — rather than writing the test under the drafted name. [until: reviewed 2026-09-10]
- **A check name that greps to nothing can still be a *rename* the goal's own stage prose ordered.**
  `sqlite-queue` asked for `a_worker_reads_the_same_claim_columns_by_position_on_all_three_dialects`
  while its stage 2 says to extend the claim-column test "rather than writing a second one", and
  that extended test was on disk under the name it was first written with. When the grep misses,
  read the stage prose for the test the check is *about*: the check name is drafted and the stage's
  instruction is the specification. [until: reviewed 2026-09-10]
- **A `loop-goal.toml` program check runs its `file` through `leg.run`, which prepends `run`, so
  reproducing one by hand as `nvs <file>` answers `unrecognized subcommand` and reads as a misfiled
  check.** Every `exact`, `ordered`, `contains` and `min-bytes` check in the list omits `args` for
  exactly that reason (`tools/loop.py:1449`), so a missing `args = ["run"]` is never what is wrong
  with a red one. Reproduce a program check as `target/debug/nvs.exe run <file>` and judge stdout
  alone — `exact` compares `stdout_lines` and never reads stderr, so a fixture whose stderr is full
  of warnings can still be green. [until: reviewed 2026-09-11]
- **Adding a request that takes a cursor obliges a case at every construct the corpus has ever
  reached, not one case.** `every_request_answers_every_construct` holds each row whose
  `Request::takes_cursor()` is true to the whole vocabulary — 26 columns today — so `references`,
  `documentHighlight` and `typeHierarchy` are 26 cases each, while `codeLens`, asked of the whole
  document, owed one. Price the row with `nvs lsp-test tests/lsp/ --coverage` before landing the
  variant, and take the cursor placements from `tests/lsp/actions/`, which is already one case per
  construct. [until: gone crates/nvs-lsp/src/coverage.rs:which is the ratchet]
- **A handoff item can name a helper that does not do what the item says it does.** This session's
  item named `nvs_lsp::definition`'s `named_at` as "what already finds the name inside one"; it finds
  the innermost *node* at an offset and hands back that node's whole span, and
  `nvs_syntax::walk` modelled no name span at all — `crates/nvs-lsp/src/redactions.rs`'s module doc
  is where that was written down, two crates from the item. Read the named anchor's own doc comment
  before designing around it: an item is written by a session that had the file open and is
  paraphrasing from memory, so it is a pointer and not a specification. [until: reviewed 2026-09-11]
- **`peek.py --locate` takes symbol names only, and a path written after one is read as another
  symbol to look up.** `--locate at members crates/nvs-lsp/src/completion.rs` searched the whole
  tree for a symbol spelled like that path, and answered `at` with 250 definitions from every
  crate — the scoping the call looked like it had was never there. Scope with a `re:` target
  instead (`"crates/nvs-lsp/src/completion.rs:re:^fn "`), and put `--context N` ahead of every
  positional target, because a flag written between two of them is an argparse error rather than a
  flag applying to the rest. [until: reviewed 2026-09-11]
- **`nvs-lsp`'s one-construction-site guard fires on a module that merely *names* `SymbolIndex`, the
  `use` line included.** `crates/nvs-lsp/tests/index.rs`'s
  `the_crate_has_exactly_one_symbol_index_construction_site` closed that name to `index.rs`,
  `lib.rs` and `server.rs`, so the first feature module to read the index — completion — failed a
  test whose message is about *building* one. Widen it by asserting that a module outside those
  three names the type only behind a `&` and in an import, which is what the test's own comment
  already claims, rather than by adding the module to the list.
  [until: gone crates/nvs-lsp/tests/index.rs:names SymbolIndex other than behind]
- **A check naming a *build-time* join against the registry cannot be all in `build.rs`.** A build
  script is compiled and run before the crate it belongs to, so `nvs_stdlib::registry::class` is
  unreachable from `crates/nvs-stdlib/build.rs` and only the inputs that are *documents* can be
  joined there. Split it the way the layer splits — the documents in the build script, the registry
  half in the module the generated table lands in, and the refusal in a `-p <crate>` test, which
  under `tools/verify.py`'s stop-at-the-first-failure is the build that does not finish.
  [until: reviewed 2026-09-11]
- **Moving a gap's owner tag onto the item's last line trades one `owners.py` failure for the next
  one.** The repair two bullets up makes the tag *readable*, and what it then reads is `unowned`,
  which `--reasons` refuses until `carried-gaps.md` § *Unowned* carries a bullet naming that
  module's path — so a session that fixed the paragraph placement and stopped leaves the same floor
  check red for a different reason. Re-run `python tools/owners.py --check --reasons` after the
  repair, never the bare `--check`, and distrust a handoff that calls the floor green without
  quoting the tool. [until: reviewed 2026-09-11]
- **A `.nvst` case's `--RUN--` line is a closed enum in `crates/nvs-test/src/case.rs`, not a command
  line.** Naming a real flag there — `test --list --format=json` — fails the case before it runs, with
  "`--RUN--` is `run`, `test`, … not `…`", however correct the spelling is on the binary. Add the
  spelling as a `Subcommand` variant first: the enum, `args()`, the parser arm, that arm's error
  message and the section table in `crates/nvs-test/src/lib.rs`, which is five edits in two files.
  [until: gone crates/nvs-test/src/case.rs:pub enum Subcommand]
- **A `surfaces` suite assertion runs over the client's source as text, so a comment can fail it.**
  Those tests assert what a module does *not* do — `tasks.ts` spawns no `child_process`, the AST
  panel never passes `--strict` — by matching the file, and the module doc explaining why it does
  not do that names the very string the assertion refuses. Strip the comments before matching, the
  way `editors/vscode/test/surfaces/ast.test.ts` does, or keep the prose off that spelling.
  [until: gone editors/vscode/test/surfaces/tasks.test.ts:child_process]
- **A goal's whole check list can read green with two of its stages unbuilt, because a `command`
  check's `want` names the suite's *label* and not a case in it.** `[…, "surfaces:", "0 failing"]` is
  satisfied by whichever cases that directory already holds, so goal `editor-surfaces` would have
  closed with its Test Explorer never written. Read an all-green ledger against the goal prose's
  stages and the `stage =` strings in `loop-goal.toml` — a stage with no check is the hole — and name
  cases in `want` the way `cargo-named` names tests. [until: reviewed 2026-09-11]
- **A setting added to `editors/vscode/package.json` turns a Rust test red, not a TypeScript one.**
  `crates/nvs-lsp/tests/extension_reference.rs` reads the manifest against the settings table in
  `docs/reference/tools/40-editor.md` and asserts the two name the same keys with the same defaults,
  so a contribution that lands in the manifest and the headless suite alone fails the whole cargo
  gate on a change that touched no Rust. Add the table row in the same slice and then run `python
  tools/reference.py`, because that chapter is one of `docs/novis.md`'s sources and `--check` over it
  is an acceptance check of its own. [until: gone crates/nvs-lsp/tests/extension_reference.rs]
- **A `@symbol` target names the member alone, so `@Shared::sweep` and `@SafepointView::request`
  answer *no definition or mention* — which reads like the symbol has been deleted.** `peek.py`
  matches a Rust definition by its own name, and a method's name does not carry its type; the
  qualified spelling is the one a `Core` member and a `.nvst` case take. Ask for `@sweep`, or
  `--locate sweep request`, and read the `impl` the hit lands in. [until: reviewed 2026-09-11]
- **A `.nvst` case exercises `nvs run`, not `nvs serve`, however server-shaped the rule it cites is.**
  `nvs-test` spawns the `nvs` binary once per case (`crates/nvs-test/src/run.rs:344`), so a check whose
  mechanism is named by a `rule:http-server/...` ceiling is still answered by the CLI run path in
  `crates/nvs-cli/src/main.rs`, and a group scoped to `serve.rs` writes that mechanism where the case can
  never reach it. Read the `[[check]]`'s own `args` before choosing the file set: `args = ["test", ...]`
  over a `.nvst` tree means the surface under test is `nvs run`, whatever the stage header says the stage
  is about. [until: reviewed 2026-09-11]
- **`peek.py` refuses a target that comes after a flag, and says `unrecognized arguments` as
  though the target were malformed.** `TARGET` is a `nargs='*'` positional, so
  `peek.py a.rs:@sym --window 30 b.rs:120-160` leaves argparse a second run of positionals it
  has nowhere to put, and the usage block it prints reads as a bad locator rather than a bad
  order. Put every target first and every flag — `--window`, `--context`, `--in`, `--locate` —
  last, and reach for the per-target `:3` context suffix when only one target wants it.
  [until: reviewed 2026-09-11]
- **A `loop-goal.toml` check's drafted `tests` list can be *mixed*: some names are claims the tree
  landed under other names, the rest are unwritten work.** Map the whole list onto existing tests
  and a name ends up pointing at a test that does not assert it; write every name and two are
  duplicates. Check each name's claim against the candidate test's *body*, one entry at a time — a
  conjunction reads as covering two drafted claims while asserting one.
  [until: reviewed 2026-09-11]
- **An `[unread: … owner: …]` trailer counts only on the doc comment's *last* line, and goal
  `config-is-written`'s own example spans two.** `tools/directives.py` searches the last non-empty
  line of the field's doc comment, so a trailer wrapped across two `///` lines is reported as
  "something shaped like a trailer that this does not read" rather than accepted. Write the whole
  trailer on one line whatever its length: `rustfmt.toml` sets no `wrap_comments`, so a 200-column
  doc line survives `cargo fmt` untouched. [until: reviewed 2026-09-11]
- **A `NOT IMPLEMENTED` note in `crates/nvs-config/src/default.toml` stands over every key after it,
  not only the one below it.** `tools/directives.py`'s template parser clears the prose block on a
  blank line or a block header and nowhere else, and `--check-template` never notices, because it
  reads that prose only for a key the tree declares unread. Leave a blank line after the last key a
  note covers. [until: gone crates/nvs-config/src/default.toml:NOT IMPLEMENTED]
- **A project command writes `nvs.toml` into whatever directory it runs in, so an integration test
  that runs the binary in a scratch directory gets a tree it did not write.** `nvs check` there used
  to measure a program against nothing and now measures it against a deny-all file of its own
  making, which is how `crates/nvs-cli/tests/check_grants.rs:88` turned red on a change that never
  named it. A fixture meaning *no configuration* has to say so: pass `--no-init`, or set
  `NOVIS_NO_INIT` on the child. [until: reviewed 2026-09-11]
- **A member answering `CoreTy::Instance(C)` panics at run time while `C` has no instance member
  and no slot**, because `crates/nvs-stdlib/src/instance.rs:270` skips such a class when it builds
  the descriptor table, so the call compiles and type-checks and then dies in `FATAL: … is not a
  `Core` class with instances`. It reads like a registry wiring mistake and is an ordering fact.
  Land the class's first instance member in the slice that first answers one — the conformance floor
  wants it too, a handle with no members having no three questions to be asked.
  [until: reviewed 2026-09-11]
- **A `loop-goal.toml` check's `-p <crate>` goes stale the session a module moves between crates,
  and the failure then reads exactly like unwritten work.** Stage 2's event framing moved from
  `nvs-server` to `nvs-runtime` so `nvs-stdlib` could name it without closing a cycle, and its three
  checks went on naming `-p nvs-server` — which reports all nine tests as "did not run" while every
  one of them passes one crate over. When a `cargo-named` check reports its *whole* list missing,
  grep the test names across `crates/` before writing anything: a name that resolves somewhere else
  is a check to re-point, never a test to write again. [until: reviewed 2026-10-11]
- **`python tools/peek.py --locate` swallows the positional targets in the same call and says
  nothing about it.** A call written `peek.py sse.rs:362-448 --locate Foo Bar` prints the `file:line`
  anchors alone, so the window you asked for comes back missing and reads as though the file had
  nothing at that range. Send `--locate` as a call of its own and put the windows in the next one.
  [until: reviewed 2026-10-11]
- **A `loop-goal.toml` check naming a test the tree seems to already have can want a *second* test,
  because the older name is itself an earlier check's.** Stage 5's
  `connection_defaults_are_finite_for_every_field` reads exactly like a rename of
  `every_connection_bound_is_finite_with_nothing_configured`, which stage 1's `-p nvs-server` check
  names verbatim — so renaming would have turned an earlier stage red to close a later one. Grep the
  whole of `loop-goal.toml` for the name the tree already has before renaming anything, and where
  both names are claimed, the claim the two differ on is what the second test asserts.
  [until: reviewed 2026-10-11]
- **A `cargo-named` check naming a test *target* can never match anything.**
  `tests = ["spec_registry_coverage"]` read "did not run" while all ten tests in that file passed:
  `crate_tests` runs each test executable directly, so the `Running tests/<target>.rs` line cargo
  would have printed is nowhere in the output the check greps. When the name is a *file* under
  `tests/`, re-point it at the `fn` names inside it, in both toml copies.
  [until: gone tools/loop.py:def crate_tests]
- **An anchor inherited from a handoff or a goal file drifts, and the line it lands on still reads
  like the right one.** This goal opened with `crates/nvs-host/src/isolate.rs:665` for a drain gate
  and `crates/nvs-codegen/src/emit.rs:3415` for a `THROWN` test; both had moved about 150 lines onto
  plausible neighbours — a cancellation branch and a switch terminator — so reading one confirms a
  claim it does not make. Re-derive an inherited anchor by symbol
  (`python tools/peek.py 'file.rs:re:<symbol>'`) before citing it or editing beside it, and write the
  number back into the handoff. [until: reviewed 2026-09-11]
- **A `[context] modules` comment's `:NN-NN` anchor can name the construct *beside* the one its
  sentence is about, and `orient.py` inlines that window into the pack as the answer.** A handoff item
  inherits the anchor, so one wrong range in the manifest sends session after session to the wrong
  code with the right name printed over it. Trust the *name* in the comment over the digits beside
  it — `python tools/peek.py --locate <Type>` is the half that survives the file moving — and repair
  the manifest's range when you find one off. [until: reviewed 2026-10-11]
- **`nvs-ir` names `nvs_hir::…` freely in its doc comments and cannot name it in code: `nvs-hir` is
  one of its `[dev-dependencies]`, not a dependency.** `InstKind::New { class:
  nvs_hir::errors::FINISH_MARKER }` reads exactly like every neighbouring doc reference and fails
  with `E0433: unresolved module or unlinked crate`. The route down is a re-export through
  `nvs-types`, which depends on both — add one beside `CORE_SCRIPT_FINISH_CLASS` in
  `crates/nvs-types/src/lib.rs` rather than promoting the dev-dependency.
  [until: gone crates/nvs-ir/Cargo.toml:nvs-hir.workspace = true]
- **A `loop-goal.toml` check can name an *end-to-end* outcome the crate it is filed under owns half
  of.** `a_fatal_on_the_served_path_runs_no_exit_hook` is `-p nvs-host`, but the host hands every
  non-cancelled ending to the drain seam and `nvs_stdlib::script::run_exit_hooks` is what refuses a
  `FATAL`, so an empty-log assertion there guards the other crate's half and gating the ending out of
  the host would put one policy in two homes. Assert the half the named crate owns — the ending it
  handed over — and cite the seam that refuses it. [until: reviewed 2026-09-12]
- **`nvs_syntax::parse` hands back trivia but no tokens, so a formatter's runs are the *complement* of
  the trivia spans.** `Parsed` is `stmts`, `trivia` and `index`; every byte the grammar skips is a
  trivium, so what lies between two consecutive trivia is tokens and nothing else, and that is the
  tiling `crates/nvs-fmt/src/print.rs` walks. It is enough to rewrite whitespace and comments and it is
  not enough to *insert* a space inside `$a+$b`, so the first rule that needs token boundaries either
  puts them in `Parsed` beside the trivia — which `Lexer::with_trivia`'s own doc anticipates — or reads
  them from `nvs_syntax::tokenize`. [until: reviewed 2026-09-12]
- **A layout rule `nvs fmt` learns is green only if the corpus already follows it.**
  `crates/nvs-fmt/tests/identity.rs` asserts every `.nvs` file under `examples/` and `tests/` comes
  back byte for byte, and that test is goal `fmt`'s stage-2 acceptance check, so it is a floor every
  later stage has to keep green. Before writing a rule into the printer, format the corpus with it and
  read what moves — a rule the corpus disagrees with lands *with* a corpus reformat or not at all.
  [until: gone crates/nvs-fmt/tests/identity.rs:the_identity_printer_reproduces_every_corpus_file]
- **Seven `.nvs` files under `examples/` are CRLF in this working copy, so a formatter stage that
  writes a line break has to write the file's own.** `.gitattributes` says `eol=lf` and git
  normalizes them on the way into the index, but the checked-out bytes here are CRLF, so an inserted
  `"\n"` leaves one line of a file ending differently from every other and
  `crates/nvs-fmt/tests/identity.rs:36` fails on exactly those seven with a diff that prints as
  nothing. Take the break from the text once per file, the way `crates/nvs-fmt/src/brace.rs`'s
  `line_break` does, rather than writing `"\n"` at each site.
  [until: gone crates/nvs-fmt/src/brace.rs:line_break]
- **A `Rewrite` that moves a declaration has to cover one code run, and `use Core\Str;` is not one.**
  `crates/nvs-fmt/src/print.rs` tiles a file into trivia and the code between two of them and applies
  each edit inside one of those runs, so an edit spanning the space after a keyword is never taken and
  trips the `debug_assert` in `push_code` later. Move the one token that differs — `imports.rs`
  permutes the paths and not the declarations — and ask `print::one_code_run` before moving anything.
  [until: gone crates/nvs-fmt/src/print.rs:a rewritten range lies inside one code run]
- **A `.nvs` file can be CRLF in the working tree and LF in the index, and then every line of a
  diff against it reads as changed.** `.gitattributes` pins `*.nvs text eol=lf`, so a fresh
  checkout is LF, but `core.autocrlf=true` left `examples/match.nvs` and a dozen more CRLF on disk
  with `git status` still clean, because the commit-side normalization makes them equal. Run `git
  ls-files --eol <path>` before believing such a diff, and build a frozen fixture with the Write
  tool rather than `cp`, because `nvs fmt` writes LF. [until: reviewed 2026-10-12]
- **A `nvs-fmt` stage that reads the text between two nodes is reading comments too.** The gap
  between two children of one production is trivia as well as code, so a scan for a separator in it
  finds the `,`, `{` or `=>` inside a comment — and an arm boundary taken from one puts a rewrite in
  the middle of a run the printer copies whole, which trips `print.rs`'s ordering assertion in debug
  and corrupts the file in release. Mask the trivia out with `indent.rs`'s `commented` before
  searching a gap, the way `arm_starts` and `imports.rs` both do.
  [until: gone crates/nvs-fmt/src/indent.rs:arm_starts]
- **Changing what a request's params are breaks the extension silently.** `regions()` in
  `editors/vscode/src/regions.ts` wraps its `sendRequest` in a `try`/`catch` returning `[]`, so params
  the server refuses read as "this file has no markup" and the HTML services stop forwarding, with
  neither `cargo test` nor `npm run compile` seeing it. Grep `editors/vscode/src` for the `METHOD`
  constant of any request whose dispatch arm in `crates/nvs-lsp/src/server.rs` you retype, and land
  both sides together. [until: gone editors/vscode/src/regions.ts:} catch {]
- **A character a session cannot type — a private-use code point, an unusual glyph — can reach a file
  through `Write` and then match no `old_string` you write afterwards.** The character is dropped from
  the later call, so the one line needing a fix is the one line that cannot be addressed. Write such a
  character as code — `String.fromCodePoint(0xe000)` — from the start, and if a raw one is already on
  disk, `git restore <file>` and redo the edit rather than hunting for an anchor.
  [until: reviewed 2026-09-12]
- **A `cd` in one Bash call is still in force in the next one, so a later `git status` or `grep` answers
  about the wrong directory instead of failing.** The tool keeps its working directory between calls
  even though shell state does not, so `cd website && npm run sync:rules` leaves every following call
  rooted in `website/`, where `git status --porcelain -- website` reports zero changes and a `grep -r`
  finds nothing. Prefix the next call with `cd /d/mwl &&`, or run the one-off as
  `cd <dir> && <cmd>` knowing the move sticks. [until: reviewed 2026-09-12]
- **A `## commit:` section cannot name a path the session renamed away, so `git mv`'s staged deletion
  is left out and the rename lands half-committed.** `session.py --wrap` refuses a path that does not
  exist, and it commits by pathspec, so the old filename stays staged in the index while the new one
  goes in — a fresh checkout of that commit carries both. Name only the new path in the section, then
  commit the leftover deletion straight after the wrap. [until: reviewed 2026-09-12]
- **A new rule's `because` names only the records that created or amended it, and a "depends on"
  record listed there fails `records.py --check` against a file nobody may edit.** The relation is
  bidirectional, so every id in a `because` obliges that record's own `changes:` block to name the
  rule back, and a frozen record never acquires one — listing `0051` beside `0179` reported
  `0051.md:3 ... its changes: does not name the rule`. Put the ancestry in the new record's
  `Depends on:` bullet and leave `because` at the records that wrote the rule.
  [until: reviewed 2026-10-12]
- **A repository file read with Python's bare `open()` on Windows is decoded as cp1252, not UTF-8, so
  one em dash in a fixture raises `UnicodeDecodeError` mid-probe.**
  `json.load(open('crates/nvs-stdlib/tests/vectors/webcrypto.json'))` fails exactly that way, and it
  reads as a corrupt fixture rather than as a default nobody in this tree chose. Pass the encoding —
  `io.open(path, encoding='utf-8')` — in any throwaway `python -c` that reads a file out of the tree,
  which is one more reason to reach for `tools/peek.py` whenever the thing you want is nameable as a
  target. [until: reviewed 2026-09-12]
- **Registering a `Core` class trips two closed lists the five-edit checklist does not name.** A class
  with slots and no instance members fails `a_class_with_slots_has_instance_members_and_the_reverse`
  until it is added to that test's `HANDLES`, and a member answering `tainted` fails `nvs-types`'
  `a_verified_signature_does_not_launder_its_claims` until it joins that roster. Run `cargo test -p
  nvs-stdlib -p nvs-types`, not just the first, and write the sentence each list wants beside the entry.
  [until: reviewed 2026-09-12]
- **A `[context]` gap is closed by adding the path to `modules`, and there is no `files` field.**
  `tools/orient.py`'s `named_files` resolves every leftover `modules` pattern against `git ls-files`,
  so a goal may name a fixture, a tool or a `.nvst` case there, while a key the loader does not know
  is silently nothing at all. Put the path in `[context] modules`, or in a `[context.stage.N]
  modules` overlay when only one stage reads it, with the one-line comment the other entries carry.
  [until: gone tools/orient.py:named_files]
- **A `cargo-named` check's name can describe an assertion whose only copy is a `.nvst` case, and
  renaming closes nothing.** Stage 7's `webcrypto_jwe_vectors_decrypt_to_their_payloads` read as one
  more rename beside three JWE tests that all *write* tokens, while the decrypt direction lived only
  in `tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst`, which no `cargo test`
  output ever names. Read the *direction* a check's name states against what the crate's own tests
  assert before sizing the item: a Rust-side proof that is absent gets written, not spelled
  differently. [until: reviewed 2026-09-12]
- **An acceptance check that runs a `tools/*.py --show` can go red on the console's *code page* rather
  than on the tree, and the tell is a traceback ending in `UnicodeEncodeError`.** `rules.py` printed
  `core-classes/crypto-interop-tier`'s metadata and then died on the `‖` in its body, because a console
  here is cp1252 and that tool carried no `sys.stdout.reconfigure`, so a rule that was written and
  correct reported as unwritten work. Run a red check's own `argv` and read its *last* line before
  touching the tree; the repair is the three lines `tools/rules.py:445-450` now carries, in whichever
  tool prints the prose. [until: reviewed 2026-09-12]
- **A `loop-goal.toml` check name can cite an RFC section the tree deliberately stopped using as its
  oracle, and a grep for that citation then finds nothing at all.** A test's doc comment is the only
  place such a swap is recorded — `crates/nvs-stdlib/src/crypto.rs:4158-4162` says the P-256 exchange
  is read from the frozen WebCrypto set, so five checks citing RFC 5903, 7515, 7520 and 8037 read as
  unwritten work while every claim under them is asserted. When a grep for a check's cited RFC number
  returns nothing at all, read the doc comment of the test covering that primitive before budgeting
  the vectors. [until: reviewed 2026-09-12]
- **A `loop-goal.toml` `nvs-suite` check's `cases` are drafted names, and a case can be on disk under
  the name its author chose.** Eight of goal `webcrypto`'s stage 4 paths read "not written yet"
  against green cases, because no check named the tree's own spellings —
  `crypto-public-key-round-trips-every-kind-through-every-encoding.nvst` is the check's
  `crypto-public-keys-read-raw-spki-and-jwk-and-write-them-back.nvst`. List the directory for that
  member and compare `--TEST--` lines before writing anything: where the tree holds the claim the
  repair is the name in the goal file, and where it holds half of it the drafted case is real work.
  [until: reviewed 2026-10-12]
- **A `# NOT IMPLEMENTED` note in `default.toml` claims every key below it in the same block.**
  `tools/directives.py`'s template parser ends a prose block at a blank line or a header and never at
  a setting, so a note written above `#idle` also marked the `#max_redirects` under it and
  `-p nvs-config --test directives` failed naming that key. Put an unimplemented key last in its
  block behind a blank line, and keep the field's `[unread: <why> owner: <who>]` trailer on one line
  — the reader takes it from the doc comment's last line. [until: reviewed 2026-09-12]
- **A `Core` row's `mixed` parameter refuses `tainted` and admits `secret`**, so a comment beside one
  saying "the value is refused" is about one axis only. `CoreTy::Mixed` interns as `mixed`, which the
  assignment relation widens onto every qualifier bit, so the `secret` axis is a call-site rule instead —
  `crates/nvs-types/src/expr/quals.rs`'s `reject_secret_*_argument` family, one per carrier of the graph
  copy. Before pinning a `secret` refusal at a `mixed` parameter, run the three-line program and read the
  output: nothing reported means that carrier has not joined the family yet.
  [until: reviewed 2026-09-13]
- **A red `the one-file reference still regenerates from the binary` check can be about prose rather
  than about Rust.** `docs/novis.md` is generated from three inputs — `nvs meta --json`, the chapters
  under `docs/reference/lang/` and `docs/reference/tools/`, and an optional
  `docs/reference/core/<Class>.md` per class — so an untracked chapter somebody added turns that floor
  red with no member having changed. Run `git status --short` before diagnosing it, and run `python
  tools/reference.py` only once the prose it reads is settled, because the tool bakes whatever is on
  disk into a document that is then committed. [until: reviewed 2026-09-13]
- **A `rule:` target after an option on `peek.py`'s command line is refused as an unrecognized
  argument.** The option ends the positional run argparse is collecting targets into, so
  `peek.py a.rs:1-20 --context 5 rule:types/conversion` fails outright while the same call with the
  `rule:` token written before `--context` reads both. Put every target first and every option
  last, which is the order the tool's own usage line prints them in.
  [until: reviewed 2026-09-13]
- **A `rule:` token written as an illustrative placeholder turns the rulebook floor red.** `python
  tools/rules.py --check` resolves every `rule:` citation under `docs/`, the playbook and the handoff
  included, so an example that spells `rule:` followed by a made-up topic and slug fails the `1 floor`
  rulebook check for every later session while reading as good prose — the bullet that first described
  this trap did exactly that. Write a real id in an example, `rule:types/conversion`, or describe the
  token in words; `session.py --wrap` refuses a wrap body that carries one, but an Edit made by hand
  goes straight to the tree, so run the check yourself after one. [until: reviewed 2026-09-13]
- **The pack's "THE DRIVER'S LAST ACCEPTANCE CHECK FAILED" line survives a hold, so a done-claim that a
  person then closed by hand opens the next session on a check that is already green.** The driver writes
  that failure when it stops and the ledger keeps it verbatim, while the commits closing it can land under
  someone else's hand while the run is held — so the line can describe a tree several commits older than
  the one the session is in. Run the named check's own `argv` once before treating it as the session's
  work; when it exits 0 the job is to collect the acceptance and re-claim, not to re-fix.
  [until: reviewed 2026-09-13]
- **A `# NOT IMPLEMENTED` note in `crates/nvs-config/src/default.toml` covers every key *below* it
  down to the next blank line, not just the one under it.** Writing it above a run of `#username`,
  `#password` and `#password_file` marked all three, and `directives.rs`'s
  `every_unimplemented_key_in_the_default_file_is_marked_as_one` then failed naming `password` — a
  key whose field has a reader and rightly carries no `[unread:]` trailer, so the failure reads as a
  missing trailer rather than as a misplaced note. Order the block so every key that is read sits
  above the note and the unread one directly under it. [until: reviewed 2026-10-13]
- **`peek.py`'s answer dies with `OSError: [Errno 22]` when it is piped into `head`.** The pipe closes
  under it and Python reports the broken write as a traceback on top of a partial answer, so a call that
  looked like a narrow read comes back as a crash with no usable result. Narrow the target instead — a
  `:NN-NN` window, a `re:pat:3`, a `--locate` — rather than trimming a wide answer downstream.
  [until: reviewed 2026-09-13]
- **A `rule:` token is not derivable from the rule's title, and only `rules.py --check` says so.** Three
  citations written from a remembered title were each one word off the real id —
  `member-names-are-full-words` for `members-are-full-words`, `a-unit-is-a-type` for `units-are-types`,
  `library-placement-tests` for `tier-placement`. Grep the topic's JSON for `"id"`, or paste the token into
  `python tools/peek.py rule:<id>` and see whether a fragment comes back.
  [until: reviewed 2026-09-13]
- **A `peek.py` `re:` target multiplies its context by every hit, and a member's name hits
  four places.** `socket.rs:re:answerSocket:30` came back with the registry row, the reference
  card, the helper body and two test assertions — six windows and about 6k of context for one
  question about a signature. Anchor the pattern at the definition (`re:^fn name`, `re:^const
  NAME`) or spend one `--locate` call first, and keep the `:N` small when the word is a spelling
  the whole file talks about. [until: reviewed 2026-09-13]
- **A new `nvs.toml` key its boot validator names by string already counts as read.**
  `tools/directives.py` scans non-test crate source for the dotted key as a literal, and a refusal in
  `crates/nvs-config/src/http.rs` is one — so the `[unread:]` trailer and the `NOT IMPLEMENTED` note a
  key nothing *applies* looks like it owes are refused, by that gate and by
  `no_key_with_a_reader_still_claims_to_be_unread`. Land the key with its refusal and no trailer, and
  say in the handoff that nothing acts on the value yet. [until: reviewed 2026-09-13]
- **A `[[check]]`'s `want` string can be one its tool never prints when the finding count is zero.**
  Stage 5 wanted `none -- every carried-gaps owner is live or struck` from `python tools/playbook.py
  --check`, whose carried-gaps section sat inside an `if rows:` with no empty branch, so fixing every
  row made the section vanish rather than say `none` and the check stayed red over correct data. When a
  `want` line is still absent after the data is right, read the tool's printer for that heading before
  re-reading your own edit. [until: reviewed 2026-09-13]
- **An acceptance `want` is a substring of a tool's own output, so rewording a summary line breaks a
  check in a stage you are not touching.** `owners.py`'s roster summary is read by a floor
  `[[check]]` wanting `, 0 owned by a retired goal`, and rewriting those counts into the `label: N`
  form the live stage asked for would have gone red on a stage that passed sessions ago. Before you
  reword any line a tool prints, grep `docs/agent/loop-goal.toml` for a fragment of it — a mode whose
  output is a verdict can take the new form while the human-facing roster keeps the frozen sentence.
  [until: reviewed 2026-09-13]
- **A tool's prose citing a crate file without its `crates/` prefix fails `tools/check-links.py`, and
  the path it reports is a *suffix* of the one you wrote.** `MENTION_RE` anchors a bare mention at a
  top-level directory, so `nvs-stdlib/src/tests/vectors.rs` matches from its inner `tests/` and is
  resolved as `tests/vectors.rs` against the repository root, where nothing is — three of those in one
  comment held the whole floor red. Cite a crate file from the root, and read a `retired` finding whose
  path you cannot find in the line it names as the tail of a longer one.
  [until: gone tools/check-links.py:MENTION_TOPS]
- **The sweep stage 4 of goal `gap-register` was authored from both over- and under-reports, so the
  list it prints is not the work.** `python tools/peek.py "crates/**/*.rs:re://! #+ .*(not yet|owe|still
  missing|not armed)"` matches `owe` inside `lowers` and `borrowed`, so headings about lowering and
  borrowing come back, while `# Not here yet` and `# What is not a region yet` match nothing and stay
  hidden — the tree holds three times as many as the goal's list names. Run `python tools/owners.py`
  instead and read its `OWED` section: the pattern is word-bounded and lets up to three words sit
  between `not` and `yet`. [until: gone tools/owners.py:OWED]
- **Renaming a module-doc section leaves every citation of it pointing at a heading that no longer
  exists, and nothing `verify.py` runs reads a `§ *Title*` reference.** Splitting `route.rs`'s and
  `serve.rs`'s headings stranded `crates/nvs-server/src/lib.rs:88`, and moving a gap block moved the
  `file.rs:NN-NN` spans that `docs/agent/goals/57-m7-server-surface.md:67` quotes for it. Grep the
  old title *and* the old line span across `crates/` and `docs/agent/goals/` in the same call that
  makes the edit.
  [until: reviewed 2026-09-14]
- **A module doc's `# What is not here yet` is as likely to be stale as to be a real gap.** The prose
  was written before the goal that built the member, and nothing re-reads a paragraph when code
  lands under it, so a section listing unregistered members often describes a surface that is
  registered and tested today. Grep the registry, the lexer or the symbol the paragraph names before
  writing one up as a gap with an owner: what has landed becomes prose, and only what the code still
  refuses becomes a `# Known gaps` item.
  [until: gone tools/owners.py:sections outside Known gaps]
- **A program check that fails once and cannot be reproduced is a race inside the fixture, and the
  error names the member that tripped over the damage rather than the one that caused it.**
  `examples/queue-purge.nvs` missed its claim window, so its own `Core\Queue::delete` removed the row
  and the next line reported `Core\Queue::status: no job 29`, which reads as a queue bug. Check
  `.loop/log.md` for whether the line repeats, then re-run the fixture a dozen times serially and
  concurrently — a bounded wait whose answer is discarded is what turns a timing miss into a false
  accusation further down. [until: reviewed 2026-10-14]
- **A stage the handoff calls unwritten can be entirely on disk under other names, and a
  `cargo-named` check only ever reads names.** Goal `m5-proofs`'s stage 2 landed both deadlock tests
  and its `.nvst` case in one commit spelled `a_deliberate_deadlock_is_ended_by_the_deadline`, while
  the check names `two_tasks_waiting_on_each_others_channel_are_ended_by_the_groups_deadline`, so
  two sessions read "did not run" as work outstanding. When a check reports a test that did not run,
  `git log -S` the *claim* before writing it: where the assertion is already there under another
  name and no other check names that name, the whole fix is the rename.
  [until: reviewed 2026-09-14]
- **A `# Known gaps` section is **one** item to `tools/owners.py`, however many paragraphs it has,
  so a second `— owner:` trailer inside it fails as `two owner tags in one item`.** The failure
  reads like a formatting quibble and is not: the block's *last* line is the tag, and a paragraph
  break does not start a new item, so a module that owes two unrelated things either names one owner
  for both or writes them as a numbered list. Run `python tools/owners.py --check
  --untagged-is-an-error` after editing any `# Known gaps` block — it names the file and line, and
  it is a floor check, so a stray trailer holds the whole run. [until: reviewed 2026-10-14]
- **A `re:` peek target's `:N` suffix caps the hits, and `:0` means *uncapped*, not *default*.**
  `python tools/peek.py 'docs/agent/loop-goal.toml:re:^stage = :0'` printed all 712 matching lines
  and about 8k of context, where the same target without the suffix would have stopped at the
  default. Leave the suffix off unless you want more than the default, and never write `:0` over a
  file whose matches you have not counted first. [until: reviewed 2026-09-14]
- **`peek.py`'s targets are one run of positionals, and an option written between two of them drops
  everything after it.** `python tools/peek.py A.rs:1-9 --context 4 B.md` answers `unrecognized
  arguments: B.md`, because `argparse` closes the positional list at the first option it meets and
  the tool's usage line does not say so. Write every flag before the first target — `peek.py
  --context 4 A.rs:1-9 B.md` — or drop the flag and give the one target that needs it its own
  `:re:pat:3` suffix, which is per-target anyway.
  [until: gone tools/peek.py]
- **The acceptance failure the pack quotes can already be repaired on disk, because commits land
  between the sweep that wrote the line and the session that reads it.** Goal `m5-proofs`' 100k-task
  check was reported "did not run" by a sweep that ended `17:06`, and the rename giving that test
  the name the check filters on was committed at `18:50:52`, before the run the pack came from
  began. Compare the ledger entry's run header against `git log --date=iso` before diagnosing, and
  where head is the newer of the two, re-run the check's own `argv` and believe that.
  [until: reviewed 2026-09-14]
- **A goal's § *Standing decisions* can name a mechanism `tools/loop.py` no longer has.** Goal
  `m4b-editor` and two `loop-goal.toml` comments cite `tools/loop.py:1606`'s `MEMO_DIRS` hashing
  `crates/` and `examples/` alone — that name is gone, the line is `is_carried`, and the memo keys each
  check against the partitions `reads_of` names, under which the host checks were memoized, the
  opposite of what the comment claimed. Read the mechanism at the anchor before arguing from it, and
  rewrite the sentence in the same session rather than working around it. [until: reviewed 2026-09-14]
- **A goal that tells you to read a candidate crate's own manifest may be naming a crate that is in
  neither `Cargo.lock` nor `~/.cargo/registry`, so there is nothing on disk to read.** A crate reaches
  the registry cache only once something in the workspace resolves it, and a dependency still being
  considered never has. Read the feature map and the dependency kinds from
  `https://crates.io/api/v1/crates/<name>/<version>` and its `/dependencies`, and name the version you
  read in the record, because the answer is a property of that release. [until: reviewed 2026-09-14]
- **A new directive has homes outside Rust.** The key is a field on `tree.rs`'s block struct,
  resolves in that block's module and is transcribed into `default.toml` — but the rule fragment
  printing the block's keys **is** the rule, and the record deciding it goes in `because` in
  `docs/rules/<topic>.json`, the only home for that metadata. Edit the fragment and the json, then
  `python tools/rules.py --render`, or `verify.py` fails on the stale generated chapter — a sentence
  about a file nobody edits rather than about the key you added. [until: reviewed 2026-09-14]
- **`cargo test --lib -p nvs-cli <name>` cannot run one of that crate's cases: it has no library
  target.** The cases live in the `nvs` binary, so the answer is `no library targets found in
  package` and not a missing test. Run `cargo test --bin nvs <name>` — no `-p`, which is what
  `AGENTS.md` asks for anyway, and the bin name is unique across the workspace.
  [until: exists crates/nvs-cli/src/lib.rs]
- **The handoff's `## Next group` can name a slice that already landed, and the pack still prints it
  as your item in full.** Stage 5's six recording-manager cases were written in `bcdfb3b4a`, two
  sessions before the handoff that listed them as open, so this session's item was work already on
  disk. The driver's failing acceptance check is the discriminator and it costs one call — it names
  the artefact that is actually missing, so `grep -rl 'fn <that name>('` over the crate before
  starting says which items of the group are left to do. [until: reviewed 2026-09-15]
- **Inserting a Rust item above an existing one, anchored on that item's `#[attribute]` line, steals
  its doc comment.** Every `///` above an item attaches to whatever item comes next, so the old doc
  lands on the new function and the only report is `missing documentation` naming the *old* one, which
  reads as a doc you forgot on code you did not touch. Anchor the insertion on the first line of the
  target's own doc block instead, so the doc travels with the item it describes.
  [until: reviewed 2026-09-15]
- **A floor check can name a test a *later stage of the same goal* has since renamed, and the
  acceptance failure then reads exactly like unwritten work.** A stage that widens a test renames it,
  and the carried claim goes nameless in that same commit, so the floor — which runs rarely — reports
  it sessions later. `git log -S '<the missing name>' -- crates/` names the renaming commit in one
  call, and the repair is then that name in both toml copies, never a second test.
  [until: reviewed 2026-09-15]
- **A block inserted directly above an item lands inside the doc comment of the item before it, and
  everything still compiles.** `///` lines attach to whatever item follows them, so a new `struct`
  anchored on the line of an existing one ends up wearing the first half of that item's comment
  while the old item keeps the second — two docs that each read as a non-sequitur, with no warning
  anywhere (`crates/nvs-cli/src/serve.rs`'s `FleetLease` and `Scheduled` were one, repaired here).
  Anchor an insertion on the blank line *above* a doc comment rather than on the item, and read the
  `///` lines on both sides of the seam back afterwards. [until: reviewed 2026-10-15]
- **`peek.py --locate Type::member` finds a call site or nothing, never the definition, when the
  member is on an inherent `impl`.** A member is written `fn of(` under `impl Registry`, so the
  qualified spelling only exists where somebody *calls* it: `--locate Registry::of` answered with a
  line inside that file's own tests and `Registry::request` with `NOT FOUND`, while both were defined
  and public three hundred lines above. Locate the type instead, or take the file's outline with
  `grep -n '^\s*pub fn ' <file>` and read the region. [until: reviewed 2026-09-15]
- **`peek.py` refuses every target written after an option flag.** `peek.py a.rs:re:x --context 30
  b.rs:re:y` dies with `unrecognized arguments: b.rs:re:y`, which reads like a misspelled target and
  sends you hunting the locator syntax instead of the argument order — argparse stops collecting
  positionals at the first optional. Put every target first and the options last, or give each target
  its own `:N` context suffix, which wins over `--context` anyway. [until: reviewed 2026-09-15]
- **A `peek.py` `re:` target whose pattern matches a common token prints every hit in the file, and
  a big file answers with tens of thousands of lines.** `crates/nvs-host/src/isolate.rs:re:fn |struct
  |impl ` came back as 88 hits and 173 KB, spilled to a persisted file, and answered nothing the
  question had asked. Write the pattern so it can only match the construct you want — `re:fn
  over_socket|enum Charge` — or use `--outline <file>` and `--locate <symbol>`, which return seams
  and `file:line` anchors rather than bodies. [until: reviewed 2026-09-15]
- **A doc link to a `#[cfg(test)]` item, or to another module's private `fn`, builds clean, passes
  clippy and fails only `verify.py --doc`.** `verify.py`'s default gate does not include the doc leg
  and only a `DONE` claim runs it, so every unresolvable link a goal writes is inherited by the goal's
  last session as a wall of `broken-intra-doc-links`. Write a `#[cfg(test)]` item in plain backticks,
  path-qualify one that lives in a sibling module (`[`registration::Action`]`), and add `()` to a name
  that is both a function and a module. [until: reviewed 2026-09-15]
- **A handoff whose `## Next group` says its stage has nothing open still picks *that* stage's
  `[context.stage.N]` overlay for the next session.** `orient.py` reads the stage number from that one
  line, so a goal whose later stage is still red opens the next session on the finished stage's rules,
  shapes and traps and none of the red one's — a stage-13 rulebook flip arrived with stage 12's
  runaway rules and no `A rule fragment` shape, though the overlay naming it was already on disk. When
  a stage closes and the goal has another, name the **next** stage in that line — the driver's failing
  check says which one it is. [until: reviewed 2026-09-15]
- **A rule cited beside a claim does not always state that claim.**
  `rule:core-classes/queue-storage-is-a-table` cited `rule:core-classes/schema-plan` for keeping a
  filtered index out of v1, but that exclusion is `rule:core-classes/schema-vocabulary-is-closed`'s, so
  a record that builds its `changes.modifies` list from the citation edits a fragment that never made
  the claim. Grep the claim itself across `docs/rules/*/*.md` before naming a rule in a `changes:`
  block, and fix the misdirected citation in the same commit. [until: reviewed 2026-09-15]
- **A `splice.py` block unique in the file can stop being unique inside the same patch.** Blocks
  apply in order against one buffer, so an earlier block's NEW text is matched too — a one-line
  accessor copied into a second impl made `&self.columns` appear twice — and the line numbers in the
  refusal are post-edit, matching neither the file nor a `grep -n`. Widen the anchor to take the
  signature above it, rather than re-grepping a file that does not hold the duplicate.
  [until: reviewed 2026-09-15]
- **A `loop-goal.toml` check can name work the tree's own module docs argue *against*, and the rule
  it cites is what settles it.** `nvs_stdlib::queue::schema`'s doc priced an accumulating `errors`
  array and declined it, while `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` had
  specified that accumulation all along, down to the record section fixing its cap. Read the cited
  rule before believing a comment that prices a trade: a fragment is always currently true, and a
  module doc's gap section is the half that goes stale under it. [until: reviewed 2026-09-15]
- **A `loop-goal.toml` `command` check is not ordered before a fixture by sitting above it.** The
  sweep runs the floor's programs before any command, so two `nvs queue migrate` blocks written
  above `examples/queue.nvs` to converge its table ran a hundred checks after it, and a column
  added to the queue's schema left all three queue fixtures claiming nothing. File order orders a
  tier and `tools/loop.py`'s `Goal.setup_checks` orders the tiers, so a check preparing something
  outside the tree carries `setup = true`, and `memoize = false` beside it.
  [until: gone tools/loop.py:setup_checks]
- **A goal that overturns a behaviour leaves the carried floor checks asserting the old one, and
  nothing goes red until `session.py --wrap` gates a `DONE`.** A `cargo-named` check matches a test
  by name, so one deleted by the work that made it false reads as `did not run` rather than as a
  failure, on a floor that runs one session in ten. When a slice deletes or renames a test, grep its
  name in
  `docs/agent/loop-goal.toml` *and* `docs/agent/goals/<n>-<slug>.toml` in that same slice and
  re-point the check at what now holds the truth. [until: reviewed 2026-09-15]
- **A gap item's `— owner:` tag has to be the item's *last* line, and a paragraph continuing that
  item under it reads as untagged.** `tools/owners.py` takes the tag off the last line, so a gap
  re-owned by writing the tag under the sentence that named the new owner failed
  `--untagged-is-an-error` on the goal-end sweep, which the floor's one-session-in-ten schedule had
  not run since. Write the tag under the item's final paragraph, and run `python tools/owners.py
  --check --untagged-is-an-error` in the same session as any edit to a `# Known gaps` block.
  [until: reviewed 2026-09-15]
- **A `file:NN` anchor into a prose doc cannot be re-derived from a symbol, and a doc that indexes
  work loses lines every time an entry goes green.** A goal's stage 0 named an index row as
  `<doc>:62`'s owner cell and the anchor landed in the middle of the next section's prose, because
  rows had been struck from the table since the goal was written and three of the rows still there
  were plausible candidates. Date the anchor rather than guessing which one: `git log
  --diff-filter=A --format=%H -1 -- <the file that wrote the anchor>` names the commit it was
  written at, and `git show <sha>:<doc> | sed -n '<NN-14>,<NN+2>p'` prints the line it meant.
  [until: reviewed 2026-09-15]
- **Renaming a Rust test can turn a carried floor check red, because a `cargo-named` check names its
  tests as strings.** Stage 2 wanted three test names that `crates/nvs-types/tests/arrays.rs` almost
  had, and renaming the near-misses into them would have broken the `stage = "1 floor"` block in
  `docs/agent/loop-goal.toml` that names the old three verbatim. Before renaming or deleting a test,
  grep `docs/agent/loop-goal.toml` for its name — a hit means the name is an interface, and the new
  test is written *beside* the old one with a claim of its own rather than over it.
  [until: gone docs/agent/loop-goal.toml:kind = "cargo-named"]
- **Renaming a test to match the live goal's check name can turn a *carried floor* check red, and the
  report then reads as unwritten work.** The floor is the previous goal's list verbatim and may never
  be edited to make something pass, so where two checks claim one test the live goal's check is the
  half that moves. Grep all of `docs/agent/loop-goal.toml` for the name the tree already has before
  renaming anything: a hit under `stage = "1 floor"` means rename the check, in both toml copies.
  [until: reviewed 2026-09-16]
- **`peek.py`'s `--context N` swallows every target written after it.** `python tools/peek.py
  A.rs:@sym --context 20 B.rs:@other` stops with `unrecognized arguments: B.rs:@other`, because the
  option's value and the positional targets are one argparse list and the run of targets after the
  flag has nowhere to land. Put `--context`/`--window` after the last target, or use the target's own
  `:3` suffix, which wins over the flag anyway. [until: reviewed 2026-09-16]
- **A `///` that links a test by a `tests::` path passes every session's gate and fails the goal's
  last one.** `mod tests` is `#[cfg(test)]`, so `cargo doc` resolves no `tests` item and
  `-D warnings` turns the link into `unresolved link` — and `python tools/verify.py` compiles the
  docs only under `--doc`, the gate a DONE claim needs and no ordinary session runs, so they
  accumulate across crates unseen. Name a test in backticks and never in brackets, and when `--doc`
  is red on one, `grep -rn 'tests::' --include=*.rs crates/` finds every sibling in one call.
  [until: reviewed 2026-09-16]
- **Striking a ratchet file's last key fails the gate that reads it, and the message's first
  branch is the wrong one.** `every_outstanding_key_names_an_owner` in
  `spec_registry_coverage.rs` asserted `checked > 0` and offers "delete the parity program" before
  "the reader broke"; the files are how the next spec section is walked and `owner_problem`'s own
  cases already exercise the arithmetic. Assert the empty file still holds its header instead of
  taking either branch. [until: gone crates/nvs-stdlib/tests/spec_registry_coverage.rs:RATCHETS]
- **`loop.py --goal-only` names one red check at a time, so one stale carried-floor entry hides the
  next and each sweep costs minutes.** A rename inside the goal's own work breaks that goal's own
  floor: a `cargo-named` check matches a test's name and an `nvs-suite` check a case's path, so
  stage 5 renaming both halves of the 25-digit decimal claim left `did not run` and, one sweep
  later, `is not written yet`. Check every name in the goal file at once before re-running it —
  each `cases` path against the disk, each `tests` name as a *prefix* of some `fn` in the tree,
  since `cargo test` matches by substring. [until: reviewed 2026-09-16]
- **`splice.py` cannot tell two byte-identical blocks in one file apart.** Five identical
  `Inst { … }` literals in `crates/nvs-ir/src/lower/control.rs` are one anchor five times over, and
  a patch naming it edits the first and leaves four. Widen each anchor with the lines around it, or
  — when every occurrence wants the same edit — give that one block to the Edit tool's `replace_all`
  and keep the rest of the run in the patch. [until: exists tools/splice.py:--occurrence]
- **The handoff's `## State` can name a different acceptance check than the one the driver is red on,
  and the two have different repairs.** Goal `unowned-closures`'s stage 6 is `owners.py`'s
  `unowned: 0`, so a `## State` naming that number reads as "the whole register is the work", while
  the red floor check wanted the tool's `0 owned by a retired goal` line — whose repair is one owner
  tag on one gap. Read the `want` the pack prints in full and grep the tool's own output for that
  exact string before budgeting anything: `python tools/owners.py` prints nine counts and a
  sentence, and a check may want any of them. [until: reviewed 2026-09-16]
- **A `peek.py` `"## Heading"` target never matches a `# Known gaps` block.** That heading lives
  inside a `//!` doc comment, so the line starts with the comment marker and the heading locator —
  which matches a line beginning with the hashes — reports no heading and prints nothing. Every
  module doc this goal reads holds one, so ask for it as `crates/nvs-ir/src/lib.rs:re:Known gaps:40`
  or take the line number from `python tools/owners.py` and read a range around it.
  [until: reviewed 2026-09-16]
- **A `carried-gaps.md` bullet's `gap N` can name a number the module no longer has, and
  `owners.py --check --reasons` passes anyway.** That check only asks that *some* bullet name the
  module's path, so § *Unowned*'s three bullets citing `crates/nvs-runtime/src/graph.rs` gaps 1–3
  read as covering a file whose list now holds two — one of those gaps closed and one renumbered.
  Read the module's own numbered list before trusting a register bullet's number, and re-point the
  bullet in the slice that touches the file. [until: reviewed 2026-09-16]
- **A module's gap numbers shift the moment a bullet above one is closed, so a goal list or a handoff
  item naming `gap 2` can point at a different bullet than it did when it was written.**
  `tools/owners.py` numbers a `# Known gaps` bullet by its position in the block, and closing the one
  above it renumbers every one below without touching a word of the prose that names them elsewhere.
  Match on the gap's *sentence*, not its digit — `python tools/owners.py | grep <file>` prints each
  open bullet's opening line beside the number it currently has. [until: reviewed 2026-12-16]
- **A `# Known gaps` item can describe a hole another crate already closed.** `nvs-server`'s
  scheduler gap said a fire's context carried no configuration, while `nvs-cli`'s `Fires` implementor
  had been putting the tree on it since a commit that touched only that one file — nothing walks a
  gap's own claim, so the register counts items rather than truths. Read the code an item anchors
  before building it, and where it is already closed strike the item and state the contract in the
  doc that now holds it, which is `AGENTS.md`'s *tested code ahead of a record wins* applied to a
  gap. [until: reviewed 2026-09-17]
- **A `# Known gaps` item can be prose the code has already closed.** `crates/nvs-cli/src/openapi.rs`
  gap 5 said an enum capture reaches the document as *any* while `RouteParam::admits` had been handing
  `schema` the case spellings under a guard test. Nothing recompiles a gap's sentence when the code under
  it moves, so the item is the one part of a module doc that is not evidence. Read the path an item names
  and the test beside it before deciding its owner, and strike the item where the code is ahead of it.
  [until: reviewed 2026-09-17]
- **A stale carried floor check is reported the session the floor gate opens, and reads exactly like a
  regression this session caused.** `tools/loop.py`'s `FLOOR_GATE_EVERY` holds the floor on most
  iterations — `.loop/log.md`'s `goal cost` says `(floor gate shut)` over ten checks where an open one
  runs a hundred and eighty — so the earliest-stage red is only the earliest of what ran. Read the
  previous entries' `goal cost` lines before treating an earlier-stage failure as something the last
  commit did. [until: gone tools/loop.py:FLOOR_GATE_EVERY]
- **`peek.py --locate` takes bare symbol names, not `path:re:pat` targets, and a target handed to it
  answers `NOT FOUND` rather than saying so.** Its positional form accepts `file:re:pat`, so the same
  string looks like it should work under `--locate`, where it is looked up as a symbol whose name is
  that whole string. Pass a `re:` target as an ordinary positional — it prints the matching line alone
  — and keep `--locate` for names. [until: gone tools/peek.py:--locate]
- **A handoff item's anchor can name the right check and the wrong column, because a fixture read
  for its key list says nothing about its column types.** An item calling `every_construct`'s indexed
  `token` column text needing a prefix meant an unreachable gap: `crates/nvs-db/src/ddl.rs:1201`
  declares it `ScalarType::Uuid`, and `crates/nvs-db/src/schema.rs:684` refuses an unbounded column
  in any key. Read the column declarations before budgeting a slice a past session anchored, and
  `git log -- <file>` a construct said to be missing — a removed one names why it went.
  [until: reviewed 2026-09-17]
- **A `cargo-named` check runs its test *by name*, so work landed under a different test name leaves
  the check reporting "did not run" over finished code.** The name is the contract the goal was
  authored with, and a check edit has to be made twice — in `docs/agent/loop-goal.toml` and in the
  goal's own copy under `docs/agent/goals/`. Read the stage's `tests = [...]` before naming a test,
  and rename the test to the check when they have already drifted.
  [until: reviewed 2026-09-17]
- **Editing a rule fragment leaves the website's copy of it stale, and nothing turns red.**
  `python tools/rules.py --render` rewrites `docs/rules/<topic>.md` and `verify.py` gates that,
  but `website/src/content/docs/docs/rules/` is a second rendering no Python tool touches — it was
  already a goal behind at this session's HEAD. Run `node scripts/sync-rules.mjs` from `website/`
  after any fragment or `<topic>.json` edit, and expect the diff to carry whatever the sessions
  before you left behind as well as your own rule.
  [until: exists tools/verify.py:sync-rules]
- **A module doc section you *write* re-breaks a floor check an earlier goal closed tree-wide, because
  `tools/owners.py` reads a heading's wording and never what the section says.**
  `# Reaching a core that has not started yet` described a landed handle and owed nothing, but `OWED`
  matches `not … yet` in a title, so `sections outside Known gaps: 0` was red for a whole goal. Title a
  new `//!` section for what the module does, and run `python tools/owners.py | grep "sections
  outside"` in any session that adds one. [until: gone tools/owners.py:sections outside Known gaps]
- **A handoff calling a stage "landed, end to end" claims behaviour, not that stage's acceptance
  artefacts.** Goal `core-class-tests` stage 2's checker, IR and codegen were all on disk and the
  class test answered, while none of its three `cargo-named` tests and neither named `.nvst` path
  existed. Grep a red check's `tests` against `crates/` and its `cases` against `tests/conformance/`: a
  whole list missing while the behaviour runs means the artefacts are the work, not a re-point.
  [until: reviewed 2026-09-17]
- **Adding a bullet here can turn `python tools/chain.py --check` red, because the live goal's
  `[context] playbook` selector then matches two bullets.** A selector is a lead-in prefix
  (`Writing a test case > a tds`), so a bullet whose first words repeat an existing one makes it
  ambiguous, and the check names the *goal file* rather than the playbook the edit landed in. End
  the selector in `*` to take every match, in `docs/agent/loop-goal.toml` and in the
  `docs/agent/goals/<goal>.toml` copy both. [until: reviewed 2026-09-17]
- **The full `verify.py` gate can go red on files no session of yours wrote, because another writer
  edits this tree at the same time.** A clean `cargo check --all-targets` minutes earlier and errors in
  a crate you barely touched are the tell; `git status --short` and `git diff --stat` say whose each
  change is. Where one file carries both writers' work, write your own hunk to a patch with the Write
  tool, `git apply --cached --recount` it, and leave that path out of the wrap's `## commit:` list,
  which stages whole files. [until: reviewed 2026-10-17]
- **`Edit` matches `old_string` as a substring, so leading whitespace in the anchor does not pin the
  indent — it matches every line indented at least that far.** Adding a field beside `public: true,`
  in 22 files, the pass anchored at twelve spaces also hit the twenty-space copies in
  `crates/nvs-runtime/src/object.rs`, and the follow-up pass anchored at twenty spaces inserted the
  field there a second time; `cargo build` caught it as *field specified more than once*, and
  nothing before that did. For a repeated one-line insertion write the whole run as a `python
  tools/splice.py --patch` file, which matches every block before it writes a byte.
  [until: reviewed 2026-12-17]
- **An `ir::Class` field typed with an `nvs_types` type fails to compile in `nvs-codegen`.** That
  crate lists `nvs-types` under `[dev-dependencies]` only, so the lib half sees `nvs-ir` and
  `nvs-runtime` and nothing above them, and the error reads as a missing dependency rather than a
  deliberate boundary. Re-export the type from `crates/nvs-ir/src/lib.rs` and name it `nvs_ir::…`
  in codegen. [until: reviewed 2026-09-17]
- **A `peek.py` `re:` target carrying a context number multiplies that context by every file the
  glob matches.** `"crates/nvs-stdlib/src/*.rs:re:Core.Request:2"` came back as 45 KB in one call,
  because the two lines of context are applied per hit and a crate-wide glob had dozens of them.
  Ask for the matching line alone first — a bare `re:pat` — and add the context number only once
  the target has narrowed to one file. [until: reviewed 2026-09-17]
- **`peek.py --locate` silently discards the positional targets in the same call.** A call that
  read three regions and located one symbol answered with the anchor alone, so all three reads had
  to be sent again. Give `--locate` its own call, or drop it and read the regions whose anchors you
  already hold. [until: reviewed 2026-09-17]
- **`cargo fmt` after a run of Edit-tool changes floods the session with full-file diffs.** The harness
  reports every file a command touched that you had previously read, and a tree-wide `cargo fmt`
  touches all of them at once — one call cost about 12k of context here, more than the edits it was
  tidying. Format the crates you actually changed (`cargo fmt -p nvs-types -p nvs-ir`) or let
  `verify.py`'s fmt leg report instead, and reach for the tree-wide one only when the wrap is already
  written. [until: reviewed 2026-09-17]
- **A `peek.py` flag written *between* two targets makes it reject every target after it.**
  `peek.py A.rs:10-20 --context 4 B.rs:re:pat` fails with `unrecognized arguments: B.rs:re:pat`,
  because argparse cannot split one variadic positional list around an option — the same call with
  the flag last reads all of them. Write every target first and every flag at the end, which is how
  the tool's own usage line spells it. [until: reviewed 2026-09-17]
- **A milestone's `Carried by` cell goes stale the session the *last* goal carrying it walks, and
  only the goal-end sweep reads it, so it turns a DONE claim red one check before the switch.** M7's
  cell still listed nine walked goals because nothing rewrites it when a goal is reached — the cell is
  derived from the chain and the registers, and no session-level gate derives it. When `plan.py
  --check` reports a structural finding naming a milestone, run `python tools/plan.py --past` to see
  which milestone completed and `python tools/plan.py --sync` to write the cell; it is one edit and
  never the work the goal was doing. [until: reviewed 2026-09-18]
- **Deleting a rule strands `rule:` citations in files nothing may rewrite — a frozen record's body,
  a retired goal's `.md` — and `records.py --check` also refuses the dead id in the creating record's
  `changes.creates`.** `rules.py --check` scans `docs/**/*.md` whole, so "frozen history is never
  edited" and "no citation dangles" cannot both hold. Drop the `rule:` prefix in the frozen prose —
  the id still reads as a name — and delete it from `changes.creates`, which is the machine-read
  relation, not the reasoning. [until: reviewed 2026-09-18]
- **A goal's absence gate greps a *path list*, and a path left off it is where the retired spelling
  survives.** Goal `one-type-test`'s gate named `crates`, `tests` and the doc trees but not
  `examples`, so it stayed green while three example programs still spelled the type test PHP's way and
  the floor went red on one of them. Spelling the word inside the bullet is the second way to redden
  such a gate, since the playbook is on its path list. When a gate's claim is "this word appears
  nowhere", run `git
  grep -l -i -w <word>` over the whole tree and compare it against the check's `argv` before treating
  the gate as the specification. [until: reviewed 2026-09-18]
- **A carried floor check's `argv` pins a tool's *flags*, so a kind you retire inside that tool
  cannot take its flag with it.** A floor is carried verbatim from the goal that closed, so
  `owners.py --check --reasons` is still a floor `[[check]]` after `--reasons` stands for nothing,
  and an argparse that dropped it would exit 2 on a stage that passed sessions ago. Leave the flag
  accepted and doing nothing, say that in its `--help`, and fold what it used to add into the gate's
  default. [until: gone tools/owners.py:--reasons]
- **A `[[check]]` that greps a ratchet file for a bare class name matches the header comment that
  explains the key shape, not a key.** `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`
  names `Core\BigInt` twice while stating what a key looks like, so an `exit = "nonzero"` gate over a
  bare `BigInt` goes red the day the last key is struck rather than green. Match a line that is not a
  comment — `git grep -E "^[^#]*<Class>"` — which is why the `Core\Test` gate beside it greps `Test::`.
  [until: gone crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt]
- **A goal's seed handoff opens a session with "nothing of it has landed" over work already on disk.**
  `tools/loop.py:3802` writes `docs/agent/goals/<n>-<slug>.handoff.md` over the live one at the switch,
  and that seed was written before the goal ran — goal `bigint`'s named a first item, `num-bigint` in
  `crates/nvs-stdlib/Cargo.toml`, that the previous goal's sessions had committed a dozen commits back.
  Read `git log --oneline -12` and grep the item's own anchor before taking it: a slug in a subject
  line is the one thing a seed handoff cannot know. [until: reviewed 2026-12-01]
- **`check-links.py` reads a repo-root path in backticks inside a `.py` docstring as a link, so
  deleting a tracked file turns a *tool's prose* red as well as a doc's.** Deleting the index
  `tools/owners.py:102` calls `CARRIED_GAPS` left that tool and `tools/playbook.py` passing every
  other gate while the link check reported `retired  tools/owners.py:102 -> …`, because both module
  docs spelled the full path while explaining why the file is gone. Run `python tools/check-links.py`
  in the same slice as any deletion, and in prose — a bullet here as much as a docstring — that has
  to go on mentioning the path, name the constant that holds it rather than spelling it.
  [until: reviewed 2026-09-18]
- **A CI run can fail with no steps and no logs, and the only place GitHub says why is the check
  run's *annotations*.** `gh run list` reports `"conclusion":"failure"`, every job ends two seconds
  after it started with `steps: 0`, and `gh run view <id> --log-failed` answers `log not found` —
  which reads like a broken `gh` or a stale run rather than what it is, jobs that were never started.
  `gh api repos/<owner>/<repo>/check-runs/<job id>/annotations` prints the sentence itself, the job
  id being the one in that `log not found`, and nothing else in the API carries it.
  [until: reviewed 2026-09-18]
- **A conformance case reported as `did not finish within` the runner's deadline is a cost failure, not
  a wrong answer, and the cases that reach it spawn child processes.** The sweep runs the suite pooled
  beside cargo builds, so every process creation and pipe wakeup becomes a scheduling question, and a
  child dribbling its output a line at a time costs orders of magnitude more to drain than the same
  bytes in bulk. Reproduce under `nproc`-many busy loops before reading the tree as regressed, and see
  `nvs_test::run::CASE_TIMEOUT`'s doc for what the deadline is for. [until: reviewed 2026-09-18]
- **An acceptance check can be red over a *doc* the last session's own wrap wrote, and the fix is a
  sentence in the file you are about to overwrite anyway.** `chain.py --check`, `rules.py --check`
  and `check-links.py` scan the whole tree, `docs/agent/handoff.md` included, so a handoff sentence
  can halt a DONE claim the session that wrote it reported green — the tell is a failure detail
  naming `docs/agent/*.md:NN` rather than a crate path. Fix it in the wrap's own `## handoff` or
  `## playbook:` section, never by hand: step 4 rewrites both files, so a hand edit is overwritten
  by the very call that is supposed to land it. [until: reviewed 2026-09-18]
- **A `peek.py` `re:` pattern must not contain a colon, and PowerShell must not see its `|`
  unquoted.** `peek.py 'file.rs:re:name: "co:6'` reads everything after the last colon as the
  context count and answers `NO SUCH FILE`, and a double-quoted argument holding `(a|b)` is split
  by the shell before Python is started at all. Single-quote the whole target and keep colons out
  of the pattern; a pattern that needs one is a `Grep` call instead. [until: reviewed 2026-09-18]
- **An example that logs writes its own path into its `.out`, so it only matches when the file is
  named the way `dossier.py` names it.** A log record carries the file the program was started
  with, so an absolute Windows path froze an output no Linux run could reproduce — `--bless` and
  the sweep now both name an example repo-relative and posix, from the repository root. Check a
  suspect example with `python tools/dossier.py --run examples --only '<feature>'` rather than with
  a bare `nvs run` from wherever you are standing.
  [until: gone docs/examples/types/Log-Level/01-the-five-levels-and-what-each-is-for.out:source]
- **A `loop-goal.toml` floor check that fails with a `Traceback` is a bug in the tool the check
  runs, not work the tree still owes.** `tools/bench.py --serve-vs-fpm` died on `'NoneType' object
  cannot be interpreted as an integer` because `--reps` grew a `None` default that only `main`
  resolves, while the serve leg went on reading `args.reps` — a sentinel added for one leg that no
  other leg was read against. Reproduce a red check's own argv by hand before budgeting it as a
  slice, and when a flag's default becomes a sentinel, `grep -n 'args\.<name>'` for every leg that
  reads it rather than the one being changed. [until: reviewed 2026-09-19]
- **An `[[app]]` entry in `nvs.toml` naming a file that does not exist aborts every program in the
  tree with `E0605`, not just that one.** A key that matches nothing would silently drop the
  application it was meant to cover back to the global configuration, so the loader refuses the
  whole file — which means writing a proof's grants before writing the proof makes every other
  example and fixture unrunnable in the meantime. Write the `.nvs` files first and the grant block
  second, and when a run fails on a path you have not created yet, that is what happened.
  [until: reviewed 2026-09-19]
- **Editing a reference chapter stales the perf figure of every feature in it.** A language feature's
  implementing file *is* its `docs/reference/lang/*.md` chapter, so `rule:testing/member-perf-ledger`
  re-measures all eighteen of `lang:types` when one paragraph moves, and `dossier.py --verify
  --group` then reports `perf: stale` against features nobody touched. Re-record the group in one
  call: `python tools/dossier.py --record-perf --group lang:types`, straight after any chapter edit
  rather than after reading that failure. [until: gone tools/dossier.py:impl_hash]
- **`python tools/dossier.py --record-perf --force` widens the scope past the `--id` beside it and
  re-measures every bench whose figure is current, appending a row for each.** One call meant to
  refresh a single rewritten bench appended thirty-nine rows to `docs/perf/members.ndjson`, which is
  append-only, so there is no narrower undo than `git checkout -- docs/perf/members.ndjson`. Revert
  the ledger and re-run `--record-perf --id <feature>` without `--force`: the figure is missing
  again after the revert, so it measures without being forced to. [until: reviewed 2026-09-19]
- **A failing acceptance check handed to a retry session can already be green, because its cause was
  a commit that landed inside the sweep's own window from a writer that is not the loop.** Here a
  by-hand rename left the handoff citing a rule id that no longer existed, and the ledger keeps only
  stderr's first line — the `N finding(s):` header, never the finding — while `git status --short`
  is clean by the time the retry session looks. Re-run the check's own argv before reading `loop.py`
  or the goal file, and when it passes compare `git log --date=iso` against the sweep's window in
  `.loop/logs/*-console.log`. [until: reviewed 2026-09-19]
- **A `// bench:` count added after the figure was recorded is invisible until the bench is
  measured again.** The declaration set is stored in the ledger record rather than read from the
  bench file, so `python tools/dossier.py --id <feature>` keeps printing the old `(declares …)`
  list and a declaration that is wrong is never checked against anything. Add the line and re-run
  `python tools/dossier.py --record-perf --only '<feature>' --force` in the same pass.
  [until: reviewed 2026-09-19]
- **`verify.py`'s `nvs-fmt` step rewrites a `.nvs` proof in place, and inside a property hook's
  `set (T $v) { … }` body it de-indents every statement by four.** What it hands back disagrees with
  the hook examples in `docs/reference/lang/50-classes.md`, and the wrap commits the formatter's
  version, so the indentation that lands is not the one you wrote. Write the hook body the way it
  reads best, run `python tools/verify.py` before the wrap so the rewrite happens there, and keep
  what `nvs-fmt` wrote — re-indenting it by hand only dirties the next session's tree.
  [until: reviewed 2026-09-19]
- **`rule:testing/a-failing-proof-is-fixed-or-recorded`'s second answer is not available to a session
  working a goal: `python tools/owners.py --check` refuses a `— owner: <goal-slug>` trailer
  outright.** A recorded gap may name only a milestone at M9 or later whose plan file states the
  scope, because a goal walks and is retired while the gap outlives it. Budget a proof's finding as
  work to *build* in the session that found it, and reach for a module doc `# Known gaps` entry only
  where a milestone will really carry it. [until: reviewed 2026-09-19]
- **An edit to a reference chapter marks every perf record keyed to that file stale, so a bench
  measured before the edit is owed again.** `dossier.py` keys a figure to the text of the file the
  feature is implemented at, and for a language feature that file is the chapter, not a crate. Make
  every chapter edit first and run `--record-perf` last, or pay the release build twice.
  [until: gone tools/dossier.py:stale]
- **A `lang:` feature's perf figure is keyed to the reference chapter the feature is documented in,
  so editing one sentence of that chapter makes every figure in it stale.** `dossier.py --verify`
  then reports `perf: stale: docs/reference/lang/60-iteration.md changed since it was last
  measured` for features the session never opened, which reads like unfinished work rather than a
  re-measure. Run `python tools/dossier.py --record-perf --group <group>` after any edit to a
  reference chapter and before the gate. [until: reviewed 2026-09-19]
- **A `dossier.py --verify` check's failure detail is its *stderr*, which is the release-binary
  notice, while the reason it failed is on stdout.** The acceptance report for goal `lang-iteration`
  read `exit 1 -- dossier: target/release/nvs.exe is missing or older than the tree`, which is a line
  `current_binary` prints before it builds that binary itself; the real reason was the gate line
  saying three features owed every proof. Run the check's own argv and read stdout — `dossier gate:`
  names what is owed — rather than rebuilding anything by hand. [until: reviewed 2026-09-19]
- **A `lang:` feature's perf figure is hashed against its *reference chapter*, so one word changed in
  `docs/reference/lang/*.md` re-stales every figure in that chapter's group.** The repair is one
  `python tools/dossier.py --record-perf --group '<group>'`, which re-measures only what has no current
  figure — but it has to come after the session's **last** chapter edit, or it is paid twice, which is
  what happened here when a proof turned up two stale sentences in the chapter it was proving. Write the
  proofs, fix the prose, then record. [until: reviewed 2026-09-19]
- **The line the driver quotes from a failed `dossier.py` check is often not the failure.** `dossier:
  target/release/nvs.exe is missing or older than the tree` is the routine note it prints on stderr
  *before* asking cargo for a current binary, so it heads the ledger entry while the real verdict sits
  two lines below it and says something else entirely. Re-run the check's own argv and read the last
  lines of its output, never the one line the ledger quotes.
  [until: gone tools/dossier.py:is missing or older than the tree]
- **A `peek.py` `re:` pattern carrying a double quote finds nothing under PowerShell, and the miss
  reads as the symbol being absent.** `powershell.exe` drops the embedded quotes out of a native
  command's argument unless they are backslash-escaped, so `'str.rs:re:name: "contains"'` reaches
  the tool as `name: contains` and reports no match over a file holding that exact line. Escape
  them — `'str.rs:re:name: \"contains\"'` — or search on the unquoted half alone, and never read a
  `re:` miss as evidence until the pattern is one bare word. [until: reviewed 2026-09-20]
- **A comment-only edit to a member's implementing file makes every figure in that file stale, and
  the goal's own acceptance check then goes red on features the session never touched.**
  `rule:testing/member-perf-ledger` re-measures a figure when the implementing file's text moves
  with its `mod tests` cut off, so one `# Known gaps` paragraph added to `arr.rs` put nine green
  `Core\Arr` members back in the `perf` column of `python tools/dossier.py --group 'Core\Arr'
  --owed`. Close them with one `--record-perf` over the whole group after `verify.py` is green,
  rather than reading that column as new work. [until: reviewed 2026-09-20]
- **A `Core\Arr` slice's Rust test stales every other member's perf row in the same file, and they
  then read as unwritten work.** `rule:testing/member-perf-ledger` makes a row current against the
  *implementing file's* text rather than the member's, so one `#[test]` added to `arr.rs` put all
  ten finished members back on the owed list with their benches sitting on disk. Record the whole
  group once, after the session's last edit to that file — `python tools/dossier.py --record-perf
  --group 'Core\Arr'` re-measures exactly the stale rows and leaves the current ones alone.
  [until: reviewed 2026-09-20]
- **`--dry-run` does not reach `dossier.py --record-perf`, so a measurement meant as a trial appends
  to the ledger anyway.** The flag belongs to `--emit-goals`, and `--record-perf --only
  'Core\Arr::contains' --dry-run` ended with `4 records appended to docs/perf/members.ndjson`. Read
  the run's last line rather than the flag, and measure only once the session's last edit to the
  implementing file has landed — `git diff --stat docs/perf/members.ndjson` says what really went in.
  [until: reviewed 2026-09-20]
- **A generated dossier goal can already be satisfied when its first session opens, while its item
  list still says every feature "owes examples, hostile, perf, tests".** That list is a snapshot
  `dossier.py --emit-goals` took, so a neighbour that finished a whole class leaves this one green
  and its `file:NN` anchors stale: `core-arr-2-4` opened complete, its `Core\Arr::find` anchor
  naming `map`. Run `python tools/dossier.py --id '<the first item>'` before writing a line; on
  `complete.`, run the goal's own `[[check]]` argv from `docs/agent/loop-goal.toml` and go to the
  DONE gates. [until: reviewed 2026-09-20]
- **A `dossier` hostile check that is red for the driver and green every time you run it by hand is
  a clock, not a weak proof.** `types:exception`'s two `Core\Db` attacks run in 1.3s and 3.4s
  standalone and still hit their 60s limit under the acceptance sweep, on the same release binary
  and an idle machine whose neighbouring checks took 2-5s. Read the FAIL line's `having printed N
  line(s)` clause in `.loop/logs/<run>-console.log` before touching the attack: a program hung on
  one step and one running every step too slowly have different repairs, and raising the timeout is
  weakening the proof either way. [until: reviewed 2026-09-27]
- **A generated dossier goal can arrive already satisfied, and its item anchors can name a different
  member.** `docs/agent/goals/dossier/*.md` is written once by `--emit-goals`, so an earlier goal's
  `--partition --group` fan-out covers the whole class rather than its own `--only` list, and goal
  `core-arr-4-4`'s anchor `crates/nvs-stdlib/src/arr.rs:533` named `column` while its item was
  `range`. Run the goal's own `dossier.py --verify --only ...` line before reading any anchor: when
  it prints `nothing owed` and two `0 failed` lines the slices are already on disk, and the session's
  work is whatever that check does not count — the descriptions first.
  [until: reviewed 2026-09-20]
- **A `dossier.py --bless` or `--record-perf` rebuilds `target/release/nvs.exe` whenever any Rust in
  the tree has moved, and that build is minutes, not seconds.** A slice that writes its examples,
  blesses them, then edits a crate for the Rust-side test and blesses the next slice pays that build
  twice, and a `| tail` on the call hides every line of it so the second one reads as a hang. Make
  every Rust edit of the group first, then bless and record once for the whole group, and never pipe
  a `dossier.py` call. [until: reviewed 2026-09-20]
- **`python tools/dossier.py --bless` runs `target/release/nvs.exe` and rebuilds it whenever the
  tree has moved, so a bless that follows a Rust edit pays a whole release build.** Its staleness
  check is the tree rather than the crate the example touches, so blessing one member's examples
  after splicing that member's Rust test in costs the build again for the next member. Write every
  example, attack and bench of a group first and bless them in one call, then splice the Rust tests
  in afterwards. [until: reviewed 2026-09-20]

- **`python tools/verify.py` runs `cargo fmt`, which rewrites the very text a perf record is keyed
  on, so a `--record-perf` taken before the wrap is stale by the time the gate reads it.** A
  member's figure is accepted while a hash of its implementing file matches, and formatting one line
  a session spliced in moves that hash for every member in the file. Measure the group after
  `verify.py` has come back green, not while it is still running. [until: reviewed 2026-09-20]
- **`dossier.py --record-perf --force` ignores `--id` and re-measures the whole roster.** A
  `--force --id 'Core\Bytes::compare'` appended 205 rows to `docs/perf/members.ndjson` and turned up
  two unrelated benches that now miss their own declarations, none of which was asked for. `--id`
  scopes the audit only; `--only <id> [<id> ...]` is what scopes a recording, and it is the flag to
  reach for when a bench's declaration changed while its implementing file did not, since the
  `impl_hash` currency rule otherwise reports the figure as already current.
  [until: reviewed 2026-09-20]

- **A bench that indexes an array with a `uint` expression records two allocations per op and misses
  an `allocations 0` declaration.** `crates/nvs-ir/src/lib.rs`'s `# Known gaps` item 21 owns it: a
  `uint` subscript is lowered to a fresh decimal string on every access, where an `int` one takes the
  runtime's integer-key entry point. Write the index as `$keys[($total % 3) as int]`, which is what
  `benches/members/core/Str/length.nvs` already does, rather than reading the miss as the member
  under test allocating.
  [until: gone crates/nvs-ir/src/lib.rs:array subscript is rendered to a decimal string]
- **A `covers:` marker added to a Rust test re-stales every perf figure measured against that
  file.** `dossier.py` keys a member's figure to the implementing file's text, so a one-line comment
  in `cache.rs` put `Core\Cache::local`'s just-recorded figure back into `OWED` the moment the next
  slice marked its own test. Do every Rust edit of a group first, then `--record-perf --only …`
  once at the end. [until: reviewed 2026-09-21]
- **`python tools/peek.py` takes its options before its targets, and a flag written between two
  targets makes it refuse every target after it.** `peek.py A.rs:@sym --context 12 B.rs:@sym
  C.rs:230-248` answers `unrecognized arguments` and lists the trailing two, which reads as a
  misspelled target rather than a misplaced flag. Put `--context` and `--window` first —
  `peek.py --context 14 A B C` — and read that refusal's list as where the flag sits.
  [until: reviewed 2026-09-21]
- **`python tools/dossier.py --record-perf --group 'Core\Cli'` does not reach `Core\Cli\Color` or
  `Core\Cli\Live`.** A group is matched as the class name itself, not as a prefix, so a goal whose
  features live in nested classes leaves those benches reading `perf stale` after what looks like a
  whole-group run. Record each class with its own `--group`, and read the `N features` line the run
  prints against the number of features the goal owes. [until: reviewed 2026-09-21]
- **`dossier.py --record-perf --force` ignores `--id` and re-measures the whole tree.** A
  `--record-perf --id '<feature>' --force`, reached for because the unforced run says the figure is
  already current, walked every bench and appended 249 rows to `docs/perf/members.ndjson`. Scope a
  forced re-measure with `--group '<group>'`, which `--force` does honour, and `git checkout --
  docs/perf/members.ndjson` puts the ledger back before the scoped run.
  [until: reviewed 2026-09-21]

## Running things

- **One case, quickly:** `nvs test tests/conformance/core/str-case-members.nvst`, or `nvs test
  tests/ --filter str-` over the tree. [until: reviewed 2026-09-06]
- **A scratch `.nvs` under `.agent-tmp/` run with `nvs run` is the fastest way to find out whether a
  shape lowers**, and the cheap half of an unreachability proof: the smallest program that would
  take a checker arm either runs or stops at `nvs-ir`'s known-gap panic naming what it will not
  lower. A scratch file is top-level statements like `examples/*.nvs` — no `Main::main` entry point,
  and a `for` header takes expressions only, so the loop variable is declared on the line above it.
  [until: reviewed 2026-09-06]
- **`nvs run` printing the right output and exiting **127** is a heap corruption at teardown**, not
  a missing command: Windows reports a double release that way, with nothing on stderr, and a
  refcount bug is otherwise silent until the WSL valgrind leg catches it. Check `$?` on every
  scratch run rather than reading the output and moving on. [until: reviewed 2026-09-06]
- **A leak whose "definitely lost" size is `16 + strlen(a literal in the probe)` is a temporary
  abandoned on a throwing edge**, and the allocating stack carrying no `nvs_` frame at all is the
  confirmation. Most such edges are closed by `nvs_ir::lower::Lowering`'s owned-temporaries stack,
  so a probe that `catch`es a throw from a `Core` member taking a `string` is a fair leak check.
  What is still open is named in that field's own doc comment, plus the producers that release
  inline — a normalized subscript key, a `match` subject. [until: reviewed 2026-09-06]
- **A field or element read off a *temporary* is a fresh producer, not an aliasing read.**
  `lower_property_access`/`lower_index` retain what they read and release the base, and
  `Lowering::aliasing_read` recurses into the base and answers `false` for these shapes. A consumer
  must not retain such a read a second time: every retain decision in `nvs-ir` goes through
  `aliasing_read`, and a new one that reaches for the syntactic `is_aliasing_read` instead is how
  the double-retain gets back in. [until: reviewed 2026-09-06]
- **A before/after measurement is worth a `git stash`, and the base half is what makes it an A/B
  rather than two readings** — stash, `cargo build --release -p nvs-cli`, `bench.py <cases> --reps
  9`, pop, rebuild. The base run should reproduce the ledger's own sweep row for row; without it a
  small move is indistinguishable from a quiet machine. Budget two release rebuilds and the harness
  re-printing every stashed file it has seen, twice; a `git worktree` avoids the re-print and pays a
  full cold build, which is worse. [until: reviewed 2026-09-06]
- **`bench.py` says when the machine was busy, and it means it.** A sweep whose min and median
  differ by more than a quarter on `00-baseline` prints a note, and since the baseline is subtracted
  from every row, that run's ratios are all shifted. Re-run before quoting, and do not reason about
  a small row move from a sweep carrying that note. [until: reviewed 2026-09-06]
- **A `#[global_allocator]` in a test target must be `#[cfg(debug_assertions)]`.** `nvs-runtime`
  installs its pooled allocator under `all(not(test), not(debug_assertions))`, so a second one in a
  `nvs-stdlib` test binary links under `cargo test` and fails under `cargo test --release` with
  *"cannot define multiple global allocators"*; `verify.py` runs the debug profile and never sees
  it. `crates/nvs-stdlib/tests/allocation_policy.rs` is the worked example.
  [until: reviewed 2026-09-06]
- **A release build relinking the runtime moves a bench row by several percent, with no code
  change.** Two builds whose member was byte-identical read differently, and rows nothing touched
  moved with them. An A/B on a single row is only worth reading when the delta is well past that
  noise; quote the median, and re-run the base binary once before believing a small regression.
  [until: reviewed 2026-09-06]
- **A `Ctx` that outlives the `Unit` whose code it ran reads freed class descriptors, and the crash
  lands nowhere near the cause.** Compiled code bakes each `ClassDesc`'s address in, so an exception
  left on the context points into the `Rc<ClassTable>` the `Unit` owns; drop the unit first and
  `ctx.pending()` reads freed memory — a wrong message, or a *misaligned pointer dereference* inside
  `nvs_runtime::object::drop_one`, intermittently. `nvs_codegen::Unit::install_in` is the one
  spelling: call it before running any of a unit's code, whether or not you care about `catch`.
  [until: reviewed 2026-09-06]
- **`target/release/nvs.exe` is whatever the *last* session built, and rebuilding it costs two
  minutes for a verdict the debug binary already gives.** A `.nvst` case a stale binary fails may
  simply predate it, and `cargo build --release -p nvs-cli` relinks the world for a one-line edit.
  Build `cargo build` and run `target/debug/nvs.exe`: it is the binary `tools/loop.py` builds after
  every acceptance check, so it is the cheap answer and usually the faithful one. The exception is
  `tools/dossier.py`, which runs every proof program against release: `--bless` prints
  `target/release/nvs.exe is missing or older than the tree -- cargo build --release -p nvs-cli`
  and then runs that build itself (`tools/dossier.py:513`), so the line is the start of a
  six-minute wait rather than a refusal, and the driver's acceptance sweep reports the same line.
  Make every `crates/` edit of the group before the first `--bless` or `--record-perf`, because
  each edit buys another relink.
  [until: reviewed 2026-09-20]
- **`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh …` fails from the Bash tool and works
  from PowerShell.** Git Bash rewrites any argument that looks like a POSIX path before `wsl.exe`
  sees it, so the command arrives as `bash: C:/Program Files/Git/mnt/…: No such file or directory` —
  and still exits 0, which reads as a clean leak check on a fixture that never ran. Run the
  identical line through the PowerShell tool, or put `MSYS_NO_PATHCONV=1` in front of it; any
  absolute POSIX path handed to a Windows `.exe` through Git Bash is a candidate.
  [until: reviewed 2026-09-06]
- **A `holes.py` site guarded by a predicate over the same list it matches is an
  internal-consistency check, and the reachable holes are its *neighbours*.** A `panic!` arm below a
  predicate that already excluded every other atom owes `collect::<Option<_>>`, not a diagnostic,
  while each of the item's spellings hits a different site one call apart. Run the item's spellings
  as scratch files under `.agent-tmp/` before designing anything — the panic a worklist item names
  is often not the one that fires. [until: reviewed 2026-09-06]
- **A spec §§ 1-12 member owes a *fifth* thing: striking its line from
  `crates/nvs-stdlib/tests/spec-members-outstanding.txt`.** That file is the ratchet
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs` reads, and it fails on a *stale* line naming a
  member that is registered now exactly as loudly as on an unregistered member it does not list — so
  the failure after landing a member is the list saying it did not shrink. Keys are `§<section> <the
  spec's own Member-cell spelling>`, so `§1 chunk` and `§2 chunk` are two lines.
  [until: gone crates/nvs-stdlib/tests/spec-members-outstanding.txt]
- **A `Core` instance's slots hold only values Novis already holds, so a member wanting native
  mutable state has to accumulate instead** — there is no destructor to free a `sha2::Sha256`
  context with. The COW-correct read/write of a slot holding an array is
  `identity_store::borrow`/`edit`/`replace`, generic over `(receiver, index, class, member)` despite
  the module's name; `instance::set_slot` is the raw write and `instance::slot` the borrowed read.
  `Core\Hash\Stream` is the worked example. [until: reviewed 2026-09-06]
- **A new domain module is `mod`, not `pub mod`, so its `CLASS`/`NAME` are `pub(crate)`.** The
  workspace warns `unreachable_pub`, and half of `nvs-stdlib`'s modules are `pub mod` while the
  newer half is not, so copying `uuid.rs`'s `pub const NAME` into a privately-declared module is a
  warning at build time. `objmap.rs`'s `NAME` is the shape to copy. [until: reviewed 2026-09-06]
- **A `Core` symbol that is not a member breaks
  `every_registered_member_has_an_implementation_address`.** `nvs_stdlib::symbols()` used to be
  exactly one entry per `CLASSES` member and that test asserts the count, so a constructor symbol
  from `registry::CONSTRUCTORS`, or anything else chained in beside the members, has to be added to
  the test's right-hand sum in the same edit. The failure is a bare `left: N, right: M` in `-p
  nvs-stdlib --lib`, naming no symbol. [until: reviewed 2026-09-06]
- **A `CoreTy::Array(&CoreTy::Uint)` parameter receives `Tag::Int` elements**, so a helper that
  reads each one through `as_uint` alone answers the member's most obvious call site with a fatal. A
  written `[97, 98]` type-checks against `array<uint>` and stays int-tagged into the helper; a
  scalar `uint` parameter has no such problem because the call site materializes the literal at the
  declared type. Read both tags (`str.rs`'s `code_point`), and probe the literal spelling in a
  scratch `.nvs` before writing the case. [until: reviewed 2026-09-06]
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is `args:
  [3]`. A variadic tail is one argument, whatever the call writes, and an instance member's receiver
  is argument slot 0 and is not in `params`, so `plus(Duration)` is `args: [2]`. A mismatch is an
  index-out-of-bounds panic at the first call. [until: reviewed 2026-09-06]
- **A member on `registry::WRITTEN_CLASS_MEMBERS` takes one argument its row does not declare** —
  the class its call site wrote, in slot 0 — so its helper's `args: [N]` is `params` + 1, plus the
  options bag's flattening. `crates/nvs-stdlib/tests/conformance_coverage.rs` looks for such a
  member spelled `Class::name<`, not `Class::name(`, because that is what every call site writes.
  [until: reviewed 2026-09-06]
- **Registering a `Core` class narrows `Core`'s blanket trust for that name.** An unregistered
  `Core\X::y()` is waved through by `nvs_hir::members`; once `X` is in `registry::CLASSES`, an
  unknown member on it is a diagnostic. Adding a class can turn a fixture that "compiled" into one
  that reports — which is the point, but check the fixtures that name it.
  [until: reviewed 2026-09-06]
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying
  why that crate, `cargo deny check`, and `python tools/gen-attribution.py`
  (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`). A license identifier
  new to the tree goes into both `deny.toml`'s allow list and `tools/gen-attribution.py`'s
  `PREFERENCE` in the same commit, because that script fails if the two disagree. The dependency
  sweep is a pass the user fires by hand
  (`rule:packaging/a-dependency-break-is-absorbed-never-forwarded`); never start it as a side
  effect. [until: reviewed 2026-09-06]
- **A registry rule quoted by test name may not be that rule, or may not exist.** A handoff can cite
  `a_union_is_only_ever_a_parameter` for "a union cannot be a return type" when the real test is
  `a_union_option_excludes_null`, which restricts an option's type and nothing else, and
  `CoreTy::Union`'s doc says "legal in either direction" outright. One `grep -n 'fn [a-z_]*('
  registry.rs` over the test names settles it; designing around a constraint that is not there costs
  a member's whole surface. [until: reviewed 2026-09-06]
- **Adding a row to either compiler-owned roster — `nvs_hir::errors::TREE` or
  `nvs_hir::interfaces::RESERVED` — fails a test in `nvs-ir`, and the message names neither the roster
  nor the name you added.** Both are restated as one hard-coded, alphabetically sorted label list in
  `lower/tests.rs`'s `a_file_with_no_class_still_carries_every_compiler_declared_class`, so the failure
  is a bare `left: [...]`/`right: [...]` diff in `-p nvs-ir --lib`. A new § 10 class owes the `TREE`
  row, `nvs_runtime::ThrownClass`'s variant, its `name()` arm, its `ALL` entry, that assertion, and the
  spec's own tree drawing; a new global interface owes the `RESERVED` row and that same assertion.
  [until: reviewed 2026-09-08]
- **`Value::as_str_bytes` and `Value::as_text` make the *same* tag check, so a `from_utf8` after the
  first can never catch anything.** Both go through `Value::str_ptr`, which answers for `Tag::Str`
  alone, so a `bytes` argument reaches neither, and a defence a later rule made unreachable reads
  exactly like a live one. Grep for the tag a defence claims to catch before trusting it;
  `allocation_policy.rs`'s `no_member_revalidates_a_string_argument` is the source scan that keeps
  the pair out now. [until: reviewed 2026-09-06]
- **A roster probe binds its subject; it never `echo`s it, and it gets one panic per run.** An `echo
  $subject` line goes through `concat_operand` first, so the probe reports that function's missing
  row rather than the operator's; `mixed $x = $a & $a;` asks the question meant. The checker reports
  every diagnostic in a file at once while lowering panics on the first shape that gets that far, so
  put the shapes you expect refused in one file and the ones you expect to lower in another.
  [until: reviewed 2026-09-06]
- **`return $local;` retains nothing — it hands the binding's own reference out and tells
  `release_all_locals` to skip that name.** Any binding `release_all_locals` was never going to
  release silently loses the retain: an `inout` parameter is a `Ty::Ref` cell, not refcounted, so
  returning it hands the caller a value with no owner, and `nvs run` prints the right answer and
  exits 127. A `return`/`release_all_locals` exemption keyed on a name has to be re-read whenever a
  new binding representation enters `Env`; an exit 127 is worth `git stash`-ing before you assume it
  is yours. [until: reviewed 2026-09-06]
- **A `static` method's slot 0 is the *called class*, not an empty receiver.** Calling one from Rust
  (`nvs_runtime::abi::call` on a `Unit::function("Class::method")` address) with a `null` first slot
  segfaults inside the callee, nowhere a message could print:
  `rule:statements/static-is-a-member-modifier`'s late static binding puts a `ClassDesc` there,
  `nvs_ir::lower` seeds it as `Param(0)` at `Ty::ClassDesc`, and `Value::class_desc` is the
  encoding. An instance method's slot 0 is the receiver, so the trap shows only the first time
  native code calls a `static` one. [until: reviewed 2026-09-06]
- **A scratch `.nvs` file needs its `<?nvs` opener, and without one it "runs" and exits 0.**
  Everything before the tag is inline HTML, which lowers to an `echo` of the raw span, so the file
  prints its own source back and reports success — or, when that span is the whole file, dies in
  `nvs-ir`'s control-flow slice listing every statement it does lower, which reads as "`echo` is
  unsupported". Check that the output is the program's answer and not the program, or copy the first
  line from `examples/targets.nvs`; the `.nvst` harness supplies the tag inside `--FILE--`.
  [until: reviewed 2026-09-06]
- **A `FATAL` cannot be triggered mid-run by the safepoint, because the flag can only be set before
  the run and the script frame's own entry poll fires first** — which is why
  `a_fatal_is_never_caught` sees empty output. The stack limit is no better: `arm_stack_limit`'s
  soft tier answers a catchable `Recursion` long before the floor. What is reachable from source
  after locals are live is a `Fault::fatal` from `nvs-stdlib` — `Core\Arr::countBy` over an
  `array<float>` is one — so reach for that when a test needs a fatal at a chosen point.
  [until: reviewed 2026-09-06]
- **Windows compiles none of a crate's `#[cfg(unix)]` half, so `verify.py` on this host is silent
  about it** — a Unix-only type can be green here and not compile at all. The check is one call,
  `wsl.exe -- bash -lc 'cd /mnt/<drive>/<repo> && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo
  test -p <crate>'`, and a second for `cargo clippy -p <crate> --all-targets` because the lints are
  just as unrun. `mio::net` re-exports no address type at all, so the symmetric-looking
  `mio::net::SocketAddr` is `E0425` — invisible to every Windows leg. [until: reviewed 2026-09-06]
- **A thread-local whose `Drop` joins threads deadlocks on Windows, and the symptom is a test that
  runs its whole body and then never reports.** A pool reached through a `thread_local!` drops from
  a TLS destructor, which Windows runs under the loader lock — the lock the joined thread needs to
  exit — so `cargo test` sits until killed, and leftover `nvs_host-*.exe` processes say the binary
  hung rather than the build. Read the last line the test body printed before suspecting the code
  under test; detach the handles and let the threads see a shutdown flag.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` acceptance check reports the *first* diagnostic, not the tree's whole
  distance from passing.** A check that has said `E0703 — 'spawn script' is not compiled yet` for
  sessions reads as one construct away when `nvs check` on the same file reports two more, one of
  them a construct nowhere in `nvs-syntax`'s AST. One `./target/debug/nvs.exe check <file>` of a
  failing `exact` check's own fixture, before planning the group that closes it, is the difference
  between a group and a milestone. [until: reviewed 2026-09-06]
- **A valgrind sweep that goes red on *every* fixture at once is one allocation on the startup path,
  and the stack names it in one call.** `valgrind --leak-check=full -q <binary> run
  examples/hello.nvs` prints the allocating frame at the top of the stack. The trap is the shrug: a
  deliberate, documented, once-per-process leak reads like something to accept, but the sweep is
  all-or-nothing and a gate with one known-red fixture is a gate nobody reads — a `Box::leak` that
  only widens a borrow to `&'static` has a scoped form, `nvs_runtime::script::scoped`, so reach for
  that before a suppression. [until: reviewed 2026-09-06]
- **`Wake::current()` decides which of two isolate boundaries you just measured**, and a `#[test]`
  or a criterion `b.iter` has no scheduler under it, so it takes the inline one:
  `nvs_host::Isolate::run` outside a task runs the child on the caller's stack and reads several
  times faster than the real path with a stack of its own, and only the second is the boundary a
  program crosses. `benches/abi-probe/shared/isolate.rs` is the shape: build the scheduler and task
  outside the clock, time inside the task body, and hand criterion a batch through `iter_custom`.
  [until: reviewed 2026-09-06]
- **A refcount cycle that is still live at exit is a `definitely lost` under valgrind and always
  will be — read the fixture before you read the runtime.** Novis refcounts and has no cycle
  collector (`crates/nvs-runtime/src/object.rs` says so), so a self-referential object's last
  reference is its own field and dropping the local frees nothing; `examples/serialize.nvs` breaks
  its rings by hand before it ends. A leak stack whose top frame is an object allocation
  (`nvs_object_new`) is the shape to suspect — grep the `.nvs` for a cycle before opening the Rust.
  [until: reviewed 2026-09-06]
- **A checkout on a non-system Windows drive grants `Authenticated Users` modify, so this
  repository's own `nvs.toml` fails `rule:config/ownership-is-the-trust-boundary`.** Such a drive's
  root carries that ACE by default and everything under it inherits it, so a run reading the tree
  through `Files::trust` stops with `E0607`. The fix is on the machine and needs the user's say-so —
  `icacls <path> /inheritance:d` then `icacls <path> /remove:g "<the account>"` for `nvs.toml` and
  the checkout root, account names being localized; a scratch tree under `%TEMP%` passes as it is.
  [until: reviewed 2026-09-06]
- **`tools/bench.py --warm-start` measures `target/release/nvs.exe`, and nothing builds it.** On a
  machine that has only ever built debug the check fails with `no Novis binary at …` rather than
  with a number; `cargo build --release -p nvs-cli` once is the fix, and `bench.py`'s header says
  why the harness refuses to build anything itself. A release binary older than `crates/` still
  measures — the staleness warning goes to stderr and the check stays green — so a start-up
  regression can hide behind a binary nobody rebuilt. [until: reviewed 2026-09-06]
- **`verify.py` in an interactive session fails at build with `failed to remove file … nvs.exe` (os
  error 5) while the unattended loop is mid-session.** The loop's own test run holds the binary and
  cargo cannot replace a running exe on Windows — contention, not a broken tree. Check first
  (`Get-CimInstance Win32_Process` shows `loop.py` and a `target\debug\nvs.exe`), commit finished
  slices before verifying so the loop's wrap cannot sweep them, and retry when the binary frees;
  never kill the loop's `nvs.exe` to win the race. [until: reviewed 2026-09-06]
- **`tools/try.py` cannot run a case that carries a second `--FILE <name>--` section, and what it
  prints reads as if the case itself were broken.** It hands everything after `--FILE--` to the
  compiler, so a `--FILE nvs.toml--` capability grant arrives as Novis source and answers with parse
  errors on `[capabilities.net]`. `target/debug/nvs.exe test <case.nvst>` runs one case the way the
  suite does and honours the extra sections; for a single-file case `try.py` is still the cheap way
  to capture an `--EXPECTF-ERROR--` block — paste only the `error[...]` line and the `-->` line
  under it. [until: reviewed 2026-09-06]
- **A new `Core` member trips two gates the recipe does not name, one of them in another crate.**
  `registry::tests::no_registry_card_cites_an_adr`: `nvs meta --json` ships a card verbatim, so a
  `short`, `ret` or `desc` may not cite a rule — leave the citation to the doc comment above the
  row. A row answering `CoreTy::TaintedStr` fails `nvs_types::core_lib`'s
  `a_verified_signature_does_not_launder_its_claims`, the closed set of members whose answer is
  qualified `tainted`; widening it is the point, but the edit is in `crates/nvs-types`, which `-p
  nvs-stdlib` never compiles. [until: reviewed 2026-09-06]
- **`tools/origin.py` exits the moment its stdin closes, so a backgrounded launch is already gone
  before you run the example — and the failure reads exactly like never having started one.** Stdin
  EOF is the shutdown protocol, so a shell that backgrounds the command hands it EOF at once: it
  binds, prints `origin: serving …`, exits 0, and `examples/http.nvs` fails with the same
  `connecting to 127.0.0.1:8099 failed` line the no-origin case gives. Hold the pipe open — `tail -f
  /dev/null | python -u tools/origin.py` — or run it in the foreground of its own call.
  [until: reviewed 2026-09-06]
- **An isolate reads the configuration in force where it was *spawned*, so an `[[app]]` block keyed
  on the child's own path never reaches it.** `Ctx::isolate` clones the parent's `Request`, and the
  snapshot was resolved once for the entry file, so the block that reaches a child is the one the
  parent matched. A block's sub-tables need no plumbing: `crate::snapshot`'s per-app fold merges
  every other key of the block onto the global table, so grep for the field on
  `nvs_config::tree::App` before assuming a block cannot carry a table — `deny_unknown_fields` is
  all that refuses it. [until: reviewed 2026-09-06]
- **`nvs_helper!` takes exactly one function per invocation, and a second one inside the block fails
  naming the *next* member's doc comment rather than yours.** An added member written above an
  existing one produces ``error: no rules expected `#` `` pointing at that member's `///` line, with
  nothing pointing at what you wrote; the macro's rule in `crates/nvs-runtime/src/abi.rs` is a
  single `fn`, no repetition. Close your block and open a new `nvs_runtime::nvs_helper! { … }`,
  which is why `cli.rs` has several invocations rather than one. [until: reviewed 2026-09-06]
- **A lowering arm that hands back its own operand owes a retain, and skipping it is a heap
  corruption that passes `nvs.exe run` and fails only under the conformance runner.** The exit
  status is `-1073740940` (`0xC0000374`, `STATUS_HEAP_CORRUPTION`) with no message: a double release
  only trips the allocator once the process does enough afterwards. `Lowering::convert`'s free `from
  == to` row carries the rule, so a new arm returning its operand repeats its
  `aliasing_read`/`emit_retain` pair; do not copy `lower_class_reference`, which owes none because a
  descriptor is immortal. [until: reviewed 2026-09-06]
- **A stale `target/debug/nvs.exe` answers `E0405: Core\Foo has no member named bar` for a member
  you just registered, which reads as a registration bug rather than as a stale binary.** The class
  name resolves without the registry (`QName::is_reserved_global_class`), so the sentence is the one
  a missing `address()` arm or `CLASSES` row would produce. The binary's package is `nvs-cli`, not
  `nvs`: `cargo build -p nvs` fails with "package ID specification `nvs` did not match any packages"
  and leaves the old executable in place, so refresh with `cargo build --bin nvs` or plain `cargo
  build`. [until: reviewed 2026-09-06]
- **`cargo deny check`'s `advisories` fails on a lockfile entry that is not yours.** The verdict is
  four lines — `advisories FAILED, bans ok, licenses ok, sources ok` — and the red one is `detected
  yanked crate (try 'cargo update -p chacha20')`, which arrives through `rand` and predates any
  dependency slice. A dependency slice owns `licenses`, `bans` and `sources`; read those three and
  say in the handoff that the fourth was already red, rather than fixing an unrelated lockfile entry
  or reporting your own change as the failure. [until: gone Cargo.lock:name = "chacha20"]
- **A `.nvst` case whose program branches on the platform is only half-run by
  `target/debug/nvs.exe`.** `/var/tmp/nvs-linux/debug/nvs` is a Linux binary the valgrind leg keeps
  current, so `wsl.exe -- bash -lc "cd /mnt/<drive>/<repo> && /var/tmp/nvs-linux/debug/nvs test
  <case>.nvst"` runs the case on Linux with no build. Check its date first: it is as old as the last
  valgrind sweep and has none of this session's Rust. [until: reviewed 2026-09-06]
- **A test `verify.py` reports as "failed beside the other test binaries and passed alone" is red,
  and running the gate again is not the fix.** The tool has already re-run that binary by itself;
  what is left is a test leaning on something the load beside it takes away — a wall-clock spin read
  as CPU time, a channel checked after its sender's thread may have exited, a cost margin that holds
  only on an idle box. Give the test what it depends on (`nvs_host::cpuclock::burn_at_least`, an
  assertion that tolerates a dropped sender, `#[cfg_attr(debug_assertions, ignore)]` into the
  driver's release slot) in a commit of its own. [until: gone tools/verify.py:alone]
- **Write a goal's acceptance fixture red, but never write its `nvs.toml` block red.** A `.nvs`
  fixture naming a missing member fails at `E0405` and costs only itself, but `deny_unknown_fields`
  sits on every struct in `crates/nvs-config/src/tree.rs`, so an unrecognised table fails at boot
  for every program in the repository. The config block belongs to the slice that adds the struct
  that reads it. [until: gone crates/nvs-config/src/tree.rs:deny_unknown_fields]
- **A `Core\Db` fixture's preconditions live in three files nobody edits together, and none is
  visible from the crate you just landed.** `db.connect ... is not granted` means `nvs.toml` is
  written by hand and nothing derives it from the fixture; behind that, `nvs_host::tls::anchors()`
  is a compiled-in root store, `config_over` in `tls.rs` is the seam a configured bundle would plug
  into, and `tests/db/compose.yaml` owns the certificates. Run the fixture once with the built
  `target/debug/nvs.exe` rather than reasoning about any of them. [until: reviewed 2026-09-06]
- **`docker` under Git Bash needs `MSYS_NO_PATHCONV=1`, and the error names a path you never
  wrote.** MSYS rewrites every argument starting with `/` — the container's `/bin/sh`, a `cp` of
  `/certs/ca.crt` — into a Windows path before `docker` sees it, so the failure is `stat C:/Program
  Files/Git/usr/bin/sh: no such file or directory`. Prefix the command; a relative `-f
  tests/db/compose.yaml` is unaffected. [until: reviewed 2026-09-06]
- **A `tests/db/compose.yaml` comment is a claim about the image, not about the service as
  configured.** MariaDB 11.4 does not turn TLS on by itself, MySQL 8.4 generates a CA at
  `/var/lib/mysql/ca.pem`, and SQL Server keeps its certificate inside the instance rather than on
  the filesystem. Ask the running container what it serves — `docker compose exec <svc>` — before
  writing anything that verifies a certificate against it. [until: reviewed 2026-09-06]
- **`[queue]` is a *root* table, so `workers` starts a worker for every `nvs run` over that tree,
  not just the queue fixture.** There is no per-`[[app]]` spelling:
  `rule:core-classes/queue-storage-is-a-table` makes `workers` a property of the instance, so a
  worker waiting on an unreachable server charges every unrelated fixture. Grep for a root table's
  `[[app]]` twin before assuming a block only reaches the program it was written for.
  [until: reviewed 2026-09-06]
- **`default-features = false` can drop a backend the defaults were choosing, and the failure is a
  `compile_error!` in a crate you never named.** `mysql_common`'s defaults pull `flate2/zlib`, which
  puts C in the graph, and turning them off leaves `flate2` with no compression backend at all. The
  fix is a direct workspace entry for `flate2` with `features = ["rust_backend"]`; read a vendored
  crate's `[features] default` before assuming `default-features = false` is only a slimming.
  [until: reviewed 2026-09-06]
- **`failed to load manifest for workspace member` names a directory nobody put a crate in, and it
  fails the whole workspace.** The root `Cargo.toml` has `members = ["crates/*", "benches/*"]`,
  expanded before any crate is read, so a `.nvs` or data tree added under `benches/` breaks every
  build, test and clippy run until it is named in `exclude`. The tell is that nothing you touched is
  in the message. [until: gone Cargo.toml:benches/*]
- **`python tools/db-matrix.py` reports `ok` for a case that never ran, so a green matrix proves
  nothing about a case you just wrote.** Every case returns early when its gate answers `None` —
  `NVS_DB_MATRIX_DRIVER` unset, a gate misspelled, `framed()` where `postgres()` was meant — and the
  tool runs `cargo test -q`, so a skip and a real assertion print the same `ok`, and wall clock says
  nothing either. Confirm once by breaking the case's own assertion and re-running the tool: a leg
  naming your case in its `FAILED` line ran it, then revert. [until: reviewed 2026-09-06]
- **Running one matrix case by hand with `NVS_DB_MATRIX_*` has two traps, both a failing
  assertion.** `cargo test` runs in the crate directory, so a relative `NVS_DB_MATRIX_CA` resolves
  under `crates/nvs-db/` and fails `NotFound`; and the password's one home is
  `tests/db/compose.yaml`, so `matrix.rs` has no default. Give the anchor absolute (`docker compose
  -f tests/db/compose.yaml cp <svc>:/certs/ca.crt <dir>` exports it), or run `python
  tools/db-matrix.py --driver <name> --no-up`. [until: reviewed 2026-09-06]
- **A red valgrind sweep is `ring`'s assembly or the fixture's exit status before it is your
  refcounts.** Memcheck cannot follow AES-NI masking, so a TLS `[db.main]` fixture reports
  `Memcheck:Cond` in `ring::aead` (`tools/valgrind.supp` holds those), and valgrind passes the exit
  status through, so `examples/limits.nvs` (exit 1 by design) reads as a leak without
  `--error-exitcode=97`. `grep -c "definitely lost"` the stderr first: `0` means neither cause is
  yours. [until: reviewed 2026-09-06]
- **A valgrind stack whose allocation site is `NvsArray::make_unique` under `nvs_array_set` names an
  array *literal*, not the array write path.** The empty array is a per-thread singleton, so
  `nvs_codegen`'s `emit_array_new` chain always separates on its first `nvs_array_set`, which makes
  `make_unique` the allocation site of every non-empty literal in the program. Read such a report as
  "an array literal was never released", not as a dropped copy-on-write clone.
  [until: reviewed 2026-09-06]
- **`tools/leak-check.sh` greps *every* loss record, so the frames under its `exit 97` are usually
  not the leaking stack.** Its `grep -E "definitely lost|nvs_stdlib|nvs_ir|nvs_runtime::" | head
  -12` matches crate frames in *possibly lost* and *still reachable* records too, and the twelve
  lines run out before the leak. The run is kept in `/tmp/leak-err` inside WSL: `wsl.exe -- grep -n
  -B2 -A14 "definitely lost" /tmp/leak-err` shows the blocks that are yours.
  [until: gone tools/leak-check.sh:head -12]
- **A `loop-goal.toml` fixture check whose frozen `want` belongs to a later stage fails until that
  stage lands, and reads as a regression.** `tools/loop.py` stops each sweep at its first failure,
  sorted by stage, so one such fixture can hide every check behind it; `.loop/log.md`'s `goal cost:
  Ns over N check(s)` is the line that says the sweep was cut short. Read the `[[check]]` block's
  `stage` before believing the banner, and never rewrite the fixture to print the frozen strings —
  the output is frozen and the source is not. [until: reviewed 2026-09-06]
- **`tools/try.py` runs `target/debug/nvs.exe` and never builds it, so a member you just added
  reports `E0405` as if its registry row never landed.** The driver builds that binary at session
  start and nothing afterwards refreshes it — a green `cargo test -p nvs-stdlib` does not, because a
  test binary is its own. One `cargo build` before `try.py` is the fix, not anything in
  `registry.rs`. [until: gone tools/try.py]
- **The `novis-db` compose stack can be *stopped* rather than broken, and a `[1 floor]` check then
  names a database the program never opens.** The line is `warning: no queue worker started:
  [db.main] ... did not open`, about the first service `nvs.toml` tries. `docker ps -a --format
  "{{.Names}}\t{{.Status}}"` shows it: every container `Exited (255)` at one timestamp is a host
  restart, and `docker compose -f tests/db/compose.yaml up -d` restores it.
  [until: reviewed 2026-09-06]
- **Stderr naming a path the repository never mentions is the shared test database talking; the exit
  code beside it is the finding.** `nvs.toml`'s `[queue] workers = 1` makes every `nvs run` drain
  every `novis_test` queue, so a job `crates/nvs-stdlib/tests/queue.rs` pushed prints as an
  unrelated `could not read <path>`. Read the run's block in `.loop/logs/<run>-console.log`: `exit
  3221226505` is `0xC0000409`, Rust's abort on Windows, and its panic is only there.
  [until: reviewed 2026-09-06]
- **`php-cgi -b` exits after 500 requests and `php -S` closes the connection to frame its body, so a
  client sees a reset socket and an empty body.** The FastCGI SAPI honours `PHP_FCGI_MAX_REQUESTS`
  and recycles the process at that count, which arrives as `WinError 10054` mid-run; PHP's built-in
  server answers `Connection: close` with no `Content-Length`. Set `PHP_FCGI_MAX_REQUESTS=0` in the
  child's environment as `tools/bench.py` does, and read to EOF when a response carries neither
  header. [until: reviewed 2026-09-06]
- **`cargo test -p nvs-cli --lib` is `error: no library targets found in package`, and the unit
  tests it was meant to run are in the bin target.** `nvs-cli` has no `lib.rs`, so a module's
  `#[cfg(test)] mod tests` runs under `cargo test --bin nvs <filter>`, and
  `crates/nvs-cli/tests/*.rs` drive the built binary instead of linking to anything. The same is
  true of every binary-only crate here. [until: exists crates/nvs-cli/src/lib.rs]
- **Guessing what a backend emits costs a design; a throwaway `#[test]` that prints it costs one
  build.** The plausible answer for `nvs_codegen::compile_object`'s relocations — ELF-style
  `PltRelative` to helpers, needing a stub past 2 GB — is not what this host does: a COFF object
  routes every external reference through a `.rdata$.refptr` cell, so the only kinds are
  `Relative`/32 and `Absolute`/64, with implicit addends that must be added to
  `Relocation::addend()`. Delete the probe once it has answered and put what it found in the module
  doc. [until: reviewed 2026-09-06]
- **A `cranelift-jit` panic in `compiled_blob.rs` (`TryFromIntError(NegOverflow)`) is address
  layout, not the tree; `-- --test-threads=1` settles it.** It is a PC-relative relocation whose
  target landed over 2 GB from the JIT buffer, which `-p nvs-cli`'s
  `ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once` provokes when it shares
  address space with other test binaries. Re-run alone before touching a line.
  [until: reviewed 2026-09-06]
- **`nvs-cli`'s `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names` asserts a
  ratio, so a loaded machine fails it from either side.** Each half takes the fastest of five
  attempts, but the halves do not run at the same moment, so under load warm can come out *slower*
  than cold — an inversion no cache regression produces, since a real one narrows the gap toward 1x
  rather than crossing it. `cargo test --bin nvs cache::tests::a_warm_start` alone
  settles it in a second.
  [until: gone crates/nvs-cli/src/cache.rs:a_warm_start_is_faster_than_a_cold_one]
- **A wall-clock regression in `nvs run` can sit entirely outside the code that caused it; one
  `Instant::now()` per phase in a *release* build says so.** Outside timing narrows to "somewhere in
  the cache path" and no further; the cost was `nvs_config::trust::check` on the cache directory,
  paid twice because `cache::from_config` runs for the compile and again for `script::Compiler`'s
  resolver. Instrument before the third external measurement. [until: reviewed 2026-09-06]
- **A new `examples/*.nvs` fixture is a capability denial until the repository's own `nvs.toml`
  grants it, and the driver reports that as wrong output.** A member
  `rule:security/capability-question-is-grant-and-scope` denies by default prints nothing on stdout
  and one `RuntimeError` on stderr, so an `exact` check reads as "stdout was [], wanted [...]" with
  nothing pointing at the config. `grep -n 'entry = "examples' nvs.toml` lists the existing
  `[[app]]` blocks; copy the neighbouring one with the narrowest grant, in the same slice as the
  fixture. [until: reviewed 2026-09-06]
- **The acceptance sweep's database holds the schema an older iteration built, so a schema change
  fails the sweep on state and not on code.** Converging `[db.main]` onto the queue's new value
  needed `--including-risky` for two narrowed columns, and the old partial index `nvs_jobs_dedupe`
  collided by name because a construct outside the vocabulary is invisible to introspection rather
  than reported as a difference. Converge the container by hand — `docker exec
  novis-db-postgres-1 psql -U novis -d novis_test` — before reading a red check as a bug.
  [until: reviewed 2026-09-07]
- **An item can name the last mile of a change whose earlier mile never landed, and it reads the
  same either way.** "A proven call site emits no tag check" presumes the checker proved something,
  and a call through `callable(int): string` still answered `mixed` with its arguments unchecked —
  so the emission decision had nothing to read. Spend one call proving the premise before designing
  against it: `target/debug/nvs.exe run` over a three-line probe says what the compiler answers
  today, and the driver already built that binary at this commit. [until: reviewed 2026-09-07]
- **A new file under `examples/` is walked by `crates/nvs-cli/tests/ast.rs`, which asserts that every
  node's span sits inside its parent's and that siblings are in offset order.** An example writing
  `|>` fails both, because the parser substitutes the left side into a call written to its right, and
  the panic prints a JSON node rather than saying which property broke. Run `cargo test
  --test ast` when an example's tree will not follow its source, and hand that file the walk's
  `in_source_order = false`. [until: reviewed 2026-09-07]
- **A stack overflow from `target/debug/nvs.exe` on a deep expression tree that
  `target/release/nvs.exe` walks is the debug build's fatter frames, not a bug you just wrote.** A
  48-stage `|>` chain runs in both, a 64-stage one only in release, and a 400-term `$n + 1 + 1 ...`
  does the same: the parser's guard counts stack frames, and a left-associative chain is a loop that
  charges it none. Run a hostile case or an example against both binaries before committing it;
  `dossier.py` takes release first and will only ever have judged that one.
  [until: reviewed 2026-09-07]
- **`nvs-lsp` gaining a `nvs-types` dependency fails `no_crate_the_server_links_writes_to_stdout`,
  and every line named is in `nvs-runtime`.** The type checker links `nvs-stdlib`, which links the
  runtime, whose `OutputSink::Stdout` is how a program's `echo` reaches a terminal, so the guard's
  `stdout()` pattern fires two hops past the crate you added and names neither. The exemption is in
  that test as `THE_OUTPUT_SINK`, paired with `nothing_under_the_server_wires_a_program_to_stdout`;
  widen the closure again and check both rather than dropping the pattern.
  [until: gone crates/nvs-lsp/tests/stdout_policy.rs:const THE_OUTPUT_SINK]
- **A stray `nvs.exe` locks `target/debug/nvs.exe`, and the rebuild that cannot replace it still
  looks like it worked.** A language server left running by a test, a probe or a killed editor holds
  the binary open, so the linker fails with `Zugriff verweigert (os error 5)` — a line in the middle
  of otherwise ordinary output — and the *old* binary stays on disk, so the next run tests the code
  you just changed away. `tasklist //FI "IMAGENAME eq nvs.exe"` before trusting a rebuild that
  disagrees with your edit, `taskkill //F //IM nvs.exe` to clear it, and check the mtime rather than
  the exit status. [until: reviewed 2026-09-08]
- **A new `Core\Request` member that answers `mixed` fails a test in `nvs-types`, and the message
  names a count rather than the member.** `core_lib.rs`'s
  `every_request_member_returning_outside_data_returns_it_tainted` asserts the *exact set* of rows
  under the `Core\Request` prefix that answer an unqualified type — it is the gate that catches a
  member handing a peer's bytes back unmarked — so a row answering `mixed` lands in it and the
  assertion reads "closed at eight". Add the row to the set and rewrite the count and the sentence
  naming `query` and `post` as the known hole. [until: reviewed 2026-09-08]
- **A `.nvst` case that fails while a unit test over the same code passes means the binary is
  stale, not the logic.** `cargo build --workspace` then an edit then `cargo test -p nvs-types`
  leaves `target/debug/nvs.exe` at the *pre-edit* behaviour, and the resulting diagnostic looks
  exactly like a real bug in whatever you just wrote — a checker relaxation reads as an ordering
  bug in your own comparison. Rebuild with `cargo build` before believing a `.nvst`
  failure that a `-p <crate>` test contradicts. [until: reviewed 2026-09-08]
- **`nvs run --request` hands a program the request but never matches it against the route table, so
  `Core\Request::route()` answers `null` in that leg.** `crates/nvs-cli/src/main.rs:1651` installs the
  table because `Core\Router`'s own members read it, and says in the same breath that a program run off
  the command line is matched against nothing — only the server's door and `runner.rs:504`'s test seam
  call `Inbound::set_route`. A fixture that needs the match spells the door's walk itself, over
  `Core\Request::method()` and `Core\Request::path()`, which is what `examples/parses.nvs` does.
  [until: gone crates/nvs-cli/src/main.rs:matched against nothing]
- **A socket a container publishes reaches the WSL leg only under `/mnt/wsl`, and on Docker Desktop
  the daemon's own `/mnt/wsl` is not that tmpfs.** The daemon runs in the `docker-desktop` distro,
  whose `/mnt/wsl` is a private directory of its root filesystem and whose mount of the tmpfs every
  distro shares is `/mnt/host/wsl`, so a bind source spelled `/mnt/wsl/...` from Windows lands where
  the leg never looks, and Docker Desktop's WSL integration — on or off — changes none of it: `wsl.exe
  -d docker-desktop -- ls /mnt/wsl /mnt/host/wsl` beside the leg's own `ls /mnt/wsl` is the tell.
  Docker creates a missing source root-owned 755 and the `redis` image binds as uid 999 whatever
  `user:` says, so the `certs` service chowns it; probe without `--rm`, because a dead container
  leaves an empty directory that reads as an unshared mount.
  [until: gone tests/db/compose.yaml:/mnt/host/wsl]
- **Nothing this repository runs on Windows compiles a `#[cfg(unix)]` arm, `verify.py` included.**
  `cargo check -p nvs-db --all-targets` was green over a MariaDB socket arm that read a private field
  of `Greeting` and named a type its test module does not import, because that arm is not in the
  Windows build and `verify.py` has no WSL leg. Compile every new `cfg(unix)` arm before you commit
  it: `wsl.exe -- bash -lc "cd /mnt/d/mwl && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo test -p
  <crate> -- <test names>"` builds and runs it in seconds warm, over the leg's own target directory.
  [until: reviewed 2026-09-09]
- **A value built on the boot thread and handed to a `nvs_host::Worker` must be `Send`, and
  `nvs_server::arm`'s `Armed` is not.** It holds an `Rc<Cell<usize>>`, so a `[[schedule]]` roster
  armed where its refusals belong — at the boot, before any core exists — cannot then be moved onto
  the core that ticks it, and the failure names `Worker::spawn`'s bound rather than the schedule.
  Hand the worker a flag saying it is the one that ticks and arm inside its body:
  `crates/nvs-cli/src/serve.rs`'s `Core::ticks` is the shape.
  [until: gone crates/nvs-server/src/schedule.rs:Rc<Cell<usize>>]
- **A `Core` member whose return type mentions `tainted` fails a test in `nvs-types`, and the
  message names neither the roster nor the member you added.** `core_lib.rs`'s
  `a_verified_signature_does_not_launder_its_claims` asserts a closed `BTreeSet` of every
  `(class, member, answer)` whose interned return type carries the qualifier, so a new row comes
  back as a bare `left`/`right` set diff in `-p nvs-types --lib`, two crates from the row you wrote.
  Add the tuple and a sentence to that test's own doc comment in the same slice as the registry row,
  the way its `Core\Zip` and `Core\Xml\Node` paragraphs read. [until: reviewed 2026-09-09]
- **A `Core` member answering `tainted` anything fails a test in a crate you did not edit.**
  `nvs-types`'s `a_verified_signature_does_not_launder_its_claims` holds a hand-written closed
  roster of every qualified answer, so `-p nvs-stdlib` stays green and the gate stops at
  `-p nvs-types --lib`. Adding one member is three edits in `crates/nvs-types/src/core_lib.rs` —
  the set entry, the doc paragraph naming that class's members, and the count in the assertion's
  own sentence — none of which is in conventions.md's five-edit recipe.
  [until: reviewed 2026-09-10]
- **A `Core` row whose parameter is a required *shape* needs its `$name` written in the spec's
  signature column, or two gates contradict each other.** `spec_registry_coverage.rs`'s
  `signature_names` reads a `{…}` with no `$name` as the *trailing options bag* and demands
  `names: [OPTIONS_NAME]`, while `registry.rs`'s
  `every_registry_row_names_one_parameter_per_positional_slot` forbids that very name on a positional
  slot — and a shape is positional. Amend the spec row to `{…} $settings` rather than the registry
  row, which is what that parser's own comment about `Core\Task::all` says a shape looks like.
  [until: reviewed 2026-09-10]
- **A `Core` member can be compile-folded, and then `conventions.md`'s five edits reach half of
  it.** `nvs_types::links` rewrites `Core\Router::url` into `nvs_core_router_link(<prepared path>,
  $params)` because its answer needs the route table, so a sibling needing that table is also a
  change in `nvs-types`, `nvs-ir`'s `lower_route_link` and `nvs-hir`'s `ROUTER_SCAN_MEMBERS`. Grep
  a class for a `link`-style module before scoping a slice on it, and budget for that arm having to
  flatten a `CoreTy::Shape` by hand. [until: reviewed 2026-09-10]
- **A hand-built `NVS_DB_MATRIX_*` environment reports a bad password as a poisoned `Once` in
  whichever case ran second.** `schema()` holds a `Once`, so the first failed handshake poisons it
  and every later case panics at `crates/nvs-stdlib/tests/queue.rs:394` naming neither the server nor
  the case that failed. Drive the suite with `python tools/db-matrix.py --driver <name>`, which
  exports the whole group; by hand, the password is `tests/db/compose.yaml`'s `POSTGRES_PASSWORD` and
  not the user name. [until: reviewed 2026-09-10]
- **A `Core` member that takes a capability is *six* edits, and the sixth is in another crate.**
  `conventions.md` § *A `Core` member* lists five, all in the class's own module, but a gated member
  also owes a row in `nvs_stdlib::registry::CAPABILITIES`, and that row cannot be written until
  `nvs_config::Cap` carries the variant — the enum is closed and both of its `match self` arms are
  exhaustive. Land the grant first (the variant, its `[capabilities.<family>]` struct in
  `nvs_config::tree`, and the `ALL`/`name`/`grant`/`grant_mut` arms), then the row, whose
  `crate::<module>::NAME` is still private if that class had no rows before.
  [until: reviewed 2026-09-10]
- **`cargo test -p nvs-lsp` and `python tools/verify.py` are both green while the `.lspt` corpus is
  red.** Nothing under `tools/` runs `nvs lsp-test`, so the corpus is gated only by the loop's own
  acceptance checks and a session that changes what a request *answers* hands the driver a failure it
  did not cause. Run `target/debug/nvs.exe lsp-test tests/lsp/` and `--coverage` after touching an
  answer, and expect the breakage at a cursor mid-word, where a case froze a list the new arm adds
  to. [until: exists tools/verify.py:lsp-test]
- **A `note:` on the configuration-resolution path prints on every `nvs` run in this checkout, and a
  landed test pins that stream empty.** `crates/nvs-cli/tests/bundle.rs:172` asserts `nvs run` writes
  nothing to stderr, and `cargo test` runs it in `crates/nvs-cli`, which fails
  `rule:config/ownership-is-the-trust-boundary` on any Windows drive whose root grants
  `Authenticated Users` write. Carry the reason as a value for the command an operator typed to
  render, rather than printing from the resolver.
  [until: gone crates/nvs-cli/tests/bundle.rs:nothing extra on standard error]
- **`serve::tests::the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle`
  fails under load and passes alone**, and `verify.py`'s message for it names a shared port, path or
  container, none of which it has. It drives a fleet-wide semaphore across threads, so a loaded machine
  lets one core's handler run after the ceiling already refused it — a session whose diff is nowhere near
  `crates/nvs-server` can spend calls proving the failure is not its own. Re-run `python tools/verify.py`
  and only investigate if two consecutive runs name it. [until: reviewed 2026-09-12]
- **`nvs-host`'s clock-reading watchdog tests fail under `verify.py`'s side-by-side test run and
  pass alone, so the gate goes red on a crate your slice never touched.**
  `a_capped_request_publishes_a_baseline_and_an_uncapped_one_publishes_nothing` and
  `a_run_that_is_no_core_is_sampled_and_reported_never` measure process CPU time every other test
  binary competes for, and `the_watchdog_reports_a_wedged_worker_without_a_heartbeat` allows a
  heartbeat a 250 ms wall-clock margin the same load eats, so which of the three fails varies per run.
  Read the failing name before looking for your own change in it; a rerun only moves it, and it stops
  the gate before the `.nvst` trees and clippy have run at all. [until: reviewed 2026-09-18]
- **A `cargo-named` floor check can go red on a *timing* test that the driver's own load broke, and
  it reads exactly like a regression the last commit caused.** `cache::tests::a_warm_start_is_faster
  _than_a_cold_one_by_the_margin_this_test_names` failed once after a session that touched only
  `nvs-stdlib`, because the guard took each arm's fastest sample over the whole sweep and a release
  prebuild running beside it stalled every warm arm. Run the named test by itself before believing a
  perf guard's failure — it passes at 10x on an idle box — and if it is load-sensitive, fix the
  statistic rather than the margin. [until: reviewed 2026-10-13]
- **A floor check that passed for sessions can go red because the Docker daemon restarted without
  the test servers, and the line the ledger prints names something else.** `examples/queue.nvs`
  opens its stderr with a `warning[W1008]` about an ungranted `cache.shared` store, while the
  failure is two lines below it: `[db.main]` at `127.0.0.1:15432` refusing the connection. Run
  `docker ps` before reading a green-yesterday check as a regression, and `docker compose -f
  tests/db/compose.yaml up -d --wait postgres redis` brings the two servers back.
  [until: reviewed 2026-09-13]
- **`a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile` fails under a full
  `verify.py` and passes on its own.** It asserts how many reads land while a compile is running,
  which is a timing claim, and `test` runs its binaries side by side — a loaded machine lets the
  compile finish between the two reads and the count comes back one short. Re-run `python
  tools/verify.py` before believing it: the site is `crates/nvs-cli/src/script.rs:1709` and it is
  nothing a stdlib or docs session touched.
  [until: gone crates/nvs-cli/src/script.rs:a reader was answered once and then waited the compile out]
- **`python tools/try.py` runs a case's `--FILE--` program and materializes none of its other
  `--FILE <name>--` sections.** A probe written as a multi-file case — an `nvs.toml` and a child script
  beside the program — runs with those files simply absent, so what comes back is `could not read
  child.nvs` rather than the answer the probe was asking for. Write the extra files into `.agent-tmp/`
  with the Write tool and name them from the repository root inside the program (`.agent-tmp/child.nvs`),
  which is the directory `try.py` runs it from. [until: reviewed 2026-09-13]
- **A red `nvs-suite` check reports the *suite's* aggregate, so a one-case failure reads as the case
  the check names.** `[2 the deadlock]` came back `exit 1 -- 1897 passed, 1 failed` over
  `tests/conformance/`, which says nothing about which of the 1898 failed, while the named case
  passed 8 runs and the whole suite passed 3, one under a concurrent `cargo build --tests`. Re-run
  `target/debug/nvs.exe test tests/conformance/` and read the failing case's own name before
  believing the check's. [until: reviewed 2026-10-14]
- **`hyper` polls its connection's read before it writes the answer it already holds, so a blocking
  transport under it deadlocks** — the symptom is a test binary that runs its whole body and never
  reports. It reads the head, dispatches, then asks for the next request, which on a blocking stream
  waits for a client waiting for that answer. Make a transport's read answer `WouldBlock` rather
  than wait, as `crate::io::Nonblocking`'s does; and kill the hung `.exe` before rebuilding, or the
  link fails `LNK1104` naming nothing. [until: reviewed 2026-09-14]
- **`hyper`'s client reads the socket before it writes, so a blocking transport under it wedges
  before the request ever leaves.** What you get is a thirty-second hang and
  `hyper::Error(Canceled, …)`, which names the server's idle bound rather than the cause. Give a
  client the `WouldBlock` transport the server half has — `nvs_config::control`'s `Client`, adapted
  by `nvs_server::io::Nonblocking` — and read its connection future finishing as the answer having
  arrived, not as a truncation. [until: reviewed 2026-09-14]
- **`#[cfg(unix)]` code is not compiled by anything this box runs, so a green `verify.py` says
  nothing about it.** The first Linux build of `nvs-cli`'s signal half reported
  `function_casts_as_integer` on a line that had been green here for sessions, and clippy's
  `-D warnings` on the driver's own leg would have failed on it. Build it yourself in seconds
  against the warm target dir the leg uses — `wsl.exe -- bash -lc "cd <the repo's /mnt path> &&
  CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo build --quiet"` — whenever a session writes or
  edits a `#[cfg(unix)]` block. [until: reviewed 2026-09-15]
- **`cargo test --lib <name>` runs none of a binary crate's unit tests.** `nvs-cli`'s cases live in
  the `nvs` bin target, so a `--lib` filter reports `0 passed` and `136 filtered out` with no hint
  that the target holding the test was never built. Filter with `cargo test --bins <name>` for
  anything under `crates/nvs-cli/src/`, and read the `filtered out` count beside the `0 passed`
  rather than the `ok`. [until: reviewed 2026-09-15]
- **Nothing in this tree builds a crate with a feature turned off, so a `#[cfg(feature = ...)]` arm
  rots unnoticed.** `verify.py` is the workspace at its default features, so the `exporter`-less shape
  of `nvs-server` and `nvs-cli` compiles in no gate at all. `cargo check -p nvs-cli
  --no-default-features --all-targets` is what proves it — a debug `-p` on purpose, and it does write
  the second metadata copy AGENTS.md rule 5 names, which for a `check` over a warm graph is seconds
  rather than a second build — so run it after touching anything those arms reach.
  [until: exists tools/verify.py:no-default-features]
- **A change to `nvs_db::ddl`'s emitted SQL is asserted from `nvs-stdlib` too, so a green
  `verify.py -p nvs-db` says nothing about it.** `Core\Queue`'s schema is a `Schema` value and its
  cases read the DDL the emitter writes for it (`crates/nvs-stdlib/src/queue.rs`), which is the way
  `rule:core-classes/db-crate-boundary` points that edge on purpose — the statements have one home,
  and what asserts over them lives where both crates are visible. Run `verify.py -p nvs-stdlib`
  beside the `-p nvs-db` one, or go straight to the full gate. [until: reviewed 2026-09-15]
- **Running one matrix leg by hand needs the trust anchor `db-matrix` exports and then deletes, and
  `NVS_DB_MATRIX_CA` must be absolute.** Cargo runs a unit test with the crate directory as its cwd,
  so a relative one panics as *the matrix server accepts a handshake* with a `NotFound` inside it.
  Copy it out with `docker compose -f tests/db/compose.yaml cp <service>:/certs/ca.crt <abs path>`,
  and prefix any `docker` call carrying a container-side path with `MSYS_NO_PATHCONV=1`, or Git Bash
  rewrites it to `C:/Program Files/Git/...` first. [until: reviewed 2026-09-15]
- **A column added to `nvs_stdlib::queue::schema()` has five hand-written copies of that table to add
  it to, and four of them are in other crates.** The schema value is the one home for what `nvs_jobs`
  *is*, but nothing derives the DDL in the three `queue-*-sqlite.nvst` cases or the bind arrays in
  `crates/nvs-stdlib/tests/queue.rs` (`push`, `push_keyed`, `push_marked`) and
  `crates/nvs-stdlib/tests/queue_sqlite.rs` (`push_in_two`), so a wider insert meets a narrower table — which fails the case suite locally and
  the matrix against a real server. Grep `nvs_jobs` across `crates/` and `tests/` before running
  anything, and add the slot to every list that grep names. [until: reviewed 2026-09-15]
- **A case list rerun over a second transport fires the first transport's own assertions.**
  `crates/nvs-db`'s suite over a Unix socket failed the cases asking whether the session is TLS, and
  panicked in the `let Location::Server(..) = .. else` arms that were `unreachable!`. Read every
  match on `nvs_db::matrix::Location` before adding a leg, and gate a case on what the *leg* carries
  — `handshake.rs`'s `upgrades` asks for an anchor — rather than on which driver it is.
  [until: gone crates/nvs-db/tests/handshake.rs:fn upgrades]
- **`schedule::tests::a_fleet_lease_is_renewed_while_its_run_is_in_flight` can fail one
  `python tools/verify.py` run and pass the next with nothing changed.** It asserts the lease was
  renewed *while* the run was still in flight, so the renewing task missing its slice reads exactly
  like a broken renewal, and `verify.py` runs the server's test binary beside every other one on
  purpose. Re-run `python tools/verify.py` once before reading it as a regression — twice red is a
  real one, and `verify.py`'s own message about a test that "passed alone" is the tell.
  [until: reviewed 2026-09-16]
- **A red `1 floor` check over an `examples/*.nvs` program can be the goal file gone stale under a
  landed decision, which reads exactly like a regression.** `examples/uncaught.nvs` wanted
  `#0 Boom::inner()` after `rule:errors/record-producers` fixed the JSON frames as
  `{function, file, line}` objects. Run the program and read the rule the check's comment cites
  first: where the rule says the new output is right, the repair is the example or the `want`, in
  both toml copies. [until: reviewed 2026-09-16]
- **`nvs-cli`'s `sd_notify_messages_are_ready_…_then_stopping` can fail with foreign
  `READY=1`/`STOPPING=1` lines ahead of its own**, then pass alone and on the next full run.
  `Notify::install` writes a process-global `OnceLock`, so this case's recorder also collects what a
  served life running beside it in the same binary reports. Re-run `python tools/verify.py` before
  reading it as yours; fixing it means scoping that global, not editing the case.
  [until: gone crates/nvs-cli/src/service.rs:fn install]
- **`cargo test` does not rebuild `target/debug/nvs.exe`, so a caret you read out of it after a
  test-only build is the previous commit's.** The driver builds the binary at the commit a session
  *starts* from and `cargo test` builds only test targets, so `nvs check` keeps rendering the old
  diagnostic while the suite already sees the new one — which reads as a bug in the change you just
  made. Run `cargo build` before reading a diagnostic out of `nvs.exe` whenever you have touched Rust
  this session. [until: reviewed 2026-10-16]
- **`cargo check --target aarch64-apple-darwin` cannot check an aarch64 `#[cfg]` from this machine.**
  It builds the whole graph for that target, and `ring`'s build script hands Apple flags (`-arch`,
  `-mmacosx-version-min=11.0`) to the local `cc`, which refuses them before any crate of this
  repository is reached. Keep architecture-dependent code to constants and pure functions a
  host-independent unit test can still exercise — `crates/nvs-cli/src/cache.rs`'s aarch64 relocation
  writers are the shape — and leave the cfg itself to the CI matrix.
  [until: reviewed 2026-09-17]
- **`nvs-stdlib`'s `socket_ping_keeps_a_quiet_live_peer_open` fails under a full `python
  tools/verify.py` and passes alone.** The case asserts that a quiet peer stays open, against a socket
  whose `idle` bound is 200 ms of wall clock, so the rest of the test binaries running beside it is
  enough to starve the ping and the `receive` throws `Timeout` instead. Re-run the gate once before
  believing that failure — `verify.py` tells you itself that the binary passed alone — and if it is
  the only red, it is not yours to fix inside an unrelated group.
  [until: reviewed 2026-09-17]
- **`verify.py`'s "failed beside the other test binaries and passed alone" can be a hash-order
  flake rather than a shared resource.** A test comparing two values through `format!("{:?}")`
  prints any `HashSet` field in an order that differs per process, so it fails at random and passes
  the rerun the tool does to check — which reads exactly like a shared port or temp path. Read the
  two sides of the assertion first: the same elements in a different order means the `Debug` impl is
  what needs fixing, not the isolation. [until: reviewed 2026-09-17]
- **A ratio-based perf guard in the floor can go red inside the full sweep and pass alone, and a
  sweep failure line without `asked … times` in it is
  that.** `a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names`
  measures four worker cores against one, and the valgrind sweep that runs immediately before it
  leaves a shadow that lands on the placed half alone — 22.2 ms against 0.8 ms on its own, with the
  serial half unchanged at 3.4 ms, 0.14 s after the last fixture exited. `tools/loop.py`'s
  `asked_again` re-asks a red `--release` check after each wait in `COST_SETTLES`, so a red that
  recovers is the shadow and one that survives the ladder is the tree; either way, read the margin from
  `cargo test --release -p nvs-abi-probe --test perf_guards <name>` run alone.
  [until: reviewed 2026-10-18]
- **A `cargo test --release -p nvs-cli` recompiles the whole crate for two and a half minutes after
  any commit, and a cost guard run straight after that measures the link.**
  `crates/nvs-cli/build.rs:99` names `.git/index` in a `cargo:rerun-if-changed`, so committing
  anything at all invalidates the release units, and the warm-start margin then read 1.8x where it
  names 4x on a commit that prints 13.2x once the box is idle. Build it with `--no-run` first and
  run the filter as a second call, and never read one red cost margin as a regression before that.
  [until: gone crates/nvs-cli/build.rs:.git/index]
- **`--bless` echoes a non-ASCII expected output as `?` on a Windows console, under a line telling
  you to read what it wrote.** Blessing an example that prints `ü` showed `?` while writing the
  correct `303 274` to the `.out`, so the echo disagrees with the file and the tool's own
  "a blessed output is a claim, not a formality" points at the wrong one of the two. Check the
  bytes with `od -c <file>.out` before believing the echo, and never "fix" an example because the
  bless line looked wrong. [until: reviewed 2026-09-18]
- **A blessed `.out` holding non-ASCII looks broken when the blesser echoes it back, and the file is
  fine.** `python tools/dossier.py --bless` prints what it wrote through the terminal's own code
  page, so on Windows `Café` comes back as `Caf?` and a Cyrillic line as a row of question marks
  while the `.out` on disk holds correct UTF-8. Read the file instead of the echo before deciding an
  example is wrong — `python -c "import sys; sys.stdout.write(ascii(open('<file>','rb').read().decode('utf-8')))"`.
  [until: reviewed 2026-09-18]
- **Two tests are red under the full gate today and pass alone, and neither is yours.**
  `nvs_host`'s `a_race_past_a_dead_address_answers_on_the_next_one` takes its dead address from a
  listener it dropped, so a binary beside it can take that freed port and answer the race;
  `nvs_server`'s `a_disconnected_clients_isolates_are_left_behind_on_no_core` reads a response load
  turns into a reset. Read the "passed alone" verdict before hunting what you broke, and fix either
  in a commit of its own.
  [until: gone crates/nvs-host/src/net.rs:a_race_past_a_dead_address_answers_on_the_next_one]
- **A cost-class guard whose figure is a *ratio* cannot be repaired by fixing the statistic.** The
  fan-out guard in `benches/abi-probe/tests/perf_guards.rs` already takes the minimum of five
  interleaved rounds, and a box with no free cores still collapses it from 3.86x to 0.08x — the same
  reading a picker that stopped spreading gives. Measure what the machine can give at that moment
  too, the same children on plain threads started once per batch, and report the guard not measured
  rather than failed when that figure is under the floor.
  [until: gone benches/abi-probe/tests/perf_guards.rs:plain_thread_fan_out]
- **A cost-class `bench.py` guard can go red on the machine rather than the tree, and its `start
  floor` line is the tell.** One sweep read a 77.5 ms floor where an idle box reads 4.6, and load
  here dilates a process multiplicatively, so the subtraction left 200 ms of machine — the spread
  gave nothing away, that run's median sitting 4% over its minimum. Read the floor line before the
  verdict: `--warm-start` abstains above `QUIET_FLOOR_MS`, a floor level rather than a multiple of
  the budget, and a figure it does report is a minimum over twenty-five reps.
  [until: gone tools/bench.py:QUIET_FLOOR_MS]

- **`verify.py`'s test leg red on a *different* test each run, each of which "passed alone", is the
  box and not the tree.** Two runs here died on a lease-renewal test and then on a revalidation one,
  and the third was 13 of 13 green: the box was taking 133 ms to run `nvs --version` with the CPU at
  6%, so the disk was contended and no load average would have said so. Read `bench.py
  --warm-start`'s `start floor` before touching a timing assertion — near 5 ms is believable, tens of
  ms fails a different test every run. [until: reviewed 2026-09-19]
- **A `.nvs` under `docs/examples/` or `tests/hostile/` that opens a database needs its own
  `[[app]] entry` block in `nvs.toml`, written after the file exists.** A `db.connect` grant is per
  entry path and the `root = "."` block carries none, and a block naming a path not yet on disk
  stops *every* program in the tree with `E0605`. Point it at `[db.schema]` — in-memory SQLite, no
  container — and grant `connect` alone, which is enough to `create table`.
  [until: reviewed 2026-09-19]
- **A red cost margin at the end of a sweep is the machine when one arm of the ratio moved and the
  other did not.** The warm-start margin read 3.62x and 3.85x thirty seconds apart against the 4x it
  names, then 12.3x on the same binary three minutes later — its warm arm tripled while its cold arm
  held at 57ms, so the compiler was untouched and the shadow fell on the half that maps pages. Read
  both figures off the failure line before believing a ratio guard; the link is not what does it,
  since the same test reads 12.3x straight off a 2m09s release build. [until: reviewed 2026-09-19]
- **`python tools/dossier.py --record-perf` refuses to measure on a busy machine, and the answer is
  to run it again.** Its calibration prices one iteration by timing an empty program against a unit
  one, so when something else holds the CPU the unit's fastest run lands below the floor's and it
  raises `the calibration did not measure anything` — the bench is not what is wrong. Re-run it: three
  refusals in a row are ordinary on this box, and changing `--reps` is not what makes the next one
  land. [until: reviewed 2026-09-19]
- **The first `python tools/dossier.py --run`, `--verify`, `--bless` or `--record-perf` after an edit
  under `crates/` sits for minutes, and it is building, not hanging.** Those four rebuild
  `target/release/nvs.exe` themselves when a build input is newer than it, and say so on stderr
  before cargo starts. Do not run `cargo build --release` for it and do not pass `--nvs
  target/debug/nvs.exe` to get around it: the attacks are sized for release, and one in five of them
  outlives its own `timeout-ms` against the debug binary.
  [until: gone tools/dossier.py:def current_binary]
- **`nvs-server`'s `a_fleet_lease_is_renewed_while_its_run_is_in_flight` fails under `verify.py` and passes alone.** It asserts a lease was renewed inside a wall-clock window, and 160 sibling tests sharing the machine widen it. Re-run it with `cargo test -p nvs-server --lib schedule::tests::a_fleet_lease_is_renewed_while_its_run_is_in_flight`, and when it passes finish the gate by hand: `cargo clippy --all-targets`, then `nvs test tests/conformance` and `tests/differential`. [until: gone crates/nvs-server/src/schedule.rs:the key was held again while the run was in flight]
- **An unbounded `.nvs` program outlives the tool call that started it and keeps
  `target/debug/nvs.exe` open.** The next `cargo build` then fails with `error: failed to remove
  file ... Zugriff verweigert (os error 5)`, which reads as a permissions problem while the process
  is still growing — one left running here reached 6 GB. Start anything that may not terminate
  under `timeout 60`, and check `Get-Process nvs` before believing a build error about that file.
  [until: reviewed 2026-09-19]
- **An `[[app]] entry` in `nvs.toml` naming a file that is not on disk yet stops every program in
  the repository, not only the one the block is about.** `E0605` is raised while the configuration
  tree is read, so a block written ahead of the proof it grants leaves `nvs run` and `dossier.py
  --bless` refusing files that have nothing to do with it, and the error names the missing *entry*
  rather than the program that was run. Write the `.nvs` first and its block second, or put both in
  one edit, and read the path in an `E0605` before believing it is about the file you just asked for.
  [until: reviewed 2026-09-19]
- **A bench's `// bench: calls N` declaration counts Novis frames, and a `Core` member call is not
  one of them.** A round whose only call is `Core\Debug::render` measures `calls 0.000`, so
  `--record-perf` refuses the figure and the bench reads as though its loop had been optimised away.
  Declare `calls` for the Novis functions a round enters, and read `allocations` and `bytes` to see
  that the native work really happened — one rendering counts 23.9 allocations and 1,157 bytes per
  op while its `calls` figure stays at zero.
  [until: reviewed 2026-09-19]
- **A `nvs` start-cost measurement taken from the repository root is measuring this tree's
  `nvs.toml` as much as the binary.** That file is a fixture the goals keep adding to — 207 live
  directives, 32 of them `[[app]]` blocks at about 42 µs each — so resolving it costs 1.9 ms of a
  start whose whole budget is 6, and it grew most of a millisecond while a fixed budget was being
  read against it. Name the configuration with `--config` so only the binary varies, and price what
  an implicit one costs by running `nvs config dump --config <file>` against a one-line config
  before believing a start-cost guard that went red. [until: reviewed 2026-09-19]
- **A `Core\Task` attack sized against the release binary runs for minutes under the debug one the
  hostile sweep uses, and the sweep reports that as `still running after Ns -- unbounded`.** A map
  over 100k elements and a `Core\Task::all` nest 50k deep each finished in well under a second when
  sketched, then took 17s and past 120s as a hostile case, because `dossier.py --run hostile` drives
  `target/debug/nvs.exe`. Time a new task-tree attack with `time target/debug/nvs.exe run <file>`
  before declaring its `timeout-ms`, and size the steps so the whole case lands near two seconds —
  the sweep runs 170 of them. [until: reviewed 2026-10-19]
- **`nvs-cli`'s `sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping` fails under load
  with two extra states in front of the ones it wants, and passes on the next run.** The recorder it
  reads is process-wide, so a sibling test in the same binary that reports `READY=1`/`STOPPING=1`
  lands in its list; `verify.py`'s own failure text names the shape. A `verify.py` run that is red on
  this test alone is re-run rather than diagnosed, and a session that touched no Rust has not caused
  it. [until: gone crates/nvs-cli/src/serve.rs:sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping]
- **A floor check that fails in the sweep and passes when you run it by hand can be
  load-dependent rather than flaky.** `examples/queue-purge.nvs` took 1.3s alone and 2.8s under
  sixteen busy cores, and its queue worker held a connection carrying a spent 2s handshake
  deadline that only ever fires on a read which has to wait, so the defect was invisible on an
  idle machine and certain under the sweep's overlapped release build, ThreadSanitizer and fuzz
  legs. Before calling a red floor check a flake, re-run it with the machine loaded and compare
  the run's wall-clock against every deadline the path files. [until: reviewed 2026-09-20]
- **A deeply nested literal crashes `target/debug/nvs.exe` where the release binary refuses it
  cleanly.** The parser's recursion guard reports `E0108` from nineteen nested `[` on, but the debug
  build's larger frames overflow its 1 MiB main-thread stack one level past that, so the probe prints
  `has overflowed its stack` and no diagnostic. Probe a nesting limit with `target/release/nvs.exe`,
  which `parser::MAX_RECURSION_DEPTH`'s own doc comment now says.
  [until: gone crates/nvs-syntax/src/parser/mod.rs:This bounds the parse, not the stack a build runs it]
- **`python tools/dossier.py --record-perf` measures a member against `target/release/nvs.exe`
  without rebuilding it, so a session that edited `crates/` measures the previous session's code.**
  It prints `target/release/nvs.exe is missing or older than the tree` in one line and goes on, and
  the goal's own acceptance check reports that same line as a failure until somebody builds. Run
  `cargo build --release -p nvs-cli` after the last Rust edit and before `--bless` or
  `--record-perf`; `verify.py`'s `fmt` step rewriting a crate file counts as an edit and makes the
  binary stale again. [until: reviewed 2026-09-20]
- **A bench whose round calls back into Novis cannot meet `// bench: allocations 0`, and
  `--record-perf` refuses the figure rather than recording it.** `nvs_runtime::call_closure` builds
  each call's argument slots in a heap vector (`crates/nvs-runtime/src/lib.rs` `# Known gaps` 10), so
  a fold over three ints with nothing of its own on the heap still measures 5 allocations per op.
  Declare `allocations 0` only where the round enters no closure — `count` does and `map`, `filter`
  and `reduce` do not — and read the measured number as the member's cost plus one vector per
  callback. [until: reviewed 2026-09-20]
- **Two timing tests fail under `python tools/verify.py` on a loaded machine and pass on their
  own.** `nvs_stdlib`'s `http::socket::tests::socket_ping_keeps_a_quiet_live_peer_open` and
  `nvs-lsp`'s `a_warm_index_answers_within_the_reanalysis_bound` both assert a wall-clock bound —
  the second one reported 204.1 ms against a 200 ms ceiling — and `test` runs every binary side by
  side. Read the two lines `verify.py` prints under such a failure: it re-runs the binary alone and
  says so, and neither test is anything a stdlib or docs session touched.
  [until: reviewed 2026-09-20]
- **`dossier.py --bless` and `--record-perf` run `target/release/nvs.exe`, so a Rust edit between
  writing a proof and blessing it costs a full release rebuild.** A `covers:` marker added to
  `arr.rs` after three examples were blessed made the binary stale, and the rebuild then ran twice
  because another process held a copy of `nvs.exe`. Make every Rust edit a slice needs first, then
  bless and record in one pass. [until: reviewed 2026-09-20]
- **A `cargo build --release -p nvs-cli` piped into `tail` reports `tail`'s clean exit, so a link
  that failed because another process holds `target/release/nvs.exe` reads as a green build.** An
  editor running `nvs lsp` out of this tree holds that exact file; this session's build printed
  nothing, exited 0, left the binary at its old timestamp, and the `--bless` after it spent the
  relink anyway. Run a release build with no pipe, and check the binary's timestamp moved.
  [until: reviewed 2026-09-20]
- **A `Core\BigInt` bench that declares `allocations 0` is refused, because every member of that
  class materializes its receiver.** `crates/nvs-stdlib/src/bigint.rs`'s `operand` rebuilds the
  magnitude from the instance's two slots on each call, so even a member answering an `int`
  allocates once, and `--record-perf` writes no record while the declaration disagrees with the
  count. Read what a sibling member already recorded in `docs/perf/members.ndjson` before choosing
  the number a new bench declares. [until: reviewed 2026-09-20]
- **A bench that picks its chained input out of an array measures the array too.**
  `benches/members/core/Bytes/join.nvs` read its separator as `$separators[$total % 3]`, and a
  packed list synthesizes a key on every read, so the first figure was 8.0 allocations at 168.6
  ns/op where the member's own work is 6.5 at 130.4. Chain through locals a ternary picks
  between — `($total % 2) == 0 ? $short : $long` — and re-measure with `--force`, which appends a
  second ledger row rather than replacing the first. [until: reviewed 2026-09-21]
- **`serve::tests::sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping` fails under
  load with one `STOPPING=1` too many, and passes alone.** It asserts the exact list a `Type=notify`
  unit is owed (`crates/nvs-cli/src/serve.rs:3913`), so a fifth message from a binary running beside
  it reads as a broken shutdown in whatever the session just touched. Re-run that one test on its
  own before treating a red `test` step as a regression, and re-run `python tools/verify.py`, which
  re-runs only the step that failed. [until: gone crates/nvs-cli/src/serve.rs:sd_notify_messages]

## Writing a test case

- **`var` and a written type are two different declarations, and `var T $x = …` is neither.** `var
  $x = …` infers; a declared type is `T $x = …;` with no `var` at all, so `var Core\Regex\Pattern $p
  = …` is four diagnostics (`E0101` three times, then `E0301` for a name never declared), none of
  which says "drop the `var`". The inferring spelling is the one every case reaches for, so the
  typed one looks like it should take a keyword too. [until: reviewed 2026-09-06]
- **An array literal written directly as a `Core` argument or a `foreach` subject is `array<mixed>`
  and is refused.** `Core\Arr::replaceRange($a, 1, 2, ["X"])` is `E0401: expected array<string>,
  found array<mixed>` and `foreach (["a", "b"] as string $t)` is `E0401: expected string, found
  mixed`, because the receiving type is never pushed into the literal and `var` refuses to infer an
  element type (`E0414`). Declare a typed local one line above — `array<string> $rows = [...]` — and
  pass or iterate that. [until: reviewed 2026-09-06]
- **A case cannot index into an `array<mixed>`'s elements, and `Core\Json::encode` is the way round
  it.** `$q["b"] as array<string>` panics `nvs-ir` outright (*"got `Tagged as Array`"*) —
  `rule:types/conversion`'s `array<T> as array<U>` row is the one still missing — so a member
  answering a nested shape has no spelling that reaches past the first level.
  `Core\Json::encode($q)` renders the whole structure in one line, byte-identical to PHP's
  `json_encode`; `$q["a"] as string` on a top-level scalar does lower. [until: reviewed 2026-09-06]
- **A named class does not satisfy a shape type, whatever its properties are called.** `View::y(new
  Point(3, 4))` against a `{y: int}` parameter is `E0401: expected {y: int}, found Point`, because
  `rule:types/shape-type`'s width subtyping is shape-to-shape only (`nvs_types::expr::assign`). The
  only widening a case about a shape receiver can write is a narrower shape; do not design one
  around the class direction. [until: reviewed 2026-09-06]
- **A `?array<T>` is indexable after a `!= null` guard, but a `?array<T>` *answer* indexed directly
  is still `E0482`.** `narrow` drops `null` and `Lowering::untag_narrowed` untags the read at the
  variable, so every read inside the guard works; `Core\Arr::first($rows)` indexed inline has no
  test to narrow. Bind it first (`?array<string> $row = Core\Arr::first($rows);`); under `??` every
  level of the chain is guarded and `$a["nope"]["j"] ?? "d"` needs no test.
  [until: reviewed 2026-09-06]
- **Registering a `Core` member and writing its conformance case are one slice, not two.**
  `crates/nvs-stdlib/tests/conformance_coverage.rs` fails the moment a registry row has no `.nvst`
  case calling it, and `verify.py` reports that as a `-p nvs-stdlib` failure with nothing about the
  member in the message. A class constant counts too: `Core\Path::SEPARATOR` needs a case that
  writes it. [until: reviewed 2026-09-06]
- **A multi-file `.nvst` case runs every file's statements, each in its own variable scope.**
  `--FILE <relative/path>--` repeats and writes another file into the case's directory, so a class
  in a second file is reachable and a bare `echo` at its file scope prints where the `require` is. A
  required file's `$x` is not the caller's
  (`rule:statements/a-required-file-shares-declarations-not-locals`), and an autoloaded file runs
  only its declarations. [until: reviewed 2026-09-06]
- **A `--EXPECTF-ERROR--` case must not also *use* what the broken declaration would have
  provided.** Diagnostics are ordered by phase, not by file, so an `E0303` from the entry point's
  reference prints before the resolution error the case exists to pin, and the block no longer
  matches at its first line. A compile-error case's entry file should do the least that reaches the
  diagnostic — often a bare `require` — and `%A` covers the span between two diagnostics, notes
  included. [until: reviewed 2026-09-06]
- **A rule added to `nvs_syntax::check_declarations` reaches far less of the corpus than a grep
  suggests.** Only `nvs-cli` and `nvs_hir::requires` call that walk, so every `nvs-types` fixture,
  parser test and `nvs-codegen` fixture goes straight past it — a `<?nvs` snippet in a Rust string
  is not automatically subject to everything the compiler enforces. Grep for the *callers* before
  budgeting a corpus rewrite. [until: reviewed 2026-09-06]
- **A row the checker accepts is not a row that runs.** `nvs-codegen` refuses with *"does not lower
  a binary operator over mismatched representations"*; `rule:types/arithmetic`'s promotions and
  `E0715` are out of that hole, so what reaches it is mostly a `Ty::Tagged` operand no diagnostic
  names. Run the rows in a scratch `.agent-tmp/*.nvs` before writing a case off a rule's compiling
  rows, and pin one that does not lower in the crate's `tests/`. [until: reviewed 2026-09-06]
- **`"…" as bytes` is how a case writes a `bytes` it can read, and `Core\Encoding::fromHex("…")` is
  how it writes one it cannot.** There is no `bytes` literal; `as bytes` is total and free but only
  reaches octets that are valid UTF-8, so an arbitrary buffer — a lone `ff`, a truncated sequence —
  has to come from `fromHex`. Assert the result with `toHex` either way, since `echo` has no `bytes`
  row and `rule:types/conversion` makes `bytes as string` checked. [until: reviewed 2026-09-06]
- **`<`/`<=`/`>`/`>=`/`<=>` over two `string`s is `E0715` where it is written, and the diagnostic
  names the member that says what was meant.** `Core\Str::compare` is the ordering two strings have;
  the same code refuses a `bytes`, an `array<T>`, a `callable`, an enum case (order `as int`
  instead) and `null`, while the object family keeps `E0411`. A case asserting text ordering asserts
  `Core\Str::compare`, and one wanting a fixed slice still uses `==`; a `Core` member answering
  `uint` (`Core\Str::length`) in `$int + …` is `E0407`, not a widening. [until: reviewed 2026-09-06]
- **`emit_binop`'s `integral` set is `Int | Uint | Bool`, so an enum operand needs the
  reinterpretation first.** `==` over two enum values lowers because `nvs-ir` compares one
  representation down — `InstKind::Reinterpret` to the backing integer is free, the row `$m as int`
  already uses. A new lowering that emits a `BinOp` over `Ty::Enum` directly fails with *"a `Eq`
  over representation Enum(Int)"*. [until: reviewed 2026-09-06]
- **Never put `--ORACLE--` in a `tests/conformance/` case.** CI runs that suite on three hosted
  runners with no PHP, so an oracle section makes the runner *skip the whole case* there. Verify
  against PHP while authoring — `php -r '…'` is on `PATH` under Windows and inside WSL
  ([docs/setup.md](../setup.md)) — then drop the section or put the case in `tests/differential/`,
  where an oracle belongs; and a trailing space before a `\n` is unreliable in an `--EXPECT--`
  block, so echo a sentinel after it. [until: reviewed 2026-09-06]
- **A `nvs-types` test that asserts an interned type's `describe` string is fragile.** A union
  orders its members by type id, so registering a member anywhere can flip `T|null` to `null|T`.
  Compare against `interner.make_union([...])` instead. [until: reviewed 2026-09-06]
- **`NvsStr::from_raw`/`NvsArray::from_raw` return an *owning* handle.** Reading a refcount through
  one in a unit test releases a reference when it drops, so the test ends in a heap corruption
  rather than an assertion failure. Wrap it in `std::mem::ManuallyDrop`; `crate::arr::borrowed` is
  that wrapper for an argument, and `crate::instance::slot` is the borrowed read of an object's
  slot. [until: reviewed 2026-09-06]
- **`Core\Path` emits a platform separator, so a case that prints or counts a built path is
  leg-dependent.** Normalize a printed path with `Core\Str::replace($p, Core\Path::SEPARATOR, "/")`,
  and count what a member dropped by `Core\Str::length($rebuilt) != Core\Str::length($p)` rather
  than by string inequality, which counts every re-rendered separator on one leg only. A `Core\Time`
  case has the same hazard elsewhere: never assert `Zone::system()`'s answer or a wall-clock value.
  [until: reviewed 2026-09-06]
- **Clippy refuses a float literal that approximates π or e, and refuses `assert!` over two
  constants.** A compile-time invariant belongs in `const _: () = assert!(…);`, not a `#[test]`.
  [until: reviewed 2026-09-06]
- **A property's declared default runs, and the constant is checked — a literal of the declared type,
  `= []`, `= null` where the type admits one, an enum case and another class's `const` are the
  constants it accepts.** `public int $n = 4;` reaches the slot of every fresh instance, inherited
  defaults included, because `nvs_runtime::NvsObj::new` writes a per-class image the descriptor
  carries. A union is read one level deep, so `?string $label = "plain"` and `"read"|"write" $mode =
  "read"` place against the member the literal inhabits. A `decimal` and a non-empty array literal
  are still `E0472` at a property, and a `static` property is skipped entirely since it occupies no
  instance slot. [until: reviewed 2026-09-06]
- **Two `foreach` headers in one file may reuse a binding name only at the same type.** A binding is
  function-scoped, so `foreach ($names as string $n)` followed by `foreach ($heap as int $n)` is
  `E0406: `$n` is already declared` at the *second* header, while a second `string $n` walk is fine
  — and a typed declaration *inside* one loop body is fine too, declared once and assigned each
  iteration. A case walking two differently-typed collections needs two names.
  [until: reviewed 2026-09-06]
- **`live_bytes()` cannot see an allocation freed again inside the call under test, so a
  `live_bytes` delta passes a no-allocation guard either way.** `nvs_array_set` renders an `NvsStr`,
  hands it to the packed arm and drops it before returning — live delta zero, exactly like the pair
  that never allocated. Use `counting_alloc::allocated_bytes()`, the monotone total, for any claim
  about a *transient* cost, and assert in the same test that the old spelling *does* allocate, or a
  broken counter reads as a passing guard. [until: reviewed 2026-09-06]
- **A compiled function called with an empty argument slice faults.** `nvs_runtime::call(f, &mut
  ctx, &[])` on a method is an access violation (`0xc0000005`, a bare `STATUS_ACCESS_VIOLATION`
  naming no test), because the callee reads its argument slot regardless. Give the method one
  parameter it ignores and pass `Value::int(0)`, as `nvs-codegen`'s `stack_limit.rs` fixtures do; it
  bites only through `unit.function("Class::member")`, never `run_with`/`output_of`.
  [until: reviewed 2026-09-06]
- **An allocation guard over a member that *builds* something measures the result's own storage
  first.** A `counting_alloc::allocated_bytes` delta over one `map`-shaped walk into a fresh
  `NvsArray` counts the output's own `Vec` doubling, which the guard cannot tell from the allocation
  it exists to catch. Walk twice and measure the *second* pass, where every write lands at a
  position that already exists; `a_callback_that_does_not_want_a_key_synthesizes_none` is the shape.
  [until: reviewed 2026-09-06]
- **A `cargo test` that dies with a bare `STATUS_ACCESS_VIOLATION` may not reproduce, so run it
  again before bisecting.** One arrived in `-p nvs-codegen --test throwing` on the first run after a
  relink and never returned. The deterministic cause — an empty argument slice, the neighbouring
  bullet — reproduces every time, which is how the two are told apart. [until: reviewed 2026-09-06]
- **A test binary outside `nvs-runtime` measures allocations through `nvs_runtime::budget`, and may
  not install a `#[global_allocator]` of its own.** `budget::live_bytes`, `::allocated_bytes` and
  `::allocations` are `pub` and maintained in every profile, and `nvs-runtime` registers
  `budget::Accounting` in every `not(test)` build, so a second global allocator in a test file is
  `error: the #[global_allocator] in this crate conflicts with global allocator in: nvs_runtime`.
  `crates/nvs-stdlib/tests/allocation_policy.rs` reads the shared counters.
  [until: reviewed 2026-09-06]
- **Measure compiled code's allocations as a difference between two run lengths or two arities,
  never as an absolute zero.** A run allocates its array, locals and output buffer once, and
  `call_closure` its retained argument slice once per call, so "400 passes allocated what 4 did"
  holds where "allocated nothing" cannot. Pair it with a control arm that *does* allocate per
  access; only with no callback (`sort($list)`) is an absolute bound right.
  [until: reviewed 2026-09-06]
- **A `-p nvs-stdlib` test can hand a `Core` member a real `callable`.** `nvs_runtime::call_closure`
  reads only `CLOSURE_ARITY_SLOT` and the `CLOSURE_INVOKE` address, so a `ClassTable::define` +
  `set_methods` pair with a plain `unsafe extern "C" fn` is a whole closure;
  `crates/nvs-stdlib/tests/allocation_policy.rs`'s `closure_of` is the shape, and leaks the table: a
  descriptor's address is its identity. The callee owes the exit sweep or valgrind catches it.
  [until: reviewed 2026-09-06]
- **A `catch` binding declared inside a loop body still belongs to the function, so a later
  file-scope `catch` cannot reuse its name.** Several file-scope `try`s each binding `$e` compile,
  but once one is inside a `for` or `foreach`, every later clause reports `E0406: `$e` is already
  declared` pointing at the loop's clause. Give each `catch` in a case its own name (`$capped`,
  `$zero`, `$beyond`); in the same family, `Core\Str::repeat` takes a `uint`, so an `int` counter
  needs `$d as uint` at the argument or it is `E0401`. [until: reviewed 2026-09-06]
- **A `.nvst` case's own helper has to be a `public static function` inside a class.** A plain
  `function render(...)` at file scope is `E0215: a function must be a method`
  (`rule:classes/no-free-functions-or-constants`), caught only when the case is run. Wrap it in a
  `final class` and call it `Render::pairs($m)`; a compiler-owned generic (`Core\ObjectMap<Tag,
  int>`) is accepted in that method's parameter list. [until: reviewed 2026-09-06]
- **`function (…) { … }` is `E0222` outright; `fn (…) => …` or `fn (…) => { … }` is the one closure
  literal.** Calling it through its variable or `$f(...$args)` lowers, but a call through a
  `callable` answers `mixed`, so the result takes an `as T` where a narrower type is declared
  (`rule:types/closure-literal`). When a sweep's rows are *data*, a typed array plus `foreach`,
  counting into an `int` declared above the loop, is the shape. [until: reviewed 2026-09-06]
- **An `--ORACLE--` helper must not be named after a PHP built-in, and the failure does not say
  so.** PHP writes *Cannot redeclare function* to stdout, which the runner compares, not reports, so
  a helper named `pos` fails as `--ORACLE--: PHP exited 255` with `php stderr: <empty>`. Name a
  helper for what it renders (`render`, `show`); `php -r 'var_dump(function_exists("<name>"));'`
  settles a doubt, and `key`, `next`, `end`, `reset`, `current`, `compact` collide too.
  [until: reviewed 2026-09-06]
- **A differential case checks itself, so write the rows and run it rather than pricing PHP's answer
  by hand first.** An `--ORACLE--` case's failure output prints both columns side by side, which is
  the whole comparison in one call. Reach for `php -r` on the *divergence* half instead, where the
  frozen `--EXPECT--` is Novis's own output and PHP's answer only appears in the case's prose — the
  one sentence the runner cannot check. [until: reviewed 2026-09-06]
- **An `int` literal does not reach an `array<float>`'s element type, so a case about the one
  numeric domain declares `array<int|float>`.** `Core\Arr::contains($floats, 1)` is `E0401: expected
  float, found int` at the argument: the needle is typed `T`, and arguments do not widen.
  `array<int|float> $numeric = [1.0, 2.5];` makes `T` the union, and `contains($numeric, 1)` answers
  `true`, the `rule:expressions/equality-semantics` row worth pinning. [until: reviewed 2026-09-06]
- **A `Core` member's refusal is either a `FATAL:` line no `catch` sees or an ordinary `Throwable`;
  the `Fault::` constructor decides which.** `Fault::fatal` (every argument-shape guard in
  `nvs-stdlib`) unwinds past `catch (Throwable $e)`, so pin it with `--EXPECT-ERROR--` and read the
  message off `2>`; `Fault::thrown` (`nvs_stdlib::ordering::compare_values`, so
  `Core\Arr::min`/`sort` over a mixed subject) is caught at file scope. Check the constructor, not
  the member. [until: reviewed 2026-09-06]
- **A `?bool` cannot be tested for truth, so a member answering one has no `yn` rendering at all.**
  `if ($found as bool)` on a `?bool` panics `nvs-ir`'s truthy-condition slice with *"got Tagged"* —
  `as bool` does not narrow the binding out of `Ty::Tagged`, and the guarded-branch conversion that
  works for `?int` and `?string` has no counterpart because the condition is what fails. Keep a
  `bool`-valued subject out of a case about a `?T`-answering member, or render it through a member
  that answers `string`. [until: reviewed 2026-09-06]
- **An `--ORACLE--` case must never `echo` a `NAN`.** PHP 8.4 and later emit *"Warning: unexpected
  NAN value was coerced to string"* onto the same stream as the output, so the oracle's expectation
  carries a warning Novis's side cannot print and the case fails on a row that agrees; `INF` is
  fine. Render the value through a guard — `$v == $v` is false for exactly one `float`, on both
  sides — and echo a sentinel, as `math-int-div-and-mod-match-intdiv-and-fmod`'s `Show::real` does.
  [until: reviewed 2026-09-06]
- **A `CoreTy::Var("T")` signature binds `T` to the first argument, so a mixed-type pair never
  reaches the runtime.** `Core\Math::min`, `max` and `clamp` are all `Var("T")`, so
  `Core\Math::min(0, "a")` and even `Core\Math::min(2, 1.5)` are `E0401` at the *second* argument.
  Declare the union on the bindings (`int|string $zero = 0;`); a `float` parameter does not widen an
  `int` literal either, so `Core\Math::mod(7, 2.0)` needs `7 as float`. [until: reviewed 2026-09-06]
- **A PHP notice or deprecation lands on the oracle's *stdout*, so an `--ORACLE--` case that trips
  one can never match.** `hexdec("beefy")`, `base_convert("-255", 10, 16)` and `chr()` outside
  `0..255` each print a `Deprecated:` line before answering, and the rule is general: any input a
  twin *repairs* rather than refuses is a candidate. Run the oracle body through `php -r` while
  authoring — the notice is visible there and invisible in the `.nvst` diff — and split the repaired
  inputs into an `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--`. [until: reviewed 2026-09-06]
- **A frozen `--EXPECT--` cannot hold a decomposed grapheme cluster or an invisible byte, and
  nothing warns you.** `"cafe\u{0301}"` sliced at its last cluster renders like the precomposed `é`,
  and a refusal that quotes its subject unescaped (`Core\Encoding::encodeText`) puts a raw C1
  control in the expectation; either fails with two halves that look the same. Echo
  `Core\Encoding::toHex($s as bytes)` for any cell that is not plainly ASCII.
  [until: reviewed 2026-09-06]
- **A `?string` does not narrow into a `string` return position, and the fix is `as` rather than a
  different `if`.** `if ($v == null) { return "<none>"; } return $v;` is `E0403: this method
  declares `string` but returns `string|null``, and inverting the test reports the same one line up.
  Write `return $v as string;` after the null test; the same `as` re-supplies the type inside an
  expression (`Core\Str::slice($s, ($at as uint) as int)`). [until: reviewed 2026-09-06]
- **`preg_split("//u", $s, -1, PREG_SPLIT_NO_EMPTY)` is the mbstring-free code point splitter, and
  works on the Windows `php`.** PCRE carries its own UTF-8 support, so a `Core\Str` slice that needs
  PHP to count code points has a real oracle rather than a frozen `--EXPECT--`. What it cannot give
  is a code point's *number* (`mb_ord`) or a grapheme (`grapheme_strlen`: no `intl` either), so an
  oracle needing those still decodes UTF-8 by hand in the `--ORACLE--` block or freezes the rows and
  cites the UCD table. [until: reviewed 2026-09-06]
- **`Core\Json::encode` refuses a `bytes` value, so it is the way round an `array<mixed>` only while
  every element is a scalar or a string.** `Core\Bytes::unpack`'s answer is where that bites: a
  format holding an `a`, `A` or `Z` field yields a buffer element, and encoding the list throws
  *"Core\Json::encode(): tag 11 has no JSON encoding"*, which reads as a bug in the case rather than
  the missing row it is. Render such a case with `Core\Arr::count` for the shape and
  `Core\Encoding::toHex` per buffer, and keep `Json::encode` for the numeric formats.
  [until: reviewed 2026-09-06]
- **A `.nvst` helper cannot take a `Core` enum parameter, and the diagnostic names the same type
  twice.** `function m(string $s, Core\Charset $c)` called with `Core\Charset::Ascii` is `E0401`
  reading *expected `Core\Charset`, found `Core\Charset`*: a source annotation does not unify with
  the registry's `CoreTy::Enum`, and `mixed` is refused at the `Core` call inside. Write the enum at
  each call site, one inline `try`/`catch` per row. [until: reviewed 2026-09-06]
- **A `gaps.py --errors` row marked `thrown_as` carries a *class*, and asserting which one is the
  row's point.** `Fault::thrown_as(ThrownClass::Logic, …)` and `…::Parse` are ordinary `Throwable`s,
  so `catch (Throwable $e)` reaches both and says nothing; the discriminating clause is `catch
  (LogicError $e)` or `catch (ParseError $e)`, and two on one `try` compile at file scope with
  separate binding names. The roster is `nvs_runtime::throwable::ThrownClass`'s doc comment.
  [until: reviewed 2026-09-06]
- **A case that names a `Core\Class::member` inside a *string* writes the literal single-quoted.**
  Single-quoted, only `\\` and `\'` are escapes (PHP's rule), so `'Core\Time::fromIso(): '` is
  exactly those characters. `gaps.py --errors` drops a site only when the literal run before the
  message's first format hole appears in the suite, so assert `Core\Str::startsWith($e->message,
  '…')` with the prefix spelled out, not a tail a dependency bump will rewrite.
  [until: reviewed 2026-09-06]
- **`echo "label=", <a call that may throw>` prints the label before it throws, so a `try` body
  leaks its prefix on the rows it exists to refuse.** The refusal's own text arrives welded to the
  label on one line. Bind the call first (`var $written = Core\Json::encode($v);`) and echo on the
  next line, so the refusing row prints nothing and the `--EXPECT--` block stays a list of the rows
  that answered; in the same family, a local obeys `E0112`, so `var $written_nan` is refused and
  `$writtenNan` is the spelling. [until: reviewed 2026-09-06]
- **Check a `gaps.py --errors` site's own parameter types before taking it as catchable.** A
  `Fault::thrown` guarding an *element* of a typed parameter may be unreachable:
  `Core\Csv::format`'s "column N holds a value that is not a `string`" sits behind an
  `array<array<string>>` parameter, and every way round it is `E0401` or the `array<T> as array<U>`
  cast that panics `nvs-ir`. An entry is a candidate, not a plan; four `nvs run` calls on a scratch
  file judge one. [until: reviewed 2026-09-06]
- **`Core\Bytes::join`'s allocation refusal cannot be reached from source, and the obvious probe
  pins the wrong member.** Its size is a sum, so a huge separator (`Core\Bytes::fill(1e12, 44)`) is
  refused by `fill`, and the row pins `fill` twice. The reachable count-shaped refusals are
  `Core\Str::repeat`/`padStart`/`padEnd`, `Core\Bytes::repeat`/`fill` and `Core\Random`'s two; a
  *product*-sized member reaches `nvs_runtime::affordable`'s sentence, a `uint`-sized one only the
  allocator's. [until: reviewed 2026-09-06]
- **An agreement case gets its sweep from a `mixed`-taking helper and `Core\Json::encode`, not from
  a closure.** Each member answers a different type and `Core\Json::encode` renders every one, so a
  `public static function same(string $name, mixed $a, mixed $b, mixed $c): int` in a `final class`
  compares renderings, echoes `DISAGREE` and the name, and returns 0 or 1. The case sums `$ok = $ok
  + Sweep::same("count", …);` per member and asserts the total. [until: reviewed 2026-09-06]
- **Do not put an append past `9223372036854775806` in a case: it kills the run.** A key of
  `9223372036854775807` followed by `$a[] = v` trips `Table::append`'s `"the append counter never
  names a live key"` `debug_assert`, and `nvs run` dies mid-file, so every later row is lost and the
  failure reads as the harness. Why that is a crash rather than PHP's `Error` is open work, not a
  case's to pin.
  [until: gone crates/nvs-runtime/src/array.rs:the append counter never names a live key]
- **Run `php -m` before designing an oracle around an extension's function.** The Windows `php` has
  no `mbstring`, so every `mb_*` dies with *"Call to undefined function"*, and neither leg has
  `gmp`, so `Core\Math::gcd`/`lcm` have no callable twin; `bcmath` is on both. Write the oracle as a
  second implementation in PHP, as `Core\Path::normalize` does (`gaps.py --differential` looks for
  the Novis member in `tests/differential/`, not the twin). [until: reviewed 2026-09-06]
- **A counter declared `uint` cannot be incremented by a literal.** `uint $n = 0; $n = $n + 1;` is
  `E0407: int and uint have no representable common type in arithmetic`, because the literal is an
  `int`, followed by an `E0401` reporting the result as `mixed`. The spelling that compiles is `$n =
  $n + (1 as uint);` — parenthesised, since `as` binds looser than `+` — and it is worth it whenever
  the total is compared against `Core\Arr::count`, which answers `uint`.
  [until: reviewed 2026-09-06]
- **An `--ORACLE-DIVERGES--` block is *one line*, however long the prose is.** The harness answers
  `not a valid case: line 3: `--ORACLE-DIVERGES--` is one line` and refuses the whole file, so a
  divergence written as paragraphs has to be folded into one. Write it as one line from the start
  with a capitalised lead-in (`THE INTEGER BAND:`) where a heading would have earned a break;
  `json-decode-refuses-the-number-band-json_decode-degrades.nvst` is the worked shape.
  [until: gone crates/nvs-test/src/case.rs:`--ORACLE-DIVERGES--` is one line]
- **`Core\Str::length` counts characters, and a CRLF is one of them — so it is the wrong ruler for a
  round trip.** A `Core\Csv` probe measuring `"a\r\nb"` read 3 on both sides, which reads as "the
  reader normalized the CRLF away"; it had not. Any claim about *which bytes* survived a member is
  written with `Core\Encoding::toHex($s as bytes)`, and `length` is kept for a count of characters.
  [until: reviewed 2026-09-06]
- **A case asserting two float computations agree has to pick rows whose *intermediate* is exactly
  representable, or it pins one libm.** `Core\Math::hypot($x, $y)` against `Core\Math::sqrt($x * $x
  + $y * $y)` can agree on every row of a table only because MSVC happens to round them the same
  way, and nothing says glibc on the WSL leg does. Restrict the table to Pythagorean triples, zeros
  and dyadic fractions so the intermediate is exact and IEEE 754 requires the two to agree — a
  property of the table, not the host. [until: reviewed 2026-09-06]
- **A `Core` member accepts a `tainted` argument where its row says so; the trap is the type of the
  *answer*.** Only a `Sink` refuses one (`rule:security/unclassified-parameter-refuses-tainted`),
  and a contagious answer carries it into elements and union members: `Core\Str::split($t, ",")` is
  `array<tainted string>` and the plain type is `E0401`. `tainted ?string` does not parse and fails
  as `E0102: expected an expression` a line *before* the mismatch. [until: reviewed 2026-09-06]
- **`Core\Math::atanh` is not exactly odd, and the two legs disagree about which rows it fails on.**
  Every other member satisfies `f(-$x) == 0.0 - f($x)` bit for bit, but `atanh` takes a logarithm of
  an asymmetric expression, so MSVC and glibc miss on different magnitudes and a parity sweep fails
  on the other leg. Assert a *relative* agreement for that one member (`abs($there + $back) <=
  abs($there) * 1.0e-15`, the infinite row by exact equality). [until: reviewed 2026-09-06]
- **`int as float` is checked and refuses past 2^53, not past `int`'s own range.** `nvs_runtime`'s
  `int_to_float` is `value.unsigned_abs() <= F64_EXACT_INT_LIMIT`, so `Core\Math::INT_MIN as float`
  throws `cannot convert `int` -9223372036854775808 to `float`` even though -2^63 is exactly
  representable. A case sweeping a `float`-typed member over an `int` table is bounded at
  ±9007199254740992; reaching `int`'s extremes on the float side needs a `float` *literal* (`0.0 -
  9223372036854775808.0`). [until: gone crates/nvs-runtime/src/helpers.rs:F64_EXACT_INT_LIMIT]
- **`Core\Math::cbrt` is not correctly rounded and the two legs disagree, so an *irrational* cube
  root's round trip cannot be frozen either way.** `cbrt(2.0)` cubed is exactly `2.0` under glibc
  and a last digit short of it under MSVC; `sqrt` is the only root member IEEE 754 requires to be
  correctly rounded, so only its inexact rows are the same on every platform. A *perfect* cube does
  round trip on both legs, because the answer is representable and every libm's final refinement
  lands on it — assert that half and not the irrational one. [until: reviewed 2026-09-06]
- **Float `<`, `>`, `&&` and `||` all lower, and `0.0 / 0.0` answers `NAN` rather than throwing, so
  a case can assert "near, not equal".** The `E0715` refusal is about strings and the other
  unordered domains; two `float`s compare for order fine. The tolerance spelling is `float $tol =
  0.000000000001 * ($mag + 1.0);` then `if (($d < $tol) && ($d > 0.0 - $tol))`, with the magnitude
  taken by hand because `Core\Math::abs` answers `int|float` and not a `float`; `-0.0` echoes as
  `-0` and is told from `0.0` through `1.0 / $x`. [until: reviewed 2026-09-06]
- **`php` inside WSL says whether a float landmark is exact on both legs, without a Linux build of
  `nvs`.** PHP calls the same libm Rust's `f64` methods do, so `wsl.exe -- bash -lc "php
  /mnt/<drive>/<repo>/.agent-tmp/rows.php"` answers whether glibc rounds it the way MSVC does.
  Domain endpoints and halvings (`acos(-1.0) == PI`, `tanh(20.0) == 1.0`) are safe equalities on
  both legs; the interior (`sin($x) * sin($x) + cos($x) * cos($x)`) needs the tolerance spelling.
  [until: reviewed 2026-09-06]
- **A float round trip has a well-conditioned direction and an ill-conditioned one; pick the
  direction rather than loosening the tolerance.** Composing a member with its inverse amplifies the
  inner answer's last digits where the outer member is steep, so one way round is host-independent
  and the other measures libm: `asinh(sinh($x))` and `acosh(cosh($x))` are contractions, while
  `atanh` amplifies by `1 / (1 - $y * $y)`, so that trip is `tanh(atanh($y))` over `(-1, 1)`.
  [until: reviewed 2026-09-06]
- **A spread argument lowers, so a case that composes a variadic member composes it rather than
  folding by hand.** `Core\Path::join(...Core\Path::split($p))` is how `path.rs`'s doc comment
  writes the round trip. A helper over the `array<string>` still fits when the *pieces* are the
  subject; inside it an array is indexed by the *string* of the offset (`$parts[$i as string]`), and
  an `int` counter compares against `Core\Arr::count($parts) as int`. [until: reviewed 2026-09-06]
- **`Core\Path::split` keeps `.` and `..` as elements, so `join` of its result is not the normal
  form.** Resolving them is `normalize`'s job alone, so the round trip is an identity only *through*
  the normal form and exact only on a path that is already normal. Assert
  `normalize(join(split($p))) == normalize($p)`, never `join(split($p)) == normalize($p)`, which
  fails on ordinary rows. [until: reviewed 2026-09-06]
- **A `!= null` guard does not re-type a nullable local for an *argument* position; `as string`
  inside the guarded branch does.** `rule:expressions/nullable-conversion`'s narrowing only lets
  `->` reach a member of a `?Foo`, so `Core\Str::replace($r, …)` inside `if ($r != null)` is `E0401:
  expected string, found string|null`, and declaring the local `?string` changes nothing. Write `$r
  as string` in the branch (a checked `rule:types/conversion`), or `$r ?? "<null>"` where the value
  is only echoed. [until: reviewed 2026-09-06]
- **A green conformance case can be pinning the bug you are about to fix.** A case that reaches for
  a construct incidentally (`$maybe["gone"] ?? …` as a way to spell a read) freezes whatever that
  construct did when it was written, and its `--EXPECT--` block is only as authoritative as that
  session. When a case goes red under a fix, check its expectation against PHP (`php -r '…'`) before
  adjusting either side. [until: reviewed 2026-09-06]
- **`python tools/loop.py --list` names the exact `.nvst` *filenames* a goal owes, and a case under
  another name does not count.** The owed name is also a *specification* — each clause of it is a
  row the case must have. Run `python tools/holes.py --item N` before writing, which prints the same
  names under "cases that may belong to it"; renaming afterwards is a `git mv` plus a re-run.
  [until: reviewed 2026-09-06]
- **A property default is a scalar literal or `[]`, and a *hooked* property takes none at all.**
  `public array<string> $rows = ["a"];` is `E0472`, and a `{ get => …; set { … } }` block after a
  default is a *parse* error that cascades, so the hook reads as broken syntax rather than as the
  illegal default before it. Seed an `array<T>` property in `constructor` from a typed local
  (`array<int> $seed = […]; $this->counts = $seed;`), as
  `tests/conformance/lang/every-write-spelling-agrees-on-a-refused-element-target.nvst` does.
  [until: reviewed 2026-09-06]
- **A parser refusal that yields `ExprKind::Error` doubles its own `--EXPECTF-ERROR--` block.**
  `Error` types as `mixed`, so every binding fed by one reports an `E0401` beside the refusal that
  caused it, once per site. Prefer handing the *operand* back in place of the refused prefix;
  `parse_unary` in `crates/nvs-syntax/src/parser/expr.rs` keeps `Error` only for a cast naming a
  type it cannot produce. Decide which a new refusal wants before writing the expected block.
  [until: reviewed 2026-09-06]
- **`lower_first_method` lowers `T`'s *first* method, so a lowering fixture puts the method under
  test first and its helpers after it.** Declaring the callee at the top, `.nvst`-style, snapshots
  the callee's own body — a `safepoint`, a `param` per declaration and a `return` — which reads as a
  lowering that produced nothing rather than as the wrong function. Reorder the two members.
  [until: gone crates/nvs-ir/src/lower/tests.rs:fn lower_first_method]
- **A named case a goal owes may be pinning a hole, not a landed feature, and the failure mode is a
  *missing* line rather than a wrong one.** The shapes run, exit 0 and print something plausible, so
  freezing what `nvs run` printed pins the divergence as the expectation — a conformance case takes
  no `--ORACLE--`, so nothing else checks it against PHP. Run the shapes in a scratch
  `.agent-tmp/*.nvs`, write the same program as `.php`, and diff the two before filling in
  `--EXPECT--`. [until: reviewed 2026-09-06]
- **An array literal's key arrow is `=>`, not the shape literal's `:`, and an enum declares its
  cases without PHP's `case` keyword.** `["a": 1]` and `case Off = 0;` are not near-misses but parse
  failures (`E0102`/`E0101`, `E0220`) repeated per line, none naming the spelling that works, and
  they hide whatever else the case asserts. `examples/arrays.nvs` has `["alpha" => 1]`;
  `tests/conformance/enum/an-enum-carries-negative-and-zero-cases.nvst` has `enum Level: int { Off =
  0, On = 1, }`. [until: reviewed 2026-09-06]
- **An `--EXPECTF-ERROR--` section is matched whole, not as a prefix.** A sweep that ends at its
  last `error[...]` line fails against a compiler that then prints `error: aborting due to N
  errors`. End the section with `%A` and that line, count included — a second assertion that no
  *extra* diagnostic crept in; `tests/conformance/lang/a-void-call-is-not-an-operand.nvst` has the
  shape and the conventions' skeleton does not. [until: reviewed 2026-09-06]
- **An inline-HTML run is assertable as a *value*, which lets a case count one rather than read
  it.** A run is a statement and a closure's block body is a statement list, so
  `Core\Out::capture(fn (): void => { ?>text<?nvs })` lowers, and the `Core\Cli\Text` it answers
  takes `as string`. `Core\Str::compare($t as string, "text")` is the comparison — the only way to
  put a raw span and an `echo` of the same literal side by side. [until: reviewed 2026-09-06]
- **The coverage gate matches a member's *fully qualified* call spelling, so a case written through
  `use Core\Test;` covers nothing it calls.** `crates/nvs-stdlib/tests/conformance_coverage.rs`
  greps `--FILE--` sections for `Class::member(` with the whole class name, so `Test::assertTrue(`
  after a `use` answers for nothing and `cargo test` fails with "N registered `Core` member(s) are
  never used by a conformance case". Write each new member once in the `Core\…` spelling somewhere
  in the case. [until: reviewed 2026-09-06]
- **A `--ORACLE-DIVERGES--` section is one line, and the runner refuses the case at the parse
  otherwise** (`line N: `--ORACLE-DIVERGES--` is one line`). The standing cases read as paragraphs
  because they are one very long line that a viewer wraps, so a divergence written as prose with
  blank lines looks exactly like them on disk and fails before anything runs. Write it as one line
  from the start. [until: gone crates/nvs-test/src/case.rs:`--ORACLE-DIVERGES--` is one line]
- **A `$` inside a double-quoted string interpolates, and the escape is `\$`.** A character class
  written out (`!#$%&'*+-/=?^_`) or a `printf` template (`"%1$s %2$d"`) has its `$…` read as a
  variable, so the diagnostics are `E0301`s about undeclared `$s` and `$d` that blame the fixture's
  subject. Escape it (`"!#\$%&…"`) or single-quote it (`'%1$s %2$d'` folds to the same
  `ConstArg::Str` and interpolates nothing); a backtick and an apostrophe need nothing inside double
  quotes. [until: reviewed 2026-09-06]
- **`echo` writes its operands one at a time, so a throwing call in the middle of one leaves half a
  line on stdout.** `try { echo "[", $t, "] port=", Show::port($t), "\n"; } catch (Throwable $e) {
  echo "[", $t, "] refused\n"; }` prints `[x] port=[x] refused`, which reads as a subtly wrong
  expectation, and freezing it pins the prefix of every refused row. Bind the call to a local inside
  the `try` and echo the whole line after it — or route each verdict through one helper returning a
  `string`, which also lets verdicts be counted over a corpus. [until: reviewed 2026-09-06]
- **A counting sweep adds `$b ? 1 : 0`, never `$b as int`.** `bool` converts to `string` and to
  `bool` alone (`E0708`, `rule:types/conversion`), so the *invariance over a sweep* shape spells its
  counter `$n = $n + (Core\Str::contains(…) ? 1 : 0);`. The rest of the shape works: an
  `array<Core\Time\Duration> $each = [0s, 1ns, …]` iterates with a typed `foreach` binding,
  `continue` skips a row a law does not apply to, and a bare `Core\X::member($arg);` is a legal
  statement when only the throw is wanted. [until: reviewed 2026-09-06]
- **A top-level element of an `array<mixed>` reads fine as a `mixed` argument, which is how a sweep
  over heterogeneous rows is written.** The refusal to index into an `array<mixed>`'s *elements* is
  about the second level only: `$lefts[$i as string]` handed straight to a `mixed` parameter lowers.
  So a table whose rows are an `int`, a `float`, an array and an object is parallel arrays
  (`array<string> $labels`, `array<mixed> $lefts`, `array<mixed> $rights`) plus a counter and a
  `public static function` that asks the members about one row. [until: reviewed 2026-09-06]
- **A closure is an object of a compiler-synthesized class, so it renders as that class's name.**
  `rule:types/closure-literal` makes a literal an object with one field per capture, so
  `Core\Test::assertSame($f, $g)` over two `callable`s prints ``a `Script$fn0` `` and
  ``a `Script$fn1` `` — a name no rule owns. Keep a `callable` out of a case about how a value
  renders; the same goes for `Tag::Unset`, which no file-scope expression produces at all.
  [until: reviewed 2026-09-06]
- **A `Core` member's numeric parameter is often `uint`, and a helper factoring a sweep has to
  declare it that way.** A literal `64` places as `uint` at the call site, so
  `Core\Str::repeat("36", 64)` compiles and hides the rule, but a count arriving through a helper
  parameter typed `int` is `E0401: expected uint, found int` at every call. Type the parameter
  `uint` (arithmetic on it stays `uint`); and since `Core\Str::length` *returns* `uint`, a loop
  counter fed from it wants `as int` or a `uint` of its own. [until: reviewed 2026-09-06]
- **A `Core` member whose parameter is an enum-case union will not take the whole enum.**
  `Core\Hash::hmac`'s third parameter is `rule:types/enum-case-type`'s
  `Core\Digest::Sha256|Core\Digest::Sha384|Core\Digest::Sha512`, not `Core\Digest`, so a helper
  forwarding a digest through a parameter typed `Core\Digest` is refused at the forward. Declare the
  union verbatim: it parses in a parameter position and widens to `Core\Digest` for the members
  taking the whole enum, so one helper forwards to both `hmac` and `Core\Hash::stream`.
  [until: reviewed 2026-09-06]
- **An enum case behind a `mixed` reads *falsy* when its backing integer is `0`; behind its declared
  type it is truthy.** `rule:types/literal-types` spends no representation on hiding the backing
  value and the runtime table dispatches on the tag, so the erased row diverges from what
  `rule:enums/truthiness` decided. Do not assert the erased row as if it were the rule's answer.
  [until: reviewed 2026-09-06]
- **A `!= null` narrowing does not survive into a loop body, so a nullable receiver is nullable
  again inside a `foreach`.** `if ($found != null) { … }` narrows, and `$found->group($key)` inside
  a `foreach` four lines below is `E0459` ("this receiver is nullable"); passing `$found` to a
  helper declaring `Core\Regex\Match` is `E0401: found null|Core\Regex\Match` from inside the loop
  and accepted outside it. Declare the helper's parameter `?Core\Regex\Match` and read it with `?->`
  plus `??`, rather than narrowing once at the top and trusting it. [until: reviewed 2026-09-06]
- **A case counts what a callback did through a captured *object*, since a closure captures by value
  and there is no `use (&$n)`.** `rule:types/implicit-capture` still shares an object at file scope:
  `final class Seen { public int $calls = 0; }`, `var $t = new Seen();`, then `fn (Core\Regex\Match
  $m): string => { $t->calls = $t->calls + 1; return "X"; }` handed to `Core\Regex::replaceWith`. A
  block-bodied `fn` declares its return type, and there is no `++` or `+=`.
  [until: reviewed 2026-09-06]
- **The handoff's named group may already be on disk under a filename that does not say so, and
  `gaps.py` will not tell you.** `gaps.py` counts *cases* per member, so a member three sweeps
  mention in passing still ranks thin while the property is already pinned. Before reading the
  implementation, `ls tests/conformance/core/ | grep -i <class>`, `sed -n '2p'` over every hit and
  read the case *bodies*; if the item is already answered, say so in the handoff.
  [until: reviewed 2026-09-06]
- **`Core\Str::slice`'s third argument is a *length*, not an end offset, and getting it wrong still
  counts plausibly.** `Core\Str::slice($alphabet, $i, $i + 1)` is "from `$i`, take `$i + 1`
  characters", so a counted assertion still comes out right while row 16 hands the decoder a
  seventeen-character operand. Spell it `Core\Str::slice($s, $i, 1)`, and have a counting sweep echo
  the *set* it counted as well as the count. [until: reviewed 2026-09-06]
- **A nesting sweep is spelled `array<array<array<mixed>>>`, and `flatten` cannot be iterated to a
  fixed point.** `Core\Arr::flatten`'s parameter is `array<array<T>>`, so `Core\Arr::flatten($x)`
  over its own `array<mixed>` answer is `E0401`. Write one step over many shapes:
  `array<array<array<mixed>>> $rows` with `foreach ($rows as array<array<mixed>> $row)` makes
  nestings data, and `Core\Str::countOf($json, "[") == 1` asserts an answer holds no nested array.
  [until: reviewed 2026-09-06]
- **`--EXPECT-ERROR--` means "this run must fail", so a program that exits 0 cannot assert its
  stderr.** `crates/nvs-test/src/run.rs` reports `expected the run to fail, and it succeeded` before
  comparing the stderr you wrote, which bites `Core\Debug::dump`, which writes only there. End the
  program with a deliberate `throw new RuntimeError(...)` and absorb the fatal report with a
  trailing `%A`. [until: gone crates/nvs-test/src/run.rs:expected the run to fail, and it succeeded]
- **A `.nvst` case cannot hold `Core` instances in an `array<mixed>`.** `$one as Core\Uri` on an
  element is `E0711` ("`rule:types/conversion` tabulates no conversion into an object"), so a sweep
  that builds many objects and then asks one question of each has no way back to the object. Collect
  the *answers* instead — `$built[] = $u->toString();` — and assert over the text.
  [until: reviewed 2026-09-06]
- **A `--EXPECT--` block cannot tell a composed `é` from a decomposed one, and the failure prints as
  two identical-looking blocks.** A subject built with `\u{301}` fails against an expectation typed
  as the composed character, and *expected* and *actual* render alike; only `od -c` shows it. Assert
  a decomposed cluster against a source-escaped literal (`… == "e\u{301}fac" ? "1" : "0"`), and keep
  the eyeball line on a subject with one spelling. [until: reviewed 2026-09-06]
- **A sweep over `Core\Json::decodeAs<T>` needs one helper per `T`, not one helper.** The class is
  written at the call site (`rule:core-api/shape-rules` R4; `WRITTEN_CLASS_MEMBERS`), so no
  parameter can carry it. Write one `try { … return true; } catch (Throwable $bad) { return false;
  }` per type over the same document builder and sweep the *documents*, as
  `tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst` does.
  [until: reviewed 2026-09-06]
- **`Core\ObjectSet`'s `union`, `intersect` and `diff` answer a set no declaration accepts, so a
  derived set is only *chained*.** Their `return_ty` is `CoreTy::Instance(NAME)`, no type argument,
  so `Core\ObjectSet<Tag> $u = $a->union($b);` is `E0401` and bare `Core\ObjectSet $u` is `E0442`.
  Write every law as one chain (`$a->diff($b)->union($a->intersect($b))->diff($a)->count() == 0`);
  `Core\ObjectMap` is the same.
  [until: gone crates/nvs-stdlib/src/objset.rs:return_ty: CoreTy::Instance(NAME)]
- **`Core\Uri::parse` refuses a stray `%`, so `normalized`'s malformed-escape branch is owed no
  case.** The module doc's "two references that both wrote the same stray `%` are still the same
  reference" reads as behaviour a `compareTo` case can pin, but `parse` throws on `http://h/50%`, so
  no program can build such a `Uri` and the branch is defensive against one built some other way.
  Check what `parse` admits before writing a row about what a normalizer keeps.
  [until: reviewed 2026-09-06]
- **A mechanical sweep over the `.nvst` corpus has to treat each `--SECTION--` as its own program.**
  A whole-file answer to "is this name used elsewhere" keeps a differential case's Novis half in the
  old spelling because its PHP `--ORACLE--` twin mentions the same `$i`, and licenses an edit in one
  section on the strength of a use in another. Split on `^--[A-Z]` and scope both the backward
  search for the declaration (a stacked run of declarations is the common shape, so not the previous
  line only) and the "used elsewhere" check to the enclosing section. [until: reviewed 2026-09-06]
- **An interface method needs `public` in a `.nvst` case but not in a `nvs-types` unit fixture.**
  `crates/nvs-types/tests/common`'s `check_src` runs `parse` → `resolve` → `check_program` and
  nothing else; the casing/visibility pass `nvs-cli`'s `front_end` runs
  (`crates/nvs-syntax/src/casing.rs`, `E0122`) is not in it. So `interface Labelled { function
  label(): string; }` is a fine unit fixture and a broken case file, and the fix is one keyword
  rather than a hunt. [until: reviewed 2026-09-06]
- **A nested `is` guard *replaces* the residue rather than intersecting with it.** There is
  no intersection type, so inside `if ($v is Labelled) { if ($v is Counted) { … } }`
  the subject is a `Counted` and nothing else, and `$v->label()` there is `E0405: `Counted` has no
  method named `label``, pointing at the inner interface for a member the outer guard proved. Read
  what the outer guard bought into a local before writing the second test, as
  `tests/conformance/lang/an-is-guard-narrows-to-an-interface.nvst` does.
  [until: reviewed 2026-09-06]
- **Two sort keys that look different usually agree, and a case that does not separate them pins
  nothing.** `rule:programs/implementing`'s `implementors` sorts by `QName::segments()`, and the
  obvious counter-example to a rendered-string sort agrees; they part only where one segment is a
  proper *prefix* of the other and the longer one's next byte is below `\` (0x5C): `App\Sub\A`
  against `App\SubA`. Work the divergence out on paper before writing the fixture.
  [until: reviewed 2026-09-06]
- **`use Core;` does not place a bare `#[Command]`, and the failure reads as if the roster edit did
  not land.** A `use` aliases one *name*, so with only `use Core;` in scope the attribute resolves
  to `\Command` and the answer is `E0303: `Command` is not declared` — the ordinary undeclared-name
  refusal, pointing at the wrong file. Import it as `use Core\Command;` exactly as `#[Test]`'s is
  `use Core\Test;`, or write `#[\Core\Command(...)]` fully qualified when the case is about the
  match rather than the import. [until: reviewed 2026-09-06]
- **A new compile-time refusal can break a green `.nvst` written for the *runtime* half of the same
  rule sentence.** `rule:routing/an-absolute-link-takes-a-configured-origin` makes a leftover
  `$params` key the query string *and* an undeclared one a compile error; landing the second half
  turned `tests/conformance/core/router-url-turns-a-leftover-params-key-into-a-query-string.nvst`
  red with `E0759`. Before adding a refusal, `grep -rl` the corpus for what it refuses.
  [until: reviewed 2026-09-06]
- **A new compiler rule that every existing fixture violates is one helper edit and some line
  numbers, not N rewrites.** `crates/nvs-types/tests/routes.rs` builds fixtures through one
  `route_src` helper, so `rule:attributes/access-is-a-required-sibling` landed there
  (`with_access`). A `.nvst` is out of a helper's reach: `--EXPECTF-ERROR--` quotes *source line
  numbers*, so a line inserted into `--FILE--` shifts every one below it, and a `reject/` case must
  still fail for its *own* reason. [until: reviewed 2026-09-06]
- **Putting a member on the intrinsic list breaks every conformance case that made it throw from a
  *literal*.** Once the checker reads it (`rule:expressions/intrinsic-list-is-closed`), the runtime
  throw the case caught becomes `E0769`/`E0770`, reported as `standard output does not match /
  actual: <empty>`. Bind the pattern to a `string $name = "…";` and pass that, and `grep` the
  member's spelling in `tests/conformance/` *before* adding the arm. [until: reviewed 2026-09-06]
- **A case can *look* like it exercises the intrinsic-literal fold and exercise nothing: the list
  names the member that reads the pattern, not its siblings.** `rule:expressions/intrinsic-literals`
  lists `Core\Regex::compile`, not `matches`, so `Core\Regex::matches($s, "^order-\\d+$")` only
  checks that two runtime calls agree. Check the line against `INTRINSICS` in
  `crates/nvs-types/src/intrinsics.rs`, and prove it: a deliberate error in the same text is an
  `E0769` from `nvs check`. [until: reviewed 2026-09-06]
- **A fixture that needs a route table does not need an `autoload` root.** The table is collected
  over every declaration the program checks, so a class declared in the entry file lands in it —
  `crates/nvs-cli/tests/fixtures/api/base.nvs` is one file with a class and an `echo`, and `nvs
  build --openapi` emits both its operations. `examples/routes.nvs` splits across a root to
  demonstrate `rule:packaging/autoload-probes-fold-into-the-cache-key`'s scan, not because an
  emitter fixture has to. [until: reviewed 2026-09-06]
- **A test helper that names a file in `CARGO_TARGET_TMPDIR` after its *input* races the other tests
  that ask for the same input.** Tests run in parallel, so a reader catching another thread's
  `fs::write` half-done sees a truncated document: intermittently, and never under `cargo test --
  <one test name>`. Run the *whole* binary on the stashed tree; the fix is a per-call counter in the
  name (`crates/nvs-cli/tests/openapi.rs`'s `document`). [until: reviewed 2026-09-06]
- **The `unreachable from source` phrase is counted from the line the `Fault::` sits on, not from
  its statement.** The gate wants it within 8 lines, so a builder chain between the comment and the
  `Fault::fatal` pushes it out of `DECLARATION_WINDOW`. Put the phrase on the comment's last line
  and run `cargo test --test conformance_coverage every_error_path` after a batch.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
- **A reachable `Fault::fatal` is pinned with `--EXPECT-ERROR--`, and the program's own
  `try`/`catch` around it is worth writing anyway.** `FATAL:` goes to standard error and the process
  stops there, so `--EXPECT--` holds only what printed *before* it: every other row must come first,
  or a fatal in the middle truncates them. Writing the call inside a `try` with an `echo
  "unreachable"` in the `catch` makes the *uncatchable* half an assertion.
  [until: reviewed 2026-09-06]
- **A stem that has to reach `conformance_coverage.rs`'s corpus needs `Core\Test` with one
  backslash, which a Novis string literal will not give you.** The gate reads every case file as raw
  text and looks for the site's message stem literally; a `"Core\\Test::…"` written in the program
  lands as two backslashes and matches nothing, while an `--EXPECT--` line is plain text and carries
  it. Echo the message (or `Core\Str::slice` of its opening) and let the expectation hold the stem.
  [until: reviewed 2026-09-06]
- **`Core\Attributes` retrieval reads any *recognized* attribute's payload, and `get<T>` answers
  only the first of two on one method.** `rule:attributes/structural-retrieval` is structural, so
  `Core\Attributes::get<{name: string, about: string}>(Deploy::deploy(...))` answers a
  `#[Core\Command]`; read an alias with `all<T>` and index it. A payload-less `#[Option]` is
  indistinguishable from no attribute. [until: reviewed 2026-09-06]
- **An `--ORACLE--` helper named after a PHP built-in is a fatal, and the runner reports it as a
  *case* failure.** A `function pos($v)` in the oracle half is `Cannot redeclare function pos()`,
  because `pos()` is `current()`'s alias and PHP has ~1,900 globals; the runner prints `PHP exited
  255` plus the stderr. Prefix every oracle helper with `php` (`phpAfter`, `phpLines`, `phpSort`) as
  the `str-before-and-after` and `arr-*` cases do, and keep the un-prefixed spellings for the Novis
  side. [until: reviewed 2026-09-06]
- **A bare array literal is `array<mixed>` wherever it sits inline, a `Core\Arr` member's second
  argument or a `foreach` header, and does not narrow.** `Core\Arr::diff($a, [4, 2])` is `E0401:
  expected array<int>, found array<mixed>` at the literal, and `foreach ([1, 2, 3] as int $n)` is
  `E0401: expected 'int', found 'mixed'` pointing at the *binding*. Declare a typed local on the
  line above (`array<int> $against = [4, 2];`) and use that. [until: reviewed 2026-09-06]
- **Widening a union of enum-case types rewrites every `E0401` that names it, and a
  `--EXPECTF-ERROR--` case has the whole list frozen in it.** The accepted set is generated from the
  parameter's type, so a refusal case
  (`tests/conformance/core/hash-hmac-refuses-a-weak-digest.nvst`) is a *roster* case, and `%A` never
  covers the message. Before adding a case to a `CoreTy::EnumCase` union, `grep -rn "<the union's
  first case>" tests/conformance/` and update every hit. [until: reviewed 2026-09-06]
- **`Core\Test::assertSame`'s two parameters are one `CoreTy::Var("T")` bound to argument 1, so
  `assertSame(null, $x)` does not exist.** Writing the literal first binds `T` to `null`, and the
  subject is then `E0401: expected 'null', found 'mixed'` at the *second* argument, which reads as
  "this member refuses a null comparison" and is only the signature. `mixed $nothing = null;` then
  `assertSame($nothing, $subject)` is the only way to ask identity's symmetry through this member.
  [until: reviewed 2026-09-06]
- **`target/debug/nvs test <one-case.nvst>` runs a single case and prints its actual stdout on a
  mismatch**, so freeze an `--EXPECT--` by running the case with an *empty* one and pasting back
  what it printed. The report indents the output by four spaces, so de-indent the paste; and a value
  that can be empty or end in a space needs a delimiter in the `echo` (`"[", $x, "]"`), because a
  trailing space is invisible there and exact in the comparison. [until: reviewed 2026-09-06]
- **A `.nvst` case's bindings are script-scoped, `foreach` and `catch` bindings included, so a
  second sweep cannot reuse the first's names at another type.** `foreach ($whole as int $n)` after
  `foreach ($reals as float $n)` is `E0406: … is already declared`, pointing at a line far above;
  the *same* type is fine. Name each binding for what it holds (`$i` for the integer rows), or hoist
  the body into a `public static function` that owns the names. [until: reviewed 2026-09-06]
- **A `Core\Time\TimeOfDay`'s four slots are not readable as properties, and the bottom of each
  field is a *checker* refusal, not a runtime one.** `$t->hour` is `E0405: `Core\Time\TimeOfDay` has
  no property named `hour``, so a case reads a field back out of `format` (`$t->format("HH") as
  int`, `format("SSSSSSSSS")` for nanoseconds). `at`'s parameters are `uint`, so `at(-1, 0)` is
  `E0401` and never reaches the member: only the top bound has a runtime half.
  [until: reviewed 2026-09-06]
- **A `Core\Time\Date` reaches further at both ends than a `DateTime`, so a range-ends sweep builds
  the two halves differently.** `Core\Time\Date::at(9999, 12, 31)` and `at(-9999, 1, 1)` pass while
  `Core\Time::at(9999, 12, 31, $utc)` and `at(-9999, 1, 2, $utc)` throw, because jiff's timestamp
  range stops inside the calendar's last day and first two; the refusal says the conversion
  overflowed. Read a year's length from March rather than from either end.
  [until: reviewed 2026-09-06]
- **`as` binds tighter than arithmetic, and `false as string` is the empty string.** `$which[$i - 1
  as string]` parses as `$which[$i - (1 as string)]` and fails `E0716: '-' has no meaning for
  'string'`, so a computed index needs its own parentheses, `($i - 1) as string`. A `bool` printed
  with `as string` renders `1` for true and *nothing* for false, so a column of booleans in
  `--EXPECT--` silently changes width and passes review; `$b ? "y" : "n"` keeps such a row legible.
  [until: reviewed 2026-09-06]
- **A `.nvst` case configures its run through `--FILE nvs.toml--`, and a scratch probe at the repo
  root does not.** `crates/nvs-test/src/run.rs` runs every case in a temp directory with each
  `--FILE <path>--` section written into it. The repo's `nvs.toml` sets `origin =
  "https://example.test"`, so `nvs run scratch.nvs` from the root answers `Core\Router::urlAbsolute`
  and proves nothing. Probe from a scratch directory carrying the case's `nvs.toml`.
  [until: reviewed 2026-09-06]
- **An array subscript keyed by `bytes` or by a `string|int` union is an ICE in `nvs-ir`, not a
  diagnostic.** `lower_array_key` has arms for `Str`, `Int` and `Uint` only, so `$seen[$k] = true;`
  over a `string|int $k` the checker admitted panics ("an array key lowered to …"). Write `$seen[$k
  as string]`, the spelling key normalization maps an integer onto; the fix proper is a `nvs-types`
  diagnostic. [until: gone crates/nvs-ir/src/lower/expr.rs:an array key lowered to]
- **A `--EXPECT--` is byte-exact, and a loop that echoes its separator *after* each item leaves a
  trailing space nothing shows you.** `echo $a, "/", $b, " ";` inside a `foreach` costs a run: the
  expected block cannot carry a trailing space (an editor or a hook strips it, and the diff prints
  identically on both sides). Collect the rows into an `array<string>` and echo
  `Core\Str::join($rows, " ")`; a fold member's `int|float|decimal` answer concatenates with `.`
  even where `as string` on that union does not. [until: reviewed 2026-09-06]
- **A zero-width match right after a non-empty one is dropped by all four `Core\Regex` iterating
  members.** `replace`, `replaceWith`, `matchAll` and `split` use the `regex` crate's
  `captures_iter`/`split`, which skip it, PCRE does not: `Core\Regex::replace("ab", "b*", "-")` is
  `-a-`, PHP says `-a--`. Only an oracle sees it, so a differential row must not put a zero-width
  match after a wide one. [until: gone crates/nvs-stdlib/src/regex.rs:captures_iter]
- **`Ctx::take_pending` answers an empty string for a *throw* in a hand-built context, so a
  bare-harness test cannot assert on a thrown message.** `Ctx::buffered()` installs no runtime error
  class and `Thrown::message()` returns `String::new()` when its object is null, so the member seems
  to have said nothing. Assert the `rule:errors/propagation` *status* (`THROWN` against `FATAL`);
  `benches/abi-probe/tests/invariants.rs`'s `decode_on_this_stack` is the shape.
  [until: reviewed 2026-09-06]
- **When `run_until_idle` returns with tasks still parked, suspect the turn's "nothing woke" exit
  before a lost wake.** One drain takes the queued ids of *several* pokes at once, so surplus pokes
  come back ready over an empty queue, and `Reactor::turn` reporting `0` woken reads as idle. A test
  that runs wide is worth writing over landed behaviour;
  `a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count` in
  `crates/nvs-host/src/blocking.rs` is the example. [until: reviewed 2026-09-06]
- **`Timers::publish` bumps a deadline equal to its base by a nanosecond, and two adjacent
  `Instant::now()` calls can be equal.** A test that arms at `let start = Instant::now()` on a
  `Timers` built the line before and sweeps at exactly `start + margin` is a nanosecond short of
  overdue and reports nothing, only under load. Arm strictly after the base whenever the sweep
  instant derives from the armed one. [until: gone crates/nvs-host/src/timer.rs:.max(NOTHING + 1)]
- **A frozen `--EXPECTF-ERROR--` block's line numbers are read off the runner, not counted by
  hand.** `--FILE--`'s first line, the `<?nvs`, is *not* `case.nvs:1` — the line after it is, so the
  number is the `.nvst` line minus one more than the header, and an off-by-one lands the caret on the
  statement above the one that is wrong. Write the block with any plausible numbers and run `target/debug/nvs test
  tests/conformance/core --filter <slug>` once — it prints the real `case.nvs:NN:CC` in the actual
  half, and columns do not shift with the header. [until: reviewed 2026-09-06]
- **A `nvs-host` test makes a context *fail* with `Ctx::set_pending("message")`, not with
  `Thrown::new`.** `Thrown::new` is `unsafe` and wants a `*const ClassDesc`; `set_pending` takes a
  bare message and `take_thrown` promotes it through the context's error class. A *named* failure
  needs one installed first (`ClassTable::define("Throwable", &["message", "previous", "backtrace",
  "location"], &[])` plus `set_runtime_error_class`); without one `take_thrown` answers a null
  `Thrown`. [until: reviewed 2026-09-06]
- **`await` is contextual, so `await 5` does not parse and a bare `await $handle;` statement is not
  an `await` either.** A literal after it leaves `await` an identifier and the parse fails with
  `E0101 expected ';'` past the literal; in statement position it reads as a type name, so the case
  reports "`$handle` is already declared". Bind either side: `int $n = 5;` then `await $n`, and `var
  $ignored = await $handle;`; `spawn script …;` as a statement is fine. [until: reviewed 2026-09-06]
- **A `.nvst` case runs in a temp working directory, so a relative path resolves against *that*, and
  a case asserting a path *fails* passes either way.** A spawn case naming
  `"examples/isolate/capture.nvs"` was green because the file was missing there too. A case that
  needs a second file writes it as a `--FILE <name>--` section beside `--FILE--`: forward-slash
  relative paths, not `crates/nvs-test`'s `RESERVED_NAMES`. [until: reviewed 2026-09-06]
- **A new `Core` member owes `conformance_coverage.rs` three things, reported one gate at a time,
  after `cargo test` is otherwise green.** One case naming it, *three* cases naming it
  (`every_core_class_has_a_conformance_floor_of_three` is per member, not per class), and every
  `Fault::` message stem in the corpus or `unreachable from source` within 8 lines of the site.
  Where a `.nvst` cannot reach the accepted path, all three cases ask about the refusal.
  [until: reviewed 2026-09-06]
- **PHP's spellings of a constructor and of an untyped array local do not compile in a `.nvst`
  case.** The constructor is `constructor`, not `__construct` (E0114, then E0409 for every property
  it would have assigned), and `var $x = [1, 2, 3];` is E0414 because an array literal has no target
  type to infer from. Write `array<int> $x = [1, 2, 3];` and `public function constructor(...)`.
  [until: reviewed 2026-09-06]
- **A `Qual` classification is enforced on the `tainted` axis only, so a `secret` claim has to be
  probed first.** `nvs_types::expr::quals::admits_tainted_argument` reads a parameter's mark for a
  tainted argument; nothing reads it for `secret`, which is refused at every mark exactly as an
  unclassified parameter refuses it. Probe with a scratch `.nvs` under `.agent-tmp/` run through
  `./target/debug/nvs.exe run <path>`, which prints every diagnostic for every line at once.
  [until: reviewed 2026-09-06]
- **`as array<SomeClass>` is refused with `E0711`; convert to `array<mixed>` and convert each
  element where it is read.** `rule:types/conversion`'s `array<T> as array<U>` row checks every
  element by its runtime tag, which a class is not decided by, so it surfaces once a container of
  objects rounds through something answering `mixed` such as `Core\Serialize::decode`. The spellings
  that compile are `... as array<mixed>` then `$copy[0] as Cell`, and nested, `($outer[0] as
  array<mixed>)[0] as Cell`. [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0711")]
- **A `.nvst` case can carry its own `nvs.toml`, so a changed config spelling breaks tests a `grep`
  over `crates/` misses.** `--FILE nvs.toml--` writes one into the case's working directory, and the
  `router-url-absolute-*.nvst` cases do exactly that, one pinning the diagnostic that names the key
  in `--EXPECT--`. When you move a spelling that appears in a config file or a diagnostic, `grep
  -rl` over `tests/` and `examples/` in the same call as `crates/`; the message text is pinned both
  in the crate's `Fault` and in the case that catches it. [until: reviewed 2026-09-06]
- **A relative path in an included configuration file resolves against that file's own directory,
  `[[app]] root` included.**
  `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` keys `root =
  "srv/www/shop"` written in `conf.d/shop.toml` on `conf.d/srv/www/shop`, and the refusal is `E0605
  cannot read`, which reads as a broken fixture. Write the `..` the operator would have to write.
  [until: reviewed 2026-09-06]
- **An invalid `--FILE nvs.toml--` fails the whole case before the program starts: `E0601`/`E0609`
  on stderr, empty stdout.** `boot_snapshot` reads it for real, and
  `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s typed tree refuses an
  `[[app]]` block with no `root` or `entry`, a root-level `origin` and any unknown key. `entry =
  "nvs.toml"` is a legal key matching no program, the shape a decoy block wants.
  [until: reviewed 2026-09-06]
- **A path rule cannot be tested against a real symlink on Windows; `nvs-config`'s `Files` trait is
  the seam instead.** Creating a symlink needs a privilege CI does not have, so
  `a_path_reaching_a_granted_root_through_dotdot_or_a_symlink_does_not_match` runs its `..` half
  against `resolve::Disk` and the real filesystem, and its symlink half against a fake whose
  `canonical` maps one path to another. `crates/nvs-stdlib/tests/capability.rs`'s `Fake` is the
  shape; the methods it must never reach are `unreachable!()` with a sentence saying why.
  [until: reviewed 2026-09-06]
- **`python tools/try.py` does not reproduce a multi-file case's working directory, so a
  sibling-file read fails under it.** It copies the `--FILE--` body to `.agent-tmp/` and runs that
  alone. Check a multi-file case with `target/debug/nvs test tests/conformance/<tree>` (a single
  `.nvst` path works too), which is what `verify.py` runs; `try.py` is for the single-file shape.
  [until: reviewed 2026-09-06]
- **`Core\Config::restore` in a `.nvst` case can end it with a `FATAL`: the ceiling drops while the
  raised allowance is live.** A case that raises its `[limits]` ceiling, allocates and restores
  breaches at the next helper call; drop the ballast before restoring, since a request cannot
  un-allocate by lowering its own limit. A case may carry `--EXPECT--` and `--EXPECTF-ERROR--`
  together, so both the stdout before a `FATAL` and the `FATAL` line are pinnable.
  [until: reviewed 2026-09-06]
- **A closure captures by value, so a `.nvst` case cannot count anything by incrementing a captured
  variable.** `var $seen = 0; var $f = fn (): void => { $seen = $seen + 1; };` compiles, runs and
  leaves `$seen` at `0` however often `$f()` is called, and the case fails on a line that looks like
  a bug in the member under test. `echo` from inside the closure, or count on the outside.
  [until: reviewed 2026-09-06]
- **A `loop-goal.toml` check naming a crate with no `tests/` directory is not misfiled; create the
  integration test there.** A package's ordinary `[dependencies]` are on the extern list of its test
  targets, so a new `crates/nvs-host/tests/limits.rs` can `use nvs_runtime::{Ctx, nvs_safepoint}`
  and drive compiled code with no `dev-dependencies` edit. The question is whether the crate can
  reach the thing the test asks about, never whether a test like it already lives there.
  [until: reviewed 2026-09-06]
- **A `[limits]` reader answers off `Snapshot::table`, not the typed `Config` beside it.**
  `Ctx::configured_memory_limit` reads `self.base.table` through `Request::get`, so `Snapshot {
  config: Config { limits: Some(..), .. }, .. }` reports no ceiling. Build both halves from one TOML
  string (`parse::<toml::Table>()`, then `table.clone().try_into()`) as
  `crates/nvs-runtime/tests/configured_limits.rs` does.
  [until: gone crates/nvs-config/src/snapshot.rs:pub table: toml::Table]
- **A `static` a test handler records into is shared by every test in the binary, and cargo runs
  them on separate threads.** A second test registering a handler that writes the same slot turns a
  `take()` from either reader into a `None` for the other, in a case that passes alone under
  `--test-threads=1`. Compiled code is called through a bare `extern "C"` pointer that captures
  nothing, so the fix is a slot and a recorder per test (`crates/nvs-host/tests/limits.rs`'s
  `SEEN_LIMIT` and its twin), not a lock held longer. [until: reviewed 2026-09-06]
- **A capability is invisible through the request overlay, so narrowing one via `Core\Config::set`
  pins nothing.** `capability::refusal` reads `config.snapshot().config.capabilities`, never the
  `Request` overlay, and `Request::set` refuses the block anyway: a grant is a list, with no
  quantity to compare. Narrow a capability in the snapshot, as
  `a_child_cannot_widen_a_capability_its_parent_narrowed` does.
  [until: gone crates/nvs-config/src/directive.rs:"capabilities", class: Class::RuntimeTighten]
- **A `cfg(unix)`-only test cannot satisfy a `cargo`-named acceptance check; the driver reads "did
  not run" as a failure.** A world-writable directory exists on Windows in one line, `icacls <dir>
  /grant *S-1-1-0:(OI)(CI)(M)`, which `nvs_config::trust::check` sees through
  `GetEffectiveRightsFromAclW`; spell the principal as the SID because `icacls` is localized. Assert
  on `Untrusted::Breach` and on the directory's last component, not the canonical path, which is
  `\\?\C:\...` on Windows (`a_world_writable_cache_directory_is_refused` is the shape).
  [until: reviewed 2026-09-06]
- **A path helper that normalizes `..` defangs the very escape the case was written to assert.**
  `crates/nvs-config/tests/request.rs`'s `p()` resolves `.` and `..` away, so a `..` case built with
  it hands `Capabilities::allows` a path that has already escaped and passes on the sibling-root
  rule rather than on `rule:security/path-scope-canonicalise-then-prefix`.
  `crates/nvs-config/tests/capability.rs` keeps `raw()` (what the caller wrote) and `lexical()`
  (what the filesystem answers) apart; only the fake `Files` may turn one into the other.
  [until: reviewed 2026-09-06]
- **An inline array literal is `array<mixed>`, so a typed `foreach` binding or a generic `T` bound
  from one fails.** `rule:types/conversion` leaves an untyped literal unplaced, so the `foreach`
  binding is `E0401` at the binding and `string $one = Core\Cli::select("q", ["a", "b"])` is `E0401:
  expected string, found mixed` because `crate::generics::bind` reads `array<mixed>`. Name the
  array's type where it is built (`array<string> $choices = [...]`), not where it is walked or
  passed. [until: reviewed 2026-09-06]
- **A `--RUN-- test` case resolves no configuration tree, so `Core\Config` is empty inside every
  `#[Test]`.** `crates/nvs-cli/src/runner.rs` builds a bare `nvs_runtime::Ctx::stdout()` and only
  `nvs run` resolves `./nvs.toml`, so a `--FILE nvs.toml--` beside a `--RUN-- test` case is written
  and read by nobody, and `Core\Config::get` is `null`. Two sequential `spawn script` children do
  see the configuration:
  `tests/conformance/config/config-set-is-invisible-to-the-next-request.nvst`.
  [until: gone crates/nvs-cli/src/runner.rs:nvs_runtime::Ctx::stdout()]
- **A memory-limit breach is observed at the next loop back edge or the next member call, whichever
  the program reaches first, and an `$a[] = ...` on its own is neither.** The allocator raises
  `SafepointFlags::MEMORY_LIMIT` at the crossing, so a fixture that grows inside a `while` stops
  inside it; straight-line growth reaches nothing until `run_helper` asks ahead of a member's body.
  Put the loop or the member call where you want the breach reported, and expect nothing printed
  after the crossing. [until: gone crates/nvs-runtime/src/budget.rs:fn publish]
- **A `reject` case pins the *first* diagnostic, and the recovery type behind it writes a second
  one.** `check_read` answers `mixed` after reporting, so a `static` method declared `: int` whose
  body reads `$this->size` also reports `E0403 declares int but returns mixed`, and `%A` does not
  cover the trailing `aborting due to 2 errors`. Declare the surrounding position `mixed`
  (`this-is-not-read-in-a-static-method.nvst`) or pin both errors deliberately; `python tools/try.py
  <case>.nvst` prints the exact block to freeze, reading the `.nvst` in place.
  [until: reviewed 2026-09-06]
- **A `--FILE--` body starts at byte 0 of the written file with no leading newline, which an
  offset-0 case relies on.** `crates/nvs-test/src/case.rs`'s section reader takes the lines after
  the header verbatim, so `a-shebang-line-opens-code.nvst` hands the lexer `#!` at offset 0. Had the
  harness kept the separator's newline, a case about the first bytes of a file
  (`rule:tooling/shebang-opens-code-mode`) would pass while testing nothing; know this before
  writing one. [until: reviewed 2026-09-06]
- **A `namespace` in Novis is a statement, not a block, so a case that needs two of them needs two
  files.** `namespace App { ... }` is `E0243`, and declarations cross a `require`
  (`rule:statements/require-is-the-only-inclusion-construct`), so the shape is a `--FILE app.nvs--`
  plus a `require './app.nvs';` at the top of the root file, whose statements alone print. `try.py`
  cannot run it, but `target/debug/nvs test <path>.nvst` takes a single case path and reproduces its
  working directory. [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0243")]
- **The conformance floor reads only a case's main `--FILE--`, so a member called only in an
  auxiliary file counts for nothing.** `crates/nvs-stdlib/tests/corpus/mod.rs`'s `sources()` takes
  the last, unnamed section deliberately, so a member only a child isolate can call, written into
  three `--FILE child.nvs--` sections, still reads as zero cases. Close it with a question the
  *parent* asks in its own section, such as relaying the child's answer back through `args:`.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:The section alone]
- **A new `Core` member owes three conformance cases, not one, and the second gate says so late.**
  `every_part_one_member_has_a_conformance_case` wants one;
  `every_core_class_has_a_conformance_floor_of_three` counts distinct case files per member, and a
  repeated question does not. Budget three shapes, run `cargo test --test
  conformance_coverage` first, and never add to `BELOW_THE_FLOOR`.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const FLOOR: usize = 3]
- **A `Core\Fatal::onLimit` handler runs only when the breach lands inside the reserve, so a large
  overshoot prints nothing.** `Ctx::run_limit_handler` adds the reserve back to the reduced ceiling,
  so a request already holding more than `[limits] memory` breaches again at the handler's first
  helper call (`echo` is one) and is abandoned without a word; the `FATAL` message is identical
  either way. Size the ballast to land between `memory` minus the reserve and `memory`.
  [until: reviewed 2026-09-06]
- **A subclass fixture with its own constructor must call `parent::constructor(...)` on every path,
  or `E0410` fails it.** A `class Dog extends Animal { public function constructor(string $n, int
  $age) {} }` written to make `rule:classes/constructor-compatibility`'s case plausible is itself
  refused, so an `assert!(!diags.has_errors())` fails on a diagnostic the author never considered,
  worst in a negative fixture's control half. One `parent::constructor($n);` line fixes it.
  [until: reviewed 2026-09-06]
- **`gaps.py`'s two sections measure two different suites, and an item that mixes them asks for a
  case that already exists.** The *conformance depth by class* block is `tests/conformance/`; the
  *differential gap* block is `tests/differential/`, and a member at the depth floor in the first
  may already have oracle cases in the second. Only the differential block's roster ("a PHP twin and
  no oracle case") answers a differential item; one `ls tests/differential/core/ | grep <class>`
  settles it. [until: gone tools/gaps.py]
- **Rewording a `Fault::` message or a diagnostic's help breaks cases pinning a substring of it;
  `cargo test` never runs them.** An `--EXPECT--` pins the line, an invariance sweep asserts
  `Core\Str::contains($message, "at offset " . $j . " is")` and prints `0 of 8`, and a differential
  helper asserts `Core\Str::startsWith($message, ...)` and prints a column of `|`, so the failure
  waits for `verify.py`. Before changing it, `grep -rn` the corpus under `tests/` for the head, a
  distinctive interior phrase, and `startsWith`. [until: reviewed 2026-09-06]
- **`echo "status: ", Core\Command::run()` prints the label before whatever the callee echoes.**
  `echo`'s arguments are written as they are evaluated, so a member that produces output inside the
  call interleaves with the text around it and the expectation reads `status: greet: Hello...`,
  which looks like a matcher bug. Take the value into a variable first; every dispatching or
  callback-taking member has this shape. [until: reviewed 2026-09-06]
- **A `-p nvs-stdlib` test can hand a `Core` member a compiled class, and the installation point is
  named for something else.** `Ctx::class_desc`, the one route from a native member to a program's
  class, reads the table through `Ctx::set_runtime_error_class`'s handle and nothing else, so a
  dispatch test installs an `ErrorClass::new(Rc::new(table), id)` over an unrelated class. The row
  is `MethodRow { arity }` excluding the receiver and slot 0 is the called class as a
  `Value::class_desc`; `crates/nvs-stdlib/src/command.rs`'s `dispatching` is the shape.
  [until: reviewed 2026-09-06]
- **A `.nvst` case's program name is `case`, and that is stable enough to freeze in an
  `--EXPECT--`.** `nvs-test` writes every case as `case.nvs` and runs `nvs run case.nvs`
  (`crates/nvs-test/src/run.rs`), so anything reading `nvs_runtime::Ctx::program_name`, such as
  `Core\Command::completions`, answers `case` in a conformance case and the file's own stem
  everywhere else. It is neither the `.nvst` file's name nor `nvs`; guessing either produces a diff
  that looks like a bug in the member. [until: gone crates/nvs-test/src/run.rs:"case.nvs"]
- **An error path only a non-`nvs run` context reaches still owes a case; `--RUN--` reaches it.**
  The gate wants the message cased, and `Core\Command::completions`'s refusal is reachable, not from
  a command line. `--RUN--\ntest\n` runs the file's `#[Test]` methods, so the case is a `#[Test]`
  echoing `$e->message`; an empty `--EXPECTF-ERROR--` claims the run failed.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION: &str]
- **Every `Fault::thrown` site owes a case printing its message, or an "unreachable from source"
  declaration.** `every_error_path_is_asserted_or_declared_unreachable` greps each message's stem
  across every case file, so a message that interpolates a dependency's text cannot be asserted.
  Give a refusal a fixed sentence, and satisfy the gate with `catch (LogicError $e) { echo
  $e->message, "\n"; }`.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION: &str]
- **The error-path gate reads eight lines above the `Fault::` token and stops at the first, so one
  comment cannot cover two.** A block comment above `let x = f().map_err(|_| { Fault::thrown(...)
  })?;` covers nothing if "unreachable from source" sits more than `DECLARATION_WINDOW` lines above
  the token (the closure's own lines count), and a second site further down never sees it. Put one
  short declaration immediately above each site, each ending with its own reason.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION_WINDOW]
- **A `?T` a `Core` member answered cannot go straight back into a `T` parameter, and the diagnostic
  lands at the argument.** `?int $step = Core\Totp::check($code, $secret);` then
  `Core\Totp::check($code, $secret, $step)` is `error[E0401]: expected int, found int|null` pointing
  at `$step`, which reads like the member's row is wrong. Put the second call inside the `else` of
  `if ($step == null)`, where the type is narrowed; every case that round-trips a `?T` answer pays
  one `if`. [until: reviewed 2026-09-06]
- **A `.nvst` case can write its own `nvs.toml`, so a capability-gated member's granted error paths
  must be asserted.** `--FILE nvs.toml--` lands in the case's directory, where
  `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2 reads it, so
  `[capabilities.script] spawn = true` grants the run and no `nvs.toml` grants nothing. Never repair
  the gate with `OWED_A_CASE` or a false `DECLARATION`; it reads throws sited in `nvs-stdlib`.
  [until: reviewed 2026-09-06]
- **An `--EXPECTF-ERROR--` block needs a `%A` at every gap between literal lines, including after
  the last one.** A block ending `%A` / `error: aborting due to N errors` matches because the `%A`
  swallows the caret excerpt and the blank line, but a pinned `   = help: ...` line before the abort
  leaves nothing to absorb the blank line between them, and the diff reads as identical text
  refused. Pin a help line when it is the claim (`no-client-member-accepts-an-unbounded-wait.nvst`)
  and put a `%A` on the line after it. [until: reviewed 2026-09-06]
- **A `nvs-types` test that compiles a snippet lives in `crates/nvs-types/tests/`, never beside the
  registry tests in `src/`.** An inline `mod tests` in `core_lib.rs` asks the lowered registry its
  questions and never sees a diagnostic, so an anchor there is for the wrong host.
  `crates/nvs-types/tests/common/mod.rs`'s `check_in_method`/`check_src` are the harness and
  `crates/nvs-types/tests/core_members.rs` owns the options-bag rules; `cargo test -p nvs-types`
  runs both targets, so only the fixture decides. [until: reviewed 2026-09-06]
- **A `.phpt` section `crates/nvs-test` parses but does not honour fails the case outright; a member
  needing one is two slices.** `case.rs`'s `NOT_YET` table reports such a case `unsupported`, and
  each entry names a blocker that may have stopped being true. Before a case that sets up its own
  world, read `NOT_YET`, not the section table in `crates/nvs-test/src/lib.rs`, which lists sections
  that do nothing too. [until: gone crates/nvs-test/src/case.rs:const NOT_YET]
- **A file in `nvs-stdlib` gets exactly one `#[cfg(test)]`; a second fails a test in another
  directory that never names it.** `crates/nvs-stdlib/tests/capability.rs`'s
  `nvs_stdlib_reaches_the_os_only_through_the_gate` scans every `src/` file down to its first
  `#[cfg(test)]` and asserts there is only one, since a second higher up would hide the whole file.
  A `#[cfg(test)] pub(crate) fn` beside `mod tests` trips it; put the fixture inside `mod tests` and
  reach it as `crate::tests::<name>`.
  [until: gone crates/nvs-stdlib/tests/capability.rs:reaches_the_os_only_through_the_gate]
- **A park asserted after the first `sched.run()` can be the connect's, not the read's.**
  `NvsTcp::connect_timeout` goes through `finish_connecting`, which parks wherever the platform says
  the handshake is still in flight, so `report.parked == 1` can hold with the read never having
  reached the reactor. Turn until the origin thread signals it holds the request, sending that
  signal before it writes any reply bytes;
  `a_socket_read_runs_on_the_reactor_and_parks_its_coroutine` in
  `crates/nvs-stdlib/src/http/transport.rs` is the shape. [until: reviewed 2026-09-06]
- **A `Core` instance has no property a program can reach, so a rule that writes one is writing a
  member.** `rule:core-classes/ratelimit-gcra` spells `Core\RateLimit\Decision` as `readonly
  allowed: bool, ...`, but `$d->allowed` does not compile; `CoreTy::Instance`'s doc comment is the
  rule. Check `CoreTy::Instance` before transcribing any rule that writes a `Core` value's fields
  with a colon.
  [until: gone crates/nvs-stdlib/src/registry.rs:no constructor, no property and no subclass]
- **A trailing `%A` on its own line in an `--EXPECTF-ERROR--` does not match nothing.** When the
  output a `%A` was absorbing goes away, the case fails with an identical-looking expected and
  actual block, because the diff prints the pattern rather than what it expanded to and the only
  visible difference is the extra `%A` line. Delete the wildcard when the output it covered goes
  away; a `%A` is an absorber, not an optional tail. [until: reviewed 2026-09-06]
- **Three registry guards beyond the conformance floor cost a cycle each when a `Core` class lands
  and are cheap up front.** `no_registry_card_cites_an_adr` refuses a rule citation in a
  `MethodDoc`; `no_member_revalidates_a_string_argument` reads the source, so `Value::as_str_bytes`
  then any `from_utf8` fails it even in an error message; and a class declaring `slots` must declare
  `instance` members too.
  [until: gone crates/nvs-stdlib/src/registry.rs:fn no_registry_card_cites_an_adr]
- **An `examples/` fixture's source is frozen by nothing, so it can name members that never existed
  and blame your class.** `examples/reflect.nvs`, `crypto.nvs` and `cli.nvs` were written ahead of
  their members and asked for a property, an unregistered member (`Core\Arr::length`) and a
  top-level `function`, surfacing as `E0405`/`E0401`. Only its `[[check]]` `want` lines are frozen:
  `nvs run` it and `grep -n 'name: "..."'` in the owning module before reading the failure as a gap.
  [until: reviewed 2026-09-06]
- **`nvs_stdlib_reaches_the_os_only_through_the_gate` is textual, so naming a forbidden type is as
  fatal as calling it.** The list is spellings (`std::fs`, `std::process::Command`, `std::env::var`)
  matched against the source above the file's `#[cfg(test)]` marker, so `fn write_chunk(file: &mut
  std::fs::File, ...)` and a `use std::fs::File;` both fail a gate about effects on a signature that
  performs none. Make the helper generic over `W: Write`, or infer the type from the door that
  answered it.
  [until: gone crates/nvs-stdlib/tests/capability.rs:reaches_the_os_only_through_the_gate]
- **The error-path gate's stem is the text before the first `{`, so a message opening on a format
  hole is silently ineligible.** A `Fault::thrown` written as `format!("{CLASS_NAME}::set(): ...")`
  has an empty stem, falls under the fourteen-character floor, and the gate never asks for the case
  that catches it, which is the wrong direction for a ratchet. Write a refusal a case will assert as
  a literal that opens with real text; the name constants are worth less here than the gate is.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:fn fault_sites]
- **A negative `decimal` has no literal spelling, and the diagnostic is `E0401: expected decimal,
  found int`.** `rule:types/numeric-literal-placement` target-types the literal, and a unary minus
  in front of one is an ordinary operator over an `int`, so `decimal $d = -4;` does not compile.
  Build it by subtraction from a `decimal` that does, `decimal $four = 4; decimal $minusFour = 0 -
  $four;`, as `tests/conformance/lang/decimal-arithmetic-is-exact-and-keeps-its-scale.nvst` does
  with `0 - $price`. [until: reviewed 2026-09-06]
- **A green conformance case can pin the absence of a rule its own record requires, and reads as
  coverage, not a gap.** `an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst` asserted what
  the compiler did before `rule:attributes/structural-retrieval`'s last paragraph existed in code,
  so landing it turned the case red. Before landing a rule the acceptance list names, `grep -rn` the
  `.nvst` corpus for the spelling it will start refusing. [until: reviewed 2026-09-06]
- **A rule's table row can be stale about the tree the way a `loop-goal.toml` comment can; the fix
  is to amend the rule.** `rule:programs/framework-core-half`'s `Core\Cldr` row claimed data
  `nvs_stdlib::cldr` never held, so the row was a member *and* a ~170-language table, a different
  size of slice than it predicts. The same one `grep` per claim applies, but a rule's body always
  states the current rule, so the stale row is a bug to fix in the same session; read the row's
  reason apart from its claim about the tree, since only the claim was wrong.
  [until: reviewed 2026-09-06]
- **A refusal only the environment can trigger cannot be cased, and the error-path gate counts it as
  owed, not exempt.** `Core\IO::stdin` throwing when stdin is a terminal asked for a case that
  cannot exist (a case runs through `Command::output` with `Stdio::null`), and an "unreachable from
  source" comment would be a lie. Fix the condition or move the rule where it can be asserted.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION: &str]
- **A `.nvst` case's locals are function-scoped, so two `catch` clauses or two `foreach` bodies
  cannot share a declared name.** `catch (RuntimeError $error)` in one `try` and `catch (IOError
  $error)` in the next is `E0406: $error is already declared`, and the later `catch` resolves the
  name against the first clause's class. Name the second one differently, or declare once above and
  assign inside. [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0406")]
- **An object cannot cross an isolate boundary outward unless it crossed inward first.**
  `Live::admit` compares descriptor addresses, so a `class Node` declared in both parent and child
  is two descriptors and `return new Node(...)` is `ok=false`. Pass the object in as `args:` and
  have the child hand it back, as `examples/cycles.nvs` does.
  [until: gone crates/nvs-runtime/src/graph.rs:on the receiving side is a different class]
- **A `.nvst` case whose program ends non-zero must carry an `--EXPECTF-ERROR--` section even when
  it writes nothing to stderr.** `crates/nvs-test/src/run.rs` is the one rule for the exit status,
  so an `exit(42)` case fails with `expected the run to succeed` and no hint that a section is
  missing. `%A` matches an empty stderr;
  `tests/conformance/error/a-limit-fatal-is-not-catchable.nvst` is the shape.
  [until: gone crates/nvs-test/src/run.rs:error expectation must fail]
- **A spawned child never drains its `Core\Script::onExit` queue, so no `.nvst` case can put two
  exit-queue endings in one file.** `nvs_stdlib::script::run_exit_hooks` is called only from
  `crates/nvs-cli/src/main.rs`'s top-level script frame, so a `spawn script` child ends `ok=true`
  with its hooks never run and no diagnostic. A bound asserted on both sides over that table needs
  two endings and a script has one; whether a spawned child should drain is open, since the rule
  says "at most once per script". [until: reviewed 2026-09-06]
- **A `secret` cannot be measured from source, so a width invariant is asserted through the members
  bounded by it.** `Core\Bytes::length($key)` on a `secret bytes` is `E0401`, because
  `rule:security/secret-qualifier` does not widen downwards. Assert every draw is accepted by the
  members that refuse every other width, and name the width over plain `bytes`, as
  `crypto-names-the-key-length-bound-on-both-sides.nvst` does. [until: reviewed 2026-09-06]
- **A `.nvst` case cannot put non-UTF-8 octets on disk, so a binary probe file has no writer
  today.** `Core\IO::write` and `Core\IO\File::write` take `CoreTy::Text(Qual::Neutral)`, so writing
  a `bytes` is `E0401`, and a NUL or lone `0xFF` cannot travel through an argument vector either. A
  bytes row on `Core\IO` or `Core\Storage::put` is a member change, not a case.
  [until: gone crates/nvs-stdlib/src/io.rs:CoreTy::Text(Qual::Sink), CoreTy::Text(Qual::Neutral)]
- **A `.nvst` case has no clock to move, so any property over more than one time step is a Rust
  test.** `Core\Test::advance` refuses where no `#[Test(at: ...)]` fixed a clock, and a case is
  top-level statements never inside one (`test-advance-refuses-without-a-fixed-clock.nvst` says so).
  A case can only watch `Core\Time::sleep` across a wall-clock second, asserting the biconditional
  *the answer moved exactly when the step did* rather than "unchanged" (flaky one run in thirty);
  the step axis belongs in the module's own `#[cfg(test)] mod tests`. [until: reviewed 2026-09-06]
- **A bound on a wall-clock second can be asserted on both sides without a fixed clock, by waiting
  for the second, not sleeping.** Spin on `Core\Time::now()->toEpochSeconds()` until it changes
  before signing, so the token is minted at the start of its own second, then loop `while (... <
  $exp) { Core\Time::sleep(5ms); }` and assert the refusal at `== $exp`, which survives an oversleep
  a fixed `sleep(1s)` would turn into a wrong number.
  `tests/conformance/core/jwt-a-token-verifies-for-its-whole-lifetime-and-not-one-second-past-it.nvst`
  is the shape. [until: reviewed 2026-09-06]
- **A `Core` value type's slots are observable through `Core\Debug::render`, which is how a case
  asks what a `Cli\Style` holds.** A captured stream is `ColorDepth::None`, and `--ENV--`
  `CLICOLOR_FORCE=1` works on Linux but not Windows, where `enable_virtual_terminal()` fails
  `GetConsoleMode` on a pipe. `Core\Debug::render($value) as string` prints the slots on every host;
  compare whole renderings. [until: reviewed 2026-09-06]
- **An assertion resting on `[cache.local] max_size` has to be sized against a probe; the arithmetic
  is not the writes' sum.** `Local::put` (`crates/nvs-stdlib/src/cache.rs`) prices an incoming entry
  before removing the one it replaces, so a rewrite needs room for the two largest entries and a cap
  at twice the total evicts nothing. Run a copy writing the same values under distinct keys; if it
  does not forget the first key, the case is vacuous. [until: reviewed 2026-09-06]
- **`gaps.py` ranks a class by case count, not by question, so a handoff item derived from it can
  name work already on disk.** A counted sweep over `Core\Cli\Style::of`'s axes was already
  `cli-a-styles-seven-slots-are-independent.nvst`, inside the very count that ranked the member
  thin. The case titles say which question each asks and are one `ls tests/conformance/core | grep
  -i <class>` away; do that first, and when the item is landed say so in the handoff rather than
  writing a second case asking the same thing. [until: gone tools/gaps.py]
- **A `..` below a component that does not exist is collapsed on Windows and refused everywhere
  else; no case may assert either.** `Core\IO::within($base, "nothing/..")` answers `$base` on
  Windows and throws `IOError` on Linux, because `nvs_config::capability::resolved` asks the
  platform canonicalizer first and Win32 normalizes `nothing\..` out before the syscall while
  `realpath` stops at the missing component. Use a `..` whose parent exists
  (`io-within-resolves-and-then-proves-containment.nvst` does) or a name that merely contains `..`.
  [until: reviewed 2026-09-06]
- **`python tools/try.py` ignores `--ENV--`, so an env-dependent case runs against the machine's own
  environment.** The scratch runner honours `--FILE--` and the expectations but never sets the
  section's variables, so a `Core\Env` case comes back `whole: 0 of 4` and reads as a broken member.
  Check an `--ENV--` case with `target/debug/nvs.exe test <path/to/one-case.nvst>`, which takes a
  single file as well as a tree. [until: exists tools/try.py:--ENV--]
- **A gate's name records the goal that wrote it, not the set it walks; read the test body before
  believing the name.** `every_part_one_member_has_a_conformance_case` iterates `registry::CLASSES`
  whole with no Part I filter, because the registry held Part I and nothing else when it was
  written, and the floor gate beside it is the same. A Part II twin is a direction rather than a
  hole, and one `sed -n` over the named test's body settles it faster than a chain of inferences
  from its name. [until: reviewed 2026-09-06]
- **Write a differential sweep as one `--ORACLE--` case and let the runner partition it.** An oracle
  failure prints PHP's whole output beside Novis's, so one run says which subjects agree; the rest
  become a second case with an `--ORACLE-DIVERGES--` line. Substitute control bytes on both sides
  first (`Core\Str::replaceAll` against PHP's `strtr`), because
  `rule:tooling/terminal-output-is-a-sink` renders a band wider than CR and LF.
  [until: reviewed 2026-09-06]
- **`Core\Csv::format` takes no `escape` knob, and the module doc's dialect section reads as though
  it does.** `csv.rs`'s *The dialect: three bytes* names `{separator?, quote?, escape?}`, which is
  the reader's set; `FORMAT_OPTIONS` is `separator`, `quote` and `header`, and the writer's only
  spelling of an inner quote is doubling. A case planning to turn `{escape:}` on the writer against
  `fputcsv`'s fifth argument has nothing to turn; what it can measure is that PHP's `""` dialect is
  this writer's.
  [until: gone crates/nvs-stdlib/src/csv.rs:are the three knobs every real CSV dialect]
- **A `uint` parameter refuses an `int` sweep value or an `int` index, and the diagnostic lands at
  the call, not the literal.** `Core\Str::repeat`'s count and both of `Core\Bytes::fill`'s arguments
  are `uint`, so `foreach ($lengths as int $n)` fails `E0401: expected uint, found int` at the call;
  `Core\Bytes::at` takes an `int` index while `Core\Bytes::length` answers `uint`, so `uint $i <
  length` fails at `at`. Declare a length list as `array<uint>` bound `as uint $n`, and carry an
  index as `int` with the length cast `as int`. [until: reviewed 2026-09-06]
- **Before writing a depth case, read the landed neighbour's whole `--FILE--`, not the handoff's
  summary of it.** A handoff item is written by a session that had the module open and the neighbour
  case closed, so its summary of a landed file is the part most likely to be stale; one said a case
  "feeds two chunks" when it swept five widths across four subjects. The uncovered claims are the
  ones the landed case's own text does not make, and only its text says which those are.
  [until: reviewed 2026-09-06]
- **A `-p nvs-db` test cannot build a `PgConn`, because its `wire` field is `Wire` at the default
  type parameter.** Anything reachable only through an inherent method on `PgConn` needs a socket
  and a certificate, and a unit test has neither. Write the sequencing as a free function generic in
  the stream (`start_statement(wire, state, ...)`, `reset_session(wire, state)`) with `PgConn`'s
  method a two-line delegation; `pg.rs`'s `Peer` then scripts a server for it with no socket, as
  `authenticate` already does. [until: gone crates/nvs-db/src/conn.rs:pub(crate) wire: Wire,]
- **A crate with the workspace's lints cannot free a `nvs_runtime::Value`, so a test in one leaks
  every string it builds.** `Value::release` is `unsafe`, `unsafe_code` is `forbid` at the workspace
  root, and `forbid` is the one level no `#[expect]` can lift. Split the decision from the
  allocation with a private enum holding the parsed scalar plus one `into_value` arm per variant,
  and assert the enum; `NvsStr` has a safe `Drop`, and `crates/nvs-db/src/pg.rs`'s `PgScalar` is the
  worked example. [until: gone Cargo.toml:unsafe_code = "forbid"]
- **A `.nvst` case that configures anything is a multi-file case, and `python tools/try.py` cannot
  run one.** `--FILE nvs.toml--` is the only way to grant a capability or write a `[db.*]` block to
  a case, and `try.py` concatenates the sections into one `.nvs`, so the TOML or a fixture's prose
  arrives as Novis source and the run dies in dozens of parse errors (`error[E0319]: disk is not a
  constant that exists`). The shipped runner takes one path or a list and prints the same
  expected/actual diff, and is already built when the session opens. [until: reviewed 2026-09-06]
- **The coverage gates read a case's `--FILE--` alone, so a class named only in the `--TEST--` title
  attributes nothing.** `Attribution::holders` attributes a `->member(` only to classes named in the
  body or a registered return type. Write the receiver's class name inside the body, in a comment if
  nowhere else, or `every_part_one_member_has_a_conformance_case` fails naming the member.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:fn holders]
- **A unit test that disables the feature under test can make two distinct names identical, and
  every assertion then passes.** `pg.rs`'s statement tests run on a `no_cache()` `StatementCache`,
  where a `Bind`'s portal and its statement are both the empty string, so `frontend::bind` called
  with the two swapped sent a `PBDES` that looked right in every case and drew SQLSTATE 26000 on
  every real connection. Assert the names a message carries and not only its tag, and give the
  enabled path a case at a non-zero capacity. [until: reviewed 2026-09-06]
- **The coverage gates and `gaps.py` attribute a case to a class textually, so a new return variant
  or call spelling drops it.** `Attribution::new` reads `return_ty:`, its `arrows` matches
  `->member(`, `tools/gaps.py`'s `RETURNS_RE` is the same regex, and `conformance_coverage.rs`
  spells a call `Class::name(`. Teach every one the variant, and spell `return_ty:` inline; a named
  `const` is invisible to the regex. [until: reviewed 2026-09-06]
- **A `Fault::` site whose message opens with literal text owes a conformance case; a `Core\Db`
  member has none to write.** `every_error_path_is_asserted_or_declared_unreachable` asks for a case
  per greppable literal message, and a member needing a live server cannot pay it. `Core\Db`'s
  refusals are `format!("{QUERY}: ...")` off a `const` naming the member; write the `const` before
  the message. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const DECLARATION: &str]
- **A `?T` narrows only inside a bare `if ($x != null)`, and `&&` does not do it.** `E0459` refuses
  `$row->date("d")->format(...)` and its help names that spelling, but `if ($a != null && $b !=
  null)` fails on both receivers just the same, and so does negating the test into an `else`. The
  compact spelling is `$x?->member() ?? "null"`, one line a column as `examples/db.nvs`'s `Db\Row`
  readers are written, instead of a nested pyramid of `if`s.
  [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0459")]
- **A `Core` class owes a case per member from the moment it is registered, even before anything can
  produce an instance.** The gate is a text match, so registering a class whose only producer has
  not landed fails `cargo test -p nvs-stdlib`; the two are one slice. No server is needed:
  `tests/conformance/core/db-rows-answers-the-types-the-results-table-names.nvst` is the shape.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const FLOOR: usize = 3]
- **A memory bound read off `budget::live_bytes` alone can be vacuous; `allocated_bytes` is the
  counter that shows it.** A stream measured with its reused buffer already allocated peaks at tens
  of bytes at any row count, so a case asserting only "the peak stayed under a bound" passes
  identically when the drain never ran or the counters are not maintained; `budget`'s module doc
  names the distinction. Assert that the churn grows with the input while the live peak does not,
  two counters doing nothing cannot fake. [until: reviewed 2026-09-06]
- **A test that reads its own file with `include_str!` must not spell its needle as a literal.** The
  scan finds the assertion's own source and answers about that instead of about the code.
  `affected_is_the_matched_count_and_changed_is_mysql_only` in `crates/nvs-db/src/pg.rs` searches
  for `concat!("fn ", "changed")`, the same bytes assembled from two literals that are not those
  bytes; `oid_constants` escapes only because it counts inside a slice its own body sits outside of,
  which is luck. [until: reviewed 2026-09-06]
- **Message tags written as `[b'D', b'H', b'q', b'W']` fail `clippy` under `-D warnings`
  (`byte_char_slices`).** The failure arrives from `verify.py`'s clippy step rather than from `cargo
  test`. Write the tags as `*b"DHqW"`, since iterating a dereferenced byte-string literal yields the
  same `u8`s, and expect the same lint on any protocol case that sweeps field or message tags.
  [until: reviewed 2026-09-06]
- **`nvs_runtime::call` hands a test back `Result<Value, i32>`, not the helper's `Fault`; the
  exception is on the `Ctx`.** Matching on the `Err` for what a member threw is `expected i32, found
  Fault`; read `ctx.pending_class()` (which falls back to `ThrownClass::name` when no class table is
  installed, as in every unit test) and `ctx.pending()` for the message. `ctx.take_pending()` *is* a
  `catch (Throwable)`, so a case proving something survives a catch performs one rather than
  describing one. [until: reviewed 2026-09-06]
- **A fixture cannot observe a state an in-process worker never yields inside; the tell is a counter
  polling out at `0`.** `[queue] workers` runs the job's isolate on the same core as the program
  polling `Core\Queue::stats`, so a job that returns straight away is `Claimed` only across a window
  in which the polling task is never scheduled, and polling faster cannot reach it. Make the job
  park — `examples/queue/receipt.nvs` sleeps for a beat and says why — and expect the same of any
  state a worker passes through between two of its own statements. [until: reviewed 2026-09-06]
- **A live-server case over `Core\Queue`'s statements belongs in `crates/nvs-stdlib/tests/queue.rs`,
  and a moved case can skip silently.** `rule:core-classes/db-crate-boundary` has `nvs-stdlib`
  depend on `nvs-db`, so a `-p nvs-db` test cannot `use nvs_stdlib::queue::CLAIM_POSTGRES`, and
  `tools/db-matrix.py` runs only the crates in `SUITES`. Prove it asserted, not skipped: `docker
  exec novis-db-postgres-1 psql -c "select …"` for the row it wrote. [until: reviewed 2026-09-06]
- **A hand-built MySQL greeting must give `scramble_2` its trailing NUL, or the plugin name comes
  back one byte short.** `HandshakePacket::new` writes `auth_plugin_data_len = scramble_2.len() + 8`
  while the deserializer reads back `max(13, len - 8)` bytes, so a 12-byte tail round-trips as 13
  and eats the first character of `auth_plugin_name` — the refusal names `aching_sha2_password`,
  which reads like a typo in the driver. Pass `NONCE[8..]` plus a `0` byte;
  `HandshakePacket::nonce()` trims it back off. [until: reviewed 2026-09-06]
- **`mysql_common`'s `Column` serializes `column_length` and `character_set` in the opposite order
  to its own deserializer.** The reader matches the wire; only a case asking about the charset or
  width notices — `rule:core-classes/db-column-types` reads `tainted string` against `tainted bytes`
  off the charset. Write the definition bytes by hand in wire order (`typed_column_def` in
  `crates/nvs-db/src/mysql.rs`), never through `MySerialize`. [until: reviewed 2026-09-06]
- **A MySQL twin of a PostgreSQL case cannot reuse the query text, and the failure is a panic.**
  `mysql_one_value` reads a text column, and MySQL types `SLEEP()` and `CONNECTION_ID()` as
  integers, so the helper dies with `the statement answered Int(0), which is not text`. Wrap the
  expression in `CAST(… AS CHAR)`; MySQL answers a missing table with error `1146` where
  `to_regclass` answers `NULL`, so assert it separately as a `ServerError`.
  [until: reviewed 2026-09-06]
- **A `-p nvs-stdlib` test asserting what a member reads off a throw's slot has to install an
  exception class table first.** `Ctx::pending_slot` answers `None` until `set_runtime_error_class`
  runs, so a member deciding on the `KIND_SLOT` sees nothing. `ClassTable::define` the four-slot
  `RuntimeError` root and a subclass wide enough for the slot (`Thrown::new_as` drops it silently);
  `retries_recover_an_induced_deadlock` is the shape. [until: reviewed 2026-09-06]
- **A fake server that parses the client's message with a needle the value may contain is a coin
  flip, and the tell is an acceptance failure that does not reproduce.** `crates/nvs-db/src/pg.rs`'s
  SCRAM fake took the client nonce with `rsplit_once("r=")`, and a nonce drawn from RFC 5802's
  printable set can itself contain `r=`, so on a rare run the fake echoed a suffix and the driver
  rightly refused its own exchange. Split on the attribute's own delimiter — `,r=` — which the
  grammar guarantees the value cannot hold. [until: reviewed 2026-09-06]
- **A matrix case's queue name isolates its rows but not its locks, and the failure lands in a
  neighbouring case.** InnoDB locks what a statement scans, so `clear`'s `delete … where queue = ?`
  holds every scanned row until commit, and a neighbour's `for update skip locked` skips its own
  row. `framed()` takes `FRAMED_WRITES`; when a `db-matrix` leg fails in a case you did not touch,
  rerun it before believing it. [until: gone crates/nvs-stdlib/tests/queue.rs:FRAMED_WRITES]
- **A TDS request counted in packets is not a request.** A case that splits `Script`'s recorded
  bytes at every header and counts requests passes on short values and fails the moment one is
  `nvarchar(max)`, because one `sp_prepexec` past the negotiated packet size leaves as several
  packets. Reassemble to `Status::EOM` before counting — `crates/nvs-db/src/tds/testing.rs`'s
  `flushed` is the helper — and keep the first packet's status, since that is the one
  `Status::RESET_CONNECTION` rides. [until: reviewed 2026-09-06]
- **A sans-io driver's fixture can agree with its parser on a byte order the wire does not use, and
  every gate stays green.** `tds/stream.rs`'s `login_ack` and its fixture both wrote LOGINACK's
  `TDSVersion` little-endian, and a real SQL Server refused; no `.nvst` case can open a socket.
  Before claiming a handshake reaches a server, run a scratch `.nvs` under `.agent-tmp/` against
  `tests/db/compose.yaml`'s endpoint with `target/debug/nvs.exe run`. [until: reviewed 2026-09-06]
- **A real SQL Server case cannot keep rows in a temporary table, and `sp_reset_connection` does not
  restore the isolation level.** A `CREATE TABLE #t` inside `sp_prepexec` is gone before the next
  statement, and the reset leaves `transaction_isolation_level` where the last transaction set it.
  Use a permanent table, `DROP TABLE IF EXISTS` first, and have `reset_session` send the isolation
  restore per `rule:security/db-pool-reset-is-a-boundary`. [until: reviewed 2026-09-06]
- **A by-value `reset(self)` on a connection whose rows borrow it cannot be tested against a
  streaming result set; the case will not compile.** `SqliteRows` (`crates/nvs-db/src/sqlite.rs`)
  holds a `&Cell<State>` into its `SqliteConn`, so `conn.reset()` while rows are alive is E0505
  rather than the runtime refusal. Set `State::Poisoned` directly and assert the state a caller
  reaches with no rows in hand; the borrow checker holds the other half.
  [until: reviewed 2026-09-06]
- **A `Scope::Path` capability root in a case's `nvs.toml` has to be absolute; a relative one
  refuses everything while reading like a grant.** `nvs_config::capability`'s matcher canonicalises
  the queried path and asks `path.starts_with(root)` with the root as written, so `read =
  ["scratch.db"]` never matches and the refusal is the ordinary "not granted" sentence. Grant an
  absolute root; `:memory:` is not a path and cannot be granted. [until: reviewed 2026-09-06]
- **A `?T` from a `Core\Db\Row` reader narrows through `if ($x != null)` and through nothing else —
  not a declared local, and not an `&&` chain.** `Core\Time\Date $born = $row->date("born");` is
  `E0401` (expected `Core\Time\Date`, found `null|Core\Time\Date`) even though the scalar `string
  $unwrapped = $named->string("owner");` next door compiles, and `if ($a != null && $b != null)`
  still raises `E0459` on both receivers in the body. Write one `if` per object receiver; a
  `?decimal` or `?bool` needs none, since `echo` and a ternary condition take them.
  [until: reviewed 2026-09-06]
- **A `\` at the end of a line inside a Novis string literal is a literal backslash, not a
  continuation, and a long SQL statement is where that bites.** Rust's `"…\` + newline eats the
  newline and the next line's indentation, so a `create table` wrapped that way reaches the driver
  as `…, \` plus the indentation, and SQLite answers `unrecognized token: "\"` at an offset in a
  statement the case never wrote. Keep a statement literal on one line however long it gets, or
  build it by concatenation. [until: reviewed 2026-09-06]
- **A `live_bytes` balance taken across a thread's first `[]` is off by one `ArrayHeader`.**
  `nvs_array_new` hands out a per-thread singleton, so the first call on a thread allocates a header
  that is never freed, and a balance test outside `nvs-runtime` running compiled code reads it as a
  leak. Call `nvs_runtime::prime_empty_array()` before `let before = live_bytes()`.
  [until: gone crates/nvs-runtime/src/array.rs:pub fn prime_empty_array]
- **A `loop-goal.toml` `cases` name can already be pinned by another check of the same file, at a
  granularity a `.nvst` case cannot reach.** A claim needing two connections to one server or a
  worker loop lives in a `cargo-named` test or an example fixture, so grepping `tests/` for the
  drafted name finds nothing and the claim reads as open work forever. Search the goal file's other
  checks before the corpus: `grep -n` the drafted name's distinctive words with the underscores
  swapped in. [until: reviewed 2026-09-06]
- **No `.nvst` case can reach a live queue server, but a `.nvst` case can reach a live SQLite
  database.** `Core\Queue`'s statements exist for every driver but SQL Server
  (`nvs_stdlib::queue::runs`), so a runtime claim about a queue on a wire driver belongs in a
  `-p nvs-stdlib` `#[test]`, and a `queue-*.nvst` pushing to one ends in `--EXPECTF-ERROR--`. A `[db.main]` with `driver = "sqlite"` and `path = ":memory:"` opens inside a
  case, and a per-connection `:memory:` shows whether two calls shared one.
  [until: reviewed 2026-09-06]
- **A rule's own example can contradict the section that owns the key set, and a test written from
  the example pins the wrong thing.** The disagreeing text is the illustrative one while the other
  section is what every other file cites, so a case asserting "the routing keys parse" off the
  example fails for the right reason on the wrong input. Before pinning a rule from a section,
  `peek.py` the section that owns the key set, and fix the example rather than reconciling it in
  your head, because the next session writes its case from the same paragraph.
  [until: reviewed 2026-09-06]
- **`Core\Out::capture` answers a `Core\Cli\Text`, so `==` between two captures is identity and a
  case comparing them counts zero agreements while printing the right bytes.**
  `rule:expressions/object-identity-equality` makes `==` on two objects identity, and the
  neighbouring `Core\Response` cases hide it by only ever printing a capture. Interpolate each into
  a `string` first — `string $s = "{$captured}";` renders the carrier through
  `rule:security/capture-answers-the-carrier` — and `==` then compares content.
  [until: reviewed 2026-09-06]
- **A `Core` member no `.nvst` case can reach still owes the floor three cases, so write its logic
  as a free function over the carrier and pin it twice.** Every `Core\Request` member refuses in a
  case, so the corpus pins only the refusal; the behaviour is pinned by `#[cfg(test)]` tests
  building an `Inbound`, which needs a free function taking `&Inbound`, not a body in `nvs_helper!`.
  `crates/nvs-stdlib/src/request.rs`'s `cookie_of` is the shape. [until: reviewed 2026-09-06]
- **A `Core` member whose error paths only a served request reaches fails the error-path gate under
  both obvious declarations.** "unreachable from source" claims the checker refuses the call, and
  `OWED_A_CASE` claims a case nobody can write. Use `ASSERTED_OFF_THE_CORPUS`: "no case can reach
  this" within eight lines above the site plus a `#[test]` asserting it, only when no case can
  exist. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:ASSERTED_OFF_THE_CORPUS]
- **An `--EXPECTF-ERROR--` section's `-->` line is indented by the width of the line number, and any
  edit above the code moves the number.** The gutter is `" ".repeat(digits) + " --> "`, counted from
  `--FILE--`'s first line, so a neighbour's two-space `  --> case.nvs:9:40` copied onto a case whose
  error is on line 14 fails with a diff whose halves look identical. Run `python tools/try.py
  <case>` and copy the whole location line verbatim. [until: reviewed 2026-09-06]
- **A `-p nvs-stdlib` test's `Ctx::set_memory_limit` bounds the fixture too, and the failure blames
  the member.** The ceiling covers everything the context allocates from `Ctx::buffered()` onward,
  so a limit set under what the multipart parse itself costs makes `files()` throw first and the
  assertion read as the walk being broken. Keep the limit roomy in absolute terms (1 MiB) and put
  the distance into the number the member is asked for (4 MiB). [until: reviewed 2026-09-06]
- **A `Core` class a `foreach` walks is not accepted where a `Core` row declares `Iterable<T>`.**
  `Core\IO::writeStream($path, $part->content())` fails as ``expected
  `array<bytes>|Iterable<bytes>|Iterator<bytes>`, found `Core\Request\PartContent` ``; conforming to
  `foreach` and to a declared `Iterable<T>` are two questions, only the first answered today. Run
  `target/debug/nvs test <case>.nvst` before writing the `--EXPECTF-ERROR--` block.
  [until: reviewed 2026-09-06]
- **A `-p <crate>` check is impossible rather than unwritten when that crate's `Cargo.toml` does not
  name the crate owning the surface.** `crates/nvs-server/Cargo.toml` names no `nvs-stdlib`, so a
  check there about `files()` can never run. Read `crates/<crate>/Cargo.toml`'s `[dependencies]`
  first; adding a dev-dependency to make the filing true puts the test one layer above the rule it
  asserts. [until: reviewed 2026-09-06]
- **A handoff item can name an unknown the same test module has already answered, and the fixture is
  usually a few lines below the anchor it gave you.** A member that landed with tests brought its
  fixtures with it — a context granting a capability through a `nvs_config::Snapshot`, a scratch
  directory under `std::env::temp_dir()` — so an item predicting a setup cost is often already paid.
  One `grep -n 'fn ' crates/<crate>/src/<file>.rs | awk -F: '$1>NNNN'` over the test module lists
  every fixture it holds, and it is the first call to make. [until: reviewed 2026-09-06]
- **A `-p nvs-stdlib` test driver may already own the reference you release, and the double release
  panics one crate away with `attempt to subtract with overflow`.** `parts_of` takes `files: Value`
  by value and ends with `files.release()`, so a test adding `dropped(files)` panics in
  `nvs_runtime::object::drop_one`. Read the tail of any `Value`-taking helper first; its
  `#[expect(unsafe_code, reason = …)]` says whether it releases its argument.
  [until: reviewed 2026-09-06]
- **Closing a conversion gap turns the conformance case that pinned the gap red, and the case's name
  does not say which type it used.**
  `command-run-throws-for-a-parameter-no-argument-converts-into.nvst` reached `Core\Command::run`'s
  `ArgConv::Unconverted` refusal through `decimal`, so landing the `decimal` arm left it asserting
  nothing, and its `--EXPECT--` failed at the full verify rather than at the edit. `grep -rln '<the
  type>' tests/conformance/` before widening any roster, and rewrite the case with a type still on
  the far side of the gap. [until: reviewed 2026-09-06]
- **A `.nvst` case that reads standard error must end in failure, and the runner blames the program
  rather than the section.** `crates/nvs-test/src/run.rs` makes any case with an `--EXPECT-ERROR--`
  section exit non-zero, so a `#[Command]` case that echoes `Core\Command::run()`'s status and
  returns reports `expected the run to fail, and it succeeded`. End the case with `exit($status as
  int);`, and prefer `--EXPECTF-ERROR--` with a trailing `%A`. [until: reviewed 2026-09-06]
- **`python tools/try.py` runs a case's `--FILE--` and drops its `--ARGS--`.** A `#[Command]` case
  therefore prints the usage page and exits 2 under `try.py` while passing under the real runner,
  which reads as the case being wrong. `target/debug/nvs.exe test <case>.nvst` runs one case with
  every section honoured and is the binary the driver's acceptance check uses, so it is the answer
  for any case with `--ARGS--`, `--ENV--`, `--INI--` or `--RUN--`.
  [until: exists tools/try.py:--ARGS--]
- **A `Core` class whose every member needs a store cannot meet the conformance floor with a happy
  path; the floor is a gate, not a target.** `conformance_coverage.rs` fails `cargo test -p
  nvs-stdlib` below three `.nvst` cases per member, so a `Core\Session` row cannot land before its
  cases. Plan each member with its refusals: no `[session]` block, an unreachable backend, a call
  before `start`. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:BELOW_THE_FLOOR]
- **`Diagnostic::with_help` lands in `notes`, not in `suggestions`, and the struct has a
  `suggestions` field to lead you the wrong way.** `with_help` pushes `format!("help: {text}")` onto
  `notes` (`crates/nvs-diagnostics/src/diagnostic.rs`), while `suggestions` is only ever a
  machine-applicable `Suggestion { message, replacement, span }` and is empty for every
  configuration diagnostic in the tree, so a refusal test reading `refused.suggestions` compiles and
  then fails on an empty vector. Assert a help sentence as `refused.notes.iter().any(...)`.
  [until: reviewed 2026-09-06]
- **A `-p nvs-server` test can drive a real upgrade handshake, and one connection carries the
  upgradable request and an ordinary one after it.** `hyper` keeps a connection whose `Connection:
  Upgrade` request was answered `200`: the response is framed normally and keep-alive continues.
  `only_an_upgradable_request_is_offered_a_slot` is the shape, and
  `request.extensions().get::<hyper::upgrade::OnUpgrade>()` answers "can this connection be
  upgraded". [until: reviewed 2026-09-06]
- **A test that asserts memory was released fails on the scheduler, because `nvs_host::Scheduler`
  keeps every finished task's whole `Ctx`.** `CoroutineResult::Return` pushes a `Finished { id, ctx,
  outcome }` onto `self.finished` and only `take_finished` removes one, so a context outlives its
  join and `live_bytes` sees no drop. Read after `run_until_idle` returns, where the `Scheduler` is
  dropped, or drain the list first. [until: reviewed 2026-09-06]
- **A difference of two `nvs_runtime::budget::live_bytes()` readings is never exactly the payload,
  and the shortfall belongs to whoever reads second.** The counter is a thread-local fed by the
  global allocator, so it counts everything live on the thread, the fixture's own copy included.
  Name an allowance for the second reader's footprint (64 KiB against a 2 MiB signal still catches a
  retained arena) rather than an exact number. [until: reviewed 2026-09-06]
- **A `Core` member's throw does not come back from `nvs_runtime::call` — it is on the context, and
  the `Err` is a bare status integer.** Asserting a refusal's wording off the `expect_err` value
  reads `the refusal did not name the missing connection: 1`. `Fault::thrown` records the message on
  the context, so read `ctx.pending()` after the call;
  `an_upgrade_on_a_request_no_connection_offered_a_slot_for_is_refused` is the shape.
  [until: reviewed 2026-09-06]
- **`conformance_coverage`'s error-path gate reads the eight lines above the throw, and a `match`
  arm is its own site.** One "no case can reach this" declaration above a `map_err(|error| match …)`
  does not cover the arms inside it, so the gate reports the arm's line even though a comment sits
  three lines above the call. Put the comment on the arm; and since the gate names the message
  prefix rather than the line, two arms formatting the same prefix are one entry and go green
  together. [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:no case can reach this]
- **A `.nvst` case cannot annotate a `var` and cannot declare a bare function, so naming a `Core`
  class takes a static method's parameter list.** `var $msg: ?Core\Socket\Message = …` is a run of
  `E0101`/`E0102`s and a top-level `function describe(…)` is `E0215`
  (`rule:classes/no-free-functions-or-constants`). Name the class in a `public static function`'s
  parameter list inside a `class` block, where `Attribution::holders` reads it.
  [until: reviewed 2026-09-06]
- **A second `#[cfg(test)]` item in an `nvs-stdlib` module fails a test in another file, about a
  scan rather than your item.** `nvs_stdlib_reaches_the_os_only_through_the_gate` reads each module
  to its first `#[cfg(test)]` and refuses a second, so a test-only helper beside the state it reads
  turns the crate red elsewhere. Put every test-only item in the module's `mod tests`, reaching
  private state through `super::`. [until: gone crates/nvs-stdlib/tests/capability.rs:more than one]
- **A `-p <crate>` test asserting a process-wide ceiling races every other test in the binary; make
  the count a parameter rather than serialise the tests.** A bound in one `static AtomicU64` is only
  assertable at a ceiling of one, so `Slot::take(1)` answers whichever other test held a connection
  at that instant. Add `take_from(count: &'static AtomicU64, ceiling)` with `take(ceiling)`
  delegating, and let the test own a `static COUNT`. [until: reviewed 2026-09-06]
- **A `-p nvs-server` test cannot ask for a second connection, and `run_until_idle` forbids it, not
  `|| ControlFlow::Break(())`.** It returns as soon as the core has nothing runnable, and an
  `accept` on a client not yet connected is that, so a `keep_serving` counting to two is a
  sixty-second silent hang. Assert server-side, where `serve_on_this_core` parks until every
  connection is done; `a_connection_whose_isolate_panics_is_contained` is the shape.
  [until: reviewed 2026-09-06]
- **In every `-p nvs-server` accept-loop test the drain has begun before the connection's isolate
  runs a line, so anything keyed off `Draining::is_draining()` fires on its first wait.**
  `serve_on_this_core`'s `keep_serving` is `|| ControlFlow::Break(())` in all of them, so the loop
  breaks and calls `draining.begin()` before the child it spawned has run. Make the drain cap the
  wait with a period and the close what the timeout becomes. [until: reviewed 2026-09-06]
- **A decided-but-unlanded mechanism leaves one fingerprint — the rule's types present, no
  production caller — and grepping its key type finds it.** A `loop-goal.toml` check can be misfiled
  and blocked at once: a cache that never revalidates passes "an open connection keeps its unit
  across an edit" by pinning the rule's absence. `grep -rn <KeyType> crates/` before reading the
  module that would use it, whose doc may state the gap as settled policy.
  [until: reviewed 2026-09-06]
- **Every name in a `cargo-named` check has to run on the platform the driver runs `cargo` on, so
  the `#[cfg(unix)]` a permission claim wants fails the check forever.** A `#[cfg(unix)]` test reads
  as green locally and leaves the driver reporting `did not run` on Windows. Take the verdict as a
  parameter, inject a `Breach` everywhere, and assert the platform's own spelling on the other side
  (a mode on Unix, `nvs_config::trust::exposure` on Windows). [until: reviewed 2026-09-06]
- **A `-p nvs-cli` test whose program `spawn script`s needs a scheduler and a reactor;
  `nvs_host::Isolate::run` alone installs neither.** Neither failure names it: `script.spawn` `is
  not granted`, then `needs a scheduler on this thread`. Copy `crates/nvs-cli/src/script.rs`'s
  `run_serving`: `Scheduler::new()`, `spawn(granting_ctx(), TaskRoot::Request, …)`,
  `reactor::install(Reactor::new()?)`, `run_until_idle`. [until: reviewed 2026-09-06]
- **A `.nvst` case that pins a rendering meets two errors about the case rather than the member.**
  `Core\Debug::render` answers `Core\Cli\Text`, not `string`, so `string $r =
  Core\Debug::render($v);` is `E0401`; and a rendered property name begins with `$`, so `"{\n $token
  => …"` is `E0301: $token is not declared`. Escape the property as `\$token`;
  `test-assert-matches-inline-never-holds-a-secret-property.nvst` carries the spelling.
  [until: reviewed 2026-09-06]
- **`conformance_coverage.rs` counts only the fully qualified call spelling, so cases that write
  `Test::member(` under a `use Core\Test;` are reported as "asked by 0 case(s)".** The gate builds
  `{class}::{name}(` from the registry row and asks whether the corpus contains it, and
  `corpus::Attribution` indexes a case the same way, so an aliased call site is invisible to both
  and two tests fail without saying the word alias. Write the member's FQ spelling at least once per
  case; the surrounding calls may stay aliased. [until: reviewed 2026-09-06]
- **`Core\Request::route()` throws a `LogicError` where there is no request, and a `.nvst` case that
  catches `RuntimeError` around it does not catch.** The runner fixture `in-process-request.nvs`
  catches `RuntimeError` and is green only because `nvs test` runs the entry with a request in
  front; the message "there is no request here" identifies it, the trace frame does not. Catch
  `LogicError`; `rule:security/request-state-throws-in-an-isolate` says why.
  [until: reviewed 2026-09-06]
- **A `-p nvs-cli` runner fixture has no `nvs.toml`, so a `[db.<name>]` block built in Rust skips
  the boot-time path resolution.** `nvs_config::db` resolves `tls_ca_file` against its file's
  directory, and a tree assembled in Rust has no file, so a relative `"tests/db/ca.crt"` resolves
  against whatever directory `cargo test` chose. Build such paths from `CARGO_MANIFEST_DIR`;
  `runner.rs`'s `compose_postgres` is the shape. [until: reviewed 2026-09-06]
- **A socket fixture that half-closes after writing makes every "exactly one response" assertion
  vacuous, and the control row fails.** Half-closing makes `read_to_string` return, but a pipelined
  pair written down one socket and then half-closed reads back one response, whatever the door did.
  Use a bounded wait instead of an EOF — `set_read_timeout` a few hundred milliseconds, read until
  close or timeout — and write the pipelined pair as a control row. [until: reviewed 2026-09-06]
- **An isolate that parks on something the connection cannot fail wedges the whole core, as a silent
  hang.** A park on a `nvs_host::channel` receiver whose sender the same closure holds never
  returns: `Peer::drop` cancels then waits, and neither it nor `run_until_idle` unwedges the task.
  Park on the body instead: a head promising 100 bytes whose client sends four and closes parks the
  program inside `next_chunk`, where the connection can fail it. [until: reviewed 2026-09-06]
- **A state-bleed suite parameterised over a request boundary and an isolate boundary cannot have a
  memory row.** "The next run's arena is its own" holds across a request but not an isolate, because
  `rule:security/isolate-shares-nothing` gives a child isolate its parent's budget. Parameterise
  only the state a rule says is never shared and keep memory in its own cases: only
  `nvs_runtime::budget::live_bytes()` sees what an earlier run left. [until: reviewed 2026-09-06]
- **A test outside `nvs-runtime` cannot install a current context, so an object it allocates by hand
  is in no live list and the teardown sweep misses it.** `NvsObj::new` links into whatever
  `CurrentCtx` names, and `CurrentCtx` is not in `nvs_runtime`'s `pub use ctx::{…}`. Go the way a
  request goes: `nvs_runtime::call(entry, ctx, &[…])` around an `unsafe extern "C" fn`;
  `build_a_cycle` (`crates/nvs-host/src/isolate.rs`) is the shape. [until: reviewed 2026-09-06]
- **A `.nvst` case gets exactly one in-process request, and the child's deferred output has nowhere
  to go.** `Core\Test::request` is refused from inside the request it answers, and `runner.rs`'s
  `answer` takes the `Completion` before the deferred queue drains, so an `afterResponse` `echo`
  reaches a buffer nobody reads. Pin a cross-request claim at the boot (`[session] backend =
  "local"` is `E0626`) and an after-the-answer claim from the caller's side.
  [until: reviewed 2026-09-06]
- **A `Core\X::member` spelling does not survive `spec_registry_coverage.rs`'s `cells()`, and the
  failure is silent.** `cells()` reads a `\` as a cell escape and re-emits it twice —
  `Core\Str::trim` comes back as `Core\\Str::trim` — so a member regex over its output matches
  nothing. Parse `docs/spec/02-php-migration.md` with `tools/check-migration.py`'s own row regex,
  transcribed into the Rust walk.
  [until: gone crates/nvs-stdlib/tests/spec_registry_coverage.rs:fn cells]
- **A differential case's output may not carry a lone carriage return, and the failure reads as the
  member losing it.** Both sides are normalised (`crates/nvs-test/src/expect.rs`), and a `\r`
  outside a `\r\n` survives on the Novis side and not on PHP's, so a `trimStart` case padded with
  carriage returns fails. Assert the carriage return by length —
  `Core\Str::length(Core\Str::trimStart("\r\rx"))` against `strlen(ltrim("\r\rx"))`.
  [until: reviewed 2026-09-06]
- **A `close`d SQLite `:memory:` connection is handed straight back by the pool, so a case cannot
  assert that a reconnect is a fresh database.** `connect`, create a table, `close`, `connect` again
  prints `table: found`, because the release pooled the connection and the second `connect` got it
  back, database and all. Assert what is true of the two handles — the closed one is still refused,
  the new one runs a statement. [until: reviewed 2026-09-06]
- **A `Core` member declared on two classes owes the conformance floor twice, and the wrong receiver
  counts for neither.** The floor is per class; `corpus::Attribution` attributes `->member(` to the
  classes a case mentions, so three cases writing `$db->stream(` leave `Core\Db\Transaction::stream`
  at zero. Add a `$db->transaction(fn (Core\Db\Transaction $tx) => …)` half, and run `cargo test -p
  nvs-stdlib --test conformance_coverage` before `verify.py`. [until: reviewed 2026-09-06]
- **A `-p nvs-stdlib` test cannot build any `nvs_db::Connection`, so a `Core\Db` member is testable
  only below the driver.** Its fields are `pub(crate)`; `filed_connection` downcasts a
  `HeldConnection` to `nvs_db::Connection`, so a fake reaches only driver-free members. Split the
  member's tail into a function over the receiver alone (`stream.rs`'s `park_row`), drive it with
  `crate::instance::build(&CLASS, [...])` and count with `NvsObj::refcount_of`.
  [until: reviewed 2026-09-06]
- **A `.nvst` case's second `foreach` cannot re-declare the first one's loop-local, and the error
  points at the first declaration rather than at the line you just wrote.** A case is top-level
  statements in one `ScriptFrame` (`rule:statements/storage-that-outlives-a-call`), and a block is
  not a storage scope, so two loops each opening with `var $form = …` is `second declaration`
  reported against a line ten above the cursor. Rename the second loop's locals; the tell is a caret
  on a line the edit did not touch. [until: reviewed 2026-09-06]
- **A pattern letter the grammar refuses is a shared fixture across three trees and a Rust test, and
  grepping for the letter finds only half.** Cases printing only the refusal's class keep the letter
  in a variable, so `grep -rn "not a pattern letter"` misses them. Grep for the message and the
  pattern string (`"yyyy-` and `format(\"`) before widening a closed grammar, and pick the
  replacement from the letters CLDR reserves and gives no field — `j`, `J`, `C`, `l` — since every
  letter that module's gap once named now formats. Two of those cases also **count** the alphabet
  and the field roster, so widening the subset moves four numbers in one `--EXPECT--`.
  [until: reviewed 2026-09-16]
- **A test that a method table was bound needs a call the compiler cannot devirtualize, and
  `static::m()` is the reliable one.** `$obj->m()` on a known class lowers to a direct
  `InstKind::Call`; late static binding lowers to `InstKind::CallVirtual`. Use a `Base` whose
  `shout()` calls `static::speak()` and a `Derived` overriding `speak`, assert on
  `Derived::shout()`, then delete the binding call, watch it fail, and put it back.
  [until: reviewed 2026-09-06]
- **A count of a marker in emitted SQL counts that marker inside other constructs too, and
  PostgreSQL's identity is the one that bites.** `create.matches(" DEFAULT ").count()` against the
  number of defaulted columns is off by one on PostgreSQL alone, because its identity spelling is
  `GENERATED BY DEFAULT AS IDENTITY`. Split the statement into lines, find each column's own clause,
  strip the dialect's identity spelling, and assert presence against
  `Column::default_value().is_some()`, which also catches two columns swapping their defaults.
  [until: reviewed 2026-09-06]
- **A `#[cfg(test)] mod tests` may already hold a `type` alias for the struct you are about to add,
  and `use super::*` lets the alias win.** A real `ColumnRow` above `catalog.rs`'s placeholder `type
  ColumnRow = (String, …)` failed as `expected struct, variant or union type, found (String, …)`.
  `grep -n '<TypeName>' <file>` before naming a type after something the file's tests name; delete
  the placeholder and move its `.0`/`.1` accesses onto the fields. [until: reviewed 2026-09-06]
- **A `.nvst`'s expected output pins line numbers in the `--FILE--` block above it, so a tree-wide
  script that touches case files must preserve their line count.** A rewrite that joined two `//`
  comment lines into one shifted every `--> case.nvs:NN:CC` below the join, and cases whose expected
  output is program text survive the same join, which is what makes it look safe when spot-checked.
  The budget is lines, not bytes: rewrite the token in place and leave the break standing rather
  than reflowing and re-recording. [until: reviewed 2026-09-06]
- **A `-p nvs-db` case that introspects reads the *whole* database, so an unrelated table can refuse
  the read.** `nvs_db::direct::schema_of` assembles every table the catalog answers and fails with
  `ReadError::Vocabulary` at the first column type the closed vocabulary cannot name — on the shared
  matrix server that was `nvs_jobs.script text`, written by a hand-written list in another crate, and
  the failure reads as the fixture being wrong. Narrow the *comparison* to the fixture's own tables,
  and ask `information_schema` which column the server means before blaming the round trip.
  [until: reviewed 2026-09-06]
- **A count the allocator is meant to refuse has to be past the address space, not merely past the
  RAM.** macOS backs a mapping lazily and says yes to a terabyte, so a case asking for one gets the
  allocation and then spends its whole 60-second budget writing the pages, where linux and windows
  refuse the same count outright and the case reads as green. Ask for a petabyte — still far under
  the `isize::MAX` seam, past every 64-bit address space — as
  `count-shaped-producers-refuse-alike.nvst` does.
  [until: gone tests/conformance/core/count-shaped-producers-refuse-alike.nvst:1000000000000000]
- **A `file:line` cited by the goal prose can point into a case's `--EXPECT--` section, where it is
  output rather than source.** The `//// ____` at
  `tests/conformance/core/encoding-every-encoder-agrees-with-its-own-decoder-over-a-table.nvst:114` is
  an encoder's expected stdout, not a divider comment, and the corpus holds no `////` comment anywhere —
  the only `////` in source sits inside the text of a `//` comment at line 47 of that same file. Read
  which section a cited line falls in before writing the test that pins it: a case file is several
  languages stacked, and only the `--FILE--` one is Novis. [until: reviewed 2026-09-07]
- **A Rust test that sweeps the corpus reads your new refusal cases as corpus, and the failure lands
  in a crate you did not touch.** `tests/conformance/` holds cases whose whole subject is a
  diagnostic, so a sweep asserting "no corpus file reports `E0nnn`" fails the moment someone pins
  `E0nnn` — the report named a `-p nvs-syntax` test while the edit was six `.nvst` files. Such a
  sweep has to drop a case carrying `--EXPECT-ERROR--` or `--EXPECTF-ERROR--`, which states a
  diagnostic rather than carrying one. [until: reviewed 2026-09-07]
- **A `Diagnostic`'s help line *is* a note, so "this refusal carries no note" is an assertion that can
  never hold.** `Diagnostic::with_help` pushes `help: …` onto the same `notes` vector `with_note` fills,
  so `d.notes.is_empty()` is false for any diagnostic that offers a fix at all, and a test asserting a
  *second* note is absent fails on the first one. Assert on a note's content — `n.contains("PHP 8.5")` —
  rather than on the vector's length. [until: reviewed 2026-09-07]
- **A `%A` on its own line cannot reach a diagnostic's `= note:` or `= help:` continuation, because
  the newline the wildcard ends on has to match one in the output.** Pinning `PHP 8.5's |> applies a
  callable` as its own expected line fails against `  = note: PHP 8.5's …` — the wildcard absorbs the
  indentation happily, but the pattern still demands a line break immediately before the literal, and
  the diff then prints two blocks that read alike. Write the wildcard inline instead, `%A= note: …`,
  which matches and also survives the gutter widening when the case's line number reaches two digits.
  [until: reviewed 2026-09-07]
- **A `reject` case's `--EXPECTF-ERROR--` block lists the *later* passes' diagnostics too, because a
  parse-band refusal does not stop the compile.** A case pinning `E0250` for a `return` in a
  `finally` also collected an `E0301` for an undeclared local in its own scaffolding, between the two
  errors it meant to pin and inside the `aborting due to N errors` count. Declare every local a
  reject case writes (`var $n = 1;`), and take the expected block from the runner's own `actual:`
  dump rather than from what the refusal alone would print. [until: reviewed 2026-09-07]
- **A `.lspt` case's second `--FILE--` cannot exist as a buffer alone.** `nvs_hir::requires`
  canonicalizes a `require` target against the filesystem before any source map is consulted, so a
  file that exists only as an overlay is reported `E0311` "cannot be loaded", which reads as a case
  that named the wrong path rather than as a runner that never wrote it. `nvs_lsp::suite`'s
  `Materialised` writes every section into a scratch directory and opens the buffers over those real
  paths — anything else that drives the front end from text alone owes the same.
  [until: gone crates/nvs-lsp/src/suite.rs:Materialised]
- **A free `function` at top level is refused, so nothing in its body is type-checked and a cursor
  request inside one answers about nothing.** An `.lspt` case that wrapped `new User()` in
  `function make(): void` got `none` from `definition` for a reason that had nothing to do with the
  request: `check_declarations` reports `E0215` for the declaration
  (`rule:classes/no-free-functions-or-constants`) and the walk never types the body. Put a fixture's
  executable code at the top level or inside a method. [until: reviewed 2026-09-07]
- **A `.lspt` case that writes `$u = new User();` records no member entry at all.** `var` is what
  declares a variable, so a bare assignment is `E0301` and the receiver has no type — the checker
  still records `ExprInfo::New` for the `new`, and nothing for `$u->name`, so a `definition` or
  `hover` case on the member answers `none` while the same case on the class name passes. Write
  `var $u = new User();` in any case whose cursor is on a member.
  [until: reviewed 2026-09-07]
- **A Rust test can be leaning on a request having *no* handler, and its title says nothing about
  which one.** `nvs-lsp`'s `a_tree_of_cases_reports_one_summary_line` builds a two-case tree it
  expects to score `0 passed, 2 failed`, and one of those cases failed only because `answer` had no
  `completion` arm — so landing the arm turned a fixture into a pass and broke a test about summary
  lines. Before implementing a `.lspt` request, grep the crate's own tests for its name and re-point
  any fixture that was failing for want of a handler at a genuinely mismatched `--EXPECT--`.
  [until: reviewed 2026-09-08]
- A trailing `->` with a **keyword on the next line** is parsed as that keyword being the member
  name, not as recovery. `$u->` followed by `if (true) {` is one `$u->if(true)` call, because a
  member name accepts a keyword the way PHP's does, so a case written to pin resilient behaviour
  ends up freezing an answer about `if` rather than about the arrow. Reach `MemberName::Missing` by
  leaving the arrow at end of file and putting the unclosed brace *above* it.
  [until: reviewed 2026-09-08]
- **A `foreach` binding with no type is a syntax error, not an untyped binding**: `rule:types/grammar`.2
  makes every binding write its type, so `foreach ($xs as $v)` reports `E0101` at the name and a case
  asking what a bare binding carries never gets that far. A handoff item can name the construct in that
  PHP shape anyway, because nothing in an LSP file set contradicts it. Run `nvs check` over the
  `--FILE--` document before freezing anything about it. [until: reviewed 2026-09-08]
- **A `-p nvs-lsp` test needs no file on disk to analyse a document.** `Documents::open` registers
  the buffer as an overlay and `SourceMap::load` hands that back before it reaches the filesystem,
  so `analyse` answers for a URI naming a path that does not exist. `crates/nvs-lsp/tests/publish.rs`'s `TempDir`
  is there for `require` resolution and republish-by-path, not for the analysis, so copy it only
  when a case has a second file. [until: reviewed 2026-09-08]
- **A cursor in inline HTML lands on a node, so "inside no production" is narrower than it looks.**
  `InlineHtml` is one of `nvs_syntax::walk`'s own kinds, and so are `Error` and every operator, so
  `SyntaxIndex::at` answers something for a cursor almost anywhere in a file. The offsets that are
  genuinely inside nothing are the trivia runs between two nodes — a blank line between two
  statements is the one to reach for. [until: reviewed 2026-09-08]
- **A `.lspt` case covers the construct the *parser* chose, and `array<mixed> $rows = [1, 2, 3];`
  is a `LocalDecl` rather than the refused declaration it reads as.** The matrix's `Error` column
  comes from a member the parser could not read at all — `var $total = 1;` inside a class body —
  while a top-level annotated declaration parses cleanly and only the checker objects to it. Write
  the case, run `nvs lsp-test <dir> --coverage`, and read which cell moved; nothing else says where
  the cursor landed. [until: reviewed 2026-09-08]
- **A document-wide answer credits `C::CONST`'s receiver, never the access.** The receiver is a
  `ConstFetch` node of its own, so a diagnostic about the constant — reported at the class expression,
  `crates/nvs-hir/src/members.rs:1058` — fills that cell and leaves `ClassConstAccess` empty. Only a
  position past the receiver's end is inside the access, and a class constant has no token there.
  [until: reviewed 2026-12-01]
- **A hostile case built out of depth meets two limits that are not the ones it looks like: the
  parser stops at 96 levels of *grammar recursion*, about nineteen nested parentheses rather than
  ninety-six, and `var` refuses an array literal outright (`E0414`).** Both come back as a compile
  diagnostic, which the hostile runner counts as a failure rather than as the runtime surviving.
  Run the candidate with `target/debug/nvs.exe run` first, keep a legal nest near a dozen
  parentheses, and write the nested type out — `array<array<string>> $t = [["leaf"]];`.
  [until: reviewed 2026-09-08]
- **A test that walks this repository's `Cargo.toml` files cannot be proved by seeding the line it
  exists to catch into a workspace member, because cargo re-resolves every member before the test
  binary runs and a fictional dependency stops at the network.**
  `crates/nvs-runtime/tests/manifest_policy.rs` walks every `Cargo.toml` outside `target/` and
  `.git/`, and `benches/*` is a member of the root workspace while `fuzz/Cargo.toml` declares its own
  empty `[workspace]`. Seed the line in `fuzz/`, watch the assertion fail naming that path, and revert
  — that walk reads the file and cargo never resolves it. [until: reviewed 2026-09-08]
- **A new pattern in the grammar's `#code` splits spans that another suite already asserted on, and
  `span()` throws instead of failing softly.** `editors/vscode/test/grammar/tokenize.ts:89` wants exactly
  one span reading the text, so `span(spans, "$total = 1;")` stops resolving the moment a keyword or a
  number rule claims part of it, and the error names the text rather than the pattern that took it.
  Before adding a construct family, grep the other `*.test.ts` under `editors/vscode/test/grammar/` for a
  `span(` whose text holds a word the new pattern claims, and split that assertion into the pieces the
  new rule leaves. [until: reviewed 2026-09-08]
- **A new name pattern splits spans the other grammar suites assert on, and the failure names the
  test helper rather than the grammar.** `tokenize.ts`'s `span()` throws `0 spans read exactly
  "$total = "` the moment a variable rule lands, because every suite that asserted on a run of
  uncoloured code now sees that run in pieces — one pattern moved assertions in four files. Before
  adding a pattern to `#code`, grep the other `*.test.ts` for a span text holding a `$`, a `->` or a
  name the new rule will claim. [until: reviewed 2026-09-08]
- **A word the TextMate grammar leaves alone is not its own span, so a test cannot fetch it by its
  text.** `vscode-textmate` emits one span per run of identically-scoped bytes, so an uncoloured name
  is swallowed into the plain run around it — ` DEFAULT = Currency` is one span — and
  `spans.filter((s) => s.text === "Currency")` finds the coloured occurrence alone. Assert a negative
  as an absence (`filter(…).length === 0`) or against the whole run the way `names.test.ts`'s
  `plain(" Base {")` does, never by expecting an uncoloured span to exist.
  [until: gone editors/vscode/test/grammar/tokenize.ts]
- **A `--POST_RAW--` body carries the case file's trailing newline and a `--POST--` body does not.**
  `tests/conformance/core/a-raw-request-body-reaches-the-program-verbatim.nvst` sends 42 visible
  characters and reads `content-length` back as 43, while
  `tests/conformance/core/a-body-read-twice-answers-the-same-octets.nvst` echoes a `--POST--` body with
  a `"\n"` of its own and matches. So a case that echoes a raw body writes no newline after it, and one
  that reads a field out of a raw body gets that newline inside the last field's value — write the body
  the member will read rather than the one that looks tidy in the file.
  [until: reviewed 2026-09-08]
- **A `--POST_RAW--` body carries the section's own closing newline, so `echo Core\Request::body(),
  "\n"` prints a blank line the `--EXPECT--` does not have.** `--POST--` does not — it is a field
  list the runner re-encodes — so a case copied from a form-bodied neighbour fails on a line that
  looks identical in the diff. Echo a raw body with no separator of your own, and say so in a
  comment beside it. [until: reviewed 2026-09-08]
- **A `loop-goal.toml` case can name a refusal whose checker half was never written, while the rule
  that specifies it reads as landed.** `rule:security/derived-codec-qualifiers` needs a tainted
  payload's receiving fields to declare the qualifier, but only its `secret` half has a diagnostic
  (`crates/nvs-diagnostics/src/lib.rs:966`), so the case named for the other half is a compiler
  slice. Grep the diagnostic registry for a rule's token before writing an `--EXPECTF-ERROR--` case:
  a fragment states what is decided, never what is on disk. [until: reviewed 2026-09-08]
- **A unit test handing a member a class with a derived JSON codec owes that class a native
  constructor.** `ClassTable::define` plus `crates/nvs-runtime/src/object.rs:1194`'s `set_codec` looks
  like the whole fixture, but hydration runs the class's *real* constructor through
  `crates/nvs-runtime/src/object.rs:2798`, which faults on a class with no `CONSTRUCTOR` row — and
  `crates/nvs-stdlib/tests/allocation_policy.rs:288`, the nearest shape, declares none. Budget the
  fixture as its own slice, and leak the table, because a descriptor's address is its identity.
  [until: reviewed 2026-09-08]
- **`nvs-test` has no dependencies on purpose, so a runtime type is unreachable from it.** Its
  `Cargo.toml` carries the reason, which is that a failing runner must not look like the thing it
  tests failing. A slice asked to route the `.nvst` sections through a runtime type routes them
  through the `.nvsr` *file* instead, whose one reader is `inbound_from` in
  `crates/nvs-cli/src/main.rs`, and names the other writer where the derivation is duplicated.
  [until: gone crates/nvs-test/Cargo.toml:No dependencies on purpose]
- **`conformance_coverage`'s error-path gate reads an 8-line window, so one comment cannot
  declare two adjacent internal-error guards.** A new `Core` member on
  `registry::WRITTEN_CLASS_MEMBERS` has three `Fault::fatal` guards in a row for the constants
  its call site wrote, and a leading paragraph covering all three reaches only the first —
  `every_error_path_is_asserted_or_declared_unreachable` then names one line and reads like a
  missing case. Write "unreachable from source" plus the diagnostic that refuses the call
  directly above **each** guard.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
- **A shape field whose name is a PHP keyword cascades into a hundred parse errors, and the first one
  reads as a defect in the field's *type*.** `{list: array<string>}` fails at `list`, which lexes as a
  keyword rather than an `Ident`, so `parse_shape_type`'s `expect(TokenKind::Ident, "a field name")`
  points at the name while every error after it blames the `<` of the type beside it — which reads
  exactly like a shape that cannot hold an `array<T>`. Rename the field before believing the type is at
  fault: `{items: array<string>}` parses, takes `tainted`'s distribution and is silent.
  [until: reviewed 2026-09-08]
- **An `--EXPECTF-ERROR--`'s `= help:` line sits in the same gutter as its `-->` line, so its
  indentation widens with the line number too.** A block pinning the same help sentence at
  `case.nvs:5` and again at `case.nvs:15` needs two spaces on the first and three on the second, and
  the diff reads as two identical lines refused because the `%A` above absorbed the caret excerpt and
  left the leading space to the literal. Copy each `= help:`/`= note:` line out of the runner's actual
  half per error block rather than writing one and repeating it. [until: reviewed 2026-09-06]
- **A compiled `parse`'s throw reaches a test as `Fault::Pending`, not `Fault::Thrown`, and its
  sentence is on the context.** `call_static` leaves the implementor's message pending rather than
  building a fault around it, so an arm matching `Fault::Thrown(_, said)` — what the *engine's own*
  refusals look like one arm along — panics on a refusal that worked. Match `Fault::Pending(_)` and
  read `ctx.take_pending()`, as `crate::command`'s `parse_each` does. [until: reviewed 2026-09-08]
- **A driver's stream type has more than one default type parameter to move, and the compiler blames
  the caller rather than the type.** Giving `pg.rs` a `PgStream` meant changing `Wire<S = …>`'s
  default *and* `PgRows<'a, S = …>`'s, and the one error was an `expected &mut Wire<NvsTls>, found
  &mut Wire` on `PgConn::query`'s call to `start_statement`, which reads as a bug in the free
  function. Before widening a driver's transport, `grep -n '= NvsTls<NvsTcp>'` in that module and
  move every default in one edit. [until: reviewed 2026-09-09]
- **A `-p nvs-server` test that begins the *process* drain cannot put it back, and every test in the
  binary shares that bit.** `Drain::process` hands out a handle on one `OnceLock` atomic and `begin`
  is the only writer there is, so a case asserting that a server is not draining races the case that
  began it, whichever order cargo runs them in. `is_draining_answers_the_same_on_every_core` is the
  one case here that begins it and its doc comment says so; write every other one against
  `Draining::detached()`, which is what that constructor exists for. [until: reviewed 2026-09-09]
- **A `-p nvs-server` test *can* serve a second connection when its core is an `nvs_host::Worker`, and
  the bullet about `run_until_idle` is only about the one-shot fixture.** `run_the_core` keeps taking
  turns while anything is parked, so a `keep_serving` that counts down accepts every connection it is
  given — it fires *after* each accept and never before one
  (`crates/nvs-server/src/serve.rs:1373`). Hold one of those connections in flight with a body three
  bytes short: `crate::body::Pull::next_chunk` parks the run with its place in the admission count
  still taken, so "two requests in flight at once" is a state to write rather than a race to win.
  [until: reviewed 2026-09-09]
- **A `-p nvs-cli` task that runs an isolate finishes *two* tasks, so
  `Scheduler::run().finished` comes back at twice the requests the arm spawned.** The isolate is a
  task of its own beside the request's, and an `assert_eq!(finished, requests)` then fails at
  exactly `2 ×` with nothing in the message about isolates to point at why. Count what the request
  itself did — an `AtomicUsize` the task bumps once its `Completion` reads `ok` — and leave
  `finished` to a case that is about the scheduler. [until: reviewed 2026-09-09]
- **A dotted-decimal address with leading zeros does not parse in Rust, so it cannot stand for "the
  same endpoint spelled another way" in a `net.listen` case.**
  `"127.000.000.001:8080".parse::<std::net::SocketAddr>()` is an `AddrParseError`, because Rust
  refuses the octal-ambiguous form outright rather than reading it the way `inet_pton` does, so a
  case built around it panics in its own helper before it asserts anything. The spelling that does
  exercise `Scope::Endpoint`'s address comparison is the IPv4-mapped v6 one,
  `[::ffff:127.0.0.1]:8080`, which parses and unmaps onto the granted v4 address.
  [until: reviewed 2026-09-09]
- **A `#[cfg(test)]` `Ctx::stdout()` in a crate the language server links fails `nvs-lsp`, not the
  crate you wrote it in.** `crates/nvs-lsp/tests/stdout_policy.rs` greps the *source text* of every
  linked crate, so a unit test that builds a context the obvious way leaves `cargo test -p
  nvs-stdlib` green and turns two `-p nvs-lsp` tests red, naming
  `rule:ide/stdout-belongs-to-the-protocol` rather than the member you were writing. Build a test
  context as `Ctx::new(OutputSink::Sink)`, which is what every other `nvs-stdlib` unit test already
  does. [until: gone crates/nvs-lsp/tests/stdout_policy.rs]
- **A `Fault` message's stem reaches `conformance_coverage.rs`'s corpus only through an `--EXPECT--`
  line, because a `"Core\\Zip: ..."` literal in a `--FILE--` section is two backslashes on disk.**
  `every_error_path_is_asserted_or_declared_unreachable` does a plain substring search over each
  case's whole text for the run of the message before its first `{`, and `Core\\Zip` does not
  contain `Core\Zip`. Print the message — `echo $e->message` — rather than testing it with
  `Core\Str::startsWith`, and keep every hole out of the stem, which means a refusal must not
  interpolate a library's own error text if the case is to freeze it. [until: reviewed 2026-09-09]
- **A refusal only a symlink can reach has no fake to hide behind.**
  `crates/nvs-stdlib/tests/capability.rs` swaps in a fake canonicalizer for this, but a member
  resolving through `nvs_runtime::capability::canonicalize` reaches `nvs_config::resolve::Disk` and
  takes no resolver, and Windows will not create a link without the privilege. Create the real one,
  return with a printed reason where the host refuses, and prove the assertions run once under
  `wsl.exe -- bash -lc 'CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo test …'`.
  [until: gone crates/nvs-runtime/src/capability.rs:resolve::Disk]
- **A `member` row in `docs/spec/02-php-migration.md` whose Novis cell names a class in prose alone is
  invisible to every walk over that table.** `migration_member_refs`'s second regex captures
  `Core\X::y` and nothing else, so the whole zlib group — fourteen `member` rows saying `Core\Compress`
  — read as covered while pinning no member at all, and both
  `every_migration_member_row_names_a_registered_member` and its conformance twin passed over it
  vacuously. Grep a family's rows for `::` before believing the table has them pinned, and spell the
  member into the cell in the slice that registers the class. [until: reviewed 2026-09-09]
- **A `-p nvs-stdlib` test cannot call another module's walk, because the guard that refuses a value
  is private to the module that owns it.** `csv::write_record`, `encoding::bytes_of` and
  `uri::scalar_text` all are, so a test asking several modules one question reaches their members
  the way a program does: `nvs_helper!` generates a `pub` symbol per member, and
  `nvs_runtime::call(crate::csv::nvs_core_csv_format, &mut Ctx::buffered(), &args)` drives one, with
  `ctx.take_pending()` for the refusal. Build the argument list its parameters take and release it
  yourself — the callee borrows its args. [until: reviewed 2026-09-09]
- **A `.nvst` case that pins a log record now pins its own line numbers.**
  `rule:errors/a-record-names-where-it-was-produced` puts
  `"source":{"file":"case.nvs","line":N}` on every `Core\Log::write` record and
  ` at case.nvs:N` on the plaintext one, so a comment added anywhere above a
  write moves an expectation that had nothing to do with it. Write the prose
  first and take the numbers from the runner's own `actual:` block afterwards,
  never the other way round. [until: reviewed 2026-09-09]
- **A test that writes one log record twice gets one line, and the second write leaves the buffer
  empty.** `Core\Log::write` goes through `nvs_runtime::floor::admit_log_record`'s table
  (`rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`), whose identity clears `ts`,
  `request_id`, `trace_id` and `span_id`, so two writes differing only in the request or the trace
  are one record and the failure reads as serde's `EOF while parsing a value`. Give each write its
  own message or `source`, and reach for `floor::expire_log_windows` only where the *count* is the
  subject. [until: reviewed 2026-09-09]
- **A `-p nvs-hir` member fixture reports the same `Class::member` miss twice, so a test that counts
  the diagnostics fails on a check that works.** One `self::missing()` in a method body comes back as
  two identical `E0309`s from `MemberResolver::check`, and every older test in that module asserts
  with `.any()`, which hides it. Assert over the whole filtered set — all of them carry the help, or
  none of them do — rather than indexing the first and pinning a count of one.
  [until: reviewed 2026-09-09]
- **A `budget::live_bytes` delta charges the first measurement in a test binary for what the core
  builds once and keeps.** The leaked per-core descriptor table lands inside the first member
  call's window — tens of kilobytes of fixed cost — so a small input and a large one measure
  nearly alike and no proportionality assertion holds. Measure a throwaway input first and drop
  the reading; the counters are thread-local, so nothing another test does beside it is in the
  number. [until: reviewed 2026-09-09]
- **A debug print in a `#[cfg(test)]` module fails two gates and neither is in the crate you
  edited.** `no_crate_the_server_links_writes_to_stdout` scans the *source* of every crate the
  language server links, so a `println!` in an `nvs-stdlib` unit test reads exactly like one in a
  member body, and clippy denies `print_stderr` workspace-wide, so `eprintln!` is not the repair.
  Carry the detail in the `assert_eq!` message instead, which is where a reader of the failure
  looks anyway. [until: gone crates/nvs-lsp/tests/stdout_policy.rs:no_crate_the_server_links_writes_to_stdout]
- **A check whose name says a value is "refused" can still be a *rows* test.**
  `a_null_written_into_a_non_nullable_option_is_still_refused` reads as a checker case and
  `crates/nvs-stdlib/Cargo.toml` names no `nvs-types`, so the first move looks like splitting the
  check. What that crate can hold is why the refusal must exist: the option's omission already arrives
  under `Tag::Null`, so a written one would be the argument the helper reads as not given. Ask what
  the row makes true before asking which crate compiles the refusal. [until: reviewed 2026-09-10]
- **A `.nvst` `foreach` that binds a *typed* variable over an array **literal** is `E0401: expected
  'string', found 'mixed'`, and the diagnostic points at the binding.** An array literal in an
  unannotated position interns as `array<mixed>`, so its elements are `mixed` whatever was written
  inside it, and the binding is the first place that becomes a mismatch. Declare the subject first —
  `array<string> $names = [...]; foreach ($names as string $name)` — which is what the corpus already
  does everywhere a `foreach` binding carries a type. [until: reviewed 2026-09-10]
- **A `crates/nvs-stdlib/src/` file may hold exactly one `#[cfg(test)]`, and a second one fails a
  test three directories away.** `crates/nvs-stdlib/tests/capability.rs`'s OS-gate scan stops at
  the first one in a file, so it asserts there is only one — a `#[cfg(test)] fn` fixture above the
  test module hides every real member under it from the scan, and the failure names the file rather
  than the rule. Put a shared fixture *inside* the test module and make the module `pub(crate) mod
  tests`, which is what `crates/nvs-stdlib/src/keyring.rs` does for the ring builders its own tests
  and `signature.rs`'s both use.
  [until: gone crates/nvs-stdlib/tests/capability.rs:has more than one]
- **A `.nvst` case that makes an in-process request runs its own top-level statements again on the
  far side, so a *generated* key is a different key there.** `Core\Test::request` answers with this
  same program's script frame (`rule:testing/in-process-request`), so a `Core\Crypto::generateKey()`
  above the call draws once in the parent and again in the child, and a link one of them signed never
  verifies for the other — the case then fails as though the member under test were broken. Write a
  fixed key (`Core\Bytes::fill(32, 65)`) for anything a case mints on one side of that call and checks
  on the other. [until: reviewed 2026-09-10]
- **An options-bag member refuses a non-literal options argument with `E0453`, not with a type
  mismatch.** A case pinning "the filter is one trailing shape" by writing
  `Core\Queue::purge("email", Core\Queue\State::Dead)` reads as an `E0401` about the shape's type,
  and the answer is `an options argument must be written out as {...} at the call site` — the
  options are flattened one per argument, so there is nothing for a value to be checked against.
  Write the case, run `target/debug/nvs.exe test <file>`, and paste back the line it printed.
  [until: reviewed 2026-09-10]
- **A `-p nvs-stdlib` test *can* run a real statement, on exactly one driver.** The bullets above
  are right that no `nvs_db::Connection` can be built there, and wrong by implication that no
  statement can run: `nvs_db::sqlite::open` is public and answers a bare `SqliteConn`, so an
  in-memory database is a real engine needing no container and no `NVS_DB_MATRIX_DRIVER`. Reach for
  it when the subject is a *statement* rather than a member, build the schema from the value the
  product uses so the fixture cannot drift, and see `crates/nvs-stdlib/tests/queue_sqlite.rs`.
  [until: reviewed 2026-09-10]
- **A `queue_sqlite.rs` fixture row left `Claimed` is claimed again by the next `claim(…, NOW,
  NOW)`.** The claim's second arm is the visibility timeout — `state = 1 and claimed_at <= ?` — so a
  cutoff *at* the instant a fixture wrote `claimed_at` makes that row due beside the pending one, and
  the helper answers the older id while the case asserts the one it just pushed. Pass `NOW - WINDOW`
  as the cutoff in any case that leaves a claimed row behind, and keep `NOW` for the cases whose whole
  queue is pending. [until: reviewed 2026-09-10]
- **A `.nvst` case cannot run `nvs queue migrate`, so a case over a converged database converges it
  itself.** `--RUN--`'s spellings are a closed set with no operator subcommand among them, and the
  schema has no program-reachable form — `nvs_stdlib::queue::schema()` is Rust. Take the DDL from
  `nvs queue migrate --config examples/queue-sqlite.toml --dry-run` into the case's own `--FILE--`,
  and create `nvs_dead_jobs` beside `nvs_jobs`: `status` reads a receipt back from both.
  [until: reviewed 2026-09-11]
- **An `.lspt` case whose cursor lands on a construct no case had reached costs twelve more cases, not
  one.** `nvs_lsp::coverage`'s vocabulary is every construct some cursor resolved to, so three cases
  before a `}`, a `)` and a `]` opened `Block`, `Paren` and `ArrayLiteral` as columns and
  `every_request_answers_every_construct` demanded four cursor rows and a whole-document row at each.
  Price it with `nvs lsp-test tests/lsp/ --coverage` first, and put a claim about *many* cursors in a
  `-p nvs-lsp` test, where it opens no column.
  [until: gone crates/nvs-lsp/src/coverage.rs:which is the ratchet]
- **A structural "exactly one construction site" test cannot count the type's own name.** `nvs-lsp`'s
  `SymbolIndex` is built through `Self::default()` rather than a struct literal and is re-exported by
  name from `lib.rs`, so counting `SymbolIndex {` finds only the declaration and counting `SymbolIndex`
  finds the export list — neither is a site where anything is built. Count the **private** entry type
  the site fills in (`Indexed {`), which no other module can name, and exempt `lib.rs` explicitly, which
  is what `crates/nvs-lsp/tests/index.rs` does. [until: gone crates/nvs-lsp/src/index.rs:struct Indexed]
- **Adding one LSP request fails a closed-list test in a file no goal's file set names.**
  `crates/nvs-lsp/tests/handshake.rs`'s `initialize_declares_exactly_the_capabilities_this_goal_ships`
  asserts the *exact* serialized capability key set, so one new provider field fails it with
  `Extra: [...]` from a suite the slice never opened. Its comment also names each absent request and
  why it is absent, so landing one leaves that comment arguing against the tree. Edit the expected
  set and the comment together with the capability.
  [until: gone crates/nvs-lsp/tests/handshake.rs:initialize_declares_exactly_the_capabilities]
- **A negative fixture naming call hierarchy fails a guard two files away.**
  `crates/nvs-lsp/tests/index.rs`'s `call_hierarchy_is_not_answered` greps every source file in the
  crate for `callHierarchy` and `CallHierarchy`, so `prepareCallHierarchy` written as the unknown
  request in `case.rs`'s closed-set test fails with a message about a second index nobody asked for.
  When a test needs a request name that will never be answered, pick a real LSP method no rule has an
  opinion on — `moniker` is what that test uses now — rather than the one name a rule guarantees will
  never be ours. [until: gone crates/nvs-lsp/tests/index.rs:fn call_hierarchy_is_not_answered]
- **A parameter-name hint is drawn *inside* the literal it annotates, so a `.lspt` case freezing one
  opens a new column in the coverage matrix.** A case answering `times:` over `greet("world", 2)`
  credited `inlayHint` with `Int`, which no cursor request had a case at, so the coverage gate came
  back asking for seven cases nobody meant to owe. Freeze the rendering at a construct the corpus
  already reaches — a second `string` argument does it — and leave the shape that opens a column to
  the crate's own Rust test.
  [until: gone crates/nvs-lsp/tests/coverage.rs:every_request_answers_every_construct]
- **A regex stripping `/* … */` out of a TypeScript source eats a glob with it: `"**/*.nvs"` opens a
  block comment two characters in, and the next `*/` is inside the next glob.** An assertion over the
  stripped `editors/vscode/src/tests.ts` reported a client that never looks for a `.nvst` file,
  because `const CASES` had been deleted with the line above it. Drop comment *lines* instead —
  `!/^\s*(\/\/|\/\*|\*)/` over the split source — which is exact where every block comment is a doc
  comment on its own lines, and blind to what a string holds. [until: reviewed 2026-09-11]
- **A `.lspt` case whose document holds a construct no other case reached adds a column to the coverage
  matrix, and `every_request_answers_every_construct` then owes a case for every cursor-taking request
  at it** — one `regions` case over inline HTML cost seven more. The vocabulary is the corpus's own and
  nothing declares it (`crates/nvs-lsp/src/coverage.rs:56`), so a construct exists the moment one case
  reaches it. Run `target/debug/nvs.exe lsp-test --coverage tests/lsp/` before `cargo test -p nvs-lsp`:
  the new column is a row of dots with one number in it. [until: reviewed 2026-09-11]
- **A case that is red on purpose spends `nvs-test`'s whole 60 s `CASE_TIMEOUT`, and an unbounded
  growth loop spends it on the host's memory.** `while (true) { $a .= $a; }` under a 16 MiB ceiling
  neither breaches nor aborts inside that timeout: it doubles into the machine's RAM and is killed
  on the clock. Bound such a loop just past the ceiling it must be stopped at — twenty-four
  doublings of five bytes, in
  `tests/conformance/error/a-loop-that-calls-nothing-is-stopped-by-the-memory-ceiling.nvst` — and
  leave a pure-CPU spin unbounded, since only the first takes the host with it.
  [until: reviewed 2026-09-11]
- **A balance assertion across a primitive that *reports* a refusal reads the report as a leak.**
  `abi::report_refusal` runs `Ctx::memory_breach`, which `format!`s the message the `FATAL` carries,
  so `nvs_array_append`'s refusal moves `budget::live_bytes` by a couple of hundred bytes while
  `nvs_array_set`'s moves it by nothing. Assert "allocated nothing" on the primitives that answer a
  degenerate value, and assert the status plus the unchanged operand on the ones that answer a
  status. [until: gone crates/nvs-runtime/src/ctx/limits.rs:pub fn memory_breach]
- **A `-p <crate>` check whose fixture you found in another crate's tests can already have a twin in
  the target crate's own `#[cfg(test)]` module.** `crates/nvs-host/tests/limits.rs`'s `closure_of` is
  the visible builder of a registered handler, but `crates/nvs-runtime/src/ctx/hooks.rs`'s test module
  holds `hook_of`, which builds the same arity-1 closure out of `ClassTable`, `MethodRow` and the
  `CLOSURE_*` constants — all of them `nvs-runtime`'s own surface, so nothing has to move crates.
  Grep the crate the check names for `ClassTable::new` before concluding its `args` is the wrong half.
  [until: gone crates/nvs-runtime/src/ctx/hooks.rs:fn hook_of]
- **A fixture that pins a *waited* bound with `Core\Time::now` flakes on the WSL leg, and the
  failure accuses the code it is pinning.** `examples/pool.nvs` read the wall clock across a
  500ms pool `acquire` whose deadline is a monotonic `Instant`, so a realtime step under the
  wait — which WSL2 takes — reported 464ms and printed a verdict accusing the pool of refusing
  early. Measure an elapsed interval with `Core\Time::monotonic()->minus($mark)->toMilliseconds()`,
  and read `rule:http-server/every-deadline-is-monotonic` for why the two clocks are not
  interchangeable. [until: reviewed 2026-09-11]
- **A `Core` member's refusal *message* is not on the `Err` that `nvs_runtime::call` answers with.**
  That `Err` is a status code — `nvs_runtime::THROWN` — so destructuring it as a `Fault` is an
  `E0308` against `&i32`, and there is no message on it to read. The throw is on the context:
  `ctx.pending_class()` is the `catch` name, `ctx.pending()` is the sentence, and `ctx.take_pending()`
  clears it so a later assertion on the same context is not reading the first refusal.
  [until: reviewed 2026-12-11]
- **A case reading a program's own output off the wire asserts nothing once a door replaces that
  response, and it still goes green.** A connection door writes the response itself, so the line the
  request wrote is nowhere on the wire and a `contains` over what is left can still hold. When a
  slice makes a door write the response, move the request's claims onto the fixture's own reporting
  and re-check what each `read_until` needle now matches. [until: reviewed 2026-10-11]
- **Registering a `Core` class is gated by the three-case conformance floor, so a class nothing
  builds yet has to open its floor with refusals.** `Core\Sse\Message` has no constructor and no
  member handing one back, so its three cases are compile-error cases that still type-check the
  whole file. Run `target/debug/nvs.exe test tests/conformance/core/<case>.nvst` and paste the
  diagnostic back rather than predicting it: a missing member is *no method named* on an arrow and
  *no member named* on a `new`, and a `return` of an unknown call raises a second error the expected
  block would then have to carry. [until: reviewed 2026-09-11]
- **A handoff item can call a `.nvst` reject case "behaviour that is already landed" when the refusal
  it pins does not exist yet.** Stage 4's `a-catch-arm-naming-the-finish-marker-is-refused.nvst` needed
  a diagnostic no crate had written, and `catch (Core\Script\Finished $e)` compiled and ran instead.
  One `target/debug/nvs.exe run` over a three-line probe in `.agent-tmp/` settles it before any case is
  written — the binary is already built at the commit the session opens on, so the probe costs one call
  and no build. [until: reviewed 2026-09-12]
- **A goal stage's *other* check can already own the file your test belongs in, and `Write` replaces
  it whole.** `crates/nvs-codegen/tests/markup.rs` held the three cases of stage 4's sink check, and
  a new file under the obvious name for a markup test landed on top of them — the harness reports
  that as "updated" rather than "created", which is the only tell before the deleted tests show up
  as a check that stopped passing. Before writing a new test file, check whether the goal's other
  checks name tests that would live in it, and if a `Write` reports "updated", recover the original
  with `git show HEAD:<path>` and append instead. [until: reviewed 2026-09-12]
- **An `nvs fmt` case that asserts a file is unchanged fails on a rule it was not written about.**
  `assert_eq!(formatted(source), source)` holds only if the input is already canonical under *every*
  landed rule, so a control-structure brace on its own line, or a `?>` at column 0 inside a block,
  fails the case for a reason that has nothing to do with what it asserts. Write the input as the
  formatter already answers it and vary only the bytes under test — the assertion prints both whole
  files, and the line that differs names the rule that actually moved.
  [until: reviewed 2026-09-12]
- **An `nvs-fmt` case panics with "does not parse" when its fixture breaks a *semantic* rule rather
  than the grammar.** `format` refuses on any error the parse reported, and the parse reports
  `rule:classes/no-free-functions-or-constants` too, so a top-level `function` in a fixture reads as a
  syntax error in the panic. Put a fixture's code in a class's `public static function`, the shape
  `examples/match.nvs` already has. [until: gone crates/nvs-fmt/src/lib.rs:d.is_error()]
- **A second `#[cfg(test)]` in `crates/nvs-stdlib/src/lib.rs` fails a guard test, not the build.**
  `crates/nvs-stdlib/tests/capability.rs`'s `nvs_stdlib_reaches_the_os_only_through_the_gate` scans
  a source file down to its *first* one and asserts there is one, so a test-only `mod` among the
  shipped ones hides everything under it. Declare a helper the whole crate's cases share inside the
  foot of `lib.rs`, as `granting` is. [until: gone crates/nvs-stdlib/tests/capability.rs:has more than one]
- **A local declared inside a `foreach` body is declared for the whole `.nvst` file, so two loops
  cannot both call their subject `$key`.** A case is top-level statements under one variable scope,
  so the second `Core\Crypto\PublicKey $key = …` is `E0406` — and it points at the *inner* line with
  "first declared here" on the other loop, which reads as though a loop body were a scope and the
  two declarations were somehow the same one. Give each loop's subject its own name, or declare the
  local once above both loops and only assign inside them.
  [until: reviewed 2026-09-12]
- **A shape-typed declaration at the start of a statement is `E0117`, not a type.** `{sub: string}
  $claims = …` parses its `{` as a block, and the fix the diagnostic offers — wrap it in `({…})` —
  is for an object *literal* in expression position, so following it produces three more errors
  rather than one declaration. Write `var $claims = Core\Json::decodeAs<{sub: string}>(…)` instead:
  the written type is already on the call, and a shape type still reads fine in a parameter or a
  return position, where no block can begin. [until: reviewed 2026-09-12]
- **A `Core` member row cannot land without its body and three cases, so a handoff group that splits
  "register the member" from "write the body" has no green state in between.**
  `every_part_one_member_has_a_conformance_case` and `every_core_class_has_a_conformance_floor_of_three`
  both read the registry, and `BELOW_THE_FLOOR` is empty and only shrinks, so the row is red from the
  moment it exists until three cases call it. Cut the slices so the whole member — row, card, helper,
  `address()` arm and cases — lands under one verification, and let the per-slice commits be the
  seam instead. [until: reviewed 2026-12-12]
- **A field of `webcrypto.json` is not text just because a dump printed it as one.** `iterations`,
  `clock` and `lifetime` are JSON numbers and `deterministic` is a bool, so
  `webcrypto::text(case, "/iterations")` compiles and then panics at run time with *has no text at
  /iterations*, and a Python dump through `str()` prints all four as quoted strings and hides it. Read a
  count with `webcrypto::number` (`crates/nvs-stdlib/src/tests/vectors.rs:89`) and print
  `type(v[k]).__name__` rather than the value when checking what a pointer will answer.
  [until: reviewed 2026-09-12]
- **A published vector typed from memory is wrong about as often as it is right, and `WebFetch` cannot
  reach an appendix that sits late in a long RFC.** RFC 5903 § 8.1's initiator scalar came back wrong
  after its first two words, and a fetch of `rfc7515.txt` truncates before Appendix A.3, so neither
  recall nor the obvious fetch is a source on its own. Fetch the document where it is short enough to
  arrive whole, and otherwise lean on the vector being self-checking — a scalar, key or signature with
  one digit wrong cannot agree or verify, so a passing test has confirmed its own literals.
  [until: reviewed 2026-09-12]
- **A `uint` parameter refuses an `int`, and a `foreach` over a table of counts is where a case meets
  that.** `Core\Crypto::deriveKey`'s `$iterations` and every other counted argument are `uint`, and an
  `int` reaching one is `E0401 expected uint, found int` at the call rather than a widening, so a
  bounds sweep declared `array<int>` compiles everywhere except the line that matters. Declare the
  table `array<uint>` and the loop variable `uint`. [until: reviewed 2026-09-12]
- **A line in `--EXPECT--` that begins with `--` is read as a section header, and the case fails as
  *"not a valid case: unknown section"* rather than as a mismatch.** The `.nvst` reader splits on
  `--NAME--` before it compares anything, so a frozen multipart body or a diff-shaped expectation
  ends the `--EXPECT--` block where its first `--` line starts. Normalise it away in the case, as
  `tests/conformance/core/http-client-streams-a-file-as-the-whole-body.nvst` does with
  `Core\Str::replaceAll`, rather than looking for an escape the format does not
  have. [until: gone tests/conformance/core/http-client-streams-a-file-as-the-whole-body.nvst]
- **An array literal written straight into a `CoreTy::Union` arm is typed before the arm is
  considered, so `["a", "b"]` against `string|array<string>` is refused as `array<mixed>`.** The
  checker places an enum-case or a scalar literal inside a union, but an array literal infers its own
  element type first and then fails to assign, which reads as the option's type being wrong rather
  than the literal's. Bind it to a typed variable one line up — `array<string> $lines = ["a", "b"];`
  — and pass that, which assigns cleanly and documents the arm at the call site.
  [until: test an_array_literal_is_placed_against_a_union_arm]
- **A `TcpStream` accepted from a non-blocking `TcpListener` is non-blocking on Windows and blocking
  on Linux.** Windows' accept inherits the listening socket's mode and Linux's does not, so a
  handler that reads `WouldBlock` as the end of a connection closes it in the client's face and the
  case fails with a reset it never asked for. Say `stream.set_nonblocking(false)` on every accepted
  connection before reading it, as `transport.rs`'s `answer` does.
  [until: gone crates/nvs-stdlib/src/http/transport.rs:set_nonblocking]
- **A `Core` refusal's exact text is pinned by a conformance case, so `cargo test` stays green while
  `verify.py` fails at step 7 on a reworded message.** The `.nvst` expectation holds the whole
  sentence, and changing only the advice half of one is enough to fail it. Grep
  `tests/conformance/` for a distinctive phrase of the message before you reword it rather than
  after the verify run. [until: reviewed 2026-09-13]
- **An all-`null` argument array in a `Core\Http\Client` test writes a body.** `[Value::null();
  REQUEST_ARITY]` leaves the `json` slot holding `Tag::Null`, and that key omits as
  `Const::NeverWritten`, so `judge_verb` refuses the call as a `GET` carrying a body before the case
  reaches the thing it is about. Write `args[JSON] = Value::unset()` in any request test that is not
  about the body. [until: gone crates/nvs-stdlib/src/http.rs:Const::NeverWritten]
- **A conformance case counts toward a class's floor only where its `--FILE--` body writes the class
  name, and a nullable return does not carry the name in for it.** `corpus::holders` attributes every
  `->member(` in a case to the classes that case *mentions*, plus one class per member whose return type
  is a bare `CoreTy::Instance` — a `CoreTy::Nullable(&CoreTy::Instance(…))` is deliberately excluded,
  since a case holding one has narrowed it first. So a class reached only through a nullable answer needs
  its name written inside `--FILE--`, in a comment if nowhere else; the `--TEST--` title is not read at
  all.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:fn holders]
- **A `Core` member test's own argument bag cannot leave `headers` as `Value::null()`.** The
  compiler always passes an empty array there, so the member answers `Fatal(… expected Array for
  headers, got tag 0)` before it reaches what the case is about — while an unwritten *body* key is
  `Value::unset()`. Write `Value::array(NvsArray::new())` into that slot and release it beside the
  URL. [until: reviewed 2026-09-13]
- **A `[cache.process] max_size` case is sized against a shard's *share*: the tier is 64 shards and each
  evicts against `max_size / 64`.** A cap of `3K` — the local tier's own eviction case's figure — leaves
  a shard 48 bytes, under `ENTRY_OVERHEAD`, so every entry is forgotten as it arrives and the case
  asserts nothing. Keep an entry under the share, cross the whole cap several times over, and assert the
  bound rather than which key went, because which shard a key lands in is a hash's business.
  [until: reviewed 2026-09-13]
- **`??` binds tighter than `as`, so `$store->get($k) ?? "" as string` casts the *fallback* and
  leaves the whole expression `string|mixed`.** The obvious spelling for reading back a
  `mixed`-returning member is `string $back = $x->get($k) ?? "" as string;`, and it fails
  `E0401: expected string, found string|mixed` with the caret under the whole expression — which
  reads as though `??` had not removed the `mixed`, rather than as a precedence question. Write the
  coalesce in parentheses, `($x->get($k) ?? "") as string`, which is what every case reading a
  `mixed` back wants. [until: reviewed 2026-09-13]
- **An option of a trailing bag is not a named argument at the call site, and writing one is `E0486`
  listing parameters the member never documented.** `$store->getSecret("k", $ring, fill: $f)` is
  refused with "the parameters are: `key:`, `keys:`, `options:`", because the bag is one parameter
  and `rule:core-api/shape-rules` R2's "callable by name" is about the keys *inside* it. Write the
  bag out — `$store->getSecret("k", $ring, {fill: $f})` — which is the spelling every `put`-with-a
  -`ttl` case already uses. [until: reviewed 2026-09-13]
- **`tungstenite` refuses a `101` that chose an unoffered subprotocol before Novis sees it.** Its
  handshake compares the reply's `Sec-WebSocket-Protocol` against the offers it sent and fails with
  `Server sent an invalid subprotocol`, naming neither the protocol nor the member, so a case
  asserting Novis's own wording against a live origin fails on a refusal that was correct. Assert
  that wording of `transport::settled` directly — the judgement `openSocket`'s scripted arm reaches —
  and assert of the live `101` only that it was refused. [until: reviewed 2026-11-12]
- **A `??` default that is not itself `tainted` makes the binding's type a union, not the tainted
  one.** `string $t = $m?->text() ?? "";` is `E0401: expected string, found string|tainted string`,
  because the join keeps both spellings rather than widening to the marked one — so a case written to
  pin a reader's qualifier reads as being refused for the union instead, and its `--EXPECTF-ERROR--`
  freezes the wrong claim. Write the default with the mark on it, `?? ("" as tainted string)`, and the
  diagnostic says `found tainted string`. [until: reviewed 2026-09-13]
- **A loopback peer that answers a close by hanging up reads as a protocol error, not as the end.**
  `tungstenite` writes its close echo only on the next read, write or flush, so a peer thread that
  breaks out of its loop and drops the socket sends a bare FIN and the client's codec answers
  `ResetWithoutClosingHandshake`. A case asserting `null` after a close rests on the read loop in
  `crates/nvs-stdlib/src/http/socket.rs` reading a failure after *this* end closed as the end; one
  about the *peer* closing has to make the peer flush first.
  [until: gone crates/nvs-stdlib/src/http/transport.rs:talking_origin]
- **A `switch` label or a `match` arm written as a bare digit run never crosses representations, because
  it is lowered against the subject's own type.** `lower_switch` and `lower_match` pass
  `Some(subj_ty)` to `lower_expr`, so `case 2:` beside a `float` subject is a `ConstFloat` and a case
  meant to pin the cross-representation row pins the matched-pair one instead, silently. Write the
  other side as a binding — `uint $two = 2;` then `case $two:` — or as a literal no placement can move
  (`2.0` beside an `int`), and read the answer back rather than trusting the shape.
  [until: gone crates/nvs-ir/src/lower/control.rs:Some(subj_ty)]
- **`$x is Iterable` with no type argument is `E0442`, so an agreement row over `iterable`'s members
  cannot be spelled the obvious way.** `rule:iteration/concrete-generic-implements` makes the argument
  mandatory at every site naming either reserved interface, an `is` test included, and the diagnostic
  says so where a reader expects a bare interface name to work. Write `$x is Iterable<int> || $x is
  Iterator<int>`: the walk compares the label alone, so which argument is written never changes the
  answer. [until: reviewed 2026-09-14]
- **A group test that cancels the task awaiting it sees `Outcome::Cancelled` only when that task
  stands on a `HelperFrame`.** A stack carrying no helper frame is one the scheduler may force-unwind,
  so a bare Rust closure parked inside `run_group` is torn down where it parks and never reaches the
  line that records the answer — the assertion then reads as though the group answered something else.
  Open `nvs_runtime::HelperFrame::enter()` at the top of the awaiting task's body, which is what a real
  `Core\Task::all` stands on.
  [until: gone crates/nvs-host/src/scheduler.rs:unwindable]
- **`python tools/try.py` runs a scratch `.nvst`'s `--FILE--` and nothing else, so a case whose claim
  needs a request or a sibling file answers the wrong question there.** `--GET--`, `--HEADERS--` and
  `--FILE <path>--` are the conformance runner's sections: under `try.py` the child script never lands
  beside the program and `Core\Request` refuses exactly as it does in any CLI program, which reads as a
  broken case rather than as a runner that was never asked. Iterate such a case with
  `target/debug/nvs.exe test <path/to/case.nvst>`, which takes one file as readily as a directory.
  [until: reviewed 2026-09-14]
- **A child isolate's `Failure` crosses back as `class: "Error", message: ""` when the parent fixture
  has no error class.** `isolate::finish` renders it through `Ctx::take_thrown`, which reads the
  *parent's* `set_runtime_error_class` — `Ctx::isolate` copies it down — so a case built on a bare
  `Ctx::new` asserts against an empty failure and reads exactly like a boundary that dropped the
  child's breach. Give the parent a one-class `ClassTable` first, as
  `crates/nvs-host/src/isolate.rs`'s `parent` does. [until: reviewed 2026-09-14]
- **A Problems-panel assertion cannot tell the Task's diagnostic from the server's.** `nvs lsp`
  publishes with `source: "nvs"` (`crates/nvs-lsp/src/diagnostics.rs:216`) and the `$nvs` problemMatcher
  sets the same source and the same code, so `languages.getDiagnostics` answers entries the API gives no
  way to separate. Withhold the server for that assertion — `nvs.lsp.enable` false, wait for its entries
  to clear, run the task, restore it in a `finally` — as
  `editors/vscode/test/host/surfaces.test.ts:112` does.
  [until: gone crates/nvs-lsp/src/diagnostics.rs:code_description: None]
- **A case that creates a file under `std::env::temp_dir()` passes here and fails on Linux**, where
  `/tmp` is mode 1777 and `nvs_config::trust::check` walks the parents of what it is handed. Windows
  has no such parent and `verify.py` here never runs the Unix half, so it first fails in the WSL
  leg. Use a directory beside the test binary, as `crates/nvs-server/src/control.rs`'s `scratch`
  does. [until: reviewed 2026-09-14]
- **A `-p nvs-cli` case that writes a tree under `scratch("x")` and then drives it through
  `asked("x", …)` loses the tree.** `ctl.rs`'s `scratch` empties the directory it hands back, and
  `asked` reaches it again through `endpoint`, so the `nvs.toml` the case just wrote is deleted
  before the reload re-reads it — and the failure arrives as `E0605: cannot read …nvs.toml` from
  inside the server's answer rather than from the case. Give the tree a name of its own
  (`scratch("reload-live-tree")`) and leave the endpoint's to `asked`. [until: reviewed 2026-11-01]
- **A `.nvst` case cannot carry a body that is not UTF-8.** `--POST_RAW--` crosses to
  `nvs run --request` as a `String` (`crates/nvs-test/src/request.rs:100`), so nothing in that path
  spells one. Build the octets in the program instead — `Core\Encoding::fromHex` into
  `Core\Test::request`'s `body` key. [until: gone crates/nvs-test/src/request.rs:pub body: Option<String>]

- **`echo` inside an in-process request escapes its string**, so a `--EXPECT--` quoting a throw's
  message sees `&quot;` and `&#39;`. Write the message with no `"` and no `'` in it, or expect the
  entities. [until: reviewed 2026-09-15]
- **A comment added inside a `--FILE--` block moves every line number its `--EXPECTF-ERROR--`
  pins, and the offset is four: source line 1 is the `<?nvs` on file line 4.** Rewording the
  prose above the code in
  `tests/conformance/core/a-route-capture-is-the-value-the-match-converted-not-the-segment-text.nvst`
  turned a green case red with a `case.nvs:18` against a `case.nvs:19` and nothing else changed.
  Keep a comment rewrite the same number of lines when the case pins a diagnostic, or update the
  expectation in the same edit — and read the failure's `-->` before looking at the message, since
  the message is what you did not touch. [until: reviewed 2026-09-15]
- **A child's ceiling is what *remains* of its parent's budget, so a sub-cap case asserting an exact
  byte count fails by a few kilobytes.** `Ctx::narrow_under` resolves a narrowing against `Remains`
  (`crates/nvs-runtime/src/ctx/isolate.rs:590`), and the shortfall is whatever the run has spent by
  then, which is not a constant. Assert the direction — tighter than the parent's ceiling for a
  narrowing, no wider for a refused widening — reading the parent's own number while it is still
  reachable. [until: gone crates/nvs-runtime/src/ctx/isolate.rs:Remains]
- **A `nvs_config::Snapshot` built by hand with `..Default::default()` configures nothing, however
  complete its typed `config` is.** A directive is read through `nvs_config::Request::get`, which
  looks in `Snapshot::table` — the raw `toml::Table` a boot kept — so a case that filled only
  `config` gets an uncapped context that grants nothing, and the assertion that fails is about the
  ceiling rather than about the missing half. Fill both from the same written text, which is what
  `crates/nvs-cli/src/serve.rs`'s `tree_of` does. [until: reviewed 2026-09-15]
- **A test over `nvs_server::metrics::every_core` sees every other test in the binary that served a
  request, because the roster is one per process.** Asserting a length or a total is therefore a race
  against whatever the harness scheduled alongside, and it passes alone and fails under `cargo test`.
  Give the case its own `route` label and assert on that series by name
  (`crates/nvs-runtime/src/metrics.rs:1249`), and hold the cores alive across the gather with a
  `Barrier` — a thread that exited is indistinguishable from one the roster never reached.
  [until: gone crates/nvs-runtime/src/metrics.rs:static CORES]
- **A served request is an isolate, and an isolate used to be armed no memory ceiling at all** — a
  `[limits] memory` case written against `nvs run` passes while the same program served over a
  socket runs to its end. `Ctx::new` displaces the thread's threshold with `Armed::NONE`, and a
  child whose own `memory_limit` stayed `0` answered `false` to `over_memory_limit` at every poll,
  so neither half of the ceiling could fire. Assert a ceiling on the path a request takes — through
  `Isolate::run` or the accept loop — never on a root `Ctx`, the one shape that was already right.
  [until: gone crates/nvs-runtime/src/ctx/isolate.rs:isolate.set_memory_limit(remaining]
- **A `foreach` binding inside a closure crashes the lowering when a binding of the same name is in
  scope outside it.** `nvs_types` resolves the inner name to the outer binding and records it as a
  capture, so `nvs-ir` panics with `the closure at 0:NN..NN captures `$name`, which is not bound in
  the enclosing frame` rather than reporting anything a case could expect. Give the inner binding its
  own name — no `--EXPECTF-ERROR--` catches this, because the failure is the compiler's process
  exiting 101 and not a diagnostic. [until: reviewed 2026-09-15]
- **`as` binds tighter than `==`, so a comparison written for an `echo` needs its own parentheses.**
  `$a == $b as string` parses as `$a == ($b as string)`, and a `.nvst` case surfaces it as `E0466:
  disjoint` plus `E0710: cannot be converted` naming the *right* operand — neither diagnostic says
  "precedence", so the case looks like a type error in the values rather than in the punctuation. Write
  `($a == $b) as string`, or better render the member's own text (`$at->toIso()`) so the line asserts a
  value rather than a `bool`.
  [until: reviewed 2026-09-16]
- **A `Core` class reached only through an `array<T>` return is held by no case that does not
  name it.** `crates/nvs-stdlib/tests/corpus/mod.rs`'s `Attribution` follows a `CoreTy::Instance`
  return to the class it builds and stops there, so `$info->methods()` then `$row->name()`
  attributes nothing to `Core\Reflect\MethodInfo` and the floor of three fails over members the
  case plainly asks. Write the class's own name in, which a `foreach (… as Core\Reflect\MethodInfo
  $row)` binding already does.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:CoreTy::Instance(made)]
- **Two `== null` tests joined by `||` narrow neither receiver, and the `else` is where it bites.**
  `if ($a == null || $b == null) { … } else { $a->member(); }` is four `E0459`s — the narrowing runs
  per test, so only a nested `else if ($b == null)` leaves both receivers non-nullable in the final
  arm. Two descriptions from `Core\Reflect::forClass` in one case is the shape that meets it, and the
  nested spelling is what every such case is written in. [until: reviewed 2026-09-16]

- **A case that reaches an instance only through a `?T`-returning member counts as asking that class
  nothing.** `Attribution::holders` takes the top-level return type alone, so a case whose only door
  is `Core\Reflect::forClass` (which answers `?Core\Reflect\ClassInfo`) holds no class, every `->member(`
  in it attributes to nobody, and a new member reads as `asked by 0 case(s)` however many cases call
  it. Name the class in a comment — or reach it through a member returning the bare `Instance` — and
  the same cases count. [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:holders]
- **A `Core` class only ever answered *inside an array* is attributed to no case, and its members
  read as asked by zero.** `crates/nvs-stdlib/tests/corpus/mod.rs`'s `Attribution::builds` maps a
  member to the class it builds from `CoreTy::Instance` and `InstanceAt` alone, so a row class
  reached through `array<PropertyInfo>` holds a case only where the case spells its qualified name.
  Write that name in — a typed `foreach ($roster as Core\Reflect\PropertyInfo $row)` does it.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:CoreTy::Instance(made)]
- **A `Fault::` message opening on a format hole cannot be asserted, and a `///` does not declare
  the site.** `every_error_path_is_asserted_or_declared_unreachable` keys a site by the literal text
  before its first `{`, so `Core\Metrics::{member}: …` had the stem `Core\Metrics::`, which no case
  could match. Put plain words before the first hole, and the `unreachable from source` sentence in
  a `//` comment within eight lines above the `Fault::` line — not on the enclosing `fn`.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_error_path_is_asserted_or_declared_unreachable]
- **A `.nvst` case that names a variable `$argv` collects a second diagnostic the case never asked
  for.** `rule:statements/no-host-populated-variables` refuses every host-populated name, so the line
  carries `E0211` *as well as* whatever it was written to pin, and an `--EXPECTF-ERROR--` block written
  for one error a line silently needs two. Spell an argument list `$words` or `$parts` in a case; that
  rule's own table is the reserved set. [until: reviewed 2026-09-16]
- **A `Core` row's key material is not always a `Secret*` leaf, and a registry walk that assumes it is
  reads two protocol members as taking no key at all.** `Core\Jwt::signObject` declares its key as
  `CoreTy::Instance(Core\Crypto\KeyPair)` and `Core\Signature::sign` carries its ring inside a
  `CoreTy::Shape`, so a predicate matching only `SecretBytes`/`SecretBlob`/`SecretStr` misses both.
  Walk `Shape` and `Options` as well as `Array`, `Union` and `Nullable`, and name the key-carrier
  classes — `registry::tests::no_protocol_class_exposes_a_raw_value_accessor`'s `CARRIERS` is that
  list. [until: gone crates/nvs-stdlib/src/registry.rs:CARRIERS]
- **A regex tier read back out of `ExprTypeTable` covers `Core\Regex::compile` and nothing else.**
  `crates/nvs-types/src/intrinsics.rs`'s roster carries one Regex row, so a literal handed straight to
  `matches`, `match`, `matchAll`, `replace`, `replaceWith` or `split` is never folded and has no
  recorded tier — a test counting `regex_tiers()` over the conformance suite sees an eighth of the
  patterns it looks like it sees. Find the pattern argument through `nvs_syntax::walk` and the class's
  own registry rows instead, and settle each one by handing it to `compile`, which is what
  `every_literal_regex_pattern_in_the_suite_has_its_tier_recorded` does. [until: reviewed 2026-09-16]
- **The one-second JWT case fails about one run in many, with `expired at N and it is now N` on the
  line that should verify.** Waiting for the second to turn before signing narrows that window
  without closing it, because `Core\Jwt::sign` reads the clock again itself. Re-run the case alone
  before believing a red conformance step; the fix is for it to assert against the `exp` the token
  carries rather than the second the spin saw. [until: reviewed 2026-09-16]
- **A `.nvst` case that does arithmetic on a length needs two conversions nothing in the corpus
  makes obvious.** `Core\Bytes::length` answers `uint` while `Core\Bytes::slice`'s length parameter
  is `?int`, so `Core\Bytes::length($b) - 5` is `E0401`; and `$a / $b` is `int|float`, whose
  `as int` **throws** on a non-integral value rather than truncating, so a loop that splits a buffer
  into equal pieces needs `Core\Math::intDiv`. Write `(Core\Bytes::length($b) as int)` for the
  subtraction and `Core\Math::intDiv($a, $b)` for the division, and run
  `target/debug/nvs.exe test <one case>` before believing either shape.
  [until: reviewed 2026-09-16]
- **A conformance case can quote a known gap's own refusal message, so closing the gap fails a case
  whose name says nothing about it.** A gap's message is observable behaviour while the gap stands,
  so pinning it was right — `json-an-absent-optional-field-reads-apart-from-an-absent-required-one`
  carried "…is `nvs_stdlib::json`'s own known gap" in its `--EXPECT--`. Before verifying a gap
  closure, grep `tests/` for a phrase of the message you are deleting, and rewrite the case holding
  it to assert the new behaviour. [until: reviewed 2026-09-16]
- **`every_error_path_is_asserted_or_declared_unreachable` keys on a message *stem*, so deleting an
  echoed refusal from a case un-covers every `Fault::` site whose message opens the same way.** One
  case echoing `` Core\Json::decodeAs(): `…` `` was covering four unrelated engine faults, and moving
  that refusal to a compile-time one left all four owing. Declare each site instead — `unreachable
  from source` plus the code that refuses first, within 8 lines and **on one line**, since the gate
  matches the phrase per line and a wrapped one is invisible.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION]
- **Moving a `<|>` cursor inside an existing `.lspt` case empties a cell of
  `crates/nvs-lsp/tests/coverage.rs`'s matrix, naming a construct the case still contains.** The
  matrix is keyed on the node the *cursor* sits in, and a qualifier is a node of its own:
  `Sta<|>tus::Draft` covers `ConstFetch`, `Status::Dr<|>aft` covers `ClassConstAccess`. Where a case
  is the only one covering its cell, change what it expects rather than where its cursor sits.
  [until: gone crates/nvs-lsp/tests/coverage.rs:empty cell]
- **A grep for a Novis construct over `tests/differential/` mostly matches PHP, not Novis.** Every
  case there carries a `--FILE--` Novis half and an `--ORACLE--` PHP half in one file, so a sweep
  measuring how much of the corpus a language change would break counts every `__construct` and
  free `function` in the oracle as a Novis site and can overstate the blast radius by an order of
  magnitude. Scope the measurement to `tests/conformance/` — which has no oracle section
  ([the bullet above](playbook.md)) — or split each differential file at `--ORACLE--` before
  counting. [until: reviewed 2026-09-16]
- **A `%2$s` in a *double-quoted* Novis literal is an interpolated `$s`, not a `Core\Str::format`
  placeholder.** Interpolation happens in the lexer, long before the intrinsic pass reads the
  template, so a positional-placeholder fixture written with double quotes fails as ``E0301 `$s` is
  not declared`` and pins nothing about `format`. Single-quote any template containing `$`
  (`'%2$s'`), which is what the cases in `crates/nvs-types/tests/intrinsics.rs` already do.
  [until: reviewed 2026-10-16]
- **A fixture writing `Core\X::member(...)` for a member with an options bag panics in lowering
  rather than failing to check.** The thunk a first-class callable synthesizes asks each parameter
  for one erased type, and a `Core` options bag has none — it is flattened a slot at a time at an
  ordinary call and never at a thunk. Use a member with no options bag for a callable-reference
  fixture, or assert through an ordinary call; the panic is in `crates/nvs-ir/src/lower/mod.rs`.
  [until: gone crates/nvs-ir/src/lower/mod.rs:shape parameter was erased]
- **`parent::Name` in a type position does not parse, so a `reject` case naming it pins a
  twenty-error cascade rather than the refusal it meant to.** The type grammar takes `Owner::Name`
  and a bare `Name`; `parent::` is neither, so the parser stops at the `::` and re-reads the rest of
  the method as class members, each with its own diagnostic. Assert what a subclass's *own* name
  gives — `Sub::Id` is `E0405` — and leave `parent::`/`self::`/`static::` to a case about the type
  grammar itself. [until: reviewed 2026-09-17]
- **A ratio guard in `benches/abi-probe/tests/perf_guards.rs` measures the guards beside it: libtest
  runs a binary's tests on as many threads as the host has cores.**
  `a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names` inverts
  to under 1x in the full binary and is healthy alone, its placed half starved of the cores it fans
  out onto. Every guard there takes `serialised()` first and
  `every_guard_in_this_binary_takes_the_lock` counts the line, so write a new one holding it.
  [until: gone benches/abi-probe/tests/perf_guards.rs:fn serialised]
- **The process's drain bit is begun by other cases in the same test binary, so a new case may not
  rest on it.** `nvs-cli`'s serve cases end an in-process server by dropping `EndsTheServer`, which
  calls `nvs_server::Draining::process().begin()`, and that bit is one per process rather than one
  per case — a case reading it reads whatever its neighbours have already done to it. Take
  `nvs_runtime::Drain::detached()` for anything a case has to be able to *not* drain, and end what
  the case started by withdrawing it rather than by draining the process.
  [until: gone crates/nvs-cli/src/serve.rs:struct EndsTheServer]
- **A TDS parameter that starts `0xFF` is now a *form* and not a refusal case, so a hand-written
  non-UTF-8 value can silently become a bound `varbinary`.** `nvs_db::tds::encode` marks a `bytes`
  with `BINARY_MARK` (`crates/nvs-db/src/tds/rpc.rs:115`) and `bind` strips it, which is the only
  channel a parameter list of opaque octets has — so `&[Some(&[0xFF, 0xFE])]`, written to pin the
  refusal, now binds one octet of binary instead. Start a case's *invalid* octets at `0xFE`, which
  no UTF-8 sequence begins with either and no form claims. [until: reviewed 2026-09-17]
- **A reconnect case over the shared store must let the first command land first.**
  `crates/nvs-stdlib/src/cache/redis.rs`'s `Connection::command` replays only a command that met an
  already-open stream, so a scripted store that drops the socket before answering the *first* command
  makes the client fail where a case wanted it to re-dial, and the failure reads as a broken handshake
  rather than as a script that never reached the reconnect. Answer once, drop the socket, and assert on
  what the second command put on the second connection.
  [until: gone crates/nvs-stdlib/src/cache/redis.rs:failure.sent && replay]
- **A case that completes a TLS handshake builds the process's one outbound client, and every
  other module's `https` case then panics `AlreadyExists`.** `nvs_host::tls::configure` settles
  one client per process and refuses a call made after any session has run, so a `cache` case
  dialling `rediss://` ahead of `http::transport`'s `trusted()` decided nine of its cases'
  outcomes from another module. Call `crate::tests::outbound_client()` before opening a session —
  it is the one place the client is built, and the `OnceLock` is what makes test order stop
  mattering. [until: gone crates/nvs-stdlib/src/lib.rs:outbound_client]
- **An `nvs-types` fixture reaches the checker and never the declaration pass, so a casing or
  visibility refusal is invisible to every helper in `crates/nvs-types/tests/common/mod.rs`.**
  `check_src` and its siblings run `parse_file` → `resolve_file` → `check_program`, and
  `check_declarations` — where `rule:core-api/identifier-casing` is answered — is in none of them,
  so the fixture reads an empty `Diagnostics` and looks like unbuilt work. Call
  `common::check_declarations_only` for a declaration-shape refusal. [until: gone crates/nvs-types/tests/common/mod.rs:check_declarations_only]
- **A conformance case that *constructs* its subjects can pin a shape nobody wrote down.**
  `path-agrees-a-trailing-or-repeated-separator-is-nothing.nvst` doubles every separator of six
  paths, so widening `Core\Path`'s grammar with a UNC root turned one constructed path into a
  different root and four members stopped agreeing about it. After a grammar change read the cases
  that *build* their inputs before the ones that write them out; `tools/verify.py`'s `.nvst` step is
  where it surfaces, a step after the unit tests that all passed. [until: reviewed 2026-09-17]
- **An oversized allocation refuses differently under `try.py` than under `nvs test`.** A scratch
  `Core\Random::bytes(9223372036854775807)` run through `python tools/try.py` dies as
  `FATAL: the request exceeded its memory limit`, while the same line inside a `tests/conformance/`
  case throws the allocator's own catchable `…is larger than any buffer this process could hold` —
  the two runners hand the request different ceilings, and the scratch pad's answer is the wrong one
  to freeze into an `--EXPECT--`. Author the row wherever you like, then run it with
  `target/debug/nvs test <case>.nvst` before believing what it printed.
  [until: reviewed 2026-09-17]
- **A markdown cell read out of the spec arrived with its backslashes doubled**, so a Class column's
  `Core\Reflect` matched no registered class and every member of that row read as unregistered.
  `cells` in `crates/nvs-stdlib/tests/spec_registry_coverage.rs` wrote the backslash when it set its
  `escaped` flag and again when the next character turned out not to be a `|`. Walking a markdown
  cell character by character, write the escape once: a cell is the *rendered* text, so an escape
  that escaped nothing is still one character. [until: reviewed 2026-09-17]
- **A socket case whose lifetime is shorter than its own handshake flakes under the whole test
  run.** The clock `opened` starts runs from before the real loopback upgrade, so a sub-second
  `maxDuration` can expire before the talking peer lands a frame, and the case then fails its *was
  it talking* assertion rather than its bound. Size such a lifetime above the connection it also
  covers, and lengthen the peer's script rather than tightening the bound. [until: reviewed 2026-09-17]
- **A `.nvst` case cannot pin the IR *type* a constant is emitted at — a callee reads its own
  parameter slot, so a default arriving under the wrong `Ty` still prints the right answer.** The
  slot's type comes from the signature and nothing converts between the two, so an emitted
  `Ty::Int` where the parameter declares `Ty::Enum` is invisible end to end. Assert `Inst::ty`
  in a `crates/nvs-ir/tests/*.rs` fixture instead, the way `parameter_defaults.rs` does with
  `int_constants`. [until: exists crates/nvs-ir/src/verify.rs]
- **`is` takes a *type* where PHP's operator took a bare class name, so respelling a test against a
  generic interface turns `E0442` on.** The right of `is` goes through `rule:types/grammar`'s
  production, and `rule:iteration/two-interfaces` makes `Iterator<T>`'s argument mandatory
  everywhere that production is used, so `$it is Iterator` is refused where the PHP spelling
  compiled. Write the argument — `$it is Iterator<int>` — and say in
  the case that the walk compares descriptors and erases it, which is why no answer moves.
  [until: reviewed 2026-09-18]
- **A renamed lowering test, or a renamed `print.rs` mnemonic, strands an `insta` snapshot.** The
  file is `crates/nvs-ir/src/lower/snapshots/nvs_ir__lower__tests__<fn>.snap`, and a mnemonic sits
  *inside* every snapshot whose fixture lowers it, so the failure reads as a lowering regression
  rather than as the rename it is. `git mv` the snapshot in the same slice and grep that directory
  for the old mnemonic. [until: gone crates/nvs-ir/src/lower/snapshots]
- **A go-to-definition case answers only for an expression at file scope, and only for an `is` test
  the checker did not settle.** `crates/nvs-lsp/src/definition.rs`'s `jump` helper answered `none`
  for `$b->area()` and `new Square()` written inside a `function` body while the same expressions at
  file scope answered, and `$s is Shape` over a `Square`-typed subject folds to a constant that
  leaves no node to navigate from. Write the subject as a `mixed` local at file scope, and keep the
  tested class one the declaration cannot settle. [until: reviewed 2026-09-18]
- **`core/jwt-a-token-verifies-for-its-whole-lifetime-and-not-one-second-past-it.nvst` fails
  intermittently, and accuses `Core\Jwt` rather than the clock.** It mints a token and verifies it
  against the real wall clock, so a run crossing a second boundary between the two reads
  `exp == now` and prints the expiry refusal where the verifying line was frozen. Re-run that case
  alone before believing a red `conformance` leg that names it — it passed by itself immediately
  after failing in the suite. [until: reviewed 2026-12-01]
- **`verify.py` formats a new `.nvs` under `tests/` or `examples/` for you, and nothing formats one
  under `docs/examples/` or `benches/members/`.** `crates/nvs-fmt/tests/identity.rs:49` holds the
  first two trees to the formatter's layout and `verify.py`'s `nvs-fmt` step brings a new or modified
  file into it — quotes, spacing and brace placement, never what a `.out` recorded — so only a bare
  `cargo test` still fails on one. Leave an example or a bench in its neighbours' style: those trees
  are not corpus, and formatting one only diverges it.
  [until: gone crates/nvs-fmt/tests/identity.rs:the_identity_printer_reproduces_every_corpus_file]
- **`Core\Config` never answers for an `[[app]]` block's `mode` or `origin`, so an example built on
  either prints `(unset)` while the setting is in force.** `Snapshot::build` lifts those two onto
  the snapshot's own fields and drops `app` from the folded table
  (`crates/nvs-config/src/snapshot.rs:158`), so a block is readable only through its `[app.limits]`
  and `[app.capabilities]` — as `limits.*` and `capabilities.*`, never under an `app.` prefix. Show
  a block through one of those, or through behaviour the way `examples/routes.nvs` does.
  [until: gone crates/nvs-config/src/snapshot.rs:table.remove("app")]
- **A capability grant written as a list reads back through `Core\Config::get` exactly as an
  ungranted one does — as nothing at all.** `nvs_config::request`'s `as_text` turns a TOML scalar
  into text and answers `None` for an array, so an `[[app]]` fixture granting
  `fs.read = ["some/dir"]` makes a dossier example print `(nothing)` for the one grant the block
  exists to show, while a `read = true` beside it prints fine. Write a page's fixture grants as
  booleans and send the reader to `nvs config dump`, which is `nvs_config::audit` and does render a
  list. [until: gone crates/nvs-config/src/request.rs:as_text]
- **`Core\Config::get` answers nothing for a key only the run mode derives, so a directive page
  prints `(nothing)` for a value that is genuinely in force.** `get` reads the request overlay, the
  secrets and the written table and no fourth thing (`crates/nvs-config/src/request.rs:88-97`),
  while `mode::DERIVED` is applied by whoever reads the key — so `debug.inline` answers nothing in
  a checkout whose mode has it `false`. Give such an example a `?? '(nothing, so the mode decides)'`
  fallback, or write the key in `nvs.toml` the way `[control] socket` and `[debug] keep_temporary`
  are written. [until: reviewed 2026-09-18]
- **An `nvs-config` census test cannot re-derive the registry's lookup, because
  `directive::governs` is `pub(crate)`.** A case wanting to show that a row is load-bearing —
  that `http.client.pool_idle_timeout` would fall through to the `http` blanket without one of
  its own — cannot filter `DIRECTIVES` by the boundary test, and widening that function for a
  test would put the lookup rule's one implementation behind a public name. Ask `lookup` a near
  miss instead: `http.client.pool_idlex` must resolve to `http` and not to `http.client.pool_idle`,
  which pins the same dot-boundary claim through the API the registry already exports.
  [until: reviewed 2026-09-18]
- **`Core\Config::get` answers nothing for a directive whose value is a list, so writing one into
  `nvs.toml` to make an example page print a value buys nothing.** `[http.client.tls] roots =
  ["bundled"]` is exactly what this checkout trusts and the page still printed `(nothing)`, because
  `nvs_config::request`'s `get` returns `None` for a key naming a table or a list rather than a
  rendering nothing could set back. Run the example against the binary before writing a line into
  `nvs.toml` for a page's sake — a scalar sibling in the same block, `min_version` here, is what makes
  a page print a value. [until: reviewed 2026-09-18]
- **`keys_in("limits.hard")` is not `keys_in("limits")`, and a test asserting the two blocks agree
  key for key fails on the design rather than on a defect.** The ceiling block is its own struct in
  `crates/nvs-config/src/tree.rs` carrying only the six budgets a request may write, while
  `[limits]` also carries the reserve pair, the recursion cap, the two decompression keys and `hard`
  itself — a ceiling over a value no request can move would bound nothing. Subtract the keys some
  `limits.*` row carves out of the block before comparing the pair, which is what
  `every_budget_with_a_ceiling_answers_the_request_and_the_operator_differently` does.
  [until: reviewed 2026-09-18]
- **A `docs/examples/config/<directive>/` example prints `(unset)` for its own key unless the
  repository-root `nvs.toml` writes it**, because that file is the configuration every example runs
  under and nothing supplies a second one. Its cache blocks say why they are written at exactly the
  figures the code ships: a page printing `(unset)` teaches nothing about a key an operator is about
  to write. Write the key at its shipped default, so the tree behaves identically, and say so in a
  comment beside it. [until: reviewed 2026-09-18]

- **`Core\Config::set('limits.memory', '1')` is accepted and ends the request on the spot**, because
  a request lowering its own budget is always allowed and one byte is a ceiling the next allocation
  passes. An attack that churns a budget down to its smallest accepted value therefore fatals at
  round zero, and every section under it never runs while the case still reports as a pass — a clean
  fatal is one. Churn between values the program still fits in, and put the deliberate exhaustion
  last. [until: reviewed 2026-09-18]
- **`throw new Exception` does not compile, and a hostile case that does not compile is a failure
  rather than a pass.** Novis declares `RuntimeError`, `LogicError`, `ParseError`, `IOError`,
  `TimeoutError` and `ArithmeticError`; `Exception` is PHP's name for the root and E0303 says only
  `no matching declaration`, which reads like a missing import. Take the spelling from
  `grep -rho "throw new [A-Za-z\\\\]*" tests/conformance` before writing one.
  [until: reviewed 2026-09-18]
- **A `limits.cpu_time` a program sets for itself does not stop it under `nvs run`.** The watchdog
  is registered once from the ceiling in force before the task starts, and `RunningRequest::new`
  answers `None` for a request under no cap (`crates/nvs-cli/src/main.rs:2459`), so a later
  `Core\Config::set` moves the number and nothing charges it. An attack that needs a CPU stop writes
  the ceiling into the `nvs.toml` the run reads; a memory ceiling set from inside *is* charged, and
  is the cheap way to reach a limit handler.
  [until: gone crates/nvs-cli/src/main.rs:let (view, cpu_limit) = ceiling]
- **A dossier `directive:` slice's claim is often already pinned elsewhere in the same crate, and the
  pack points only at the registry row.** `[trace] sample`'s bounds are an `export.rs` unit test and
  `[opcache]`'s revalidation semantics are two cases in `crates/nvs-config/tests/snapshot.rs`, so a
  registry case written from the rule alone re-asserts one of those under a new name. Grep the
  crate for the key before writing, and assert the field those cases do not read — the blanket over
  the keys a block *accepts* for one whose semantics are pinned, `Apply` for one whose class is.
  [until: reviewed 2026-09-18]
- **A directive reported as owing `tests 0 of 1 cases` usually already has its test, and what is
  missing is the `covers:` marker.** `dossier.py` attributes a directive by that marker alone, so a
  block with a whole test file of its own still reads as owing a case — `[queue]` owed one while
  `crates/nvs-config/tests/queue.rs` and three registry cases in `directives.rs` already pinned every
  claim worth making. Grep the crate's `tests/` and its `src` unit tests for the block's name first
  and mark the case that pins the claim, including one in `src/`, which counts.
  [until: reviewed 2026-09-18]
- **An example that prints generated output can bake its own file name into the blessed `.out`.**
  `Core\Command::completions` registers a script against the program's name, which under `nvs run`
  is the file's stem, so `02-the-script-is-written-from-your-commands.nvs` blessed a bash function
  called `__02_the_script_is_written_from_your_commands_complete`. Keep the slug short for any
  example whose output quotes the program's own name, and read the blessed lines rather than
  trusting the exit status. [until: reviewed 2026-09-18]
- **A `Core\Db` refusal the *call* caused is a `LogicError` and not a `Core\Db\DbError`, so a case
  catching the wrong one dies on an uncaught throw with the right message.** `nvs-db` builds no fault
  and answers in `io::ErrorKind`s, and `crates/nvs-stdlib/src/db/bind.rs:149` splits them: `InvalidInput`
  is every mistake in the call — a nested `transaction` asking for its own isolation level, a value with
  no bound form — while only a refusal the server itself worded is a `DbError` carrying a `kind`. Read
  the driver member's own `# Errors` paragraph for which one it answers before writing the `catch`.
  [until: reviewed 2026-09-18]
- **A landed test's comment can claim a construct does not compile, and be out of date.**
  `time-datetime-startof-and-endof-are-one-agreement-over-every-unit.nvst` said an
  `array<Core\Unit>` element and a helper's own `Core\Unit` parameter were both `E0401`,
  and both compile today, so an example written from that comment would have avoided the
  two shapes a reader actually writes. Probe the claim with a five-line program through
  `target/debug/nvs.exe run` before believing it, and rewrite the comment in the same
  session. [until: reviewed 2026-09-19]
- **Two `catch` clauses in one scope cannot share a binding name, and the diagnostic is `E0406:
  already declared` pointing at the earlier clause.** A catch binding is declared for the whole
  enclosing scope rather than for its own block, so the PHP habit of calling every one `$e` — or two
  clauses in one file both calling theirs `$notThisOne` — fails to compile for a reason that has
  nothing to do with what is being caught. Give each clause a name saying what that branch means, or
  put each `try` in its own static method the way the landed `types/` cases do.
  [until: reviewed 2026-09-19]
- **A multi-operand `echo` prints each operand as it is evaluated, so a call that throws in the
  middle of one leaves the earlier operands already on stdout.** `echo $name, ": ", Thing::of($x),
  "\n"` writes `$name` and the separator before the throw, and the `catch` arm that prints the same
  prefix again produces a blessed `.out` reading `Hallbeck: Hallbeck: skipped, …` — which looks like
  a runtime bug and is not one. In an example or a case whose `try` calls something, bind the value
  to a local inside the `try` and `echo` only once it exists. [until: reviewed 2026-09-19]

- **`if ($x != null)` narrows only a plain variable, only while nothing in the block reassigns it,
  and `while ($x != null)` does not narrow the body at all.** Walking a `previous` chain the obvious
  way fails twice over: `while ($at != null) { $at = $at->previous; }` reports the receiver nullable
  because the loop test does not narrow, and moving the test into an `if` still fails because the
  assignment inside the block drops the narrowing — and a property path like
  `$e->previous->message` is never narrowed by testing the path. Read the value into a fresh local
  at the top of the body, test *that*, and assign the next step to the outer name.
  [until: reviewed 2026-09-19]
- **A `#[Core\Json\Derive]` class with no constructor compiles and then has no codec at run time**, and
  the `LogicError` blames the one thing that is not wrong — the attribute the class plainly carries.
  `rule:core-classes/derive-field-list` makes a decode an ordinary `new`, so with no parameter list the
  field list is empty, and `crates/nvs-types/src/derive.rs:1878` stays quiet on the assumption
  `ctor_init` already reported it, which it does not when every property has a default. Give the class
  a constructor taking every field. [until: reviewed 2026-09-19]

- **A `ParseError`'s `issues` come in *name* order from a shape door and in *declaration* order from a
  class door**, so an expectation written from the shape written at the call site is red for any field
  set that is not already alphabetical. `Core\Arr::shapeAs<{ok: int, bad: int, alsoOk: string, alsoBad:
  bool}>` reports `alsoBad` before `bad`, while a `#[Core\Json\Derive]` class declaring `zeta, alpha,
  mid` reports them in exactly that order. Run the program before writing the `--EXPECT--` block, and
  name fields whose two orders differ when the order is what a case pins.
  [until: reviewed 2026-09-19]

- **Assigning to the binding a `!= null` test narrowed drops the narrowing on that statement's own
  right-hand side**, so the walk `while ($cursor != null) { … $cursor = $cursor->previous; }` is
  `E0459` on the assignment while every other read in the body is fine. The narrowing holds for the
  body but not across the write that re-widens the binding. Spell the step `$cursor =
  $cursor?->previous;` — `?->` answers `null` and the loop's own condition is what ends it.
  [until: reviewed 2026-09-19]
- **A fixture that recurses to a fixed depth stops at about 4,400 calls, not the "about 65,000
  frames" `rule:errors/stack-depth` names.** That figure is 8 MB of reserved stack over a minimal
  frame; an ordinary static method with one local throws `RecursionError` between 4,400 and 5,200
  calls, debug and release alike, and a fatter body stops sooner. Keep a proof that must succeed
  under about 2,000, and let one that must fail recurse with no base case at all.
  [until: reviewed 2026-09-19]
- **A `!= null` test joined by `&&` narrows neither receiver, so `->` on either is still `E0459`.**
  `if ($a != null && $b != null) { echo $a->city; }` reports "this receiver is nullable" for both,
  while the same two tests written as two separate `if` blocks compile —
  `rule:expressions/nullable-conversion`'s narrowing reads one test per guarded branch and not a
  conjunction of them. Write one `if` per nullable receiver, or `?->` with `??` where the value is
  only echoed. [until: reviewed 2026-09-19]
- **A bare array literal in a `foreach` head is `array<mixed>`, so the binding cannot be typed.**
  Nothing in `foreach ([1, 2] as int $d)` gives the literal a target to take its element type from,
  so the checker settles on `mixed` and then refuses the `int` binding. Declare the list first —
  `array<int> $ds = [1, 2]; foreach ($ds as int $d)` — which every proof program under
  `docs/examples/` and `tests/hostile/` has to do anyway. [until: reviewed 2026-09-19]
- **`as` binds tighter than an arithmetic operator, and a `float` that is not whole refuses to
  become an `int`.** `$minutes / 60 as int` parses as `$minutes / (60 as int)` and is `int|float`,
  and the parenthesised `($minutes / 60) as int` then throws `cannot convert this value to int` for
  every quotient with a fraction. There is no `Core\Math::intdiv`, so a proof program that wants
  whole-number division keeps the value in `int` arithmetic (`%` and subtraction) instead.
  [until: reviewed 2026-09-19]
- **About twenty nested parentheses already reach the parser's recursion limit, and a hostile case
  that trips it fails.** The limit is 96 levels and one parenthesised expression costs several, so
  `((((...))))` written to look extreme stops the program being read at all — which
  `tests/hostile/README.md` counts as a failure, because an attack that does not compile was never
  delivered. Keep a depth attack in a hostile file well under it, or build the nesting at run time
  out of values. [until: reviewed 2026-09-19]
- **A new `.nvs` or `.nvst` anywhere under `tests/` joins `nvs-syntax`'s `lossless` corpus, which
  only fails at the end of the session.** `lossless` demands every byte be covered by a token or a
  trivium, which the `<?nvs` a shebang file refuses was not. Expect a case exercising a recovery
  path to be what finds a hole in `rule:ide/tokens-plus-trivia-reproduce-the-file`.
  [until: reviewed 2026-12-19]

- **A `decimal` class constant declares cleanly and fails at the *read*, with `E0792: has no
  compile-time value to inline`.** A constant is inlined at every use site, and
  `crates/nvs-types/src/defaults.rs`'s module doc says the `ConstArg` variant carrying
  `InstKind::ConstDecimal` is unbuilt, so the grid folds the literal to nothing — the declaration
  itself says nothing about it. Keep the rate as an `int` percentage and divide, or write the
  literal at the use site. [until: exists crates/nvs-types/src/defaults.rs:ConstArg::Decimal]
- **A numeric literal takes `decimal` from a declared target, and a ternary is not one.**
  `decimal $postage = $heavy ? 4.90 : 2.50;` is `E0401: expected decimal, found float`, because the
  arms are checked with no expectation to place them in and `float` is what a bare fraction is.
  Declare the binding with one literal and assign the other in an `if`, or write `as decimal` on
  each arm. [until: reviewed 2026-09-19]
- **A `type` alias naming another `type` alias is refused, and the diagnostic calls it a class.**
  `E0307` reads the written atom rather than what it resolves to
  (`crates/nvs-hir/src/resolve.rs:284`), so `type B = A;` over `type A = int;` is "a `type` alias may
  not name a single class, interface or enum on its own" — right about the shape, wrong about the
  kind. Chain through a wrapper instead, `type B = ?A;` or `type B = array<A>;`, which is what a
  hostile case needs to build a long chain that compiles at all. [until: reviewed 2026-09-19]
- **`echo` of a control character prints its picture, not the character.** A case that echoes `"\r"`,
  `"\v"`, `"\f"` or `"\e"` to show what an escape produced gets `␍ ␋ ␌ ␛` back, because the terminal
  sink substitutes the U+240x picture for a control character, so an `--EXPECT--` written from a run
  pins the picture rather than the escape. Assert an escape by comparing it with the same code point
  written the long way — `"\r" == "\u{D}"` — or by `Core\Str::length`, and keep control characters out
  of the expected output entirely. [until: reviewed 2026-09-19]
- **`Core\Arr::append($a, $x)` called in a loop is quadratic, and a hostile case built that way runs
  for minutes rather than seconds.** The call holds a second reference to the array while it builds
  its answer, so copy-on-write copies the whole thing every round — twenty thousand appends copied
  on the order of two hundred million elements, four and a half minutes in a debug build, where the
  identical loop finished in under two seconds once it stopped. Grow an array in a loop with
  `$a[] = $x` and keep `Core\Arr::append` for the one-off where a second array is what you actually
  want. [until: reviewed 2026-09-19]
- **An array type takes one parameter, so `array<string, uint>` does not parse.** The key type is
  never written — a string-keyed array of counts is `array<uint>` and the literal supplies the keys
  — so the comma is read as call syntax and what comes back is an `E0101`/`E0102` pair pointing at
  the `<`, saying nothing about types. Declare the value type alone and key the literal:
  `array<uint> $seen = ["get" => 0];`. [until: reviewed 2026-09-19]
- A hostile case that builds a big array with `Core\Arr::append` in a loop never finishes: a million
  rounds hangs, and fifty thousand took two minutes where ten thousand takes five seconds. Each call
  copies the array it is given, so the loop is quadratic in the number of entries. Build a large
  array with `Core\Arr::fill(n, v)`, and keep an appending loop to about ten thousand rounds.
  [until: reviewed 2026-09-19]
- **A hostile file whose last step is a top-level `return;` is not `ends-early`, and declaring the
  marker fails the sweep.** The suite reads that marker as "the program did not reach its end", and a
  `return` outside any method is an ordinary ending, so a file that behaves exactly as its comment
  says is reported as `declares ends-early, but ran to its last line`. Keep `ends-early` for a limit,
  an uncaught throw or `exit`, and run `python tools/dossier.py --run hostile --group <group>` while
  you still have the file open rather than leaving it to the driver's acceptance check.
  [until: reviewed 2026-09-19]
- **A `// covers:` marker added at the top of a `--FILE--` block shifts every line number that
  case's `--EXPECTF-ERROR--` pins.** Such a section names the offending line three times over
  (`--> %s:29:14`, the gutter number and the caret row), and `%s` wildcards the path alone, so one
  inserted line fails the case on a number rather than on its claim — in `class/` and `lang/` as
  much as in `reject/`. Put the marker on the **last** line of the `--FILE--` block in any case
  carrying an `--EXPECTF-ERROR--` section, at the top everywhere else, and anywhere at all when
  `%A` swallows it. [until: reviewed 2026-09-19]
- **`echo` with several arguments writes each one as it is evaluated, so a call in argument position
  that echoes puts its own lines in the middle of yours.** `echo "  bump gave ", $l->bump(), "\n";`
  on a `PropertyObserver` class prints `  bump gave ` first, then the observer's lines, then `1` —
  which reads as a broken pipeline rather than as evaluation order, and costs a blessed `.out` or an
  `--EXPECT--` block to discover. Assign the call to a variable first and echo the variable when the
  callee can print anything: `int $bumped = $l->bump();`. [until: reviewed 2026-09-19]
- **A declaration inside a `try` block is in scope after the block, so a case that repeats a setup
  under a second heading fails with `E0406` rather than running.** The three `try` blocks of an
  agreement case naturally want the same variable names in the section that follows them, and the
  compiler sees one scope. Give the second set its own names; nothing about the assertion depends on
  reusing them. [until: reviewed 2026-09-19]
- **A `covers:` marker added to a `.nvst` case above its last refused line turns a green case red.**
  An `--EXPECTF-ERROR--` block's `-->` anchors are absolute line numbers inside the `--FILE--`
  program, so one comment line added at the top moves every one of them, and the failure then reads
  as a diagnostic that changed rather than as a line that moved. Put the marker after the last
  statement the block quotes — the end of the `--FILE--` block is always safe — and run
  `target/debug/nvs test <case>.nvst` before committing it.
  [until: reviewed 2026-09-19]
- **A hostile step that reaches the memory limit ends the program, so every step written below it
  never runs and nothing says so.** An `array<Level>` filled with ten million cases stops at
  `FATAL: the request exceeded its memory limit`, and the file then passes as soon as
  `// hostile: ends-early` is declared, while proving one step of the four it claims in its own top
  comment. Put the step that cannot be caught last, and read the program's own output before
  declaring `ends-early`. [until: reviewed 2026-09-19]
- **A fault raised inside a `Core` member reaches a `catch` one frame up with an empty `location`,
  so a case that prints it pins the emptiness rather than a site.** The site is seeded on the
  catchable edge of the frame that raised it — a `finally` seeds none — which is what
  `a-helper-fault-names-the-frame-it-was-raised-in-when-it-is-caught-there.nvst` is written around,
  while `backtrace` is filled either way. Print `Core\Arr::count($e->backtrace)` when the claim is
  that the throw unwound, and leave `location` to the case that owns it.
  [until: reviewed 2026-09-19]
- **Adding a `// covers:` marker to a `.nvst` case shifts every line under it, and a case whose
  `--EXPECT--` pins a `file:line` then fails.** `a-location-is-the-throw-site-and-a-rethrow-moves-it`
  froze `rethrown=case.nvs:16`, and the one marker line moved the rethrow to 17, so a one-line
  attribution edit turned the conformance suite red. Before marking a case, `grep -n 'case.nvs:'` it:
  where a number is pinned, move it by the number of lines the marker adds, in the same edit.
  [until: reviewed 2026-09-19]
- **A `Core\Task` group collects each task's `echo` and prints it one whole task at a time, so an
  `echo` trace cannot show that two tasks interleaved.** Two tasks printing `starts` and `wakes`
  around a `Core\Time::sleep` come out as `long, long, short, short` however they really ran, and
  the order tasks *start* in is not the order the fields were written — a group whose `slow` field
  is written before `quick` traces `quick starts` first. Write the trace into a
  `public static string` and echo it after the group returns, which is what
  `tests/conformance/task/a-wait-parks-only-the-calling-task-and-answers-a-value.nvst` does.
  [until: reviewed 2026-09-20]
- **Adding a line to a `.nvst` case's `--FILE--` block shifts every anchor its `--EXPECTF-ERROR--`
  names.** The expected diagnostics carry `--> case.nvs:NN:CC` counted from the start of that block, so
  one inserted `// covers:` marker moves all of them by one and the case goes red on the anchors alone.
  Bump each `NN` in the same `splice.py` patch, and run `target/release/nvs.exe test <case>.nvst` before
  the wrap. [until: reviewed 2026-09-20]
- **A `lang:` feature's perf figure goes stale the moment its *reference chapter* is edited, because
  the chapter is what the feature is "implemented at".** Fixing one stale sentence in
  `docs/reference/lang/90-attributes.md` turned every measured `lang:attributes/…` figure into
  `perf: stale: … changed since it was last measured`, including features this session never touched.
  Re-measure with `python tools/dossier.py --record-perf --only '<feature>' …` in the same session,
  and budget for it whenever a slice corrects the chapter its own feature is read from.
  [until: gone tools/dossier.py:impl_hash]
- **A `Core\Attributes` retrieval probe proves nothing about `$member` unless the target itself
  carries a matching attribute.** A computed `$member` was being ignored rather than folded to the
  empty result, and a fixture whose class carries no matching literal prints `null` under either
  behaviour, so the first probe read as agreement with `rule:attributes/structural-retrieval`.
  Attach a matching literal to the class *and* to the member before asserting anything about
  `$member`, the way
  `tests/conformance/lang/attributes-retrieval-answers-by-shape-in-declaration-order.nvst` does.
  [until: reviewed 2026-09-20]
- **A property default has to be a constant, so an array literal with anything in it is refused
  where it looks like it should work.** `public array<string> $notes = ["one", "two"];` is
  `E0435` — a default is evaluated once at compile time and written into every fresh instance's
  slot, and only a scalar literal, `[]`, an enum case or another class's `const` may be written
  there. Give the class a no-argument `constructor` and fill the property in it, which also keeps
  the class enumerable by `Core\Program::implementing`. [until: reviewed 2026-12-31]
- **A `covers:` marker added to a reject case moves every line number its `--EXPECTF-ERROR--` block
  names.** The marker belongs inside the `--FILE--` block, so a line inserted after `<?nvs` shifts
  every `case.nvs:N:C` below it by one, and the `-->` gutter widens by a space when a number crosses
  into two digits. Bump each location by the number of lines inserted, in the same edit, rather than
  waiting for the conformance suite to name them. [until: reviewed 2026-09-20]
- **An empty `--EXPECTF-ERROR--` section in a `.nvst` asserts that the run *failed*, so a `--RUN-- test`
  case whose tests all pass reports `expected the run to fail, and it succeeded`.** The section is the
  stderr expectation, and its presence alone is what requires a non-zero status, so copying the shape
  from a neighbouring runner case that has failing tests carries that assertion along with it. Drop the
  section entirely when every `#[Test]` in the case passes, and keep it only where the run really ends
  non-zero. [until: reviewed 2026-09-20]
- **Adding a `// covers:` marker to a case under `tests/conformance/reject/` moves every line number
  its `--EXPECTF-ERROR--` block pins.** That block reproduces each diagnostic's `--> case.nvs:NN:CC`
  anchor, counted from the `<?nvs` of the `--FILE--` block, so one inserted comment renumbers all of
  them while the case still reads as untouched. Put the marker and the `NN + 1` bumps in one
  `tools/splice.py` patch, then run `target/debug/nvs.exe test <path.nvst>` before believing it.
  [until: reviewed 2026-09-20]
- **A `.nvst` run case whose program succeeds must not carry an empty `--EXPECTF-ERROR--` section.**
  The runner reads the section's *presence* as "this run is expected to fail", so a case whose tests
  all pass fails the suite with `expected the run to fail, and it succeeded` — the neighbouring case
  that has the section is one whose tests deliberately fail, which is why copying its skeleton
  misleads. Write the section only when the program's exit status is non-zero, and check with
  `target/debug/nvs.exe test <case>.nvst` before the full sweep. [until: reviewed 2026-10-20]
- **`Core\Str::from` does not exist, so a `.nvs` proof that turns a number into text uses the `as
  string` cast.** The diagnostic is `E0405: Core\Str has no member named from`, which names the miss
  and not the spelling that works, so it reads as a member still to be written. Write `($k as
  string)` — `Core\Str::repeat('k', 1000) . ($k as string)` is what builds a long key from a counter.
  [until: reviewed 2026-09-20]
- **Two `catch` blocks in one scope may not name the same variable, and a multi-step attack file is
  where that bites.** A hostile case is several numbered steps in one program, so a second
  `catch (RecursionError $error)` under a first `catch (LogicError $error)` is
  `E0406: $error is already declared`, and the case then reports as "never ran -- it does not
  compile", which reads exactly like work nobody has written yet. Give each step's catch its own
  name — `$stopped`, `$refused` — because a `.nvs` file's top-level statements share one scope.
  [until: reviewed 2026-09-20]

- **A closure that declares a return type and then only throws does not compile.**
  `fn(int $n): int => throw new LogicError('no')` is `E0401: expected int, found never`, while the
  same closure with no `: int` on it is accepted, so the same throwing step is fine in one attack
  file and refused in the next. Write that step as a static method declaring the return type and
  throwing in its body, and hand the closure a call to it. [until: reviewed 2026-09-20]
- **A bench that reaches its subject array through a nested array index charges the member two
  allocations per op.** `benches/members/core/Arr/first.nvs` chained its rounds through
  `array<array<uint>> $feeds` and `$feeds[$total % 2]`, so a declared `// bench: allocations 0` was
  refused at 2.000 and the ns figure was twice the member's: `firstKey` read 59.4 ns/op that way and
  29.4 measured alone. Chain with two plain arrays and a branch, and read a `FAIL` on a declared
  count as a question about the bench before the member. [until: reviewed 2026-09-20]

- **`?T ?? <literal>` is `mixed`, so it cannot feed a slot whose type is named.** `$total = $total +
  (Core\Arr::first($a) ?? 0)` does not compile against `uint $total`, and neither does `array<int>
  $held = Core\Arr::first($outer) ?? [];` — both are `E0401`, `found mixed`. `echo` accepts it, so a
  proof program only fails where a type is written down: bind `?uint $v = Core\Arr::first($a);` and
  branch on `$v != null`. [until: reviewed 2026-09-20]
- **A hostile step that repeats a whole-array member prices its repeat count by the entries, and a
  count copied from a sibling attack is then wrong by orders of magnitude.**
  `Core\Arr::chunk($long, 25000)` over 100000 entries, 20000 times, is two billion entry copies: it
  ran 3m23s in a debug build against a declared `timeout-ms 60000`, where the same 20000 repeats of
  `Core\Arr::firstKey` cost nothing because that member reads one end. Nothing grows here, so this is
  not the quadratic-append trap above; divide the repeat count by the entries the member walks, and
  time the file yourself before committing it. [until: reviewed 2026-09-20]
- **A hostile step that hands a member an outsized key *and* an outsized subject multiplies the
  two.** `Core\Arr::column` hashes the column key once per row, so a key of a million characters
  over a 200000-row table is 200 GB of hashing and had not finished after two minutes, while each
  half on its own takes a second. Give an outsized input its own three-entry subject, and keep the
  big subject's keys ordinary. [until: reviewed 2026-09-20]
- **An allocating `Core` member can end the program rather than throw, because a request's memory
  ceiling is a `FATAL` and no `try` catches it.** `Core\Arr::fill` throws a catchable error for a
  count nothing could ever hold, and `Core\Arr::fill(4000000000, 'x')` instead reaches the ceiling,
  which ends the process and takes every later step of the attack with it. Run a new attack once
  with `target/debug/nvs.exe run` before fixing its order: the step that ends the program goes last,
  and the file declares `// hostile: ends-early`. [until: reviewed 2026-09-20]
- **A hostile case's uncatchable last step can run forever instead of ending, and the cause is the
  member missing its per-entry ceiling ask.** `Core\Arr::range(1, 9000000000)` ran past ninety
  seconds while `range(1, 40000000)` reported a `FATAL` naming a gigabyte held: a native accumulate
  loop passes no statement boundary, so the breach is read only where the member returns. Time such
  a step by hand, and where it does not end ask `nvs_runtime::affordable` per entry the way
  `nvs_runtime::sequence::drain` does. [until: reviewed 2026-09-20]
- **A proof program that adds a counting member's `uint` answer into an `int` accumulator does not
  compile, and the hostile sweep reports the whole file as `never ran`.** `Core\Arr::count` returns
  `uint`, and `E0407` refuses `int + uint` outright, so an attack whose accumulator is the usual
  `int $seen = 0` loses every step it had rather than one line. Declare the accumulator `uint` the
  moment anything in the sum came from a member that counts, and read a red hostile case's message
  before deciding the attack itself is wrong. [until: reviewed 2026-09-20]

- **Indexing an `array<mixed>` gives `mixed`, so a case that walks a nested structure needs
  `as array<mixed>` on every step down.** `$node = $node["next"];` is `E0401` — `mixed` is not
  `array<mixed>` — which makes a depth-counting assertion look impossible to write from Novis when
  it is one cast away. Write `$node = $node["next"] as array<mixed>;`, and reach for it whenever a
  case descends what it built with `$node = ["next" => $node];`. [until: reviewed 2026-09-20]
- **A `-p nvs-stdlib` test of a member with an options block must fill the options the way a call
  site does, and `Value::null()` is not that.** `Core\Arr::diff` takes five arguments, and
  `on_of` reads `on` as a plain int (`0`, `1`, `2` for `Core\SetOn`'s three cases), so passing null
  for the default is a `FATAL` that reads exactly like the member refusing the two arrays. Read the
  option's `Const::` default in the registry row's `CoreOption` list before building the argument
  slice — a `Const::EnumCase` arrives as `Value::int(<case index>)` and only a `Const::Null` one
  arrives as `Value::null()`. [until: reviewed 2026-09-20]
- **A member the dossier says owes a Rust test can already have one, along with the reader helper you
  are about to add.** `arr.rs`'s test module runs past two thousand lines, so
  `count_by_counts_each_bucket_in_first_occurrence_order` and its `counts_of` sat far below where a
  new `countBy` test goes, and writing both cost a compile cycle on `E0428`. `grep -n
  nvs_core_<class>_<member> <file>` first — a hit inside the test module is a `covers:` marker to add
  above an existing `#[test]`, never a second test. [until: reviewed 2026-09-20]
- **A `Core` member whose call site writes a type argument is never credited by a call, so
  `dossier.py` reports `tests 0 case(s)` over cases that plainly pin it.** The detector matches
  `Core\Arr::shapeAs(` and the source reads `Core\Arr::shapeAs<{n: int}>(`, so three green conformance
  cases went uncounted and the member read as owing both its tests. Grep `tests/` for the member
  before writing anything, and where the claim is already pinned the whole edit is `// covers: <the
  member>` on one `.nvst` and one Rust `#[test]`. [until: reviewed 2026-09-20]
- **A test that arms a wall-clock deadline before it starts the scheduler charges the fixture's own
  setup to that deadline, and then fails under a sanitizer alone.**
  `a_timer_and_a_deadline_are_the_same_wheel` armed a socket 10ms out and afterwards bound a listener,
  installed a reactor and built a scheduler, so under `tools/tsan.sh` the read answered `TimedOut`
  without ever parking and `parked` came back 1 where the test wanted 2 — green on Windows, red on the
  floor, with nothing in the diff to blame. Arm a deadline inside the task that waits on it, where the
  window is the wait's own, and leave a margin the slowest leg can spend.
  [until: reviewed 2026-09-20]
- **A hostile case's memory ending cannot be reached with `Core\Arr::append` in a loop.** Each call
  copies the array, so growing to the 256 MiB ceiling one entry at a time is quadratic, and 200,000
  appends of a one-element array did not finish inside 200 s — well past any `timeout-ms`. Seed the
  list with the values the attack is about and double it with `Core\Arr::appendAll($kept, $kept)`,
  which reaches the ceiling in under a second. [until: reviewed 2026-09-20]
- **The parser's 96-level nesting limit is spent about five levels per bracket, so a case nesting 90
  of them is refused before its first step runs.** `Core\Ast::parse` accepts `echo` behind 15 opening
  brackets and refuses 20, and an uncaught `ParseError` in step 1 means no later step is delivered at
  all. Let the case find the boundary itself — raise the depth in a loop and count the refusals —
  rather than writing a number the grammar can move. [until: reviewed 2026-09-20]
- **A hostile case that nests source "as deeply as the parser allows" is refused well short of 96
  levels, and the refusal reads as the attack having worked.** One `if (1) { … }` costs the parser
  several of its 96 levels, so `Core\Str::repeat('if (1) { ', 90)` throws a `ParseError` while 40
  parses and walks, and a case written to reach a deep tree then asserts nothing about depth.
  Probe the depth with `target/debug/nvs.exe run` on a throwaway program before writing the case,
  and put the level that parses in the comment beside the number. [until: reviewed 2026-09-20]
- **A `Core` member whose call is folded at compile time still owes a measured figure.**
  `nvs_stdlib::attributes`' module doc says both retrievals are answered by `nvs check` and that
  their symbols abort if reached, which reads as a `[skip] perf` entry — but
  `benches/members/lang/attributes/reading-attributes-back-core-attributes-get-and-all.nvs` has
  always measured those reads, because materializing the folded constant costs a running program
  something. Before writing `[skip] perf`, list `benches/members/lang/<topic>/` for a bench over the
  same surface. [until: reviewed 2026-09-20]
- **A `.nvst` credits a `Core` member only through the written `Class::member(` spelling, so an
  instance call earns nothing and a message in `--EXPECT--` earns everything.**
  `bigint-refuses-a-zero-divisor-…` calls `->shr(` and `->sign(` and credits neither, while its
  expected `Core\BigInt::shl(): …` line credits `shl`; `scan_calls` reads the whole file and only
  that spelling is sound without a type checker (`tools/dossier.py:1018`). Check `python
  tools/dossier.py --id '<feature>'` before believing an existing case covers a member, and give
  every case you write for an instance member a `// covers:` line.
  [until: gone tools/dossier.py:scan_calls]
- **A `uint` variable in a `.nvs` proof cannot be initialised from an integer literal or from a
  division.** `40 * 1048576` is an `int` and `$a / $b` over two `uint`s is `uint|float`, so both are
  an `E0401` against a `uint` declaration, and the error points at the whole expression as if the
  member returned the wrong type. Write `(40 * 1048576) as uint`, and turn a division into a
  multiplication on the other side: `$peak * 5 > $limit * 4` is "more than four fifths of the
  limit". [until: reviewed 2026-09-20]
- **A `getSecret` whose `fill` merely *reads* its own key does not throw; only one that asks to
  *fill* it does.** `nvs_core_cache_get_secret` elects a filler once per name, so a plain read
  inside the fill is an ordinary miss, and the `LogicError` its reference card describes comes from
  the nested election alone — which is why
  `tests/conformance/core/cache-get-secret-fill-asking-for-its-own-key-is-a-logic-error.nvst` passes
  an inner `{fill: …}`. Write the inner call with its own `fill` when the attack is the self-wait,
  or the step reports the member answering where you meant it to refuse.
  [until: reviewed 2026-09-21]
- **A `.nvs` proof over `Core\Task\Channel` that never closes the channel hangs, and the hang is only
  reported when the timeout runs out.** A reader's `foreach` waits for as long as the channel is open
  and empty, so "nobody ever closes it" is a deadlock the runtime does not break, and an attack
  written around it spends its whole `timeout-ms` before it is named as a failure. Close on every
  path out of the producing task, and pin the deadlock's *absence* — a close that arrives while the
  reader is already waiting — rather than its presence. [until: reviewed 2026-09-21]
- **An example is run as `nvs run <file>` with no words after it and no terminal, so a feature whose
  subject is input has to print something when it is given none.** `dossier.py` runs and blesses an
  example with a bare argument vector and captured streams, and there is no `--ARGS--` section like
  the one a `.nvst` case has. Write the program the way a real tool behaves with nothing given: an
  empty list from `Core\Cli::arguments`, a prompt taking its `{default: …}`, a `colorDepth()` of
  `None`.
  [until: reviewed 2026-09-21]
- **A `-p nvs-stdlib` test cannot read which class a member threw off `nvs_runtime::call`.** That
  function answers `Result<Value, i32>`, so a `match` on `Fault::Thrown(class, message)` does not
  compile at all, and `ctx.take_pending()` gives the message alone — the class sits in a pending slot
  no test reaches without installing an exception class table first. Assert the sentence from Rust and
  pin the class from the `.nvst` or hostile case that catches it by name, the way
  `tests/hostile/core/Cli/select/01-a-menu-nobody-can-answer.nvs` catches `LogicError`.
  [until: reviewed 2026-09-21]
- **A `Core` member that `python tools/dossier.py --id` reports as owing a Rust test can already
  have one, under a name that never mentions it.** `Core\Cli::live` read `0 Rust` while
  `a_live_region_is_scoped_and_restores_the_terminal_on_a_panic` had pinned it all along, because a
  test is attributed by its `// covers:` marker and by nothing else
  (`rule:testing/proof-attribution`). Grep the member's own module for a `#[test]` about its subject
  before writing one: adding the marker to the test that exists is usually the whole edit, and a
  second test earns its place only by pinning something the first does not.
  [until: reviewed 2026-09-21]
- **Dividing a `uint` makes the value `mixed`, so a chained bench input that uses `/` does not
  compile at all.** `/` on two `uint`s is `uint|float` and a `%` over that is `mixed`, which every
  `uint` parameter refuses with `E0401` — and it lands exactly where `benches/members/README.md`
  tells you to chain iteration N's input to N−1's result. Chain with `*` and `%` only
  (`($total * 7) % 256`), and cast where a member wants the other width
  (`Core\Arr::slice($files, $done as int)`). [until: reviewed 2026-09-21]

## Splitting a file that got too big

- **The mechanism is two lines, and it is a pure move.** Rust lets one inherent `impl` and one set
  of free functions live in several modules of the same crate: each child starts with `use
  super::*;` (which reaches the parent's private imports and its siblings' names once `mod.rs` globs
  them back), and every item that crosses a seam becomes `pub(super)` — the reach it had as a
  private item of one file. A child can also see the parent's private items, so plumbing stays
  private in `mod.rs`. [until: reviewed 2026-09-06]
- **A `pub(crate)` item needs an explicit `pub(crate) use` in `mod.rs`** or `crate::thing::name`
  stops resolving for the rest of the crate. A glob `use self::child::*;` covers the in-directory
  names; the re-export list covers the crate-facing ones, and the two coexist.
  [until: reviewed 2026-09-06]
- **Cut by entity, never by line number, and check the count afterwards.** A range that starts one
  line late leaves a `#[test]` attached to the previous item — which is a silent lost test, not an
  error, unless the function happens to take arguments. `grep -c '#\[test\]'` before and after, and
  the test count in `verify.py`'s output, are the two checks that catch it.
  [until: reviewed 2026-09-06]
- **A moved test module also renames its snapshots**, on top of the *Tooling* bullet about `insta`
  snapshots moving with their module: the file name is the test's module path, so
  `parser::tests::foo` becoming `parser::tests::stmt::foo` needs the `.snap` moved and its `source:`
  line updated. Do that by hand instead of accepting the `.new`, and the diff stays a rename rather
  than a delete plus an unreviewable add. [until: reviewed 2026-09-06]
- **A big file hides doc comments attached to the wrong item.** Two of `nvs-types`' sat far from the
  function they described, invisible in a multi-thousand-line file and obvious the moment it became
  eight. When a carve leaves a doc block stranded above an unrelated item, that is a bug the split
  found, not one it made. [until: reviewed 2026-09-06]
- **Header prose splits with the code.** A module doc that grew a paragraph per rule *is* the split
  plan: each paragraph already names the rule it belongs to. What is left in `mod.rs` afterwards is
  its charter, which `docs/agent/doc-style.md`'s length targets say is the part that matters.
  [until: reviewed 2026-09-06]
- **A private `const` that falls out of scope becomes a binding pattern, not an error.** `TY_XML` in
  a `match` arm is a new variable once the module that holds it is a sibling, so the arm matches
  everything after it; only a multi-pattern arm gets an `E0408` naming it, and an arm of its own
  compiles and matches every column type. `pub(super)` on the consts before the first build is the
  cheap order; reading the first build's errors is the other one. [until: reviewed 2026-09-06]
- **A blanket `pub(super)` pass reaches top-level items and nothing else, and the compiler names the
  rest in one build.** Fields, `impl` methods and an `unsafe fn` all sit outside a `^(fn |struct
  |…)` regex, so the first build after a split is a wall of `E0616`/`E0624` errors, one mechanical
  widening each. The one a build does not show is a doc link: `[`hydrate`]` stops resolving when
  `hydrate` moves to a sibling, and only `verify.py --doc` says so. [until: reviewed 2026-09-06]
- **Address a moved test by its name, never by its offset.** A test's item block starts at the doc
  comment above it, so an offset table addressed at `#[test]` lines files the first documented case
  of each group into the previous module, silently. A name table also refuses to run at all when a
  case is added or renamed, which is the failure you want. [until: reviewed 2026-09-06]
- **A line-at-a-time brace counter drifts on a multi-line string literal.** `db.rs` holds one whose
  body is `… [capabilities.net]\nconnect = [\"127.0.0.1\"]\n`, and its unmatched `[` left the
  scanner at depth 1 for the rest of the file, so the last eight items looked like nothing at all. A
  splitter needs a stateful scan over the whole file — string, raw string, char, block comment — not
  a regex per line, and the reconstruction assertion is what turns that into a loud failure.
  [until: reviewed 2026-09-06]
- **An item scanner keyed on blank lines merges the methods a file wrote with none between them.**
  `ctx.rs`'s last four `Ctx` methods run `}` straight into the next `///`, so a blank-line-only rule
  found no start after the first and filed all four into one slice — silently, because the partition
  still reconstructs. The second condition is *the previous line was the `}` that closed an item*,
  and it has to be `}` specifically: a multi-line `#[expect(…)]` also returns the depth to zero, at
  the `)]` one line above the item it is attached to. [until: reviewed 2026-09-06]
- **A 3,000-line `impl` block is not a seam problem, and splitting one widens nothing.** An inherent
  `impl` may sit in any module of the type's own crate, and a private item is visible in its
  defining module and every descendant — so `Ctx`'s private fields stayed in `ctx/mod.rs` and not
  one of them changed. What widens is only what a **sibling** reads: a private method or type moved
  away from its caller, each `pub(super)` — the reach it already had. Write each fragment back
  inside a generated `impl Ctx { … }` and the move is mechanical. [until: reviewed 2026-09-06]
- **A relative reference in a doc comment is the split's own test.** `grep -n
  "above\|below\|neighbour"` over the moved halves finds the sentences that stopped being true, and
  one hit in `ctx.rs` was not stale prose but a mis-seam: `set_inbound`'s "the inbound half of the
  channel the three methods above are the outbound half of" was the file saying it did not belong in
  `output.rs`. A comment that describes its own neighbours is the only place a wrong cut announces
  itself, because the compiler never will. [until: reviewed 2026-09-06]

## Writing Novis itself

- **A `#[global_allocator]` declared in a library crate is only picked up by a binary that actually
  links that crate.** `nvs_runtime::alloc::Pooled` is registered from `nvs-runtime`'s own `lib.rs`,
  but rustc links an `--extern` crate lazily, so a new test binary whose sources never name
  `nvs_runtime` silently measures the platform heap. Name the crate in any binary that measures
  allocation; `benches/abi-probe/tests/perf_guards.rs`'s
  `an_allocation_round_trip_stays_in_the_pooled_cost_class` fails loudly in exactly that case, its
  ratio going to 1. [until: reviewed 2026-09-06]
- **A named `const` holding a `Cell` is `clippy::declare_interior_mutable_const`, which is denied
  here.** The obvious way to build a `thread_local!` array — `const EMPTY: Class = …;` then `[EMPTY;
  N]` — is refused, because a constant is copied at each use rather than referenced. The spelling
  that works is an inline const block in the repeat, `[const { … }; N]`, which is also a
  const-repeat of a non-`Copy` type and so still `const`-initializes the thread local.
  [until: reviewed 2026-09-06]
- **A block-bodied closure must write its return type, and `fn () => { … }` is `E0450`.** An
  expression-bodied closure infers its type from the expression and a block-bodied one cannot, and
  every `Core` member taking a `callable` whose callback does work rather than computing a value
  meets this. The spelling is `fn (): void => { echo "x"; }`. [until: reviewed 2026-09-06]
- **One compile error hides every later one, so "the first red fixture line" moves backwards as you
  fix it.** A parse error suppresses name resolution entirely, so behind it can sit a member the
  spec says does not exist and a subscript that panics `nvs-ir`, none of them reported until the
  parse error is gone. Budget a fixture as "run it again after every fix until it exits 0", not as
  "one report, one slice" — and do not trust a handoff's claim about which line a fixture stops at
  without running it. [until: reviewed 2026-09-06]
- **`var` takes no type annotation, and writing one costs four diagnostics a line.** `var string $s
  = …` makes the parser expect a name, find `string`, and emit `E0101` twice, `E0102`, a third
  `E0101` and an `E0406` claiming `$` is already declared — none says "a `var` has no type". The
  spellings are `var $s = …` (inferred) and `string $s = …` (declared); look for the second
  `E0101`'s "`var` infers its type from the initializer". [until: reviewed 2026-09-06]
- **`bool as string` renders `false` as the empty string**, so a line built out of `as string` over
  predicates silently loses its false columns and still looks like a shorter tally. `bool as int` is
  not a conversion at all — it is `E0708`, whose help line names the spelling to use — so the way to
  show a predicate's answer is `$b ? 1 : 0`, or a two-line helper returning a character.
  [until: reviewed 2026-09-06]
- **A `Core` member answering a union cannot be handed straight back to a parameter declared `T`.**
  `Core\Arr::append($a, Core\Arr::sum($empty))` is `E0401: expected int, found int|float|decimal`,
  because `sum`/`product`/`average` answer the whole union whatever their subject's element type
  was, and `as int` over a union does not lower. A case that wants to feed a fold's answer back into
  the array writes the literal and asserts separately that the member answers it; rendering the
  union is fine, since `echo` and `as string` both take it. [until: reviewed 2026-09-06]
- **An array literal written straight into an `array<array<mixed>>` element reads as `array<mixed>`,
  and does not satisfy an `array<array<T>>` parameter.** `Core\Arr::flatten([$s, $s])` inside an
  `array<array<mixed>>` literal is `E0401: expected array<array<mixed>>, found array<mixed>` at the
  inner literal, because the inner literal is checked against the outer's element type. Bind it
  first (`array<array<string>> $pair = [$s, $s];`) and pass the binding.
  [until: reviewed 2026-09-06]
- **An option bag's value may be a variable, and a `uint` parameter accepts an integer literal at
  the call site.** `Core\Arr::from($c, {limit: $limit})` lowers with `$limit` a `uint` parameter, so
  a swept bound does not need one call site per value, and `Drive::at(0)` against `public static
  function at(uint $limit)` needs no `as uint`. The brace literal is an expression like any other —
  only its keys are fixed by the member's row. [until: reviewed 2026-09-06]
- **A `== null` guard does not narrow a `?T` binding for an early return — `as T` states the
  narrowing.** `if ($found == null) { return "none"; } return $found;` is `E0403: this method
  declares `string` but returns `string|null``, and inverting the guard fails identically. `return
  $found as string;` compiles; a `?uint` needs the same cast because `echo` has no `uint` row, so
  one `Show::render(?T $found): string` covers both. [until: reviewed 2026-09-06]
- **A `--release` acceptance check costs a thin-LTO relink of every test binary, not just the one
  holding the guard.** At `codegen-units = 1` any upstream touch relinks them all, so a `--release`
  check in `loop-goal.toml` names its test file, and the rest of the package gets a second, debug
  check where the guards skip themselves via `#[cfg_attr(debug_assertions, ignore)]`. No cheaper
  profile: the cost class is a claim about the profile Novis ships. [until: reviewed 2026-09-06]
- **Measure a build with nothing else touching `target/`.** The same narrowed release build timed
  twice as long with a `du -sh target` walking the tree beside it: on a link-heavy build the disk is
  the contended resource, so a second reader of the same tree doubles it. A timing run that
  disagrees with an earlier one by 2x is usually this and not the change under test.
  [until: reviewed 2026-09-06]
- **A `?Instance` does narrow at file scope, and a case reaching for a `?Match` needs no helper
  class.** `var $found = Core\Regex::match($s, $p); if ($found == null) { … } else { … }` compiles,
  and inside the `else` the receiver is the class type. The neighbouring trap — a `?array<T>` that
  cannot be indexed even after a `!= null` guard — is about the element type, so do not wrap every
  nullable in a `public static function`. [until: reviewed 2026-09-06]
- **`int`'s own low end is not a writable literal, so a case pinning a 64-bit bound spells it
  `-9223372036854775807 - 1`.** `-9223372036854775808` is `E0429: this integer literal is too large
  for `int`` — the minus applies to an already-overflowed literal. Where a field's range spans `int`
  and `uint`, as for `Core\Bytes::pack`'s `J`/`P`, neither refusal has a writable argument, so
  assert the reach at both ends. [until: reviewed 2026-09-06]
- **A closure held in an array can be handed to a `Core` member's `callable` option**, and a call
  through the holding variable lowers too. `foreach ($filters as callable $filter) {
  Core\Out::capture($body, {through: $filter}) }` over an `array<callable>` lowers and runs, because
  the call is `nvs_runtime::call_closure`, not a lowered `Call`. A `mixed` is not implicitly
  narrowed, so `array<string> $row = ["a", $cell]` over a `mixed $cell` is `E0401`.
  [until: reviewed 2026-09-06]
- **An integer literal past `int` lowers in a `uint` argument and panics `nvs-ir` inside an
  `array<uint>` literal.** `Core\Random::bytes(9223372036854775808)` is fine, but `array<uint>
  $counts = [1, 9223372036854775808];` dies with *"integer literal `9223372036854775808` doesn't fit
  an `int`"*. Compute such rows, and multiply by a declared `uint $two = 2;` (`$n * 2` over a `uint`
  is `E0407`, the `2` being an `int`). [until: reviewed 2026-09-06]
- **One `RuntimeSig` may serve two runtime symbols, and changing one symbol's declaration then
  miscompiles the other.** `nvs_array_unset` was emitted through `RuntimeSig::ArrayAppend`, so
  growing `nvs_array_append` would have given `nvs_array_unset` the wrong arity. Before editing an
  `extern "C"` in `nvs-runtime`, grep `crates/nvs-codegen/src/emit.rs` for its `RuntimeSig::`
  variant and give any second `runtime_ref` its own `Signatures` entry. [until: reviewed 2026-09-06]
- **A closure passed to a `Core` member may call a static method in its body, and `"\u{0000}"` is
  how a case writes a NUL.** Only a call through the variable holding a closure is refused, so `fn
  (int $a, int $b): int => Key::magnitude($a) - Key::magnitude($b)` handed to `Core\Arr::sort`'s
  `{by}` lowers and runs. The code-point escape is the only way to put a NUL in a case (there is no
  `\0`), and `Core\Str::length("a\u{0000}b")` reads 3. [until: reviewed 2026-09-06]
- **A sweep over code points writes `Core\Str::fromCodePoint($c as uint)`, and orders two strings
  with `Core\Str::compare`.** `Core\Str::fromCodePoints` takes `array<uint>` while `Core\Arr::range`
  answers `array<int>`, and `array<int> as array<uint>` is a conversion `rule:types/conversion`
  still owes; `<`/`>` over two `string`s does not lower. `Core\Str::compare($a, $b) <= 0` is an
  `int` comparison, and over fixed-width zero-padded hex it is the numeric one, so a case can assert
  that a `Core\Uuid::v7` sweep never goes backwards. [until: reviewed 2026-09-06]
- **`Core\Math::INT_MAX as float` throws at run time, so a case wanting a huge finite float reaches
  for `Core\Math::FLOAT_MAX`.** `int as float` is one of `rule:types/conversion`'s checked
  conversions and 2^63-1 is not representable in an `f64`, so the row is `Uncaught Exception: cannot
  convert `int` 9223372036854775807 to `float`` with nothing said at compile time. `Core\Math` has
  `FLOAT_MAX`, `FLOAT_MIN` (the smallest positive normal, not `f64::MIN`), `INFINITY` and `NAN`; a
  derived infinity is `Core\Math::FLOAT_MAX * 10.0` or `Core\Math::log(0.0)`.
  [until: reviewed 2026-09-06]
- **A `Core` member declared `CoreTy::Union(NUMBER)` answers `int|float|decimal`, which no binary
  operator meets a plain `int` or `float` across.** `Core\Math::abs($n) == $m` type-checks and dies
  at run time with *"nvs-codegen does not lower a binary operator over mismatched representations"*.
  Write `Core\Math::abs($x) as int` / `as float`; `grep -n 'CoreTy::Union' <the module>` lists the
  members that owe it. [until: gone crates/nvs-stdlib/src/math.rs:const NUMBER]
- **A `Core`-owned enum cannot be written as a type — parameter or `array<T>` element — and the
  diagnostic prints the same name on both sides.** `function step(Core\Unit $u)` is `E0401: expected
  `Core\Unit`, found `Core\Unit``, because `nvs_types::core_lib` interns the name as an enum type
  and the annotation resolves to something else. A `.nvst` that steps by several units writes the
  case literal at each `plus`/`minus` call site. [until: reviewed 2026-09-06]
- **A local's name is checked, and a snake_case one is `E0112`.** `int $words_n = …` is *"local
  variable names must be camelCase, e.g. `wordsN`"*, so a counter or table name costs a whole run of
  the case to learn what the name could have avoided. Write `$wordsN` from the first draft; a
  `.nvst` wants several near-identical names at once (one `catch` binding per clause, all
  function-scoped), which is where the underscore creeps in. [until: reviewed 2026-09-06]
- **A conversion that can throw cannot live in a helper written for infallible rows — it needs the
  frame's landing block.** `nvs_ir::lower::Lowering::coerce` performs `rule:types/conversion`'s
  implicit `int`/`uint` → `float` widening, which throws above 2^53, so `&Env` had to be threaded
  through every one of its call sites and `close_nullsafe` before the row could land. Before adding
  a fallible row to `coerce` or any helper like it, check that it is handed the landing block at
  all. [until: reviewed 2026-09-06]
- **A shift count carries its operand's signedness, so a `uint` shift needs a `uint` count.** `$u <<
  64` is `E0407: int and uint have no representable common type in arithmetic`, because
  `nvs_types::expr::operators::bitwise_result` refuses a mixed-signedness pair for all five binary
  bitwise rows and a count is just the right-hand operand. Declare `uint $width = 64;` and shift by
  that; on the `int` arm a negative count is `ArithmeticError: Bit shift by negative number` and a
  count of 64 or more answers `0` (or all-sign for `>>`). [until: reviewed 2026-09-06]
- **A `decimal` binding takes a plain decimal literal, not a `d` suffix.** `decimal $d = 1.25d;` is
  four errors deep — the lexer reads `1.25d` as a duration literal and reports `E0007: `.` has no
  meaning in a duration`, then the checker reports the wreckage as `E0401: expected `decimal`, found
  `mixed`` — and none of them names the real problem. `rule:types/numeric-literal-placement` makes a
  numeric literal untyped until placed, so `decimal $d = 1.25;` is the whole spelling.
  [until: reviewed 2026-09-06]
- **An enum is not spelled the way PHP spells it: bare names, commas, no `case` keyword.** `enum
  Mode: int { case Read = 1; }` parses as a class and cascades `E0220` *"an enum declares only cases
  and an optional backing type"* at the `{` then `E0101` *"expected a class member"* per line. Write
  `enum Mode { Read = 1, Write = 2 }` (`enum Mask: uint { … }` for a `uint` backing, since a case
  past `int` is `E0437`). [until: reviewed 2026-09-06]
- **A nullable property cannot default to `null`.** `public ?string $s = null;` is `E0472: a
  property default must be a `string|null` constant — not a constant of the declared type`, and the
  same for every `?T`; a local `?object $m = null;` is fine, it is only the property-default folder
  that refuses the `null` literal. Declare the property non-nullable and fill it in `constructor`.
  [until: reviewed 2026-09-06]
- **"invalid pointer width (got 128, expected 64)" from cranelift, with `i128` as the failing
  operand, is the signature of a missing `InstKind::Untag`.** A tagged value reached an instruction
  that wanted an object: a write through a property has two receivers to untag, and
  `write_back_array`'s property arm (`$m->rows["0"] = "w"` through a narrowed `?T` local) lacked the
  `untag_receiver` call `lower_reassignment`'s arm has. Read it as an untag hole, not a codegen bug
  — the checker is happy and the lowering never panics, so nothing above catches it.
  [until: reviewed 2026-09-06]
- **A local's slot is re-pointed in four places in `nvs_ir::lower`, and a rule hooked into
  `bind_local_value` catches three.** `write_back_holder` and `write_back_array` `env.insert`
  directly, and `foreach (… as inout $v)` re-points the array binding by hand, so a nested `inout`
  foreach silently left the outer array stale. `grep -n "env.insert(" crates/nvs-ir/src/lower/` is
  the whole check for any rule phrased "whenever this name is rebound". [until: reviewed 2026-09-06]
- **A `holes.py` item's title is an inventory's claim, not a proof: only a
  `panic!`/`todo!`/`unimplemented!` counts as a site, attributed by file.** The class-test lowering's
  counted panic was a diagnostic while the `assert!(matches!(ty, Ty::Object))` four lines below
  aborted `$m is Box` over a `mixed`. Read the whole function and the site's *else* branch,
  and spend one scratch `.agent-tmp/*.nvs` per operand shape before believing the item.
  [until: gone tools/holes.py]
- **A `#[should_panic(expected = "known gaps")]` test pins a lowering hole, so closing the hole
  turns it red.** The failure reads like a regression and is the opposite. `grep -n "should_panic"
  crates/<crate>/src` first, rewrite it as a snapshot test of what now happens, and run `cargo insta
  test -p <crate> --lib` (plain `cargo test` stops at the first `.snap.new`), checking `git status
  --short | grep pending-snap` before `cargo insta accept`. [until: reviewed 2026-09-06]
- **An increment in value position runs the write where its own branch runs, and `echo` prints
  operand by operand.** `if ($no && ($k++ > 0))` leaves `$k` at `0`, `$absent ?? $s++` increments
  only when the left side is `null`, and a `match` arm's increment runs only for the arm that
  matched — all PHP-identical, all easy to write an expectation against as though the operand were
  unconditional. `echo "made: ", Cell::make()->count++, "\n";` prints `made: ` before `make` runs,
  so a case whose operand has a side effect has to expect the interleaving.
  [until: reviewed 2026-09-06]
- **A storage resolved at compile time and a spelling PHP resolves at run time agree until two
  classes disagree, so a scratch file judging one has to make them disagree.** `static::$total`
  inside `Base` answered `Base`'s slot for `Sub::viaStatic()` where PHP answers `Sub`'s, and only a
  subclass that *redeclares* the static shows it (it is `E0499` now rather than a silent
  difference). Write the redeclaring subclass into the probe before trusting a match with PHP.
  [until: reviewed 2026-09-06]
- **A closure's declared parameter types are checked by nobody, and a mismatch is an arbitrary
  dereference.** `fn (string $s)` mapped over an `array<int>` dies in `nvs-runtime`'s `string.rs` on
  a misaligned pointer, because `rule:types/closure-literal` gives `callable` no parameter list, so
  `invoke` reads each slot at its declared representation. Spell the element type and the parameter
  type the same, and read a crash with no Novis frame as this first. [until: reviewed 2026-09-06]
- **An integer literal in an array-literal element position keeps `int`, whatever the array's
  declared element type says.** `array<uint> $u = [7, 8];` compiles with elements tagged `int`,
  while `[7 as uint, 8 as uint]` carries `uint` — `rule:types/numeric-literal-placement` applies at
  a parameter and a binding but not at an element. Convert in the literal whenever the tag is the
  subject; only a `callable`'s parameter-tag check observes it today. [until: reviewed 2026-09-06]
- **A property default may not be an array with entries in it.** `public array<string> $rows = ["a"
  => "x"];` is `E0472` (*"a property default must be a `array<string>` literal"*, which reads as if
  the literal were mistyped), and `[]` is the only array a declaration may carry. Fill a pre-filled
  array property with element writes after the `new` or in `constructor`; a `public static` one has
  no constructor, so its writes are top-level statements. [until: reviewed 2026-09-06]
- **A panic's message names the shape it was written for, not the shape that reaches it.**
  `stmt.rs`'s property-write panic still blamed "a receiver erased to a plain `object`" after that
  half had landed; what actually reached it was a computed member name and an undeclared property on
  a class kind the checker excused, and believing the message would have rebuilt a feature that was
  there. Enumerate the arms of whatever records the table entry and probe one scratch
  `.agent-tmp/*.nvs` per arm before taking the panic's own account of itself.
  [until: reviewed 2026-09-06]
- **"No shape left the checker accepts" is a claim, and the cheap judge is the roster and the binary
  rather than the arms.** `StmtKind` has more variants than `lower_stmt` has arms, and `autoload`,
  `namespace`, `use` and `type` reached the catch-all from source the checker accepted. Grep the
  enum's variants, subtract the arms, then write one scratch `.nvs` per survivor and `nvs run` it;
  reading the arms only re-derives the claim. [until: reviewed 2026-09-06]
- **Inside a `namespace`, a qualified name resolves relative to it — `Core\` included.** After
  `namespace App;`, `Core\Str::length("abc")` and `#[Core\Route(...)]` are `E0303: App\Core\Str is
  not declared`, which names the joined path rather than the missing backslash. Write `use
  Core\Str;` or the leading `\` — the same rule makes `App\User::class` inside `namespace App;`
  `App\App\User`, as PHP does. [until: reviewed 2026-09-06]
- **Widening what the checker accepts for an integer literal opens a hole in `nvs-ir` one crate
  down.** `lower_int_literal` decides `ConstInt` versus `ConstUint` from the `expected: Option<Ty>`
  its caller threads, not from anything the checker recorded, so a literal newly placed at `uint`
  still panics with *"nvs-ir: integer literal `…` doesn't fit an `int`"* wherever the position hands
  no `Ty` down — `lower_binary` passes `Some(lty)` to its right operand but only the whole
  expression's `expected` to its left. The two crates have to make the same placement; the checker's
  half alone is not the feature. [until: reviewed 2026-09-06]
- **`inout ...$rest` does not parse, and spreading into a variadic `inout` tail is accepted in
  silence.** `Parser::parse_arg` tests for `...` before it eats `inout`, so the marked spelling is
  `E0714` plus a cascade of `E0101`/`E0102`, while `Adder::many(...$rest)` against `function
  many(inout int ...$xs)` compiles and runs with nothing written back. `E0714`'s "a spread's
  entries" half is reachable only through a fixed `inout` parameter; a case wanting the variadic row
  has to wait for that hole. [until: reviewed 2026-09-06]
- **A new `nvs_ir::Helper` needs a fourth edit, and the three obvious ones build without it.** The
  variant, `nvs_ir::print`'s name and `nvs_codegen::emit`'s `helper_symbol` row all compile; what
  fails is `cranelift-jit` panicking at run time with `can't resolve symbol nvs_value_add` and no
  Novis frame in the message, because a `#[no_mangle]` helper is found in
  `nvs_runtime::helpers::symbols()`'s `(name, address)` table, never by name in the host process.
  `grep -n "nvs_call_closure" crates/` names all four sites of a neighbouring helper at once.
  [until: gone crates/nvs-runtime/src/helpers.rs:fn symbols]
- **A checker refusal phrased "this operand is not one of these rows" does not cover a `mixed`
  operand, and the hole opens one crate down.** `reject_unary_arith_operand` decides from
  `equality_domain`, which answers `None` for `mixed` or a union, and `!matches!(…, None |
  Some(Numeric))` takes `None` as accepted — rightly, the deferral is what `mixed` is for — so `-$m`
  walked past the checker into `nvs-codegen`'s representation catch-all. Every such site owes a
  tagged answer in the runtime or a diagnostic naming `mixed` explicitly; running the shape in a
  scratch `.agent-tmp/*.nvs` tells the two apart in one call. [until: reviewed 2026-09-06]
- **`nvs-codegen` has two catch-alls under one operator, and `Ty::Bool` passes the cheap-looking
  gate.** `emit_binop`'s representation gate near the top is `matches!(ty, Ty::Int | Ty::Uint |
  Ty::Bool)`, so an unrowed `bool` pair never reaches either message: `true + true` lowered to an
  `iadd` over the `i8` and printed a number, where the same hole over two `string`s merely refused.
  When closing an operator table at the checker, probe the `bool` row first and read its answer
  rather than its exit status — a refusal you can see is the good case. [until: reviewed 2026-09-06]
- **A `catch` binding has no methods at all, and every PHP accessor on one is `E0405`.** `catch
  (Throwable $e) { echo $e->getMessage(); }` is refused where it is written, with a help naming the
  property that answers the same question (`->message`);
  `nvs_types::expr::calls::report_exception_accessor` is that mapping's home. A case that wants to
  show what was thrown reads `$e->message`.
  [until: gone crates/nvs-types/src/expr/calls.rs:fn report_exception_accessor]
- **A discriminant chosen as "one past the roster" collides with the next roster entry, and nothing
  in the roster's own crate says so.** `FN_PARAM_TAG_ANY`/`CLOSURE_PARAM_TAG_ANY` — the
  closure-parameter nibble meaning "no argument can be wrong for this one" — sat at the end of the
  `nvs_runtime::Tag` roster, so adding `Tag::Unset` made every `mixed` closure parameter demand a
  tag, and the failures named closure arguments rather than the tag. Both constants are `15` now,
  parked at the top of the nibble; `nvs-codegen`'s `the_any_nibble_denotes_no_tag_at_all` holds
  three crates' copies of the number together. [until: reviewed 2026-09-06]
- **A property access records the class the receiver was typed as, not the class that declared the
  property.** `nvs_types::expr::members::check_property_member` writes `ExprInfo::Property { class:
  qname }` from the receiver — right for `InstKind::FieldGet`, wrong for any per-property fact
  `nvs-ir` looks up by that label: an inherited `lateinit` read through a subclass answers `false`
  against the parent's own-only record and the guard is silently not emitted. Flatten such a table
  along the class graph where it is recorded; the failure is quiet, because a method that never
  touches `$this` runs fine on a null receiver. [until: reviewed 2026-09-06]
- **An `nvs-ir` instruction's kind is often bound to a local first, so grepping `self.emit(` for a
  literal `InstKind::` misses sites.** `let call = InstKind::HelperCall { … };` followed by
  `self.emit(cur, ty, call)` several lines down (`convert.rs`'s `as ?T` and `array<T> as array<U>`
  rows) is invisible to a scanner that parses the kind out of each call. Turn `nvs_codegen::emit`'s
  tolerant `None` arm into an `internal(...)` refusal and run the crate's tests: the backend already
  knows which instructions return a status, so let the suite enumerate the producers.
  [until: reviewed 2026-09-06]
- **An attribute's name is already load-bearing for two other passes before
  `rule:attributes/attach-sites-and-forms` says what it means.**
  `rule:core-classes/derive-attribute` matches `#[Json\Derive]`/`#[Json\Field]` nominally against a
  closed `Core`-owned roster, and `rule:programs/autoload` harvests every attribute name as a
  reference the autoloader places by prefix. A rule about what an attribute name may resolve to has
  to exempt the first and leave the second alone;
  `tests/conformance/lang/a-class-named-only-by-an-attribute-is-autoloaded.nvst` and the
  `json-derive-*` cases are what fail first. [until: reviewed 2026-09-06]
- **A size check is not a loop bound.** `Core\Bytes::repeat` and `Core\Str::repeat` checked the
  product (`affordable` on `len * times`) and then ran `for _ in 0..times`, so an empty subject made
  the product zero for every count and the loop spun a caller-supplied `uint` of iterations
  appending nothing — an unbounded spin on the request path that builds, is correct, and returns.
  Look for a loop whose iteration count is the caller's count rather than the result's size; the
  size seams cannot see it, and both members short-circuit on an empty subject now.
  [until: reviewed 2026-09-06]
- **`nvs_runtime::affordable` is not the last check, and the gap is one header wide.** It accepts
  any size up to `isize::MAX` and knows nothing of the container header the allocation prepends, so
  a `Core\Str` producer handed exactly `isize::MAX` cleared it and panicked in `str_layout`'s two
  `expect`s — a FATAL out of the fallible `NvsStr::try_build`; `try_str_layout` closes it for every
  `built_fallibly` caller. The shape to recognise is a member whose size check and allocation are
  two different expressions (`padding_run` bounds the run while `built_fallibly` allocates the run
  plus the subject), and the boundary count is the largest the first accepts, not a round number.
  [until: reviewed 2026-09-06]
- **An expression-bodied `fn (): void => Something();` panics `nvs-codegen`, and the block-bodied
  form of the same closure does not.** `Core\Out::capture(fn (): void => M::run())` dies with
  `nvs-codegen does not lower an operand used before it is defined`, while `fn (): void => {
  M::run(); }` and `fn (): int => M::n()` both run, so it is the `void` return that is unlowerable.
  Write the braces; the panic names neither the closure nor its return type.
  [until: reviewed 2026-09-06]
- **A refusal the parser writes takes an `E02xx` code, not the `E01xx` "next free parser code".**
  The bands are by kind, not by which crate reports them: `E01xx` is a malformed parse, `E02xx` is a
  rejected PHP construct, and `E_IMPORT_ALIAS_UNSUPPORTED` (`E0212`), `E_ENUM_MEMBER_UNSUPPORTED`
  (`E0220`) and `E_IMPORT_GROUP_UNSUPPORTED` (`E0238`) are all reported from `parser/decl.rs`. Find
  the sibling refusal's code first and take the number next to it, whatever band `brief.py`'s
  next-free list points at. [until: reviewed 2026-09-06]
- **A function's disassembly holds more cold blocks than its IR does, so "skip the `Propagate`
  blocks" does not skip the cold path.** Codegen invents blocks no `nvs_ir` block corresponds to —
  an ArithmeticError construction behind a `seto` check, the landing block's `Release` — and
  `rule:testing/debug-probes`'s probe calls are conditional, so a whole-section `call` count in
  `--dump-asm` prices three other mechanisms. What separates hot from cold in the VCode text is one
  shape, `testX` immediately followed by `jnz labelA; j labelB`, which is how every status word is
  checked: walk the `blockN:` graph from the entry and never follow that `jnz`.
  [until: reviewed 2026-09-06]
- **A fixture that needed a `mixed` value has probably reached for whatever inferred `mixed` that
  week, and closing a gap moves it.** `an_array_index_through_a_mixed_base_defers_to_the_tag`
  (`crates/nvs-types/src/expr_table.rs`) used `T::UNTYPED[0]` over an unannotated constant purely
  because that inferred `mixed`; giving constants a type turned it into an `int` subscript and
  failed the deferral it was written to assert. Give such a fixture the erasure it means — a `mixed`
  parameter — and when a slice widens what the checker knows about a shape, `grep` the test tree for
  that shape used as a *source*. [until: reviewed 2026-09-06]
- **`Core` is two rosters, not one, and the second is the exception tree.** Narrowing a `Core` name
  from the `is_core()` spelling test to `nvs_stdlib::registry` looks total — the registry calls
  `CLASSES` the whole roster — and refuses `new Core\Test\Failure(...)`, which lives in
  `nvs_hir::errors::TREE`; `QName::is_reserved_global_class`'s doc says so, on the predicate you are
  not editing. Any check reading `is_core()` as "in the registry" owes `errors::is_exception_class`
  beside it. [until: reviewed 2026-09-06]
- **A "return-only" type has two enforcement sites, and a tree can have neither while looking like
  it has one.** `rule:types/grammar`'s `void`/`never` were return-only in prose alone: `never $p`
  panicked `nvs-ir`'s `erase_checked_ty`, while `void $p` type-checked, lowered, and died in codegen
  with `internal error: reading a value of representation 'void'` — a third outcome
  `crates/nvs-ir/tests/type_atoms.rs` cannot see, because it stops at `lower_program`. Two minutes
  of `nvs run` on a two-line scratch file tells a shape that panics, one that is refused, and one
  that reaches codegen apart. [until: gone crates/nvs-ir/tests/type_atoms.rs:lower_program]
- **`nvs-ir` does not depend on `nvs-hir`, so an `ExprInfo` variant carrying a `QName` cannot be
  destructured by name there.** `nvs_types::expr_table::ExprInfo` names `nvs_hir::QName` freely and
  `nvs-ir` gets away with it only because every site calls `class.to_string()` without writing the
  type; a lowering helper wanting a `&[nvs_hir::QName]` fails with `unresolved module or unlinked
  crate nvs_hir` at the signature, which reads as a missing `use` and is not one. Convert to
  `String` at the `self.exprs.lookup(...)` site and let the helper take `&[String]`; adding the
  dependency would put the whole HIR in the lowering crate's graph for one type name.
  [until: reviewed 2026-09-06]
- **A `Core` attribute name cannot be a `rule:attributes/attach-sites-and-forms` shape alias,
  because there is no `Core`-seeded alias table.** `nvs_hir::AliasTable` is collected from source
  `type` declarations only (`crates/nvs-hir/src/aliases.rs`), so `Core\Command` reaches
  `nvs_types::attributes::resolve_shape_alias` with no entry and the answer is `E0726: … is not a
  `type` alias` — a refusal no stdlib edit can lift. Every `Core`-owned attribute is a nominal match
  on `nvs_types::derive::ATTRIBUTES` and owes a pass that checks its payload, or `#[Command(nmae:
  "x")]` is admitted in silence. [until: reviewed 2026-09-06]
- **A `Class::CONST` whose class does not exist passes checking and panics `nvs-ir`.** `echo
  \Core\Http\Method::Get;` type-checks clean and dies in `crates/nvs-ir/src/lower/expr.rs` with "a
  `Class::CONST` … with no value recorded in the typed-expression table", because `nvs_types::check`
  reports no `E0405` for the class half of that spelling. A one-line probe that compiles is not
  evidence that the name it writes resolves to anything — the same hole lets a payload roster naming
  an undeclared enum admit a case of the wrong enum. [until: reviewed 2026-09-06]
- **`array<K, V>` is a spelling the docs write and the type system has no form for.**
  `nvs_types::ty::Ty::Array` carries one `TypeId` and there is no `CoreTy` for a keyed array — keys
  are `int|string` by construction and not part of the type — so `array<string, mixed>` in the spec
  is declared as `CoreTy::Array(&CoreTy::Mixed)` and the key rule is enforced at the call site where
  it can be. Same family: `Core\Arr::append`'s second parameter is one element, so
  `Core\Arr::append($a, $b)` over two arrays is `E0401: expected int, found array<int>`; build the
  combined array with a `foreach` and one `append` per element.
  [until: gone crates/nvs-types/src/ty.rs:Array(TypeId)]
- **A field added to `nvs_types::Env` builds clean and fails `--all-targets`.** There are three
  construction sites and one (`crates/nvs-types/src/lower.rs`) is inside a `#[cfg(test)]` module, so
  `cargo build -p nvs-types` is green while `cargo check -p nvs-types --all-targets` is the first
  thing that reports `E0063: missing field`. The two real sites are `check.rs`'s per-file loop and
  `signatures.rs`, which wants a scratch value because its pass runs before anything fills the new
  table. [until: reviewed 2026-09-06]
- **A refusal over a derived fact fires on declarations already refused for something else, and the
  existing `--EXPECTF-ERROR--` case is what catches it.**
  `rule:core-classes/derive-generates-what-is-missing`'s "an attribute with no effect is a mistake"
  read as "refuse a `#[Json\Derive]` class whose field list came out empty", but
  `reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst` has both its properties refused, so
  its list is empty too and its frozen error count moved. Before adding a diagnostic whose condition
  is the absence of a result, grep the reject tree for a case that already makes that result absent,
  and model the outcome per item (kept / skipped / refused) rather than as an `Option`.
  [until: reviewed 2026-09-06]
- **Renaming a `pub` item breaks intra-doc links written in its neighbours, and the only step that
  says so is `verify.py`'s last one.** A sibling field's doc comment saying "exactly as
  [`Self::query`] does" builds, tests, clippies and formats cleanly and then fails `cargo doc` with
  `-D rustdoc::broken_intra_doc_links` — a whole verify run spent on a four-character edit. Before
  renaming, `grep -n "Self::<oldname>\|\[\`<oldname>\`\]"` over the crate; an intra-doc link is
  invisible to every other tool in the gate. [until: reviewed 2026-09-06]
- **A new `CoreTy` variant that wraps another type silently falsifies every `_ => {}` walker in
  `crates/nvs-stdlib/src/registry.rs`.** Each recursive `match` over `CoreTy` — `collect_written`
  and the ones inside `mod tests` — ends in a wildcard under "a variant that carries no nested type
  carries no variable either", and `CoreTy` is `#[non_exhaustive]`, so a `Classified(Qual, &'static
  CoreTy)` wrapper compiles clean while making `written()`, the nullable check and the enum-case
  check blind to what it wraps. Add a leaf instead, as `CoreTy::Text(Qual)` / `CoreTy::Blob(Qual)`
  are, and change only the sites that name the sibling leaf specifically
  (`nvs_types::core_lib::lower` and one registry test). [until: reviewed 2026-09-06]
- **`every_error_path_is_asserted_or_declared_unreachable` reads its declaration phrase only within
  `DECLARATION_WINDOW` lines above the `Fault::` line, so a long comment with the phrase at the top
  declares nothing.** `conformance_coverage.rs`'s scan walks upward from the site and stops at the
  first line holding `Fault::`, and the failure re-prints the worklist line with no hint that the
  comment exists. Put `unreachable from source` in the comment's last sentence, and give a helper
  with several guards one declaration per guard rather than one at the top of the function.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
- **A grapheme count does not decompose into one correction per seam, and regional indicators are
  the whole of why.** A cached `Core\Str::length` makes a concatenation want `left + right - (a
  cluster spans the join)`, right for every UAX #29 rule but GB12/GB13, which group a run of
  `Regional_Indicator`s into pairs: `"🇩" . "🇩🇪"` is two clusters where each side alone is one, and
  the parity that fixes two pieces is wrong for three. `nvs_runtime::graphemes::seam_joins` refuses
  the seam when a regional indicator sits on both sides and the caller leaves the count uncached; a
  cached aggregate over Unicode text may only be corrected locally for the rules that are local.
  [until: reviewed 2026-09-06]
- **A `Core` call that builds its argument vector by hand must call `account_for_arg` itself, and
  only a valgrind run will say it did not.** `nvs_ir::lower::lower_route_link` bypasses
  `lower_call_args` and lowered `$params` without staging it on `owned_temporaries`, so every
  `Core\Router::url("…", ["id" => 7])` leaked one array header per call while compiling, running,
  and passing every test. Wherever lowering hand-rolls what a shared helper does, the accounting is
  the half that gets dropped: run `MSYS_NO_PATHCONV=1 wsl.exe -- bash tools/leak-check.sh <fixture>`
  from the Bash tool, and pin it as `a_resolved_route_link_releases_its_params_array` does — require
  a `Release` of the argument's own `ValueId`, never a count of releases.
  [until: reviewed 2026-09-06]
- **Classifying a `Core` class can break that class's own structural unit test, and the
  `rule:security/sink-predicate` ratchet says nothing about it.** `Core\Test`'s rows share one
  `MESSAGE: &[CoreOption]`, and `test::tests::the_only_option_is_a_message_that_defaults_to_absent`
  asserts `matches!(bag[0].ty, CoreTy::Str)` over every one, which `CoreTy::Text(Qual::Neutral)` is
  not — so the registry-wide `every_member_parameter_carries_a_qualifier_classification` passes
  while a test naming a member you did not think you were editing fails. Grep the class's own `mod
  tests` for `CoreTy::Str` before classifying it. [until: reviewed 2026-09-06]
- **A prepared `Core` member has two bodies, and the registry names the one that throws.**
  `nvs_stdlib::registry`'s `symbol` for `Core\Router::url` is `nvs_core_router_url`, whose whole
  body is `Err(no_such_route(...))` — the unfolded path a computed argument takes — while
  `nvs_ir::lower::expr::lower_route_link` swaps a literal name for a prepared path and calls the
  second symbol, `nvs_core_router_link`, which does the work. For any member
  `rule:expressions/intrinsic-literals` prepares rather than folds, grep the crate for a sibling
  symbol before believing the body the row points at, or run four lines with `nvs run`.
  [until: reviewed 2026-09-06]
- **A class is not generic at the `new` site.** `new Core\Task\Channel<int>(2)` is `E0441: this
  target takes no type arguments` — the `<T>` positions are the built-in ones (`array<T>`,
  `Iterator<T>`, `Core\Program::implementing<T>()`), not a user or `Core` class's constructor. A
  container's element type is carried by the `foreach` binding (`foreach ($chan as int $v)`) and by
  the declared type of what goes in, which is what `examples/channel.nvs` is written against.
  [until: reviewed 2026-09-06]
- **Dropping a suspended `corosensei` coroutine unwinds its stack, and a `catch_unwind` in the way
  aborts the process.** `Coroutine::drop` raises a private `ForcedUnwind` marker and expects it
  back; a `catch_unwind` under the task root that swallows it makes corosensei's own `panic!("the
  ForcedUnwind panic was caught and not rethrown")` fire inside a `Drop` and double-panic to
  `STATUS_STACK_BUFFER_OVERRUN` (`0xc0000409`; a plain abort on Linux), naming
  `corosensei-0.2.2/src/coroutine.rs` twice — the second, *"cannot propagte coroutine panic with
  #![no_std]"*, is the double-panic, not the cause. `nvs_runtime::Teardown` makes `run_task`
  re-raise instead of contain, and anything else gaining a `catch_unwind` between a coroutine's root
  and its suspension points owes the same guard. [until: reviewed 2026-09-06]
- **A non-blocking `connect` cannot be completed by asking `peer_addr`, whatever `mio`'s own example
  says.** On Windows that call answers `Ok(the target address)` for a socket whose connect never
  succeeds, and `take_error` is `Ok(None)` too until the attempt actually ends, so a connect built
  on either alone reports success and hands back a stream that is writable and dead. The pair sound
  on both platforms is `take_error` plus a zero-length write — `Ok(0)` when connected,
  `NotConnected`/`WouldBlock` in flight, the refusal itself on Linux — and
  `crates/nvs-host/src/net.rs`'s `finish_connecting` is the worked shape.
  [until: reviewed 2026-09-06]
- **A poll asked to wait a bounded time can come back early, so "0 woken" is not "nothing can wake
  these tasks".** A platform rounds a wait to its own timer granularity and returns a fraction of a
  millisecond before the deadline, and `run_until_idle` breaks on `Some(0)`, so a timer written
  straight into `Reactor::turn` abandons a lone sleeping task and the test hangs on an assert rather
  than the clock; `turn` retries until it has genuinely reached the earliest deadline
  (`crates/nvs-host/src/reactor.rs`). The giveaway that it is this and not a lost wake: the same
  code with a second runnable task passes. [until: reviewed 2026-09-06]
- **A task cannot reach the `&mut Scheduler` that is resuming it, and every obvious design for the
  task tree dies on that.** `Scheduler::run` holds `&mut self` for the whole turn, so a running task
  can call neither `spawn` nor `cancel`; the part a task needs — id counter, parent links, cancel
  flags, pending children — lives in an `Rc<RefCell<..>>` the scheduler publishes in a thread-local
  for the length of its turn (`scheduler.rs`, the shape `crate::reactor` already uses). Never call
  `Coroutine::force_unwind` from a task's own stack: teardown belongs on the scheduler's stack, so
  cancellation marks and the next turn unwinds. [until: reviewed 2026-09-06]
- **A running task cannot wake a peer, and `Scheduler::wake` is not the route.** It takes `&mut
  Scheduler`, the frame currently resuming the task, so anything that unblocks another task goes
  through `nvs_host::Wake`: a handle taken while the waiting task is running, which queues a
  `TaskId` on the task tree and is drained into the run queue by `Scheduler::run`. Two things are
  load-bearing — it holds the tree rather than reading the `TREE` thread-local (a `TaskId` is unique
  only within its tree, and two schedulers on one thread is a shape most tests take), and the drain
  runs at the top of `run` as well as after every resume, or a wake issued between turns leaves the
  parked task asleep with no error anywhere. [until: reviewed 2026-09-06]
- **One new row in `nvs_stdlib::registry::CLASSES` owes four gates, three of them outside the crate
  you are editing.** `every_registered_member_has_an_implementation_address` wants a symbol with a
  real address, `every_part_one_member_has_a_conformance_case` wants a `.nvst` under
  `tests/conformance/` whose `--FILE--` writes `Core\X::y(`,
  `every_core_class_has_a_conformance_floor_of_three` wants three, and `BELOW_THE_FLOOR` only
  shrinks. That is not a bar on registering a member whose runtime is a later slice: the coverage
  gate greps the `--FILE--` section and never runs anything, so `--EXPECTF-ERROR--` cases discharge
  it, and the address gate takes a body that says at its own site why it was reached — never a
  placeholder `Fault::`, which `every_error_path_is_asserted_or_declared_unreachable` then wants a
  case or a declaration for. [until: reviewed 2026-09-06]
- **A bare-message failure with no runtime error class installed loses its message at
  `Ctx::take_thrown`, and answers a null `Thrown`.** `set_pending` files a `Pending::Message` and
  the promotion into an object needs a class descriptor — `Thrown::new_as` returns `Thrown::none()`
  for a null one, whose `message()` is `""` — so a Rust-side test that built `Ctx::new(...)` by hand
  reads `left: ""` against the message it just set, with nothing pointing at the missing class.
  `crates/nvs-host/src/group.rs`'s `ctx_with_error_class` is the four-line fixture
  (`Ctx::set_runtime_error_class`); asserting on `matches!(.., Threw(_))` instead is a weaker test
  for no reason. [until: reviewed 2026-09-06]
- **A `Core` member may not park, because a forced unwind cannot cross its `extern "C"` frame.**
  `nvs_host::Scheduler::tear_down` cancels a parked task with `corosensei`'s `force_unwind`, and a
  member that suspended (`Core\Time::sleep` through `Host::sleep`) is a `nvs_helper!` frame on that
  stack — first `the ForcedUnwind panic was caught and not rethrown`, then, once `run_helper`
  re-raises on `Teardown::in_progress()`, `panic in a function that cannot unwind` naming the
  helper. `extern "C-unwind"` is not the fix, since the JIT frame below has no landing pads; a
  cancelled task dies by `rule:errors/propagation`'s return status at its next safepoint, which is
  what `nvs_safepoint` gives `SafepointFlags::CANCEL`. [until: reviewed 2026-09-06]
- **A task that dies by `rule:errors/propagation`'s return status leaves a pending message on its
  context, and whatever collects that context must not read it as a throw.** The symptom is a
  program whose deadline works printing `uncaught in a cancelled sibling: the request was cancelled`
  and then an uncaught exception at the `Core\Task::map` call site: the child's `ctx.pending()` was
  the safepoint's own record of the teardown. Ask `Ctx::cancelled()` before `pending()` (as
  `nvs_host::group::Child::run` does) and leave a cancelled child's slot empty; every future
  collector of a child context — the request boundary under `nvs serve`, whatever reports a `spawn
  script` — owes the same check. [until: reviewed 2026-09-06]
- **A `CoreTy::Uint` parameter arrives tagged `Tag::Uint` (3), not `Tag::Int` (2), so
  `Value::as_int()` on one answers `None`.** `uint` is a tag of its own by `rule:types/arithmetic`,
  and the failure is neither a compile error nor a wrong number but the member's own "expected an
  int, got tag 3" fatal, which reads as a caller bug and is not one. `Value::as_uint()` is the
  reader, `Value::uint(…)` is what a `-p nvs-stdlib` test hands such a member, and
  `crates/nvs-stdlib/src/arr.rs` has the shape to copy. [until: reviewed 2026-09-06]
- **`Qual::Sink` needs no rule of its own in `nvs-types`, and that is not the gap it looks like.** A
  classified `CoreTy::Text(q)`/`Blob(q)` lowers to exactly what its unclassified spelling does
  (`nvs_types::core_lib`), so a sink parameter's refusal is ordinary assignability — a `tainted
  bytes` argument does not satisfy a plain `bytes` — and grepping for code that reads `Qual::Sink`
  finds `crates/nvs-stdlib` and nothing else. What needs a checker rule is the opposite shape, a
  sink whose parameter is `mixed` (`Core\Debug::dump`, `Core\Serialize::encode`), where nothing
  below the call can still see the qualifier; those live in `expr/quals.rs` as call-site walks over
  the written arguments. [until: reviewed 2026-09-06]
- **The two `Core` coverage gates run in opposite directions, and a class with no members owes
  neither anything.** `conformance_coverage.rs` walks `registry::CLASSES` and asks for a `.nvst` per
  member; `spec_registry_coverage.rs` walks `docs/spec/01-core-library.md`'s `| Member | Signature
  |` rows and asks the registry for each — so a memberless class like `Core\Script\Handle` turns
  both green with no spec edit and no case. Adding a spec row for such a class is the trap: it would
  demand a member the class exists in order not to have; its prose home is
  `docs/spec/00-overview.md` § 2 instead.
  [until: gone crates/nvs-stdlib/tests/spec_registry_coverage.rs]
- **A `CoreCall` to a symbol with no registry row links only if `nvs_stdlib::symbols()` chains it
  in** — an arm in the module's own `address()` is not enough, and the failure is a Cranelift panic
  at run time reading `can't resolve symbol nvs_core_script_spawn`, naming no Novis file.
  `symbols()` walks `registry::CLASSES` and `CONSTRUCTORS`, so a member with a row is found for
  free; a rowless symbol — the prepared link entry points, `spawn script` and `await` — needs its
  own `.chain([...])` there beside `address`'s arm, plus a term in
  `every_registered_member_has_an_implementation_address`' arithmetic. Two registrations, not one,
  and the second has no compile-time gate at all.
  [until: gone crates/nvs-stdlib/src/lib.rs:fn symbols]
- **The live graph carrier keeps the source object's descriptor, so `rule:classes/graph-copy`'s
  *unresolvable class* has no counterpart there until someone hands it a receiving table.** `decode`
  resolves a class by name and refuses one the program does not declare, but `copy_graph` never
  resolved anything, because both sides of a `clone` are one program — at the isolate boundary they
  are not, since `nvs-cli` compiles one unit per written path (`copy_graph_into(value,
  Some(&resolve))` and `Live::admit` carry the rule there). Do not read a refusal in `graph.rs` as
  covering both carriers; the `Carrier` trait is the list of what they share.
  [until: reviewed 2026-09-06]
- **A transferred call argument is released by the landing block, not by the normal edge.**
  `release_temporaries_since` skips a `TemporaryKind::Transferred` entry, so a lowering test
  asserting "the transferred value is never released" fails on the error path, where the frame still
  owes it: a callee that returned non-OK never took the reference. Assert per block — the call's own
  block for what the normal edge does, `inst.on_error`'s for what the throw does.
  [until: reviewed 2026-09-06]
- **`===` and `!==` do not exist**, and reaching for one in a `.nvst` is `E0232` on the operator
  rather than a type error you can read past: Novis keeps exactly one equality operator, `==`, which
  never converts either operand. A null test is `$x == null`.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E_IDENTITY_OPERATOR_UNSUPPORTED]
- **A new `[limits]` key needs four edits, and the one that is easy to miss makes the other three
  read as a silent default.** `Request::get` resolves a bare name to `limits.<name>` only when
  `nvs_config::value::unit_of` knows the leaf, so a key with its `Limits` field
  (`crates/nvs-config/src/tree.rs`), its `DIRECTIVES` row (`crates/nvs-config/src/directive.rs`) and
  its `Ctx` reader but no `unit_of` arm never reaches `[limits]` and answers its default — nothing
  refuses, only the number is wrong. The roster is the `Limits` field, the `DIRECTIVES` row, the
  `unit_of` arm, and the reader; nothing fails to compile without the third.
  [until: gone crates/nvs-config/src/value.rs:fn unit_of]
- **`rule:errors/on-limit`'s roster of resource limits is restated in four places, and three of them
  are prose no test reads.** The ADR's own list (`docs/decisions/0020.md` § 1),
  `nvs_runtime::Limit`'s enum doc and `Core\Fatal::onLimit`'s reference card `short` in
  `crates/nvs-stdlib/src/fatal.rs` each enumerate them, and only the enum's `name()` arm is
  load-bearing, so nothing fails when the other three drift. `grep -rn "cpu_time" docs/decisions
  crates/nvs-stdlib/src crates/nvs-runtime/src` finds all of them in one call.
  [until: reviewed 2026-09-06]
- **`unsafe_code` is `forbid` at the workspace root, so the first `unsafe` in a crate is a manifest
  edit before it is a code edit.** `-F unsafe-code` cannot be turned off by any attribute —
  `#[expect(unsafe_code)]` still fails with *usage of an unsafe block*, and the note names the
  command line rather than the lint table. Replace that crate's `[lints] workspace = true` with
  `[lints.rust]` + `[lints.clippy]` copied verbatim from `Cargo.toml`'s `[workspace.lints.*]` (both
  tables, or the crate silently loses every clippy lint), set `unsafe_code = "deny"`, and say in a
  comment which call needs it — `crates/nvs-config/Cargo.toml` is the shape.
  [until: gone Cargo.toml:unsafe_code = "forbid"]
- **A source path is read off the filesystem in more places than the front end, and the
  configuration snapshot is the one that bites.** A bundled executable resolves its entry and
  `require` graph out of an appended payload, so the path handed to `nvs run` is synthetic — and
  `nvs_config::Snapshot::build` canonicalizes that same path through `trust::canonical` to key
  `rule:config/an-application-is-its-entry-file-path`'s `[[app]]` blocks, refusing the run with
  `E0605` after the program compiled cleanly (`run_run` now substitutes the executable's own path,
  since a bundle is one trust domain). Before changing a byte source, `grep -n
  'canonicalize\|read_to_string' crates/` for the other readers; only one of them is in `nvs-hir`.
  [until: reviewed 2026-09-06]
- **Making an operator throw is four edits, not one, and the codegen guard is the last of them.** A
  guard added to `emit_binop` alone dies at run time with `internal error: an arithmetic throw with
  no error edge`, because `nvs-ir`'s `lower/operator.rs` `fallible` match decides whether the
  instruction has an `Inst::on_error` edge to leave on. The roster is the `fallible` arm, the
  codegen guard, the same rule in `nvs_runtime::helpers` for the tagged path a `mixed` operand
  takes, and every `.nvst` that used the old answer as an *instrument* — the math cases that read
  the sign of a zero with `1.0 / $z` had to move to `Core\Math::fdiv` — so grep for the shape (`1.0
  /`, `/ 0.0`), not the feature. [until: reviewed 2026-09-06]
- **Refusing a construct that already parses breaks the tests that used it as a fixture, not as a
  subject.** `<>` was a row in `operators_longest_match_wins`, and `new class { … }` was the body
  two `casing.rs` fixtures used to prove the casing pass descends into a nested declaration; all
  three called a helper asserting a clean parse, so they failed with the new code rather than with
  anything about the shape they test. Before writing a refusal, grep the crate for the spelling;
  move the fixture to the helper that collects both halves (`casing.rs`'s `parse_and_check`) or hand
  the shape to the new test, never weaken the refusal. [until: reviewed 2026-09-06]
- **A new refusal is a hypothesis until the whole conformance tree has run it, and the tree is where
  the counterexample lives.** `E0787`'s first shape — refuse every write to a property with a `get`
  hook and no `set` — passed its own new case and was refuted by
  `lang/the-inout-marker-is-required-at-both-ends-and-replaces-every-ampersand.nvst`, whose class
  arms its own `get`-only property from its constructor, turning the rule scope-shaped rather than
  blanket. Write the refusal, run `./target/debug/nvs.exe test tests/conformance`, then write the
  case that pins it — a case written first only pins the rule you already believed.
  [until: reviewed 2026-09-06]
- **A qualifier written on an element of an array literal is gone before any call-site rule looks at
  it.** `check_array_literal` (`crates/nvs-types/src/expr/literals.rs`) joins nothing — with no
  expectation a literal infers `array<mixed>` — so `Core\Json::encode(["token" => $secret])`
  compiles while `Core\Json::encode($secret)` and a declared `array<secret string>` are both
  refused; that is `rule:security/secret-qualifier`'s unmodelled container axis, not a hole in the
  sink. Probe the container spelling with a scratch `.nvs` before writing the case that claims it,
  or the case pins a refusal that never fires. [until: reviewed 2026-09-06]
- **A diagnostic reported at a read fires a second time on a declaration another diagnostic already
  refused, and the cases it turns red look unrelated.** `E0792` — a class constant whose value folds
  to nothing — is reported at the read on purpose, and then reported over the top of `E0246` (`const
  LIMIT = 9;` has no type, so it folds to nothing *because* it was refused) and `E0727` (`const
  secret bytes BLOB = "b";` cannot fold because there is no `bytes` literal). Before reporting on a
  use of a declaration, ask which declared types can never have reached that point cleanly and
  return early for each with the other diagnostic named. [until: reviewed 2026-09-06]
- **A hand-built forwarding call must forward the whole calling convention, and the two parameter
  shapes that do not survive abort inside `nvs-runtime` rather than reporting.**
  `rule:types/callable-is-a-closure`'s `(...)` thunk passes its own parameters straight through,
  which is wrong for exactly two: an `inout $x` wants an address where the thunk has an `int`, and a
  variadic tail wants the collected array where it has the first element — the variadic one dies as
  `misaligned pointer dereference` at `nvs-runtime/src/array.rs` with nothing pointing back at the
  callable. Write one case per *declaration* shape the callee can have, not per call site, reading
  `nvs_types::signatures::MethodSig`'s field list for what those shapes are.
  [until: reviewed 2026-09-06]
- **Hand-built IR that needs a `Ty::Tagged` operand emits the bare constant and then
  `InstKind::Tag`; the `Ty` on the instruction is not a cast.** `low.emit(b, Ty::Tagged,
  InstKind::ConstNull)` compiles and lowers, then aborts with `internal error: cranelift rejected
  the code generated for `C::current`: should be implemented in ISLE: inst = `v25, v26 = isplit.i64
  v46`` — the `isplit` is the 128-bit pair being taken apart, and nothing in the message names the
  lowering that caused it. `convert`'s `(_, Ty::Tagged)` arm produces the `Tag` for every
  source-level widening, so a synthesized body is the only place it is written by hand; every other
  `InstKind::ConstNull` under `lower/` is `Ty::Null` for this reason. [until: reviewed 2026-09-06]
- **A crate's own `# Known gaps` list can be stale about that crate's body, and the body is the
  rule.** `nvs_types::layout`'s doc said a promoted constructor parameter claimed no slot while
  `own_properties` had been giving one an ordinary slot for some time, and `nvs_types::defaults`'s
  said `= null` stayed refused for want of a representation that had landed underneath the paragraph
  — a `Known gaps` bullet is status, an ADR is a decision, and status goes stale silently. One
  scratch `.nvs` under `.agent-tmp/`, or one `grep` for the shape the paragraph calls impossible, is
  the whole check. [until: reviewed 2026-09-06]
- **Rewording a `Core` reference card breaks a golden in another crate, and `-p nvs-stdlib` never
  shows it.** `crates/nvs-cli/tests/meta.rs`'s `the_golden_for_str_length_matches_the_contract` pins
  `Core\Str::length`'s whole card — the one card `nvs meta --json`'s golden froze — so a `str.rs`
  edit surfaces as a `nvs-cli` failure with no hint of what moved. `grep -rn "<the card's first
  clause>" crates/nvs-cli/tests/` before rewording.
  [until: gone crates/nvs-cli/tests/meta.rs:the_golden_for_str_length_matches_the_contract]
- **A new expression level between assignment and the ternary is not one edit — the ternary's `else`
  branch parses at the assignment level and will swallow it.** `rule:expressions/catch-expression`'s
  `catch` slotted into `parse_assignment_inner` in one line, and `f() catch (A) => $y ?: 1 catch (B)
  => 2` still came out with one arm because `parse_ternary`'s else called `parse_assignment`, which
  re-entered `parse_catch`; `parse_ternary_else` is the same body over `parse_ternary`, for the else
  branch only, since the `then` branch is delimited by its own `:`. Any future level added above the
  ternary owes the same check, and a unit test in `crates/nvs-syntax/src/parser/tests/expr.rs` is
  what catches it, not a `.nvst`. [until: reviewed 2026-09-06]
- **An ADR can specify a diagnostic code that is already taken, and a warning belongs in `W1xxx`
  whatever the ADR says.** `rule:expressions/bare-throwable-arm-warns` said its unbound-arm advisory
  was `E0778`, which has been `E_INSTANCE_METHOD_CALLED_STATICALLY` since well before it; the
  `W1xxx` band sits at the foot of `crates/nvs-diagnostics/src/lib.rs`. The number an ADR states is
  a claim to check against that file, not a fact to copy; when it is wrong, fold the ADR body in the
  same commit and move everything naming the old code (`loop-goal.md`, the `[[check]]` test name in
  `loop-goal.toml`) with it. [until: reviewed 2026-09-06]
- **A type atom whose first token is already a statement keyword needs the *statement* arm guarded
  too, not just `can_start_type`.** `parse_statement`'s dispatch matches
  `TokenKind::Keyword(Keyword::Abstract | Keyword::Final | Keyword::Class)`
  (`crates/nvs-syntax/src/parser/stmt.rs`) above the `_ if self.can_start_type()` arm, so
  `class<Animal> $cls = …;` parsed as a class declaration and produced a run of E0101s about a
  missing class name even with `Parser::at_class_reference` in `can_start_type`. Every keyword arm
  earlier in that match is a second door the new atom has to be let through; the conversion slot
  (`$x as class<Animal>`) passes with no such edit, so a test that only exercises `as` reports green
  on half a feature. [until: reviewed 2026-09-06]
- **Adding a `Ty` variant to `nvs-types` breaks exactly one match, and it is not one you would
  guess.** Nearly every `match` over `Ty` in that crate has a `_` arm, so `Ty::ClassRef` compiled
  everywhere except `expr/operators.rs`'s `equality_domain` —
  `rule:expressions/disjoint-comparison-refused`'s domain partition, exhaustive on purpose so a new
  type cannot silently become comparable to everything. That is the one place a new variant owes a
  decision rather than an arm (a new `EqDomain` variant costs the enum, `equality_domain`, and
  `reject_unordered_operand`'s match); decide it in the ADR that adds the type, not at the compiler
  error. [until: gone crates/nvs-types/src/expr/operators.rs:fn equality_domain]
- **A representation that can hold `null` has six lowering sites, not the four a `grep` for
  `Ty::Tagged` finds — and `==` against a written `null` is the one that looks covered and is not.**
  Five are where they look (`lower_coalesce`, `lower_isset_operand`, `truthy_convert`, `coerce`,
  `lower_binary`); the sixth is `lower_expr`'s own dispatch arm, which routes `$x == null` to
  `lower_null_identity` before `lower_binary` ever sees it whenever exactly one side is the literal,
  so a row added to `lower_binary` is dead for the spelling every test writes and a null value
  compares unequal to `null` while `isset` answers correctly one line above. When a construct has a
  fast path keyed on one operand being a literal, the fast path is a separate site a grep for the
  operator's own lowering will not find. [until: reviewed 2026-09-06]
- **A `Core` member that answers `Iterable<T>` owes a named class, and `CoreTy::Iterated` is not
  it.** That variant is "parameter position only", so a member returning a sequence answers a
  `CoreTy::Instance` of a class carrying the list, plus a `registry::ITERABLES` row for its element
  type and an `iterate()` on `instance::DISPATCH_ROSTER` (held in step by
  `an_iterable_class_answers_the_iteration_protocol`). Such a class has one slot and no member of
  its own, which fails `a_class_with_slots_has_instance_members_and_the_reverse` until it is added
  to that test's `HANDLES` list — the failure names the class and not the rule; `Core\IO\Lines` is
  the worked example and `crate::cursor::over` does the rest. [until: reviewed 2026-09-06]
- **A `Core` member cannot return a shape, and a registry row carries no nested `Qual` — two facts
  the `CoreTy` enum states only by omission.** There is no `CoreTy::Shape`:
  `crate::instance::SHAPE_ROSTER` is for values the engine builds (`Core\Issue`,
  `Core\Script\Result`), so a member whose spec answer is a record answers a `CoreTy::Instance`
  class with zero-argument members, as `Core\Regex\Match` and `Core\Process\Result` do. And
  `nvs_types::core_lib::qual_of` reads `Text`/`Blob` at the top level and inside a `Variadic` only,
  so `Array(&CoreTy::Text(Qual::Sink))` marks nothing — what refuses a tainted element is the
  ordinary argument check, because `array<tainted string>` is not `array<string>`.
  [until: reviewed 2026-09-06]
- **A `Core` member may not reach the operating system, and the gate that says so is a list of
  spellings rather than of effects.** `nvs_stdlib_reaches_the_os_only_through_the_gate`
  (`crates/nvs-stdlib/tests/capability.rs`) forbids literals such as `std::fs`,
  `std::process::Command` and `std::env::var` in this crate's `src/`, so a member reading an
  environment variable fails while `std::io::stdin().is_terminal()` or a raw `libc::ioctl` beside it
  passes unremarked. Neither is a judgement about capabilities:
  `rule:security/capability-check-at-the-door` puts the reaching in `nvs-runtime`
  (`nvs_runtime::terminal`, a module beside `capability` because a door that asks no `Cap` is not a
  door), never a `registry::CAPABILITIES` row — decide the crate before the first line, since moving
  it afterwards is a rewrite. [until: gone crates/nvs-stdlib/tests/capability.rs:const FORBIDDEN]
- **A `Core` class may not declare a slot before the member that reads it exists, and the test that
  says so names neither.** `a_class_with_slots_has_instance_members_and_the_reverse` holds slots and
  instance members to the same emptiness, with a `HANDLES` list for the classes whose slots
  something *else* reads (`Core\Regex\Pattern`, `Core\Script\Handle`, `Core\IO\Lines`, …). A class
  registered ahead of its readers declares `slots: &[]` and gains them in the same slice as the
  members, rather than joining `HANDLES` — that list is for state read from outside the class, not
  for state nothing reads yet. [until: gone crates/nvs-stdlib/src/registry.rs:const HANDLES]
- **A `System` block is still readable by a request — it is `Core\Config::set` that refuses one, not
  `get`.** `rule:observability/metrics-and-trace-blocks-are-system` makes the whole `[trace]` block
  `System`, which reads like "a request cannot see it" and is not: `nvs_config::Request::get`
  answers off the snapshot's own table for any dotted key, and only `set` consults the directive's
  class and returns `false` for `Class::System`. `ctx.config().and_then(|c|
  c.get("trace.propagate"))` is the whole read, the shape `http.rs`'s `bound_of` and `redirects_of`
  already use; what `System` buys is that a request cannot *change* it. [until: reviewed 2026-09-06]
- **One `CAPABILITIES` row makes the whole class capability-bearing, and every sibling member then
  owes a row of its own.** Adding `(Core\Cache, "shared", NetConnect)` is one line, and it turns
  `every_capability_bearing_member_declares_its_capability`
  (`crates/nvs-stdlib/tests/capability.rs`) red on `Core\Cache::local`, a member that needs no
  grant; the allowlist that once took such members is gone rather than frozen, so a member that
  reaches nothing declares `None` in `registry::CAPABILITIES` beside a comment saying what it
  reaches instead. The class holding the *operations* is untouched, because a door is one class and
  the thing behind it is another (`Core\Cache\Store` declares nothing behind `Core\Cache::shared`;
  `Core\RateLimit\Decision` is its own `CoreClass`).
  [until: gone crates/nvs-stdlib/tests/capability.rs:fn every_capability_bearing_member_declares_its_capability]
- **`every_error_path_is_asserted_or_declared_unreachable` stops its upward scan at the first
  `Fault::`, including one inside the declaration itself.** A comment saying "it is a `Fault::fatal`
  rather than a throw" is read as its own site's boundary, and `DECLARATION_WINDOW` is 8 lines from
  the phrase to the `Fault::`, which `cargo fmt` can push past by breaking a method chain. Name the
  mechanism without the prefix ("fatal rather than thrown") and put the comment directly over a bare
  `Err(Fault::fatal(…))` rather than over a closure chain.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_error_path_is_asserted]
- **A new Core member owes three conformance cases, not the one the five-edit recipe names.**
  `conformance_coverage.rs`'s `every_core_class_has_a_conformance_floor_of_three` counts cases per
  member and fails with "asked by 1 case(s)", and a second case asking the same question does not
  count. Spend the three on the conventions' depth shapes while the member is fresh; one file can
  ask for two members landed together.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_core_class_has_a_conformance]
- **A `match` over `nvs_syntax::ast` written outside that crate cannot be exhaustive, and the
  compiler will not say so.** `ExprKind`, `StmtKind`, `ClassMemberKind`, `NewTarget` and
  `DestructureElement` are `#[non_exhaustive]`, so a walk in another crate needs a wildcard arm and
  silently treats every later production as a leaf. Put any per-variant table over the AST in
  `nvs-syntax` itself, as `crates/nvs-syntax/src/walk.rs` does, where a new variant is a build
  error. [until: gone crates/nvs-syntax/src/ast.rs:#[non_exhaustive]
- **A module's "known gap" can be wrong about the tree, and one stale field comment is how it gets
  that way.** A gap saying a mechanism is missing, against a rule saying it exists, can rest on a
  single doc line that stopped being true when the mechanism landed, and the gap then travels from
  handoff to handoff. When a doc and a rule disagree about what exists, `grep -n` the enum, or probe
  a four-line scratch file with `target/debug/nvs.exe check`, before believing either.
  [until: reviewed 2026-09-06]
- **A spec option spelled with a keyword can be refused by the parser rather than by any rule, and
  the diagnostics never say "keyword".** `default`, `match`, `class` and `for` are all plausible
  option names, and `{default: "ada"}` produced `E0101 expected a field name` and a cascade of
  recoveries, none naming the cause. Before renaming an option away from what its rule spells, check
  `crates/nvs-syntax/src/token.rs`'s keyword table;
  `nvs_syntax::parser::expr::parse_object_literal_fields` is where an object-literal field was
  widened to take one. [until: reviewed 2026-09-06]
- **`options` is a reserved parameter name, and a row using it positionally fails a registry gate
  whose message reads as a tautology.** `registry::OPTIONS_NAME` is the trailing bag's name under
  `rule:core-api/shape-rules` R2, so `every_registry_row_names_one_parameter_per_positional_slot`
  reports "gives a positional parameter the trailing bag's own name" with `left: "options"` /
  `right: "options"`. Rename the positional parameter (`select`'s list is `$choices`) and fix the
  rule's own table in the same slice.
  [until: gone crates/nvs-stdlib/src/registry.rs:every_registry_row_names_one_parameter]
- **A rule added to `Core\Cli`'s shared prompt path reaches three of the five prompts, not five.**
  `ask`, `confirm` and `secret` decide nothing before `ask_terminal`, but `select` and `multiSelect`
  ask `watched(ctx)` first and return early, which a grep for `ask_terminal` counting five call
  sites cannot see. Put anything that must reach every prompt in a predicate beside `watched` —
  `answerable` is the shape — that the early-deciding members read too. [until: reviewed 2026-09-06]
- **A rule the compiler enforces at the call site has no enforcement on the erased path, and nothing
  in the tree says so.** `rule:classes/property-observer-pipeline`'s `onPropertySet` is emitted
  beside every `FieldSet` from the declaration, but a write through a `mixed` receiver has none, so
  `nvs_object_slot_set` stored the slot and told nobody. Ask of any compile-time-answered rule what
  `SlotGet`/`SlotSet`, `call_erased_method` and `value_to_string`'s `Tag::Object` arm do;
  `nvs_runtime::write_erased_property` holds the write half and `nvs_object_slot_get` still has the
  gap. [until: reviewed 2026-09-06]
- **A new `TypeAtom` compiles the whole workspace green and lowers to `mixed`, silently.**
  `nvs_types::lower`'s atom match ends in `_ => env.interner.mixed()`, so the type parses, every
  declaration slot accepts it and every value in it is unchecked — a loosening no test asks about.
  Add the `nvs_types::ty::Ty` variant in the same slice; its exhaustive matches then name every arm
  to write. [until: gone crates/nvs-types/src/lower.rs:_ => env.interner.mixed(),]
- **A rule reading `env.signatures` from `nvs_types::lower` fires on every annotation: that pass
  runs twice and the first run has no table.** `signatures::collect_members` lowers every property
  type while the table is still an empty placeholder, so a roster check in `lower_property_key` sees
  every roster empty; only `env.symbols` is complete before both passes. Site such a rule where the
  table is real — `check.rs` re-lowers a property annotation, but a method parameter's annotation is
  lowered once, during collection. [until: reviewed 2026-09-06]
- **A slice the plan files under `nvs-ir` can be unbuildable there, because that crate holds no
  class table.** `Lowering` carries `exprs`, `checked_types` and an `EnumTable` and nothing else
  about a declaration, so a set the checker derived from the hierarchy cannot be re-derived below
  the erasure. Record the set in the checker as `ExprInfo::EnumCase`/`ExprInfo::PropertyKey` do and
  read it back in `nvs-ir`, keyed by the annotation's span when the lowering holds the `Type` node
  and not the `Expr`. [until: reviewed 2026-09-06]
- **A `Fault::fatal` fails the error-path gate unless the comment above it holds the literal words
  *unreachable from source*.** `conformance_coverage.rs`'s gate greps the eight lines above the site
  for that phrase, not for a reason, so a comment arguing the case in its own words fails as though
  the member owed a `.nvst` case for an unreachable path. Use the phrase; `Core\Str::length`'s
  `u64::try_from` arm carries the wording for a site no diagnostic refuses.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_error_path_is_asserted]
- **A `Read`/`Write` stream that may be upgraded is an enum taken by value, not a `&mut` swap.**
  `NvsTls::over` consumes its `NvsTcp`, so securing in place needs a placeholder variant nobody may
  observe; `nvs_stdlib::mail::Session::secure` takes `self` and returns a new one instead. Any
  in-band upgrade owes the second half too: refuse, rather than clear, a non-empty read buffer,
  since bytes held from before the handshake replayed after it is the *NO STARTTLS* injection class.
  [until: reviewed 2026-09-06]
- **A `Value` that survives context teardown must be detached from the `Ctx`'s intrusive list, not
  just left alone.** A survivor still linked holds a `prev` naming the list head inside the freed
  context, so its last `unlink` writes into freed memory, and the symptom is a third party — a flaky
  test seeing NUL-overwritten bytes in a string that never allocated an object.
  `crates/nvs-runtime/src/object.rs`'s `Detach` guard is the fix; when a flake survives disabling
  the sweep body, suspect the list, and print the corrupted text rather than the status.
  [until: reviewed 2026-09-06]
- **A `Ctx` field owning a Novis reference must be released in `Drop::drop`, not left to its field
  drop; the failure is a refcount underflow.** Rust runs `Drop::drop` before dropping fields, so a
  field still holding a reference when the sweep at the end of that body runs is released twice, and
  the panic names `refcount.get() - 1` and no field. Join the `set_*(Value::null())` lines already
  in `Ctx::drop` for any new owning field. [until: reviewed 2026-09-06]
- **A debug assertion in `dismantle` may not say "the context releasing this object allocated it",
  and its panic aborts.** `nvs_host::isolate::finish` drops a child's `Thrown` while the parent is
  installed and a value can outlive its whole context, so the check is routinely false — and under
  `nvs_object_release`, which is `extern "C"`, a false positive is `thread caused non-unwinding
  panic. aborting`. Assert the structural invariant instead, linked on the list its stamp names, as
  `assert_linked_where_it_says` does. [until: reviewed 2026-09-06]
- **A `Core` row can name a class `registry::CLASSES` does not hold, and only
  `every_instance_type_names_a_registered_class` stands in the way.** `Throwable` lives in
  `nvs_hir::errors::TREE` and is seeded by `nvs_types::error_lib`, so
  `CoreTy::Instance("Throwable")` resolves like any class; that test reads `CLASSES` alone and its
  `EXCEPTION_TREE` list is the second roster. A rule stated over `registry.rs`'s rows is not the
  whole rule wherever another crate answers for a class.
  [until: gone crates/nvs-stdlib/src/registry.rs:EXCEPTION_TREE]
- **`nvs_runtime::capability` has two path resolvers that answer different spellings of one file —
  on Windows a `\\?\C:\…` against a plain `C:\…`.** `capability::canonicalize` goes through
  `nvs_config::capability::resolved`, which pins a not-yet-existing path's deepest existing ancestor
  for `Core\IO::within`, while `std::fs::canonicalize` returns the verbatim form. A door that
  resolves goes through `nvs_config::capability::resolved` and asks any other question, existence
  included, separately. [until: reviewed 2026-09-06]
- **`nvs-stdlib` may not write the string `std::fs` on any non-comment line, a type name included.**
  `nvs_stdlib_reaches_the_os_only_through_the_gate` is a literal substring scan with no allowlist,
  so `&std::fs::Metadata` or `std::fs::TryLockError::WouldBlock` is reported as "reaches the
  operating system directly" about a line that reaches nothing. Spell the type
  `nvs_runtime::capability::Metadata`, convert a `TryLockError` through `std::io::Error`'s
  `ErrorKind::WouldBlock`, and keep `std::fs` to `//` lines.
  [until: gone crates/nvs-stdlib/tests/capability.rs:nvs_stdlib_reaches_the_os_only_through]
- **A landed member's reference card can describe an unlanded sibling, and that sentence is a claim
  rather than a note.** `Core\IO::list`'s card called `walk` "the streaming half" — the
  `list`/`walk` pair `rule:core-api/shape-rules` R6 forbids — written while `walk` was only a name
  in the spec. Before registering a member, grep the other cards and module docs for its name; the
  spec row and the rule outrank the prediction, and the stale sentence is fixed in the same slice.
  [until: reviewed 2026-09-06]
- **`ChannelBinding::unrequested()` cannot authenticate against a TLS-enabled PostgreSQL, and it is
  the spelling that reads as correct.** `unrequested` is the gs2 header `y,,`, which asserts the
  server offers no channel binding, and a server offering `SCRAM-SHA-256-PLUS` — every SSL
  connection — reads it as a downgrade and fails with "channel binding check failed". Use
  `unsupported` (`n,,`); `crates/nvs-db/src/pg.rs`'s module doc holds the reasoning.
  [until: reviewed 2026-09-06]
- **Code written through `splice.py` is never rustfmt-shaped, and `verify.py` runs `fmt` after the
  build.** A slice that compiles and tests green still costs two full gate runs, because the
  formatting failure arrives only after build and tests have already run. Run `cargo fmt --all`
  before `verify.py`; and a `match` arm whose body is only `if cond { … }` is `collapsible_match`
  under `-D warnings`, so write it as a match guard. [until: reviewed 2026-09-06]
- **Registering a `Core` class or Part Two member is more than the five edits: two ledgers under
  `crates/nvs-stdlib/tests/` must shrink.** `spec-classes-part-two-outstanding.txt` holds a line per
  unregistered §§ 14-19 class and `spec-members-part-two-outstanding.txt` one per member, and
  `spec_registry_coverage.rs` fails a stale line as loudly as a missing one, only from a full `-p
  nvs-stdlib` run. `grep -n '<Class>'` both files before writing the row; a class's first member
  strikes its class line and the header sentence counting rows too.
  [until: gone crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt]
- **A `Core` class with slots and no instance members needs a line in `registry.rs`'s `HANDLES`, and
  the failure names your class.** `a_class_with_slots_has_instance_members_and_the_reverse` asserts
  the two rosters empty together, so an opaque handle (`Core\Db\InList`, `Core\Html\Markup`) reads
  as "you forgot the members". List it in the test's `HANDLES` const with a doc-comment clause
  saying who reads the slot, and make the class's name constant `pub(crate)` so the test can name
  it. [until: gone crates/nvs-stdlib/src/registry.rs:const HANDLES]
- **A `string`/`bytes` parameter or option written as `CoreTy::Str` fails the
  qualifier-classification gate, under `-p nvs-stdlib --lib` only.**
  `every_member_parameter_carries_a_qualifier_classification` wants
  `CoreTy::Text(Qual::…)`/`Blob(Qual::…)` or a place on its `UNCLASSIFIED` roster, options in a
  `{…?}` bag included, and names the member rather than the parameter. A lookup key is
  `Qual::Neutral`, not `Qual::Sink` — `Sink` means the content becomes an instruction;
  `Core\Regex\Match::group` is the precedent.
  [until: gone crates/nvs-stdlib/src/registry.rs:every_member_parameter_carries_a_qualifier]
- **The registry's guards are not all in `registry.rs`; `crates/nvs-stdlib/src/lib.rs`'s `mod tests`
  holds the roster-wide ones.** Every symbol resolving to an address, the symbol count matching rows
  plus row-less constructs, and symbol uniqueness live there, so a grep of `registry.rs` for how
  rows map to symbols finds only the constructor-versus-member check. Grep `lib.rs` too before
  designing around row-to-symbol mapping; `cargo test -p nvs-stdlib --lib` is what catches a
  collision. [until: reviewed 2026-09-06]
- **A checker table with no back-end reader looks landed from the front, and a member planned
  against it has nothing to read.** `nvs_types::derive`'s `#[Db\Derive]` roster and
  `ExprTypeTable::db_codec` were filled while `nvs_ir::ir::Class::codec` and
  `nvs_runtime::ClassDesc::codec()` still carried `#[Json\Derive]`'s list alone, so a derived class
  reached runtime with an empty codec. Grep the consumer of a table the checker fills before
  planning the member that spends it; `crates/nvs-types/src/derive.rs`'s known gaps say which half
  landed. [until: reviewed 2026-09-06]
- **Adding a class to `registry::GENERIC_CLASSES` makes its bare spelling a compile error
  everywhere, reported as `E0442` at the `.nvst` line.** `nvs_types::lower`'s `generic_params`
  consults that roster at every written occurrence, so landed cases that spelled `Core\Db\Rows`
  without a type argument fail with "takes 1 type argument(s), not 0". Before adding a row, grep the
  bare spelling across `tests/`, `examples/` and `docs/spec/` and budget an edit per hit; `Core` has
  no defaulted type parameter. [until: gone crates/nvs-stdlib/src/registry.rs:GENERIC_CLASSES]
- **A reference card may not cite a rule, and the doc comment two lines above it must.**
  `registry::tests::no_registry_card_cites_an_adr` refuses a citation in any `MethodDoc`
  `short`/`ret`/`desc`, `ParamDoc`, `ErrorDoc`, `CoreConst::desc` or `EnumDoc` case, because `nvs
  meta --json` ships the card verbatim to a reader with no rules tree, and it surfaces only at `-p
  nvs-stdlib --lib`. State the fact in the card and leave the citation in the `///` beside it.
  [until: gone crates/nvs-stdlib/src/registry.rs:no_registry_card_cites_an_adr]
- **`docs/novis.md` is generated from the built binary, so regenerating before rebuilding writes
  nothing and says "unchanged".** `python tools/reference.py --no-examples` reads `nvs meta --json`
  out of `target/debug/nvs.exe`, which is true of the binary and false of the tree after a source
  edit. The order is `python tools/verify.py` (which rebuilds), then regenerate, then `--check`.
  [until: reviewed 2026-09-06]
- **Giving a spec § 10 exception class a property of its own is six edits across four crates, and
  the two that are not `match` arms bite.** `nvs_hir::errors::OWN_PROPERTIES`, its `*_SLOT` and
  `error_lib::own_properties`'s type arm are loud; the quiet ones are
  `nvs_ir::lower::exception::synthesized_exception_constructors`, a hand-kept list nothing ties to
  the row, and the functions list of
  `a_file_with_no_class_still_carries_every_compiler_declared_class`. Land all in one change;
  `INSTA_UPDATE=always python tools/verify.py -p nvs-ir` rewrites the snapshots that go red.
  [until: reviewed 2026-09-06]
- **A `catch` binding in a closure that shadows the enclosing frame's name is lowered as a capture,
  and the compiler panics.** The panic in `crates/nvs-ir/src/lower/expr.rs` reads "the closure at …
  captures `$full`, which is not bound in the enclosing frame" and points at nothing near the
  `catch`. Rename the closure's exception binding; a program hitting this panic is looking for a
  shadowed binding, not a capture it wrote.
  [until: gone crates/nvs-ir/src/lower/expr.rs:which is not bound]
- **`nvs_config::value`'s `Unit::Duration` reads one unit, not a compound, so `"1m30s"` is refused
  where `"90s"` and a bare `90` agree.** `rule:types/duration-literal`'s literal is `1h30m`, so the
  two spellings look like one feature, and the refusal is `is not a duration` against a value the
  language accepts. Write a config bound as a bare count of the base unit, as `nvs_config::db`'s own
  bounds do; nothing in a block's module widens `crates/nvs-config/src/value.rs`'s parser.
  [until: reviewed 2026-09-06]
- **A job whose program threw is `Ok` from `nvs_host::Isolate::run`; the judgement is
  `Completion::ok`.** `run(ctx)` answers `Result<Completion, GraphError>` and the `Err` half is only
  the argument graph refusing to cross, so a fixture that throws on purpose reads as success to an
  `if let Err(…)` and lands in the wrong bucket with no error anywhere. Read `Completion::ok`, which
  is already false for a throw and for a budget teardown alike. [until: reviewed 2026-09-06]
- **A module's known-gap bullet names the blocker its author hit, not every blocker between there
  and the feature.** A gap that reads as one missing type can hide two more layers behind it, so
  reading it as a work estimate sizes a slice at a third of its cost. Spend three greps, one per
  layer — is the member callable, can the pass name the thing, does the pass have the input — before
  budgeting; `crates/nvs-types/src/intrinsics.rs`'s gap list is where the layered answer for the
  literal-host check lives. [until: reviewed 2026-09-06]
- **A `PgRows` holds its `&mut Ctx` borrow to the end of the scope because it has a `Drop`, and
  `E0499` names neither.** NLL ends a borrow at its last use only for a type with no destructor, so
  a second `ctx` call after draining a row stream fails with "first borrow might be used here, when
  `answered` is dropped". Write an explicit `drop(answered)` before touching `ctx` again; the same
  holds for anything handed out of `&mut Ctx` that releases on drop, a held connection included.
  [until: gone crates/nvs-db/src/pg.rs:Drop for PgRows]
- **A defaulted type parameter on a shared type breaks every unqualified associated path on it,
  three files away.** `StatementCache<H = String>` compiles, then
  `StatementCache::capacity_for(block)` fails with `E0283: cannot infer type of the type parameter
  H`, because a default is not inference fallback and rustc unifies `H` only from a concrete
  inherent impl. Move the items that never mention `H` out of the impl into free functions rather
  than reaching for a turbofish; a 20-line `rustc --crate-type lib` probe under `.agent-tmp/`
  answers this in one call. [until: reviewed 2026-09-06]
- **`nvs_db::encode` is PostgreSQL's text input format, not a driver-neutral rendering, and another
  driver reusing it gets quietly wrong rows.** It looks neutral — a `Value` in, `Option<Vec<u8>>`
  out, both drivers binding `&[Option<&[u8]>]` — but `bool` renders `t`/`f`, `bytes` as `bytea` hex
  and non-finite floats as `Infinity`/`NaN`, none of which errors on MySQL. Use the driver's own
  encoder (`nvs_db::mysql::encode`, `nvs_db::tds::encode`) and its `Dialect`; a second driver's
  `query` is the dialect and the encoder, not one branch in the drain. [until: reviewed 2026-09-06]
- **A helper landed ahead of its caller fails `verify.py` on `dead_code`; `#[expect(dead_code,
  reason = "…")]` is the marker that removes itself.** `cargo clippy --all-targets -- -D warnings`
  stops the build on an uncalled private function; `#[allow]` goes stale silently while `#[expect]`
  fails the day the function is used. Do not pair it with a `#[cfg(test)]` test of the same function
  — the test build makes the expectation unfulfilled, itself a warning — and spell it
  `#[cfg_attr(not(test), expect(dead_code, reason = "…"))]` when the tests are the only reader.
  [until: reviewed 2026-09-06]
- **Enabling a path leaves `#[expect(dead_code)]` on more functions than its doc lists, and clippy's
  `note:` names the one you already fixed.** The leftover sits on a helper of a helper, and
  `unfulfilled_lint_expectations` prints its reason, so the error appears to be about the function
  whose attribute was just removed. Run `grep -n 'expect(' <the module>` for every reason naming the
  path before the build. [until: reviewed 2026-09-06]
- **A driver's `io::ErrorKind` decides whether a refusal reaches Novis as a `Db\DbError` at all.**
  `nvs_stdlib::db`'s `statement_failure` builds the `rule:core-classes/db-error` error from its
  `ErrorKind::Other` arm alone and answers an `IOError` for every other kind, so a driver wording a
  server `ERR` packet as `PermissionDenied` strips `kind`, `sqlState` and `driverCode` with every
  driver unit test green. A server-worded refusal is `io::Error::other(ServerError { … })`;
  `ServerError::of` tells it from a wire failure, never the `ErrorKind`.
  [until: gone crates/nvs-stdlib/src/db/bind.rs:std::io::ErrorKind::Other =>]
- **Both type diagnostic bands are full, so a new type rule cannot have its own code.** `E04xx` and
  `E07xx` are full, `python tools/brief.py` prints both as `FULL` rather than a next number, and
  `E09xx` is internal compiler errors. Reuse an existing code whose message the rule can honestly
  rewrite, or write the ADR that decides the band layout — a third band changes
  `crates/nvs-diagnostics/src/lib.rs`'s legend and every tool grouping by band.
  [until: reviewed 2026-09-06]
- **A rule clause mandating a wire feature can be un-implementable for reasons only the sibling
  driver's code shows.** `COM_STMT_BULK_EXECUTE` for MariaDB's `executeMany` was settled against by
  `crates/nvs-db/src/pg.rs` flushing every `Bind`/`Execute`/`Sync` in one `wire.send` and by
  `mysql.rs`'s `Prepared` doc saying a prepare reports `0` columns for a data-dependent result set,
  neither of which the rule mentions. Read the sibling driver and the struct docs before budgeting a
  feature from the rule's text. [until: reviewed 2026-09-06]
- **A new `CoreTy` variant does not fail `cargo build`; it fails `cargo test -p nvs-stdlib` in
  `crates/nvs-stdlib/src/ast.rs`.** `CoreTy` is `#[non_exhaustive]`, so the out-of-crate matches in
  `nvs-cli/src/meta.rs` and `nvs-types/src/core_lib.rs` carry wildcards, while `ast.rs`'s `mentions`
  is exhaustive on purpose and lives under `#[cfg(test)]`. Add the `mentions` arm in the same edit
  as the variant, and do not trust a green `cargo check`.
  [until: gone crates/nvs-stdlib/src/ast.rs:fn mentions]
- **Adding a driver to `Core\Db` is one match per member, not one match, and `open`'s arm looks like
  all of it.** `nvs_db::Connection` is destructured at seven sites under `crates/nvs-stdlib/src/db/`
  and six end in `other => driverless(...)`, so an arm added to `open` alone hands out a connection
  that refuses every statement, with no compile error. Count the `Connection::` matches before
  pricing the slice; a driver sharing MySQL's protocol also needs a seam where
  `mysql_rows`/`mysql_write` take `&mut nvs_db::MySqlConn` by name. [until: reviewed 2026-09-06]
- **A refusal sentence that looks stale in a second module can be stale about a different thing;
  widening it to the first's roster is wrong.** `crate::db`'s `driverless` and `crate::queue`'s gate
  both said "only PostgreSQL runs a statement so far", but db's is about which drivers can send and
  the queue's about which dialect its own SQL is written in. Ask what the module would do with the
  widened case — "the same statements" means a copy, "statements it has not written" means two facts
  — and grep the sentence, not the roster. [until: reviewed 2026-09-06]
- **A `MySqlRows` column description cannot be named outside `nvs-db`, so no caller there can write
  a helper that takes one.** `MySqlRows::columns` answers `&[Column]` where `Column` is
  `mysql_common::packets::Column`, re-exported nowhere, so `fn read(column: &Column, …)` is
  unspellable in `nvs-cli` or `nvs-stdlib`. Decode inline in the row loop or behind a helper taking
  `nvs_db::MySqlScalar`, which is exported; match it by reference, and `to_string()` a
  `Text`/`Bytes` that outlives the iteration, since it borrows the row. [until: reviewed 2026-09-06]
- **A TLS handshake tunnelled in another protocol's frames buffers on `write`, frames on `flush`,
  and hands out its off-switch first.** `rustls` writes a flight in several calls and flushes after
  each (`ConnectionCommon::complete_io`), so framing each `write` cuts records at arbitrary offsets,
  and `StreamOwned` hands nothing back once `NvsTls::over` has the adapter, so the switch is shared
  before the handshake. `crates/nvs-db/src/tds/prelogin.rs`'s `Tunnel` keeps the flag in an
  `Rc<Cell<bool>>` and the caller takes its `TunnelEnd` before the handshake.
  [until: reviewed 2026-09-06]
- **A TDS `DONE`'s type byte decides whether the answer ended; its `DONE_MORE` status bit does
  not.** `DONEINPROC` (`0xFF`) ends one statement inside a procedure and is never the last token —
  `sp_prepexec` sends the `RETURNVALUE` after it — so a reader keyed on `more()` ends early and then
  refuses with "carried N byte(s) after the DONE that ended it". Read `crate::tds::Done`'s `in_proc`
  field beside the status bits, and test against a real RPC transcript rather than one ending in a
  plain `DONE`. [until: gone crates/nvs-db/src/tds/token.rs:pub in_proc: bool]
- **A `pub(crate)` predicate one module borrows as a proxy for another's roster is a coupling no
  signature shows.** `queue.rs`'s tests asked `nvs_stdlib::db::rendering_for(driver).is_some()` as
  "the queue can send over this one", true only while `Core\Db`'s drivers and the queue schema's
  dialects were the same list, so giving SQL Server an encoder failed two queue tests naming the
  queue. Before widening what a predicate answers `Some` for, `grep -rn '<fn>' crates/<crate>/src`
  and read the doc comment on every hit. [until: reviewed 2026-09-06]
- **A pass that rewrites `nvs_config`'s typed tree has not changed what a request sees; the half
  that reaches the runtime is the merged table.** `Snapshot::retype` deserializes `Config` from
  `Snapshot::table` afresh, so in-place edits to `resolved.config` survive only for passes that run
  before the snapshot is built, and `app::canonicalize` is a misleading model because
  `Snapshot::build` reads `resolved.config.app` directly. Run `nvs config dump` in a scratch
  directory: a value still spelled as the operator typed it says the rewrite did not land.
  [until: gone crates/nvs-config/src/snapshot.rs:fn retype]
- **Three values an operator writes into a `[db]` block's `path` are not paths, and a resolver
  treating them as one breaks SQLite.** `:memory:`, the empty string and any `file:` URI are
  SQLite's, and prefixing them with a directory names a file nothing opens;
  `nvs_config::db::is_relative_file` is the predicate. On Windows `/no-such/x` has `is_absolute() ==
  false` because it names no drive, so joining it moves it to the base's drive — use
  `Path::has_root`, which answers the same on both platforms. [until: reviewed 2026-09-06]
- **A cancelled task that is parked is usually torn down where it parked and never sees
  `Resumed::Cancelled`.** `Scheduler::run`'s sweep drops an unwindable parked task's coroutine
  outright, so the line after `suspend_current(Waiting::Parked)` runs only for a stack standing on a
  `nvs_runtime::HelperFrame`, which cannot be unwound and is resumed to die. Write both answers at
  every park site, and hold a `HelperFrame::enter()` guard across the park in a test asserting "the
  wait answered its cancellation". [until: reviewed 2026-09-06]
- **A task holds one timer entry, and a `Ready` poll on a stream's other interest lifts it,
  cancelling an idle read timeout.** `hyper` polls the readable half (`Pending`, deadline filed)
  then the writable half in the same pass (`Ready`, deadline lifted), so the connection parks with
  no clock, and the tell is a test hanging for exactly the client's patience.
  `nvs_host::NvsStream::timed` records which interest filed the entry; anything else parking two
  interests of one stream under one task inherits the question. [until: reviewed 2026-09-06]
- **`nvs_host::run_until_idle` returns while your task is still parked, and a server is the first
  caller for which that is wrong.** It breaks out as soon as one blocking poll wakes nothing — which
  a connection's socket reports on Windows once its owning task retired — so `spawn` + one
  `run_until_idle` serves one request and exits `0` silently. Loop while `RunReport::parked > 0`, as
  `crates/nvs-cli/src/serve.rs` does; the stale readiness is delivered once and does not spin.
  [until: reviewed 2026-09-06]
- **A new `Core` class in a private module declares `pub(crate) const CLASS`, not `pub`.**
  `conventions.md`'s worked example is `crates/nvs-stdlib/src/json.rs`, a `pub mod`, so copying its
  `pub const CLASS` into a private `mod` warns `unreachable pub item`, which the workspace lint
  policy makes a `verify.py` failure. Every private domain module (`env.rs`, `cap.rs`, `out.rs`)
  spells `CLASS` and `address` `pub(crate)`.
  [until: gone crates/nvs-stdlib/src/json.rs:pub const CLASS: CoreClass]
- **`CoreTy::Text(Qual::Contagious)` on a `void` member refuses the tainted argument the rule says
  it admits.** `nvs_types`' `admits_tainted_argument` (`crates/nvs-types/src/expr/quals.rs`) reads
  `Contagious` as "admits `tainted` only where the return type can carry the bit out", so a `void`
  row marked that way gives `E0401: expected string, found tainted string`. A rule's word for a body
  is not the enum's case for an answer: a writer that returns nothing is `Qual::Neutral`, as
  `Core\Cli::write`'s `string` arm is. [until: reviewed 2026-09-06]
- **A rule's compile error can be stated over a thing the compiler cannot see, and the corpus tells
  you before the build does.** "`echo` and a typed writer on the same response is a compile error"
  reads as a rule about a body, but `echo` is bound by context, so which sink a body writes to is a
  run-time fact except in a `#[Route]` handler. Before implementing a rule stated over a run-time
  noun, grep `tests/` for the members it names — a green case exercising the combination is the
  scope you actually have. [until: reviewed 2026-09-06]
- **A field added to `nvs_runtime::Ctx` can stop every coroutine from starting, and the panic names
  neither the field nor the crate.** `corosensei` refuses an entry closure over 1024 bytes
  (`allocate_obj_on_stack`, `type is too big to transfer`), and the breach surfaces as unrelated `-p
  nvs-stdlib` tests panicking inside a cargo registry path. `nvs_host::scheduler::start` boxes the
  context across and asserts `size_of_val(&entry) <= CORO_TRANSFER_LIMIT`; keep anything new out of
  the entry closure, and note `Finished` is over the limit and never crosses.
  [until: gone crates/nvs-host/src/scheduler.rs:CORO_TRANSFER_LIMIT]
- **A handler closure in `nvs-cli` takes its parameter type from its first statement, and the
  annotation cannot be spelled from `hyper`.** `move |request| { … }` compiles only because
  `table.select(&request, &OnDisk)` pins it to `Request<Incoming>`; a read placed above that line is
  `E0282: type annotations needed`, and `nvs-cli` has no `hyper` dependency on purpose
  (`rule:packaging/a-c-dependency-answers-two-questions`). Annotate the parameter with
  `nvs_server`'s re-exported `Request` and `Incoming` rather than reordering the body.
  [until: reviewed 2026-09-06]
- **Nothing inside `nvs_server::serve_connection`'s service closure may park on the request body;
  the deadlock is `hyper`'s shape.** The h1 dispatcher runs `poll_read` then `poll_write` on one
  task and the service future is what `poll_write` polls, so a `block_on` over the body suspends the
  coroutine that owes the next `poll_read`, for the smallest body too. The service is a future
  answering `Pending` while the isolate runs as a peer task, so a pull may park the isolate but
  never the connection's own task. [until: reviewed 2026-09-06]
- **A request that parks makes `hyper` skip its post-response read, and what breaks is the
  keep-alive clock.** `Conn::maybe_notify` returns early under `is_read_blocked()`, which any
  earlier `Pending` from the IO adapter sets, so `crate::io`'s "a read in the `Write` phase ends the
  response" never fires and the connection parks under the write wait until the client's FIN. Call
  `cx.waker().wake_by_ref()` from the service once the answer exists, and debug with timestamps —
  the trace reads identically without them. [until: reviewed 2026-09-06]
- **A `Core` instance has no property a program can reach, so a rule's `$x->thing` example is a
  surface the registry cannot express.** `CoreClass` has `methods`, `instance`, `slots` and
  `constants` and no field roster, a fact stated only in `CoreTy::Instance`'s doc and in comments on
  other classes. Check that doc before believing a spec'd property spelling; the fix is a reader
  with parentheses, folded back into the rule and the spec line in the same commit.
  [until: reviewed 2026-09-06]
- **Two rules can describe one wire event, and the one you are handed may state the answer while the
  other states the precondition.** `rule:http-server/cors-is-closed-until-origins-are-named` says
  what a preflight is answered with, but `rule:http-server/a-request-resolves-in-five-steps` defines
  one as `OPTIONS` carrying `Origin` and `Access-Control-Request-Method`, and `Cors::preflight` read
  only the second header — invisible while the policy was closed. Grep `docs/rules` for the noun
  before writing the predicate. [until: reviewed 2026-09-06]
- **A table the compiler builds and a running program must read does not cross by adding a
  dependency; it crosses as a second, runtime-side table.** `nvs-runtime` is the bottom of the crate
  tree, so `grep RouteTable` over `crates/` answering "compiler only" is a decision, argued in
  `nvs_runtime::commands`' module doc § *Why the table is a runtime value at all* and copied by
  `nvs-cli`'s `runtime_commands` as strings plus one closed enum. Routes, jobs or any later table
  take that shape; check for the sibling before designing the edge. [until: reviewed 2026-09-06]
- **Widening a member's declared type in `nvs_stdlib::registry` moves four expectations, none naming
  the constant you edited.** Adding to `router.rs`'s `CAPTURE` union changed a rendering pinned in
  `crates/nvs-types/src/core_lib.rs`'s tainted-answer roster, two `--EXPECTF-ERROR--` cases under
  `tests/conformance/core/`, and the generated `docs/novis.md`. One `grep -rn` for the old rendering
  across `crates docs tests` finds all four, and since the rendered union order is not the
  declaration order, `python tools/try.py <case>` prints the real one. [until: reviewed 2026-09-06]
- **A `Core` member answering `tainted`, or any member under `Core\Request`, owes a closed set in
  `nvs-types` that fails as a set diff.** `crates/nvs-types/src/core_lib.rs`'s
  `a_verified_signature_does_not_launder_its_claims` and
  `every_request_member_returning_outside_data_returns_it_tainted` (and the
  `reveal_and_the_password_helpers…` sibling) are ratchets, so the repair is the set, the count in
  the message and the doc comment's per-member reason together. `grep -n 'closed at'
  crates/nvs-types/src/core_lib.rs` finds them before the full verify does.
  [until: gone crates/nvs-types/src/core_lib.rs:closed at]
- **A request's per-request state is not on the context the door holds; the connection's `Ctx` and
  the isolate's are different objects.** The request is a root isolate whose `Ctx` `nvs-host` builds
  and drops inside `isolate::finish`, so a write-back filed at `crates/nvs-server/src/serve.rs`'s
  request end reaches the connection's context and never the record `Core\Session::start` opened.
  Whatever a request's end owes a context happens in `isolate::finish` and `nvs run`'s root task;
  where `nvs-host` cannot name `nvs-stdlib`, it travels as a `fn` pointer on the state itself.
  [until: reviewed 2026-09-06]
- **`jiff`'s disambiguation does not spell the spring-forward rule, and the nearest option is wrong
  by half an hour.** `AmbiguousZoned::compatible()` shifts a civil time inside a gap forward by the
  gap's length and `earlier()` shifts it back, while
  `rule:config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once` wants the transition itself, and
  a case asserting only the day passes against both. Read the missing minute with the gap's `after`
  offset and take `TimeZone::following(that).next()`, as `crates/nvs-config/src/schedule.rs`'s `at`
  does; only the fall-back half is a library call. [until: reviewed 2026-09-06]
- **A `spawn_child` whose parent task returns is torn down with it, and the test sees nothing rather
  than a failure.** `rule:concurrency/nothing-is-still-running-when-a-call-returns` is enforced by
  tearing children down, so an accept loop or ticker that spawns and returns leaves an empty log or
  a `ConnectionReset` at the peer, with no error server-side. Copy
  `crates/nvs-server/src/serve.rs`'s `Served` tail with the spawn: a tally incremented before it, a
  guard whose `Drop` decrements and wakes the parent, and `while outstanding > 0 {
  suspend_current(Waiting::Parked) }` before returning. [until: reviewed 2026-09-06]
- **`nvs_host::sleep` re-arms past a wake on purpose; waiting on a peer needs
  `nvs_host::timer::wait_until`, which is not re-exported.** `park_until` loops until its deadline
  because the caller asked for an instant, so a `parent.wake()` delivered to a sleeping loop is
  swallowed and the whole bound is paid every time, with nothing failing. `grep -n "pub fn"
  crates/nvs-host/src/timer.rs` is the list, and the same is true of every `pub mod` in that crate
  whose `pub use` line is a subset. [until: gone crates/nvs-host/src/timer.rs:pub fn wait_until]
- **A named-argument binding cannot be done at run time: neither `nvs_runtime::MethodRow` nor
  `nvs_types::ResolvedCall` carries parameter names.** Every existing site binds names to positions
  in `nvs_types::expr::calls` and lowers the result, so a feature binding a runtime map to a
  callee's parameters must record the names somewhere new and emit them from the lowering. And
  `nvs_runtime::abi::call` requires `arity` initialized values, so a call built with fewer is
  unsound rather than wrong. [until: reviewed 2026-09-06]
- **Adding a row-less `Core` symbol owes four edits, and the missed ones fail as a JIT panic or as
  two bare numbers.** `nvs_stdlib::symbols()` builds from `CLASSES`' rows plus hand-written
  `.chain`s, so a symbol reached by a construct compiles and type-checks clean and then dies in
  cranelift with `can't resolve symbol …`; and
  `every_registered_member_has_an_implementation_address` in `crates/nvs-stdlib/src/lib.rs` counts
  the row-less ones as a literal, failing as `left: N / right: N-1`. The roster is the `const`, the
  `address()` arm, the `.chain([…])` in `symbols()` and that literal.
  [until: gone crates/nvs-stdlib/src/lib.rs:every_registered_member_has_an_implementation_address]
- **A `Core` helper that returns `Err` must not release its transferred argument; the double release
  aborts the process.** `spawn script`'s `args:` is `ArgOwnership::Transferred` and the value is
  still on the temporaries stack when `emit_fallible` builds the fault edge, so the landing block
  already releases it on every `Err` — an added `release()` is `thread caused non-unwinding panic.
  aborting.` inside `nvs_value_release`. `nvs_ir::lower::TemporaryKind`'s doc says it: `Transferred`
  is released on the error edge only, and that edge is the caller's. [until: reviewed 2026-09-06]
- **`hyper`'s `with_upgrades()` cannot drive a connection this server accepts; the way back to the
  socket is `http1::Connection::into_parts`.** `with_upgrades` is bounded `I: Send + 'static` and
  `crate::io::ConnectionIo` holds an `Rc` and a core-registered socket, while `into_parts` is a
  plain `self` method giving back `io` and `read_buf` after the plain `Future for Connection` ends
  at a `101` without shutting the socket. Drive the connection through `Pin::new(&mut conn)` in a
  `poll_fn` rather than moving it into `block_on`; `Connection` is `Unpin`.
  [until: reviewed 2026-09-06]
- **A `CoreTy::Union` parameter carries no qualifier classification, so a text-like union is a
  `tainted`-refusing sink by accident.** `CoreTy::classification` answers `Some` only for
  `Text`/`Blob`/`SecretBlob`/`Entry`, and `admits_tainted_argument` maps `None` to `false` like a
  declared sink, so `Union(&[Text(Qual::Neutral), Blob(Qual::Neutral)])` passes every registry gate
  and refuses with `expected string|bytes, found tainted string`. A `Qual` lives on a leaf variant:
  write two classified parameters (`send`/`sendBytes`) rather than one union.
  [until: reviewed 2026-09-06]
- **A bare `[`Ctx`]` intra-doc link fails in a module that does not `use` the type, and `verify.py`
  is not what tells you.** Siblings write `[`Ctx`](crate::Ctx)`; the bare form is `error: unresolved
  link` under `-D rustdoc::broken-intra-doc-links`, as are a link to a `#[cfg(test)]` item and a
  redundant explicit target, both woken by making a `mod` public — and the doc gate is not part of
  `python tools/verify.py`, so a green session leaves it red. Run `python tools/verify.py --doc`
  once when a session writes a module doc that links across modules. [until: reviewed 2026-09-06]
- **A route table is on the unit, not on whichever context is in hand, and a `#[Test]` isolate's own
  context has none.** `nvs run` installs the table on the script's context (`main.rs`'s
  `set_routes`), but a `#[Test]` method runs in an isolate sharing only compiled code, so a match
  off `ctx.routes()` in a `Core` member answers `null` silently and looks like an empty table. Match
  on the side holding the compiled unit, and treat `ctx.routes()` in a helper that may run under
  `nvs test` as a bug. [until: reviewed 2026-09-06]
- **"Which declaration is this expression inside" is a stamp on the table, not a field on
  `nvs_types::Ctx`.** A `current_method` beside `current_class` costs an edit at every construction
  site of that struct (`grep -n 'Ctx {' crates/nvs-types/src/`) for a fact one table wants. Take a
  mark before the body in `check::check_method` (`ExprTypeTable::inline_snapshot_mark`) and name the
  owner of everything recorded after it at both exits; the shape generalises to any per-declaration
  fact an expression-level table wants. [until: reviewed 2026-09-06]
- **Adding a row to `Core\Db\Connection` fails a test in `db::transaction`, and the failure names
  neither the class nor the member.** `a_transaction_is_a_closure_and_transaction_is_a_queryable`
  sweeps every `CONNECTION.instance` row against `TRANSACTION`'s as debug-printed `Vec<String>`s, so
  a connection-only member arrives as a buried element diff. Add the name to
  `crates/nvs-stdlib/src/db/registry.rs`'s `BEYOND_QUERYABLE`, which the test checks from both ends;
  a "two rosters are identical" guard needs its exception set spelled as data the guard also checks.
  [until: gone crates/nvs-stdlib/src/db/registry.rs:BEYOND_QUERYABLE]
- **A `Core` member that answers a walk returns a registered memberless class, never
  `crate::cursor`'s unregistered one.** `every_instance_type_names_a_registered_class` requires a
  `CoreTy::Instance` return type to name a `CLASSES` row and `CoreTy::Iterated` is parameter
  position only, so a second internal class on `instance::INTERNAL_CLASSES` cannot compile. Copy
  `Core\IO\Lines`: a memberless `CLASSES` row, an `ITERABLES` row for the element type, a `HANDLES`
  entry, and `iterate`/`advance`/`current` on `instance`'s dispatch roster; `crate::cursor` is only
  for a snapshot already held. [until: reviewed 2026-09-06]
- **`instance::build` takes a reference over and `instance::slot` lends one, so parking a borrowed
  slot value into a new instance double-releases.** Every other `Core\Db` member only borrows the
  receiver's block name in passing, so none shows that a member keeping the value owes a `retain()`;
  the program exits 127 with nothing on stderr and its stdout byte-perfect. Grep for a
  `crate::instance::build` whose slot values are not all freshly constructed;
  `crates/nvs-stdlib/src/db/stream.rs` states the rule. [until: reviewed 2026-09-06]
- **`mixed as string` does not reach a `bytes`; the failure is a runtime throw inside the walk, not
  a diagnostic at the line.** `value_to_string` refuses `Tag::Bytes` as `rule:types/conversion`
  requires and `mixed` gives it no second chance, so `$value as string` compiles and throws `cannot
  convert a `bytes` value to `string`` on the first leaf. Write `$value as bytes as string` — narrow
  the `mixed` to the type it holds, then take the checked row; and `Core\Json::encode` refuses an
  array holding a `bytes`, so a member that starts answering octets invalidates every JSON-rendering
  fixture. [until: reviewed 2026-09-06]
- **A `Core` member whose return type changes is a handful of edits in its module and then a corpus
  pass, and the corpus pass is the bigger half.** Most failing cases want only an `as string` at
  each call site; the rest pin a *refusal* that moved one member along, so their `--EXPECT--`
  becomes `cannot convert `bytes` to `string`: not well-formed UTF-8 at byte N` and their titles
  must say so. Write the mechanical half as a script under `.agent-tmp/` handed to Python by path,
  never by `sed`. [until: reviewed 2026-09-06]
- **A `.nvst` case can carry its own `nvs.toml`, so the corpus already pins how a command behaves
  under a configuration — grep `--FILE nvs.toml--` before making one read the tree.** Every case
  spawns the real binary with the case directory as its working directory
  (`crates/nvs-test/src/run.rs`), so hoisting a catchable runtime refusal to compile time is a
  language change the corpus notices:
  `tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst` expects
  the runtime refusal its program catches. [until: reviewed 2026-09-06]
- **A `Core\Db` statement member is declared twice under one symbol, and changing its arity means
  changing both rows.** `nvs_stdlib::db::registry` gives `query`, `queryAs`, `execute`,
  `executeMany` and `stream` a row on `CONNECTION` and one on `TRANSACTION` reaching the same body,
  so editing one leaves a helper declared at two arities — a runtime panic in `nvs_helper!`'s `args:
  [N]` check, not a build error. `grep -c 'name: "<member>"' crates/nvs-stdlib/src/db/registry.rs`
  and expect `2`; a `splice.py` "block appears 2 times" refusal is the same tell.
  [until: reviewed 2026-09-06]
- **A crate's known-gap bullet can give a reason that is false about the driver it names.** A gap
  bullet naming *another* crate's mechanism is a claim about that crate as it was when the bullet
  was written — `crates/nvs-stdlib/src/db/mod.rs`'s `stream` `{chunk?}` gap argued from a portal
  walk that `nvs_db::pg`'s `open_portal` does not do. Grep the named mechanism before implementing
  the gap or restating its reason. [until: reviewed 2026-09-06]
- **`literal_default` does not fold a class constant, and every sibling pass's `folded_str` copies
  that hole forward.** `defaults::literal_default`'s `ClassConstAccess` arm lives in its callers, so
  a new compile-time string read built on it folds `Foo::WHY` to `None` and silently treats a named
  constant as computed. Use `defaults::fold_const_reference` (it needs a `&Ctx<'_>`), as
  `crates/nvs-types/src/reasons.rs`'s `folded_str` does. [until: reviewed 2026-09-06]
- **A `QName::to_string()` at the top of a per-call checker hook is an allocation per call site, and
  what catches it is `nvs-cli`'s 10k cold-compile guard failing inside cranelift.**
  `script::tests::ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once` fails as a
  wrong count or a `TryFromIntError(NegOverflow)` panic in `cranelift-jit`, passes alone, and passes
  with the change stashed — `git stash push -- crates/<touched>` is the whole bisect. Test the
  `&str` half of the roster row first and stringify only after it matches.
  [until: reviewed 2026-09-06]
- **A backend flag in `nvs-codegen` is pinned by a source grep, not by a symbol, so a refactor can
  unpin a memory-safety policy while every test stays green.**
  `crates/nvs-codegen/tests/backend_policy.rs` reads `crates/nvs-codegen/src/lib.rs` as text and
  asserts the literal `("enable_probestack", "true")`, so respelling the tuple or splitting the list
  per backend silently drops the stack-clash guarantee. Grep that test for the flag before touching
  the list. [until: gone crates/nvs-codegen/tests/backend_policy.rs:enable_probestack]
- **A value that joins a front-end answer to a configuration digest cannot be computed in the front
  end, and `Cargo.toml` says so before the anchor does.** `crates/nvs-hir` does not depend on
  `nvs-config`, so `Digest`, `EnvHash` and the program-id combine are unspellable there and the seam
  is the host, `crates/nvs-cli/src/main.rs`, which holds both halves. When an anchor names the crate
  holding one input, read the `Cargo.toml` of the crate holding the other before opening the file.
  [until: reviewed 2026-09-06]
- **SQLite reports no `notnull` for a primary-key column, so an introspected `INTEGER PRIMARY KEY`
  reads back nullable and no `Table` can be built from the row.** `pragma_table_info` sets `notnull`
  only where the text said `NOT NULL`; a rowid alias's is implicit. The fix belongs in the catalog
  statement — `crates/nvs-db/src/catalog.rs`'s `p."notnull" = 0 AND p.pk = 0` — and an introspected
  fixture is asserted against the `Schema` it produced, never against the catalog's own words.
  [until: gone crates/nvs-db/src/catalog.rs:notnull]
- **`nvs_db::ddl` writes no `CREATE TABLE IF NOT EXISTS`, so a hand-written DDL list cannot be
  swapped for a `Schema` value on its own.** The emitter is deliberately unguarded —
  `mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists` at
  `crates/nvs-db/src/ddl.rs:1210` asserts the absence — because a plan is computed from the current
  state and a guard has no portable spelling for an index anyway. So the command that ran the list
  has to become a convergence in the same slice, and any column the value adds has to be nullable to
  arrive as a `Safe` step on a table that already has rows. [until: reviewed 2026-09-07]
- **A `NaN` a `float` expression produced carries the hardware's sign bit, and that bit is not the
  same on every leg of the matrix.** `sqrt(-1.0)` is negative where the SSE default `NaN` is and
  positive on aarch64, so anything reading it — `f64::total_cmp` above all — answers one way on
  linux-x86_64 and the other on macos-aarch64 while each looks right on its own. Fold every `NaN` to
  one before ordering, as `nvs_stdlib::ordering::ordered` does, and never assert on a `NaN`'s sign.
  [until: reviewed 2026-09-07]
- **A numbered gap in another module's `//!` list is a fact with two homes, and closing one does not
  touch the file that cites it.** `crates/nvs-stdlib/src/queue.rs`'s gap 5 said what was left of it
  was "[`crate::db`]'s gap 2 and not this module's to close", while that list had since grown a send
  path for all five drivers — so the refusal an operator reads pointed at the wrong crate, and the
  two sentences were each locally plausible. Read the cited gap's own list before writing a sentence
  that defers to it; no test in either crate compares them. [until: reviewed 2026-09-07]
- **A new `nvs_types::ty::Ty` variant compiles after one arm, and the sites that must learn it are
  the ones the compiler never names.** `Ty` is `#[non_exhaustive]`, so every match outside the crate
  carries a `_` already and inside it only `equality_domain` is exhaustive: the isolate entry
  refusal, `check_expr`'s `wants_callable`, `generics`' two walks and
  `nvs_ir::lower::erase_checked_ty` each took their wildcard in silence. Grep the neighbour the new
  variant behaves like and decide every hit by hand before believing a green build.
  [until: reviewed 2026-09-07]
- **A closure parameter cannot be written without a type, and nothing in `nvs-types` is where that is
  decided.** `parse_param` (`crates/nvs-syntax/src/parser/expr.rs:1688`) is shared by every parameter
  list in the language and reports `E0101` for a missing type, so `fn ($u) => ...` never reaches the
  checker at all and a fixture written to test inference dies in `check_src`'s parse assertion rather
  than in the assertion it was written for. Relax it there, for a closure's list alone, before writing
  any checker test that omits a parameter type.
  [until: gone crates/nvs-syntax/src/parser/expr.rs:"expected a parameter type"]
- **Relaxing a parser refusal can open an ICE three crates away, because an `Option` in the AST
  doubles as "a diagnostic was already reported".** `Param::ty` was `None` only after `E0101`,
  so `nvs_ir::lower::closure` read it through a `panic!` no program could reach — until a
  closure literal was allowed to omit the type. Grep the *consumers* of the field a relaxation
  leaves empty for `unwrap`, `expect` and `panic!`, and repair it by recording the checker's
  answer under a span they can still find.
  [until: gone crates/nvs-syntax/src/ast.rs:Option<Type>]
- **A `Core` row that spells its callback's signature does not on its own hand
  `check_fn_literal` a substituted expected type.** `nvs_types::expr::args`' `check_generic_args`
  checks every argument but the options bag in its **first** pass, before `sig.substituted`, so a
  closure at a parameter mentioning a type variable is checked against `unplaced_expectation`'s
  answer — nothing — and an unannotated parameter still has nothing to take. Read that function's
  three passes before budgeting the registry rows: the callback argument has to be deferred the way
  the bag is, and that is a slice of its own rather than a line in the row's.
  [until: reviewed 2026-10-07]
- **`check_expr` reports the mismatch itself, so a pass that checks an argument only to *learn* its
  type must call `infer`.** `crates/nvs-types/src/expr/mod.rs:129` compares the result against the
  expectation and reports `E0401` there, which is why `check_generic_args`' first pass hands an
  expectation only to positions that are already final — a mid-pass check against a
  half-substituted type reports a mismatch the last pass would have reported correctly, and the
  message names the unsubstituted variable's `mixed`. Call `infer` where the expectation is there to
  place a literal rather than to judge it. [until: reviewed 2026-09-07]
- **A `Core` callback whose result the member discards returns `mixed`, never `void`.** `void` is
  what the row means and it refuses every `fn (): int` body a program may reasonably write, where
  `Ty::Void` *is* assignable to `mixed` in return position — so `mixed` takes a `fn (): void` body
  and a value-returning one alike, and still constrains the parameters. Write `CoreTy::Mixed` at any
  callback the member does not read back, and a concrete return only where it uses the answer.
  [until: reviewed 2026-09-07]
- **Deleting a `Core`-only parameter variant deletes the *bound*, not just the binding, and the
  hole is silent.** `CoreTy::CallableShapeTo` looked like pure binding machinery, so replacing it
  with `CoreTy::Var("S")` plus a walk reads as a clean simplification — but `S` binds to anything,
  so `Core\Task::all(5)` then type-checks and answers `mixed`, and only the runtime notices.
  Before retiring a bespoke parameter spelling, ask what refuses the argument once it is gone: if
  the answer is "a type variable", the variant was a bound and Novis has no bounded type variable
  to replace it with. [until: gone crates/nvs-stdlib/src/registry.rs:ShapeOfCallables]
- **A new helper shared by two checking functions in `nvs-types` has no room for a seventh
  parameter: `expr`, `args`, `live`, `scope`, `ctx` and `env` are already six, and clippy's
  `too_many_arguments` fires at eight.** Extracting the common half of two checkers therefore fails
  the clippy leg the moment it adds one argument saying which caller it serves, and the message
  names the helper rather than the extraction. Fold the discriminator into the data it selects over
  — an enum whose variants each hold the `&[TypeId]`, rather than an enum passed beside it — which
  keeps the count at seven and reads better at both call sites. [until: reviewed 2026-09-07]
- **An `@example` written inside `examples/` is refused unless it says `../examples/`, because the tag's
  two halves read two different paths.** `E0325`'s walked-directory test looks at the path *as written*
  (any `Normal` component named `examples` or `tests`), while `E0324`'s existence test resolves that same
  path relative to the file that wrote it — so `@example doc-comments.nvs`, sitting beside the file it
  names, is refused for being in no walked directory. Write the walked directory into the path and let
  `..` carry the resolution: `@example ../examples/doc-comments.nvs` passes both halves.
  [until: gone crates/nvs-hir/src/members.rs:let walked = written.components()]
- **A parser loop that left-nests a tree is charged nothing by `enter_recursive`, so the guard
  bounds the descent and never the result.** A left-associative tier consumes its chain in a loop
  rather than by recursing, and the depth that matters is what the first recursive walk over the
  AST must survive, not the parser's own stack. A tier written outside `parse_left_assoc` charges
  one level per link and holds it to the end of the chain, as `parse_postfix`'s `chain_len` does.
  [until: reviewed 2026-09-07]
- **`ExprTypeTable::declared_ty` answers for a parameter's annotation and not for a property's**: the
  signature pass lowers a property's type into a table of its own, and `nvs_types::check` copies types
  out of it and not spans. So a modifier read off `declared_ty` at a property declaration is always zero
  and fails no build. Read it off the access's own `ExprInfo::Property` entry instead, the way
  `nvs_lsp::semantic`'s `qualifiers_recorded` does — or, at a declaration where there is no access to
  ask, off `ExprTypeTable::property_default_ty`. [until: gone crates/nvs-lsp/src/semantic.rs:copies types out of and not]
- **`build_signatures` points `env.exprs` at a table it throws away, so nothing the signature pass
  lowers is readable afterwards.** `lower_type` records every annotation it resolves under
  `ExprTypeTable::record_type`, which reads as though a property's declared type were available
  later — it is not, because that pass runs against `placeholder_exprs`. Handing it the real table
  is not the fix either: `nvs_ir::lower::lower_decl_type` consults `declared_ty` *first*, so every
  property annotation would silently change lowering path. Carry what a later pass needs across on
  its own, the way `SignatureTable::property_default_types` does.
  [until: gone crates/nvs-types/src/signatures.rs:let mut placeholder_exprs]
- **A `TextEditorDecorationType` has no field for a CSS filter, and `textDecoration` is the one that
  reaches the rendered rule verbatim.** A blur is written `textDecoration: "none; filter: blur(Npx);
  clip-path: inset(0)"` — the declaration is closed with `none` and the rest follows it, several
  properties deep. The clip is not optional: a blur bleeds about its radius past the decorated
  range, and `overflow` cannot cut it back because a decoration is a non-replaced inline box, while
  the `display: inline-block` that would make `overflow` apply moves the character cells a
  decoration may never move. Anything put in that field is CSS the extension chooses on the user's
  behalf, so it stays geometric: a colour belongs in a `ThemeColor` field, which
  `rule:ide/novis-ships-names-not-colours` is about.
  [until: gone editors/vscode/src/redactions.ts:filter: blur]
- **A decoration that draws *nothing* is a dead server, not bad CSS — check which one before
  touching the stylesheet.** In DevTools the difference is one glance: a range that was decorated
  and styled badly carries a decoration class on its token span, and a range nobody answered for is
  a bare `span.mtk<N>` with only its theme colour. `nvs/redactions` returning nothing looks exactly
  like a filter that failed to apply, and a CSS change was made, shipped and written up as broken on
  that misreading before `Novis: Restart Language Server` turned out to be the whole fix. The
  status item and the `nvs.lsp.trace.server` channel answer it without guessing.
  [until: gone editors/vscode/src/redactions.ts:filter: blur]
- **`cargo build --release` by hand cannot replace `target/release/nvs.exe` while an editor's `nvs
  lsp` holds it**, and on Windows it fails with `failed to remove file ... Zugriff verweigert (os
  error 5)`. A `nvs.path` pointing at that binary is a lock on it for the life of the window, so a
  release build made to test a server change silently leaves the old binary in place and the editor
  keeps answering from it. `tools/dossier.py` and `tools/loop.py` retry that failure once with the
  old binary renamed aside (`tools/relink.py`), so a release build through either owes nothing — by
  hand, stop the server first with `nvs.lsp.enable` set to `false`, or close the window.
  [until: gone editors/vscode/src/extension.ts:const SUBCOMMAND]
- **No tier in this repository draws a decoration, so a change to how one looks is unverified until
  someone opens an editor.** `test:headless` runs the reveal state machine and the position
  conversion in plain Node, and the extension-host tier does not render; the README's *No pixel
  tier* says why neither will. A CSS change here is a proposal, not a landed fix — say so, and get
  it looked at before writing it down as one. [until: gone editors/vscode/README.md:No pixel tier]
- **A second `kind` on `nvs/redactions` is a change in three files, and the two that are not the
  server fail quietly.** The client conceals every kind it is handed, so a marker spelling added to
  `crates/nvs-lsp/src/redactions.rs` alone bars an identifier until
  `editors/vscode/src/concealment.ts`'s `MARKERS` has been taught it, and
  `docs/reference/tools/40-editor.md` names the kinds in prose, so `docs/novis.md` goes stale under
  a check nobody edited. Write all three, then `python tools/reference.py --no-examples`.
  [until: gone editors/vscode/src/concealment.ts:const MARKERS]
- **`nvs run`'s entry point has a second caller, and nothing near it says so.** `run_run` in
  `crates/nvs-cli/src/main.rs` is also how `bundle::run` starts a bundled program, so a new parameter
  breaks a call site in `crates/nvs-cli/src/bundle.rs` that no anchor in the `Command::Run` region
  points at — and what to pass there is a decision rather than a mechanical fill-in. Grep
  `super::run_run` before changing that signature, and answer for the bundle first: nothing stands in
  front of it to have handed it anything, so its answer is almost always the absent one.
  [until: gone crates/nvs-cli/src/bundle.rs:super::run_run]
- **A diagnostic raised while parsing a type is discarded when that type sits in a local
  declaration.** `parse_stmt_maybe_local_decl` trial-parses the type and backtracks on
  `self.diags.len() > cp.diags_len` (`crates/nvs-syntax/src/parser/stmt.rs:956`), so a precise
  refusal from `parse_type_atom` is dropped and the reader gets `expected an expression` instead.
  Write its fixture in a parameter, property or `type` alias, and collect it with
  `check_src_allowing_parse_errors`.
  [until: gone crates/nvs-syntax/src/parser/stmt.rs:cp.diags_len]
- **A shape's synthesized class cannot carry a codec, because two different shapes share it.**
  `nvs_ir::lower::shape_class_label` keys on the sorted field *names* alone
  (`crates/nvs-ir/src/lower/mod.rs:2957`), so `{n: int}` and `{n: string}` are one class and one
  `ClassDesc` — which is why `nvs_stdlib::json`'s encoder walks a shape's slots by tag instead of
  reading `desc.codec()` (`crates/nvs-stdlib/src/json.rs:583`). Anything handing a shape's per-field
  wire types to a native helper has to carry them at the *call site*, next to the
  `WRITTEN_CLASS_MEMBERS` descriptor rather than inside it. [until: reviewed 2026-09-08]
- **A shape class arms no slot, so `NvsObj::new` leaves an absent field `null` rather than
  never-written.** `ClassDesc::defaults` is the list that arms the marker and
  `nvs_ir::lower::record_shape_class` writes `defaults: Vec::new()`, so a native decoder filling a
  `{a?: int}` has to write `Value::unset()` into the slot itself — leave it and `{a?: int}` and
  `{a: ?int}` hold the same thing, which is two types that intern apart reading as one. `NvsObj::new`'s
  own doc comment names the never-written marker, which is exactly what makes the omission look
  impossible. [until: reviewed 2026-09-08]
- **A member name threaded into a shared reader has to be `&'static str`, because `claim_body` keeps
  it.** Widening `form_of` from `post` alone to `postAs` as well by adding a `member: &str` compiles
  everywhere except `claim_body(ctx, member, …)`, which reports `E0521: borrowed data escapes outside
  of function` at the *call site* and never names the field on `Ctx` that outlives the call. Every
  caller passes a literal, so the repair is `&'static str` down the whole chain — check what a `Ctx`
  setter stores before threading a name through a reader that two members share.
  [until: reviewed 2026-09-08]
- **`Tag::Unset` means two opposite things, and one arm answers both.** A `lateinit` slot still
  throws under `??` and `isset`, while the same tag on a `$shape{…}` class means the subject carried
  no such optional key, which `rule:types/shape-type` reads as absent. `ClassDesc::is_shape` splits
  them at `crates/nvs-runtime/src/object.rs:3460`, so an edit there runs both
  `a-property-that-was-never-written-is-read-as-a-throw.nvst` and
  `an-optional-field-is-absent-and-a-nullable-one-is-null.nvst`.
  [until: exists docs/rules/classes/an-unwritten-property-read-throws.md:shape]
- **A member seeded onto a compiler-declared interface is invisible to `nvs_hir::members`.**
  `member_declared_rec` (`crates/nvs-hir/src/members.rs:1307`) walks `implements` into `Parses`, finds
  no `MemberTable` entry — nothing declares a reserved interface in source — so `Slug::tryParse($s)` is
  `E0309` even though `crate::conformance` agrees the class inherits it. Do not repair that by trusting
  the roster the way line 1315 trusts `Core`: while the default body has no compiled function, the
  refusal is the safe answer and the alternative is a dispatch to nothing.
  [until: reviewed 2026-09-08]
- **A `Qual::Contagious` string parameter answering an *object* refuses a tainted argument, exactly
  as an unclassified `CoreTy::Str` does.** `crates/nvs-types/src/expr/quals.rs:304` admits one there
  only where `tainted_result(return_ty) != return_ty`, and a class never carries the qualifier.
  `Qual::Neutral` is the mark for a parse that checks its text and answers an object
  (`crates/nvs-stdlib/src/time.rs:251`), and classifying a row means deleting its line from
  `registry.rs`'s `UNCLASSIFIED` ratchet in the same edit.
  [until: gone crates/nvs-types/src/expr/quals.rs:tainted_result(return_ty, interner) != return_ty]
- **Widening a `nvs_types` enum that crosses into `nvs_runtime` breaks a third crate no file set
  named.** `nvs-cli/src/main.rs` maps `nvs_types::commands::ArgConv` onto the runtime's by an
  exhaustive hand-written `match`, and `capture_conv` does the same for `CaptureConv`, so one variant
  replaced in the checker is a compile error two crates away. Grep `nvs_types::` across
  `crates/nvs-cli/src/main.rs` before pricing such a slice, and decide the bridge's answer for the new
  arm in that slice rather than from the build.
  [until: gone crates/nvs-cli/src/main.rs:nvs_types::commands::ArgConv::]
- **A route row's rendered type cannot tell a class from an enum, so a reader that must branch on the
  *kind* of type needs the row to say which.** `TypeInterner::describe` renders `Ty::Class(q, [])` and
  `Ty::Enum(q, _)` as the same bare qualified name, and `closed_set` answers `None` for both, so
  neither thing `RouteParam` carried discriminated them. Compute the answer where the interner is
  still alive and carry it on the row; `crates/nvs-types/src/routes.rs`'s `RouteParam::parses` is the
  shape. [until: reviewed 2026-09-08]
- **Widening a `CoreTy::Union` a `Core` member answers breaks frozen spellings of it in two suites,
  and neither failure names the union.** A `.nvst` quotes the rendering inside a diagnostic about
  something else, and `nvs_types::core_lib`'s `a_verified_signature_does_not_launder_its_claims`
  freezes every `tainted`-answering member's rendered return type. Grep `tests/` and `crates/` for a
  slice of the old rendering first, and take the new one from the binary rather than composing it —
  the order is the interner's. [until: reviewed 2026-09-08]
- **`as` refuses a `Core` class as its target, so a capture union's `Core\Uuid` arm cannot be
  downcast the way a user class's can.** `E0711` says the target "names no class to test the value
  against", because `rule:types/conversion` tabulates conversions into a *declared* class and
  `Core\Uuid` is not one — while `$capture as Slug` compiles right beside it and reads as if the two
  were symmetric. Read the engine's arm by rendering it instead: `echo $match->param("id")` prints the
  canonical text, since every arm of that union renders.
  [until: reviewed 2026-09-08]
- **`Parses::tryParse` does not resolve on a user implementor, though the interface declares it with
  a body.** `crates/nvs-types/src/iter_lib.rs:123` seeds `tryParse` with `has_body = true`, so it
  reads as available, but `Slug::tryParse("x")` on a class implementing `Parses` is
  `E0309: 'Slug' has no method named 'tryParse'` — only `parse` is reachable. Write `parse` with a
  `catch` where you wanted the pair, and never put `tryParse` on a user class in a `docs/reference/`
  example, because `tools/reference.py` compiles every one of them.
  [until: reviewed 2026-09-08]
- **A capability check added at the *top* of a door that already refuses on a missing directive
  re-orders refusals the `.nvst` corpus pins exactly.** Two cases assert `open_configured`'s
  "no shared store is configured" over a tree that grants nothing, so a `require` above the
  directive read turns both into a capability denial. Ask the grant after the directive, which is
  `rule:security/capability-costs-nothing-unasked`'s shape, and grep the corpus for the refusal's
  own words first. [until: reviewed 2026-12-01]
- **A `pub(crate)` stream type in a `Wire<S>` default fails 48 times in `nvs-stdlib` and never names
  the cause.** `MySqlRows<'a, S = MyStream>` is public, so the default is part of its signature and
  every stdlib line holding rows becomes a `private type` error pointing at the caller. Make the
  stream `pub` and re-export it beside the rows type in the same edit — `PgRows` and `TdsRows` carry
  the same default. [until: reviewed 2026-09-09]
- **An `unsafe impl Send for T {}` switches the auto-trait check off, so the fields your safety
  argument rests on are never re-checked — including a dependency's.** `nvs_codegen::Unit`'s `Send`
  rests on `cranelift_jit::JITModule` being `Send` (the last `Arc` handle drops one on whatever core
  it lands on), and nothing about writing the impl makes the compiler agree; a cranelift bump could
  take it away and leave every test green. Pin each foreign clause with its own assertion —
  `const fn sends<T: Send>() {} sends::<JITModule>();` beside the type's own crossing test, and
  deliberately *not* the clause the argument does not need. [until: reviewed 2026-09-09]
- **A short buffer on a datagram receive is two different syscall answers.** `recv_from` on Windows
  refuses the call with `WSAEMSGSIZE` and hands back neither a count nor a sender, where the Unixes
  keep what fits and answer normally, so a case pinning truncation is green on CI's Linux legs and
  red locally. Hand the kernel a buffer no datagram can overflow and make the `$max` cut in the
  member — `nvs_stdlib::net`'s `DATAGRAM_CEILING` is that shape.
  [until: reviewed 2026-09-09]
- **A new `Core` member with a bare `CoreTy::Bytes` or `CoreTy::Str` parameter fails a gate that
  offers the wrong fix.** `every_member_parameter_carries_a_qualifier_classification` prints your
  row as a line to paste into `UNCLASSIFIED`, which is a freeze of the rows written before the
  classification existed and not a list to join. Classify instead —
  `CoreTy::Blob(Qual::Contagious)` for a member that reshapes octets and learns nothing about where
  they came from, and it is accepted inside a `CoreTy::Union` member too.
  [until: reviewed 2026-09-09]
- **A `CoreTy::Uint` parameter arrives as a `uint`-tagged `Value`, so `as_int()` answers `None` and
  the member dies at run time rather than at build time.** The failure is a `FATAL ... expected a
  non-negative `uint` ..., got tag 3` from the argument guard a session writes as unreachable, which
  reads like a caller error and is actually the accessor: a row's `Const::Uint` default is
  materialised as a `uint`, not as an `int`. Use `Value::as_uint` for every `CoreTy::Uint` slot —
  `crates/nvs-stdlib/src/compress.rs`'s `uint_of` is the shape — and reach for `as_int` only where
  the row says `CoreTy::Int`. [until: reviewed 2026-09-09]
- **A cycle guard on an array arm is not evidence that an array can cycle.** An array is
  copy-on-write, so `$a[] = $a` appends a copy and an array-only descent is finite with no guard at
  all (`crates/nvs-runtime/src/graph.rs:20`); `crates/nvs-stdlib/src/json.rs:817` guards its array arm
  only because an **object** inside that array puts the same allocation back on the path. Audit a
  walker by asking whether it ever descends into an object, not whether it descends into a container.
  [until: gone crates/nvs-runtime/src/graph.rs:copy-on-write storage]
- **A `Core\X::y()` call lowers to `InstKind::CoreCall`, not `InstKind::Call`.** `Call` is a method
  the program itself declared; a `Core` member goes to `CoreCall`, which codegen hands to
  `emit_helper` under `rule:errors/propagation`'s fixed signature
  (`crates/nvs-codegen/src/emit.rs:585`), so a change to what a `Core` call site carries written
  against `Call` reaches nothing. Read the `emit` arm for the variant first, and copy
  `InstKind::ShapeCodecConst` when what you add is a compile-time constant the member reads off
  its own `args`. [until: gone crates/nvs-ir/src/ir.rs:CoreCall]
- **A new `Core` row trips frozen assertions in modules it does not touch.** A
  `CoreTy::Text(Qual::Launder)` parameter fails `nvs_stdlib::html`'s
  `every_launderer_for_an_auto_escaping_sink_answers_a_carrier`, which asserts that roster whole
  across every class, and an unclassified `CoreTy::Str` in a `CoreOption` fails
  `every_member_parameter_carries_a_qualifier_classification`, because an option is a parameter.
  Run `python tools/verify.py -p nvs-stdlib` after writing a row and before the full gate.
  [until: reviewed 2026-09-09]
- **A depth-ceiling constant is not evidence that a recursive walk over a document that deep
  survives.** `Core\Xml`'s ceiling is 1024 and `nvs_host::TASK_STACK_SIZE` is 1 MiB, so one native
  frame per node overflowed at about 900 — inside a bound the parser still accepted, turning a
  stated refusal into a crash. Price the frame against that stack rather than against the constant,
  and put the stack on the heap: `crates/nvs-stdlib/src/xml.rs:@instance_of` is the shape.
  [until: reviewed 2026-09-09]
- **Replacing a bag guard's "never nullable" invariant with the omitted-versus-null pairing turns
  every `CoreTy::Mixed` option red, not just the one you set out to make nullable.** `mixed` admits
  `null` without spelling it, so `Core\Queue::push`'s `args` had to take `Const::NeverWritten` the
  moment `Core\Uri::with` did — a class away from the slice, and the assertion names the member
  rather than the rule. Before landing such a pairing, `grep -n 'CoreTy::Mixed' crates/nvs-stdlib/src/`
  for the rows it will reach, and widen each helper's "not given" branch from `Tag::Null` to
  `Tag::Null | Tag::Unset` in the same edit. [until: reviewed 2026-09-10]
- **No registry spelling can make a `mixed` parameter refuse a qualifier, so a gap reading as a
  missing `CoreTy` is a missing call-site rule.** A `Qual` never refuses on its own — `expr::args`
  only *narrows* the compared type for a mark that admits one, so every refusal is `is_assignable`'s,
  and `crates/nvs-types/src/expr/assign.rs:70` accepts every qualified atom into `mixed`. Write it
  beside its siblings in `nvs_types::expr::quals`, over the written argument, and file its check
  under the crate that owns the diagnostic. [until: reviewed 2026-09-10]
- **A Novis array key is text whichever shape the array is in, so `SlotKey::Index` is a storage
  detail and not a second kind of key.** A hashed array renders a position as its decimal —
  `set_index(1, …)` on one then answers `SlotKey::Str("1")` — so a walk that treats the two variants
  as different keys gives one array two answers depending only on whether it ever had a gap. Compare
  and write `SlotKey::to_str()`, never the variant.
  [until: gone crates/nvs-runtime/src/array.rs:SlotKey::Index]
- **A spec signature can be unrepresentable in `CoreTy`, and the sibling roster entry has already
  resolved it.** § 16 wrote `Core\Signature::verify(...): array<string, mixed>` *and* called the
  result `tainted`, which cannot both hold — the qualifier is defined over `string` and `bytes`
  alone. Before transcribing a spec row whose **result** is qualified, read the neighbouring class's
  module doc: `crates/nvs-stdlib/src/jwt.rs`'s *a claim is text* section had the whole trade worked
  out already.
  [until: gone crates/nvs-stdlib/src/registry.rs:there is no `tainted array<T>` in `nvs_types`]
- **A new `ExprKind` variant breaks no build outside `nvs-syntax`, so nothing tells you which passes
  still owe it an arm.** `ExprKind` is `#[non_exhaustive]` (`crates/nvs-syntax/src/ast.rs:770`), so
  every match on it in another crate already carries a wildcard — the checker's is
  `_ => env.interner.mixed()` and lowering's is a panic, both of which a new node reaches silently.
  Grep for the sibling variant you copied (`ExprKind::Conversion`) across `crates/` and add an arm
  everywhere it appears, rather than trusting `cargo check` to list them.
  [until: reviewed 2026-09-10]
- **`is_assignable` is not "holds one at run time", and the gap is exactly one row.** A fold that reads
  `$x is T` as *is the subject assignable to `T`* answers `true` for `int $n; $n is float`, because
  `rule:types/conversion`'s one implicit conversion lets an `int` **occupy** a `float` position while
  carrying a tag it never has. Decompose unions yourself and refuse that pair before calling it —
  `always_holds` in `crates/nvs-types/src/expr/type_test.rs` is the shape, and the `false` direction
  reuses `types_are_disjoint` rather than negating this one. [until: reviewed 2026-09-10]
- **A new `panic!` naming a shape in `nvs-ir` fails `-p nvs-ir --test refusals` on a count, not on the
  panic.** `tools/holes.py` reads a refusal site off the construct rather than off the wording, so a
  "this row is not lowered yet" panic trips `CEILING` even when the attribution half of that gate is
  green, and rewording one to dodge the recognizer is the move that file forbids by name. Land the
  whole shape, close another site in the same slice, or raise `CEILING` and name in its doc comment
  the item that brings it back down. [until: reviewed 2026-09-10]
- **A multi-table `delete` on MySQL is driven from a derived row, and its predicate goes on the
  join.** `delete j, d from nvs_jobs j left join nvs_dead_jobs d …` answers nothing for a job that
  has already moved tables — there is no left side to hang the other row on — and a trailing `where
  j.state <> 1` drops the driving row for a claimed job, taking that arm with it. Drive it from
  `(select ? as jid, ? as qname) r`, give each table its own `left join` carrying its own predicate,
  and prepare it against a live server before writing the doc comment.
  [until: gone crates/nvs-stdlib/src/queue.rs:delete j, d]
- **A `Split` on SQLite cannot always open the immediate transaction its statement doc names.**
  `nvs_db::SqliteConn::begin_immediate` refuses a connection already in one, being outermost by
  construction, so a member run on the request's shared connection would refuse the very enqueue
  `rule:concurrency/enqueue-commits-with-your-write` says commits with the caller's write. Branch on
  `depth()`: `begin_immediate` at zero and `begin(None, false)`'s savepoint inside one, which is
  `nvs_stdlib::queue`'s `sqlite_opened`.
  [until: gone crates/nvs-db/src/sqlite.rs:immediate transaction is an outermost one]
- **`lsp_types` 0.97's `ServerCapabilities` has no `typeHierarchyProvider` member**, though it carries the
  three requests, their params and the client half, so the gap reads as a misspelling. A capability the
  struct cannot express is one no client ever asks about, which looks exactly like a handler nobody wired
  up. `nvs_lsp::declared_capabilities` serializes the struct and inserts the key into the object.
  [until: gone crates/nvs-lsp/src/capabilities.rs:typeHierarchyProvider]
- **A bit in the safepoint word is the whole request tree's, so a per-context request cannot live
  there.** `Ctx::child` and `Ctx::isolate` share the word, which made one task's cancellation stop
  its siblings and its parent — and the only case that caught it was
  `tests/conformance/task/a-deadline-is-the-only-spelling-for-a-bounded-wait.nvst`, reporting a
  *later* call as returned rather than timed out. Before adding a flag, ask whether one context
  raising it should stop every other context in the tree; if not it is a field on `Ctx`, the way
  `cancelled` is, and not a bit in `SafepointFlags`.
  [until: gone crates/nvs-runtime/src/ctx/mod.rs:safepoint_word]
- **The pre-check refuses the runtime's own report too, and it fails as an empty string.** A request
  reaching `rule:errors/on-limit`'s tier 1 is past its balance, so every `NvsStr::new` on the
  reporting path answers the immortal empty and the handler gets a record whose keys are `""`. Hold
  `nvs_runtime::budget::Reporting` across the whole escalation, as `Ctx::run_limit_handler` and
  `nvs_host::ladder` do. [until: gone crates/nvs-runtime/src/budget.rs:pub struct Reporting]
- **A `budget::Detached` bracket around a store that is *handed* its bytes corrupts both balances.**
  The caller's `Vec` was allocated under the request, so releasing it inside the bracket charges the
  process for what the request was charged — `live_bytes` drifts up and `detached_bytes` down without
  bound, each wrong alone though the sum is right. Copy into an allocation the bracket makes and
  release the caller's temporary outside it (`crates/nvs-stdlib/src/cache.rs:@store_put`); a store
  whose entry is an `Rc` the request also holds cannot be bracketed at all.
  [until: gone crates/nvs-runtime/src/budget.rs:pub struct Detached]
- **A type name a goal file hands you can already be taken at the crate root, and the collision only
  shows up at the call site.** Goal `event-streams` names the response cell's halves `Emit` and
  `Drain`, and `nvs_runtime::Drain` has been the server's drain bit all along, so re-exporting the
  new one flat would have put two types of that name in `crates/nvs-server/src/serve.rs`, which
  imports both. Keep such a pair inside its module and write `stream::Drain` at every call site
  rather than aliasing one of them into a second name.
  [until: gone crates/nvs-runtime/src/drain.rs:pub struct Drain]
- **A `CoreTy::Union` parameter refuses a `tainted` argument however its arms are marked, so a
  `string|bytes` parameter is stricter than the `string` beside it.** `CoreTy::classification`
  answers `None` for a union (`crates/nvs-stdlib/src/registry.rs:1059`) and
  `rule:security/unclassified-parameter-refuses-tainted` makes an unclassified text-like parameter
  refuse one, so the marks written on the arms reach nothing — `Core\Socket` split `send` and
  `sendBytes` into two members for exactly this and says so in its row. Decide whether the member
  has to accept tainted data *before* spelling a union, and take that split if it does.
  [until: reviewed 2026-09-11]
- **A service future that answers early takes the request's body pump down with it, and the request is
  still reading.** `serve.rs`'s `Reply::Run` arm owns `Supply` and pumps it from the `poll_fn` it parks
  in, so a head answered while the isolate runs ends that future and leaves a program parked on a pull
  nothing will ever feed — a hang with no error anywhere, not a failed read. Anything that lets a
  request answer before it ends has to move the supply onto the connection with it, which is what
  `Streamed::supply` and the pump at the top of the drive loop are.
  [until: reviewed 2026-09-11]
- **A deadline filed from inside a `hyper` poll does not survive the pass.** `nvs_host::Timers` keeps
  one per task, and any `NvsStream::poll_read` answering `Pending` files the socket's over whatever a
  body just filed, so the connection wakes at the wait it was to act before. Publish the instant
  through a shared cell and file it from `serve_connection`'s drive loop once the connection future
  has answered `Pending` — `crate::bounds::wake_at`. [until: gone crates/nvs-host/src/timer.rs:one-timer-per-task]
- **`nvs_host::Isolate`'s `finish` classifies an ending from the context alone, and a `Program` hands
  it no status.** A throw leaves its object in `Ctx::pending`, but an `exit` leaves only
  `Ctx::exit_code`, which reads `0` both for `exit(0)` and for a script that never called `exit`. A
  program records its frame's status with `Ctx::set_ending` and `finish` reads `Ctx::ending`; do not
  recover an ending from the *shape* of the pending value, because a `FATAL` and a throw both leave
  one and `Pending`'s variants are private.
  [until: gone crates/nvs-runtime/src/script.rs:Value) -> Value]
- **`nvs-host` cannot name `nvs-stdlib`, so a question both hosts ask belongs in `nvs-runtime`.**
  `nvs-stdlib` depends on `nvs-host` for `Core\Http\Client`'s socket, so a handoff item or comment
  spelling `nvs_stdlib::…` is unreachable from `crates/nvs-host/src/`, and the error reads like a
  missing manifest line rather than like the cycle it would be. Move the predicate and its constant
  down to `nvs-runtime`, and leave a `pub use` in the stdlib module so callers outside the host keep
  their spelling. [until: gone crates/nvs-stdlib/Cargo.toml:nvs-host.workspace]
- **A new `ExprKind` variant compiles workspace-wide with one arm written, and that is not the same as
  being handled.** `ExprKind::Markup` broke exactly one match — `crates/nvs-syntax/src/walk.rs:392`,
  which is exhaustive — while `nvs-hir`, `nvs-lsp`, `nvs-ir` and `nvs-types` each swallowed it in a
  catch-all, so a green `cargo check --workspace --all-targets` says nothing about whether inference or
  lowering saw the node. Before the parser is taught to produce a new expression node, `grep -rn
  "ExprKind::<the sibling variant>"` over `crates/` is the list of places that must gain a deliberate
  arm. [until: reviewed 2026-09-12]
- **A string literal folds into a unit's data section; an *object* does not follow from that, because
  its second header word is an identity rather than a payload.** `immortal_header_bytes` needs only
  the bytes, while an instance needs a `*const ClassDesc` — and a `Core` class's was leaked once per
  core, so no single address existed for a unit every core reads to bake in. Before carrying the
  immortal-string precedent to another representation, ask what else its header holds and where that
  word's identity comes from. [until: reviewed 2026-10-12]
- **A markup literal's "a brace before a non-`$` is text" does not make the whole braced run inert —
  a bare `$name` inside that text still interpolates.**
  `{Core\Html::join($nothing, "" as Core\Html\Markup)}` reads as one text segment and is three: the
  text `{Core\Html::join(`, a simple-syntax interpolation of `$nothing`, and the rest — so an `array`
  operand there is `E0707: no string form` at a line that looks like a static call. Put the call in a
  local and write `{$local}`; the hole grammar is the double-quoted string's, **both** halves of it
  (`rule:core-classes/html-literal`). [until: reviewed 2026-09-12]
- **A `docs/rules/tooling/fmt-*` example can be a spelling the grammar refuses, and a case built on it
  comes back as a refusal rather than a layout disagreement.** Those fragments were written at design
  time against a grammar that had not shipped, so `tainted ?string` (the `?` wraps the qualified type)
  and a brace-bodied `fn` with no `=>` in front of it are examples no `.nvs` file can hold, and
  `nvs-fmt`'s harness answers one with "does not parse (43 error(s))". Run a fragment's example through
  `target/debug/nvs.exe check` before building a fixture on it, and correct the fragment where it loses.
  [until: reviewed 2026-09-12]
- **A RustCrypto crate at its newest major can be generic over a different `digest` generation than
  `Core\Hash`'s.** `hkdf` 0.13 and `pbkdf2` 0.13 take a `digest 0.11` hash while `sha2.workspace` is
  0.10, so `Hkdf::<sha2::Sha256>` fails an `EagerHash` bound with an error naming
  `CoreWrapper<CtVariableCoreWrapper<…>>` and never the version. Check which `digest` major a crate
  is generic over before pinning it, and name the matching hash through the root manifest's
  `sha2-v11` row rather than downgrading the crate, which would only add a duplicate.
  [until: gone Cargo.toml:sha2-v11]
- **A new `CoreTy` variant is one edit in `registry.rs` and four outside it.** The enum is
  `#[non_exhaustive]`, which does nothing inside its own crate, so `ast.rs`'s `mentions` walk and
  `xml.rs`'s two — `mentions` and `every_text_position_is_tainted` — and `nvs-types`'
  `core_lib::lower` all stop compiling, while `classification`, `spelled` and `is_text_like` in
  `registry.rs` itself keep compiling and quietly answer wrong for the new spelling. Grep a rare
  variant (`CoreTy::SecretTaintedStr`) to get every arm in one call rather than chasing the compiler
  through them one at a time, and check the three `registry.rs` methods by hand.
  [until: reviewed 2026-09-12]
- **A `Core` class holding slots and registering no instance member fails a registry unit test until
  it is named in that test's own roster.** `a_class_with_slots_has_instance_members_and_the_reverse`
  (`crates/nvs-stdlib/src/registry.rs:5110`) asserts the two are empty together, so a class storing
  state nothing reads back — `Jwe\Key`, `Jwt\KeySet` — fails on that assertion rather than on
  anything in its own file. Add its `NAME`, made `pub(crate)`, to the test's `HANDLES` list in the
  slice that registers the class.
  [until: gone crates/nvs-stdlib/src/registry.rs:const HANDLES]
- **A `Core` option admits a qualifier in its *type*, never with a `Qual`.** `nvs_types::core_lib`'s
  `qual_of` answers `None` for an options-bag member, and `None` refuses a `tainted` or `secret`
  argument exactly as `Qual::Sink` does, so `array<string>` refuses a `secret string` however the
  option is marked. Declare the atom that carries the bits instead — `CoreTy::SecretTaintedStr` — and
  pair a type that admits `null` with `Const::NeverWritten` rather than `Const::Null`, which two
  `registry.rs` tests fail on by name. [until: reviewed 2026-09-12]
- **A member joining `WRITTEN_CLASS_MEMBERS` gets the `tainted`-field check for a written *class* and
  not for a written *shape*.** `written_class_of` records a `DecodeSite` on the member name alone
  (`crates/nvs-types/src/expr/args.rs:1649`), but reaches `check_shape_decode_site` behind an **owner**
  list a few lines down — so a reject case written as `jsonAs<{name: string}>()` compiles and runs
  while the same case written as a `#[Json\Derive]` class is refused, which reads as the rule not
  applying rather than as one roster with two halves. Add the owner to that list in the same edit that
  adds the registry row. [until: gone crates/nvs-types/src/expr/args.rs:owner_name]
- **A `Core` reader returning `instance::slot` prints the right answer and then corrupts the heap at
  teardown.** That helper *borrows* the slot, so a member handing one back to Novis over a string or
  object exits `-1073740940` after passing every assertion in the case, which `nvs test` reports as
  "expected the run to succeed" with no line number anywhere. Write a reader as
  `crate::instance::read_slot(args, &CLASS, AT, "member")`, which is the pair that borrows and
  retains, and read a crash with no diagnostic as a missing retain in the last member added.
  [until: reviewed 2026-09-13]
- **A `macro_rules!` that lands with no call site turns `verify.py` red at clippy, not at the
  build.** `cargo build` reports `unused macro definition` as a warning and exits green, while
  `tools/verify.py` runs clippy with `-D warnings` and fails on that same line, so a keystone macro
  written in one slice and first used in the next never gets through. Land the macro with its first
  real call site in the same slice — for `guarded_by!` that meant closing one refusal site and
  lowering `crates/nvs-ir/tests/refusals.rs`'s `CEILING` alongside it.
  [until: reviewed 2026-12-14]
- **`nvs_syntax::ast::StmtKind` is `#[non_exhaustive]`, so no match on it outside `nvs-syntax` may drop
  its wildcard arm.** `crates/nvs-syntax/src/ast.rs:1271` carries the attribute, and Rust requires that
  wildcard however many variants a cross-crate match spells out, so a plan to make a new variant fail to
  compile in `nvs-ir` cannot be written there. Spell the named shapes as their own arms and leave the
  wildcard as an engine invariant, or take the attribute off — the second is a decision about
  `nvs-syntax`'s surface. [until: reviewed 2026-09-14]
- **`nvs_runtime`'s `tag_name` spells Novis's type names, not PHP's, so a message that has to match
  PHP's word for word cannot reuse it.** `crates/nvs-runtime/src/helpers.rs:571` answers `array<T>`
  and `uint` where PHP says `array` and `int`, because every caller it has today is a Novis-worded
  refusal (`no ordering for a ...`) rather than a transcription. A site that owes PHP's exact wording
  — `clone`'s `must be of type object, <type> given` is the one in front of us — needs its own
  rendering with PHP's spellings, and the two must not be folded into one table without deciding
  which set of names each caller wants. [until: reviewed 2026-12-14]
- **A new `nvs_ir::Helper` row takes four edits, and the one no compiler catches is the JIT's
  symbol table.** `emit.rs`'s `helper_symbol` and `print.rs`'s `helper_name` are exhaustive
  matches the build names, but the `("nvs_…", address(…))` list in
  `crates/nvs-runtime/src/helpers.rs` is data, so a missing row is a `can't resolve symbol`
  panic out of `cranelift-jit` that only a case reaching the helper sees. Add the registry row
  in the same edit as the variant, and run such a case.
  [until: gone crates/nvs-codegen/src/emit.rs:fn helper_symbol]
- **`as` performs no shape walk, so "the walk `as` already does" is not available for `is {x: int}`.**
  `$m as ?{x: int}` is `E0711` where it is written — `rule:types/conversion` tabulates no conversion
  into an object — which means the shape row of `rule:types/type-test` is a walk to *build*, across
  the checker, `nvs-ir` and the runtime, and not a second caller of one that exists. Run the spelling
  through `target/debug/nvs.exe run` before planning a row around a walk a rule names, because a rule
  naming a conversion is not evidence the conversion lowers.
  [until: reviewed 2026-11-01]
- **An item reading "this match has no catch-all" cannot be met literally: `nvs_syntax::ast::TypeKind`,
  `TypeAtom` and `nvs_types::ty::Ty` are `#[non_exhaustive]`, so another crate must carry a trailing
  arm and is never told which variant it forgot.** Spell every variant the enum has, make the trailing
  arm say `this is a bug` (`tools/holes.py`'s `ENGINE` skips that, and `REFUSAL` never matched it),
  and add a probe asserting the row count — that count is the guard the compiler will not be.
  [until: gone crates/nvs-types/src/ty.rs:non_exhaustive]
- **A string literal's inferred type is `Ty::String`, not `Ty::StringLiteral`, unless the position
  already expected that exact literal.** `placed_literal` (`crates/nvs-types/src/expr/literals.rs:55`)
  places a literal type only against an expectation naming it, so a check that reads a written word off
  the inferred type sees `string` and matches nothing — which looks like the word was accepted. Read the
  word off the AST instead, `ExprKind::Str(span)` through `crate::string_lit::cook_string_literal`, the
  way `spawn script`'s `on:` does. [until: gone crates/nvs-types/src/expr/literals.rs:placed_literal]
- **A `spawn script` helper's `Err` edge must not release the transferred `args:` value: the
  caller's fault edge already does.** `lower_spawn_script` is the one `CoreCall` site that skips
  `forget_transferred_since`, so the temporary is still on the stack the landing block sweeps —
  while that function's own doc and every ordinary call site say the opposite, and are what a reader
  finds first. Add no release on a new `Err` path out of `Host::start_isolate` or either spawn
  helper; `crates/nvs-ir/src/lower/expr.rs:3243` proves it, and `copy_graph` leaves the root's
  reference alone for the same reason.
  [until: gone crates/nvs-stdlib/src/script.rs:let crossing = args]
- **A `spawn script` option that type-checks is not necessarily carried anywhere.**
  `nvs_types::expr::isolate` accepts and types all five options, but
  `Lowering::lower_spawn_script` builds the `CoreCall` out of four values — the entry, `args:`,
  `output:` and `on:` — so `limits:` and `grants:` are checked and then dropped, and a program that
  writes a narrowing gets none of it. Read the lowering's `written(SpawnOptionKey::…)` arms before
  writing a case over an option, because a `.nvst` that asserts a narrowing took effect will fail for
  a reason no diagnostic names.
  [until: gone crates/nvs-ir/src/lower/expr.rs:SpawnOptionKey::Limits]
- **A `spawn script` shape option arrives as an object, not an array.** `limits: {memory: "1M"}`
  is a shape literal, and a shape is an object with one slot per field it names, so a helper that
  reads it with `nvs_array_next_slot` gets `expected a shape, got tag 7` at run time and nothing
  at compile time. Walk it with `NvsObj::from_raw` plus `ClassDesc::field_name(slot)` and
  `NvsObj::field(slot)`, which is also what spares the reader a second copy of the field list.
  [until: gone crates/nvs-stdlib/src/script.rs:fn limits_of]
- **A ThreadSanitizer fiber switch written as two calls corrupts the sanitizer's heap.** Its
  instrumentation pushes at a function's entry and pops at its exit from *the current fiber's*
  shadow stack, so a `switch_to()` helper entered on one fiber and returned from on another pops one
  that is still empty, and the process dies in `__tsan_func_entry` far from the cause. Keep each
  switch inside one call that leaves and comes back — `Fiber::around` in
  `crates/nvs-host/src/tsan.rs` — and read a SEGV in `__tsan_func_entry` as a broken annotation
  rather than a race. [until: gone crates/nvs-host/src/tsan.rs:__tsan_switch_to_fiber]
- **A `#[cfg_attr(not(test), expect(dead_code, …))]` over a seam whose only callers are its own cases
  still fails `clippy --all-targets`.** The attribute is gone in the test build, so every item the cases
  do not actually reach — a `Platform::host()` nothing asserts, an enum variant no fixture constructs —
  is reported there instead, one item at a time. Reach each of them from a case rather than widening the
  attribute: a variant nothing constructs is a decision nothing holds to.
  [until: reviewed 2026-09-15]
- **A `#[cfg_attr(not(test), expect(dead_code))]` on a module covers nothing the `cfg(test)` build
  adds to it.** `crates/nvs-cli/src/service.rs`'s `registration` carries one, so an applier landed
  ahead of its caller compiles silently under `cargo build` and then warns under `cargo clippy
  --all-targets`, which is the only leg that builds the test copy at all. Give each such item its
  own `#[cfg_attr(test, expect(dead_code, reason = …))]`, and reach for `cargo clippy --all-targets`
  rather than `cargo build` while a seam still has no caller.
  [until: gone crates/nvs-cli/src/service.rs:the subcommands that reach this seam]
- **A helper only `registration::Systemd` reaches is dead code on Windows, and only in the test
  build.** `cargo clippy --all-targets -- -D warnings` stops on it while the ordinary build is
  clean, because `Systemd` is compiled on both platforms but constructed only where it is the
  host's manager, so on Windows everything under it is reachable from a case alone — and a case
  drives `Recording` instead. Spell the marker `#[cfg_attr(all(test, windows), expect(dead_code,
  reason = "…"))]`, which is what `Systemd` and `systemctl` already carry.
  [until: reviewed 2026-09-15]
- **Giving a class its first `fs.read`/`fs.write` row widens a sweep that had been reading the
  capability table as a list of path-taking *classes*.** `crates/nvs-stdlib/tests/capability.rs`'s
  `no_member_dispatches_on_a_uri_scheme` derived its subject per class, so `Core\Response::sendFile`'s
  row pulled `redirect` in with it and the sweep refused that member's `url` parameter, which names a
  URL legitimately. The derivation is per member now — `takes_a_path` in that file is where it is
  decided — so a new fs row sweeps the member holding it and nothing beside it.
  [until: reviewed 2026-09-15]
- **A refusal `nvs-db` builds with `io::Error::other` reaches a program as `Core\Db\DbError`,
  whatever its wording says.** That crate builds no fault of its own, so the `io::ErrorKind` is the
  only thing carrying a class across the boundary: `statement_failure`
  (`crates/nvs-stdlib/src/db/bind.rs:186`) reads `InvalidInput` as a mistake in the call and
  everything else as the engine's own refusal. Build a refusal about the *call* — a busy connection,
  a value with no bound form — with `io::ErrorKind::InvalidInput`, and prove it with a `.nvst` case
  catching the class the rule names. [until: gone crates/nvs-stdlib/src/db/bind.rs:ErrorKind::InvalidInput]
- **A module doc's gap sheet can carry a `Decided:` wire form that the landed encoder contradicts,
  and two green conformance cases can be pinning the landed one.** `nvs_stdlib::json`'s gap 1 decided
  a `decimal` crosses as the JSON string `"12.50"` while `Encodable` wrote it as a bare number, so
  closing the decode half without the encode half would have shipped a codec pair that is not a round
  trip. Read the gap's own first paragraph before assuming the code wins: where it says the question
  *was open* when the code was written, the sheet's answer is the later decision and the cases pinning
  the older spelling are part of the slice. [until: reviewed 2026-09-15]
- **A member declaring `: static` cannot `return new <its own class name>(…)`, and the diagnostic is
  `E0741: a body declaring `static` returns a value that is not the called class`.** `static` is the
  class the *call* named, which a subclass may be, so naming the declaring class is narrower than
  the declared return — and the message says what is wrong rather than how to spell it. Write
  `return new static(…)`, which is what `Core\Db\Codec`'s and `Core\Json\Codec`'s `fromRow`/
  `fromJson` fixtures need whenever the case asserts that nothing was reported.
  [until: reviewed 2026-09-15]
- **A closure whose own `foreach` binds a name the enclosing frame also bound panics `nvs-ir` rather
  than compiling.** `nvs_types` records the inner use as a *capture* because its scope resolved the
  outer `foreach` variable, and lowering then finds no slot for it —
  `crates/nvs-ir/src/lower/expr.rs:2667`, reproduced by nothing more than two `foreach`es over an
  `array<string>` with one variable name between them. Give the closure's loop variable a name of its
  own; the outer one is not what the closure meant either way.
  [until: test a_closure_binds_its_own_foreach_variable_over_an_enclosing_one]
- **`nvs_config::Request::set` is not a narrowing check for a `[limits]` ceiling.** Its module doc
  refuses "a `RuntimeTighten` one that does not narrow", but `capabilities` is the only such row in
  `crates/nvs-config/src/directive.rs`, so `[limits] memory` is plain `Runtime` and a request may
  set its own ceiling wider or narrower up to `[limits.hard]`. All `set` proves about a ceiling
  written at a call site is that the hard bound admits it, so read the directive row before writing
  "narrowed and never widened" about anything but a grant. [until: reviewed 2026-09-15]
- **A `?T` *inside* an inline shape is refused at the declaration, and `E0756` names the whole shape
  rather than the arm that caused it.** `json_reachable` strips a field's `null` arm only at the top
  level — `codec_field` does it before pushing the site, and the `Ty::Shape` recursion asks the
  written member type — so `{note: ?string}` reads as having no JSON representation while
  `?{note: string}` is fine. Write `rule:types/shape-type`'s optional column instead,
  `{note?: string}`, which is the independent question and is reachable.
  [until: gone crates/nvs-types/src/derive.rs:Ty::Shape(fields) => fields]
- **Widening `nvs_runtime::MethodRow` is about thirty edit sites, and a grep for the roster finds
  almost none of them.** It is a positional tuple in `nvs-ir`'s synthesized classes
  (`lower/closure.rs`, `lower/generator.rs`, which no search for `layout.methods` reaches) and a
  struct literal in two dozen test fixtures across five crates. Make the core edits, let `cargo
  build` enumerate the rest, and apply those as one `splice.py --patch` anchored on `param_tags:`
  plus the line under it — `grep -rn "param_tags:"` is the real roster.
  [until: gone crates/nvs-runtime/src/object.rs:pub struct MethodRow]
- **A `Node::Object` is spelled `{"$class":…,"$id":…,"$properties":{…}}` in JSON, so a producer whose
  wire shape has to be a plain object cannot use one.** `nvs_render::json`'s rule is that a kind JSON
  has no value for becomes a `$`-tagged one-key object, and `Node::Object` means *class instance*
  (`crates/nvs-render/src/lib.rs:340`), which a stack frame is not — a frame is `Node::Frame`, the one
  untagged exception, for exactly that reason. Read what each rendering writes before picking a node
  kind for a new producer: `grep -n "Node::Span" crates/nvs-render/src` finds every exhaustive match,
  and there are only the two. [until: gone crates/nvs-render/src/json.rs:AsNode]
- **A config field's `[unread:]` trailer is refused the moment *any* crate names the key, and
  `nvs_config::mode::DERIVED` counts as a namer.** Three gates in
  `crates/nvs-config/tests/directives.rs` fail together, the decisive one being
  `no_key_with_a_reader_still_claims_to_be_unread`. Grep the dotted key across `crates/` first;
  where a reader exists, describe the directive plainly and put what is still owed in the
  consuming crate's own `# Known gaps`.
  [until: gone crates/nvs-config/tests/directives.rs:no_key_with_a_reader_still_claims_to_be_unread]

- **A new `Core` row's parameters are swept from outside the class's module, once from another
  crate.** `registry.rs`'s `UNCLASSIFIED` is deletions-only, so a bare `CoreTy::Str` in an options
  bag fails `every_member_parameter_carries_a_qualifier_classification` and may not be listed beside
  the members already there — classify the bag `CoreTy::Text(Qual::…)`, which reclassifies everything
  sharing that slice. `crates/nvs-stdlib/tests/capability.rs`'s
  `an_open_file_is_an_object_and_never_a_resource` is the other: it sweeps every row for a
  `Core\IO\File` parameter, so such a member is green under `--lib` and red under
  `--test capability`. [until: reviewed 2026-09-16]
- **A member on `SOURCE_MEMBERS` shifts its *ABI* arguments and not its written ones, so a call-site
  check still counts from zero.** `Core\Metrics::increment` carries `args: [4]` for three declared
  arguments because argument 0 is the call site, but that constant is spliced in
  `nvs_ir::lower::expr` long after `nvs_types::intrinsics` has run, so an `Intrinsic` row addressing
  the name writes `at: 0` and a row that paid for the shift would read the argument beside it. Read
  the shift where it is applied — the lowering — rather than off the helper's `args: [N]`, which is
  the only place the widened arity is visible.
  [until: gone crates/nvs-stdlib/src/registry.rs:SOURCE_MEMBERS]
- **§ 6's "no conversion from text" compile error follows the `#[Option]` marker rather than the
  type**, so a *positional* `#[Command]` parameter at a type nothing converts compiles and throws
  when it is run. That is why `ArgConv::Unconverted` stays reachable however many conversions are
  built. Keep the conformance case for that throw positional: rewritten with `#[Option]` it stops
  testing the arm it names. [until: gone crates/nvs-types/src/commands.rs:Unconverted]
- **`nvs-types` depends on `nvs-hir`, so the checker's crate sits above name resolution, not below it.**
  A build that routes an earlier phase through something the checker owns — `require`'s path cooking
  through `nvs_types::string_lit` — is a dependency cycle Cargo refuses, not a call-site change, and the
  compile error names `and_then` or a trait bound rather than the cycle. Move the shared routine down to
  `nvs-syntax`, which `nvs-hir`, `nvs-types` and `nvs-ir` all already depend on, and leave a `pub use`
  where it was so no call site moves. [until: gone crates/nvs-types/Cargo.toml:nvs-hir.workspace]
- **A module doc's gap *number* is not a stable anchor, so a comment citing one drifts silently.**
  Striking a gap renumbers every item after it, and the citations elsewhere in the same file keep
  pointing at the old position — `crates/nvs-stdlib/src/json.rs` held five "this module's gap 1" that
  meant three different things. Cite the owning `# ` section by its title instead, and when you do
  strike a gap, grep the file and `docs/agent/goals/` for `gap [0-9]`.
  [until: reviewed 2026-09-16]
- **A member's name does not identify it: `WRITTEN_CLASS_MEMBERS` carries `queryAs` on
  `Core\Db\Connection` and on `Core\Request`.** A deferred check keyed on the method name answers one
  door's question about the other door's class, which is how `Core\Request::queryAs<SomeClass>` came
  to be refused with `E0806`. Key any classification of that roster on the owner —
  `nvs_types::derive::hydrates_a_row` and `reads_a_peers_octets` — and pin a new member with a case
  whose type argument is a **class**, since every case on those members wrote an inline shape.
  [until: reviewed 2026-09-16]
- **`nvs-server` cannot name a `Diagnostic`, so a boot reader that *refuses* cannot live there.**
  `nvs-diagnostics` is a dev-dependency of that crate and `nvs-config` re-exports no diagnostic type,
  so the natural `Connection::from_config(config, origins) -> Result<_, Diagnostic>` does not compile
  however obvious it reads. Put the keys, the parse and the refusal in `nvs-config` and hand the
  server crate the resolved overrides — `nvs_config::server::connection_bounds_for` plus
  `nvs_server::bounds::Connection::configured` is that split, and it also keeps the shipped numbers in
  one crate. [until: reviewed 2026-09-16]
- **A schema step's SQL is one statement per string, so an emitted `DECLARE … ; IF …` batch is wrong
  even where it is valid T-SQL.** `crates/nvs-db/src/direct.rs:142` is the contract — a driver takes a
  statement, never one string with several `;` in it. A step needing a value only the server holds
  writes one guarded statement instead, repeating its lookup rather than holding it in a variable:
  `crates/nvs-db/src/ddl.rs`'s `drop_default_constraint` is that shape.
  [until: gone crates/nvs-db/src/direct.rs:never one string with several]
- **A new item-producing function in `nvs-lsp`'s completion fails a test that names no arm you
  touched.** `every_completion_source_names_a_compiler_table` reads that module's own source and
  compares every function returning a `CompletionItem` against the `SOURCED` table beside it, so an
  arm added without a row fails in `crates/nvs-lsp/tests/completion.rs` rather than where it was
  written. Add the row — the function's name, and a needle from its body naming the compiler table it
  reads — and widen the array's declared length.
  [until: gone crates/nvs-lsp/tests/completion.rs:const SOURCED]
- **A serving core ends when `run_until_idle` reports `parked == 0`, so a task that parks forever on
  that scheduler wedges the shutdown.** `crates/nvs-cli/src/serve.rs:1229` loops until nothing is
  parked, so a receptionist parked on its bell there makes the count never reach zero and reads as a
  hung `nvs serve`. Give anything long-lived put on a serving core an ending of its own —
  `nvs_host::reactor::wake_at_drain` is the one this process already has.
  [until: reviewed 2026-09-17]
- **A published resolver is not an installed one, and a child staying `on: "here"` needs the
  second.** `nvs_runtime::script::publish` fills a process-wide slot a worker core reads as it
  starts, while the publishing thread still compiles through its own thread-local — so a `spawn
  script` path that falls through to the calling core answers `ResolveError::NoResolver` while the
  identical placed one succeeds. Wrap the caller in `SharedResolver::scoped`, as `nvs-cli` does for
  the core it booted on. [until: gone crates/nvs-runtime/src/script.rs:pub fn publish]
- **A `Core` class's registry row may carry a name that is a constant from another module, so grepping
  `registry.rs` for the literal class name finds nothing and the row looks absent.** `Core\Html\Markup`
  is `crate::html::MARKUP` in `registry::CLASSES` under `MARKUP_NAME`, which is
  `nvs_runtime::CARRIER_HTML_MARKUP` — a search for `Core\\Html\\Markup` in `registry.rs` returns zero
  hits, which nearly cost a filter over `CLASSES` the one class the `` html`…` `` constant needs. Grep
  the owning module for `pub const CLASS`/`NAME`, or `registry.rs` for `crate::<module>::`, and confirm
  membership there rather than by the class's spelled name. [until: reviewed 2026-12-01]
- **Changing what an `extern "C"` primitive owns breaks its Rust-side test callers as a use-after-free,
  not as a type error.** A test hands those primitives raw pointers, so nothing checks who owns the
  reference: a case that goes on using an operand the call now consumes reads freed memory, and it
  surfaces as a misaligned-pointer abort in an unrelated assertion. Grep every call in
  `crates/nvs-runtime/{src,tests}` before the edit and give each one a reference of its own —
  `string.rs`'s `lent` helper is the shape. [until: reviewed 2026-09-17]
- **A `Decided:` sentence's rationale clause can be false about the tree, and the decision still
  stands.** `crates/nvs-runtime/src/record.rs` gap 2 was answered "statically typed dumps already
  render the case", and nothing in the workspace builds a `nvs_render::Node::EnumCase` at all — the
  three renderings read the kind and no producer writes it. Build or strike the gap exactly as
  decided, and write the prose that replaces it from what you grepped, never from the rationale.
  [until: reviewed 2026-09-17]
- **Building a gap can make a `designed` rule owed the same session, and the gap register does not
  say so.** Striking `crates/nvs-runtime/src/lib.rs`'s "`nvs_safepoint` clears `COLLECT`" gap made
  `rule:observability/gc-pause-is-its-own-event` — a rule that had been unreachable prose because the
  collector did not exist — true of the routine that now does, and nothing in `owners.py`'s list
  pointed at it. Before closing a gap that builds a *mechanism*, grep `docs/rules/` for the mechanism's
  noun and check every `"status": "designed"` rule it hits: each is either owed in the same slice or a
  gap that needs an owner. [until: reviewed 2026-09-17]
- **No `Core` member anywhere may take a `Core\Xml\Node`, so a member that needs a node's context
  cannot ask for it.** `a_parsed_tree_has_no_path_back_into_execution` in
  `crates/nvs-stdlib/src/xml.rs` sweeps every registered member's parameters for the node class and
  for `Core\Xml`, and a tree is inert data on the same terms the AST is — so the obvious shape for a
  question about an element's surroundings, handing the document back in, is refused by a test two
  thousand lines away from the row. Resolve what needs ancestors while the tree is being built,
  where the scope is still known, and carry the answer on the node.
  [until: reviewed 2026-09-17]
- **A `?T` local does not narrow past the `if` that tested it, and `&&` narrows neither operand.**
  A guard clause — `if ($x == null) { echo "missing\n"; return; }` — leaves every statement after it
  still seeing `null|T`, and `if ($a != null && $b != null)` leaves both nullable inside the block,
  so a `.nvst` case written in the PHP habit fails `E0401`/`E0459` on its first run rather than at
  the line that looks wrong. Write `if ($x != null) { T $narrowed = $x; … }` and rebind once per
  value, nesting the `if`s where there are two, which also keeps the `--EXPECT--` block honest about
  interleaving. [until: reviewed 2026-09-17]
- **A `Decided:` sentence can name a site that cannot hold the check it asks for.** `test.rs` gap 2's
  answer was *assert it when a `Ctx` is built*, and there is no such moment: a context takes its
  exception class table after construction, through `nvs_codegen::Classes::install_in`, for the reason
  `Ctx::set_runtime_error_class`'s own doc carries. Build the check at the one place the omission is
  observable as a *wrong answer*, state the requirement where the contract already lives, and say in
  the handoff that the site moved — never quietly take the sheet's other option.
  [until: reviewed 2026-09-17]
- **One more element on `ClassLayout::methods`' row trips `clippy::type_complexity`.** That lint
  scores a written type at `10 × nesting` per node, so the five-element tuple sits at 220 against a
  threshold of 250 and a sixth `Vec<String>` puts it at 290 — `-D warnings` in a struct field, a `fn`
  signature or a `let`, but never in a `type` alias's right-hand side. Widen one of these rosters by
  naming the tuple, as `nvs_types::layout::MethodEntry` does, and re-export the name so a crate that
  does not depend on `nvs-types` can still write it. [until: reviewed 2026-09-17]
- **A refusal reported while parsing a type never reaches the reader at statement-initial
  declaration position.** `parse_stmt_maybe_local_decl` settles `T $x = …;` with a trial parse and
  reads *any* diagnostic as proof the tokens were an expression, so `3.14 $b = 1.0;` reports
  `E0101 expected ';'` and never `E0120`. Probe a new type-position refusal from a parameter or
  return type, where nothing backs out, and say in its doc comment that the statement head is the
  one position it does not reach.
  [until: gone crates/nvs-syntax/src/parser/stmt.rs:diags_len]
- **A `Core` row cannot land ahead of its helper.**
  `crates/nvs-stdlib/src/test.rs:2968`'s `every_row_names_a_symbol_this_module_claims` and
  `crates/nvs-stdlib/src/lib.rs:518`'s `every_registered_member_has_an_implementation_address` sweep
  every row in `registry::CLASSES` for an address, and
  `crates/nvs-stdlib/tests/conformance_coverage.rs:52` wants a case writing the member's own call
  spelling. Land the row, its card, its `address()` arm, its helper and one case as one group, and
  strike the ratchet line in the same edit. [until: reviewed 2026-12-01]
- **A spelling admitted at an argument position is refused by `nvs-hir` first, and that crate cannot
  ask the registry.** `Mailer::send` parses as a `ClassConstAccess`, so `nvs_hir::members`' walk
  reports `E0309` and the run aborts before `nvs-types` sees the call at all. Carve the argument out
  in `walk_args_admitting_method_ref`'s roster as well as marking it in `nvs_types`.
  [until: gone crates/nvs-hir/src/members.rs:METHOD_REF_ARGS]
- **`nvs_runtime::nvs_helper!` takes exactly one helper per invocation**, so a second `fn` written
  inside an existing block fails with `no rules expected #` pointing at that fn's own doc comment and
  at `$body:block` in `abi.rs`. Its one rule is `$(#[$meta])* fn $name(...) $body`, with no
  repetition around it, and the doc comment of the *second* fn is the first token it cannot match —
  which reads as a broken doc comment rather than as a block that should have been closed. Close the
  block after the helper you are writing beside and open a fresh `nvs_runtime::nvs_helper! {`.
  [until: reviewed 2026-09-18]
- **The enum a `Core` member returns is namespaced under the class, so `Env\Mode::Development` does
  not resolve and `Core\Env\Mode::Development` does.** The rules and `crates/nvs-config/src/tree.rs`
  write the type as `Env\Mode`, which is the language's name for it and not the path a program
  spells; the registry's own `MODE_NAME` at `crates/nvs-stdlib/src/env.rs:267` is
  `r"Core\Env\Mode"`, and `E0303 no matching declaration` is what the short spelling gets. Read the
  `*_NAME` constant beside the `CoreMethod` row before writing an enum case into an example.
  [until: gone crates/nvs-stdlib/src/env.rs:Core\Env\Mode]
- **An integer literal in a `??` fallback keeps its own `int`, so a `uint` binding refuses the whole
  expression.** `uint $page = ($q["page"] as ?uint) ?? 1;` is `E0401: expected `uint`, found
  `uint|int``, because the literal takes no hint from the `as ?uint` beside it. Declare the fallback
  as a binding of the target type — `uint $firstPage = 1;` — rather than writing `?? (1 as uint)`,
  which reads as a conversion nobody asked for. [until: reviewed 2026-09-19]
- **A shape `type` alias that names itself does not compile, and the diagnostic is `array type nests
  past depth 32`.** `type Node = {value: int, next: ?Node};` expands through the alias until the
  depth bound stops it and reports once per field, so a linked list or a tree written as a shape
  reads as a depth problem in an annotation nobody nested. Write the recursive case as a class;
  `crates/nvs-types/src/lower.rs`'s `lower_type_at_depth` § *Known gaps* carries the wording.
  [until: gone crates/nvs-types/src/lower.rs:array type nests past depth]
- **A diagnostic for `self`/`static`/`parent` added to `check_expr`'s own arm reports twice.**
  `nvs_hir::members` splits the three keywords by position — a bare one where a value is expected is
  its `E0321`, a class side goes through `walk_class_side` unreported — and both walk through
  `check_expr`, so an arm there doubles up on `mixed $x = self;`. Put the report at the four call
  sites that resolve a class side (`infer_class_const`, `infer_static_call`, the
  `StaticPropertyAccess` arm, `check_new_target`).
  [until: gone crates/nvs-hir/src/members.rs:walk_class_side]
- **A null test on the left of a `&&` does not narrow the operand on its right.** `$name != null &&
  Core\Str::length($name) > 2` is `E0401` at `$name`, because `rule:types/narrowing` is branch-local:
  the narrowing reaches the `if` body, not the operand written beside the test. Write the second
  check in a nested `if`, or reach the value through `?->` or `??` — and when a guard is only wanted
  for a throw, the ordinary `$count != 0 && $total / $count > 10.0` shape needs no narrowing at all.
  [until: gone docs/rules/types/narrowing.md:branch-local]
- **A nullable type is written `?T`, never `T?`, and the trailing form's seven errors name none of
  that.** `Throwable? $step = $chain;` reports `E0319: `Throwable` is not a constant that exists`
  first and then six cascading `$step is not declared` lines, so the whole report reads as a missing
  declaration rather than as a misplaced `?`. Copy the spelling from a case that already uses one —
  `tests/conformance/error/throwing-a-nullable-throwable-throws-the-object-it-holds.nvst:8` — rather
  than working back from the diagnostic. [until: reviewed 2026-09-19]
- **A `lateinit` property read inside the class that declares it is `E0428` unless a write came
  first on the same path, so the textbook injection shape does not compile.**
  `rule:classes/lateinit-read-before-write` runs the flow analysis per method, so a method that only
  reads `$this->printer` is refused however the caller set the object up, while the same read from
  outside the class is left to throw at run time. Put the read in the method that does the write, or
  outside the class. [until: gone tests/conformance/class/lateinit-refuses-a-read-before-any-write.nvst]
- **`echo a, b, c` writes each argument as it reaches it, so a throw in a later argument leaves the
  earlier text on stdout.** The arguments are evaluated and written one at a time, so a `try` block
  whose `echo` opens with a `"BAD="` label prints that label and *then* throws, and the expected
  output no longer matches. Put the call that can throw in its own statement and echo the variable.
  [until: reviewed 2026-09-19]
- **An array literal written in a `foreach` subject position binds `mixed`, so `foreach (["a", "b"] as string $x)` is refused as `E0401`.** A literal takes its element type from the declaration it is assigned to, and a subject position gives it none. Declare it first — `array<string> $queue = ["a", "b"];` — and iterate the variable. [until: reviewed 2026-09-19]
- **An example that prints `location` or `backtrace` freezes its own line numbers into the blessed
  `.out`.** `verify.py`'s `nvs-fmt` step rewrites proof files in place after you blessed them, and any
  later comment edit above the `throw` moves the number too, so the example goes red with nothing
  wrong in it. Bless such an example last, after `python tools/verify.py` has formatted the tree, and
  re-run `python tools/dossier.py --run examples --group <group>` after any edit above a `throw`.
  [until: reviewed 2026-09-19]
- **`Core\Arr::count` answers `uint`, and an `int` binding or a bare `0` beside one does not compile.**
  `int $n = Core\Arr::count($rows)` is `E0401: expected int, found uint`, and
  `uint $d = $ok ? ($v as uint) : 0` is `uint|int` because the literal is an `int` — both read as a
  mistake in the member rather than in the binding. Bind counts as `uint`, convert the other side with
  `as uint`, and write the zero as a `uint $d = 0;` on its own line before the `if`.
  [until: reviewed 2026-09-19]
- **`Core\Attributes::get`'s `$member` must be a *written* name; a computed one silently answers
  `null`.** A loop over `array<string> $names` reading one option per name prints nothing at all,
  while the same reads spelled as literals answer — `rule:attributes/structural-retrieval` decides
  it, since a written name is checked against the target's declarations and a computed one falls
  back to the empty result. Write the parameter name at the call site, and where a list is really
  wanted, build the array of results rather than the array of names.
  [until: reviewed 2026-09-20]

- **`Core\Attributes::get<S>(C::m(...))?->field` throws on a `null` answer instead of
  short-circuiting.** The member folds to a compiled-in constant, and a folded `null` does not reach
  the nullsafe test, so the program ends with `attempt to read property … on null` — while the same
  value bound to a `?S` local first, and a userland method declared `: ?S`, both short-circuit
  correctly. Bind the retrieval to a local before reading it — `open_nullsafe` in
  `crates/nvs-ir/src/lower/expr.rs` builds no guard at all unless the lowered receiver is
  `Ty::Tagged`, which a folded constant is not.
  [until: reviewed 2026-09-20]
- **A `Core` member whose registry row returns a union answers `mixed` in arithmetic, so a bench
  that adds it to an `int` does not compile.** `Core\Arr::sum`'s `return_ty` is
  `CoreTy::Union(NUMBER)` — `int|float|decimal` — and the result does not narrow to the subject's
  element type, so `$total + Core\Arr::sum($lines)` is `E0401: expected int, found mixed` over an
  `array<int>`. Write the cast the bench needs, `(Core\Arr::sum($lines) as int)`, and expect the
  same shape from `??` over a `uint`-valued member: `Core\Arr::max($widths) ?? 0` is `uint|int`
  and wants `?? (0 as uint)`. [until: reviewed 2026-09-20]

- **`echo` of a `float` prints the way PHP's default precision does, so an example cannot show
  float inexactness by printing one.** `Core\Arr::sum([0.1, 0.2])` prints `0.3` rather than
  `0.30000000000000004`, and a comment promising the long digits is wrong the moment `--bless`
  writes the `.out`. Show the difference with a comparison —
  `Core\Arr::sum($shares) == 0.3 ? 'yes' : 'no'` prints `no` — which is what
  `docs/examples/lang/expressions/arithmetic/03-a-basket-priced-in-decimal.nvs` already does.
  [until: reviewed 2026-09-20]
- **A `catch` binding belongs to the enclosing function or script rather than to the `catch`
  block, so two `catch`es in one program may not share a name.** `catch (ParseError $error)` in
  one `foreach` body and `catch (LogicError $error)` in the next is `E0406: $error is already
  declared`, which reads as a clash between two bindings that never overlap. Give every `catch`
  in a program its own name — `$badText`, `$badBase` — which a hostile case with a numbered step
  per attack needs several of at once. [until: reviewed 2026-09-20]
- **A `??` whose right side is a plain integer literal answers `mixed` when the left side is a
  `?uint`, and the diagnostic underlines the whole expression rather than the literal.**
  `Core\Bytes::indexOf(...) ?? 1` was refused as `E0401: expected 'uint', found 'mixed'` inside a
  `uint` sum, because the literal is an `int` and the two sides have no common type. Write the
  literal at the receiver's own type — `?? (1 as uint)` — or cast the whole `??` expression, which
  is what the `bool`-returning benches beside it already do. [until: reviewed 2026-09-20]
- **A keyed array's type names the value alone — `array<V>` — and `array<string, string>` does not
  parse.** A map literal `["EUR" => "Euro"]` is an `array<string>`, so writing the key type too
  stops the parser at the comma and cascades: one such declaration in a two-line example produced
  64 diagnostics, almost all of them about later lines. Write `array<int> $prices = ["cup" => 450];`
  and read only the first diagnostic. [until: reviewed 2026-09-21]
- **A ternary inside a `uint` sum types as `mixed`, so a chained accumulator needs an `if`.**
  `$total = $total + ($form == Core\Cldr\PluralCategory::Other ? 1 : 3)` fails twice, `E0407`
  mixed-signedness and then `E0401` expected `uint`, found `mixed`, because the two branches are
  `int` literals and the ternary never narrows to the `uint` on the left. Write a `uint $step = 1;`
  with an `if` above the sum instead, which is also what keeps the member's own answer in the chain
  a bench and an attack are built around. [until: reviewed 2026-09-21]
- **A `uint` in an example does not survive `/`, and a `Core\Str` offset is an `int`.** `$room * 40 /
  100` types as `uint|float`, and an `as uint` on it then throws at run time the moment the quotient
  has a fraction, while `Core\Str::slice`'s `offset` and `length` are `int` and `int|null`, so a
  width or a length held as `uint` is refused at the call. Write an example's arithmetic with `-`
  and `%` only, and cast at the call site with `as int` rather than declaring the local `uint`.
  [until: reviewed 2026-09-21]

## Divergences and refusals already pinned

- **A PHP warning printed to stdout makes an oracle leg unusable, so an `--ORACLE--` case never
  echoes a `NAN` or a `chr()` outside `0..255`.** PHP 8.4 and later warn when coercing `NAN` to a
  string and 8.5 deprecates `chr()` past 255, and the notice lands in the compared output. Otherwise
  a float may be echoed directly: Novis's rendering is PHP's, precision 14 with trailing zeros
  trimmed, so `sqrt(2.0)` prints `1.4142135623731` on both sides. [until: reviewed 2026-09-06]
- **An `--ORACLE--` case cannot call `mb_*`: the `php` on the Windows `PATH` has no `mbstring`.**
  `Core\Str` counts grapheme clusters (`nvs_stdlib::granularity::DEFAULT` is `Unit::Grapheme`) where
  `substr` counts bytes and `mb_substr` code points, and the three coincide only inside ASCII with
  no carriage return. Put the ASCII agreement in one `--ORACLE--` file and the multibyte split in an
  `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--`. [until: reviewed 2026-09-06]
- **PHP's four `str*cmp` twins disagree with each other on magnitude, so an oracle over
  `Core\Str::compare` needs a sign normalization first.** `strcmp`, `strnatcmp` and `strnatcasecmp`
  answer -1/0/1 where `strcasecmp` still returns the byte difference, while `compare` is always one
  of three literals. Wrap each PHP call in a sign function before comparing; `compare` is also the
  only ordering two strings have, since `<` over two `string`s does not lower.
  [until: reviewed 2026-09-06]
- **A class carrying `#[Json\Derive]` and declaring no field has an empty codec, so
  `Core\Json::encode`/`decodeAs<T>` refuse it with the sentence saying it does not carry the
  attribute at all.** The message is wrong about why, and whether a fieldless class should encode as
  `{}` instead is the open question behind it. Do not pin that sentence in a case over a fieldless
  class. [until: reviewed 2026-09-06]
- **An infallible allocation on a caller-supplied count aborts the process — exit 127, nothing on
  stderr, nothing catchable, every in-flight request with it.** `nvs_runtime::affordable` refuses
  only a size past `isize::MAX`, so a count below it the machine cannot serve reaches `vec![…; n]`,
  `String::with_capacity` or `NvsStr::build` and dies. Draw through `NvsStr::try_build`,
  `NvsArray::try_reserve` or the `*_fallibly` helpers in `str.rs`/`bytes.rs`/`arr.rs`, and add the
  member to `tests/conformance/core/count-shaped-producers-refuse-alike.nvst`.
  [until: reviewed 2026-09-06]
- **A JSON round-trip text comparison against PHP measures the writers, not the readers, unless
  every float carries a fraction.** `Core\Json::encode` keeps a whole-valued float's fractional part
  — `0.0`, `100.0`, `1e+308` — where `json_encode` writes `0`, `100`, `1.0e+308`. Separately,
  `Core\Json::decode` refuses the band `i64::MAX`+1 ..= `u64::MAX` that `json_decode` widens, pinned
  by `json-decode-refuses-the-number-band-json_decode-degrades`. [until: reviewed 2026-09-06]
- **`0 - $x` at `int`'s smallest value wraps back to itself in silence**, which is why
  `Core\Math::abs` is a member and not sugar for `max($x, 0 - $x)`: the composition is total at that
  row and quietly wrong, while `abs` refuses. A case asserting the divergence asserts that the
  derivation *answered* and `abs` *refused*, never what the arithmetic wrapped to — the overflow
  policy is another member's question and pinning it here fails the case for the wrong reason.
  [until: reviewed 2026-09-06]
- **The ordering refusal is one throw in one wording, and a case only reaches it through `mixed`.**
  `Core\Math::min("a", 1)` never runs — `T` unifies at the first argument, so the second is `E0401`
  — so both halves of an orderless pair are laundered through `mixed` locals, and an array through
  `array<mixed>`, before `nvs_stdlib::ordering::compare_values` sees them. All seven members then
  raise the same `Fault::thrown`, `<member> has no natural order for tag N against tag M: …`,
  differing only in the name in front. [until: reviewed 2026-09-06]
- **`continue 2` inside a `switch` is PHP's idiomatic spelling, so counting `continue N` over loops
  alone silently breaks ported code.** Skipping `switch` frames compiles and passes every test, then
  rejects `foreach { switch { case: continue 2; } }` and, with two nested loops, retargets PHP's
  inner loop to the outer one. Count every frame as PHP does, then walk outward from the frame the
  level lands on to the nearest loop;
  `tests/conformance/lang/a-break-leaves-the-level-it-names.nvst` pins it.
  [until: reviewed 2026-09-06]
- **Check every spelling against `php -r` before deciding a family is refused, because PHP does not
  always agree with itself.** `echo $a[];` and `unset($a[])` are compile errors, but `$a[] .= "x"`
  appends silently even at `error_reporting=-1`, so refusing the third is a divergence that needs
  its own row under `rule:php-migration/every-divergence-is-deliberate-and-listed`, not a
  doc-comment sentence. A probe under `.agent-tmp/` run through both `nvs run` and `php` settles a
  paragraph before it is written. [until: reviewed 2026-09-06]
- **Turning a panic into a diagnostic breaks the tests that pinned the panic, and they do not look
  like your change.** A `#[should_panic(expected = "known gaps")]` guard fails with *"panic did not
  contain expected string"* while printing the new diagnostic, and a fixture helper's own
  `assert!(!diags.has_errors())` fails without naming the feature. Delete the guard — the `.nvst`
  case is its replacement — and split the fixture helper so the one test whose point is the
  diagnostic gets the `Diagnostics` back. [until: reviewed 2026-09-06]
- **A new refusal in `check_expr` fires before `check_write_target`, so it double-reports every
  receiver that already has a better code.** `$erased->rows["0"] = "z"` printed `E0482` and `E0480`;
  the ordering is fixed because `check_write_target` reads the `ExprInfo` the target's own check
  records. Suppress with a lookup the early arm can already do — the chain root's recorded
  `HookedProperty`/`ShapeProperty`, or a syntactic `nullsafe: true` — gated on the level being an
  assignment target, so a plain read keeps its only diagnostic. [until: reviewed 2026-09-06]
- **A standing case can pin the error *class* a refusal arrives in, so a member whose classification
  you change fails `verify.py`'s conformance leg and not its unit tests.** The case's own reasoning
  paragraph is why it pinned the class, and it can be true and beside the point —
  `Core\Time::parse`'s zonal-pattern refusal was argued into `ParseError` that way. Read the
  reasoning before reclassifying, and if it is what is wrong, rewrite that paragraph rather than the
  expectation alone. [until: reviewed 2026-09-06]
- **A `Fault::fatal` site behind an `array<T>`, `array<array<string>>` or `uint` parameter is
  unreachable from source and is owed no case.** A `mixed` subject, a `?array<string>` and a `mixed`
  count all stop at the checker as `E0401`, and `rule:types/conversion`'s `array<T> as array<U>`
  does not lower, so `Core\Test::assertCount`'s two sites and `Core\Csv::format`'s column guard
  cannot be reached. Three `nvs run` probes on a scratch file settle it in one call; the assertion
  members read as if a bad subject were a runtime question, and it is a signature question.
  [until: reviewed 2026-09-06]
- **`grep -r --include=<glob> .` through the Bash tool walks this tree and silently finds nothing**,
  returning exit 0 with no output, so a miss reads as a clean answer: it found 0 hits for a pattern
  ripgrep found across files it had just been pointed at by name. Use the Grep tool for any
  repo-wide question whose answer you are about to write down; a shell `grep -n` on one named file
  is still fine. [until: reviewed 2026-09-06]
- **A name-spelling change has three corpora, not one, and the third only fails at the very end.**
  `.nvs`/`.nvst` fixtures hold the spelling literally, Rust fixtures hold it escaped
  (`"#[\\Core\\Route]"`, two bytes per separator), and a few hold it in a raw string
  (`r#"#[\Core\Route]"#`, one byte), so a regex for either of the first two matches nothing in the
  third and one test in one crate fails last. Sweep for the single-byte form after the escaped one
  and check the hits are only doc comments. [until: reviewed 2026-09-06]
- **A refusal of a shape the language used to accept has five homes, and two of them are tables.**
  The code in `crates/nvs-diagnostics/src/lib.rs`, the report site, a `tests/conformance/reject/`
  case, a row in `docs/reference/tools/30-php-differences.md`, and a `divergesFromPhp` note on the
  owning rule, which `python tools/rules.py --render` writes into `docs/divergences.md`. Then
  `python tools/reference.py` regenerates `docs/novis.md` and proves its examples, and any test that
  pinned the old rule is rewritten to the new one rather than deleted. [until: reviewed 2026-09-06]
- **A `docs/reference/lang/` example is executed, so a claim can be pinned instead of asserted.**
  `tools/reference.py` runs every ` ```nvs ` fence against the binary and checks the ` ```output `
  fence after it; ` ```nvs error ` is the fence for a program that must fail `nvs check`, its
  `output` block a substring of the diagnostic. A chapter edit therefore means `python
  tools/reference.py` to regenerate `docs/novis.md` in the same slice, or the generator's `--check`
  fails the verify. [until: gone tools/reference.py:--check]
- **A class reference has no readable name: `$cls::class` is `E0702`.** A `class<T>` value is the
  run-time descriptor and `::class` needs a class named at compile time, so `nvs-cli` reports *"this
  names no class the compiler can resolve"* and points at `Core\Reflect`
  (`rule:classes/no-free-functions-or-constants`). A `.nvst` case that wants to show which class was
  selected calls an overridden member and reads the answer, as the class-reference cases under
  `tests/conformance/class/` do. [until: reviewed 2026-09-06]
- **An interactive session beside the running loop cannot link `nvs-cli`, and the error names a file
  rather than a cause.** `cargo build` stops at `failed to remove file <repo>\target\debug\nvs.exe`
  / `os error 5` because Windows will not replace a running image and the loop's `.nvst` trees run
  one for minutes. `Get-CimInstance Win32_Process -Filter "Name='nvs.exe'"` shows it is live work —
  PIDs change between two calls — so retry rather than kill; the same collision fails
  `log::tests::the_engine_floor_rotates_and_rate_limits_itself` on a shared rotation path.
  [until: reviewed 2026-09-06]
- **A new directory under `benches/` breaks the whole workspace, and the error names a crate you
  never wrote.** `Cargo.toml`'s `members` glob is `benches/*`, so a non-crate directory there makes
  every `cargo` command fail with *"failed to read `…/benches/<dir>/Cargo.toml`"* — and nothing
  fails until the next `cargo` invocation, which may be a container later. Add it to the `exclude`
  list beside that glob, and run `cargo metadata --no-deps` as the cheap check.
  [until: gone Cargo.toml:benches/*]
- **On Git Bash, an absolute path in a `docker` argument is rewritten to a Windows one before docker
  sees it.** MSYS path conversion rewrites any argument that looks like a POSIX path, so `docker run
  --rm img cat /etc/debian_version` reads `C:/Program Files/Git/etc/debian_version`, and `docker
  exec`, volume flags and container-side commands all suffer it. `MSYS_NO_PATHCONV=1` in front of
  the call is the whole fix; the tell is an error naming a path under `C:/Program Files/Git/` you
  never typed. [until: reviewed 2026-09-06]
- **`docker compose build | tail` reports success for a failed build.** The pipeline's status is
  `tail`'s, so the exit code is 0 and the failure is only in the text scrolled past — AGENTS.md's
  `;`-chain rule wearing a different hat. Redirect to a file and echo `$?`, or read the status
  before the output. [until: reviewed 2026-09-06]
- **php-fpm interpolates no environment variable in a pool file, and the official `php:8.5-fpm`
  images cannot load `zend_extension=opcache`; both fail silently.** `pm.max_children = $N` fails
  the whole configuration with *"Unable to include"* and no key named, so
  `benches/proxied/php/pool.conf.in` is rendered by the driver; OPcache is linked statically, so the
  settings apply and only stderr says *"Failed loading Zend extension"*. Prove an ini took with `php
  -d opcache.enable_cli=1 -r '…opcache_get_status()…'`, since `enable_cli=0` makes the obvious check
  return `false`. [until: gone benches/proxied/php/pool.conf.in]
- **`toolchain: none` is not how `dtolnay/rust-toolchain` honours `rust-toolchain.toml` — the action
  has no such case.** It hands the literal word to `rustup toolchain install`, which answers
  *"invalid value 'none' for '[TOOLCHAIN]...'"* and takes every Rust job on every platform down in
  seconds. A bare `rustup toolchain install --no-self-update` is the honest form: the argument
  defaults to the active toolchain, so the file stays the one home for version, components and
  targets. [until: reviewed 2026-09-06]
- **A red CI job is not automatically a regression — ask whether it has *ever* been green.** A leg
  failing since the commit that introduced it reads as "something recent broke this" and buys a
  confident wrong suspect, and on a short CI history `gh run list` cannot answer the question at
  all. `git log -S "<a flag only that job passes>" -- .github/workflows/ci.yml` finds the commit
  that added the job; run the failing test there first, not last. [until: reviewed 2026-09-06]
- **A `#[cfg(feature = "…")]` module inside a test file is compiled only by the CI leg that turns
  the feature on.** `perf_guards.rs`'s `wasm_guards` used a helper from its parent without importing
  it and nothing local noticed: `cargo build --all-targets`, `verify.py` and clippy all compile the
  file with the module cfg'd out. When a job whose name mentions the feature fails to compile a file
  that has been green for months, check the module's own `use` list first.
  [until: gone benches/abi-probe/tests/perf_guards.rs:wasm_guards]
- **A generated file can be current on the machine that wrote it and stale everywhere else.**
  `docs/novis.md` embeds `nvs meta --json`, which answers for the platform the binary was built on,
  so a platform-varying constant such as `Core\Env::OS` makes a Windows-generated file stale on CI
  while `reference.py --check` passes locally and says only *stale*. Pin the constant to one
  spelling in `PLATFORM_VALUES` in `tools/reference.py`, or regenerate under WSL with a Linux `nvs`
  in `target/debug/` and read the `git diff`. [until: gone tools/reference.py:PLATFORM_VALUES]
- **A compiled method's argument array starts with a receiver, so a hand-built call needs `1 +
  arity` slots.** Slot 0 is `$this` or the called class descriptor and the first declared parameter
  is slot 1 (`nvs_ir::lower`, `nvs_runtime::NvsFn`), so a Rust caller passing only the declared
  arguments reads one `Value` past its slice with no diagnostic — the tell is an answer that is a
  neighbouring argument or a plausible zero. Call through `nvs_codegen::Unit`'s `call_static`,
  `call_on_new_instance` or `build_fixture`; `raw_function` is for `benches/abi-probe`'s timed loops
  only. [until: gone crates/nvs-codegen/src/lib.rs:fn raw_function]
- **A sanitizer changing an answer does not mean the sanitizer is involved.** ASAN turned an
  out-of-bounds slot read into `Some(0)` on the asan leg alone, which read as an ASAN-specific ABI
  fault; it was an ordinary off-by-one that ASAN merely made deterministic by poisoning the slot
  past the array. Before theorising about instrumentation, print what the callee actually received
  for several distinct non-zero arguments. [until: reviewed 2026-09-06]
- **`static::` anything inside a closure body panics in `Lowering::lsb`** — `nvs-ir: … names
  `static` but has neither a receiver nor a called class`. A closure is lifted to its own
  `Class$fnN::invoke` frame that captures neither `$this` nor the called class, and the checker that
  `lsb`'s panic message says should have refused it does not. Whichever way it is closed, the first
  test is a closure inside both a `static` and an instance method, because those frames carry the
  descriptor in different slots. [until: reviewed 2026-09-06]
- **A test that reads `nvs_codegen::disassemble` is x86_64-only until it says which backend it
  means.** Cranelift prints the vcode of whichever backend it emitted for, so a scan for lines
  beginning `call ` counts zero on aarch64 (`bl 0`, `blr <reg>`) — a structurally empty answer that
  a "no more calls than sites" guard passes while looking at nothing. Anything walking calls or
  branches out of the disassembly wants a `target_arch` gate, as `perf_guards.rs`'s `path_calls`
  has; macos-aarch64 is in the test matrix.
  [until: gone benches/abi-probe/tests/perf_guards.rs:path_calls]
- **`std::env::temp_dir()` in a test whose path reaches `nvs_config::trust::check` is refused on
  Unix.** `/tmp` is mode 1777, `rule:config/ownership-is-the-trust-boundary` refuses a group- or
  world-writable directory, and the check runs on the directory *and its parent* — so a tight
  scratch directory of your own is still refused, with a `Breach` naming a mode you did not set.
  Scratch beside the test binary instead (`std::env::current_exe()`'s parent, under `target/`);
  Windows hides this entirely because its temp dir is per-user. [until: reviewed 2026-09-06]
- **A test that drives an accept loop must loop on `report.parked` rather than call
  `nvs_host::run_until_idle` once.** That call returns the moment one blocking poll wakes nothing,
  and readiness collected for a task that has already ended is the ordinary way that happens — so a
  core that is still accepting stops at whichever stale wake came first, leaving the client with no
  answer and no end of file. Whether the platform reports that readiness at all is the platform's,
  which is why this is a windows-x86_64 failure against two green legs; drive with `serve.rs`'s
  `run_the_core`, which is `nvs-cli`'s worker loop with a deadline on it.
  [until: gone crates/nvs-server/src/serve.rs:fn run_the_core]
- **A rulebook edit leaves the website's generated pages stale, and `verify.py` does not look at
  them.** `docs/rules/*.md`, `ground-rules.md` and `divergences.md` come from `python tools/rules.py
  --render`, but `website/src/content/docs/docs/rules/` and `website/src/data/rules.json` come from
  `npm --prefix website run sync:rules`, which nothing in the verification pipeline runs. Expect that
  sync to also rewrite chapters you never touched — it catches up every status change landed since
  the last one — so commit the catch-up separately from your own change.
  [until: gone website/scripts/sync-rules.mjs]
- **A refusal's wording can be pinned as a substring by a test far below it in the same file.**
  `no_dialect`'s SQLite arm in `crates/nvs-stdlib/src/queue.rs` was held to the phrase "the queue has
  no statements for it yet" by `the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send`, so
  an edit making the sentence *more* true failed the build. Grep a refusal's distinctive phrase
  before editing it, and when the assertion pins wording rather than the fact under it, move it to
  the fact. [until: reviewed 2026-09-10]
- **Lifting a refusal turns a conformance case red that no check in the goal names.** `mixed as Core\Uri`
  was pinned as `E0711` in `an-object-target-naming-no-class-cannot-be-converted-to.nvst`, so widening
  the conversion roster invalidated that case's whole `--EXPECTF-ERROR--` block — help text and `%s:NN`
  anchors included, for the rows that still refuse too. `grep -rn` the diagnostic code and the target's
  own spelling across `tests/conformance/` before widening any acceptance.
  [until: reviewed 2026-09-17]
- **Respelling PHP's class-test operator as `is` can falsify the sentence around it, not just the
  word in it.** `crates/nvs-stdlib/src/debug.rs` and `docs/rules/errors/debug-dump.md` both argued a
  bound from "a program cannot act on that union: a class test against a `Core` class is `E0496`",
  which `is` makes untrue — `Core\Cli\Text|Core\Html\Markup $r = …; if ($r is Core\Html\Markup)`
  compiles and
  narrows. Run the two lines through `target/debug/nvs.exe run` before swapping the word, and rewrite
  the claim in the rule as well as in the comment. [until: reviewed 2026-09-18]
