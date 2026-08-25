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

## Verifying

```sh
python tools/verify.py                                         # build + test + clippy + fmt, one call
python tools/verify.py -p mwl-ir                               # the same, scoped to one package
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

`verify.py` runs `cargo build`, `test`, `clippy --all-targets -- -D warnings` and `fmt --check` in that
order, stops at the first failure, and prints about ten lines when green — the four separately are four
calls and tens of thousands of tokens of output nobody reads once it passes. Every step's full output is
written to `.agent-tmp/verify-<step>.log` either way. It judges nothing: a step's own exit status is the
whole verdict.

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
