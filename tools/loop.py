#!/usr/bin/env python3
"""Unattended MWL work loop. Starts one fresh `claude` session per iteration, so the per-session
context cost is constant no matter how many sessions run. See docs/agent/coordinator.md for the design
and docs/agent/loop-goal.md for the goal; the acceptance list itself is docs/agent/loop-goal.toml.

Runs on Windows, Linux and macOS. On Windows the acceptance test gets a second leg through WSL, because
a JIT is exactly where a calling-convention divergence between two targets hides; on Linux the native leg
already is that target, so there is one leg plus the valgrind sweep.

    python tools/loop.py --max-sessions 300
    python tools/loop.py --max-sessions 300 --full-output   # no truncation anywhere
    python tools/loop.py --goal-only                        # run the acceptance test and exit

Nothing on the stop path depends on a model's self-assessment: every acceptance item is an exit code plus
an exact or ordered-substring match on real output.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import threading
import time
from datetime import datetime
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError:
        sys.stderr.write(
            "loop.py needs TOML support: Python 3.11+, or `pip install tomli` on an older one.\n"
        )
        raise SystemExit(2)

sys.path.insert(0, str(Path(__file__).resolve().parent))
import disk  # noqa: E402  -- same directory; the retention policy has one home and it is there

ROOT = Path(__file__).resolve().parent.parent
PROMPT = ROOT / "docs" / "agent" / "session-prompt.md"
GOAL_MD = ROOT / "docs" / "agent" / "loop-goal.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"
RUNDIR = ROOT / ".loop"
LOGDIR = RUNDIR / "logs"
LEDGER = RUNDIR / "log.md"
STATUS = RUNDIR / "status.txt"
STOP = RUNDIR / "stop"
RUNNING = RUNDIR / "running"
GOALCACHE = RUNDIR / "goal-green.json"

IS_WINDOWS = os.name == "nt"


# --------------------------------------------------------------------------------- console


class C:
    """ANSI colours. Windows 10+ consoles understand them once VT processing is on."""

    RESET = "\033[0m"
    GRAY = "\033[90m"
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    WHITE = "\033[37m"
    DIM_CYAN = "\033[36;2m"
    DIM_GREEN = "\033[32;2m"

    enabled = True

    @classmethod
    def paint(cls, text, colour):
        return f"{colour}{text}{cls.RESET}" if cls.enabled else text


def enable_ansi():
    C.enabled = sys.stdout.isatty() or os.environ.get("FORCE_COLOR") == "1"
    if not IS_WINDOWS or not C.enabled:
        return
    try:
        import ctypes

        kernel32 = ctypes.windll.kernel32
        handle = kernel32.GetStdHandle(-11)
        mode = ctypes.c_uint32()
        if kernel32.GetConsoleMode(handle, ctypes.byref(mode)):
            kernel32.SetConsoleMode(handle, mode.value | 0x0004)  # ENABLE_VT_PROCESSING
    except Exception:
        C.enabled = False


def say(text="", colour=None):
    sys.stdout.write((C.paint(text, colour) if colour else text) + "\n")
    sys.stdout.flush()


# --------------------------------------------------------------------- session transcript
#
# Renders the NDJSON from `claude --output-format stream-json` the way Claude Code's own transcript
# reads: assistant text, thinking, every tool call with its full input, and the result each call came
# back with. Truncation is per line and per block only, and a shortened block always says how much it
# hid, so nothing is ever silently dropped; --full-output removes the caps. The raw NDJSON is in the
# session log regardless.


class Renderer:
    def __init__(self, opts):
        self.max_result_lines = opts.max_result_lines
        self.max_input_lines = opts.max_input_lines
        self.max_line_chars = opts.max_line_chars
        self.tool_names: dict[str, str] = {}

    def wrapped(self, text, prefix, colour, max_lines):
        if not text:
            return
        text = str(text).replace("\t", "    ").rstrip()
        if not text:
            return
        lines = text.split("\n")
        hidden = 0
        if max_lines > 0 and len(lines) > max_lines:
            hidden = len(lines) - max_lines
            lines = lines[:max_lines]
        for line in lines:
            if self.max_line_chars > 0 and len(line) > self.max_line_chars:
                line = line[: self.max_line_chars] + f"  [+{len(line) - self.max_line_chars} chars]"
            say(prefix + line, colour)
        if hidden:
            say(f"{prefix}... {hidden} more line(s) -- full text in the session log", C.GRAY)

    @staticmethod
    def content_text(content):
        """A message `content` field is either a plain string or a list of blocks."""
        if content is None:
            return ""
        if isinstance(content, str):
            return content
        parts = []
        for block in content if isinstance(content, list) else [content]:
            if isinstance(block, str):
                parts.append(block)
            elif isinstance(block, dict):
                if block.get("text"):
                    parts.append(str(block["text"]))
                elif block.get("type") == "image":
                    parts.append("[image]")
                else:
                    parts.append(json.dumps(block))
        return "\n".join(parts)

    def tool_input(self, obj):
        """Every argument of a tool call, not just the first one that looked interesting."""
        if not isinstance(obj, dict):
            return
        for name, value in obj.items():
            if value is None:
                continue
            text = value if isinstance(value, str) else json.dumps(value, indent=2)
            if not str(text).strip():
                continue
            if "\n" in str(text):
                say(f"       {name}:", C.DIM_CYAN)
                self.wrapped(text, "       | ", C.DIM_CYAN, self.max_input_lines)
            else:
                self.wrapped(f"{name}: {text}", "       ", C.DIM_CYAN, 1)

    def event(self, line):
        line = line.strip()
        if not line:
            return
        try:
            e = json.loads(line)
        except json.JSONDecodeError:
            say(f"   {line}", C.GRAY)
            return
        kind = e.get("type")

        if kind == "system":
            if e.get("subtype") == "init":
                say(
                    f"   [init] model={e.get('model')} cwd={e.get('cwd')} "
                    f"session={e.get('session_id')}",
                    C.GRAY,
                )
                if e.get("tools"):
                    self.wrapped("tools: " + ", ".join(e["tools"]), "   [init] ", C.GRAY, 2)
            else:
                self.wrapped(json.dumps(e), f"   [{e.get('subtype')}] ", C.GRAY, 4)

        elif kind == "assistant":
            for b in e.get("message", {}).get("content", []) or []:
                btype = b.get("type")
                if btype == "text":
                    self.wrapped(b.get("text"), "   ", C.WHITE, 0)
                elif btype == "thinking":
                    self.wrapped(b.get("thinking"), "   . ", C.MAGENTA, self.max_result_lines)
                elif btype == "tool_use":
                    if b.get("id"):
                        self.tool_names[str(b["id"])] = str(b.get("name"))
                    say(f"   > {b.get('name')}", C.CYAN)
                    self.tool_input(b.get("input"))

        elif kind == "user":
            for b in e.get("message", {}).get("content", []) or []:
                if not isinstance(b, dict):
                    continue
                if b.get("type") == "tool_result":
                    name = self.tool_names.get(str(b.get("tool_use_id")), "result")
                    text = self.content_text(b.get("content"))
                    if b.get("is_error"):
                        say(f"     ! {name} failed", C.RED)
                        self.wrapped(text, "     | ", C.RED, self.max_result_lines)
                    else:
                        say(f"     < {name}", C.DIM_GREEN)
                        self.wrapped(text, "     | ", C.GRAY, self.max_result_lines)
                elif b.get("type") == "text":
                    self.wrapped(b.get("text"), "   + ", C.WHITE, self.max_result_lines)

        elif kind == "result":
            bits = [f"{e.get('num_turns')} turns"]
            if e.get("duration_ms") is not None:
                bits.append(f"{float(e['duration_ms']) / 1000:.1f}s")
            usage = e.get("usage") or {}
            if usage:
                bits.append(f"in {usage.get('input_tokens')} / out {usage.get('output_tokens')} tok")
            if e.get("total_cost_usd") is not None:
                bits.append(f"${float(e['total_cost_usd']):.2f}")
            say(f"   [{e.get('subtype')}] " + "  ".join(bits), C.YELLOW)
            if e.get("result"):
                self.wrapped(str(e["result"]), "   ", C.YELLOW, self.max_result_lines)


# ------------------------------------------------------------------------------- capture


class Result:
    __slots__ = ("code", "out", "err")

    def __init__(self, code, out, err):
        self.code = code
        self.out = out
        self.err = err

    @property
    def first_err_line(self):
        return (self.err.strip().splitlines() or [""])[0]


def capture(exe, args, timeout=1800):
    """Run a program with stdout and stderr captured SEPARATELY -- the acceptance list distinguishes
    them (a backtrace and FATAL go to stderr, program output to stdout)."""
    path = shutil.which(exe) or exe
    try:
        p = subprocess.run(
            [path, *args],
            cwd=ROOT,
            capture_output=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return Result(-1, "", f"timed out after {timeout}s")
    except OSError as exc:
        return Result(-1, "", str(exc))
    return Result(p.returncode, p.stdout or "", p.stderr or "")


def stdout_lines(text):
    """CRLF normalized away and the trailing newline dropped, so Windows and Linux compare identically."""
    return text.replace("\r", "").rstrip("\n").split("\n")


def ordered_in(text, wanted):
    """Each element must appear, and each after the first must appear AFTER the one before it."""
    at = 0
    for w in wanted:
        i = text.find(w, at)
        if i < 0:
            return False
        at = i + len(w)
    return True


# ---------------------------------------------------------------------------------- legs


class NativeLeg:
    """The fixtures against the build this platform is.

    `prepare()` builds the CLI once and every fixture then invokes that binary directly.
    `cargo run` per fixture would pay a workspace fingerprint scan twenty-odd times over to
    start the same process -- which is exactly the shape the valgrind sweep below has always
    used, and the only reason the legs did not was that nobody had counted the invocations.
    """

    name = "native"

    def __init__(self):
        self.binary = None

    def prepare(self):
        r = capture("cargo", ["build", "--quiet", "-p", "mwl-cli"])
        if r.code != 0:
            return f"the {self.name} build failed -- {r.first_err_line}"
        self.binary = str(ROOT / "target" / "debug" / ("mwl.exe" if IS_WINDOWS else "mwl"))
        return ""

    def run(self, mwl_args):
        return capture(self.binary, ["run", *mwl_args])


class WslLeg(NativeLeg):
    """Windows only: the same fixtures against a Linux build, through the default WSL distro.

    Building once matters more here than on the native leg: every cargo invocation crosses the
    9p mount at `/mnt/<drive>`, so the fingerprint scan it opens with is the expensive part.
    """

    name = "wsl"

    def __init__(self, target_dir):
        super().__init__()
        drive = str(ROOT)[0].lower()
        self.repo = "/mnt/" + drive + str(ROOT)[2:].replace("\\", "/")
        self.target_dir = target_dir

    def bash(self, inner, timeout=1800):
        return capture("wsl.exe", ["--", "bash", "-lc", inner], timeout=timeout)

    def prepare(self):
        r = self.bash(
            f"cd {self.repo} && CARGO_TARGET_DIR={self.target_dir} cargo build --quiet -p mwl-cli"
        )
        if r.code != 0:
            return f"the {self.name} build failed -- {r.first_err_line}"
        self.binary = f"{self.target_dir}/debug/mwl"
        return ""

    def run(self, mwl_args):
        return self.bash(f"cd {self.repo} && {self.binary} run " + " ".join(mwl_args))


def wsl_available():
    return IS_WINDOWS and shutil.which("wsl.exe") is not None


# ------------------------------------------------------------------------- acceptance


PROGRAM_KINDS = {"exact", "ordered", "contains", "min-bytes"}
SUMMARY_RE = re.compile(r"(\d+)\s+passed,\s+(\d+)\s+failed")

# Checks whose cost is minutes and whose answer is a pure function of the tree: the release-profile
# probe (`lto = "thin"`, `codegen-units = 1`), the second toolchain's whole leg, and a valgrind run
# per fixture. A green verdict on these is remembered against the commit that produced it -- see
# `Goal.remembered`.
EXPENSIVE = {"abi-probe", "wsl leg", "valgrind sweep"}


class Goal:
    """The acceptance test, read from docs/agent/loop-goal.toml. `check()` returns "" when everything
    passes, or the first failure as one line.

    Two things are memoized, and neither of them skips a check:

    * **Within one run**, an identical `args` list runs cargo once. The list holds `mwl-runtime`
      twice on purpose -- stage 0 and stage 5 name different guard tests on it -- and running the
      crate's suite a second time cannot answer differently.
    * **Across runs**, the three checks in `EXPENSIVE` are remembered against the exact tree that
      made them green (HEAD plus a hash of anything dirty). A tree that has not changed cannot
      produce a different verdict, so the sweep is not re-derived; any change at all drops the
      whole cache. This is what makes `--goal-only` cheap to iterate on by hand.
    """

    def __init__(self, spec):
        self.files = spec.get("files", [])
        self.checks = spec.get("check", [])
        self.valgrind_skip = set(spec.get("valgrind", {}).get("skip", []))
        self.wsl_target = spec.get("wsl", {}).get("target_dir", "/tmp/mwl-target-wsl")
        self.program_checks = [c for c in self.checks if c["kind"] in PROGRAM_KINDS]
        self.ran = []  # (label, seconds) for every check this run actually paid for
        self._cargo = {}  # args tuple -> Result, within one check() call
        self._tree = ""
        self._green = set()
        cargo = [c for c in self.checks if c["kind"] not in PROGRAM_KINDS]
        # Stage 0 is catch-up: work a later ADR reopened inside a milestone that
        # was already reported done. It runs before everything else so the
        # ledger names it while it is unfinished -- a Stage 3 fixture failing is
        # not the thing the loop should be told about first. See loop-goal.md.
        catch_up = [str(c.get("stage", "")).startswith("0") for c in cargo]
        self.catch_up_checks = [c for c, first in zip(cargo, catch_up) if first]
        self.cargo_checks = [c for c, first in zip(cargo, catch_up) if not first]

    # -- measuring, and the two memos --------------------------------------------------

    def timed(self, label, thunk):
        """Run `thunk`, recording what it cost. `check()` reports the total and the three
        slowest, because an acceptance test nobody has ever timed is one nobody can tune."""
        started = time.monotonic()
        try:
            return thunk()
        finally:
            self.ran.append((label, time.monotonic() - started))

    def cargo(self, args):
        """`cargo` with the result shared by every check that asks for the same argument list."""
        key = tuple(args)
        if key not in self._cargo:
            self._cargo[key] = capture("cargo", args)
        return self._cargo[key]

    def tree_id(self):
        """HEAD, plus a hash of everything not committed. Two runs with the same id are two runs
        over the same bytes, so a deterministic check cannot answer them differently."""
        head = git("rev-parse", "HEAD") or "no-head"
        dirty = git("status", "--porcelain") + "\n" + git("diff", "HEAD")
        return head + ":" + hashlib.blake2b(dirty.encode("utf-8", "replace"),
                                            digest_size=8).hexdigest()

    def remembered(self, name):
        return name in EXPENSIVE and name in self._green

    def remember(self, name):
        if name in EXPENSIVE:
            self._green.add(name)
            try:
                GOALCACHE.parent.mkdir(exist_ok=True)
                GOALCACHE.write_text(
                    json.dumps({"tree": self._tree, "green": sorted(self._green)}, indent=1),
                    encoding="utf-8", newline="\n",
                )
            except OSError:
                pass

    def load_green(self):
        self._tree = self.tree_id()
        self._green = set()
        try:
            entry = json.loads(GOALCACHE.read_text(encoding="utf-8"))
            if entry.get("tree") == self._tree:
                self._green = set(entry.get("green", []))
        except (OSError, ValueError):
            pass

    # -- one program check on one leg -------------------------------------------------

    def program_check(self, leg, c):
        label = f"{leg.name} {c['file']} [{c.get('stage', '?')}]"
        r = self.timed(label, lambda: leg.run([*c.get("args", []), c["file"]]))
        wants_nonzero = c.get("exit") == "nonzero"
        if wants_nonzero and r.code == 0:
            return f"{label}: exited 0, wanted non-zero"
        if not wants_nonzero and r.code != 0:
            return f"{label}: exit {r.code} -- {r.first_err_line}"

        kind = c["kind"]
        if kind == "exact":
            got = stdout_lines(r.out)
            if got != list(c["want"]):
                return (
                    f"{label}: stdout was [{' / '.join(got)}], wanted [{' / '.join(c['want'])}]"
                )
        elif kind == "ordered":
            text = r.err if c.get("stream") == "stderr" else r.out
            if not ordered_in(text, c["want"]):
                return f"{label}: {c.get('stream', 'stdout')} lacks, in order: {' -> '.join(c['want'])}"
        elif kind == "contains":
            for needle in c.get("stderr_contains", []):
                if needle not in r.err:
                    return f"{label}: stderr lacks {needle!r}"
            for needle in c.get("stdout_contains", []):
                if needle not in r.out:
                    return f"{label}: stdout lacks {needle!r}"
        elif kind == "min-bytes":
            text = r.err if c.get("stream") == "stderr" else r.out
            if len(text) < c["min_bytes"]:
                return f"{label}: only {len(text)} bytes of output, wanted {c['min_bytes']}"
        return ""

    # -- the cargo-side checks, run once ----------------------------------------------

    def cargo_check(self, c, leg=None):
        label = f"{c['name']} [{c.get('stage', '?')}]"

        if c["kind"] == "mwl-suite":
            # The suite runner is the CLI the leg already built; `cargo run` here would be one
            # more workspace fingerprint scan to start a binary sitting on disk.
            r = self.timed(label, lambda: capture(leg.binary, c["args"]))
        else:
            r = self.timed(label, lambda: self.cargo(c["args"]))
        if r.code != 0:
            return f"{label}: exit {r.code} -- {r.first_err_line}"
        both = r.out + "\n" + r.err

        if c["kind"] == "mwl-suite":
            m = SUMMARY_RE.search(both)
            if not m:
                return f"{label}: no 'N passed, M failed' summary line in the output"
            if int(m.group(2)) != 0:
                return f"{label}: {m.group(2)} case(s) failed"
            if int(m.group(1)) < c["min_passing"]:
                return (
                    f"{label}: only {m.group(1)} passing case(s), wanted at least {c['min_passing']}"
                )
        elif c["kind"] == "cargo-named":
            for name in c["tests"]:
                if name not in both:
                    return f"{label}: test {name!r} did not run"
        else:
            return f"{label}: unknown check kind {c['kind']!r}"
        return ""

    # -- the valgrind sweep -------------------------------------------------------------

    def valgrind(self, leg):
        """Every fixture under `valgrind --leak-check=full`, over the binary the leg already
        built. In WSL on Windows, directly on Linux."""
        if self.remembered("valgrind sweep"):
            return ""
        if leg.name == "native" and shutil.which("valgrind") is None:
            return ""  # not a failure: this platform simply has no valgrind leg

        for f in self.files:
            if f in self.valgrind_skip:
                continue
            cmd = (
                "valgrind --error-exitcode=1 --leak-check=full "
                f"--errors-for-leak-kinds=definite -q {leg.binary} run {f}"
            )
            r = self.timed(
                f"valgrind {f}",
                lambda: (leg.bash(f"cd {leg.repo} && {cmd}") if leg.name == "wsl"
                         else capture("bash", ["-lc", cmd])),
            )
            if r.code != 0:
                return f"valgrind {f}: exit {r.code} -- {r.first_err_line}"
        self.remember("valgrind sweep")
        return ""

    # -- the whole thing ----------------------------------------------------------------

    def check(self, verbose=False):
        def trace(msg):
            if verbose:
                say(f"   .. {msg}", C.GRAY)

        self.ran = []
        self._cargo = {}
        self.load_green()

        for f in self.files:
            if not (ROOT / f).exists():
                return f"{f} is missing -- the acceptance fixtures are fixed, see docs/agent/loop-goal.md"

        # One build for every fixture that follows, and a broken tree is reported as a broken
        # build rather than as twenty-three fixtures with nothing on stdout.
        native = NativeLeg()
        trace("building the native CLI")
        fail = self.timed("native build", native.prepare)
        if fail:
            return fail

        for c in self.catch_up_checks:
            trace(f"cargo {c['name']} (catch-up)")
            fail = self.cargo_check(c, native)
            if fail:
                return fail

        for c in self.program_checks:
            trace(f"{native.name} {c['file']}")
            fail = self.program_check(native, c)
            if fail:
                return fail

        for c in self.cargo_checks:
            if self.remembered(c["name"]):
                trace(f"cargo {c['name']} (green on this tree already)")
                continue
            trace(f"cargo {c['name']}")
            fail = self.cargo_check(c, native)
            if fail:
                return fail
            self.remember(c["name"])

        # Windows is green, so now pay for the Linux leg.
        leg = native
        if wsl_available():
            leg = WslLeg(self.wsl_target)
            trace("building the wsl CLI")
            fail = self.timed("wsl build", leg.prepare)
            if fail:
                return fail
            if self.remembered("wsl leg"):
                trace("wsl fixtures (green on this tree already)")
            else:
                for c in self.program_checks:
                    trace(f"{leg.name} {c['file']}")
                    fail = self.program_check(leg, c)
                    if fail:
                        return fail
                self.remember("wsl leg")

        trace("valgrind sweep")
        return self.valgrind(leg)

    def summary(self):
        """One line: what this run cost, and the three checks that cost the most of it."""
        if not self.ran:
            return "nothing ran"
        total = sum(s for _, s in self.ran)
        worst = sorted(self.ran, key=lambda x: -x[1])[:3]
        slow = ", ".join(f"{label} {s:.0f}s" for label, s in worst if s >= 1)
        return f"{total:.0f}s over {len(self.ran)} check(s)" + (f"; slowest: {slow}" if slow else "")


def load_goal():
    """The acceptance list, read from disk. Call this per session rather than once per run: a
    session may rewrite `loop-goal.toml` -- strike an item, correct the test name a check demands --
    and the run it belongs to has to be checked against what it wrote, not against what the driver
    read hours earlier.

    Measured. Session 0013 of the 2026-08-26 run renamed the two tests the string-capacity check
    names, to the names the tree had actually landed them under. The driver held the pre-rename
    spec for the rest of the run, so `check()` short-circuited at check 16 of 47 on a test that
    exists nowhere -- and because a short-circuit skips everything after it, Stage 4's two counts,
    Stage 5's guards, the WSL leg and the valgrind sweep did not run for the following seventeen
    sessions. The loop could not have stopped even had the goal been reached.
    """
    return Goal(tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")))


# -------------------------------------------------------------------------------- driver


def git(*args):
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, encoding="utf-8", check=True
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return ""


def ledger(line):
    with LEDGER.open("a", encoding="utf-8", newline="\n") as fh:
        fh.write(line + "\n")
    say(line)


def orientation_pack():
    """`orient.py`'s output, to hand the session inline instead of leaving it to fetch.

    A session used to run the script itself as its first tool call, and that cost far more
    than the pack: the harness spills a result that large to a file, so the session spent a
    `cat` and a `Read` getting it back -- three calls, and a measured ~20,500 tokens for a
    pack that is 12,917 of text, the difference being the spill notice, the truncation retry
    and the readback's line numbers. Piped in on stdin it is charged once, at its own size.

    Returns "" if the script fails, and the prompt's own fallback then applies: the session
    runs it the old way rather than starting blind.
    """
    try:
        done = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "orient.py")],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=120,
        )
    except (OSError, subprocess.SubprocessError):
        return ""
    return done.stdout if done.returncode == 0 else ""


def feed(stream, text):
    """Write `text` to a child's stdin and close it, ignoring a child that exited first.

    From its own thread on purpose: the child streams NDJSON back while this goes in, and a
    46 KB write into a 64 KB pipe deadlocks against a child that is itself blocked writing
    stdout nobody is draining.
    """
    try:
        stream.write(text)
        stream.close()
    except (OSError, ValueError):
        pass


def run_session(run_id, index, prompt_text, opts, renderer):
    """One `claude -p` session, its NDJSON streamed to the console and to
    .loop/logs/<run>-NNNN.log. The run stamp is in the name because the index restarts at 1
    every run: named by index alone, session 3 of today's run appended to session 3 of last
    week's, and any per-session measurement over the directory silently mixed the two.

    The session id off the `system`/`init` event is kept, not just printed: it is the only
    handle on the harness's own transcript directory, and therefore on any subagent this
    session spawned. Without it a delegated read is invisible to every measurement below."""
    log = LOGDIR / f"{run_id}-{index:04d}.log"
    exe = shutil.which("claude") or "claude"
    cmd = [
        exe,
        "-p",
        prompt_text,
        "--model",
        opts.model,
        "--permission-mode",
        opts.permission_mode,
        "--output-format",
        "stream-json",
        "--verbose",
    ]
    pack = orientation_pack()
    session_id = ""
    with log.open("a", encoding="utf-8", newline="\n") as fh:
        # The pack's size, recorded beside the transcript that paid for it. Two sessions with
        # different pack sizes are a two-point regression against their measured `ctx_start`,
        # which is how `loop-stats.py --calibrate` derives bytes-per-token instead of assuming
        # it. Nothing downstream needs this line; every reader skips a `type` it does not know.
        fh.write(json.dumps({"type": "loop_pack", "bytes": len(pack.encode("utf-8"))}) + "\n")
        fh.flush()
        proc = subprocess.Popen(
            cmd,
            cwd=ROOT,
            stdin=subprocess.PIPE if pack else None,
            stdout=subprocess.PIPE,
            stderr=None,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
        )
        if pack:
            threading.Thread(target=feed, args=(proc.stdin, pack), daemon=True).start()
        assert proc.stdout is not None
        try:
            for line in proc.stdout:
                fh.write(line)
                fh.flush()
                if not session_id and '"session_id"' in line:
                    try:
                        e = json.loads(line)
                        if e.get("type") == "system" and e.get("subtype") == "init":
                            session_id = str(e.get("session_id") or "")
                    except json.JSONDecodeError:
                        pass
                renderer.event(line)
            proc.wait()
        except BaseException:
            # The child does not outlive its supervisor. It is an autonomous agent writing
            # this tree with permissions bypassed, and when the driver died on an encoding
            # error its child kept going unwatched -- committing work the next session then
            # found beside its own, which is what a `BLOCKED two writers` ledger line is
            # made of. Ctrl-C reaches the child on its own; every other exit did not.
            proc.kill()
            try:
                proc.wait(timeout=30)
            except subprocess.TimeoutExpired:
                pass
            raise
    return proc.returncode, log, session_id


# ------------------------------------------------------------------- subagent transcripts
#
# A subagent's turns do NOT appear in the parent's `stream-json`; only the tool call and the
# report it returned do. The harness does write each one's full transcript, next to the parent
# session's, so the whole cost of a delegated read is on disk -- it is simply somewhere nothing
# in this repository was looking. Copying it under .loop/logs/ closes that blind spot, so
# loop-stats.py can charge a subagent's calls, seconds and context to the session that spawned
# it rather than reporting a session that mysteriously did a lot with very few calls.
#
# The session id is a UUID, so globbing every project directory for it is exact and needs no
# knowledge of how the harness mangles a working-directory path into a directory name.


def claude_home():
    override = os.environ.get("CLAUDE_CONFIG_DIR")
    return Path(override) if override else Path.home() / ".claude"


def collect_subagents(session_id, run_id, index):
    """Copy this session's subagent transcripts beside its own log. Returns (files, calls)."""
    if not session_id:
        return 0, 0
    found = sorted(claude_home().glob(f"projects/*/{session_id}/subagents/*.jsonl"))
    if not found:
        return 0, 0
    dest = LOGDIR / f"{run_id}-{index:04d}.subagents"
    dest.mkdir(parents=True, exist_ok=True)
    calls = 0
    for src in found:
        try:
            text = src.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        (dest / src.name).write_text(text, encoding="utf-8", newline="\n")
        calls += text.count('"type":"tool_use"')
    return len(found), calls


# ------------------------------------------------------------------------- run marker
#
# `.loop/running` exists for exactly as long as a driver is driving this tree. It is what anything else
# -- a person, an interactive session, a by-hand refactoring pass -- checks before
# touching files the loop's sessions edit on nearly every iteration. A file-existence test, on purpose:
# asking the OS whether a pid is alive is a different answer on every platform, and getting it subtly
# wrong here would be worse than a stale marker a human deletes.


def make_room(opts):
    """Prune what previous runs left behind, then refuse to start with too little disk.

    Once per *run*, never once per session: both prunes are a single directory listing of a few
    dozen entries, so a run pays milliseconds and a session pays nothing at all.

    The refusal is the point. A run that fills the disk does not stop cleanly -- it dies inside
    a session with the tree half-edited and the next session inheriting the mess, which is
    exactly what happened on 2026-08-25. Failing at the door instead costs one line.
    `tools/disk.py` owns the policy, the numbers and the explanation."""
    freed = disk.prune_logs() + disk.prune_scratch()
    if freed:
        say(f"pruned {disk.human(freed)} of earlier runs' logs and scratch", C.GRAY)
    free = disk.free_gb(ROOT)
    if opts.min_free_gb and free < opts.min_free_gb:
        say(f"{free:.1f}G free on {ROOT.drive or '/'}; a run needs {opts.min_free_gb:g}G.", C.RED)
        say(
            "`python tools/disk.py` says what is holding it. `--clean` drops the superseded "
            "build generations, which is nearly always all of it.\n"
            "To start anyway: --min-free-gb 0.",
            C.YELLOW,
        )
        return False
    return True


def claim_run(opts):
    """Write the marker, or explain who already holds it. Returns True when the run may start."""
    if RUNNING.exists() and not opts.force:
        say(f"a loop is already running on this tree, per {rel_to_root(RUNNING)}:", C.RED)
        for line in RUNNING.read_text(encoding="utf-8").rstrip("\n").split("\n"):
            say(f"  {line}", C.RED)
        say(
            "\nTwo drivers on one working tree race on every file. If that run is actually over "
            f"(Ctrl-C, a crash, a reboot), delete {rel_to_root(RUNNING)} and start again, "
            "or pass --force.",
            C.YELLOW,
        )
        return False
    RUNNING.write_text(
        f"pid:      {os.getpid()}\n"
        f"host:     {platform.node()}\n"
        f"started:  {datetime.now():%Y-%m-%d %H:%M:%S}\n"
        f"sessions: up to {opts.max_sessions}, model {opts.model}\n",
        encoding="utf-8",
        newline="\n",
    )
    return True


def release_run():
    RUNNING.unlink(missing_ok=True)


def rel_to_root(path):
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def main():
    # A session's own output carries `§`, `↔` and em dashes, and this echoes it. On Windows a
    # redirected stdout defaults to cp1252, where the first such character raises
    # UnicodeEncodeError from inside the renderer -- which killed the driver mid-session, after
    # the child had already done work, with an empty log to show for it. A console is fine; a
    # `> file`, a `nohup` or CI is not, which is exactly where nobody is watching.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass

    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--max-sessions", type=int, default=1)
    ap.add_argument("--model", default="opus")
    ap.add_argument("--permission-mode", default="bypassPermissions")
    ap.add_argument("--max-stalls", type=int, default=10, help="consecutive no-commit sessions")
    ap.add_argument("--max-retries", type=int, default=3, help="consecutive CLI failures")
    ap.add_argument("--delay-seconds", type=int, default=0)
    ap.add_argument("--max-result-lines", type=int, default=60)
    ap.add_argument("--max-input-lines", type=int, default=40)
    ap.add_argument("--max-line-chars", type=int, default=500)
    ap.add_argument("--full-output", action="store_true", help="no truncation anywhere")
    ap.add_argument("--goal-only", action="store_true", help="run the acceptance test and exit")
    ap.add_argument("--list", action="store_true", help="print the acceptance plan and exit")
    ap.add_argument(
        "--force", action="store_true", help="start even if .loop/running says a driver is up"
    )
    ap.add_argument(
        "--min-free-gb", type=float, default=disk.MIN_FREE_GB,
        help="refuse to start below this much free disk; 0 disables the check"
    )
    opts = ap.parse_args()

    if opts.full_output:
        opts.max_result_lines = opts.max_input_lines = opts.max_line_chars = 0

    enable_ansi()

    for f in (PROMPT, GOAL_MD, GOAL_TOML):
        if not f.exists():
            say(f"missing {f}", C.RED)
            return 2

    goal = load_goal()

    if opts.list:
        legs = ["native"] + (["wsl"] if wsl_available() else [])
        say(f"{GOAL_TOML.relative_to(ROOT).as_posix()}: {len(goal.checks)} checks, "
            f"{len(goal.files)} fixtures, legs: {', '.join(legs)}", C.CYAN)
        for c in goal.catch_up_checks + goal.program_checks + goal.cargo_checks:
            if "file" in c:
                extra = " ".join(c.get("args", []))
                say(f"  [{c.get('stage', '?')}] {c['kind']:<10} {c['file']} {extra}".rstrip())
                continue
            driver = "mwl" if c["kind"] == "mwl-suite" else "cargo"
            say(f"  [{c.get('stage', '?')}] {c['kind']:<11} {c['name']}: "
                f"{driver} {' '.join(c['args'])}")
        skipped = ", ".join(sorted(goal.valgrind_skip)) or "nothing"
        say(f"  valgrind sweep over every fixture except: {skipped}")
        return 0

    if opts.goal_only:
        say("running the acceptance test ...", C.CYAN)
        fail = goal.check(verbose=True)
        say(f"\ncost: {goal.summary()}", C.GRAY)
        if fail:
            say(f"NOT GREEN: {fail}", C.RED)
            return 1
        say("GOAL REACHED: every acceptance check passes", C.GREEN)
        return 0

    LOGDIR.mkdir(parents=True, exist_ok=True)
    if not LEDGER.exists():
        LEDGER.write_text("# Loop ledger\n", encoding="utf-8", newline="\n")

    if not make_room(opts):
        return 2
    if not claim_run(opts):
        return 2
    try:
        drive(opts, goal)
    except KeyboardInterrupt:
        say("")
        say(
            "interrupted -- the working tree is still consistent, because every session commits "
            "before it exits",
            C.YELLOW,
        )
        ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- interrupted (Ctrl-C)")
    finally:
        release_run()
    return 0


def drive(opts, goal):
    """The session loop itself. Split out so `main` can hold the `.loop/running` marker across it,
    and drop it on any exit -- a normal stop, a Ctrl-C, or an exception."""
    prompt_text = PROMPT.read_text(encoding="utf-8")
    renderer = Renderer(opts)

    stalls = 0
    fails = 0
    reason = f"hit --max-sessions ({opts.max_sessions})"
    run_id = f"{datetime.now():%Y%m%d-%H%M%S}"
    ledger("")
    ledger(f"## run started {datetime.now():%Y-%m-%d %H:%M} (max {opts.max_sessions}, logs {run_id}-*)")

    for i in range(1, opts.max_sessions + 1):
        if STOP.exists():
            reason = f"{STOP.relative_to(ROOT).as_posix()} present"
            break

        head_before = git("rev-parse", "HEAD")
        STATUS.unlink(missing_ok=True)
        say(f"== session {i}/{opts.max_sessions}  {datetime.now():%H:%M:%S}", C.CYAN)

        # The prompt is re-read for the same reason `load_goal()` is called below: a session that
        # improved it should be improving the next session, not the next run. A read that fails
        # keeps the last good text rather than ending a run nobody is watching.
        try:
            prompt_text = PROMPT.read_text(encoding="utf-8")
        except OSError as e:
            say(f"   {rel_to_root(PROMPT)} did not read, using the last good one -- {e}", C.YELLOW)

        cli_exit, log, session_id = run_session(run_id, i, prompt_text, opts, renderer)
        if cli_exit != 0:
            fails += 1
            ledger(
                f"- {i:04d} CLI exit {cli_exit} (attempt {fails}/{opts.max_retries}) -- "
                f"see {log.relative_to(ROOT).as_posix()}"
            )
            if fails >= opts.max_retries:
                reason = f"claude CLI failed {fails} times in a row"
                break
            time.sleep(min(300, 30 * 2**fails))
            continue
        fails = 0

        line = STATUS.read_text(encoding="utf-8").strip() if STATUS.exists() else ""
        head_after = git("rev-parse", "HEAD")
        commits = 0
        if head_after and head_after != head_before:
            commits = int(git("rev-list", "--count", f"{head_before}..{head_after}") or 0)
        agents, agent_calls = collect_subagents(session_id, run_id, i)
        delegated = f" | {agents} subagent(s), {agent_calls} call(s)" if agents else ""
        ledger(f"- {i:04d} {commits} commit(s){delegated} | {line or '(no status written)'}")

        # The deterministic goal check outranks whatever the session reported -- against the list
        # as the session left it, which is why this is re-read rather than held from start-up. A
        # `loop-goal.toml` that does not parse keeps the last good spec: a run of 300 sessions must
        # not end on one session's typo, and the ledger names it loudly instead.
        try:
            goal = load_goal()
        except (OSError, tomllib.TOMLDecodeError) as e:
            ledger(f"       goal spec: {rel_to_root(GOAL_TOML)} did not parse -- "
                   f"checking against the last good one. {e}")
        fail = goal.check()
        ledger(f"       goal cost: {goal.summary()}")
        if not fail:
            reason = "GOAL REACHED: every acceptance check in docs/agent/loop-goal.toml passes"
            break
        ledger(f"       goal check: {fail}")

        if line.startswith("DONE"):
            reason = f"session reported DONE but the acceptance test does not pass yet: {line}"
            break
        if line.startswith("BLOCKED"):
            reason = f"blocked on a user decision: {line}"
            break

        if commits == 0:
            stalls += 1
            if stalls >= opts.max_stalls:
                reason = f"{stalls} sessions in a row produced no commit"
                break
        else:
            stalls = 0

        if opts.delay_seconds:
            time.sleep(opts.delay_seconds)

    ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- {reason}")
    say("")
    say(reason, C.YELLOW)


if __name__ == "__main__":
    sys.exit(main())
