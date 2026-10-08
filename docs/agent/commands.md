# Commands — how this repository is driven

Everything an agent runs, and the two rules about *how* to run it that have cost real sessions real time.
[AGENTS.md](../../AGENTS.md) points here rather than keeping a copy; `bun nv orient` prints the short
form of the two rules at the top of every loop session.

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
under a fixed **200k** of context, and `bun nv orient` is built to start it at well under 20k. One
`cat` of a 1,100-line module spends a tenth of the remaining budget in a single call. So: whole file when
it is small (roughly under 400 lines, or when you will touch most of it), otherwise `grep -n` for the
anchor and read the region around it. This is the one place turn economy and context economy pull against
each other, and context is the constraint that degrades the work rather than merely slowing it.
`bun nv loop-stats` prints where the last sessions actually landed, and
`bun nv loop-stats --attribute` prints which *reads* put them there.

Use a shell for what it is for — `cargo`, `git`, `bun nv`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

**An Edit the tool cannot express — and any run of three or more edits — goes through `bun nv
splice --patch <file>`.** Write one patch file under `.agent-tmp/` (gitignored; create it if it
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
round trip at a time. That is the same saving `bun nv peek` takes on the read side, for the same reason.

**The plan's status block is written by the wrap, never by hand.** `bun nv session --wrap`'s
`## plan: <Field>` section replaces a field whole, and `## plan-edit: <Field>` replaces one fragment of
it. Locating a field's exact bytes and splicing them by hand was the single most expensive repeated
action a session performed. `bun nv plan --get <Field>` prints a field's current text.

**The plan is an index and one file per milestone**, and `bun nv plan` reads both. `--show M8` prints
one milestone, `--show M8:verify` its acceptance paragraph alone, `--past` the milestones the program is
behind, and `--stale` the sentences that defer work to a goal the chain has already walked. The roster
and the status block are records under `data/plan/`, a milestone's scope is prose under `docs/plan/`,
and a wrap's `## milestone: M8` section rewrites one. `--check` reports the status fields' sizes and
fails when a milestone record disagrees with its scope file or with the chain. Nothing adds a field or a
milestone — both are decisions, not forms — and nothing refuses over a length.

**A playbook bullet is read with `bun nv playbook`, not by listing the directory.** `--show <selector>`
prints one bullet, or a whole section, as `bun nv orient` prints it. Every bullet ends with an
`[until: ...]` trailer, and `tools/nv/cmd/playbook.ts`'s module doc is the one home of its kinds.
`--check` tests the trailers, reports a bullet naming a path that has left the tree and sizes each
section; `--retire` deletes the bullets whose condition holds. `--retire` only ever removes — the
bullet, and every goal manifest's `playbook` line that reached that bullet alone, so the floor's
`bun nv chain --check` never meets a selector reaching nothing. Appending a bullet is the wrap's
`## playbook:` section and stays there.

`--check` **exits non-zero on a gating finding**, and the one that matters is a selector that does not
resolve to exactly one bullet. `bun nv orient` fetches a trap by that string, so a shared lead-in is a
bullet the loop cannot deliver to the session whose goal named it, and neither end reports anything. The
stale paths beside it stay reports — a quoted path is often gone *because* the trap was closed. CI's
`docs` job runs it.

The rulebook is the same arrangement one tree over: `bun nv rules --render` writes
`docs/rules/<topic>.md` and `docs/ground-rules.md` from the topic JSON and the fragments, which own
every line, and `--render --check` reports a rendered copy that has drifted. The decision records
under `docs/decisions/` are frozen and derive nothing by hand: `bun nv records --check`
is their audit, `--stats`, `--graph NNNN` and `--orphans` its readings. This is the pattern
`docs/novis.md` already established below — derive the machine-derivable half, and let a `--check` fail
when the committed copy stops agreeing with it.

**The chain is `data/chain.json`, a list of slugs, and `bun nv chain` edits it.** A goal's position is
its place in that list, printed as `N of M`, so moving a goal renames no file and writes no number.
`--new <slug>`, `--move <slug>` and `--remove <slug>` each take a place — `--after <goal>`,
`--before <goal>`, `--to <position>`, `--next` or `--end` — and edit that one file and nothing else. A
goal's record under `data/goals/` and its prose under `docs/agent/goals/` are written by hand around it.

**`--check` is whether the driver can walk the chain**, and it is on every goal's floor and in CI's
`docs` job. It fails on what would stop the run when the chain reaches it: a goal record the chain does
not name, a goal with no prose, a record that fails its schema, a retired goal at or behind the
installed one, a `context` naming a shape or a bullet that is not there. It also fails on prose that
names a goal by its number, because a number is a position and moves. `tools/nv/cmd/chain.ts`'s module
doc is the whole list.

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
tool instead: `bun nv splice` for a run of edits, and `bun nv peek`, which takes as many targets as
you have questions and answers them in one call:

```sh
bun nv peek crates/nvs-ir/src/lower/expr.rs:3065-3120 \
            crates/nvs-types/src/expr/members.rs:@public_property_names \
            rule:types/conversion \
            docs/decisions/0036.md:"### 4" \
            "crates/nvs-runtime/src/*.rs:re:slot_get"
bun nv peek --locate nvs_object_slot_get SlotSet ClassDesc   # file:line, no bodies
```

Locators are `120-160`, `120+30`, `@symbol`, `re:pattern` (which prints the matching line alone —
`re:pattern:3`, or `--context 3` for every target in the call, is how it comes back with the block
under it), `"## Heading"`, or nothing for a whole small file — and the path may be a glob, which is
how one call sweeps a crate. Prefer `re:` to the `/pattern/` spelling: Git Bash rewrites a leading `/` into a Win32
path before the tool sees it. `--locate` is what a handoff's `file.rs:NN` anchors are made of.

**A `rule:` citation is a target on its own** — `rule:types/conversion`, backticks and all if you
pasted it out of a doc comment — and it answers with that rule's fragment, which is the rule. The
token is the path (`docs/rules/types/conversion.md`), so nothing is looked up and the miss names
`bun nv rules --list`. There are about 18,500 of these tokens in the tree, which makes id-to-text the
most frequent lookup there is here; batch it beside the code you were reading anyway rather than
translating it by hand.

**And when you do not know the file yet, read its seams before its lines.** `bun nv peek
--outline <path>` prints one line per `fn`/`struct`/`enum`/`trait`/`impl`, with where it starts and how
far it runs; every line of it is a `:@name` target that then lands first time. This is what the re-fetch
number argues for: **56% of a session's read calls fetch a file the session had already opened** — 24.6
calls a session across only 20.3 distinct files — and since the two hottest files in this repository are
5,044 and 5,720 lines, "read it whole" was never the alternative. An outline of `lower/mod.rs` is 2.8 KB
against the file's 276 KB. `bun nv peek` counts a session's fetches per file and says so once you have
reached into the same one repeatedly.

**The 400-line floor is enforced on both sides.** `bun nv peek` refuses a bare `path` over
`--max-lines` — 400, which is AGENTS.md rule 3's "whole file under ~400 lines".
The harness's own tools have no such floor, so `bun nv guard` is a `PreToolUse` hook, wired in
`.claude/settings.json` for `Read`, `Bash` and `PowerShell`. It denies a raw command where a tool or a rule
in this file covers it: a whole `Read`, `cat`, `Get-Content` or `sed -n` of a tracked file over that same
400 lines, and the other habits `tools/nv/cmd/guard.ts`'s `RULES` list. Each denial starts `guard: <rule>:`
and names the call to make instead, and anything the guard cannot read is allowed. Claude Code is the only
harness that reads `.claude/`; every other one gets the floor from `bun nv peek` alone.

## A debug cargo command never takes `-p`

`cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` — bare, at the root — are
the three shapes `nv verify` runs and `bun nv disk`'s `LIVE_QUERIES` keep. Warm, each is a fingerprint
scan of a few seconds, so there is nothing to save by narrowing the build. There is a lot to lose:
cargo resolves features over the packages named on the command line, so `cargo test -p nvs-types`
gives `serde`, `sha2`, `base64` and their like a feature set the workspace build does not, every
workspace crate downstream takes a new metadata hash, and cargo writes a second copy of all of them
beside the first — rlibs, every test binary with its PDB, and one incremental cache per copy.
`cargo build --bin nvs` does the same to every workspace library through the LTO plan. Nothing
removes a copy: cargo has no garbage collector on stable, and `bun nv disk` keeps anything younger than
its grace. When this was found, nine copies of one day's builds held 120 GB of `target/`.

So a debug build is one of the three shapes, and a narrowing goes on what *runs*. Under `cargo test`
the target flags do exactly that — `--lib`, `--bin nvs` and `--test <name>` keep every hash:

```sh
bun nv verify -p nvs-types                             # the tree's build; only nvs-types's test binaries run
cargo test --test anon_fns a_filter                    # one integration-test target, by name, off the same build
cargo test --bin nvs cache::tests::                    # the CLI's unit tests, filtered
cargo test --lib a_filter                              # every crate's unit tests, filtered; warm, seconds
```

`--release -p nvs-abi-probe` and `--release -p nvs-cli` are the cost guards' own profile, which
nothing else builds; they stay as they are, and `bun nv disk` sweeps their old copies by rank
(§ *Disk*). The loop driver keeps the
same rule: an acceptance check written `cargo test -p <crate>`, bare or with one `--test <name>`, runs
that crate's binaries off one shared `cargo test --no-run` (`tools/nv/driver/accept.ts`).

## Verifying

```sh
bun nv affected                                                # what the change reaches, and what verifying it runs
bun nv affected --run                                          # run exactly that: nv verify, then the reached checks
bun nv affected --since HEAD~2                                 # the same, for a change already committed
bun nv affected --paths crates/nvs-ir/src/lower.rs             # the same, for these paths alone
bun nv verify                                                  # build + fmt + test + clippy, one call
bun nv verify --fast                                           # build + test only, for a mid-work check
bun nv verify -p nvs-ir                                        # the same build; only nvs-ir's test binaries run
bun nv verify --start   ... --wait                             # run it while you write the wrap file
bun nv verify --no-cache                                       # run every step, whatever the cache holds; by hand only
bun nv verify --doc                                            # the rustdoc gate alone (the driver's)
bun nv verify --list                                           # the steps in order, running none of them
cargo test --release -p nvs-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p nvs-cli --bin nvs by_the_margin         # the CLI's two cost margins (skipped in debug)
cargo test --release -p nvs-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

**`bun nv affected` is the one thing that chooses what to run.** It prints the change — the uncommitted
paths, in a loop session every commit since the session began, `--since <rev>` or `--paths` — and then
what the selection store says differs from the tree it last recorded, which is what runs
(`rule:tooling/a-check-runs-only-when-the-change-reaches-its-footprint`). Its verify half is each step,
test binary and case whose footprint holds a key the change moved, or that is new, red or owed, with
the path each came from. Its acceptance half is each plan check one of whose atoms is selected. A heavy
check — the release profile, fuzz, TSan, the database matrix, the Linux legs — is named and left to the
floor gate. A check reached only because its atoms were owed or red before the change is left out, since
it stays red until the goal is reached. `--run` runs the rest: `nv verify` first,
then the sweep over exactly the checks it named, and a binary or case verify ran green is not started
twice.

**`nv guard` refuses the commands that run more than a change reaches**: `cargo test` with no target
and no filter, `bun nv loop --goal-only` or `--run` over the whole plan, `bun nv proofs --verify` or
`--run` over every feature, and `bun nv verify --no-cache`. A person runs them by hand; the loop
driver is not a tool call, so the guard never sees it. To look
into one test, name it: `cargo test --test <name>` or `cargo test --lib <filter>`.

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
bun nv verify --start
<Write the wrap file>
bun nv verify --wait
bun nv session --wrap .agent-tmp/wrap.md
```

It is the same verification: the same steps in the same order, the same selection, the same exit status.
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

**`cargo doc` is not one of those steps.** It is `--doc`, run alone, and `bun nv loop` runs it only on
the acceptance sweep that would reach a goal, holding the goal open while it is red. A goal in progress
may carry broken doc links; the session that writes `DONE` runs `--doc` and fixes them, and a red gate
arrives in the next pack under *THE RUSTDOC GATE IS RED*. The whole argument is § *Why `doc` runs
when a goal ends rather than as a step* in the old `verify` tool's module doc, at tag `pre-overhaul`.

`fmt` is first, and it **formats rather than checks**: a `--check` was the red step in 15 of 39 loop
sessions, each fixed with `cargo fmt` and a second run, and write mode costs the same two seconds. Its
summary line names every file it rewrote, and its exit status is held to the end so a parse error is
still reported by `build`. The measurement is § *Why `fmt` formats, and runs first* in the same
module doc at tag `pre-overhaul`.

**A step, a test binary or a case that the change does not reach is not started.** Every build and run
is the `covws` build, and every run records what it was observed to use in `.cache/select.sqlite`
(`tools/nv/select/`): a test binary and a case by coverage and the footprint log, each step by the files
it reads. The change since the tree the store last recorded selects exactly the atoms whose footprint
holds a key it moved; a step not selected prints `--` and its last green summary, and a binary not
selected prints its last green `test result:` line. A green run records the tree, and marks each atom
the change reached that it did not run owed; `--no-cache` runs everything. `bun nv select --explain
<atom>` says why one was chosen or not. The module doc of `tools/nv/cmd/verify.ts` is the whole rule,
and `bun nv affected` prints what a run would start. A test binary that asks `nvs_repo` for a path
outside its package is judged by `tools/nv/keys/escape.ts`: `tools/data/impact-wide.txt` says why each
wide one is, and `tools/data/impact-data-literals.txt` lists the `"../x"` literals that are data.

`nvs-fmt`, straight after `build`, **formats** each `.nvs` file git reports as new or modified under
`tests/` and `examples/`, for `fmt`'s reason: `nvs-fmt`'s identity test fails on an unformatted one, and
the fix was always the same command and the whole run again.

**The docs gates are not in that list and `nv verify` runs none of them.** `bun nv rules --check`,
`bun nv rules --render --check`, `bun nv links`, `bun nv layout --check`, `bun nv records --check`,
`bun nv plan --check`, `bun nv chain --check`, `bun nv playbook --check` and `bun nv release --check`
are CI's `docs` job — no Rust toolchain — and `bun nv session --wrap` runs **five families** of them
in-process, so a wrap cannot commit what it just broke: the link half and the live goal's `[context]`
manifest always, the tests the goal's `cargo-named` checks name on a DONE claim, and then the rulebook
(`bun nv rules`) and the records (`bun nv records`) — each only when the session has edited the tree
that feeds it, `docs/rules/` or `docs/decisions/`. That trigger is the difference between the first
three gates and the other two. A dead link is a per-file fact, so the link gate can ask HEAD which
findings are inherited; a rulebook or record finding is a property of the whole set, so the
conservative equivalent is to ask whether this session touched that tree at all — including a rename,
which is what makes a citation elsewhere go dead. The manifest
gate is the reading `bun nv chain --check` gives the driver's floor — a `playbook` selector that reaches no
bullet, a `shapes` heading that is not there — taken before the commit rather than after the session is
gone, because that floor check halting a DONE claim is a hand the run waits for; a wrap that retires a
bullet prunes the lines that named it, so whatever the gate finds is this session's. The named-test gate
is there for the same hand: a `cargo-named` check is only its `tests` names, a filter that matches no
test runs nothing and exits 0, and the release-profile ones sit behind the floor gate in every scoped
run — so a test written under a near miss of the toml's name is green for the whole goal and surfaces
once, in the sweep that confirms the DONE claim. The wrap refuses a DONE whose named tests are not
`fn`s in the tree, with the same reading `bun nv playbook`'s `[until: test]` trailer uses.

**`docs/novis.md` is one more thing a wrap settles, and it is a write rather than a gate.** The
reference is generated from the binary and the chapters under `docs/reference/`, so a step-4 edit to
either leaves it stale *and clean* after step 3 verified it — which the
driver's acceptance sweep then reports against a session that is already gone. The wrap regenerates it
in a quarter of a second and the last `## commit:` carries it. A tree with no debug binary cannot answer
the question, and the wrap refuses there rather than committing a guess.

`bun nv layout` is the one a **code** change trips. It holds CONTRIBUTING.md's layout listing to the tree:
every row names something on disk, every crate, bench package and tracked top-level directory has a row,
and an `[audited unsafe]` marker matches the crate's own `[lints]`. So **a slice that adds or removes a
crate owes that file one line**, and `bun nv layout --rows` drafts it from the crate's own `//!`
opening sentence. Nothing else in the tree notices a new crate: that block is prose, and a build cannot
fail over it.

## What is owed, and where it is collected

```sh
bun nv affected                   # the checks a change reaches, and the paths that reach each
bun nv select                     # the atoms the change since the recorded tree selects, and why
bun nv select --since <rev>       # the same, against <rev> instead of the recorded tree
bun nv select --since A --until B # replay the change from commit A to commit B, to count what it would run
bun nv select --explain <atom>    # why one atom was selected or not: the key, the path, the item
bun nv select --explain <check>   # what a plan check is made of, and why each of its atoms was selected or not
bun nv select --stats             # counts only: atoms per kind and per reason, and the divergences
bun nv select --seed              # record every atom from nothing, for a new platform or a lost store; long
bun nv select --full              # run and record every atom that is not heavy, move the recorded tree, report each selection miss
bun nv select --mutate FILE       # apply each batch of breaking edits FILE lists, run every atom on a store copy, require red ⊆ selected
bun nv select --mutate FILE --batch N --jobs 16   # one batch only, 16 runs at once
```

**The full run catches what the keys missed.** `--full` is the safety net ADR 0226 § 6 names: an atom
red in it whose last verdict was green and which the selection did not pick is a selection miss. It is
kept in the store under `select-miss:<platform>:<atom>` until a full run finds the atom green, and the
command exits 1 while there is one. The loop runs it when the floor gate opens by its count. `--mutate
tools/data/select-mutations.json` is the proof the keys lose no coverage: each batch is a few edits that
each break something, applied together, and every atom that goes red must be one the selection picked.
It never writes the real store: `NV_SELECT_STORE` names the file every `bun nv select` process opens in
place of `.cache/select.sqlite`, and `--mutate` points it at a copy under `.agent-tmp/` that it deletes
afterwards. The edits are reverted in a `finally`, and a batch fails when `git status` differs after it.

**Verification runs what a change can reach, and the checks that cost minutes wait.** `nv verify`
runs the test binaries a change reaches and every `.nvst` case, and the loop's sweep after each session
runs every acceptance check the change reached. The heavy ones — the fuzz run, the
valgrind sweep and the WSL leg, the release-profile guards, the database matrix, the checks never
memoized — wait for the floor gate (`tools/nv/cmd/loop.ts`'s `FLOOR_GATE_EVERY`) and for the sweep a
goal is reached on. Nothing is dropped: an atom the change reached that a run did not start is `owed`
in the store, and stays selected until a run of it is green.

**Every sweep names every red check, not the first.** It runs past a red check, so one red check
never keeps the others from being judged, and the ledger's `goal check:` line is
followed by one `also red:` line per other red check, which the pack prints. A build that fails still
stops it, since nothing behind a build can run. The goal-end gates (`nv verify --doc`, `bun nv owners
--closes`) run after the sweep that would reach the goal, whether it is red or green.


**What a check is made of is decided by what the check is, and every doubt widens.**
`tools/nv/select/checks.ts`'s module doc is the table: a fixture is one atom, a suite its cases, a
cargo check its test binaries, a proofs group its programs, a `bun nv` command one atom whose reads
are recorded. An atom whose run could not be attributed, and a test binary that can leave its package
without `nvs_repo`, hold `*`, which every change moves. `bun nv select --explain <check>` prints what one
check is made of and why each atom was selected or not.

## The user-facing reference

```sh
bun nv reference                          # regenerate docs/novis.md from the binary + docs/reference/, run every example
bun nv reference --check                  # is the committed docs/novis.md current? (CI)
bun nv reference --examples-only --only 20-types   # one chapter's examples while writing it
bun nv reference --primer                 # prove `nvs agent primer`: its examples run, its refusal codes exist
```

`docs/novis.md` is the one file a language user, a search engine or a language model reads, and it
is **generated**: Part B and the tables inside chapters come from `nvs meta --json`, the prose from
one chapter per topic under `docs/reference/`. `nv verify` runs `bun nv reference` as a step after the
case trees, so the file follows the registry on every green run and a chapter example the binary no
longer agrees with fails the run. [docs/reference/README.md](../reference/README.md) is the format
and the rules.

```sh
bun nv site --check snippets              # every website snippet against `nvs run`, a real `nvs serve` and `nvs check`
bun nv site --bless website/snippets/home/hero-html.nvs       # write its .out / .http.out / .err from the binary
```

A website snippet's output files, and when the page shows both the command line's and the server's,
are `rule:testing/website-pages-are-stamped`; `tools/nv/cmd/site.ts` § *A snippet* is the mechanism.

## Trying a snippet

```sh
bun nv try .agent-tmp/promo.nvst .agent-tmp/div.nvst .agent-tmp/shift.nvst
bun nv try .agent-tmp/*.nvst          # the whole scratch pad, one call
bun nv try --keep .agent-tmp/promo.nvst
```

Each file is in the `.nvst` shape — `--TEST--` and `--FILE--` — or, with no markers at all, a bare
`<?nvs` snippet. `bun nv try` runs each one, several at a time, and prints its exit status and its
output in the order they were asked for. Write the files with the Write tool, as many as you have
questions, and run them in one call — never a heredoc into `.agent-tmp` per question.

**An experiment that comes out the way the rules say is already the case.** Give the file its
`--TEST--` sentence and an `--EXPECT--` holding what it printed, move it under `tests/conformance/`,
and `nvs test` runs it. [conventions.md](conventions.md) § *A `.nvst` test case* owns the format.

`bun nv try` runs the pipeline's `covws` build of `nvs`, or the binary `NVS_BIN` names; `nv verify`
builds it, so a snippet run after a green verification needs nothing. It judges nothing and exits 0
even when a snippet fails to compile — what it printed is the finding, not an error.

## Finishing a session: steps 4 and 5 in one call

```sh
bun nv session --template                   # the format, with this tree's answers in it
<Write one wrap file>                       # the whole tail as data
bun nv session --wrap .agent-tmp/wrap.md    # apply it, or refuse and change nothing
bun nv session --check                      # what steps 4-5 still owe, off the tree
bun nv session --wrap .agent-tmp/wrap.md --dry-run   # say what it would do
```

The first three are the tail. `--template` is the call to make: it carries every count the tree has moved
past as a ready-to-apply `## plan-edit:`, the playbook's `## ` headings, and a `## commit:` naming the docs
the wrap writes — the three things sessions were re-deriving with a `grep` each, measured at about 25 calls
over one 19-session run. `--check` and `--dry-run` are the interactive pair, not tail steps: `--wrap`
validates everything before writing a byte, so a dry run buys the same refusal one call earlier.

The wrap file is markdown whose `## ` headings are instructions: `## plan: <Field>` rewrites one status
field and `## plan-edit: <Field>` one fragment of it, `## milestone: <id>` rewrites a milestone's scope,
`## playbook: <Heading>` appends a bullet, `## handoff` replaces the handoff, `## commit: <paths>`
stages those paths and commits with that message — one section per slice, in order — and `## status` is
the loop's one line. `bun nv session --help` is the format in full.

It is applied in a fixed order — plan, milestones, playbook, handoff, commits, status — so the docs are on disk before
anything is staged, and **nothing is applied unless every section validates**: an unknown plan field, a
commit subject that is not `type(scope): subject`, a handoff missing `## Next group` or an open item in it
without a repo-rooted `crates/.../file.rs:NN` anchor (a bare `file.rs:NN` is refused too — `bun nv orient`
expands only the rooted form, and only from the item), a status line that does not start
`CONTINUE`/`DONE`/`BLOCKED`, a dead link — in a body the wrap is about to write, or anywhere in the tree
where it resolved at HEAD and no longer does — a `rule:` citation in a body that names no rule, which
is how a placeholder id reaches `git log` and turns the goal's rulebook floor red, and a goal named by
its number rather than its slug, which turns `bun nv chain --check` red for every session after and which in
a commit message nothing afterwards can even find — all refuse the whole
file and write nothing. A
half-finished tail is the one failure mode worth designing out.

The link half is `bun nv links`, which is CI's `docs` job and which `nv verify` does not run, so a
green verification says nothing about links. It is whole-tree rather than diff-scoped because the way
links die here is a **rename**: the file moves and every citation of it goes dead, in files the session
never opened. A link that was already dead at HEAD is reported and refuses nothing.

That order is also why **one wrap writes the docs and commits them**: the handoff, the playbook and the plan
are on disk before the first commit is staged, so a `## commit:` may name them in the same file that writes
them. A wrap that writes a doc and has no `## commit:` at all is refused; one whose commits simply do not
name a path it wrote appends that path to the last commit and reports it. Neither is a nicety — measured
over one 19-session run, **9 sessions closed with a hand-rolled `git add` of the handoff, the playbook
and the plan `&& git commit`** after this tool had already written all three, which is about two and a half calls each of exactly the hand-rolled git the wrap exists to remove.

Why it exists: measured over a run, the tail of a session — first `nv verify` to last commit — was **33 of
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
bun nv release --preview minor     # the exact version and notes a dispatch would produce
```

`bun nv release --check` is the gate CI's `docs` job runs: the workspace version, the
`[workspace.dependencies]` pins that restate it, the newest tag and `CHANGELOG.md` all agree.

`bun nv session` is not loop-only. Steps 4 and 5 are the same steps in an interactive session, and `## status`
simply reports itself skipped when there is no `.loop/` directory.

## Benchmarking against PHP

```sh
bun nv bench                             # the userland cases, Novis, PHP, Python and Bun side by side
bun nv bench 05 regex                    # only the cases whose name contains these
bun nv bench --check                     # do the engines still agree? (no timing)
bun nv bench --php-mode default          # PHP as installed, rather than with opcache+JIT
bun nv bench --json docs/perf/userland.ndjson   # append one record per case
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
bun nv bench --serve-vs-fpm --record benches/serve.json          # Windows-native, no proxy
bun nv bench --serve-vs-fpm --record benches/results/serve.json  # what the loop's sweep runs
bun nv bench-proxied --record benches/serve-proxied.json         # nginx in front of both, in Docker
bun nv bench-proxied --arm deployed --backend-cpus 8             # PHP's pool against our one core
bun nv bench-proxied --nvs-bin /var/tmp/nvs-target-wsl/release/nvs   # skip the image build
bun nv bench-proxied --down                                      # tear both stacks down
bun nv bench-load --record benches/serve-load.json               # saturation, every answer verified
bun nv bench-load --concurrency 1,16,256 --seconds 30            # exactly these widths, longer points
bun nv bench-load --in-flight 0                                  # the sweep without the 10k leg
```

**Three legs, three artifacts, and no arithmetic between them.** The first runs on this box with no
containers and no proxy, drives `php-cgi -b` over FastCGI with a generator written into `tools/nv/cmd/bench.ts`, and
must keep working where there is no Docker and no `wrk`. The second
is [`benches/proxied/`](../../benches/proxied/README.md), which owns every decision it makes: nginx in
front of both peers because that is the only deployment either has, two compose files brought up one at a
time, equal CPU budgets, and `oha` as the generator M7's *Verify* line actually names. Their inputs
differ in every dimension, so the two files are separate and a row from one is never a baseline for the
other.

**The third has no peer, and asks the question neither other leg can answer.** `bun nv bench-load`
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

`tools/nv/cmd/bench.ts`'s `# The serve-versus-FPM leg`, that README, and `tools/nv/cmd/bench-load.ts`'s
`# What this leg is, and why it is a third one` are the three homes; none restates another, and this
block is only the commands.

## What a feature still owes

```sh
bun nv proofs                          # the audit: one line per group, how many features have each proof
bun nv proofs --id 'Core\Str::length'  # one feature: what it has, what it owes
bun nv proofs --owed                   # only what is missing, as a worklist
bun nv proofs --run --group 'Core\Str' # run every example and attack in scope, and report what failed
bun nv proofs --verify --group G       # the owed gate, then the run, for each scope in one pass
bun nv proofs --bless <file.nvs>       # write an example's `.out` from what it prints
bun nv proofs --comments <path>        # judge a program's comments against the plain-comment bounds
bun nv proofs --record-perf --group G  # measure, append to docs/perf/members.ndjson
bun nv proofs --no-perf …              # any of the above, with the perf proof switched off
```

`rule:testing/feature-proofs` is which proofs a feature owes, and why;
each tree's README owns what a file in it is ([examples](../examples/README.md),
[attacks](../../tests/hostile/README.md), [benches](../../benches/members/README.md));
`tools/nv/cmd/proofs.ts`'s module doc owns the rest. **The roster is derived from `nvs meta --json` and
the reference chapters** (`tools/nv/proofs/roster.ts`), so nothing needs adding to a list when a
feature lands.

## How wide anything runs

```sh
bun nv machine                        # what this box is, and the widths it implies
bun nv machine --refresh              # forget the cached facts and probe again
NVS_VALGRIND_JOBS=2 bun nv loop --goal-only   # override one run's sweep width
```

**One policy, in `tools/nv/lib/machine.ts`, and no caller has its own:** half the cores the work will actually
see, floor two, capped by how many items there are and by free memory. Half and not more because the
machine is not idle — the loop overlaps the release build with the valgrind sweep deliberately, and that
build is the longer pole.

The facts it needs are a property of the box, so they are probed **once** and cached in
`.loop/machine.json`: cores, free memory, and one unit of the real work timed serially as a baseline. The
probe runs *where the work runs*, which on Windows is inside WSL — `.wslconfig` sets WSL2's cores and
memory independently of the host, so the host's count is the wrong number. An entry is re-probed when the
host name changes or after 30 days, which is how a `.wslconfig` edit gets noticed. `NVS_JOBS` overrides
every width for one run, `NVS_VALGRIND_JOBS` and `NVS_TRY_JOBS` one caller's; none is written back.

Its callers are the valgrind sweep and `bun nv try`, which runs its snippets this wide and prints the
blocks back in the order you asked for them.

## Disk

```sh
bun nv disk                           # what is on disk, what is reclaimable, what is free
bun nv disk --clean                   # reclaim it
bun nv disk --clean -n                # say what --clean would delete; delete nothing
```

**The loop runs `--clean`'s sweep itself, in-process, after every session's acceptance sweep** —
between sessions, when nothing is building and the build is warm. Every session rather than every goal,
because a goal is days of sessions and a day of builds fills the disk. Nothing about it touches the
session path. By hand it refuses while `.loop/running` exists, because a person cannot see whether a
session is mid-build. The driver's one other disk call is a free-space check that refuses to start a
run below `--min-free-gb`, whose default is `MIN_FREE_GB` in `tools/nv/cmd/disk.ts`. That refusal is
the point — a run that fills the disk dies inside a session with the tree half-edited.

What fills the disk is **build generations**. A crate's artifacts are named `<name>-<metadata-hash>`, and
that hash covers the dependency graph — so every `Cargo.toml` or `Cargo.lock` edit mints a fresh set for
every crate downstream and orphans the previous one, forever: cargo has no garbage collector on stable.
Editing *source* costs nothing, because a source-only rebuild reuses every hash. A milestone that adds a
dependency most sessions therefore adds a whole generation most sessions. One generation of this
workspace was ~6 GB when this was found, and nine of them were on disk at once.

`--clean` never deletes a `deps/` file for being *old*, the way `cargo-sweep` does — cargo never rewrites
an artifact it considers fresh, so a superseded generation and a live one can carry the same date. It asks
cargo instead: warm `--message-format=json` runs over `target/covws`, which the pipeline builds, and over
`target/debug`, which a person builds by hand, name every file the current graphs use. It also deletes
what nothing reads any more: a stray `default_*.profraw` and the memo files the selection store
replaced. Age only ever *keeps*: anything written in the last `GRACE_HOURS` survives
whatever cargo said — a build still in flight, and the `--test <name>` shape the rule above leaves open.
That grace was a day once, every `-p` copy a session minted was younger than that, and the sweep freed
nothing. Nothing it does can produce a wrong build —
cargo re-checks every fingerprint against what is really on disk, so a mistake costs a rebuild and nothing
else.

`release/deps` and `proof/deps` cannot be asked: naming their live set takes an optimized build of every
shape that uses them, which is minutes of LTO per sweep, and a shape no check runs any more would be
built for nothing. They are swept by rank instead (`staleOptimized`): every artifact keeps its newest
`KEEP_OPTIMIZED` copies and anything written in the last `OPTIMIZED_GRACE_DAYS`. A crate is built only a
few ways at once in those profiles, and every dependency edit leaves one more copy behind, so the copies
past the newest few are the orphans. When this was added, release copies of one workspace crate went back
three weeks, and the rank freed 7 GB of 12. A live copy ranked out costs its own optimized rebuild.

Three debug settings are the other half. On windows-msvc the linker copies the debug info of every
linked object into each binary's PDB, so whatever the workspace carries is written into every test
binary at once. `[profile.dev.package."*"] debug = 0` took cranelift and wasmtime out of them, and a
session's own `nv verify` got *faster* (51s → 43s) because there is less to link and to load;
`[profile.dev] debug = "line-tables-only"` then took the type and variable info of Novis's own crates
out, which was three quarters of what remained. A panic location and a backtrace still name file and
line, so every `debug_assert` and the runtime's owner-stamp check report as they did; a session that
needs to step in a debugger sets `CARGO_PROFILE_DEV_DEBUG=2` for that one build.

The third is `tools/build/tests_without_pdb.rs`, the build script of every crate with a `tests/`
directory: on windows-msvc it links each integration-test program with `/DEBUG:NONE`, so those programs
have no PDB at all. With one program per test file, their PDBs were 4.6 GB of `target/debug` and as much
of `target/covws`, and every worktree builds both again. A failing test still names its file and line,
because the panic location is compiled into the program, and the covws build's coverage still names
every function, because counters and names are sections of the program itself. What goes is a
symbolized Rust backtrace from inside an integration test: set `NVS_TEST_PDB=1` for the build that
needs one, which reruns the script and rebuilds the crates downstream of it. A profile setting cannot
do this. `[profile.test]` applies to the libraries a test links as well, so it gives every workspace
crate a second copy, and `strip` does not touch a PDB.

Five things live outside this repository and `bun nv disk` reports them without ever deleting them — another
tool's state is not a repo script's to remove. `/var/tmp/nvs-linux` and `/var/tmp/nvs-target-wsl` are the
valgrind leg's and the WSL leg's own target directories, each a full one, and `/var/tmp/nvs-target-wsl-src`
holds the WSL leg's copy of each checkout, which the next leg makes again, rebuilding the workspace's own
crates after it; deleting any frees ext4 space
but **not** Windows space, because the vhdx never shrinks on its own (`wsl --shutdown`, then compact it,
if C: is what is short). `~/.claude/projects/` keeps one JSONL per session forever.
`~/.cargo/registry/src` is re-extracted on demand and safe to delete.

## Fuzzing and callgrind on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate) needs libFuzzer, and `valgrind`/`callgrind`
(`rule:testing/perf-two-mechanisms`) has no native Windows build at all — do
both in WSL. From a Windows shell, `wsl.exe -- bash -lc "<command>"` runs a command in the default WSL
distro, which reaches the repo over `/mnt/<drive>/…`. What that distro must have installed — and why PHP goes in
it as well, at the same version as the Windows one — is [docs/setup.md](../setup.md).

**The WSL leg runs from a copy of the tree on the distro's own disk, not over the `/mnt` mount.** Every
file operation on that 9p mount is a round trip to Windows. A build barely shows it, since dependencies
compile out of `~/.cargo` and the target directory is off the mount. A fixture does: `nvs run` from the
repository root resolves every `[[app]]` root in `nvs.toml` before the program starts, and that took 1.5s
per fixture over the mount against 0.02s from ext4, and a `tests/conformance/` run reads every case file
on top. `tools/nv/driver/mirror.ts` keeps the copy at `<env.wsl.targetDir>-src/<checkout>` and brings it
to the working tree before every WSL build, so a stale copy is not a failure mode. The sync is git's own:
a tree taken through a scratch index, and a pack of only what changed sent into the distro as one stream,
so no per-file question crosses the mount. An unchanged tree syncs in about 0.4s and the first sync takes
a few seconds. rsync is the wrong tool for it: it stats every file over the mount, and a no-change pass
over the tree ran for minutes. A command you run by hand over `/mnt` still works and pays the per-file
cost.

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

That script and the loop's own valgrind sweep both pass `--suppressions=`[tools/valgrind.supp](../../tools/valgrind.supp),
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
