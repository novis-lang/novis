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
import json
import os
import re
import shutil
import subprocess
import sys
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

ROOT = Path(__file__).resolve().parent.parent
PROMPT = ROOT / "docs" / "agent" / "session-prompt.md"
GOAL_MD = ROOT / "docs" / "agent" / "loop-goal.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"
RUNDIR = ROOT / ".loop"
LOGDIR = RUNDIR / "logs"
LEDGER = RUNDIR / "log.md"
STATUS = RUNDIR / "status.txt"
STOP = RUNDIR / "stop"

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
    name = "native"

    def run(self, mwl_args):
        return capture("cargo", ["run", "--quiet", "-p", "mwl-cli", "--", "run", *mwl_args])


class WslLeg:
    """Windows only: the same fixtures against a Linux build, through the default WSL distro."""

    name = "wsl"

    def __init__(self, target_dir):
        drive = str(ROOT)[0].lower()
        self.repo = "/mnt/" + drive + str(ROOT)[2:].replace("\\", "/")
        self.target_dir = target_dir

    def bash(self, inner, timeout=1800):
        return capture("wsl.exe", ["--", "bash", "-lc", inner], timeout=timeout)

    def run(self, mwl_args):
        return self.bash(
            f"cd {self.repo} && CARGO_TARGET_DIR={self.target_dir} "
            f"cargo run --quiet -p mwl-cli -- run " + " ".join(mwl_args)
        )


def wsl_available():
    return IS_WINDOWS and shutil.which("wsl.exe") is not None


# ------------------------------------------------------------------------- acceptance


PROGRAM_KINDS = {"exact", "ordered", "contains", "min-bytes"}
SUMMARY_RE = re.compile(r"(\d+)\s+passed,\s+(\d+)\s+failed")


class Goal:
    """The acceptance test, read from docs/agent/loop-goal.toml. `check()` returns "" when everything
    passes, or the first failure as one line."""

    def __init__(self, spec):
        self.files = spec.get("files", [])
        self.checks = spec.get("check", [])
        self.valgrind_skip = set(spec.get("valgrind", {}).get("skip", []))
        self.wsl_target = spec.get("wsl", {}).get("target_dir", "/tmp/mwl-target-wsl")
        self.program_checks = [c for c in self.checks if c["kind"] in PROGRAM_KINDS]
        self.cargo_checks = [c for c in self.checks if c["kind"] not in PROGRAM_KINDS]

    # -- one program check on one leg -------------------------------------------------

    def program_check(self, leg, c):
        label = f"{leg.name} {c['file']} [{c.get('stage', '?')}]"
        r = leg.run([*c.get("args", []), c["file"]])
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

    def cargo_check(self, c):
        label = f"{c['name']} [{c.get('stage', '?')}]"
        r = capture("cargo", c["args"])
        if r.code != 0:
            return f"{label}: exit {r.code} -- {r.first_err_line}"
        both = r.out + "\n" + r.err

        if c["kind"] == "cargo-suite":
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

    def valgrind(self, wsl):
        """Every fixture under `valgrind --leak-check=full`. In WSL on Windows, directly on Linux."""
        if wsl is not None:
            build = wsl.bash(
                f"cd {wsl.repo} && CARGO_TARGET_DIR={wsl.target_dir} cargo build --quiet -p mwl-cli"
            )
            if build.code != 0:
                return f"valgrind: the linux build failed -- {build.first_err_line}"
            binary = f"{wsl.target_dir}/debug/mwl"
        else:
            if shutil.which("valgrind") is None:
                return ""  # not a failure: this platform simply has no valgrind leg
            build = capture("cargo", ["build", "--quiet", "-p", "mwl-cli"])
            if build.code != 0:
                return f"valgrind: the build failed -- {build.first_err_line}"
            binary = str(ROOT / "target" / "debug" / "mwl")

        for f in self.files:
            if f in self.valgrind_skip:
                continue
            cmd = (
                "valgrind --error-exitcode=1 --leak-check=full "
                f"--errors-for-leak-kinds=definite -q {binary} run {f}"
            )
            r = wsl.bash(f"cd {wsl.repo} && {cmd}") if wsl else capture("bash", ["-lc", cmd])
            if r.code != 0:
                return f"valgrind {f}: exit {r.code} -- {r.first_err_line}"
        return ""

    # -- the whole thing ----------------------------------------------------------------

    def check(self, verbose=False):
        def trace(msg):
            if verbose:
                say(f"   .. {msg}", C.GRAY)

        for f in self.files:
            if not (ROOT / f).exists():
                return f"{f} is missing -- the acceptance fixtures are fixed, see docs/agent/loop-goal.md"

        native = NativeLeg()
        for c in self.program_checks:
            trace(f"{native.name} {c['file']}")
            fail = self.program_check(native, c)
            if fail:
                return fail

        for c in self.cargo_checks:
            trace(f"cargo {c['name']}")
            fail = self.cargo_check(c)
            if fail:
                return fail

        # Windows is green, so now pay for the Linux leg.
        wsl = WslLeg(self.wsl_target) if wsl_available() else None
        if wsl is not None:
            for c in self.program_checks:
                trace(f"{wsl.name} {c['file']}")
                fail = self.program_check(wsl, c)
                if fail:
                    return fail

        trace("valgrind sweep")
        return self.valgrind(wsl)


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


def run_session(index, prompt_text, opts, renderer):
    """One `claude -p` session, its NDJSON streamed to the console and to .loop/logs/NNNN.log."""
    log = LOGDIR / f"{index:04d}.log"
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
    with log.open("a", encoding="utf-8", newline="\n") as fh:
        proc = subprocess.Popen(
            cmd,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=None,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            fh.write(line)
            fh.flush()
            renderer.event(line)
        proc.wait()
    return proc.returncode, log


def main():
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
    opts = ap.parse_args()

    if opts.full_output:
        opts.max_result_lines = opts.max_input_lines = opts.max_line_chars = 0

    enable_ansi()

    for f in (PROMPT, GOAL_MD, GOAL_TOML):
        if not f.exists():
            say(f"missing {f}", C.RED)
            return 2

    goal = Goal(tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")))

    if opts.list:
        legs = ["native"] + (["wsl"] if wsl_available() else [])
        say(f"{GOAL_TOML.relative_to(ROOT).as_posix()}: {len(goal.checks)} checks, "
            f"{len(goal.files)} fixtures, legs: {', '.join(legs)}", C.CYAN)
        for c in goal.program_checks:
            extra = " ".join(c.get("args", []))
            say(f"  [{c.get('stage', '?')}] {c['kind']:<10} {c['file']} {extra}".rstrip())
        for c in goal.cargo_checks:
            say(f"  [{c.get('stage', '?')}] {c['kind']:<10} {c['name']}: cargo {' '.join(c['args'])}")
        skipped = ", ".join(sorted(goal.valgrind_skip)) or "nothing"
        say(f"  valgrind sweep over every fixture except: {skipped}")
        return 0

    if opts.goal_only:
        say("running the acceptance test ...", C.CYAN)
        fail = goal.check(verbose=True)
        if fail:
            say(f"\nNOT GREEN: {fail}", C.RED)
            return 1
        say("\nGOAL REACHED: every acceptance check passes", C.GREEN)
        return 0

    LOGDIR.mkdir(parents=True, exist_ok=True)
    if not LEDGER.exists():
        LEDGER.write_text("# Loop ledger\n", encoding="utf-8", newline="\n")

    prompt_text = PROMPT.read_text(encoding="utf-8")
    renderer = Renderer(opts)

    stalls = 0
    fails = 0
    reason = f"hit --max-sessions ({opts.max_sessions})"
    ledger("")
    ledger(f"## run started {datetime.now():%Y-%m-%d %H:%M} (max {opts.max_sessions})")

    for i in range(1, opts.max_sessions + 1):
        if STOP.exists():
            reason = f"{STOP.relative_to(ROOT).as_posix()} present"
            break

        head_before = git("rev-parse", "HEAD")
        STATUS.unlink(missing_ok=True)
        say(f"== session {i}/{opts.max_sessions}  {datetime.now():%H:%M:%S}", C.CYAN)

        cli_exit, log = run_session(i, prompt_text, opts, renderer)
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
        ledger(f"- {i:04d} {commits} commit(s) | {line or '(no status written)'}")

        # The deterministic goal check outranks whatever the session reported.
        fail = goal.check()
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
