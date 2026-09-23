#!/usr/bin/env python3
"""AGENTS.md § *Session workflow* step 3, as one command.

`cargo fmt`, `tools/lints.py --check`, `tools/directives.py --check` and `--check-template`,
`tools/owners.py --check`, the fuzz workspace's lock file brought back in step, `cargo
build`, `nvs fmt` over the `.nvs` files this working tree added or changed, `cargo test`, the
`.nvst` trees through the binary the build just produced, `cargo clippy
--all-targets -- -D warnings`, and -- once `editors/vscode` exists -- that extension's headless
suites, in that order, stopping at the first failure. The script gates precede the compile steps
because they decide what the tree means rather than whether it builds. Two stretches of that
order run at the same time -- *Why steps overlap* below -- and their verdicts are still taken
and reported in list order, so the order is still the one a failure is reported in. Green prints one
line per step; a failure prints that step's output and nothing else. `fmt` and `nvs-fmt` *write*
source: they format rather than check, and *Why `fmt` formats* and *Why `nvs-fmt` formats* below
are the measurements. `fuzz-lock` and `reference` write the one derived file each of them owns.

A step whose inputs have not changed since it was last green is not run again, and that is
decided a step at a time -- *Why a step whose inputs did not change is not run* below.

`cargo doc` with rustdoc's broken-link lint denied is the one gate deliberately **not** in that
list. It is `--doc`, run alone, and `tools/loop.py` runs it when a goal's acceptance list is
green rather than on every verification -- see *Why `doc` runs when a goal ends* below.

The `conformance` and `differential` steps run `target/debug/nvs test tests/<tree>`, which is
exactly what `tools/loop.py`'s acceptance check runs, and they print the two counts the plan's
status fields quote. `nvs test` runs the cases as a pool (`nvs_test::run` has the measurement);
serially they had grown to 89s, half of every verification, and were paid again by the driver's
sweep. **Do not rebuild
`target/release/nvs.exe` to run a case** -- see `CASE_TREES` below for the measurement, and the
playbook under *Running things*.

The point is turn count, not typing. Run separately, those are as many tool calls whose
combined output runs to tens of thousands of tokens a session never reads once it is green --
and a session's wall clock is very nearly its number of turns times a constant. Run here, a
green verification is one call and about ten lines.

    python tools/verify.py                  # every step
    python tools/verify.py -p nvs-ir        # the same build; only nvs-ir's test binaries run
    python tools/verify.py --fast           # build and test only, for a mid-work check
    python tools/verify.py --doc            # the rustdoc gate alone; the driver's goal-end call
    python tools/verify.py --start          # run it detached and return at once
    python tools/verify.py --wait           # collect what --start left, with its exit status
    python tools/verify.py --full           # do not truncate the failing step's output
    python tools/verify.py --no-cache       # run every step, whatever the green cache holds
    python tools/verify.py --list           # the steps in order, running none of them

`--start` / `--wait` exist because verification is 63.6% of a session's tool-execution time and
the tail that follows it -- the wrap file -- is prose the session already knows and that cannot
fail. The two do not depend on each other until the wrap is applied, so start the run, write the
wrap, then collect: it is the same steps, the same green cache and the same exit status, with the
seconds overlapped instead of queued. `--fast` and `-p` are the other half of the same point and
were measured at 0 and 3 uses across 108 verifications: a mid-work check does not owe the full
gate, and the run at the end of the group always does.

Full output of every step is always written to `.agent-tmp/verify-<step>.log`, whether it
passed or not, so a truncated failure is one Read away from complete.

While it runs, `.agent-tmp/verify-progress.json` names the step in flight -- `{"step": "test",
"index": 3, "total": 7, "at": <epoch>, "done": [{"name": "build", "seconds": 8.1}, ...]}` --
rewritten at every step boundary and once more at the end with `"finished": <exit status>`.
Nothing in this file reads it. It exists for `tools/loop.py`'s status line: the harness hands the
driver a tool call's output only when the call returns, so a watcher of the loop otherwise sees a
spinner for the whole minute and a half a verification takes. Printing more to stdout would not
reach that watcher any sooner, and would cost the session the context this script exists to save.

This script judges nothing. A step's own exit status is the whole verdict -- there is no
threshold here, no allowance, and no way to make a red step green from this file.

## Why `fmt` formats, and runs first

It was `cargo fmt --check`, second, until it was measured. Over the 39 loop sessions in
`.loop/logs` on 2026-09-06, 15 failed a verification at `fmt` -- more than every other red step
combined (test 5, clippy or build 6, conformance 1) -- and every one of them was fixed the same
way, `cargo fmt` and the run again, at a mean of 1.3 tool calls plus a second collection: about
three calls and forty seconds a time, in two sessions of every five. Sessions had started typing
`cargo fmt --all && python tools/verify.py` to pre-empt it, which is the tell.

Measured on a copy of the tree: write-mode `cargo fmt --all` costs 2.0s, the same as `--check`,
rewrites nothing on a clean tree, and on a file that does not parse it exits 1 and leaves the
file untouched. So the step formats, and it runs *first*, so that `build` and everything after
it compile the text the commit will carry and nothing is compiled twice. Its exit status is
held rather than acted on: a parse error is a far better diagnostic coming from `build` one step
later than from rustfmt, so the run goes on, and `fmt`'s own failure is reported only when every
other step passed. rustfmt's `-l` names each file it rewrote, and the summary line quotes them
so a session sees what changed under it. Because the step can rewrite the tree, every later
step's key is taken again after it when it did.

What this trades: a second writer editing the same tree has its files formatted too. `--check`
failed on those files just the same, so this is the smaller intrusion, but an Edit in flight
against one of them can miss its anchor once.

## Why a step whose inputs did not change is not run

The rule is one verification per session, at the end. Measured against `.loop/logs`, sessions ran
this script 2.4 times on average, and the re-runs followed an edit that could not reach most of
the steps: documentation, a comment, or the layout of one file. On one day 20 of 26 sessions
reformatted a new `.nvs` file after a red `test` step and paid three and a half minutes for every
step again, the six before `test` included, which had just been green over the same bytes.

So each step is green against a key over exactly what *that step* reads, and a step whose key
has not moved is answered from `.agent-tmp/verify-green.json` rather than run.
`tools/verify_keys.py` is the table and its module doc the detail; the shape of it:

- A file under `tests/hostile/` is read by the test binaries and by nothing else, so reformatting
  one runs `build` and `test` and answers the other eleven steps.
- A `.rs` file is read four ways. `fmt`, the script steps and the test binaries read its bytes --
  a policy test here reads source as text, so for a test binary a comment is an input. `clippy`
  and the doc-tests read the code and the doc comments with the layout removed. `build` reads
  the code alone. A step that only runs `target/debug/nvs` reads the code outside every inline
  `#[cfg(test)]` module, which is all of it that binary is built from. So a comment, a
  re-wrapped line or a new `#[test]` in a source file cannot reach the `.nvst` trees,
  `reference` or `extension`.
- The plan and the goal chain are read by `owners` alone, so they are in its key and no other.

This is **not** a check being skipped: the step's inputs are identical in every respect the step
can observe, so running it again cannot reach a different answer. What it gives up is stated in
`verify_keys.py`: a step answered from the cache was proved against the line numbers the tree had
then. Only a *green* step is recorded, each one the moment it passes -- so a run that goes red at
`test` keeps the seven verdicts before it -- an entry expires after an hour, a red step's entry
is deleted, and `--no-cache` runs everything.

`build` is the one step with a second condition. `nvs-fmt`, `test`, the `.nvst` trees, `reference`
and `extension` use what `build` leaves on disk, and a key cannot say what another cargo command
has left there since. So `build` is answered from the cache only when every one of those is too
-- or, for `test`, when the test binaries can be shown to be the right ones without cargo.

That showing is `jobs_on_disk`. A comment or a re-wrapped line in a `.rs` file changes what the
test binaries *read* and nothing about what they *are*, and cargo would still recompile every
crate above the edit to say so, which is most of what such a run costs. So each time `test` builds,
it records the code-tier key it built from and the size and modification time of every file cargo
produced for a workspace package. While that key holds and every one of those files is untouched,
they are the executables this code compiles to, and `test` runs them as recorded. Any other cargo
command that rebuilt one moves its modification time, and the step goes back to cargo.

A `-p` run's `test` verdict is keyed with its package, so it never satisfies an unscoped run; an
unscoped one satisfies any `-p`, because the superset already proved the subset. Anything
unexpected -- an unreadable file, a corrupt cache -- runs the steps for real.

## Why `nvs-fmt` formats

`crates/nvs-fmt/tests/identity.rs` holds every `.nvs` file under `tests/` and `examples/` to the
formatter's own layout, and a program written from the reference chapters is not in it: the
chapters double-quote a plain string and the formatter single-quotes it. The failure arrived at
`test`, in a crate the session never touched, and the fix was always the same command and the
whole run again -- the measurement in the section above, which six playbook bullets had not moved.
It is `fmt`'s argument over again, so it has `fmt`'s answer: the step formats.

It runs after `build`, because the formatter is the binary, and over only the `.nvs` files git
reports as new or modified under those two trees, `tests/fmt/input/` excepted. That bound is the
point: the corpus at large stays the identity test's to judge, so a layout rule that changes what
the formatter prints still fails there and is never applied to a thousand files in silence. A file
that does not parse is refused by the formatter and left as it was; whether it was meant not to
parse is a test's verdict, so the step is never red. The cache remembers the bytes it has been
over, so an unchanged file is not a reason to run the step, or `build`, again.

What this trades is what `fmt` trades: a second writer's new `.nvs` file is formatted too.

## Why `test` runs its binaries side by side

`cargo test` runs the workspace's test binaries one after another, and libtest's threads only ever
share out the tests inside one binary, so the longest step here left most of the machine idle. The
step is still `cargo test`'s verdict over `cargo test`'s tests. `cargo test --no-run` builds exactly
the binaries `cargo test` would run and names them; each is run where cargo runs it -- its
package's directory, with `CARGO_MANIFEST_DIR` set, as `tools/loop.py`'s `crate_tests` runs them --
as many at a time as there are cores, the slowest of the last run first (`TEST_TIMES`); and `cargo
test --doc` runs beside them for the doc-tests no binary holds. The passed, failed and ignored
counts come out the same as cargo's.

Two things differ, both on purpose. Every binary runs, where cargo stops at the first that fails,
so a red step names every failing binary at once. And a binary that fails is run a second time,
alone. One that passes alone failed because of what ran beside it -- a fixed port, a fixed path under
the shared temp or target directory, a container name, a timeout that load breaks -- and it is
reported as that failure, red, rather than retried into green. The fix is always to give the test
its own resource, never to run it apart: a clash that serial running hides is the same clash waiting
for the loop and a person to run the suite at the same moment.

## Why steps overlap

Two stretches of the list leave most of the machine idle when run one step at a time, and each
runs at the same time instead.

The script steps (`BESIDE_BUILD`) read source and nothing `build` produces, and `build` reads
nothing they write, so they run beside it. A red script step is still the one reported, since it
comes first in the list; the `build` that ran beside it is not recorded.

`test` ends on a tail: its slowest binaries still running and most cores free. So once every job
has started and at most half the cores are still running one, the steps after `test` start on one
lane, one at a time and in list order. `clippy` waits there while `cargo test --doc` holds cargo's
build-directory lock, which costs the lane time but never a wrong verdict. Nothing starts on the
lane after a binary has failed. When one fails after the lane started, the lane finishes its
current step and stops before the failed binary is run again alone, so the second run is still
alone. The verdicts are taken in list order after `test` finishes: a red `test` is reported and
the lane's verdicts are dropped, green or not. The headline time is the wall clock of the run,
which is no longer the sum of its steps' times.

What this trades: the test binaries still running at the tail share the machine with the lane.
A test that fails under that load and passes alone is reported the way the section above
reports one, and the fix is the one that section names.

## Why `test` runs only the binaries a change reaches

The step's own key is every input in the tree, so an edit anywhere used to run all of the
workspace's test binaries: a new file under `tests/hostile/` ran `nvs-server`'s unit tests, and a
one-line edit to `nvs-lsp` ran `nvs-syntax`'s. `tools/impact.py` keys each binary on what *it*
reads -- its own package as bytes, the workspace packages it is compiled against at the code tier,
and what it opens while it runs when it says so through `nvs_repo` -- and a binary whose key has
not moved is answered from `TEST_GREEN` with the `test result:` line it printed when it was green,
so the step's counts stay the workspace's.

Every doubt resolves wide: a binary whose sources leave their package some way `nvs_repo` does
not record keeps the whole-tree key, and so does one whose dep-info or package cannot be found.
`python tools/impact.py` lists which binaries are narrow and why the others are not, and
`--explain <path>` says what an edit to a path re-runs. A verdict here has no expiry, unlike a
step's: its key holds every byte the binary can read, which is the whole of the argument.
`--no-cache` runs every binary, and a binary that fails loses its entry.

## Why `doc` runs when a goal ends rather than as a step

`cargo doc --no-deps --workspace` resolves every ``[`Foo::bar`]`` in a doc comment. The lint it
denies, `broken_intra_doc_links`, is warn-by-default and invisible to `build` and to `clippy`
alike -- 391 of them had accumulated when it was first run, 96 naming an item that does not
exist -- so it has to run somewhere.

Not here, because it is the dearest gate for the least consequential finding. A broken link stops
no build and changes no behaviour -- the code examples in a doc comment are `test`'s `doc-tests`
job, which does run every time. And its price is set by the crate graph, not the edit: rustdoc
re-documents the edited crate and every workspace crate above it, one after another, so an edit
low in the graph pays for most of the workspace on every run.

So it is `--doc`, alone, and `tools/loop.py`'s `doc_gate` runs it once, on the acceptance sweep
that would declare a goal reached, and holds the goal open while it is red. The tree a goal leaves
behind is what has to be clean; between those points a goal may carry stale links, and the session
that writes `DONE` runs `--doc` first and fixes them (`docs/agent/session-prompt.md`). One that did
not finds the finding in the next pack under *THE RUSTDOC GATE IS RED*.

## Why the documentation gates are not steps here

`rules.py --check`, `records.py --check`, `check-links.py`, `plan.py --check` and `playbook.py --check`
all exit non-zero on a structural finding, and all of them are Python-only and finish in about a
second together, so they look like cheap steps to add in front of `build`. They are CI's `docs` job
instead, and
the reason is the section above on the green cache: its keys read `crates/`, `benches/`, `tests/`,
`examples/`, `editors/` and `docs/reference/` -- and, `owners` apart, nothing else under `docs/`,
on purpose, because "prose cannot break a build" is exactly what makes a re-run after step 4 free.

Adding a docs gate here would break that either way it went. Left as it is, the gate would be
skipped by a cache hit in precisely the case it exists for -- a session edits the plan, re-runs
this, and gets a green verdict computed before the edit. Fixed by hashing `docs/`, every step-4
doc edit would invalidate the cache and buy back the forty seconds the cache was measured saving
in 33 of 41 sessions. A gate whose inputs the cache deliberately ignores does not belong behind
the cache.

Two checks that look like docs gates *are* steps here, and neither is an exception, because the
cache already hashes what each one reads. `reference.py` reads the binary `build` produced, and
`docs/novis.md` is written rather than read. `owners.py --check` reads the `# Known gaps` blocks in
`crates/` and resolves each owner against the plan and the goal chain, and all three are in that
step's key and the last two in no other; the registers it also reports on, under `docs/agent/`, it
counts and never refuses. The
session-side gate for the rest is `session.py --wrap`, which refuses at the
moment a wrap would write the breakage -- see its `playbook_collisions`, its `link_findings` and its
`rulebook_findings`. The second of those is `check-links.py` over the tree, diffed against HEAD: a
link *this* session broke refuses the wrap, and one it inherited does not, so the gate never charges
a session for another's. The third is the same idea for the rulebook, where there is no per-file
baseline to diff against, so the session's own edit under `docs/rules/` is what arms it.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import impact
import verify_keys as keys

ROOT = Path(__file__).resolve().parent.parent
TMP = ROOT / ".agent-tmp"
CACHE = TMP / "verify-green.json"
PROGRESS = TMP / "verify-progress.json"  # the step in flight; see the module doc
# Each test binary's seconds in the last run, so the next one starts the slowest first: started
# last, one long binary is the whole step's tail. A missing or unreadable file only costs order.
TEST_TIMES = TMP / "verify-test-times.json"
# What the last `cargo test --no-run` built, and from which code -- `jobs_on_disk`.
TEST_BUILT = TMP / "verify-test-built.json"
# Each test binary's green verdict: `{job name: {"key": ..., "result": ...}}`, the key being
# `tools/impact.py`'s over what that binary reads -- *Why `test` runs only the binaries a change
# reaches* in the module doc. Where each binary's run-time reads are logged is `READS_DIR`.
TEST_GREEN = TMP / "verify-test-green.json"
READS_DIR = TMP / "reads"

TAIL_LINES = 60  # of the failing step only; the full log is always on disk
CACHE_TTL = 3600  # seconds. A key cannot go stale on its own; this is a belt on braces.

# What each step reads is `tools/verify_keys.py`'s: the walked directories, the named files and
# each step's partition of them. The three facts about a step that are not a set of files are here.
#: The steps that use what `build` leaves on disk. `build` is answered from the green cache only
#: when every one of these is -- *Why a step whose inputs did not change is not run*.
NEEDS_BINARY = {"nvs-fmt", "test", "conformance", "differential", "reference", "extension"}
#: The steps that rewrite source, after which every later step's key is taken again.
WRITES = {"fmt", "nvs-fmt"}
#: The script steps, which run at the same time as `build` -- *Why steps overlap*.
BESIDE_BUILD = {"lints", "directives", "template", "owners", "fuzz-lock"}
#: Where `nvs-fmt` looks for a new or modified `.nvs` file, and the one directory under them whose
#: files are unformatted on purpose. The first is `crates/nvs-fmt/tests/identity.rs`'s corpus.
NVS_FMT_TREES = ("tests", "examples")
NVS_FMT_SKIPS = "tests/fmt/input/"
EXTENSION = ROOT / "editors" / "vscode"

# `cargo test` prints one of these per test binary.
RESULT_RE = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed")
# One test's line in libtest's output: `test <path> ... ok`, `... ignored`.
TEST_LINE_RE = re.compile(r"test \S+ \.\.\. \w+")
# `nvs test <dir>` prints exactly one of these, at the end.
CASES_RE = re.compile(r"(\d+) passed, (\d+) failed, (\d+) skipped")

#: The `.nvst` trees, and the binary that executes them.
#:
#: This is the DEBUG binary on purpose, and it is the one decision in this file worth stating.
#: `tools/loop.py`'s acceptance check -- the thing that actually judges a session -- builds
#: `cargo build -p nvs-cli` and runs these same trees through `target/debug/nvs`. So the debug
#: binary is not a cheaper approximation of the verdict; it *is* the verdict, and the release
#: binary is the approximation.
#:
#: The cost difference is the whole reason this step can exist at all. Measured over one
#: 21-session run: `cargo build --release -p nvs-cli` took 125s (thin LTO at
#: `codegen-units = 1` relinks the world for a one-line stdlib edit) and nine sessions paid it,
#: 21 minutes in all -- 8% of the run's entire wall clock. The same tree's debug build, after
#: the `build` step above has already run, took **2s** in all 21 acceptance checks, and these
#: two trees took 8s and 6s. A session was paying two minutes for a verdict it could have had
#: in fourteen seconds, and a less faithful one.
CASE_TREES = ("conformance", "differential")
# clippy/rustc summary lines worth surfacing above the raw tail.
WARN_RE = re.compile(r"^(warning|error)(\[[^\]]+\])?: (.*)$", re.MULTILINE)


class Step:
    """One command, its exit status, and a one-line summary of what it said.

    `exe`/`cwd` exist because from M4B the workspace is no longer only Rust: `editors/vscode` is a
    TypeScript package whose suites are `npm` scripts, and an `npm` script only finds its
    `package.json` from the directory holding it. Everything else here is `cargo` in the repo root
    and says so by omission.

    `env` is the third of the same kind of exception, and so far the `doc` step's alone:
    `broken_intra_doc_links` is a rustdoc lint rather than a rustc one, so it is set through
    `RUSTDOCFLAGS` and not on the command line.

    `runner` replaces the one command with a function returning `(exit status, output)`, and is the
    `test` step's alone: `args` is then what a reader types to reproduce the step, and `run_tests`
    is what actually runs."""

    def __init__(self, name, args, summarize, exe="cargo", cwd=None, env=None, runner=None):
        self.name = name
        self.args = args
        self.summarize = summarize
        self.exe = exe
        self.cwd = cwd or ROOT
        self.env = env
        self.runner = runner
        self.seconds = 0.0
        self.code = None
        self.out = ""

    @property
    def cmd(self):
        return f"{self.exe} " + " ".join(self.args)


def run(step):
    started = time.monotonic()
    try:
        if step.runner is not None:
            step.code, step.out = step.runner(step)
        else:
            p = subprocess.run(
                [step.exe, *step.args],
                cwd=step.cwd,
                capture_output=True,
                encoding="utf-8",
                errors="replace",
                env=dict(os.environ, **step.env) if step.env else None,
            )
            step.code, step.out = p.returncode, (p.stdout or "") + (p.stderr or "")
    except OSError as exc:
        step.code, step.out = -1, f"could not run `{step.cmd}`: {exc}"
    step.seconds = time.monotonic() - started

    TMP.mkdir(exist_ok=True)
    (TMP / f"verify-{step.name}.log").write_text(step.out, encoding="utf-8", newline="\n")
    return step.code == 0


def jobs_on_disk(binary_key):
    """The jobs the last build recorded, if they are still what this code compiles to: the same
    code-tier key, and every file cargo produced then untouched since. None sends `test` back to
    cargo -- the module docstring's *Why a step whose inputs did not change is not run*."""
    if not binary_key:
        return None
    try:
        built = json.loads(TEST_BUILT.read_text(encoding="utf-8"))
        if built.get("binary") != binary_key or not built.get("jobs"):
            return None
        for path, (mtime, size) in built["files"].items():
            st = os.stat(path)
            if st.st_mtime_ns != mtime or st.st_size != size:
                return None
        return built["jobs"]
    except (OSError, ValueError, TypeError, AttributeError, KeyError):
        return None


def test_jobs(package, binary_key=None):
    """What `cargo test` would run, as jobs -- or `(None, output)` when the build fails.

    A job is `{name, owner, argv, cwd, env, rerun}`, `owner` being its package. The build is the
    bare `cargo test --no-run`, so on a
    tree `build` just compiled it costs only the test harnesses, and its diagnostics are rendered
    to stderr as a plain `cargo test` would print them. `package` narrows which of the binaries
    then run and never what is built: a `-p` on the build resolves features over that one
    package's graph and writes a second copy of every workspace crate beside the first --
    AGENTS.md's rule, and commands.md § *A debug cargo command never takes `-p`* for the
    measurement. A scoped run skips the doc-tests, which cargo can only narrow with a `-p`. The
    package is read off `package_id` the way `tools/loop.py`'s `test_executables` reads it.

    With `binary_key`, the code-tier key of the tree, the build is skipped when `jobs_on_disk`
    can answer for it, and recorded under that key when it cannot."""
    jobs = jobs_on_disk(binary_key)
    note = "test binaries as last built: this code and those files are unchanged\n"
    if jobs is None:
        jobs, note = build_test_jobs(binary_key)
        if jobs is None:
            return None, note
    jobs = [j for j in jobs if not package or j["owner"] == package]
    if not package:
        jobs.append({"name": "doc-tests", "owner": "", "argv": ["cargo", "test", "--doc"],
                     "cwd": str(ROOT), "env": {}, "rerun": "cargo test --doc"})
    return jobs, note


def build_test_jobs(binary_key):
    """`cargo test --no-run`, as every workspace test binary's job -- or `(None, output)`."""
    built = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json-render-diagnostics"],
        cwd=ROOT, capture_output=True, encoding="utf-8", errors="replace")
    if built.returncode != 0:
        return None, (built.stderr or "") + (built.stdout or "")
    flags = {"lib": "--lib", "bin": "--bin", "test": "--test", "example": "--example",
             "bench": "--bench"}
    jobs, files = [], {}
    for line in built.stdout.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") != "compiler-artifact":
            continue
        if m.get("package_id", "").startswith("path+"):
            # Every file of a workspace package, not the test binaries alone: a test spawns
            # `target/debug/nvs`, and that is the `bin` artifact beside it.
            for name in m.get("filenames", []):
                try:
                    st = os.stat(name)
                    files[name] = [st.st_mtime_ns, st.st_size]
                except OSError:
                    pass
        if not m.get("executable"):
            continue
        if not m.get("profile", {}).get("test"):
            continue
        source, _, tail = m.get("package_id", "").rpartition("#")
        owner = tail.split("@", 1)[0] if "@" in tail else source.rstrip("/").rsplit("/", 1)[-1]
        target = m["target"]["name"]
        kind = next((k for k in m["target"].get("kind", []) if k in flags), "lib")
        cwd = str(Path(m["manifest_path"]).parent)
        # The reproduction a reader types stays inside the same build: a target flag under
        # `cargo test` narrows the run and keeps every hash, a `-p` would not.
        rerun = (f"python tools/verify.py -p {owner}" if kind == "lib"
                 else f"cargo test {flags[kind]} {target}")
        jobs.append({"name": f"{owner} {kind} {target}", "owner": owner, "argv": [m["executable"]],
                     "cwd": cwd, "env": {"CARGO_MANIFEST_DIR": cwd}, "rerun": rerun})
    if binary_key and jobs:
        try:
            TMP.mkdir(exist_ok=True)
            TEST_BUILT.write_text(json.dumps({"binary": binary_key, "jobs": jobs, "files": files}),
                                  encoding="utf-8", newline="\n")
        except OSError:
            pass
    return jobs, ""


def run_job(job):
    """One job to completion: `(seconds, exit status, output)`."""
    started = time.monotonic()
    try:
        p = subprocess.run(job["argv"], cwd=job["cwd"], capture_output=True, encoding="utf-8",
                           errors="replace", env=dict(os.environ, **job["env"]))
        code, out = p.returncode, (p.stdout or "") + (p.stderr or "")
    except OSError as exc:
        code, out = -1, f"could not run `{' '.join(job['argv'])}`: {exc}"
    return time.monotonic() - started, code, out


def run_tests(step, package=None):
    """The `test` step -- the module docstring's *Why `test` runs its binaries side by side*.
    `package` is `-p`: which binaries run, off the one build. `step.skip_doc` drops the doc-tests
    job, the one job with a key of its own, and `step.doc_green` reports it back when it ran.
    `step.binary_key` is the tree's code-tier key, which lets `test_jobs` skip the build."""
    jobs, note = test_jobs(package, getattr(step, "binary_key", None))
    if jobs is None:
        return 1, note
    if getattr(step, "skip_doc", False):
        jobs = [j for j in jobs if j["name"] != "doc-tests"]
    try:
        last = json.loads(TEST_TIMES.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        last = {}
    if not isinstance(last, dict):
        last = {}

    # Which binaries this change reaches -- *Why `test` runs only the binaries a change reaches*.
    # With no tree to key against, every binary runs and nothing is remembered.
    tree = getattr(step, "tree", None)
    reach = impact.Reach(tree) if tree is not None else None
    green = load_test_green() if reach is not None and not getattr(step, "no_cache", False) else {}
    binaries = [j for j in jobs if j["name"] != "doc-tests"]
    held = {}
    if reach is not None:
        for j in binaries:
            entry = green.get(j["name"])
            if isinstance(entry, dict) and entry.get("key") == reach.key(j)[0]:
                held[j["name"]] = entry
    jobs = [j for j in jobs if j["name"] not in held]
    for j in jobs:
        if reach is not None and j["name"] != "doc-tests":
            log = READS_DIR / (re.sub(r"\W+", "-", j["name"]) + ".log")
            try:
                READS_DIR.mkdir(parents=True, exist_ok=True)
                log.unlink(missing_ok=True)
            except OSError:
                pass
            j["env"] = dict(j["env"], **{impact.READS_ENV: str(log)})
            j["reads_log"] = log

    # Unknown first: a binary with no recorded time is new, and new is as likely to be slow.
    jobs.sort(key=lambda j: -float(last.get(j["name"], float("inf"))))
    workers = os.cpu_count() or 4
    # The tail: every job started and at most half the cores still running one. `main` starts
    # the steps after this one then -- *Why steps overlap* -- unless a binary has already failed.
    left, red, lock = [len(jobs)], [False], threading.Lock()
    on_tail = getattr(step, "on_tail", None)

    def one(job):
        got = run_job(job)
        with lock:
            left[0] -= 1
            red[0] = red[0] or got[1] != 0
            if on_tail is not None and not red[0] and left[0] <= workers // 2:
                on_tail()
        return got

    with ThreadPoolExecutor(max_workers=workers) as pool:
        results = dict(zip((j["name"] for j in jobs), pool.map(one, jobs)))

    failed = [j for j in jobs if results[j["name"]][1] != 0]
    step.doc_green = "doc-tests" in results and results["doc-tests"][1] == 0
    # A binary that became wide is a red step, named with the line that made it so. Unscoped
    # runs only: `tools/data/impact-wide.txt` lists the workspace's, and `-p` sees one package's.
    wide = impact.findings(reach, binaries) if reach is not None and not package else []
    # Alone, one at a time, after the pool has drained and the steps started at the tail have
    # stopped: the second run is the diagnosis.
    if failed and getattr(step, "quiesce", None) is not None:
        step.quiesce()
    alone = {j["name"]: run_job(j)[1] == 0 for j in failed}

    if reach is not None:
        stored = load_test_green()
        for j in jobs:
            if j["name"] == "doc-tests":
                continue
            if results[j["name"]][1] != 0:
                stored.pop(j["name"], None)
                continue
            # What it opened first, because the key that stands for this run holds those reads.
            reach.record(j, j["reads_log"])
            out = results[j["name"]][2].splitlines()
            lines = [ln for ln in out if RESULT_RE.search(ln)]
            # Every `test <name> ... <status>` line as well, so `tools/loop.py`'s `crate_tests`
            # can answer a check that names tests from this run rather than running the binary a
            # second time over the same inputs.
            tests = [ln.rstrip() for ln in out if TEST_LINE_RE.match(ln)]
            stored[j["name"]] = {"key": reach.key(j)[0], "result": "\n".join(lines),
                                 "tests": tests}
        reach.save()
        try:
            TMP.mkdir(exist_ok=True)
            TEST_GREEN.write_text(json.dumps(stored, indent=1, sort_keys=True), encoding="utf-8",
                                  newline="\n")
        except OSError:
            pass

    try:
        TMP.mkdir(exist_ok=True)
        times = {**last, **{n: round(r[0], 2) for n, r in results.items()}}
        TEST_TIMES.write_text(json.dumps(times, indent=1, sort_keys=True), encoding="utf-8",
                              newline="\n")
    except OSError:
        pass

    # Passing binaries first, by name, so the tail a red step prints is the failures.
    names = {j["name"] for j in failed}
    out = [note] if note else []
    if held:
        out.append(f"{len(held)} of {len(binaries)} test binaries not re-run: nothing each one "
                   f"reads has changed since it was green")
    out += [f"     Unchanged {n}\n{held[n].get('result', '')}" for n in sorted(held)]
    out += [f"     Running {n}\n{results[n][2]}" for n in sorted(results) if n not in names]
    for j in failed:
        out.append(f"     Running {j['name']}  -- FAILED, exit {results[j['name']][1]}\n"
                   f"{results[j['name']][2]}")
    for j in failed:
        if alone[j["name"]]:
            out.append(
                f"error: `{j['name']}` failed beside the other test binaries and passed alone. "
                f"It shares something with a binary that runs at the same time -- a fixed port, a "
                f"fixed path under the shared temp or target directory, a container name -- or "
                f"leans on a timeout that load breaks. Give the test its own (port 0, a directory "
                f"no other test names, its own container) rather than running it apart: "
                f"`tools/verify.py` § *Why `test` runs its binaries side by side*.")
        else:
            out.append(f"error: `{j['name']}` failed, alone as well; `{j['rerun']}` runs it again.")
    out += [f"error: {line}" for line in wide]
    return (1 if failed or wide else 0), "\n".join(out)


def load_test_green():
    """`TEST_GREEN`, or an empty table when it is missing or unreadable -- and then every binary
    runs, which is the safe direction."""
    try:
        got = json.loads(TEST_GREEN.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return got if isinstance(got, dict) else {}


def changed_sources():
    """The `.nvs` files git reports as new or modified under `NVS_FMT_TREES`, ROOT-relative."""
    try:
        p = subprocess.run(["git", "status", "--porcelain", "-z", "--untracked-files=all", "--",
                            *NVS_FMT_TREES], cwd=ROOT, capture_output=True, encoding="utf-8",
                           errors="replace")
    except OSError:
        return []
    if p.returncode != 0:
        return []
    found, entries, i = [], p.stdout.split("\0"), 0
    while i < len(entries):
        entry = entries[i]
        i += 1
        if len(entry) < 4:
            continue
        status, rel = entry[:2], entry[3:]
        if status[0] in "RC":
            i += 1  # `-z` puts the name it was renamed from in the next field
        if ("D" in status or not rel.endswith(".nvs") or rel.startswith(NVS_FMT_SKIPS)
                or not (ROOT / rel).is_file()):
            continue
        found.append(rel)
    return sorted(found)


def unformatted(cache):
    """`(todo, changed)`: the changed `.nvs` files the formatter has not been over as they now
    stand, and all of the changed ones -- the only paths the cache still has a reason to hold."""
    seen = cache.get("formatted", {})
    changed = changed_sources()
    todo = []
    for rel in changed:
        try:
            if seen.get(rel) != keys.digest_of(ROOT / rel):
                todo.append(rel)
        except OSError:
            continue
    return todo, changed


def run_nvs_fmt(step):
    """The `nvs-fmt` step -- the module docstring's *Why `nvs-fmt` formats*. Always exit 0: a
    refusal is a file that does not parse, and whether it was meant to is a test's verdict.
    `step.formatted` is each file's digest as the formatter left it, for the cache."""
    todo = getattr(step, "todo", None)
    if todo is None:
        todo = changed_sources()
    step.formatted = {}
    if not todo:
        return 0, ""
    if not Path(step.exe).is_file():
        return 0, f"note: {step.exe} is not built, so nothing was formatted"
    before = {rel: (ROOT / rel).read_bytes() for rel in todo}
    p = subprocess.run([step.exe, "fmt", *todo], cwd=ROOT, capture_output=True, encoding="utf-8",
                       errors="replace")
    lines = []
    for rel in todo:
        try:
            after = (ROOT / rel).read_bytes()
        except OSError:
            continue
        step.formatted[rel] = keys.digest_of(ROOT / rel)
        if after != before[rel]:
            lines.append(f"rewrote {rel}")
    lines.append(f"looked at {len(todo)} new or modified file(s)")
    return 0, "\n".join(lines) + "\n\n" + (p.stdout or "") + (p.stderr or "")


def summarize_nvs_fmt(out):
    files = [line[len("rewrote "):] for line in out.splitlines() if line.startswith("rewrote ")]
    looked = re.search(r"^looked at (\d+) ", out, re.MULTILINE)
    if not files:
        return f"clean ({looked.group(1)} new or modified)" if looked else "nothing new to format"
    named = ", ".join(Path(f).name for f in files[:3])
    more = f", +{len(files) - 3} more" if len(files) > 3 else ""
    return f"formatted {len(files)} file(s): {named}{more}"


def progress(done, step=None, total=0, finished=None, index=None):
    """Rewrite `PROGRESS`: the step about to run, or -- with `finished` -- the verdict.

    Best effort, and silently so: this is a watcher's convenience, and a convenience that could
    make a verification fail on an unwritable `.agent-tmp` would be the wrong trade."""
    entry = {
        "pid": os.getpid(),
        "at": time.time(),
        "done": [{"name": s.name, "seconds": round(s.seconds, 1)} for s in done],
    }
    if step is not None:
        # `index` is the step's place in the list, which `done` stops giving once a step has
        # been answered from the green cache rather than run.
        entry.update(step=step.name, index=index or len(done) + 1, total=total)
    if finished is not None:
        entry["finished"] = finished
    try:
        TMP.mkdir(exist_ok=True)
        PROGRESS.write_text(json.dumps(entry), encoding="utf-8", newline="\n")
    except OSError:
        pass


# ------------------------------------------------------------------ summaries


def summarize_build(out):
    return "ok"


def summarize_test(out):
    suites = RESULT_RE.findall(out)
    passed = sum(int(a) for a, _ in suites)
    failed = sum(int(b) for _, b in suites)
    if not suites:
        return "ran, but printed no `test result:` line -- check the log"
    unchanged = len(re.findall(r"^     Unchanged ", out, re.MULTILINE))
    note = f", {unchanged} binaries not re-run" if unchanged else ""
    return f"{passed} passed, {failed} failed  ({len(suites)} suites{note})"


def summarize_clippy(out):
    n = len([m for m in WARN_RE.finditer(out) if m.group(1) == "warning"])
    return "no warnings" if n == 0 else f"{n} warning(s)"


def summarize_doc(out):
    n = len([m for m in WARN_RE.finditer(out) if m.group(1) == "warning"])
    return "every link resolves" if n == 0 else f"{n} warning(s)"


def summarize_fmt(out):
    files = [line.strip() for line in out.splitlines() if line.strip().endswith(".rs")]
    if not files:
        return "clean"
    named = ", ".join(Path(f).name for f in files[:3])
    more = f", +{len(files) - 3} more" if len(files) > 3 else ""
    return f"formatted {len(files)} file(s): {named}{more}"


def summarize_cases(out):
    m = CASES_RE.search(out)
    if not m:
        return "ran, but printed no `N passed` line -- check the log"
    passed, failed, skipped = (int(g) for g in m.groups())
    note = f"{passed} passed, {failed} failed"
    return note + (f", {skipped} skipped" if skipped else "")


def summarize_extension(out):
    m = re.search(r"(\d+)\s+passing", out)
    return f"{m.group(1)} passing" if m else "ran, but printed no `N passing` line -- check the log"


def summarize_reference(out):
    m = re.search(r"(\d+) of (\d+) examples hold", out)
    return f"{m.group(1)} of {m.group(2)} examples hold" if m else \
        "ran, but printed no `N of M examples hold` line -- check the log"


def summarize_lints(out):
    m = re.search(r"lints: (\d+) generated tables current", out)
    if m:
        return f"{m.group(1)} generated lint tables current"
    m = re.search(r"(\d+) problem\(s\)", out)
    return f"{m.group(1)} lint table(s) drifted -- run `python tools/lints.py`" if m else \
        "ran, but printed no summary line -- check the log"


def summarize_template(out):
    m = re.search(r"spells all (\d+) leaf keys", out)
    if m:
        return f"the default file spells all {m.group(1)} leaf keys, each once"
    m = re.search(r"(\d+) problem\(s\) over (\d+) leaf key", out)
    return f"{m.group(1)} of {m.group(2)} leaf keys are wrong in the default file" if m else \
        "ran, but printed no summary line -- check the log"


def summarize_directives(out):
    m = re.search(r"directives: (\d+) leaf keys", out)
    if m:
        return f"{m.group(1)} leaf keys, every one read or declared"
    m = re.search(r"(\d+) problem\(s\) over (\d+) leaf key", out)
    return f"{m.group(1)} of {m.group(2)} leaf keys unread and undeclared" if m else \
        "ran, but printed no summary line -- check the log"


def summarize_owners(out):
    m = re.search(r"every one of the (\d+) tagged gap\(s\)", out)
    if m:
        return f"{m.group(1)} recorded gap(s), each deferred to a milestone still ahead"
    m = re.search(r"(\d+) recorded gap\(s\) name an owner this gate refuses", out)
    return f"{m.group(1)} recorded gap(s) name an owner the gate refuses" if m else \
        "ran, but printed no summary line -- check the log"


FUZZ = ROOT / "fuzz"


def run_fuzz_lock(step):
    """Resolve the `fuzz/` workspace, which rewrites `fuzz/Cargo.lock` when it has fallen behind.

    `fuzz/` is its own workspace -- cargo-fuzz requires it -- so nothing the root build does
    touches its lock, and a dependency a crate under `crates/` gains reaches that file only the
    next time cargo runs in `fuzz/`. That is a `cargo +nightly fuzz run` some session later, which
    re-locks as a side effect and leaves the file dirty under a session that did not cause it.
    Resolving here puts the rewrite in the session that added the dependency, and
    `tools/session.py`'s `GENERATED` puts it in that session's commit.

    `cargo metadata` is the cheapest command that resolves: it compiles nothing and adds what is
    missing without upgrading what is locked. Offline first, because the root build has already
    fetched whatever a workspace crate newly depends on and the index is otherwise a network
    round trip on every verification; the second attempt is for a machine whose cache has never
    held `fuzz/`'s own dependencies. Its JSON is dropped -- the lock file is the whole output."""
    lock = FUZZ / "Cargo.lock"
    before = lock.read_bytes() if lock.is_file() else b""
    args = ["cargo", "metadata", "--format-version", "1", "--manifest-path",
            str(FUZZ / "Cargo.toml")]
    said = ""
    for extra in (["--offline"], []):
        p = subprocess.run(args + extra, cwd=ROOT, stdout=subprocess.DEVNULL,
                           stderr=subprocess.PIPE, encoding="utf-8", errors="replace")
        said += p.stderr or ""
        if p.returncode == 0:
            break
    after = lock.read_bytes() if lock.is_file() else b""
    if p.returncode == 0:
        said += ("fuzz-lock: rewrote fuzz/Cargo.lock\n" if after != before
                 else "fuzz-lock: fuzz/Cargo.lock in step\n")
    return p.returncode, said


def summarize_fuzz_lock(out):
    if "rewrote fuzz/Cargo.lock" in out:
        return "fuzz/Cargo.lock had fallen behind the workspace and was rewritten -- commit it"
    return "fuzz/Cargo.lock in step with the workspace" if "in step" in out else \
        "ran, but printed no summary line -- check the log"


def doc_step(opts):
    """The rustdoc gate: every ``[`Foo::bar`]`` in a doc comment, resolved.

    `private_intra_doc_links` is allowed rather than fixed: these are internal crates nobody
    publishes, a link to a crate-private item is a correct reference that rustdoc simply will not
    turn into an anchor, and denying it would be a rule against citing the code by name.

    One home for the command, with two callers -- `tools/loop.py`'s goal-end gate and a by-hand
    `--doc`. The module docstring says why it is not one of `steps_for`'s steps. Always the whole
    workspace, whatever `-p` says: a `-p` here would check every workspace crate a second time under
    its own hashes (AGENTS.md's rule), and the gate is periodic, not a mid-work check."""
    return Step("doc", ["doc", "--no-deps", "--workspace"], summarize_doc,
                env={"RUSTDOCFLAGS": "-A rustdoc::private_intra_doc_links -D warnings"})


def steps_for(opts):
    # `--doc` is the whole run rather than an addition to it. The gate is periodic and its inputs
    # are doc comments, so pairing it with build/test/clippy would put back exactly the 42 seconds
    # a session stopped paying.
    if opts.doc:
        return [doc_step(opts)]
    steps = []
    if not opts.fast:
        # Write mode, first, with its exit status held until the end -- *Why `fmt` formats* in
        # the module docstring. `-l` is rustfmt's: name each file rewritten, which is what the
        # summary line quotes. It takes no -p in the shape this workspace uses it, and the whole
        # tree is two seconds.
        steps.append(Step("fmt", ["fmt", "--all", "--", "-l"], summarize_fmt))
        # Before the compile steps, because it decides what they enforce. `[workspace.lints]` is
        # the one home for the lint policy, but the crates that hold `unsafe` cannot inherit it --
        # cargo refuses a manifest that inherits the workspace table and overrides one entry -- so
        # they restate it and `tools/lints.py` generates those copies. A drifted copy does not
        # fail a build; it silently makes `clippy` below mean something weaker for one crate,
        # which is how `benches/abi-probe` came to be missing seven of them. Sub-second, and
        # unscoped: it reads manifests, so `-p` has nothing to narrow.
        steps.append(Step("lints", ["tools/lints.py", "--check"], summarize_lints,
                          exe=sys.executable))
        # Beside `lints`, and before the compile steps for the same reason: it decides what a
        # key in `nvs.toml` *means*, which no amount of compiling answers. `deny_unknown_fields`
        # makes `crates/nvs-config/src/tree.rs` the accepted key set exactly, and a key that
        # parses but reaches no reader is worse than one that is refused -- the operator writes
        # it, the file is accepted, and the setting silently does nothing. Unscoped: it reads
        # one Rust file and greps the rest, so `-p` has nothing to narrow.
        #
        # Every key the tree parses is decided one of three ways -- a reader, a deletion, or an
        # `[unread: … owner: …]` trailer on the field's own doc comment -- and pre-marking a key
        # to keep this step green is how a gate becomes a rubber stamp.
        steps.append(Step("directives", ["tools/directives.py", "--check"],
                          summarize_directives, exe=sys.executable))
        # The same roster against the file an operator actually reads. A key the tree gains and
        # `crates/nvs-config/src/default.toml` omits is one nobody discovers; a key that file spells
        # and the tree does not parse is one the next boot refuses. A separate step rather than a
        # flag on the one above because the two fail for different reasons and a summary line that
        # had to cover both would name neither.
        steps.append(Step("template", ["tools/directives.py", "--check-template"],
                          summarize_template, exe=sys.executable))
        # Who owns what a crate says it still owes. The gate is `tools/owners.py`'s: an item under
        # a `# Known gaps` block names a milestone still ahead of the program whose plan file
        # states the scope, and nothing else does. It is a step rather than a docs gate for
        # `reference.py`'s reason -- its input is the doc comments in `crates/`, which the green
        # cache hashes -- and the moment worth catching a wrong owner is the one the gap is
        # written in, not the acceptance sweep a session later. The plan and the chain it resolves
        # an owner against are in this step's key and no other's (`verify_keys.OWNERS_READS`); the
        # registers it reads under `docs/agent/` are counted and never refused.
        # Sub-second, and unscoped: it walks every crate's doc comments, so `-p` narrows nothing.
        steps.append(Step("owners", ["tools/owners.py", "--check"], summarize_owners,
                          exe=sys.executable))
        # The last of the steps that cost under a second, and the second that writes: `fuzz/` is
        # a workspace of its own, so its lock follows a dependency added under `crates/` only
        # when something resolves it, and `run_fuzz_lock` is that something. Unscoped, because a
        # lock is resolved over the whole graph whatever `-p` says.
        if (FUZZ / "Cargo.toml").is_file():
            steps.append(Step("fuzz-lock", ["metadata", "--format-version", "1", "--offline",
                                            "--manifest-path", "fuzz/Cargo.toml"],
                              summarize_fuzz_lock, runner=run_fuzz_lock))
    # Bare, whatever `-p` says: these are the shapes `tools/disk.py`'s `LIVE_QUERIES` keep, and a
    # `-p` build resolves features over one package's graph and writes a second copy of every
    # workspace crate beside the first -- AGENTS.md's rule. `-p` narrows which test binaries
    # `run_tests` runs, and nothing else.
    steps.append(Step("build", ["build"], summarize_build))
    exe = ROOT / "target" / "debug" / ("nvs.exe" if os.name == "nt" else "nvs")
    # Straight after `build`, because the formatter is the binary, and before `test`, because
    # `nvs-fmt`'s identity test is what an unformatted file fails -- *Why `nvs-fmt` formats*.
    # Whole-workspace runs only, for the case trees' reason below: no trustworthy binary otherwise.
    if not opts.fast and not opts.package:
        steps.append(Step("nvs-fmt", ["fmt", "<each new or modified .nvs under tests/, examples/>"],
                          summarize_nvs_fmt, exe=str(exe), runner=run_nvs_fmt))
    steps.append(Step("test", ["test"], summarize_test,
                      runner=lambda step: run_tests(step, opts.package)))
    if not opts.fast:
        # The `.nvst` trees, through the binary `build` above just produced. Until this step
        # existed, `verify.py` ran no case at all: a case that failed to compile, or whose
        # `--EXPECT--` was one byte off, left verify green and failed the *driver's* acceptance
        # check one session later, which is the most expensive place for it to fail. It runs
        # after `test` so a Rust fault is reported by the Rust step, not discovered here.
        #
        # Scoped runs skip it: `cargo build -p nvs-ir` does not produce the CLI, and a stale
        # binary would answer a question about a tree it predates.
        if not opts.package:
            for tree in CASE_TREES:
                if (ROOT / "tests" / tree).is_dir():
                    steps.append(Step(tree, ["test", f"tests/{tree}"], summarize_cases,
                                      exe=str(exe)))
        # `docs/novis.md`, regenerated from the binary `build` produced and the chapters under
        # `docs/reference/`, with every example in it run. It writes the file in place -- that
        # is how the reference follows the registry without a session remembering to -- and
        # fails on an example the binary no longer agrees with. `docs/novis.md` itself is not
        # a hashed input (it is derived), so the write does not invalidate the green cache;
        # `docs/reference/` is, so a chapter edit is a real change. `tools/reference.py`'s
        # module doc owns the rest. Scoped and `--fast` runs skip it for the reason the case
        # trees are skipped: no whole-workspace build, no trustworthy binary.
        if not opts.package:
            steps.append(
                Step("reference", ["tools/reference.py"], summarize_reference,
                     exe=sys.executable)
            )
        steps.append(
            Step("clippy", ["clippy", "--all-targets", "--", "-D", "warnings"],
                 summarize_clippy)
        )
        # The VS Code extension's headless suites -- the TextMate grammar snapshots, the
        # contributions/dependency-allowlist test and the LSP protocol round-trip. No editor, no
        # display, no network. It runs LAST because it is the only step that is not `cargo`: a
        # Rust failure should be reported by the Rust steps, not discovered here.
        #
        # Present-and-absent are both real states rather than one being an error. Before M4B the
        # directory does not exist, and `rule:ide/editor-clients-live-under-editors` is explicit that
        # nothing sits scaffolded ahead of its milestone. Once it does exist, a missing `node` is a
        # machine that is not set up (docs/setup.md) and this says so rather than passing quietly.
        if not opts.package and (EXTENSION / "package.json").is_file():
            steps.append(
                Step("extension", ["run", "--silent", "test:headless"], summarize_extension,
                     exe="npm.cmd" if os.name == "nt" else "npm", cwd=EXTENSION)
            )
    return steps


def shown(step):
    """One step's command as a reader would type it, which is not always what `run` spawns.

    `sys.executable` is an absolute interpreter path and the case-tree steps' `exe` is an absolute
    `target/debug/nvs`; printed whole they are the widest thing on the line and say nothing the
    name does not. The directory is printed only when it is not the repo root, so the `extension`
    step's `editors/vscode` stands out as the exception it is."""
    exe = "python" if step.exe == sys.executable else Path(step.exe).name
    line = " ".join([exe, *step.args])
    if step.env:
        line = " ".join(f"{k}={v}" for k, v in step.env.items()) + " " + line
    if Path(step.cwd) != ROOT:
        line += f"   (in {Path(step.cwd).relative_to(ROOT).as_posix()})"
    return line


def list_steps(opts):
    """`--list`: the order the gate walks, without walking it.

    The order is the specification -- `fmt` first because it rewrites what everything after it
    reads, `lints` and `directives` before the compile steps because they decide what the tree
    means rather than whether it builds, `nvs-fmt` between the binary it is and the `test` step
    that reads what it writes, the `.nvst` trees
    after `test` so a Rust fault is reported by the Rust step -- and a second copy of that list in
    a document drifts the day a step moves. So this prints `steps_for`'s own list, and `-p`,
    `--fast` and `--doc` narrow the listing exactly as far as they narrow a run.

    Nothing is spawned, no cache is read or written, and the exit status is 0 for a list that came
    out: the question `--list` answers is what the gate *is*, not what the tree currently says."""
    steps = steps_for(opts)
    scope = f" (-p {opts.package})" if opts.package else ""
    print(f"verify: {len(steps)} step(s) in this order{scope}; `--list` runs none of them.")
    for i, step in enumerate(steps, 1):
        print(f"  {i}. {step.name:<13} {shown(step)}")
    return 0


# ------------------------------------------------------------------ the green cache


def take_tree():
    """One reading of every input, or None if anything at all goes wrong -- and then nothing is
    answered from the cache and nothing is recorded in it."""
    try:
        return keys.Tree()
    except (OSError, ValueError):
        return None


def load_cache():
    """`{"steps": {name: {key, when, summary, seconds}}, "formatted": {path: digest}}`, empty for
    a file that is missing, corrupt or in the shape an earlier version of this script wrote."""
    try:
        entry = json.loads(CACHE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        entry = None
    if not isinstance(entry, dict) or entry.get("shape") != 2:
        entry = {}
    return {"shape": 2,
            "steps": entry["steps"] if isinstance(entry.get("steps"), dict) else {},
            "formatted": entry["formatted"] if isinstance(entry.get("formatted"), dict) else {}}


def amend_cache(change):
    """Read the cache, apply `change` to it, write it back. Read again every time because two runs
    overlap by design -- a `--doc` beside a `--start` -- and each owns only the entries it proved."""
    cache = load_cache()
    change(cache)
    try:
        TMP.mkdir(exist_ok=True)
        CACHE.write_text(json.dumps(cache, indent=1), encoding="utf-8", newline="\n")
    except OSError:
        pass


def scope_of(name, opts):
    # `-p` narrows which test binaries run and nothing else, so it is part of that one key.
    return opts.package if name == "test" else None


def held(cache, tree, opts, name):
    """The entry `name` was last green under, if the key it has now is that entry's."""
    if tree is None or opts.no_cache:
        return None
    entry = cache["steps"].get(name)
    if not isinstance(entry, dict) or time.time() - float(entry.get("when", 0)) > CACHE_TTL:
        return None
    # An unscoped `test` verdict proves every `-p`; the reverse does not hold.
    wanted = {tree.key(name, scope_of(name, opts)), tree.key(name)} - {None}
    return entry if entry.get("key") in wanted else None


def record(tree, opts, name, summary, seconds):
    key = tree.key(name, scope_of(name, opts)) if tree is not None else None
    if key is None:
        return
    entry = {"key": key, "when": time.time(), "summary": summary, "seconds": round(seconds, 1)}
    amend_cache(lambda cache: cache["steps"].__setitem__(name, entry))


# ------------------------------------------------------------------ output


def clock(seconds):
    if seconds >= 60:
        return f"{int(seconds) // 60}m{int(seconds) % 60:02d}s"
    return f"{seconds:.0f}s"


def ago(seconds):
    if seconds < 90:
        return f"{int(seconds)}s ago"
    return f"{int(seconds) // 60}m ago"


def tail(text, limit):
    lines = text.rstrip("\n").split("\n")
    if limit <= 0 or len(lines) <= limit:
        return "\n".join(lines), 0
    return "\n".join(lines[-limit:]), len(lines) - limit


def hooks_note():
    """Say once, here, if this clone has not enabled the versioned hooks.

    git does not version `.git/hooks`, so `tools/git-hooks/` is inert until a clone points at it
    and there is nothing in the tree to notice that. This is the one command every session runs,
    which makes it the cheapest place to be told -- it prints a line and never fails a run: a
    missing hook is a setup gap, not a broken tree, and verify.py judges the tree.
    """
    try:
        got = subprocess.run(["git", "config", "core.hooksPath"], cwd=ROOT,
                             capture_output=True, text=True, check=False).stdout.strip()
    except OSError:
        return
    if got.replace("\\", "/").rstrip("/") == "tools/git-hooks":
        return
    print("verify: this clone has no hooks -- `git config core.hooksPath tools/git-hooks`")
    print("        (it rejects attribution trailers; docs/agent/conventions.md says why)\n")


#: Where a `--start` run leaves its output for the matching `--wait`, and the file it writes its
#: exit status into when it is done.
BACKGROUND = ROOT / ".agent-tmp" / "verify-background.log"
DONE = ROOT / ".agent-tmp" / "verify-background.done"

#: How long `--wait` will hold before reporting that something is wrong rather than blocking a
#: session forever. A full run was ~42s when this was set and 146s at its worst since, so it is
#: still generous by a multiple on purpose.
BACKGROUND_TIMEOUT = 600.0


def start_background(argv):
    """Kick the same verification off detached, and return at once.

    Verification is 63.6% of a session's tool-execution time -- 41.9 seconds a run, and a session
    makes 1.9 of them -- while the tail that follows it is the session writing prose it already
    knows. Those two do not depend on each other until the wrap is applied, so the seconds only
    cost anything because they are spent in series.

    `--start` then `--wait` is the harness-neutral way to overlap them: start the run, write the
    wrap file, collect. By the time the wrap is written the verdict is usually already on disk and
    `--wait` returns in the time it takes to read a file. It is the same verification either way --
    the same steps, the same green cache, the same exit status -- so nothing is traded for it."""
    passthrough = [a for a in argv if a not in ("--start", "--wait")]
    BACKGROUND.parent.mkdir(parents=True, exist_ok=True)
    DONE.unlink(missing_ok=True)
    handle = BACKGROUND.open("w", encoding="utf-8", newline="\n")
    proc = subprocess.Popen(
        [sys.executable, str(Path(__file__).resolve()), "--_detached", *passthrough],
        stdout=handle, stderr=subprocess.STDOUT, cwd=ROOT,
    )
    print(f"verify: started in the background (pid {proc.pid}).")
    print("        Write the wrap file now, then `python tools/verify.py --wait` to collect it.")
    return 0


def wait_background():
    """Collect a `--start` run: its whole output, and its own exit status as this call's.

    The child announces itself finished by writing `DONE`, rather than the parent watching a pid.
    A pid is the obvious way and the wrong one here: the process that started the child has long
    since exited, so there is no handle left, and asking the OS whether a bare number is still
    alive is a different question on every platform -- and answers *yes* for a recycled pid."""
    if not DONE.exists() and not BACKGROUND.exists():
        print("verify: nothing was started. `python tools/verify.py --start` first, or just run")
        print("        `python tools/verify.py` -- there is no state to recover here.")
        return 2
    waited = 0.0
    while not DONE.exists():
        time.sleep(0.4)
        waited += 0.4
        if waited > BACKGROUND_TIMEOUT:
            print(f"verify: the background run has not finished after {BACKGROUND_TIMEOUT:.0f}s.")
            print(f"        Its output so far is in {BACKGROUND.relative_to(ROOT)}.")
            return 2
    body = BACKGROUND.read_text(encoding="utf-8") if BACKGROUND.exists() else ""
    sys.stdout.write(body if body.endswith("\n") or not body else body + "\n")
    if waited:
        print(f"-- collected after a further {waited:.0f}s; the rest of the run overlapped "
              "whatever you did in between.")
    try:
        code = int(DONE.read_text(encoding="utf-8").strip() or 0)
    except (OSError, ValueError):
        code = 0
    return code


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("-p", "--package",
                    help="run only this package's test binaries; the build stays the whole tree's")
    ap.add_argument("--fast", action="store_true", help="build and test only")
    ap.add_argument("--doc", action="store_true",
                    help="the rustdoc gate alone; tools/loop.py runs it when a goal's checks pass")
    ap.add_argument("--full", action="store_true", help="do not truncate the failing step")
    ap.add_argument("--no-cache", action="store_true",
                    help="run every step, whatever the green cache holds")
    ap.add_argument("--start", action="store_true",
                    help="run detached and return at once; collect it with --wait")
    ap.add_argument("--wait", action="store_true",
                    help="collect the run --start left, with its exit status")
    ap.add_argument("--list", action="store_true",
                    help="print the steps in the order the gate walks them, and run none of them")
    ap.add_argument("--_detached", action="store_true", help=argparse.SUPPRESS)
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if opts.start and opts.wait:
        print("verify: --start and --wait are two calls, not one flag pair.")
        return 2
    if opts.doc and opts.fast:
        print("verify: --doc is a run of one step; --fast has nothing to narrow.")
        return 2
    if opts.list:
        # Before `--start`/`--wait`, which are two halves of a run: a listing has nothing to
        # detach and nothing to collect, so the pair is a typo rather than a narrower listing.
        if opts.start or opts.wait:
            print("verify: --list runs nothing, so there is nothing to --start or --wait for.")
            return 2
        return list_steps(opts)
    if opts.start:
        return start_background(sys.argv[1:])
    if opts.wait:
        return wait_background()

    steps = steps_for(opts)
    scope = f" (-p {opts.package})" if opts.package else ""
    hooks_note()

    cache = load_cache()
    tree = take_tree()
    for step in steps:
        if step.name == "nvs-fmt":
            step.todo, step.changed = unformatted(cache)

    def answered(step):
        """The green entry that stands in for running `step`, or None if it has to run."""
        if step.name == "nvs-fmt":
            return None if step.todo else {"summary": "nothing new to format", "seconds": 0}
        return held(cache, tree, opts, step.name)

    done = []
    unchanged = {}  # step name -> the entry it was answered from
    failed = None
    deferred = None  # `fmt` red: reported only if nothing after it is -- the module docstring
    began = time.monotonic()

    def needed(i, step):
        """None when `step` has to run; otherwise the green entry that replaces running it."""
        entry = answered(step)
        if entry is not None and step.name == "build":
            # Only if nothing after it will use what `build` leaves -- the module docstring. `test`
            # does not when its binaries are provably the ones this code compiles to.
            def leans_on_build(s):
                if s.name not in NEEDS_BINARY or answered(s) is not None:
                    return False
                return s.name != "test" or jobs_on_disk(tree and tree.key("build")) is None
            if any(leans_on_build(s) for s in steps[i + 1:]):
                entry = None
        if entry is not None:
            unchanged[step.name] = entry
        return entry

    def prepare(i, step):
        if step.name == "test":
            # `--no-cache` is every step for real, and cargo's build is part of this one.
            step.binary_key = (tree.key("build") if tree is not None and not opts.no_cache
                               else None)
            step.tree, step.no_cache = tree, opts.no_cache
            if not opts.package:
                step.skip_doc = held(cache, tree, opts, "test:doc") is not None
        progress(done, step, len(steps), index=i + 1)

    def settle(step, ok):
        """One finished run's verdict, taken in list order; False when it stops the run."""
        nonlocal tree, failed, deferred
        if step.name in WRITES and step.out.strip():
            # It rewrote files, so every verdict from here on belongs to the tree as it now is.
            tree = take_tree()
        if getattr(step, "formatted", None):
            # A path that is no longer new or modified has been committed, and is dropped.
            amend_cache(lambda c: c.__setitem__("formatted", {
                **{k: v for k, v in c["formatted"].items() if k in step.changed},
                **step.formatted}))
        if getattr(step, "doc_green", False):
            record(tree, opts, "test:doc", "ok", 0)
        if step.name == "fmt" and not ok:
            deferred = step
            return True
        if not ok:
            failed = step
            return False
        record(tree, opts, step.name, step.summarize(step.out), step.seconds)
        done.append(step)
        return True

    def beside_build(i):
        """The script steps from `i` on, run at the same time as the `build` after them --
        *Why steps overlap* in the module docstring. Verdicts are still taken in list order."""
        j = i
        while j < len(steps) and steps[j].name in BESIDE_BUILD:
            j += 1
        group = steps[i:j + 1] if j < len(steps) and steps[j].name == "build" else steps[i:j]
        todo = [(k, s) for k, s in enumerate(group, i) if needed(k, s) is None]
        with ThreadPoolExecutor(max_workers=max(len(todo), 1)) as pool:
            running = []
            for k, s in todo:
                prepare(k, s)
                running.append((s, pool.submit(run, s)))
            for s, future in running:
                if not settle(s, future.result()):
                    break
        return len(group)

    def with_tail(i):
        """`test`, and the steps after it started while its slowest binaries are still running
        -- *Why steps overlap*. They run one at a time on one lane, in list order."""
        test, after = steps[i], [(k, s) for k, s in enumerate(steps[i + 1:], i + 1)]
        if needed(i, test) is not None:
            return 1
        after = [(k, s) for k, s in after if needed(k, s) is None]
        tail, stop, ran = threading.Event(), threading.Event(), {}

        def lane():
            tail.wait()
            for k, s in after:
                if stop.is_set():
                    return
                prepare(k, s)
                ran[s.name] = run(s)
                if not ran[s.name]:
                    return

        worker = threading.Thread(target=lane, daemon=True)

        def quiesce():
            # A failed binary is run again alone, and alone means nothing on the lane either.
            stop.set()
            tail.set()
            worker.join()

        test.on_tail, test.quiesce = tail.set, quiesce
        worker.start()
        prepare(i, test)
        ok = run(test)
        if not ok:
            stop.set()
        tail.set()
        worker.join()
        if settle(test, ok):
            for _, s in after:
                if s.name not in ran or not settle(s, ran[s.name]):
                    break
        return 1 + len(steps[i + 1:])

    i = 0
    while i < len(steps) and failed is None:
        step = steps[i]
        if step.name in BESIDE_BUILD or step.name == "build":
            i += beside_build(i)
        elif step.name == "test":
            i += with_tail(i)
        else:
            if needed(i, step) is None:
                prepare(i, step)
                settle(step, run(step))
            i += 1
    if failed is not None:
        # A step after the one that stopped the run was not reached, whatever the cache holds.
        for s in steps[steps.index(failed) + 1:]:
            unchanged.pop(s.name, None)
    if failed is None and deferred is not None:
        failed = deferred
    progress(done, finished=1 if failed else 0)

    total = time.monotonic() - began

    def lines():
        for s in steps:
            if s.name in unchanged:
                print(f"  {s.name:<13} {'--':>6}   {unchanged[s.name].get('summary', 'ok')}")
            elif s in done:
                print(f"  {s.name:<13} {clock(s.seconds):>6}   {s.summarize(s.out)}")

    if failed is None and not done:
        oldest = min((float(e["when"]) for e in unchanged.values() if "when" in e),
                     default=time.time())
        print(f"verify: green, nothing a step reads has changed since "
              f"{ago(time.time() - oldest)} -- nothing to re-run{scope}")
        lines()
        cost = sum(float(e.get("seconds") or 0) for e in unchanged.values())
        print(f"\nthat verdict cost {clock(cost)} and every step's inputs are as they were; "
              f"`--no-cache` runs it again anyway.")
        return 0

    note = (f" -- {len(unchanged)} not re-run, their inputs unchanged (`--` below)"
            if unchanged else "")
    if failed is None:
        print(f"verify: {len(done) + len(unchanged)} of {len(steps)} green in "
              f"{clock(total)}{scope}{note}")
        lines()
        return 0

    amend_cache(lambda c: c["steps"].pop(failed.name, None))
    print(f"verify: FAILED at {failed.name} "
          f"(step {steps.index(failed) + 1} of {len(steps)}) after {clock(total)}{scope}{note}")
    lines()
    print(f"  {failed.name:<13} {'---':>6}   exit {failed.code}")

    body, hidden = tail(failed.out, 0 if opts.full else TAIL_LINES)
    print(f"\n-- `{failed.cmd}`" + (f", last {TAIL_LINES} lines" if hidden else ""))
    print(body)
    if hidden:
        print(f"\n... {hidden} earlier line(s) hidden")
    print(f"\nfull output: .agent-tmp/verify-{failed.name}.log")
    return 1


def detached() -> int:
    """`main`, plus the sentinel that tells a waiting `--wait` the run is over.

    The status is written whatever happens, an unhandled exception included -- a `--wait` that
    blocks for ten minutes because the child died on line one is a far worse failure than a
    verification that reports red."""
    code = 1
    try:
        code = main()
    finally:
        try:
            DONE.parent.mkdir(parents=True, exist_ok=True)
            DONE.write_text(str(code), encoding="utf-8")
        except OSError:
            pass
    return code


if __name__ == "__main__":
    sys.exit(detached() if "--_detached" in sys.argv else main())
