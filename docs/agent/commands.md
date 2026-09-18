# Commands — how this repository is driven

Everything an agent runs, and the two rules about *how* to run it that have cost real sessions real time.
[AGENTS.md](../../AGENTS.md) points here rather than keeping a copy; `python tools/orient.py` prints the
short form of the two rules at the top of every loop session.

## A shell never carries file content into the tree

Create and edit files with the Write and Edit tools — never a heredoc, a `>` redirect or a `sed -i` that
writes a file. This is not a style preference. A shell tool call is one `-c` string that the shell *parses*
before it runs anything, so an apostrophe in a doc sentence, a backtick in a commit message or an
unbalanced heredoc terminator fails the whole call with `unexpected EOF while looking for matching '` —
the command never executed, and nothing tells you which quote was at fault. The dedicated tools pass
content as JSON parameters with no shell in the path, so that failure cannot occur. This repo makes the
problem worse than most: prose full of apostrophes, backtick-quoted identifiers everywhere, and two shells
with incompatible quoting grammars (PowerShell primary, Git Bash for the Bash tool).

**Reading and searching are a preference, not a prohibition.** Prefer Read, Grep and Glob: they need no
quoting, they return line numbers you can cite, and Grep takes a real regex without a shell mangling it.
A `grep`/`sed -n` through a shell is allowed where it is genuinely shaped better — a pipeline over a
*command's* output, a `wc -l` across a glob — because nothing is being written and a bad quote costs one
retry rather than a silent wrong edit.

**What you read is the budget, so read a big file in the region you need.** A loop session must finish
under a fixed **200k** of context, and `python tools/orient.py` is built to start it at well under 20k. One
`cat` of a 1,100-line module spends a tenth of the remaining budget in a single call. So: whole file when
it is small (roughly under 400 lines, or when you will touch most of it), otherwise `grep -n` for the
anchor and read the region around it. This is the one place turn economy and context economy pull against
each other, and context is the constraint that degrades the work rather than merely slowing it.
`python tools/loop-stats.py` prints where the last sessions actually landed, and
`python tools/loop-stats.py --attribute` prints which *reads* put them there.

Use a shell for what it is for — `cargo`, `git`, `python tools/orient.py`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

**An Edit the tool cannot express — and any run of three or more edits — goes through `python
tools/splice.py --patch <file>`.** Write one patch file under `.agent-tmp/` (gitignored; create it if it
is not there) with the Write tool: a `--- <path>` line, then that file's blocks between `<<<<<<< OLD` /
`=======` / `>>>>>>> NEW` markers, then the next `--- <path>`, for as many files as the edit spans. It
refuses anything but exactly one match per block, and **the whole patch applies or none of it does**, so a
stale anchor in the last file cannot leave the first one half-edited. Never reach for a
`sed`/`python - <<'PY'`/`cat > f <<'EOF'` one-liner instead: a heredoc is a shell string, so it eats the
backslashes and apostrophes this repository's Rust and prose are full of. The exact format is in
[conventions.md](conventions.md).

**The count is the point, not the difficulty.** One or two edits are cheaper as plain `Edit` calls — a
patch costs a Write plus a call, so it only wins from three. Past that it wins by a lot: measured over a
33-session run, 10.3 turns a session went on `Edit` calls issued **back to back with nothing read between
them**, in runs of up to 17, and each of those runs was one decision the model had already made, spent one
round trip at a time. That is the same saving `peek.py` takes on the read side, for the same reason.

**The plan's status block is edited with `python tools/plan.py --set "<field>" --from <file>`**, not by
hand — locating a field's exact bytes and splicing them was the single most expensive repeated action a
session performed.

**A goal's playbook selection is chosen with `python tools/playbook.py`, not by reading the file.**
`--goal` ranks every bullet against the goal's own `[context] modules` and prints a paste-ready
`playbook = [...]`; `--match <paths>` does the same for one session's file set. Two flags are the pruning
signals an append-mostly file has beyond the `[until: ...]` trailer every bullet ends with -- the syntax is
`playbook.py`'s module doc; `--check` tests the trailers and `--retire` deletes the bullets whose condition
holds. `--check` also reports a bullet naming a path that has left the tree,
and `--dupes` reports a bullet that already says what another bullet says — one trap had been written down
six times, by six sessions, in six wordings, before anything could see it. Neither appends to the playbook, and `--retire` only ever removes —
the bullet, and every goal manifest's `playbook` line that resolved to that bullet alone, so the floor's
`chain.py --check` never meets a selector reaching nothing: appending a bullet is `session.py`'s
`## playbook:` section and stays there.

`--check` **exits non-zero on exactly one of the things it prints**: a selector that does not resolve to
exactly one bullet. `orient.py` fetches a trap by that string, so a shared lead-in is a bullet the loop
cannot deliver to the session whose goal named it, and neither end reports anything. The stale paths and
the sizes beside it stay reports — a quoted path is often gone *because* the trap was closed. CI's `docs`
job runs it for the one finding.

**The plan is an index and one file per milestone**, and `plan.py` is the only thing that needs to know
which is which: `--show M8` prints one milestone, `--show M8:verify` its acceptance paragraph alone, and
`--amend M8 --from <file>` rewrites one. `--check` prices the status block against the aim the plan's own
comment states and reports an index row that has drifted from the file it names. It refuses to *add* a
field or a milestone — both are decisions, not forms — and it never refuses over a length.

**It does refuse over a structure**, which is what makes it a CI gate: a table row that does not parse, a
row naming a file that is not there, a title drifted from its H1, a milestone file no row names, a
milestone with no `**Verify:**`. An unparseable row used to be *skipped*, so a milestone simply left the
roster and every reader downstream saw the shorter table as the truth. The title cell is derived — the
milestone file's H1 is its one home — so **`python tools/plan.py --sync` writes that column** rather than
a reader picking whichever of the two copies looked right; it touches nothing else, since Order and
Loop-days are the index's own data, and it refuses on a table it cannot read whole.

The rulebook is the same arrangement one tree over: `python tools/rules.py --render` writes
`docs/rules/<topic>.md`, `docs/ground-rules.md` and `docs/divergences.md` from the topic JSON and the
fragments, which own every line, and `--check` reports a rendered copy that has drifted. The decision
records under `docs/decisions/` are frozen and derive nothing by hand: `python tools/records.py --check` is
their audit, `--stats`, `--graph NNNN` and `--orphans` its readings. This is the pattern `docs/novis.md`
already established below — derive the machine-derivable half, and let a `--check` fail when the
committed copy stops agreeing with it.

**The chain is edited with `python tools/chain.py`, never by hand.** `docs/agent/goals/` *is* the order
the driver walks — a goal is `N-<slug>.md` plus a sibling `.toml` and `.handoff.md`, the numbers run
`1..N` with no gaps, and walking the chain is sorting on the number. So **inserting or moving a goal
renames files**, and it is never one file: `--new <slug> --after N` scaffolds the three — with the
predecessor's `playbook`, `plan`, `[valgrind]`, `[wsl]` and `[docker]` blocks copied forward *as text*,
so their comments survive, and everything that is this goal's own substance left as marked `TODO` — and
renumbers every goal from the landing position on so `1..N` still holds. `--move N --to M`
(or `--after`, `--before`, `--next`) and `--remove N --delete-files` are the same operation, the second
closing the hole its number leaves. `--set N --milestone`, `--retitle N --to <slug>` and `--renumber`
are the rest; no flag at all lists the order with where the run stands, and `--check` is the gate.

**What survives a renumber is what is not written as a number.** Prose names a goal by its slug
(AGENTS.md, *The schedule is the chain*), so the only text a move rewrites is the goal's own two file
headers and the link targets that are filenames. `--check` reports any `goal 29` written into prose,
because the day one exists is the day a renumber starts lying about it.

**`--retire N` is what a walked goal ends as, and the driver runs it at every switch.** The fold is
cumulative — goal `core-depth`'s checks are in goal `concurrency`'s file and in every file after it —
so once the run has left a goal, its own `.toml` is a copy of a copy, and six of them were 830K that no
tool reads and every `grep` over `docs/` hits eight times. This deletes that `.toml` and its
`.handoff.md`, and **that deletion is the whole record**: retirement is the `.toml` being gone, so a
flag and the disk can no longer disagree about it. The `.md` stays, because it holds the goal's number
and the prose the plan cites. It refuses unless **every** `[[check]]` of that goal is in the live
`loop-goal.toml`, matched on `(kind, name)` so a floor a session legitimately edited still counts — and
that proof, unlike the position guard, is not `--force`-able.

Every mutation is a **text splice**: half of that file is the prose saying why the order is what it is,
and a `tomllib` round-trip would delete all of it. `--check` re-renders the file it just read and reports
it if that ever stops being byte-identical, alongside the two failures that are otherwise silent — a goal
TOML with no `goal-switch` marker line (which makes the switch *into* that goal refuse, stopping the run)
and one with no `files`/`[valgrind] skip` key for the floor to be unioned into.

One thing it refuses: **a goal at or before the live one**, whether it is the thing being edited or the
place something is landing. `goal-switch.py` has already folded each walked goal's checks into the one
after it, so moving, renumbering or removing anything back there invalidates a floor that is already
built, and nothing downstream notices. `--force` is there for a tree where the run is over or was never
started.

## One shell call runs one command, and its exit status is the last one's

Do not `;`-chain several probes into a single call. A chain reports only the final command's status, so a
probe that is *allowed* to fail — `ls` on a directory that may not exist returns 2 — marks the whole call
failed while holding a complete result. `2>/dev/null` does not help: it suppresses the message, not the
status. When a command may legitimately fail, either give it its own call or end it with `|| true`, and put
a `&&` between steps that genuinely depend on each other.

**That is a limit on one call, not on one turn — independent calls go out together.** Issue every probe
whose input does not depend on another's result as several tool calls *in the same message*: four greps
locating a symbol, a Read of two files you already know you need, `git status` beside `cargo --version`.
They run concurrently and each keeps its own exit status, so nothing about the rule above is weakened.
This matters more than it looks. A turn's **time-to-first-token is ~80% of its clock and does not depend
on what the turn does** — median 3.7s, and it climbs with context, 3.4s at 50k to 5.3s at 200k. So a
session's wall clock is very nearly its turn count times a constant, and merging two calls into one
message is a saving taken whether or not either call was expensive.

**But do not rely on remembering to: reach for the tool that batches for you.** Over one measured run of
39 sessions, **0 of 3,647** tool-call messages carried more than one call — against the rule in the
paragraph above, which every one of those sessions had in its context, and including runs of 52 and 57
consecutive `grep`/`sed` calls. A later 33-session run measured the same 1.00 calls per message. A rule
that loses 3,647 times is not a rule anyone is going to start following, so both sides of the work have a
tool instead: `tools/splice.py` for a run of edits, and `tools/peek.py`, which takes as many targets as
you have questions and answers them in one call:

```sh
python tools/peek.py crates/nvs-ir/src/lower/expr.rs:3065-3120 \
                     crates/nvs-types/src/expr/members.rs:@public_property_names \
                     rule:types/conversion \
                     docs/decisions/0036.md:"### 4" \
                     "crates/nvs-runtime/src/*.rs:re:slot_get"
python tools/peek.py --locate nvs_object_slot_get SlotSet ClassDesc   # file:line, no bodies
```

Locators are `120-160`, `120+30`, `@symbol`, `re:pattern` (which prints the matching line alone —
`re:pattern:3`, or `--context 3` for every target in the call, is how it comes back with the block
under it), `"## Heading"`, or nothing for a whole small file — and the path may be a glob, which is
how one call sweeps a crate. Prefer `re:` to the `/pattern/` spelling: Git Bash rewrites a leading `/` into a Win32
path before the tool sees it. `--locate` is what a handoff's `file.rs:NN` anchors are made of.

**A `rule:` citation is a target on its own** — `rule:types/conversion`, backticks and all if you
pasted it out of a doc comment — and it answers with that rule's fragment, which is the rule. The
token is the path (`docs/rules/types/conversion.md`), so nothing is looked up and the miss names
`rules.py --list`. There are about 18,500 of these tokens in the tree, which makes id-to-text the
most frequent lookup there is here; batch it beside the code you were reading anyway rather than
translating it by hand.

**And when you do not know the file yet, read its seams before its lines.** `python tools/peek.py
--outline <path>` prints one line per `fn`/`struct`/`enum`/`trait`/`impl`, with where it starts and how
far it runs; every line of it is a `:@name` target that then lands first time. This is what the re-fetch
number argues for: **56% of a session's read calls fetch a file the session had already opened** — 24.6
calls a session across only 20.3 distinct files — and since the two hottest files in this repository are
5,044 and 5,720 lines, "read it whole" was never the alternative. An outline of `lower/mod.rs` is 2.8 KB
against the file's 276 KB. `peek.py` counts a session's fetches per file and names the flag once you have
reached into the same one three times.

**The 400-line floor is enforced on both sides now.** `peek.py` has always refused a bare `path` over
`--max-lines` — 400, which is AGENTS.md rule 3's "whole file under ~400 lines" and the one home for that
number. The harness's own `Read` tool had no such floor, and it is where the largest single tool result of
a 72-session run came from: 35,040 bytes of `serve.rs` in one call, with 27 whole-file reads of the 177 KB
`loop-goal.toml` behind it. `tools/guard-read.py` is a `PreToolUse` hook, wired in `.claude/settings.json`,
that denies exactly one thing — a `Read` naming no `offset`/`limit` on a file over that same 400 lines —
and answers with the two `peek.py` calls that would have landed. Claude Code is the only harness that
reads `.claude/`; every other one gets the same floor from `peek.py`, which is why the hook imports the
number rather than holding one.

## A debug cargo command never takes `-p`

`cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` — bare, at the root — are
the three shapes `verify.py` runs and `disk.py`'s `LIVE_QUERIES` keep. Warm, each is a fingerprint
scan of a few seconds, so there is nothing to save by narrowing the build. There is a lot to lose:
cargo resolves features over the packages named on the command line, so `cargo test -p nvs-types`
gives `serde`, `sha2`, `base64` and their like a feature set the workspace build does not, every
workspace crate downstream takes a new metadata hash, and cargo writes a second copy of all of them
beside the first — rlibs, every test binary with its PDB, and one incremental cache per copy.
`cargo build --bin nvs` does the same to every workspace library through the LTO plan. Nothing
removes a copy: cargo has no garbage collector on stable, and `disk.py` keeps anything younger than
its grace. When this was found, nine copies of one day's builds held 120 GB of `target/`.

So a debug build is one of the three shapes, and a narrowing goes on what *runs*. Under `cargo test`
the target flags do exactly that — `--lib`, `--bin nvs` and `--test <name>` keep every hash:

```sh
python tools/verify.py -p nvs-types                    # the tree's build; only nvs-types's test binaries run
cargo test --test closures a_filter                    # one integration-test target, by name, off the same build
cargo test --bin nvs cache::tests::                    # the CLI's unit tests, filtered
cargo test --lib a_filter                              # every crate's unit tests, filtered; warm, seconds
```

`--release -p nvs-abi-probe` and `--release -p nvs-cli` are the cost guards' own profile, which
nothing else builds and `disk.py` never sweeps; they stay as they are. `tools/loop.py` keeps the same
rule: an acceptance check written `cargo test -p <crate>`, bare or with one `--test <name>`, runs that
crate's binaries off one shared `cargo test --no-run`, and the CLI prebuild is a bare `cargo build`.

## Verifying

```sh
python tools/verify.py                                         # build + fmt + test + clippy, one call
python tools/verify.py --fast                                  # build + test only, for a mid-work check
python tools/verify.py -p nvs-ir                               # the same build; only nvs-ir's test binaries run
python tools/verify.py --start   ... --wait                    # run it while you write the wrap file
python tools/verify.py --no-cache                              # re-run even on an unchanged tree
python tools/verify.py --doc                                   # the rustdoc gate alone (the driver's)
python tools/verify.py --list                                  # the steps in order, running none of them
cargo test --release -p nvs-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p nvs-cli --bin nvs by_the_margin         # the CLI's two cost margins (skipped in debug)
cargo test --release -p nvs-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

**Two of those are for the run that is not the final one, and both are measured as unused.** Over a
33-session run there were 108 verifications and **`--fast` was chosen 0 times**, `-p` three times — every
mid-work check paid the whole gate. Verification is 63.6% of a session's tool-execution time, 41.9s a run
at 1.9 runs a session, so a mid-work check that only needs to know whether the code compiles and its tests
pass should say so: `--fast` for build and test, `-p <crate>` when the change cannot reach further. The
run at the **end of the group** is always the full one — that is step 3, and it is not negotiable.

**And the last one need not be spent in series.** `--start` runs the whole verification detached and
returns at once; `--wait` collects it, with its exit status, and prints how much of it overlapped. Start
it, write the wrap file — which is prose you already know and cannot fail — then collect:

```sh
python tools/verify.py --start
<Write the wrap file>
python tools/verify.py --wait
python tools/session.py --wrap .agent-tmp/wrap.md
```

It is the same verification: the same steps in the same order, the same green cache, the same exit status.
Nothing is traded for the overlap, which is why this is the shape to use for step 3 whenever the wrap is
the next thing you were going to do anyway.

**`--list` is what the order is.** It prints the steps in the order the gate walks them and runs none of
them, narrowed by `-p`, `--fast` and `--doc` exactly as far as those narrow a run — so a sentence here
naming the steps would be a second copy that goes stale the day one moves, and this one does not try.
The run stops at the first failure and prints about ten lines when green — run separately those steps are
as many calls and tens of thousands of tokens of output nobody reads once it passes. Every step's full
output is
written to `.agent-tmp/verify-<step>.log` either way. It judges nothing: a step's own exit status is the
whole verdict.

**`cargo doc` is not one of those steps.** It is `--doc`, run alone, and `tools/loop.py` runs it only on
the acceptance sweep that would reach a goal, holding the goal open while it is red. A goal in progress
may carry broken doc links; the session that writes `DONE` runs `--doc` and fixes them, and a red gate
arrives in the next pack under *THE RUSTDOC GATE IS RED*. `tools/verify.py` § *Why `doc` runs when a
goal ends rather than as a step* is the whole argument.

`fmt` is first, and it **formats rather than checks**: a `--check` was the red step in 15 of 39 loop
sessions, each fixed with `cargo fmt` and a second run, and write mode costs the same two seconds. Its
summary line names every file it rewrote, and its exit status is held to the end so a parse error is
still reported by `build`. `tools/verify.py` § *Why `fmt` formats, and runs first* is the measurement.

**A repeat run on an unchanged tree is free** — about two tenths of a second. The green verdict is cached
against a content hash of every file cargo reads plus the exact `rustc -vV`, so a second run after step 4
has edited only documentation prints the verdict it already holds rather than re-deriving it. That is not
a check being skipped: the inputs are bit-identical. Only green is cached, the entry expires after an
hour, and `--no-cache` forces the real thing. A `--fast` or `-p`-scoped verdict never satisfies a wider
run; a wider one does satisfy a narrower.

**The docs gates are not in that list and `verify.py` runs none of them.** `rules.py --check`,
`rules.py --render --check`, `check-links.py`, `layout.py`, `records.py --check`, `plan.py --check`,
`playbook.py --check` and `release.py --check` are CI's `docs` job — Python-only, no toolchain, about a
second together — and `session.py --wrap` runs **six families** of them in-process, so a wrap cannot
commit what it just broke: the link half and the live goal's `[context]` manifest always, the tests the
goal's `cargo-named` checks name on a DONE claim, and then the rulebook (`rules.py`), the records
(`records.py`) and the migration table (`check-migration.py`, which is CI's too) — each only when the
session has edited the tree that feeds it, `docs/rules/`, `docs/decisions/` and `docs/spec/`. That
trigger is the difference between the first three gates and the other three. A dead link is a per-file
fact, so the link gate can ask HEAD which findings are inherited; a rulebook, record or migration finding
is a property of the whole set, so the conservative equivalent is to ask whether this session touched
that tree at all — including a rename, which is what makes a citation elsewhere go dead. The manifest
gate is the reading `chain.py --check` gives the driver's floor — a `playbook` selector that reaches no
bullet, a `shapes` heading that is not there — taken before the commit rather than after the session is
gone, because that floor check halting a DONE claim is a hand the run waits for; a wrap that retires a
bullet prunes the lines that named it, so whatever the gate finds is this session's. The named-test gate
is there for the same hand: a `cargo-named` check is only its `tests` names, a filter that matches no
test runs nothing and exits 0, and the release-profile ones sit behind the floor gate in every scoped
run — so a test written under a near miss of the toml's name is green for the whole goal and surfaces
once, in the sweep that confirms the DONE claim. The wrap refuses a DONE whose named tests are not
`fn`s in the tree, with the same reading `playbook.py`'s `[until: test]` trailer uses.

**`docs/novis.md` is the fifth thing a wrap settles, and it is a write rather than a gate.** The
reference is generated from the binary, the chapters under `docs/reference/` and the migration table's
rows, so a step-4 edit to any of those leaves it stale *and clean* after step 3 verified it — which the
driver's acceptance sweep then reports against a session that is already gone. The wrap regenerates it
in a quarter of a second and the last `## commit:` carries it. A tree with no debug binary cannot answer
the question, and the wrap refuses there rather than committing a guess.

`layout.py` is the one a **code** change trips. It holds CONTRIBUTING.md's layout listing to the tree:
every row names something on disk, every crate, bench package and tracked top-level directory has a row,
and an `[audited unsafe]` marker matches the crate's own `[lints]`. So **a slice that adds or removes a
crate owes that file one line**, and `python tools/layout.py --rows` drafts it from the crate's own `//!`
opening sentence. Nothing else in the tree notices a new crate: that block is prose, and a build cannot
fail over it.

## The user-facing reference, and its proof

```sh
python tools/reference.py                 # regenerate docs/novis.md from the binary + docs/reference/, run every example
python tools/reference.py --check         # is the committed docs/novis.md current? (CI)
python tools/reference.py --examples-only --only 20-types   # one chapter's examples while writing it
python tools/reference.py --primer --check   # prove `nvs agent primer`: its examples run, its refusal codes exist
python tools/proof.py --run               # hand novis.md to a blind `claude -p` reader, judge what it writes
python tools/proof.py --prepare           # the same run directory and PROMPT.md, for any other reader
python tools/proof.py --judge             # score the latest run; report.md beside the tasks
```

`docs/novis.md` is the one file a language user, a search engine or a language model reads, and it
is **generated**: Part B and the tables inside chapters come from `nvs meta --json`, the prose from
one chapter per topic under `docs/reference/`. `verify.py` runs `reference.py` as a step after the
case trees, so the file follows the registry on every green run and a chapter example the binary no
longer agrees with fails the run. [docs/reference/README.md](../reference/README.md) is the format
and the rules; `proof.py`'s module doc is what a failed task means.

## Trying a snippet against PHP

```sh
python tools/try.py .agent-tmp/promo.nvst .agent-tmp/div.nvst .agent-tmp/shift.nvst
python tools/try.py .agent-tmp/*.nvst          # the whole scratch pad, one call
python tools/try.py --keep .agent-tmp/promo.nvst
```

Each file is in the `.nvst` shape — `--TEST--`, `--FILE--`, `--ORACLE--` — or, with no markers at all, a
bare `<?nvs` snippet. `try.py` runs the Novis, runs the `--ORACLE--` through PHP, prints both and says
whether they agree and where they first do not. Write the files with the Write tool, as many as you have
questions, and run them in one call.

**Two things this replaces, and the second is the important one.** Measured over a 33-session run,
sessions made **370 snippet-running calls, 335 of them distinct** — 9.6 a session — each one a heredoc
into `.agent-tmp` followed by a hand-written `php -r` beside it. The turns are the cheap half. The
expensive half is that a hand-written twin is a *translation*, made under time pressure by the same agent
that wrote the Novis, at the moment it most wants the answer to be yes — and a twin that quietly differs
from what it is checking reads exactly like proof. Priority 2 is PHP-compatible observable behaviour;
that is not a place to accept a translation nobody ran.

**An experiment that comes out right is already the case.** Give the file its `--TEST--` sentence, move it
under `tests/differential/`, and `nvs test` runs the same two programs the same way — there is no second
translation step, which is the step the drift used to happen in. [conventions.md](conventions.md) § *A
`.nvst` test case* owns the format.

`try.py` needs `target/debug/nvs` built; `verify.py` builds it, so a snippet run after a green
verification needs nothing. It judges nothing and exits 0 even when a twin disagrees — that is the
finding, not an error.

## Finishing a session: steps 4 and 5 in one call

```sh
python tools/session.py --template                   # the format, with this tree's answers in it
<Write one wrap file>                                # the whole tail as data
python tools/session.py --wrap .agent-tmp/wrap.md    # apply it, or refuse and change nothing
python tools/session.py --check                      # what steps 4-5 still owe, off the tree
python tools/session.py --wrap .agent-tmp/wrap.md --dry-run   # say what it would do
```

The first three are the tail. `--template` is the call to make: it carries every count the tree has moved
past as a ready-to-apply `## plan-edit:`, the playbook's `## ` headings, and a `## commit:` naming the docs
the wrap writes — the three things sessions were re-deriving with a `grep` each, measured at about 25 calls
over one 19-session run. `--check` and `--dry-run` are the interactive pair, not tail steps: `--wrap`
validates everything before writing a byte, so a dry run buys the same refusal one call earlier.

The wrap file is markdown whose `## ` headings are instructions: `## plan: <Field>` rewrites one status
field, `## playbook: <Heading>` appends a bullet, `## handoff` replaces the handoff, `## commit: <paths>`
stages those paths and commits with that message — one section per slice, in order — and `## status` is
the loop's one line. `python tools/session.py --help` is the format in full.

It is applied in a fixed order — plan, playbook, handoff, commits, status — so the docs are on disk before
anything is staged, and **nothing is applied unless every section validates**: an unknown plan field, a
commit subject that is not `type(scope): subject`, a handoff missing `## Next group` or an open item in it
without a repo-rooted `crates/.../file.rs:NN` anchor (a bare `file.rs:NN` is refused too — `orient.py`
expands only the rooted form, and only from the item), a status line that does not start
`CONTINUE`/`DONE`/`BLOCKED`, a dead link — in a body the wrap is about to write, or anywhere in the tree
where it resolved at HEAD and no longer does — and a `rule:` citation in a body that names no rule, which
is how a placeholder id reaches `git log` and turns the goal's rulebook floor red — all refuse the whole
file and write nothing. A
half-finished tail is the one failure mode worth designing out.

The link half is `check-links.py`, which is CI's `docs` job and which `verify.py` does not run, so a
green verification says nothing about links. It is whole-tree rather than diff-scoped because the way
links die here is a **rename**: the file moves and every citation of it goes dead, in files the session
never opened. A link that was already dead at HEAD is reported and refuses nothing.

That order is also why **one wrap writes the docs and commits them**: the handoff, the playbook and the plan
are on disk before the first commit is staged, so a `## commit:` may name them in the same file that writes
them. A wrap that writes a doc and has no `## commit:` at all is refused; one whose commits simply do not
name a path it wrote appends that path to the last commit and reports it. Neither is a nicety — measured
over one 19-session run, **9 sessions closed with a hand-rolled `git add docs/agent/handoff.md
docs/agent/playbook.md docs/implementation-plan.md && git commit`** after this tool had already written all
three, which is about two and a half calls each of exactly the hand-rolled git the wrap exists to remove.

Why it exists: measured over a run, the tail of a session — first `verify.py` to last commit — was **33 of
98 tool calls**, and since context peaks by then those turns carried **42% of the session's whole token
bill**. Almost none of it was thinking; it was 6.0 calls a session on the plan, 5.2 re-deriving anchors
already known, and the rest handoff, playbook, `git add`, `git commit`, `status.txt`. `--check` is the one
to run *before* writing the wrap file: it reports plan fields whose prose names a count the tree
contradicts, whether the handoff still matches its contract, and what is uncommitted.

## Releasing

**A release is a person's, always** — the same rule as a dependency sweep, and for the same reason. No
session, loop or cron job may cut one; `.github/workflows/release.yml` has exactly one trigger and it is a
human pressing `Run workflow`. An agent that notices the tree is due a release says so and carries on.

The procedure and the one-time GitHub setup are [docs/release.md](../release.md); the version scheme is
`rule:packaging/below-1-0-the-breaking-slot-moves-left` and is not plain SemVer below
1.0. The one command worth knowing here is the local preview, which needs no runner and no credentials and
writes nothing:

```sh
python tools/release.py --preview minor     # the exact version and notes a dispatch would produce
```

`python tools/release.py --check` is the gate CI's `docs` job runs: the workspace version, the fourteen
`[workspace.dependencies]` pins that restate it, the newest tag and `CHANGELOG.md` all agree.

`session.py` is not loop-only. Steps 4 and 5 are the same steps in an interactive session, and `## status`
simply reports itself skipped when there is no `.loop/` directory.

## Benchmarking against PHP

```sh
python tools/bench.py                    # 20 userland cases, Novis and PHP side by side
python tools/bench.py 05 regex           # only the cases whose name contains these
python tools/bench.py --check            # do the two halves still agree? (no timing)
python tools/bench.py --php-mode default # PHP as installed, rather than with opcache+JIT
python tools/bench.py --json docs/perf/userland.ndjson   # append one record per case
```

`benches/userland/` holds twenty pieces of ordinary web-and-CLI PHP written twice, `NN-slug.php` beside
`NN-slug.nvs`. [Its README](../../benches/userland/README.md) owns what a case is and how to add one —
including the rule that is not obvious, that each iteration's input must depend on the last one's result,
because PHP's tracing JIT deletes a loop whose input never changes and the deleted loop still prints the
right answer.

Nothing here builds anything: the release binary on disk is the binary that runs, a `target/debug/` one is
refused by name, and the harness warns when the binary is older than the newest file under `crates/`. Two
timings and their baseline-subtracted halves are printed; wall clock is a **same-host, same-minute** ratio
and is not comparable across machines, which is why the cross-machine history in
`rule:testing/perf-two-mechanisms` is counted in instructions instead. This
suite is the § 3 secondary figure of [0026](../decisions/0026.md), the record behind that rule, in
runnable form.

## The server's throughput, in three legs that are not one series

```sh
python tools/bench.py --serve-vs-fpm --record benches/serve.json      # Windows-native, no proxy
python tools/bench.py --serve-vs-fpm --record benches/results/serve.json  # what the loop's sweep runs
python tools/bench-proxied.py --record benches/serve-proxied.json     # nginx in front of both, in Docker
python tools/bench-proxied.py --arm deployed --backend-cpus 8         # PHP's pool against our one core
python tools/bench-proxied.py --nvs-bin /var/tmp/nvs-target-wsl/release/nvs   # skip the image build
python tools/bench-proxied.py --down                                  # tear both stacks down
python tools/bench-load.py --record benches/serve-load.json           # saturation, every answer verified
python tools/bench-load.py --concurrency 1,16,256 --seconds 30        # exactly these widths, longer points
python tools/bench-load.py --in-flight 0                              # the sweep without the 10k leg
```

**Three legs, three artifacts, and no arithmetic between them.** The first runs on this box with no
containers and no proxy, drives `php-cgi -b` over FastCGI with a generator written into `bench.py`, and
is goal `server`'s acceptance check — so it must keep working where there is no Docker and no `wrk`. The second
is [`benches/proxied/`](../../benches/proxied/README.md), which owns every decision it makes: nginx in
front of both peers because that is the only deployment either has, two compose files brought up one at a
time, equal CPU budgets, and `oha` as the generator M7's *Verify* line actually names. Their inputs
differ in every dimension, so the two files are separate and a row from one is never a baseline for the
other.

**The third has no peer, and asks the question neither other leg can answer.** `tools/bench-load.py`
walks a concurrency sweep derived from the machine's core count, then puts ten thousand requests in
flight at once, and **verifies that every answer belonged to the request that asked for it** — the
other two drive `hello.nvs`, whose answers are byte-identical, so a response delivered to the wrong
connection is invisible to them by construction. It is the one leg with something to build:
[`benches/serve-probe/`](../../benches/serve-probe/src/main.rs) is a generator fast enough not to be
the bottleneck, which a Python one is not. It needs no PHP and no Docker, and a client that runs out
of ports or descriptors before the server runs out of anything is reported as `client_limited`
rather than as a server ceiling.

**Where `--record` is pointed is what makes the artifact.** `benches/serve.json` is tracked, so a run
recorded there is a figure someone chose to publish — record it by hand. The loop's acceptance sweep runs
the same leg every iteration and points `--record` at `benches/results/`, which `.gitignore` covers,
because the sweep runs *after* the session's commits: a row appended to a tracked file there is a change
no slice owns, so nothing stages it and it stays in the working tree for good.

`tools/bench.py`'s `## The serve-versus-FPM leg`, that README, and `tools/bench-load.py`'s `## What
this leg is` are the three homes; none restates another, and this block is only the commands.

## What a feature still owes, and the loop that pays it

```sh
python tools/dossier.py                          # the audit: every group, four columns, thinnest first
python tools/dossier.py --id 'Core\Str::length'  # one feature: what it has, what it owes, where each goes
python tools/dossier.py --owed                   # only what is missing, as a worklist
python tools/dossier.py --run examples           # run every example, diff against its `.out`
python tools/dossier.py --run hostile            # run every attack; the runtime must survive it
python tools/dossier.py --bless <file.nvs>       # create an example's `.out` from what it prints
python tools/dossier.py --record-perf --group G  # measure, append to docs/perf/members.ndjson
python tools/dossier.py --no-perf …              # any of the above, with the perf proof switched off
python tools/dossier.py --emit-goals             # append the loop that produces what is owed to the chain
python tools/dossier.py --emit-goals --dry-run   # ... and say what that would change, writing nothing
python tools/dossier.py --partition --group G    # cut a group into worker briefs, or refuse
python tools/dossier.py --brief 'Core\Str::at'   # one feature's brief, as a worker is handed it
python tools/dossier.py --findings [--clear]     # what the workers hit, collated for one batch fix
```

`rule:testing/four-proofs` is why four proofs and not some other
number; each tree's README owns what a file in it is ([examples](../examples/README.md),
[attacks](../../tests/hostile/README.md), [benches](../../benches/members/README.md)); `--help` owns the
rest. **The roster is derived from `nvs meta --json` and the reference chapters**, so nothing needs
adding to a list when a feature lands.

`--emit-goals` writes one goal per group under `docs/agent/goals/dossier/` and **appends them to
`docs/agent/goals/`** — the one chain, always, because that is the only file `loop.py` walks
and an emission anywhere else would be a chain nothing reads. **Deciding to run it is the user's**, like
`doc-cleanup.md` and `dependency-update.md`, for the same reason: it decides what several hundred
sessions will do next. The user made that decision, and [goal `dossier`](goals/71-dossier.md) is
what it turned into — one session whose whole job is to fire the emitter, so the roster's own goals land
on the end of the chain the driver is already walking and the run continues into them without a restart.
`Chain.refresh()` in `loop.py` is the half that makes that true; the emitter is idempotent by slug and by
claim — a goal already on the chain keeps its number, and a feature some generated goal already gates on
is never given a second one — so goal `dossier`'s check re-runs it under `--dry-run` and passes only on
*nothing appended*.

Re-running it is how the chain stays current: a group that owes nothing is left out, a goal the run has
walked is left alone, and only the goals for features no goal claims are appended from the chain's end. A
generated goal is `N-<slug>` under `goals/dossier/` like any other entry, and `chain.py` renumbers it with
the rest.

`--partition` is the one place in this repository where a session hands **writing** to subagents. The rule
in [session-prompt.md](session-prompt.md) — a subagent searches and never writes — holds everywhere else,
and the carve-out is this program alone because dossier work is the one shape that earns it: three of the
four proofs are attributed by a path derived from the feature's own id, so two workers cannot name the
same file, nothing in the goal is a design decision, and `--verify --group` judges the result
mechanically. The tool asserts the first of those on every run rather than trusting it, and writes nothing
when two lanes collide. `tools/dossier.py`'s § *Running one group's features at once* owns the protocol,
`FANOUT_WORKERS` owns the width and how it was measured, and an emitted goal's own prose repeats neither.

## How wide anything runs

```sh
python tools/machine.py               # what this box is, and the widths it implies
python tools/machine.py --refresh     # forget the cached facts and probe again
NVS_VALGRIND_JOBS=2 python tools/loop.py --goal-only   # override one run's sweep width
```

**One policy, in `tools/machine.py`, and no caller has its own:** half the cores the work will actually
see, floor two, capped by how many items there are and by free memory. Half and not more because the
machine is not idle — `loop.py` overlaps the release build with the valgrind sweep deliberately, and that
build is the longer pole.

The facts it needs are a property of the box, so they are probed **once** and cached in
`.loop/machine.json`: cores, free memory, and one unit of the real work timed serially as a baseline. The
probe runs *where the work runs*, which on Windows is inside WSL — `.wslconfig` sets WSL2's cores and
memory independently of the host, so the host's count is the wrong number. An entry is re-probed when the
host name changes or after 30 days, which is how a `.wslconfig` edit gets noticed. `NVS_JOBS` overrides
every width for one run, `NVS_VALGRIND_JOBS` and `NVS_TRY_JOBS` one caller's; none is written back.

Two callers today: the valgrind sweep in `loop.py`, and `try.py`, which runs its snippets this wide and
prints the blocks back in the order you asked for them.

## Disk

```sh
python tools/disk.py                  # what is on disk, what is reclaimable, what is free
python tools/disk.py --clean          # reclaim it
python tools/disk.py --clean -n       # say what --clean would delete; delete nothing
```

**The loop runs `--clean` itself, after every session's acceptance check** — between sessions, when
nothing is building and the build is warm. Every session rather than every goal, because a goal is days
of sessions and a day of builds is what filled the disk once. Nothing about it touches the session
path. By hand it refuses while `.loop/running` exists, because a person cannot see whether a session is
mid-build. The driver's one other disk call is a free-space check that refuses to start a run below
10 GB. That refusal is the point — a run that fills the disk dies inside a session with the tree
half-edited, which is how one run went.

What fills the disk is **build generations**. A crate's artifacts are named `<name>-<metadata-hash>`, and
that hash covers the dependency graph — so every `Cargo.toml` or `Cargo.lock` edit mints a fresh set for
every crate downstream and orphans the previous one, forever: cargo has no garbage collector on stable.
Editing *source* costs nothing, because a source-only rebuild reuses every hash. A milestone that adds a
dependency most sessions therefore adds a whole generation most sessions. One generation of this
workspace was ~6 GB when this was found, and nine of them were on disk at once.

`--clean` never deletes a `deps/` file for being *old*, the way `cargo-sweep` does — cargo never rewrites
an artifact it considers fresh, so a superseded generation and a live one can carry the same date. It asks
cargo instead: warm `--message-format=json` runs of the commands `verify.py` builds with name every file
the current graph uses. Age only ever *keeps*: anything written in the last `GRACE_HOURS` survives
whatever cargo said — a build still in flight, and the `--test <name>` shape the rule above leaves open.
That grace was a day once, every `-p` copy a session minted was younger than that, and the sweep freed
nothing; `release/deps` is never swept. Nothing it does can produce a wrong build —
cargo re-checks every fingerprint against what is really on disk, so a mistake costs a rebuild and nothing
else.

The two `debug` settings in `Cargo.toml`'s dev profile are the other half. On windows-msvc the linker
copies the debug info of every linked object into each binary's PDB, so whatever the workspace carries
is written into all ~60 test binaries at once. `[profile.dev.package."*"] debug = 0` took cranelift and
wasmtime out of them, and a session's own `verify.py` got *faster* (51s → 43s) because there is less to
link and to load; `[profile.dev] debug = "line-tables-only"` then took the type and variable info of
Novis's own crates out, which was three quarters of what remained. A panic location and a backtrace
still name file and line, so every `debug_assert` and the runtime's owner-stamp check report as they
did; a session that needs to step in a debugger sets `CARGO_PROFILE_DEV_DEBUG=2` for that one build.

Four things live outside this repository and `disk.py` reports them without ever deleting them — another
tool's state is not a repo script's to remove. `/var/tmp/nvs-linux` and `/var/tmp/nvs-target-wsl` are the
valgrind leg's and the WSL leg's own target directories, each a full one; deleting either frees ext4 space
but **not** Windows space, because the vhdx never shrinks on its own (`wsl --shutdown`, then compact it,
if C: is what is short). `~/.claude/projects/` keeps one JSONL per session forever.
`~/.cargo/registry/src` is re-extracted on demand and safe to delete.

## Fuzzing and callgrind on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate) needs libFuzzer, and `valgrind`/`callgrind`
(`rule:testing/perf-two-mechanisms`) has no native Windows build at all — do
both in WSL. From a Windows shell, `wsl.exe -- bash -lc "<command>"` runs a command in the default WSL
distro, which reaches the repo over `/mnt/<drive>/…`. What that distro must have installed — and why PHP goes in
it as well, at the same version as the Windows one — is [docs/setup.md](../setup.md).

**Build over the `/mnt` mount; do not clone into the distro to "fix" it.** Per file operation 9p is
50–100× slower, but the base is too small to show: the workspace is 1,412 files, 190 of them `.rs`,
dependencies compile out of `~/.cargo` on ext4 either way, and the target directory is already off the
mount. Measured on this workspace — a cold `cargo build -p nvs-cli` is 31.6s from the mount against 32.2s
from an ext4 copy of the same tree, a no-op rebuild is 0.31s, and one touched file rebuilds in 0.75s. A
synced Linux-side clone buys under a second per acceptance check and costs a stale-copy failure mode.

**Both Linux target directories are under `/var/tmp`, and that is not cosmetic.** `D /tmp` in the distro's
`tmpfiles.d` clears `/tmp` at every boot; WSL stops the VM as soon as the last process exits and boots
again on the next call, so a target directory in `/tmp` is gone after any idle gap and the leg silently
pays 32s of cold build instead of 0.31s. Nothing ages `/var/tmp` out — Ubuntu 24.04 ships its
`q /var/tmp` line commented out.

From the repo root (not `fuzz/` itself — cargo-fuzz expects the parent directory):
`cargo +nightly fuzz run lex -- -max_total_time=300`, and `parse` likewise.

`ast` — `Core\Ast::parse`'s own door, `nvs_syntax::walk::of_source`, asserting that every production the
walk answers with is one the typed roster names — takes `parse`'s seeds, since both read one source text:

    mkdir -p fuzz/corpus/ast
    cargo +nightly fuzz run ast fuzz/corpus/ast fuzz/seeds/parse -- -max_total_time=300

Those same seeds are replayed on stable by
`crates/nvs-stdlib/src/ast.rs`'s `core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed`, which is
the leg that runs on Windows and in the acceptance check.

`prefix` — truncated and mid-edit documents through `nvs-syntax`, which is M4B's *Verify* asking for a
five-minute run — is on the acceptance list and takes its two corpus directories as arguments:

    mkdir -p fuzz/corpus/prefix
    cargo +nightly fuzz run prefix fuzz/corpus/prefix fuzz/seeds/prefix -- -max_total_time=300

The `mkdir` is because the accumulated corpus is ignored and so absent on a fresh clone. Naming both
directories rather than copying the seeds into the corpus is what keeps the tracked half tracked:
libFuzzer writes new inputs to the first directory alone and reads the rest, so `fuzz/seeds/prefix/` is
read-only for the run and the tree it is committed in stays as it was. A panic here is a parser bug — fix
the site, add the minimized input to `fuzz/seeds/prefix/`, and pin it beside the cuts in
`crates/nvs-syntax/tests/prefixes.rs`.

CI's `fuzz-smoke` job runs every target for 300s nightly and at release, carrying the corpus between runs
-- not per push, because 60s against a corpus that starts empty every time is the same 60s repeated
(`rule:testing/the-deep-lane`).

For the instruction-count leg: `cargo build --release -p nvs-abi-probe --example callgrind_spike`, then
`valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out ./target/release/examples/callgrind_spike`.

**A hand-written refcount protocol is where a leak hides, so check one before you commit it.**
`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh <fixture> …` runs `valgrind --leak-check=full` over the
`.nvs` files you name. Run it for **any** new refcount edge, against a fixture that actually exercises it —
this repository's one real leak went unnoticed until a fixture happened to declare a refcounted local
inside a loop.

That script and `loop.py`'s own sweep both pass `--suppressions=`[tools/valgrind.supp](../../tools/valgrind.supp),
whose header is the one home for what it hides: two contexts inside `ring`'s AEAD assembly, which memcheck
reports on every TLS connection a fixture opens and which are not leaks of any kind. A fixture goes red on
that assembly rather than on anything Novis wrote as soon as it runs long enough for its queue worker to
reach the database, so the sweep is unreadable without the file — and, because the two entries are anchored
on a `ring` or `rustls` frame, still red on an uninitialised value Novis's own unsafe code produced.

The sweep alone also layers [tools/valgrind-limits.toml](../../tools/valgrind-limits.toml) over `nvs.toml`
for `examples/limits.nvs`, and that file's header is the one home for why: the fixture's cost under memcheck
is the distance to the memory cap, and its leak verdict is about the kill at the cap. `leak-check.sh` runs
the fixture against the repository's own cap.

**A new thread boundary is where a race hides, so run the sanitizer over it.** `wsl.exe -- bash -lc "bash
tools/tsan.sh"` is the `tsan:` CI job's own command — ThreadSanitizer over `nvs-host` and `nvs-runtime`,
the two crates a thread boundary runs through — and it prints `tsan: clean` and nothing else when it is.
Its header owns what each flag buys; what is worth knowing before you read a report is that a task is a
stackful coroutine, so the leg is only meaningful because
[`crates/nvs-host/src/tsan.rs`](../../crates/nvs-host/src/tsan.rs) tells the sanitizer that a resume moved
the thread onto another stack. That module's doc is the one home of the rule an annotation has to keep,
and a report that names `__tsan_func_entry` is a broken annotation rather than a race in the code under
it. Its target directory is under `/var/tmp` and its own, because `RUSTFLAGS` is part of a fingerprint
and sharing one with the leak sweep would rebuild the tree on every alternation.
