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

**An Edit the tool cannot express goes through `python tools/splice.py <target> --patch <file>`** — write
one patch file under `.agent-tmp/` (gitignored; create it if it is not there) with the Write tool, holding
the old and new blocks between `<<<<<<< OLD` / `=======` / `>>>>>>> NEW` markers, and let that script swap
them. It refuses anything but exactly one match per block, so a stale anchor is an error rather than a
silent wrong edit, and a multi-block patch that half-matches leaves the file untouched. Never reach for a
`sed`/`python - <<'PY'`/`cat > f <<'EOF'` one-liner instead: a heredoc is a shell string, so it eats the
backslashes and apostrophes this repository's Rust and prose are full of. The exact format is in
[conventions.md](conventions.md).

**The plan's status block is edited with `python tools/plan.py --set "<field>" --from <file>`**, not by
hand — locating a field's exact bytes and splicing them was the single most expensive repeated action a
session performed.

**A goal's playbook selection is chosen with `python tools/playbook.py`, not by reading 86 bullets.**
`--goal` ranks every bullet against the goal's own `[context] modules` and prints a paste-ready
`playbook = [...]`; `--match <paths>` does the same for one session's file set; `--check` reports a bullet
naming a path that has left the tree — the only pruning signal an append-mostly file can have — and prices
what the current manifest costs. It never writes to the playbook: appending a bullet is `session.py`'s
`## playbook:` section and stays there.

**The plan is an index and one file per milestone**, and `plan.py` is the only thing that needs to know
which is which: `--show M8` prints one milestone, `--show M8:verify` its acceptance paragraph alone, and
`--amend M8 --from <file>` rewrites one. `--check` prices the status block against the aim the plan's own
comment states and reports an index row that has drifted from the file it names. It refuses to *add* a
field or a milestone — both are decisions, not forms — and it never refuses over a length.

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
This matters more than it looks: a session's wall clock is very nearly its number of turns times a
constant, and read-only probing is where the turns go — an unbatched orientation pass has cost this
repository a quarter of a session's clock, one `grep` at a time. Serialize only what genuinely depends on
a previous answer.

**And when it is reading you are batching, use the tool instead of remembering to.** Over a measured run
of 39 sessions, **0 of 3,647** tool-call messages carried more than one call — against the rule in the
paragraph above, which every one of those sessions had in its context, and including runs of 52 and 57
consecutive `grep`/`sed` calls. A rule that loses 3,647 times is not a rule anyone is going to start
following. `tools/peek.py` takes as many targets as you have questions and answers them in one call:

```sh
python tools/peek.py crates/mwl-ir/src/lower/expr.rs:3065-3120 \
                     crates/mwl-types/src/expr/members.rs:@lower_shape_property \
                     docs/adr/0036-shapes.md:"## 4" \
                     "crates/mwl-runtime/src/*.rs:re:slot_get"
python tools/peek.py --locate mwl_object_slot_get SlotSet ClassDesc   # file:line, no bodies
```

Locators are `120-160`, `120+30`, `@symbol`, `re:pattern` (optionally `re:pattern:3` for context),
`"## Heading"`, or nothing for a whole small file — and the path may be a glob, which is how one call
sweeps a crate. Prefer `re:` to the `/pattern/` spelling: Git Bash rewrites a leading `/` into a Win32
path before the tool sees it. `--locate` is what a handoff's `file.rs:NN` anchors are made of.

## Verifying

```sh
python tools/verify.py                                         # build + fmt + test + clippy, one call
python tools/verify.py -p mwl-ir                               # the same, scoped to one package
python tools/verify.py --no-cache                              # re-run even on an unchanged tree
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

`verify.py` runs `cargo build`, `fmt --check`, `test` and `clippy --all-targets -- -D warnings` in that
order, stops at the first failure, and prints about ten lines when green — the four separately are four
calls and tens of thousands of tokens of output nobody reads once it passes. Every step's full output is
written to `.agent-tmp/verify-<step>.log` either way. It judges nothing: a step's own exit status is the
whole verdict.

`fmt` is second, not last, because it costs a second and a formatting slip should not cost a whole run;
it is not *first* because `cargo fmt --check` on unparseable code reports a rustfmt parse error instead
of the compiler diagnostic that typo deserves.

**A repeat run on an unchanged tree is free** — about two tenths of a second. The green verdict is cached
against a content hash of every file cargo reads plus the exact `rustc -vV`, so a second run after step 4
has edited only documentation prints the verdict it already holds rather than re-deriving it. That is not
a check being skipped: the inputs are bit-identical. Only green is cached, the entry expires after an
hour, and `--no-cache` forces the real thing. A `--fast` or `-p`-scoped verdict never satisfies a wider
run; a wider one does satisfy a narrower.

## Finishing a session: steps 4 and 5 in one call

```sh
python tools/session.py --check                      # what steps 4-5 still owe, off the tree
<Write one wrap file>                                # the whole tail as data
python tools/session.py --wrap .agent-tmp/wrap.md    # apply it, or refuse and change nothing
python tools/session.py --wrap .agent-tmp/wrap.md --dry-run   # say what it would do
```

The wrap file is markdown whose `## ` headings are instructions: `## plan: <Field>` rewrites one status
field, `## playbook: <Heading>` appends a bullet, `## handoff` replaces the handoff, `## commit: <paths>`
stages those paths and commits with that message — one section per slice, in order — and `## status` is
the loop's one line. `python tools/session.py --help` is the format in full.

It is applied in a fixed order — plan, playbook, handoff, commits, status — so the docs are on disk before
anything is staged, and **nothing is applied unless every section validates**: an unknown plan field, a
commit subject that is not `type(scope): subject`, a handoff missing `## Next group` or its `file.rs:NN`
anchors, a status line that does not start `CONTINUE`/`DONE`/`BLOCKED`, all refuse the whole file and
write nothing. A half-finished tail is the one failure mode worth designing out.

Why it exists: measured over a run, the tail of a session — first `verify.py` to last commit — was **33 of
98 tool calls**, and since context peaks by then those turns carried **42% of the session's whole token
bill**. Almost none of it was thinking; it was 6.0 calls a session on the plan, 5.2 re-deriving anchors
already known, and the rest handoff, playbook, `git add`, `git commit`, `status.txt`. `--check` is the one
to run *before* writing the wrap file: it reports plan fields whose prose names a count the tree
contradicts, whether the handoff still matches its contract, and what is uncommitted.

`session.py` is not loop-only. Steps 4 and 5 are the same steps in an interactive session, and `## status`
simply reports itself skipped when there is no `.loop/` directory.

## Benchmarking against PHP

```sh
python tools/bench.py                    # 20 userland cases, MWL and PHP side by side
python tools/bench.py 05 regex           # only the cases whose name contains these
python tools/bench.py --check            # do the two halves still agree? (no timing)
python tools/bench.py --php-mode default # PHP as installed, rather than with opcache+JIT
python tools/bench.py --json docs/perf/userland.ndjson   # append one record per case
```

`benches/userland/` holds twenty pieces of ordinary web-and-CLI PHP written twice, `NN-slug.php` beside
`NN-slug.mwl`. [Its README](../../benches/userland/README.md) owns what a case is and how to add one —
including the rule that is not obvious, that each iteration's input must depend on the last one's result,
because PHP's tracing JIT deletes a loop whose input never changes and the deleted loop still prints the
right answer.

Nothing here builds anything: the release binary on disk is the binary that runs, a `target/debug/` one is
refused by name, and the harness warns when the binary is older than the newest file under `crates/`. Two
timings and their baseline-subtracted halves are printed; wall clock is a **same-host, same-minute** ratio
and is not comparable across machines, which is why the cross-machine history in
[ADR 0026](../adr/0026-performance-measurement-methodology.md) is counted in instructions instead. This
suite is that ADR's § 3 secondary figure, in runnable form.

## Disk

```sh
python tools/disk.py                  # what is on disk, what is reclaimable, what is free
python tools/disk.py --clean          # reclaim it
python tools/disk.py --clean -n       # say what --clean would delete; delete nothing
```

**`--clean` is fired by a person, never automatically**, because it costs a rebuild and only a person
knows whether now is the time to pay one. Nothing about it touches the session path: a session runs no
extra call, and `tools/loop.py` calls only the two cheap prunes (`.loop/logs`, `.agent-tmp`) once per
*run*, plus one free-space check that refuses to start a run below 10 GB. That refusal is the point — a
run that fills the disk dies inside a session with the tree half-edited, which is how 2026-08-25 went.

What fills the disk is **build generations**. A crate's artifacts are named `<name>-<metadata-hash>`, and
that hash covers the dependency graph — so every `Cargo.toml` or `Cargo.lock` edit mints a fresh set for
every crate downstream and orphans the previous one, forever: cargo has no garbage collector on stable.
Editing *source* costs nothing, because a source-only rebuild reuses every hash. A milestone that adds a
dependency most sessions therefore adds a whole generation most sessions. One generation of this
workspace was ~6 GB when this was found, and nine of them were on disk at once.

`--clean` does not ask how *old* an artifact is, the way `cargo-sweep` does — cargo never rewrites an
artifact it considers fresh, so a superseded generation and a live one carry the same date, and 20 GB of
`target/` measured as "0 bytes older than 14 days". It asks cargo instead: one warm
`cargo build --all-targets --message-format=json` names every file the current graph uses, and everything
else beside it in `deps/` is an orphan. Nothing it does can produce a wrong build — cargo re-checks every
fingerprint against what is really on disk, so a mistake costs a rebuild and nothing else.

`[profile.dev.package."*"] debug = 0` in `Cargo.toml` is the other half, and it is why a generation now
holds 1.7 GB of debug info rather than 3.9 GB: on windows-msvc the linker copies the debug info of every
linked object into each binary's PDB, so cranelift and wasmtime were being written into all ~60 test
binaries at once. MWL's own crates keep full debug info; only the dependency wall lost it, and a session's
own `verify.py` got *faster* (51s → 43s) because there is less to link and to load.

Three things live outside this repository and `disk.py` reports them without ever deleting them — another
tool's state is not a repo script's to remove. The WSL leg's `/tmp/mwl-linux` is a second full target
directory; deleting it frees ext4 space but **not** Windows space, because the vhdx never shrinks on its
own (`wsl --shutdown`, then compact it, if C: is what is short). `~/.claude/projects/` keeps one JSONL per
session forever. `~/.cargo/registry/src` is re-extracted on demand and safe to delete.

## Fuzzing and callgrind on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate) needs libFuzzer, and `valgrind`/`callgrind`
([ADR 0026](../adr/0026-performance-measurement-methodology.md)) has no native Windows build at all — do
both in WSL. From a Windows shell, `wsl.exe -- bash -lc "<command>"` runs a command in the default WSL
distro, which mounts the repo at `/mnt/<drive>/<repo>`. What that distro must have installed — and why PHP goes in
it as well, at the same version as the Windows one — is [docs/setup.md](../setup.md).

From `/mnt/<drive>/<repo>` (not `fuzz/` itself — cargo-fuzz expects the parent directory):
`cargo +nightly fuzz run lex -- -max_total_time=300` (and `parse` likewise). CI's `fuzz-smoke` job runs both
for 60s on every push.

For the instruction-count leg: `cargo build --release -p mwl-abi-probe --example callgrind_spike`, then
`valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out ./target/release/examples/callgrind_spike`.

**A hand-written refcount protocol is where a leak hides, so check one before you commit it.**
`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh <fixture> …` runs `valgrind --leak-check=full` over the
`.mwl` files you name. Run it for **any** new refcount edge, against a fixture that actually exercises it —
this repository's one real leak went unnoticed until a fixture happened to declare a refcounted local
inside a loop.
