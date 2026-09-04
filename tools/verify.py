#!/usr/bin/env python3
"""AGENTS.md § *Session workflow* step 3, as one command.

`cargo build`, `cargo fmt --check`, `cargo test`, the `.nvst` trees through the binary the build
just produced, `cargo clippy --all-targets -- -D warnings`, and -- once `editors/vscode` exists --
that extension's headless suites, in that order, stopping at the first failure. Green prints one
line per step; a failure prints that step's output and nothing else.

`cargo doc` with rustdoc's broken-link lint denied is the one gate deliberately **not** in that
list. It is `--doc`, run alone, and `tools/loop.py` runs it periodically rather than every
verification -- see *Why `doc` is a periodic gate* below.

The `conformance` and `differential` steps run `target/debug/nvs test tests/<tree>`, which is
exactly what `tools/loop.py`'s acceptance check runs, and they print the two counts the plan's
status fields quote. They cost about fourteen seconds together. **Do not rebuild
`target/release/nvs.exe` to run a case** -- see `CASE_TREES` below for the measurement, and the
playbook under *Running things*.

The point is turn count, not typing. Run separately, those are as many tool calls whose
combined output runs to tens of thousands of tokens a session never reads once it is green --
and a session's wall clock is very nearly its number of turns times a constant. Run here, a
green verification is one call and about ten lines.

    python tools/verify.py                  # every step
    python tools/verify.py -p nvs-ir        # scope build/test/clippy to one package
    python tools/verify.py --fast           # build and test only, for a mid-work check
    python tools/verify.py --doc            # the rustdoc gate alone; the driver's periodic call
    python tools/verify.py --start          # run it detached and return at once
    python tools/verify.py --wait           # collect what --start left, with its exit status
    python tools/verify.py --full           # do not truncate the failing step's output
    python tools/verify.py --no-cache       # re-run even if the tree is provably unchanged

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

## Why `fmt` runs second

It costs a second and needs no build, so a formatting slip should not cost a whole run --
`.loop/logs` holds runs that paid `build=4s test=41s clippy=5s` before failing at `fmt`, and
sessions had started typing `cargo fmt --all && python tools/verify.py` to work around it. It
runs *second* rather than first because `cargo fmt --check` on unparseable code reports a
rustfmt parse error, which is a far worse diagnostic than the one `cargo build` would have
given for the same typo.

## Why a second run on an unchanged tree is free

The rule is one verification per session, at the end. Measured against `.loop/logs`, 33 of 41
sessions ran this script more than once -- 2.4 times on average -- and most of those re-runs
came after step 4 edited only documentation. Prose cannot break a build, so those runs paid
about forty seconds to re-derive a verdict they already held.

So the verdict is cached against a content hash of every input the steps read: every file under
`crates/`, `benches/`, `tests/`, `examples/` and `editors/`, the workspace manifests, `rustfmt.toml`,
`rust-toolchain.toml` and the exact `rustc -vV`. A repeat run whose hash matches prints the
cached verdict and exits, in about a fifth of a second. This is **not** a check being skipped:
the inputs are bit-identical, so re-running the same compiler over them cannot reach a
different answer. Only a *green* verdict is cached, the entry expires after an hour, and
`--no-cache` forces the real thing.

The cache records the scope and the step list it was produced by, so a `--fast` verdict never
satisfies a full run and a `-p nvs-ir` verdict never satisfies an unscoped one; the reverse
directions do, because a superset already proved the subset. Anything unexpected -- an
unreadable file, a corrupt cache -- makes it fall through and run the steps for real.

## Why `doc` is a periodic gate rather than a step

`cargo doc --no-deps --workspace` resolves every ``[`Foo::bar`]`` in a doc comment. The lint it
denies, `broken_intra_doc_links`, is warn-by-default and invisible to `build` and to `clippy`
alike -- 391 of them had accumulated when it was first run, 96 naming an item that does not
exist -- so it has to run somewhere.

It ran here, as a step, until it was measured. Over the 72 sessions in `.loop/logs` it averaged
**41.8s**, against 24.0s for `test`, 12.6s for `differential` and 7.5s for `clippy`; the comment
beside it still claimed twelve, which is what it cost when it was written. That made it 40% of a
green run, and a session reaches a green run about twice -- ~85 seconds a session, 7% of the
loop's entire wall clock, for a lint that fires a handful of times a month.

Two things put it in the wrong place. Its inputs are doc *comments*, which most re-runs of this
script never touch: a re-run after fixing a clippy lint paid the 42 seconds again for an answer
that could not have changed. And `.github/workflows/ci.yml` runs the identical command with the
identical `RUSTDOCFLAGS`, so a push was never going to carry a broken link either way.

So it is `--doc`, alone, and `tools/loop.py` calls it every `DOC_GATE_EVERY` sessions -- between
sessions, where the seconds are the driver's rather than a session's -- and keeps calling it every
session until it is green again (that constant is the one home for the interval). What
that trades away is in-session detection: a broken link can now surface up to `DOC_GATE_EVERY`
sessions after the comment that broke it, named by file and line in the ledger and in the next
pack. Those sessions at 42 seconds each buy that back many times over.

## Why the documentation gates are not steps here

`adr.py --check`, `check-links.py`, `plan.py --check` and `playbook.py --check` all exit non-zero
on a structural finding, and all four are Python-only and finish in about a second together, so
they look like four cheap steps to add in front of `build`. They are CI's `docs` job instead, and
the reason is the paragraph above: the green cache hashes `crates/`, `benches/`, `tests/`,
`examples/`, `editors/` and `docs/reference/` -- and nothing else under `docs/`, on purpose,
because "prose cannot break a build" is exactly what makes a re-run after step 4 free.

Adding a docs gate here would break that either way it went. Left as it is, the gate would be
skipped by a cache hit in precisely the case it exists for -- a session edits the plan, re-runs
this, and gets a green verdict computed before the edit. Fixed by hashing `docs/`, every step-4
doc edit would invalidate the cache and buy back the forty seconds the cache was measured saving
in 33 of 41 sessions. A gate whose inputs the cache deliberately ignores does not belong behind
the cache.

The one docs check that *is* a step here is `reference.py`, and it is not an exception: it reads
the binary `build` produced, so its input is hashed already, and `docs/novis.md` is written
rather than read. The session-side gate for the rest is `session.py --wrap`, which refuses at the
moment a wrap would write the breakage -- see its `playbook_collisions`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TMP = ROOT / ".agent-tmp"
CACHE = TMP / "verify-green.json"
PROGRESS = TMP / "verify-progress.json"  # the step in flight; see the module doc

TAIL_LINES = 60  # of the failing step only; the full log is always on disk
CACHE_TTL = 3600  # seconds. A tree hash cannot go stale on its own; this is a belt on braces.

# Everything cargo reads, relative to ROOT. Directories are walked in full -- a `.nvst`
# fixture, an insta `.snap` and a `Cargo.toml` all change what the steps will answer.
INPUT_DIRS = ("crates", "benches", "tests", "examples", "editors", "docs/reference")
INPUT_FILES = ("Cargo.toml", "Cargo.lock", "rustfmt.toml", "rust-toolchain.toml")
# Directories under an INPUT_DIR that are output or a package cache, never an input. `target` is
# cargo's; the other three belong to `editors/vscode` and between them hold tens of thousands of
# files, which would make the green cache's own hash the slowest thing in this script.
NOT_INPUTS = {"target", "node_modules", "out", ".vscode-test"}
EXTENSION = ROOT / "editors" / "vscode"

# `cargo test` prints one of these per test binary.
RESULT_RE = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed")
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
    `RUSTDOCFLAGS` and not on the command line."""

    def __init__(self, name, args, summarize, exe="cargo", cwd=None, env=None):
        self.name = name
        self.args = args
        self.summarize = summarize
        self.exe = exe
        self.cwd = cwd or ROOT
        self.env = env
        self.seconds = 0.0
        self.code = None
        self.out = ""

    @property
    def cmd(self):
        return f"{self.exe} " + " ".join(self.args)


def run(step):
    started = time.monotonic()
    try:
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


def progress(done, step=None, total=0, finished=None):
    """Rewrite `PROGRESS`: the step about to run, or -- with `finished` -- the verdict.

    Best effort, and silently so: this is a watcher's convenience, and a convenience that could
    make a verification fail on an unwritable `.agent-tmp` would be the wrong trade."""
    entry = {
        "pid": os.getpid(),
        "at": time.time(),
        "done": [{"name": s.name, "seconds": round(s.seconds, 1)} for s in done],
    }
    if step is not None:
        entry.update(step=step.name, index=len(done) + 1, total=total)
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
    return f"{passed} passed, {failed} failed  ({len(suites)} suites)"


def summarize_clippy(out):
    n = len([m for m in WARN_RE.finditer(out) if m.group(1) == "warning"])
    return "no warnings" if n == 0 else f"{n} warning(s)"


def summarize_doc(out):
    n = len([m for m in WARN_RE.finditer(out) if m.group(1) == "warning"])
    return "every link resolves" if n == 0 else f"{n} warning(s)"


def summarize_fmt(out):
    return "clean"


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


def doc_step(opts):
    """The rustdoc gate: every ``[`Foo::bar`]`` in a doc comment, resolved.

    `private_intra_doc_links` is allowed rather than fixed: these are internal crates nobody
    publishes, a link to a crate-private item is a correct reference that rustdoc simply will not
    turn into an anchor, and denying it would be a rule against citing the code by name.

    One home for the command, with two callers -- `tools/loop.py`'s periodic gate and a by-hand
    `--doc`. The module docstring says why it is not one of `steps_for`'s steps."""
    scope = ["--workspace"] if not opts.package else ["-p", opts.package]
    return Step("doc", ["doc", "--no-deps", *scope], summarize_doc,
                env={"RUSTDOCFLAGS": "-A rustdoc::private_intra_doc_links -D warnings"})


def steps_for(opts):
    # `--doc` is the whole run rather than an addition to it. The gate is periodic and its inputs
    # are doc comments, so pairing it with build/test/clippy would put back exactly the 42 seconds
    # a session stopped paying.
    if opts.doc:
        return [doc_step(opts)]
    scope = ["-p", opts.package] if opts.package else []
    steps = [Step("build", ["build", *scope], summarize_build)]
    if not opts.fast:
        # `cargo fmt --check` takes no -p in the shape this workspace uses it, and it is a
        # second, so it goes ahead of the two expensive steps -- see the module docstring.
        steps.append(Step("fmt", ["fmt", "--check"], summarize_fmt))
    steps.append(Step("test", ["test", *scope], summarize_test))
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
            exe = ROOT / "target" / "debug" / ("nvs.exe" if os.name == "nt" else "nvs")
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
            Step("clippy", ["clippy", "--all-targets", *scope, "--", "-D", "warnings"],
                 summarize_clippy)
        )
        # The VS Code extension's headless suites -- the TextMate grammar snapshots, the
        # contributions/dependency-allowlist test and the LSP protocol round-trip. No editor, no
        # display, no network. It runs LAST because it is the only step that is not `cargo`: a
        # Rust failure should be reported by the Rust steps, not discovered here.
        #
        # Present-and-absent are both real states rather than one being an error. Before M4B the
        # directory does not exist, and `docs/adr/0016-ide-integration.md` § 5 is explicit that
        # nothing sits scaffolded ahead of its milestone. Once it does exist, a missing `node` is a
        # machine that is not set up (docs/setup.md) and this says so rather than passing quietly.
        if not opts.package and (EXTENSION / "package.json").is_file():
            steps.append(
                Step("extension", ["run", "--silent", "test:headless"], summarize_extension,
                     exe="npm.cmd" if os.name == "nt" else "npm", cwd=EXTENSION)
            )
    return steps


# ------------------------------------------------------------------ the green cache


def rustc_version():
    p = subprocess.run(["rustc", "-vV"], capture_output=True, encoding="utf-8", errors="replace")
    return (p.stdout or "") + (p.stderr or "")


def input_paths():
    """Every file the steps read, sorted, as ROOT-relative posix strings."""
    seen = []
    for name in INPUT_FILES:
        if (ROOT / name).is_file():
            seen.append(name)
    for top in INPUT_DIRS:
        base = ROOT / top
        if not base.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in NOT_INPUTS]
            rel = Path(dirpath).relative_to(ROOT)
            seen.extend((rel / f).as_posix() for f in filenames)
    seen.sort()
    return seen


def tree_key():
    """A content hash of every input, or None if anything at all goes wrong."""
    try:
        h = hashlib.blake2b(digest_size=16)
        h.update(rustc_version().encode("utf-8", "replace"))
        for rel in input_paths():
            h.update(rel.encode("utf-8"))
            h.update(b"\0")
            h.update((ROOT / rel).read_bytes())
            h.update(b"\0")
        return h.hexdigest()
    except OSError:
        return None


def cached_verdict(key, opts, steps):
    """The stored verdict if it provably covers this request, else None."""
    if key is None or opts.no_cache:
        return None
    try:
        entry = json.loads(CACHE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    if entry.get("key") != key:
        return None
    if time.time() - float(entry.get("when", 0)) > CACHE_TTL:
        return None
    # A scoped verdict cannot stand in for an unscoped one; the reverse is fine.
    if entry.get("package") not in (None, opts.package):
        return None
    if not set(s.name for s in steps) <= set(entry.get("steps", {})):
        return None
    return entry


def store_verdict(key, opts, steps):
    if key is None:
        return
    # Merged into the standing entry, not written over it, when the tree has not moved since that
    # entry was made. `--doc` is a run of one step: replacing a full verdict with it would make the
    # next unscoped run re-derive six green steps over bit-identical inputs, which is the cost the
    # cache exists to remove. Nothing is carried over unless the key and the scope both match, so a
    # merged entry still describes exactly one tree; `cached_verdict`'s subset rule reads it back.
    held, held_seconds = {}, 0.0
    try:
        entry = json.loads(CACHE.read_text(encoding="utf-8"))
        if (entry.get("key") == key and entry.get("package") == opts.package
                and time.time() - float(entry.get("when", 0)) <= CACHE_TTL):
            held = dict(entry.get("steps") or {})
            held_seconds = float(entry.get("seconds") or 0)
    except (OSError, ValueError, TypeError, AttributeError):
        held, held_seconds = {}, 0.0
    held.update({s.name: s.summarize(s.out) for s in steps})
    try:
        TMP.mkdir(exist_ok=True)
        CACHE.write_text(
            json.dumps(
                {
                    "key": key,
                    "when": time.time(),
                    "package": opts.package,
                    "steps": held,
                    "seconds": round(held_seconds + sum(s.seconds for s in steps), 1),
                },
                indent=1,
            ),
            encoding="utf-8",
            newline="\n",
        )
    except OSError:
        pass


def drop_verdict():
    try:
        CACHE.unlink(missing_ok=True)
    except OSError:
        pass


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
#: session forever. A full run is ~42s; this is generous by an order of magnitude on purpose.
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
    ap.add_argument("-p", "--package", help="scope build/test/clippy to one package")
    ap.add_argument("--fast", action="store_true", help="build and test only")
    ap.add_argument("--doc", action="store_true",
                    help="the rustdoc gate alone; tools/loop.py runs it periodically")
    ap.add_argument("--full", action="store_true", help="do not truncate the failing step")
    ap.add_argument("--no-cache", action="store_true",
                    help="re-run the steps even if the tree is provably unchanged")
    ap.add_argument("--start", action="store_true",
                    help="run detached and return at once; collect it with --wait")
    ap.add_argument("--wait", action="store_true",
                    help="collect the run --start left, with its exit status")
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
    if opts.start:
        return start_background(sys.argv[1:])
    if opts.wait:
        return wait_background()

    steps = steps_for(opts)
    scope = f" (-p {opts.package})" if opts.package else ""
    hooks_note()

    key = tree_key()
    hit = cached_verdict(key, opts, steps)
    if hit is not None:
        print(f"verify: green, tree unchanged since {ago(time.time() - hit['when'])}"
              f" -- nothing to re-run{scope}")
        for name, summary in hit["steps"].items():
            print(f"  {name:<13} {'--':>6}   {summary}")
        print(f"\nthat verdict cost {clock(hit['seconds'])} and covers this tree exactly; "
              f"`--no-cache` runs it again anyway.")
        return 0

    done = []
    failed = None
    for step in steps:
        progress(done, step, len(steps))
        if not run(step):
            failed = step
            break
        done.append(step)
    progress(done, finished=1 if failed else 0)

    total = sum(s.seconds for s in done) + (failed.seconds if failed else 0)

    if failed is None:
        store_verdict(key, opts, steps)
        print(f"verify: {len(done)} of {len(steps)} green in {clock(total)}{scope}")
        for s in done:
            print(f"  {s.name:<13} {clock(s.seconds):>6}   {s.summarize(s.out)}")
        return 0

    drop_verdict()
    print(f"verify: FAILED at {failed.name} "
          f"(step {len(done) + 1} of {len(steps)}) after {clock(total)}{scope}")
    for s in done:
        print(f"  {s.name:<13} {clock(s.seconds):>6}   {s.summarize(s.out)}")
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
