# Commands — how this repository is driven

Everything an agent runs, and the two rules about *how* to run it. [AGENTS.md](../../AGENTS.md) states
the short form of the two rules and points here for the rest.

## A shell never carries file content into the tree

Create and edit files with the Write and Edit tools — never a heredoc, a `>` redirect or a `sed -i` that
writes a file. This is not a style preference. A shell tool call is one `-c` string that the shell *parses*
before it runs anything, so an apostrophe in a doc sentence, a backtick in a commit message or an
unbalanced heredoc terminator fails the whole call with `unexpected EOF while looking for matching '` —
the command never executed, and nothing tells you which quote was at fault. The dedicated tools pass
content as JSON parameters with no shell in the path, so that failure cannot occur. This repo makes the
problem worse than most: prose full of apostrophes, backtick-quoted identifiers everywhere, and Git Bash
parsing every shell call, with PowerShell's own quoting on top whenever a call runs `powershell`.

**Reading and searching are a preference, not a prohibition.** Prefer Read, Grep and Glob: they need no
quoting, they return line numbers you can cite, and Grep takes a real regex without a shell mangling it.
A `grep`/`sed -n` through a shell is allowed where it is genuinely shaped better — a pipeline over a
*command's* output, a `wc -l` across a glob — because nothing is being written and a bad quote costs one
retry rather than a silent wrong edit.

**What you read is the budget, so read a big file in the region you need.** A session works under the
context ceiling AGENTS.md § *The context budget* sets, and one read of a large module spends a real part
of it. So: whole file when it is small (roughly under 400 lines, or when you will touch most of it),
otherwise `grep -n` for the anchor and read the region around it. Context is the constraint that degrades
the work rather than merely slowing it. `looper log <session>` prints one session's events, which is
where it landed and what it read.

Use a shell for what it is for — `cargo`, `git`, `bun nv`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

**An Edit the tool cannot express — and any run of three or more edits — goes through `bun nv
splice --patch <file>`**, a patch file under `.agent-tmp/` written with the Write tool. Never reach for a
`sed`/`python - <<'PY'`/`cat > f <<'EOF'` one-liner instead: a heredoc is a shell string, so it eats the
backslashes and apostrophes this repository's Rust and prose are full of. The format is
[conventions.md](conventions.md) § *An edit the Edit tool cannot express*. **The count is the point, not
the difficulty**: a patch costs a Write plus a call, so it wins from three edits, and a run of `Edit`
calls with nothing read between them is one decision spent one round trip at a time.

**A playbook bullet is one file under [playbook/](playbook/)**, and Grep over that directory finds one by
its words. Every bullet ends with an `` `until: <kind> <arg>` `` trailer, whose kinds are looper's
`docs/goal-format.md` § *The playbook*. `looper playbook retire` deletes every bullet whose condition
holds, and only ever removes. Appending a bullet is `looper wrap`'s `## playbook:` section and stays
there.

The rulebook is the same arrangement one tree over: `bun nv rules --render` writes
`docs/rules/<topic>.md` and `docs/ground-rules.md` from the topic JSON and the fragments, which own
every line, and `--render --check` reports a rendered copy that has drifted. There are no decision
records: a rule's fragment holds its reasons and its Declined list, and `bun nv rules --check` fails on
a record directory or a citation of one. This is the pattern
`/llms-full.txt` follows below: derive the machine-derivable half, and let a `--check` fail when the
committed copy stops agreeing with it.

**The chain is the `chain:` list in the front matter of `docs/goals/README.md`, and `looper goal` edits
it.** A goal's position is its place in that list, so moving a goal renames no file and writes no
number. `looper goal new`, `move` and `remove` edit the chain and re-render the README; a goal's file,
`docs/goals/<slug>.md`, is written by hand around it ([loop-authoring.md](loop-authoring.md)).

**`looper lint` is whether the loop can walk the chain.** It fails on a goal file that breaks looper's
format, a stage heading that disagrees with the front matter, an open `### Next` item with no anchor,
and a goal named by its number anywhere in the tree, because a number is a position and moves.

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
A turn's fixed cost does not depend on what the turn does, so a session's wall clock follows its turn
count, and merging two calls into one message saves a turn whether or not either call was expensive.

**But do not rely on remembering to: reach for the tool that batches for you.** Sessions almost never
send two calls in one message, whatever the rule above says, so both sides of the work have a tool
instead: `bun nv splice` for a run of edits, and `bun nv peek`, which takes as many targets as you have
questions and answers them in one call:

```sh
bun nv peek crates/nvs-ir/src/lower/expr.rs:3065-3120 \
            crates/nvs-types/src/expr/members.rs:@public_property_names \
            rule:types/conversion \
            docs/agent/conventions.md:"## A rule fragment" \
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
`bun nv rules --list`. Id-to-text is the most frequent lookup there is here; batch it beside the code
you were reading anyway rather than translating it by hand.

**And when you do not know the file yet, read its seams before its lines.** `bun nv peek
--outline <path>` prints one line per `fn`/`struct`/`enum`/`trait`/`impl`, with where it starts and how
far it runs; every line of it is a `:@name` target that then lands first time. A session that reads the
same large file again and again is the case it is for, and `bun nv peek` counts a session's fetches per
file and says so once you have reached into the same one repeatedly.

**The 400-line floor is enforced on both sides.** `bun nv peek` refuses a bare `path` over
`--max-lines` — 400, AGENTS.md's "read a big file in the region you need".
The harness's own tools have no such floor, so `bun nv guard` runs as a `PreToolUse` hook. Its `RULES`
in `tools/nv/cmd/guard.ts` refuse a whole `Read`, `cat` or `sed -n` of a tracked file over
that same 400 lines, `sleep`, a kill by process name, a write through a shell, a build or test cut with
`head` or `tail`, and the other habits listed there, each with the call to make instead. The settings'
one hook runs the guard of looper's copy, `bun "$CLAUDE_PROJECT_DIR/tools/looper/main.ts" guard`, which
needs Bun alone. It checks an edit or a `git add` of a file another session claims (§ *Other sessions'
edits*), and then runs `bun nv guard --gate` through `looper.toml`'s `[gates] guard`. `bun nv guard
--check` says whether that hook is wired. Claude Code is the only harness that reads `.claude/`; every
other one gets the floor from `bun nv peek` alone.

## What the guards refuse, and the call to make instead

| Refused | Make this call instead |
|---|---|
| A whole `Read`, `cat` or `sed -n` of a tracked file over 400 lines | `bun nv peek <file>:120-160`, `<file>:@symbol` or `<file>:"## Heading"`, or `Read` with an `offset` and a `limit` |
| A heredoc, a `>` redirect or a `tee` that writes into the tree, and a `python -c` or `bun -e` that writes a file | The Write or Edit tool, `bun nv splice --patch <file>`, or a script under `.agent-tmp/` |
| A loop that runs `nvs run` over `docs/examples/`, `tests/hostile/` or `benches/members/` | `bun nv proofs --run --id <feature> --show` |
| `cargo build`, `cargo test` or `cargo clippy` with `-p` and no `--release` | The same command without `-p`, narrowed with `--test <name>` or `--lib`, or `bun nv verify -p <crate>` |
| A command that runs more than a change reaches (§ *Verifying*) | `bun nv affected --run`, or one test named with `cargo test --test <name>` or `cargo test --lib <filter>` |
| `sleep`, and a loop that polls | `run_in_background`, then end the turn (§ *Long commands*) |
| An edit or a `git add` of a file another live session claims, and `git add -A` while a claim stands | Another file, or `git add <your paths>` (§ *Other sessions' edits*) |

The first five are `bun nv guard`'s, and `tools/nv/cmd/guard.ts`'s `RULES` is their home. The last two
are `looper guard`'s.

## Scratch files and worktrees

- **Every scratch file goes under `.agent-tmp/`**: a patch, a probe script, a throwaway case, a commit
  message, a log. Never the system temp directory, a harness's scratchpad or a home directory: a file
  outside the tree is one the user cannot see, review or clean up.
- **Whoever writes a scratch file deletes it when its work is done**: a session before it stops, a
  subagent before it reports, a test in its own `afterAll` or `finally`. What a tool reads back on its
  next run stays: the `verify-*` logs and timings, `peek-ledger.json` and `select-rec/discard/`. Delete
  only your own files, because another agent may be working in the tree.
- **A git worktree is scratch too.** It goes under `.agent-tmp/worktrees/<branch>`. The agent that made
  it removes it and deletes its branch once that branch is merged into `main`, with `git worktree
  remove` and `git branch -d`. Never a forced form: a refusal means work would be lost, and that is a
  question for the user.

## One shell

- **Bash is the only shell tool.** `.claude/settings.json` turns the PowerShell tool off. Run PowerShell
  from Bash as `powershell -NoProfile -Command '…'`, or `powershell -File <script>` for a script such
  as `tools/service-live.ps1`.
- **Every Bash call runs as a recorded job.** The user and other sessions see it in looper's Running
  page and in `looper exec list`.

## Long commands

- **Wait for a long command by ending your turn.** Start it with `run_in_background`, then end the
  turn. Its report starts your next turn, so nothing polls. looper's guard refuses `sleep` and poll
  loops.
- **A command the harness moved to the background at its timeout is still running.** Its report comes
  the same way, so never run it a second time.
- **Watch a command's output with Monitor while you work, never to wait.** Stop your own background
  command with TaskStop.
- **Never trim a build's, a test's or a bench's output with `head` or `tail`.** The harness keeps the
  whole output and previews it past 30,000 characters. Pipe it into `grep` for the lines you need.
- **A heavy command takes a machine-wide slot.** cargo builds and tests, `bun test` and the commands
  under `looper.toml`'s `[jobs] heavy` take one heavy-job slot each, and wait while every slot is held.
  The `[jobs] exclusive` commands take every slot, so no build runs beside a measurement. The comment
  above `[jobs]` there says which commands are which.

## A hang stops the slice

- **Every program `bun nv` starts is watched for a hang.** A tree that waits 90 s with no output, no
  I/O and no CPU is stalled. One that uses the CPU for 10 minutes with no output and no I/O is
  spinning. Either is killed through its own `Tree`, so the kill reaches only what that run started.
  The run reports it as timed out, with the report in `hung`. `tools/nv/lib/hang.ts` is the home.
- **Every Bash call is watched the same way by looper's shell wrapper.** A hung call is killed through
  its own Job Object and exits 124. A call with a listening port, a Monitor call, and a looper wait
  that writes its `waiting` heartbeat are left alone. A command that idles on purpose goes into
  `looper.toml`'s `[jobs] hang_exempt`. `tools/looper/src/jobs/hang.ts` is the home.
- **A `HANG:` line names its report** under `.agent-tmp/hangs/`. The report gives the processes of
  the tree, the CPU each used while it was watched closely, the test libtest last said was running
  long, and the last lines printed. In CI the whole report goes to stderr.
- **A `HANG:` line stops the slice until its cause is found.** Read the report, run the unit it names
  alone with a time limit, and fix what hangs before you go on. A longer `stallMs`, or `hang: false`,
  is a fix only for a program that waits on purpose, and its comment says what it waits for.
- **Delete the report once the cause is fixed.** It is a scratch file, like everything under
  `.agent-tmp/`.

## Other sessions' jobs

Another session's command is a job too. Never kill a process by its name, `nvs.exe` and `cargo`
included, because the name may be another session's process.

| Command | What it does |
|---|---|
| `looper exec list [--all] [--project <p>] [--session <s>] [--port <n>]` | The running jobs with their owners. `--all` adds the ended ones. |
| `looper exec log <id> [--from <line>] [--lines <n>]` | A job's log, which Read cannot open outside your tree |
| `looper exec wait <id>` | Waits for the job to end. Run it with `run_in_background`. |
| `looper exec kill <id>` | Ends the job's whole tree, and records that you asked |

A verify is the exception: `looper verify` joins the run another session started (§ *Verifying*), so
never wait for or kill another session's verify.

## Other sessions' edits

A file has one editor at a time. A session's first edit of a file claims it until the file matches
`HEAD` again, so until its edits are committed or undone.

- **The guard refuses an edit of a file another live session claims.** Work on another file, wait for
  that session's commit, or ask the user to release the claim.
- **Stage your own files by name.** The guard refuses `git add` of a file another live session claims.
  It refuses `git add -A`, `git add .`, `git add -u` and `git commit -a` while such a claim is in the
  tree.

| Command | What it does |
|---|---|
| `looper claims [--json]` | Every claim in the project's trees, with its session, loop, goal, file and age |
| `looper claims release <session>` | Frees the session's claims, and records who asked. An agent may free only a dead session's. |

## A conflict in a generated file

A generated file is committed and never merged by hand. `.gitattributes` marks each one
`linguist-generated` and names the command that writes it. When a merge or a rebase stops on one,
take either side, then render it again from the merged sources and stage what the command wrote:

```sh
git checkout --theirs <file>              # or --ours: either side, since the render replaces it
bun nv render                             # data/schema/, data/spec/, classes.rs, the website's grammar and data
bun nv render --website                   # website/src/data/core.json, reference.json and website/public/llms-full.txt, with a built binary
bun nv rules --render                     # docs/rules/*.md and docs/ground-rules.md
```

Run only the command that writes the conflicted file. A side loop's catch-up does this itself for
the files `looper.toml`'s `[merge]` table names. The perf ledger `data/perf/members.ndjson`
never conflicts: it is append-only, and `merge=union` keeps the lines of both sides.

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
its grace, so a day of `-p` builds fills the disk.

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
that crate's binaries off one shared `cargo test --no-run` (`tools/nv/accept/accept.ts`).

## Verifying

```sh
bun nv affected                                                # what the change reaches, and what verifying it runs
bun nv affected --run                                          # run exactly that: nv verify, then the reached checks
bun nv affected --since HEAD~2                                 # the same, for a change already committed
bun nv affected --paths crates/nvs-ir/src/lower.rs             # the same, for these paths alone
bun nv verify                                                  # build + fmt + test + clippy, one call
bun nv verify --fast                                           # build + test only, for the fix pass
bun nv verify -p nvs-ir                                        # the same build; only nvs-ir's test binaries run
bun nv verify --fast -p nvs-ir                                 # the fix pass: build, then nvs-ir's tests
looper verify                                                  # the loop's gate: bun nv verify, joining a run already going
bun nv verify --no-cache                                       # run every step, whatever the cache holds; by hand only
bun nv verify --doc                                            # the rustdoc gate alone, a session's before DONE
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
check — the release profile, fuzz, TSan, the database matrix — is named and left to looper's sweep when
the live goal names it, and to CI's nightly deep lane whatever changed. A check reached only because its atoms were owed or red before the change is left out, since
it stays red until the goal is reached. `--run` runs the rest: `nv verify` first,
then the sweep over exactly the checks it named, and a binary or case verify ran green is not started
twice.

**`nv guard` refuses the commands that run more than a change reaches**: `cargo test` with no target
and no filter, `looper check` with no `--name`, `bun nv benches`, `bun nv debt`, `bun nv select --full`, `bun nv sweep`, `bun nv proofs --verify` or
`--run` over every feature, and `bun nv verify --no-cache`. A person runs them by hand; looper's sweep
and its hooks are not tool calls, so the guard never sees them. To look into one test, name it: `cargo test --test <name>` or `cargo test --lib <filter>`.

**A goal is verified once, in its gate stage, and two of those flags are for the fix pass after it.**
Verification is most of a session's tool-execution time, and a change to code every test passes through
selects nearly the whole tree. So a stage that is not the gate stage runs no verify: its checks are
`cargo build` and `cargo clippy --all-targets -- -D warnings`, a test it wrote, once and by name, and a
run one of its items names as its requirement. The gate stage, the goal's last, runs the full gate.
Every failure it names is fixed in one pass, and `--fast -p <crate>` reruns just the crate being fixed
before the full run goes again.

**And the gate stage's last run need not be spent in series.** Start it with `run_in_background`, write the wrap
file — which is prose you already know and cannot fail — and end your turn. The verify's report starts
the next turn, and the wrap follows it:

```sh
looper verify                        # with run_in_background; bun nv affected --run interactively
<Write the wrap file, then end the turn>
looper wrap .agent-tmp/wrap.md       # in the turn the green report starts
```

It is the same verification: the same steps in the same order, the same selection, the same exit status.
Nothing is traded for the overlap, which is why this is the shape to use for step 3 whenever the wrap is
the next thing you were going to do anyway. Edit nothing once it has started: each step reads the tree
when it runs.

**`looper verify` runs `bun nv verify`, `looper.toml`'s `[verify] run`, and joins the tree's run when
another session started one,** so two sessions in one tree share one build. Never wait for or kill
another session's verify. `.agent-tmp/verify-progress.json` names the step a run is on.

**`--list` is what the order is.** It prints the steps in the order the gate walks them and runs none of
them, narrowed by `-p`, `--fast` and `--doc` exactly as far as those narrow a run — so a sentence here
naming the steps would be a second copy that goes stale the day one moves, and this one does not try.
The run stops at the first failure and prints about ten lines when green — run separately those steps are
as many calls and tens of thousands of tokens of output nobody reads once it passes. Every step's full
output is
written to `.agent-tmp/verify-<step>.log` either way. It judges nothing: a step's own exit status is the
whole verdict.

**`cargo doc` is not one of those steps.** It is `--doc`, run alone, when a goal ends. A goal in progress
may carry broken doc links; the session that writes `DONE` runs `--doc` first and fixes them
(AGENTS.md § *In the loop*).

`fmt` is first, and it **formats rather than checks**: a `--check` that fails is always fixed with
`cargo fmt` and a second run, and write mode costs the same. Its summary line names every file it
rewrote, and its exit status is held to the end so a parse error is still reported by `build`.

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
a tree of `tools/data/nvs-fmt-corpus.txt`, and each ```nvs block of a new or modified page under one
of its `pages` entries, for `fmt`'s reason: `nvs-fmt`'s identity test fails on an unformatted one, and
the fix was always the same command and the whole run again. **A change to a layout rule runs
`bun nv nvs-fmt`**, which formats the whole corpus and every page block and names each `.out`, `.err`,
`.tainted` or `.replay` beside a rewritten file that can quote a moved line, with the command that
writes it again (`--bless` runs it). `--check` writes nothing.

**The docs gates are not in that list and `nv verify` runs none of them.** `bun nv rules --check`,
`bun nv rules --render --check`, `bun nv links`, `bun nv layout --check` and
`bun nv release --check` are CI's `docs` job, which has no Rust toolchain.
`looper wrap` runs none of them, so a session that edited the tree one of them reads runs that one
itself before the wrap: `bun nv links` after any doc edit, and `bun nv rules --check` after one under
`docs/rules/`. A rulebook finding is a property of the whole set, so a rename there is what makes a
citation elsewhere go dead.

**`website/public/llms-full.txt` is checked by CI, not by `nv verify`.** It is rendered from the
binary and the chapters under `docs/reference/`, so a change to a chapter or a card leaves it stale.
Run `bun nv render --website` after that change, and name the file in the `## commit:` of the slice
that changed its sources.

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

**The full run catches what the keys missed.** `--full` runs every atom that is not heavy, whatever
changed (`rule:tooling/a-check-runs-only-when-the-change-reaches-its-footprint`): an atom
red in it whose last verdict was green and which the selection did not pick is a selection miss. It is
kept in the store under `select-miss:<platform>:<atom>` until a full run finds the atom green, and the
command exits 1 while there is one. A person runs it; the loop never does, and CI's nightly deep lane is
what catches a miss there. `--mutate
tools/data/select-mutations.json` is the proof the keys lose no coverage: each batch is a few edits that
each break something, applied together, and every atom that goes red must be one the selection picked.
It never writes the real store: `NV_SELECT_STORE` names the file every `bun nv select` process opens in
place of `.cache/select.sqlite`, and `--mutate` points it at a copy under `.agent-tmp/` that it deletes
afterwards. The edits are reverted in a `finally`, and a batch fails when `git status` differs after it.

**Verification runs what a change can reach, and the heavy work waits.** `nv verify` runs
the test binaries a change reaches and every `.nvst` case, and looper's sweep after each session runs
the live goal's acceptance checks, each as often as its `every` says. Nothing is dropped: an atom the
change reached that a run did not start is `owed` in the store, and stays selected until a run of it is
green. What no check covers is floor debt, and `bun nv debt` pays all of it once per goal, as the floor
check `floor-debt` that looper judges last (`tools/nv/cmd/debt.ts`): every owed atom but the benches,
and every red proof program and leg, oldest first. The benches wait for a release, where `bun nv benches --owed` pays them, because the
release gate's `proofs` job is red on a stale perf record. Between two sessions only `bun nv sweep` runs, looper's `after_session` hook
(`tools/nv/cmd/sweep.ts`): it sweeps the disk, its stale build caches included, and reads the newest
nightly CI run back, writing each job it failed into `.loop/last-sweep.md`.

**CI's billing and runners are the user's.** A session that finds CI blocked by billing says so and
stops: whether the repository gets a self-hosted runner is not its call. A hosted-runner failure that
does not reproduce locally is still a failure, fixed from its log, and a platform `cfg` that hides the
symptom is not a fix.

**Every sweep names every red check, not the first.** `bun nv affected --run` runs past a red check,
so one red check never keeps the others from being judged, and its `NOT GREEN:` line is followed by one
`also red:` line per other red check. A build that fails still stops it, since nothing behind a build
can run. The goal-end gates, `nv verify --doc` and `bun nv owners --closes`, are the session's to run
before it reports DONE (AGENTS.md § *In the loop*).


**What a check is made of is decided by what the check is, and every doubt widens.**
`tools/nv/select/checks.ts`'s module doc is the table: a fixture is one atom, a suite its cases, a
cargo check its test binaries, a proofs group its programs, a `bun nv` command one atom whose reads
are recorded. An atom whose run could not be attributed, and a test binary that can leave its package
without `nvs_repo`, hold `*`, which every change moves. `bun nv select --explain <check>` prints what one
check is made of and why each atom was selected or not.

## The user-facing reference

```sh
bun nv reference                          # run every example in the chapters under docs/reference/
bun nv reference --only 20-types          # one chapter's examples while writing it
bun nv reference --primer                 # prove `nvs agent primer`: its examples run, its refusal codes exist
bun nv render --website                   # write website/public/llms-full.txt, with the Core pages' data
```

The website's `/llms-full.txt` is the one file a language user, a search engine or a language model
reads, and it is **generated**: Part B and the tables inside chapters come from `nvs meta --json`, the
prose from one chapter per topic under `docs/reference/`. `nv verify` runs `bun nv reference` as a step
after the case trees, so a chapter example the binary no longer agrees with fails the run. CI's
`reference` job runs the examples and `bun nv render --website --check`. [docs/reference/README.md](../reference/README.md) is the format
and the rules.

```sh
bun nv site --check snippets              # every website snippet against `nvs run`, a real `nvs serve`, `nvs check` and `nvs lsp`
bun nv site --bless website/snippets/home/hero-html.nvs       # write its .out / .http.out / .err and .tainted from the binary
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
looper wrap --template                      # the format, with this project's playbook headings in it
<Write one wrap file>                       # the whole tail as data
looper wrap .agent-tmp/wrap.md              # apply the rest, or refuse and change nothing
```

The wrap file is markdown whose `## ` headings are instructions: `## playbook: <Heading>` appends a
bullet, `## handoff` replaces the live goal's handoff, `## commit: <paths>` stages those paths and commits
with that message — one section per slice, in order — and `## status` is the loop's one line. **Nothing
lands unless all of it does**, because a half-finished tail is the one failure mode worth designing out.


**`bun nv links` is not part of the wrap**, and `nv verify` does not run it either, so a green
verification says nothing about links. It is whole-tree rather than diff-scoped because the way links
die here is a **rename**: the file moves and every citation of it goes dead, in files the session never
opened. Run it once before the wrap whenever the session moved or renamed a doc.

The tail is one call because context peaks by then, so each turn of it costs the most, and almost none
of it is thinking: the handoff, the playbook, `git add`, `git commit` and the status line.
`looper wrap` is not loop-only. Steps 4 and 5 are the same steps in an interactive session.

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

## The CLI's warm start

```sh
bun nv bench --warm-start --max-work-ms 9    # what CI runs: the empty program's start, less `nvs --version`
```

`bun nv bench` measures this one figure, and `tools/nv/cmd/bench.ts` owns its bounds. Nothing here
builds: the release binary on disk is the binary that runs, and a `target/debug/` one is refused by name.

## The server's throughput, in two legs that are not one series

```sh
bun nv bench-proxied --record benches/serve-proxied.json         # nginx in front of both, in Docker
bun nv bench-proxied --arm deployed --backend-cpus 8             # PHP's pool against our one core
bun nv bench-proxied --nvs-bin /var/tmp/nvs-target-wsl/release/nvs   # skip the image build
bun nv bench-proxied --down                                      # tear both stacks down
bun nv bench-load --record benches/serve-load.json               # saturation, every answer verified
bun nv bench-load --concurrency 1,16,256 --seconds 30            # exactly these widths, longer points
bun nv bench-load --in-flight 0                                  # the sweep without the 10k leg
```

**Two legs, two artifacts, and no arithmetic between them.** The first
is [`benches/proxied/`](../../benches/proxied/README.md), which owns every decision it makes: nginx in
front of both peers because that is the only deployment either has, two compose files brought up one at a
time, equal CPU budgets, and `oha` as the generator.

**The second has no peer, and asks the question the first cannot answer.** `bun nv bench-load`
walks a concurrency sweep derived from the machine's core count, then puts ten thousand requests in
flight at once, and **verifies that every answer belonged to the request that asked for it** — the
first drives `hello.nvs`, whose answers are byte-identical, so a response delivered to the wrong
connection is invisible to them by construction. It is the one leg with something to build:
[`benches/serve-probe/`](../../benches/serve-probe/src/main.rs) is a generator fast enough not to be
the bottleneck, which a Python one is not. It needs no PHP and no Docker, and a client that runs out
of ports or descriptors before the server runs out of anything is reported as `client_limited`
rather than as a server ceiling.

**Where `--record` is pointed is what makes the artifact.** A file under `benches/` is tracked, so a run
recorded there is a figure someone chose to publish — record it by hand. The loop's acceptance sweep runs
the same leg every iteration and points `--record` at `benches/results/`, which `.gitignore` covers,
because the sweep runs *after* the session's commits: a row appended to a tracked file there is a change
no slice owns, so nothing stages it and it stays in the working tree for good.

That README and `tools/nv/cmd/bench-load.ts`'s module doc are the two homes; neither restates the
other, and this block is only the commands.

## What a feature still owes

```sh
bun nv proofs                          # the audit: one line per group, how many features have each proof
bun nv proofs --id 'Core\Str::length'  # one feature: what it has, what it owes
bun nv proofs --owed                   # only what is missing, as a worklist
bun nv proofs --run --group 'Core\Str' # run every example and attack in scope, and report what failed
bun nv proofs --verify --group G       # the owed gate, then the run, for each scope in one pass
bun nv proofs --bless <file.nvs>       # write an example's `.out` from what it prints
bun nv proofs --comments <path>        # judge a program's comments against the plain-comment bounds
bun nv proofs --record-perf --group G  # measure, append to data/perf/members.ndjson
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
NVS_TRY_JOBS=2 bun nv try a.nvs b.nvs # override one run's width
```

**One policy, in `tools/nv/lib/machine.ts`, and no caller has its own:** half the cores the work will actually
see, floor two, capped by how many items there are. Half and not more because the machine is not idle:
a build or a loop's session usually shares it.

The facts it needs are a property of the box, so they are probed **once** and cached in
`.loop/machine.json`: cores and free memory. An entry is re-probed when the host name changes or after
30 days. `NVS_JOBS` overrides every width for one run, `NVS_VALGRIND_JOBS` and `NVS_TRY_JOBS` one
caller's; none is written back.

Its callers are `bun nv valgrind` and `bun nv try`, which runs its snippets this wide and prints the
blocks back in the order you asked for them.

## The loop's hooks, and the commands beside them

```sh
bun nv sweep [--keep-runs <n>]                        # sweeps the disk, reads nightly CI: looper's after_session hook
bun nv side --prepare --hook                          # looper's before_run: readies a side loop's lane, else nothing
bun nv ci --push                                      # looper's after_retire: pre-push checks, pushes main, cancels the runs it replaces
bun nv ci --check                                     # the pre-push checks alone, before a goal is claimed done
bun nv ci --watch | --status | --rerun                # waits for a failed job, lists them, reruns them
bun nv debt                                           # pays the floor debt: the floor check `floor-debt`

bun nv valgrind --nvs <binary> [<file>...]            # every examples/ program under memcheck, on Linux: CI's nightly `valgrind` job
bun nv linux-test --test <name> [<filter>]            # one filtered cargo test on a Linux build, inside WSL
bun nv services --up [--goal <slug>]                  # Docker preflight, then the goal's services up and healthy
bun nv suite <tree>... [--case <path>]... [--min-passing <n>] [--full]   # judge one case tree, with a pass floor
bun nv benches [--owed]                               # the bench turn by hand
bun nv side --prepare <slug> [--dir <path>] [--min-free-gb <n>]   # make or complete a side worktree by hand
```

Each exits 0 green, 1 red and 2 on a usage error, except `bun nv sweep`, which always exits 0 and writes
what was red into `.loop/last-sweep.md`, below the lines looper's own sweep wrote there, and `bun nv ci
--watch`, which also exits 3 when nothing failed within `--max-min`. `--goal` and
`--stage` win over `LOOPER_GOAL` and `LOOPER_STAGE`, which win over the live goal and its handoff's next
stage. looper runs four on its own: `looper.toml` wires `bun nv side --prepare --hook` as
`[hooks] before_run`, `bun nv sweep` as `[hooks] after_session` and `bun nv ci --push` as `[hooks]
after_retire`, whose comments there say why each
timeout and `on_fail` are what they are, and `docs/goals/README.md`'s floor runs `bun nv debt` as
`floor-debt`. A person runs the rest. Each command's `--help` and module doc own its flags.

**A side goal is a goal file off the chain**, run as a side loop with `looper loop add --side <slug>`.
looper makes its lane, the worktree `.agent-tmp/worktrees/lane-<n>` on branch `side/<slug>`, and
`bun nv side --prepare --hook` readies it. A side loop whose `.agent-tmp/worktrees/side/<slug>` exists
keeps that worktree. Once its goal is reached and its file deleted, the loop lands its branch: it
merges `main` in, runs verify, pauses the other loops and fast-forwards `main`. It never pushes.

**`main` reaches GitHub through `bun nv ci --push` alone.** looper's `after_retire` hook runs it at
every goal end, and then `bun nv relink --build --hook`, which rebuilds `target/release/nvs` in the root
and builds nothing in a side loop's lane (`tools/nv/cmd/relink.ts`). Last comes `bun nv vsix --hook`,
which rebuilds `editors/vscode/nvs.vsix` when the extension changed since its last build, under the same
side-lane rule (`tools/nv/cmd/vsix.ts`). A person or an interactive session watches the runs a push
starts with `bun nv ci --watch`, fixes each failure, commits, and pushes again with `bun nv ci --push`.
Only one session drives CI at a time. The push starts the runs, and the command cancels each run a
newer one replaces, so each workflow has one run going on `main` (`rule:testing/one-run-on-main`).
`tools/nv/cmd/ci.ts` is the command's procedure.

## Disk

```sh
bun nv disk                           # what is on disk, what is reclaimable, what is free
bun nv disk --clean                   # reclaim it
bun nv disk --clean -n                # say what --clean would delete; delete nothing
```

**`--clean` is run by hand, between sessions**, when nothing is building — a day of builds fills the
disk, so a long run wants it every day or so. Nothing about it touches the session path. looper's one
disk call is a free-space check that refuses to start a loop below `looper.toml`'s `[loop]
min_free_gb`, which matches `MIN_FREE_GB` in `tools/nv/cmd/disk.ts`. That refusal is the point — a run
that fills the disk dies inside a session with the tree half-edited.

What fills the disk is **build generations**, and what `--clean` deletes and keeps is
`tools/nv/cmd/disk.ts`'s module doc. The debug settings that keep each build small are `Cargo.toml`'s
`[profile.dev]` comments, and `tools/build/tests_without_pdb.rs` is why an integration test has no PDB
on windows-msvc (`NVS_TEST_PDB=1` brings one back).

`bun nv disk --deep` reports the places outside this repository without ever deleting them.
`/var/tmp/nvs-linux`, `/var/tmp/nvs-target-wsl` and `/var/tmp/nvs-target-wsl-src` are
`tools/leak-check.sh`'s and `bun nv linux-test`'s target directories and `linux-test`'s copy of each
checkout. Deleting any frees
ext4 space but **not** Windows space, because the vhdx never shrinks on its own (`wsl --shutdown`, then
compact it, if C: is what is short). `~/.cargo/registry/src` is re-extracted on demand and safe to
delete.

## A Windows checkout needs `bun nv checkout-acl` once

`nvs serve` checks the ownership of every configuration file at boot and at each reload
(`rule:config/the-ownership-check-runs-where-it-can-be-answered`), and a checkout on a non-system drive
inherits the drive's `Authenticated Users` write grant, and on the CI runners `BUILTIN\Users`' right
to create files in every folder, so every server on a checkout file and every served test under
`target/` stops with `E0607`. `bun nv checkout-acl` breaks the checkout root's inheritance, grants this
account full control, removes `Authenticated Users`, `Everyone` and `BUILTIN\Users`, and gives
`BUILTIN\Users` read and execute back, once; `--check` only reports. CI's Windows jobs run it after the tool install. Its header names the undo.

**Compiling the Unix half from Windows** is `cargo check --tests` in WSL with its own
`CARGO_TARGET_DIR` on ext4. Keep the output unfiltered and print a heartbeat, or the hang watcher
kills it while `nvs-cli` compiles: WSL's CPU time is invisible to it.

## Fuzzing and callgrind on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate) needs libFuzzer, and `valgrind`/`callgrind`
(`rule:testing/perf-two-mechanisms`) has no native Windows build at all — do
both in WSL. From a Windows shell, `wsl.exe -- bash -lc "<command>"` runs a command in the default WSL
distro, which reaches the repo over `/mnt/<drive>/…`. What that distro must have installed — and why PHP goes in
it as well, at the same version as the Windows one — is [docs/setup.md](../setup.md).

**`bun nv linux-test` runs from a copy of the tree on the distro's own disk, not over the `/mnt` mount**,
at `/var/tmp/nvs-target-wsl-src/<checkout>`. `tools/nv/accept/mirror.ts`'s module doc is why and how the copy
is kept current. A command you run by hand over `/mnt` still works and pays a round trip to Windows for
every file it opens.

**Both Linux target directories are under `/var/tmp`, and that is not cosmetic.** `D /tmp` in the distro's
`tmpfiles.d` clears `/tmp` at every boot; WSL stops the VM as soon as the last process exits and boots
again on the next call, so a target directory in `/tmp` is gone after any idle gap and the leg silently
pays a cold build. Nothing ages `/var/tmp` out: Ubuntu ships its `q /var/tmp` line commented out.

From the repo root (not `fuzz/` itself — cargo-fuzz expects the parent directory):
`cargo +nightly fuzz run lex -- -max_total_time=300`, and `parse` likewise.

`ast` — `Core\Ast::parse`'s own door, `nvs_syntax::walk::of_source`, asserting that every production the
walk answers with is one the typed roster names — takes `parse`'s seeds, since both read one source text:

    mkdir -p fuzz/corpus/ast
    cargo +nightly fuzz run ast fuzz/corpus/ast fuzz/seeds/parse -- -max_total_time=300

Those same seeds are replayed on stable by
`crates/nvs-stdlib/src/ast.rs`'s `core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed`, which is
the leg that runs on Windows and in the acceptance check.

`prefix` — truncated and mid-edit documents through `nvs-syntax`, run for five minutes — is on the
acceptance list and takes its two corpus directories as arguments:

    mkdir -p fuzz/corpus/prefix
    cargo +nightly fuzz run prefix fuzz/corpus/prefix fuzz/seeds/prefix -- -max_total_time=300

The `mkdir` is because the accumulated corpus is ignored and so absent on a fresh clone. Naming both
directories rather than copying the seeds into the corpus is what keeps the tracked half tracked:
libFuzzer writes new inputs to the first directory alone and reads the rest, so `fuzz/seeds/prefix/` is
read-only for the run and the tree it is committed in stays as it was. A panic here is a parser bug — fix
the site, add the minimized input to `fuzz/seeds/prefix/`, and pin it beside the cuts in
`crates/nvs-syntax/tests/prefixes.rs`.

A target whose inputs need bytes the LF checkout would rewrite takes a dictionary instead of seeds.
`request_head` needs CRLF, so it runs with `-dict=fuzz/dicts/request_head.dict` after the `--`, and
CI passes `fuzz/dicts/<target>.dict` to every target that has one.

CI's `fuzz-smoke` job runs every target for 300s nightly and at release, carrying the corpus between runs
-- not per push, because 60s against a corpus that starts empty every time is the same 60s repeated
(`rule:testing/the-deep-lane`).

For the instruction-count leg: `cargo build --release -p nvs-abi-probe --example callgrind_spike`, then
`valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out ./target/release/examples/callgrind_spike`.

**A hand-written refcount protocol is where a leak hides, so check one before you commit it.**
`MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh <fixture> …` runs
`valgrind --leak-check=full` over the `.nvs` files you name. Without the variable, Git Bash rewrites the
`/mnt/…` path before `wsl.exe` sees it. Run it for **any** new refcount edge, against a fixture that
actually exercises it: a leak no fixture reaches is a leak no run reports.

That script and `bun nv valgrind`, CI's nightly leak check, both pass `--suppressions=`[tools/valgrind.supp](../../tools/valgrind.supp),
whose header is the one home for what it hides: two contexts inside `ring`'s AEAD assembly, which memcheck
reports on every TLS connection a fixture opens and which are not leaks of any kind. A fixture goes red on
that assembly rather than on anything Novis wrote as soon as it runs long enough for its queue worker to
reach the database, so the run is unreadable without the file — and, because the two entries are anchored
on a `ring` or `rustls` frame, still red on an uninitialised value Novis's own unsafe code produced.

`bun nv valgrind` alone also layers [tools/valgrind-limits.toml](../../tools/valgrind-limits.toml) over `nvs.toml`
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

## HTTP Garden: `bun nv garden`

`bun nv garden [--rounds N] [--seed N] [--refs hyper,node_stdlib]` sends mutated requests to `nvs serve`
and to the Garden's hyper and Node origins, and lists what `nvs` reads differently from both. It runs
the same way on Windows with Docker Desktop and on Linux. The Garden's harness reaches each origin by
container IP, which only a container on the Garden's network can reach under Docker Desktop, so the
harness runs in one, with its files copied into the image rather than mounted. [`tools/nv/cmd/garden.ts`](../../tools/nv/cmd/garden.ts)'s head
comment owns how, and `tests/garden/` holds the files it builds from. The first run compiles hyper from
source in Docker, and later runs reuse that image. Node is the official release image. A panic in the `nvs` log fails the run;
a reading is for a person to sort into a bug, a new refused row, or strictness that stays. It is not a
CI job: each finding becomes a test in the smuggling suite, which CI runs. Run it again by hand after a
change to how a request head is read (`crates/nvs-server/src/io.rs`, `serve.rs`, `Core\Request`'s header
and body readers), and before a release.

## Without looper

Every `looper` command in these files is the project's own. looper's copy in `tools/looper/` runs it
with Bun alone, so a clone with no `looper` program runs `bun tools/looper/main.ts <command>` instead.
`tools/loop <command>` in bash and `tools\loop <command>` in cmd or PowerShell are the same. Each gate a change must pass is still a `bun nv` command, which the copy
runs through `looper.toml`'s `[verify]` and `[gates]`. `tools/nv/test/without-looper.test.ts` runs the
guard, the prose check and the commit hook with no looper on PATH.

Only what spans projects needs looper's service. Without the service, each command runs on as this
table says.

| What needs looper's service | Without it |
|---|---|
| The heavy-job slots shared across projects | `looper verify`, `looper check` and a heavy Bash call run at once, without a slot. |
| The usage wall shared across the account | A wall a session meets holds the loops of this project only. |
| `looper project add` | It exits 3, since it registers the project with the service. |
| looper's web UI | `looper status`, `looper log` and `looper exec list` print what it shows. |

`tools/git-hooks/commit-msg` is this repository's own hook, and needs no looper. It rejects a trailer
and a subject out of the shape `looper.toml`'s `[commits]` sets.

**Why it is shaped this way.** The user decided that the project runs and is developed the same way with
and without looper, so each check is written once, in `bun nv`, and `looper.toml`'s `[gates]` names it.
- `bun nv links` and `bun nv layout --check` check more than looper's own: source-file links and their
  form, crate and bench-package rows, and the `[audited unsafe]` marker.
- A check's configuration lives with the check unless looper still reads it. `[commits]` stays in
  `looper.toml` because `looper wrap` reads it, and the hook reads it there with awk. `[prose] scope` stays
  because AGENTS.md's managed block renders it. `docs/agent/prose.md` and `testing.md` are on `[sync] skip`, so they name the `bun nv` command.
- The settings have one guard entry, the copy's, and it runs `bun nv guard --gate` through
  `[gates] guard`. So the rules run once per call. The cost is two `bun` starts per guarded call: the
  copy's, then the gate's.
- The commit hook stays in sh and awk, so a commit starts no new process.
