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
    python tools/loop.py --leg-only                         # just the Linux leg, and exit

Nothing on the stop path depends on a model's self-assessment: every acceptance item is an exit code plus
an exact or ordered-substring match on real output.
"""

from __future__ import annotations

import _thread
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
from concurrent.futures import ThreadPoolExecutor
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
import machine  # noqa: E402  -- same directory; how wide anything runs has one home too

ROOT = Path(__file__).resolve().parent.parent
PROMPT = ROOT / "docs" / "agent" / "session-prompt.md"
GOAL_MD = ROOT / "docs" / "agent" / "loop-goal.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"
RUNDIR = ROOT / ".loop"
LOGDIR = RUNDIR / "logs"
LEDGER = RUNDIR / "log.md"
STATUS = RUNDIR / "status.txt"
STOP = RUNDIR / "stop"
RETRY = RUNDIR / "retry"
RUNNING = RUNDIR / "running"
GOALCACHE = RUNDIR / "goal-green.json"
LIMIT = RUNDIR / "limit.json"
INTERRUPTED = RUNDIR / "interrupted.json"

IS_WINDOWS = os.name == "nt"

# The console's key reader. The two platforms have nothing in common here and neither module
# exists on the other one; `Control` below is the only thing that touches either.
if IS_WINDOWS:
    import msvcrt

    select = termios = tty = None
else:
    import select
    import termios
    import tty

    msvcrt = None


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


class StatusLine:
    """The one line that stays on the bottom row and says what is happening right now.

    Everything else scrolls past it: `say()` erases it before it writes and redraws it after, and a
    ticker thread repaints it a few times a second so the spinner moves even while the driver is
    blocked on a build or on a child's stdout. A run that prints nothing for four minutes -- the
    orientation pack, a cold cargo build, the WSL leg -- is indistinguishable from a hung one
    without it, and the phases that go quiet are exactly the slow ones.

    Four fields, in the order they narrow:

        scope    where the run is       `session 3/12`
        phase    what it is doing now   `acceptance check`, with `31/58 53%` when that is countable
        detail   the current item       `wsl fixtures/closures.mwl`, `Edit tools/loop.py`
        elapsed  how long this phase has been going

    A count is shown only when the total is known ahead of time -- the acceptance sweep knows how
    many checks it is about to run, a session does not know how many tool calls it will make, so the
    session shows a running count and no percentage rather than a number that pretends to be one.

    Disabled whenever stdout is not a terminal. A redirected run, `nohup` or CI would otherwise
    collect thousands of repaints of a line nobody is watching.
    """

    BRAILLE = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
    ASCII = "|/-\\"
    INTERVAL = 0.12

    def __init__(self):
        self.enabled = False
        self.lock = threading.RLock()
        self.frames = self.ASCII
        self.sep = " | "
        self.cut = "..."
        self.bar = "-"
        self.dash = " -- "
        self.scope = ""
        self.phase = ""
        self.detail = ""
        self.done = 0
        self.total = 0
        self.calls = 0
        self.since = time.monotonic()
        self._frame = 0
        self._rows = 0  # rows the live block owns right now; 0 when it is not on screen
        self._stop = threading.Event()
        self._thread = None

    # -- lifecycle ---------------------------------------------------------------------

    def start(self):
        """Begin painting, if this is a console that can be painted on."""
        if self.enabled or not (C.enabled and sys.stdout.isatty()):
            return
        # A console that cannot encode a braille frame gets an ASCII one rather than a row of
        # replacement characters: `main()` reconfigures stdout to UTF-8, but a legacy code page
        # is still what a raw `cmd.exe` hands back.
        if "utf" in (getattr(sys.stdout, "encoding", "") or "").lower():
            self.frames, self.sep, self.cut, self.bar = self.BRAILLE, " · ", "…", "─"
            self.dash = " — "
        self.enabled = True
        self.since = time.monotonic()
        self._thread = threading.Thread(target=self._tick, daemon=True)
        self._thread.start()

    def stop(self):
        """Erase the line and leave the cursor where the next print expects it."""
        self._stop.set()
        with self.lock:
            self.erase()
            self.enabled = False

    def _tick(self):
        while not self._stop.wait(self.INTERVAL):
            # Outside the lock, and it has to stay outside: `poll` says things, and saying
            # anything takes this lock. Every path in this file therefore takes CONTROL's lock
            # before the ticker's, which is the whole of the argument that neither waits on the
            # other.
            CONTROL.poll()
            with self.lock:
                self._frame += 1
                self.draw()

    # -- what it says ------------------------------------------------------------------

    def set(self, phase=None, detail=None, scope=None, total=None, done=None, calls=None):
        """Update any subset of the fields. A new `phase` restarts the elapsed clock and clears
        the detail and the counters, because those belonged to the phase that just ended."""
        with self.lock:
            if scope is not None:
                self.scope = scope
            if phase is not None and phase != self.phase:
                self.phase = phase
                self.detail = ""
                self.done = self.total = self.calls = 0
                self.since = time.monotonic()
            if detail is not None:
                self.detail = detail
            if total is not None:
                self.total = total
            if done is not None:
                self.done = done
            if calls is not None:
                self.calls = calls
            self.draw()

    def advance(self, detail=None):
        """One planned step of the current phase is finished."""
        with self.lock:
            self.done += 1
            if detail is not None:
                self.detail = detail
            self.draw()

    def tool(self, name):
        """A session made a tool call. Counted, never totalled -- see the class doc."""
        with self.lock:
            self.calls += 1
            self.note(name)

    def note(self, text):
        """What the session is doing between tool calls, behind the running call count."""
        with self.lock:
            self.detail = (f"{self.calls} tool call(s){self.sep}" if self.calls else "") + text
            self.draw()

    # -- painting ----------------------------------------------------------------------

    def compose(self):
        head = self.phase
        if self.total > 0:
            done = min(self.done, self.total)
            head = f"{head} {done}/{self.total} {done * 100 // self.total}%".lstrip()
        parts = [p for p in (self.scope, head, self.detail) if p]
        parts.append(mmss(time.monotonic() - self.since))
        body = self.sep.join(parts).replace("\n", " ")
        # One column short of the width on purpose: a line that exactly fills the terminal wraps,
        # and a wrapped status line is one the next erase only half removes.
        room = max(18, self.width() - 3)  # the spinner and its space
        if len(body) > room:
            body = body[: room - len(self.cut)] + self.cut
        spin = self.frames[self._frame % len(self.frames)]
        return C.paint(spin, C.CYAN) + " " + C.paint(body, C.WHITE)

    def width(self):
        return shutil.get_terminal_size((100, 24)).columns

    def divider(self):
        """The rule above the status line.

        The status line is repainted in place at the bottom of the scrollback, so it sits flush
        against whatever the driver or the session printed last -- and both are indented, wrapped
        prose. Watching a run, the live line and the dead one above it read as one paragraph, and
        the eye has to parse the text to find out which is which. A rule makes the boundary a
        shape rather than a colour, so the rule stays grey and only the live line goes white --
        what should catch the eye is the status, not the furniture around it."""
        return C.paint(self.bar * max(18, self.width() - 1), C.GRAY)

    def keys(self):
        """The row under the status line: what a keypress would do *right now*.

        Contextual on purpose, and that is the whole design. `[r]` appears only while a usage wall
        is up, because a wall is the only thing it ends -- offered at any other time it is a key
        that silently does nothing. `[s]` is always there, and once armed the row says so and says
        how to take it back: a keypress that cannot be undone is worse than no keypress at all.

        Painted as one colour rather than per-word, so what a narrow terminal truncates is text
        and never half an escape sequence."""
        bits = []
        if CONTROL.parked:
            bits.append("[r] retry now")
        if CONTROL.stop:
            grace = CONTROL.stop_in()
            bits.append(f"[s] stopping in {grace:.0f}s{self.dash}press s to cancel" if grace > 0
                        else f"[s] stopping after this session{self.dash}press s to cancel")
        else:
            bits.append("[s] stop after this session")
        body = self.sep.join(bits)
        room = max(18, self.width() - 3)
        if len(body) > room:
            body = body[: room - len(self.cut)] + self.cut
        return "  " + C.paint(body, C.YELLOW if CONTROL.stop else C.GRAY)

    def rows(self):
        """How tall the live block is: the rule and the status line always, the key row only when
        there is a console to type at. Both `draw` and `erase` read this, and a height that
        changes mid-run is handled by giving the old block back before claiming the new one."""
        return 3 if CONTROL.tty else 2

    def erase(self):
        if self._rows:
            # Every row cleared, cursor left on the rule's row -- which is where the next line of
            # output belongs, exactly as it was when this line was one row tall.
            sys.stdout.write("\r\033[2K" + "\033[A\r\033[2K" * (self._rows - 1))
            sys.stdout.flush()
            self._rows = 0

    def draw(self):
        """Repaint every row of the live block IN PLACE.

        The spinner redraws every 0.12s, so this must consume no rows it does not already own: a
        `\\n` in here is a newline twelve times a second, and the first version of the rule had one
        -- the console scrolled itself to death instead of repainting. So the rows below the
        cursor's are claimed exactly once, by the `\\n`s under `not self._rows`, and every frame
        after that moves between them with `ESC [ A` and `ESC [ B`, which do not scroll. `erase()`
        gives them back. Net rows per printed line is what it always was: one, plus the block."""
        if not self.enabled:
            return
        want = self.rows()
        if self._rows and self._rows != want:
            self.erase()  # the block changed height; hand the old one back before claiming this
        out = "\n" * (want - 1) if not self._rows else ""  # claim the rows under this one, once
        out += f"\033[{want - 1}A\r\033[2K" + self.divider()  # up to the rule
        for line in [self.compose()] + ([self.keys()] if want > 2 else []):
            out += "\033[B\r\033[2K" + line  # and back down, one row at a time
        sys.stdout.write(out)
        sys.stdout.flush()
        self._rows = want


class Control:
    """`r` and `s`, from the console or from `.loop/`.

    A run parked behind a usage wall is waiting on a clock, and the one thing that clock cannot
    know is that the account behind it has changed. Logging in somewhere else is not something
    this driver takes part in, so the wait is interruptible: **`r` retries now**, and **`s` stops
    the run** after the current session, exactly as `.loop/stop` does.

    Three rules the shape follows, each of which is a mistake it would otherwise make:

    * **`r` exists only while a wall is up.** It has nothing to end at any other time, and a key
      that silently does nothing is a key that gets pressed twice and then distrusted.
    * **`s` is undoable.** Pressed by accident it would otherwise cost the rest of a run, so it
      toggles, and it is acted on `STOP_GRACE` seconds late -- long enough that even a parked run,
      which reads the flag four times a second, can be told to carry on.
    * **A file does everything a key does.** A keypress needs a terminal, and an overnight run is
      often started with its output redirected, where there is none; `.loop/retry` and
      `.loop/stop` work from another terminal, over SSH and under `nohup`.

    Read from the ticker thread, which is awake eight times a second anyway, AND from `wait()` in
    the main thread -- so the keys still work under `--no-status`, where there is no ticker. Never
    a blocking read: `kbhit`/`select` answer whether anything has been typed, and nothing is taken
    off stdin that was not. The child gets its own stdin pipe (see `run_session`) precisely so that
    this console belongs to the driver alone.
    """

    #: How long an armed stop waits before it is acted on. It is the cancel window, so it is
    #: measured in "noticed the wrong key and pressed it again", not in machine time.
    STOP_GRACE = 5.0

    def __init__(self):
        self.lock = threading.RLock()
        self.stop = False
        self.stop_at = 0.0  # monotonic; before this, an armed stop is still cancellable
        self.retry = False
        self.parked = False  # a wall is up, so `r` has something to end
        self.tty = False
        self.saved = None  # POSIX terminal settings, put back by `disable`

    # -- the terminal ------------------------------------------------------------------

    def enable(self):
        """Take the console, if there is one. On POSIX that means cbreak: a key has to arrive
        without an Enter behind it, and the line discipline would otherwise hold it until one
        came. All of this is optional -- no terminal simply means the files are the channel."""
        if self.tty or not sys.stdin or not sys.stdin.isatty():
            return
        if not IS_WINDOWS:
            if termios is None:
                return
            try:
                self.saved = termios.tcgetattr(sys.stdin)
                tty.setcbreak(sys.stdin.fileno())
            except Exception:
                self.saved = None
                return
        self.tty = True

    def disable(self):
        """Give the terminal back exactly as it was found. Called from `main`'s `finally`, so
        neither a crash nor a Ctrl-C can leave a shell sitting in cbreak with no echo."""
        if self.saved is not None and termios is not None:
            try:
                termios.tcsetattr(sys.stdin, termios.TCSADRAIN, self.saved)
            except Exception:
                pass
        self.saved = None
        self.tty = False

    # -- what has been typed -----------------------------------------------------------

    def _typed(self):
        """Whatever is already sitting in the console buffer, or nothing at all. Never blocks."""
        keys = []
        if not self.tty:
            return keys
        try:
            if IS_WINDOWS:
                while msvcrt.kbhit():
                    ch = msvcrt.getwch()
                    if ch in ("\x00", "\xe0"):  # a function or arrow key: drop its second half
                        msvcrt.getwch()
                        continue
                    keys.append(ch)
            else:
                while select.select([sys.stdin], [], [], 0)[0]:
                    ch = sys.stdin.read(1)
                    if not ch:
                        break
                    keys.append(ch)
        except Exception:
            # A console that cannot be read is not a run that should end. Fall back to the files.
            self.tty = False
        return keys

    def poll(self):
        """One non-blocking look at the console. Called from the ticker and from `wait`."""
        with self.lock:
            for ch in self._typed():
                if ch == "\x03":
                    # In some console modes Windows hands Ctrl-C to `getwch` as a character
                    # instead of raising. Put it back where the person pressing it meant it to go.
                    _thread.interrupt_main()
                    continue
                key = ch.lower()
                if key == "s":
                    self.stop = not self.stop
                    self.stop_at = time.monotonic() + self.STOP_GRACE
                    say(f"   [s] stop requested -- the run ends after the current session, "
                        f"unless s is pressed again within {self.STOP_GRACE:.0f}s" if self.stop
                        else "   [s] stop cancelled -- the run carries on",
                        C.YELLOW if self.stop else C.GREEN)
                elif key == "r":
                    if self.parked:
                        self.retry = True
                        say("   [r] retry requested -- the wait ends now", C.GREEN)
                    else:
                        say("   [r] does nothing right now; it ends a usage limit wait", C.GRAY)

    # -- what the driver asks ----------------------------------------------------------

    def stop_in(self):
        """Seconds an armed stop still has left to be cancelled in, or 0 once it is past."""
        return max(0.0, self.stop_at - time.monotonic()) if self.stop else 0.0

    def stop_reason(self):
        """Why the run should end now, or "". The grace is why this is not simply the flag."""
        if self.stop and self.stop_in() <= 0:
            return "s was pressed at the console"
        if STOP.exists():
            return f"{rel_to_root(STOP)} present"
        return ""

    def pending(self):
        """Is there a request the caller would act on this instant? Non-consuming, and it asks
        `stop_reason` rather than the flag so that a stop still inside its cancel window does not
        spin `wait` in a loop of instant returns."""
        return bool(self.stop_reason()) or self.retry or RETRY.exists()

    def take_retry(self):
        """Consume a retry request. The file goes with it, so one request cannot end two walls."""
        with self.lock:
            asked = self.retry or RETRY.exists()
            self.retry = False
        RETRY.unlink(missing_ok=True)
        return asked


TICKER = StatusLine()
CONTROL = Control()


# --------------------------------------------------------------------------- the console log
#
# Two logs existed before this one and neither held a whole run. `.loop/logs/<run>-NNNN.log` is
# the SESSION's NDJSON -- everything the agent did and nothing else -- and `.loop/log.md` is the
# ledger, one line per session. The driver's own half went to the console and to nowhere: the
# acceptance check is the slowest thing a run does, and when it failed after the fact the only
# record of *why* was a single line in the ledger, with the build log, the fixture's real stdout
# and the valgrind report already gone. Debugging it meant reproducing it.
#
# So every line the driver prints is teed here, stamped, and so is the full stdout and stderr of
# every subprocess `capture()` runs -- the console still shows only the failure, because a green
# `cargo test` on screen is noise and the same text on disk is the next bug report.
#
# Two destinations, on purpose:
#
#   <run>-console.log   the whole run, human-readable, every line stamped: the driver's steps,
#                       the rendered session transcripts, every check's output, in the order it
#                       happened. This is the file to open when a run went wrong.
#   <run>-NNNN.log      the session's own NDJSON also gets the DRIVER lines that belong to that
#                       session, as `loop_console` events, so one session's file is self-contained
#                       -- its acceptance check included. The rendered transcript is deliberately
#                       NOT mirrored: it is already in that file, as the events it was rendered
#                       from, and echoing it back would double every session log.


class ConsoleLog:
    """The tee behind `say()` and `capture()`. Silent and harmless until `open_run()`."""

    def __init__(self):
        self.lock = threading.Lock()
        self.run = None
        self.session = None

    def open_run(self, path):
        try:
            self.run = path.open("a", encoding="utf-8", errors="replace", newline="\n")
        except OSError:
            self.run = None

    def open_session(self, path):
        self.close_session()
        try:
            self.session = path.open("a", encoding="utf-8", errors="replace", newline="\n")
        except OSError:
            self.session = None

    def close_session(self):
        with self.lock:
            if self.session:
                try:
                    self.session.close()
                except OSError:
                    pass
            self.session = None

    def close(self):
        self.close_session()
        with self.lock:
            if self.run:
                try:
                    self.run.close()
                except OSError:
                    pass
            self.run = None

    def line(self, text, driver=False):
        """One line on the console. `driver` marks it as the driver's own rather than an echo of
        the session, which is what decides whether the session's NDJSON gets it too."""
        if not (self.run or (driver and self.session)):
            return
        stamp = f"{datetime.now():%H:%M:%S.%f}"[:-3]
        with self.lock:
            self._write(self.run, "".join(f"[{stamp}] {ln}\n" for ln in (text or "").split("\n")))
            if driver:
                self._event({"type": "loop_console", "ts": stamp, "text": text})

    def block(self, title, body, driver=True):
        """A subprocess's captured output: named, indented, and never shown on the console. The
        indent is what keeps a `cargo test` summary from reading like the driver's own lines."""
        if not (self.run or self.session):
            return
        stamp = f"{datetime.now():%H:%M:%S.%f}"[:-3]
        text = (body or "").replace("\r\n", "\n").rstrip("\n")
        with self.lock:
            self._write(self.run, f"[{stamp}] {title}\n")
            if text:
                self._write(self.run, "".join(f"[{stamp}]   | {ln}\n" for ln in text.split("\n")))
            if driver:
                self._event({"type": "loop_output", "ts": stamp, "what": title, "text": text})

    def raw(self, text):
        """A line straight into the session's NDJSON, unstamped and unmirrored. This is the
        harness's own event stream -- the one thing in that file no reader should find
        reformatted -- and it goes through here so the file has exactly one open handle."""
        with self.lock:
            self._write(self.session, text)

    # -- the two writes, both of which must never take a run down ------------------------

    def _write(self, fh, text):
        if not fh:
            return
        try:
            fh.write(text)
            fh.flush()
        except (OSError, ValueError):
            pass

    def _event(self, obj):
        self._write(self.session, json.dumps(obj) + "\n")


CONSOLE = ConsoleLog()


def say(text="", colour=None, driver=False):
    line = C.paint(text, colour) if colour else text
    CONSOLE.line(text, driver=driver)
    with TICKER.lock:
        TICKER.erase()
        sys.stdout.write(line + "\n")
        sys.stdout.flush()
        TICKER.draw()


def wait(seconds, label, until=None):
    """`time.sleep`, with the status line counting it down. A silent multi-minute sleep is the one
    pause a watcher cannot tell from a crash.

    `until` is a predicate looked at four times a second; a true answer ends the wait there and
    then. That is what makes a keypress during a usage wall feel immediate rather than landing at
    the end of whatever slice happened to be in flight."""
    end = time.monotonic() + seconds
    while True:
        left = end - time.monotonic()
        if left <= 0:
            return
        CONTROL.poll()
        if until and until():
            return
        TICKER.set(detail=f"{label}{TICKER.sep}{hms(left)} left")
        time.sleep(min(0.25, left))


def mmss(seconds):
    """`123` -> `2m03s`. Durations here run from milliseconds to tens of minutes, and a bare
    float of seconds is unreadable at the top of that range."""
    seconds = int(seconds)
    return f"{seconds // 60}m{seconds % 60:02d}s" if seconds >= 60 else f"{seconds}s"


def hms(seconds):
    """`mmss` below an hour, `4h52m` above it. Everything this driver times is minutes long
    except one thing -- the wait for a usage window to reopen, which is hours -- and `292m11s`
    is a number you have to do arithmetic on before you can decide whether to wait up for it."""
    seconds = int(max(0, seconds))
    return mmss(seconds) if seconds < 3600 else f"{seconds // 3600}h{seconds % 3600 // 60:02d}m"


def step(text, colour=C.GRAY):
    """One line of between-sessions progress, stamped with the wall clock.

    Everything a session does prints itself as it happens; everything the DRIVER does between
    two sessions used to print nothing at all -- and the driver's half is the slow half (a
    build, both suites, the WSL leg, the valgrind sweep, then `orient.py` for the next one).
    A run therefore looked stalled for minutes at a time with the last session's status line
    sitting on screen. Every phase now names itself before it starts and says what it cost.

    `driver=True`: this is the driver talking, not an echo of the session, so it is mirrored into
    the running session's own log as well as the run's. See `ConsoleLog`."""
    say(f"   [{datetime.now():%H:%M:%S}] {text}", colour, driver=True)


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

    @staticmethod
    def call_target(block):
        """`Edit tools/loop.py`, `Bash cargo build ...` -- the tool plus the one argument that says
        what it is about. For the status line only: the console above it already has every argument,
        and a bottom row that has to fit in a terminal width gets the name of the thing."""
        name = str(block.get("name") or "tool")
        args = block.get("input")
        if not isinstance(args, dict):
            return name
        for key in ("file_path", "path", "pattern", "command", "prompt", "url", "notebook_path"):
            value = args.get(key)
            if isinstance(value, str) and value.strip():
                first = value.strip().split("\n", 1)[0]
                return f"{name} {first[:60]}"
        return name

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
                    TICKER.note("writing")
                    self.wrapped(b.get("text"), "   ", C.WHITE, 0)
                elif btype == "thinking":
                    TICKER.note("thinking")
                    self.wrapped(b.get("thinking"), "   . ", C.MAGENTA, self.max_result_lines)
                elif btype == "tool_use":
                    if b.get("id"):
                        self.tool_names[str(b["id"])] = str(b.get("name"))
                    say(f"   > {b.get('name')}", C.CYAN)
                    TICKER.tool(self.call_target(b))
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


def capture(exe, args, timeout=1800, cwd=None):
    """Run a program with stdout and stderr captured SEPARATELY -- the acceptance list distinguishes
    them (a backtrace and FATAL go to stderr, program output to stdout).

    `cwd` defaults to the repository root, which is what every cargo and `mwl` invocation wants. A
    `command` check names its own, because an `npm` script only finds its `package.json` from the
    directory that holds it.

    Everything run through here -- the argv, what it cost, its exit code and BOTH streams whole --
    goes to the console log, green or not. That is the acceptance check's entire record: the
    console prints one line per check and the first line of a failure, which is the right amount to
    watch and far too little to debug afterwards. See `ConsoleLog`."""
    path = shutil.which(exe) or exe
    began = time.monotonic()
    try:
        p = subprocess.run(
            [path, *args],
            cwd=cwd or ROOT,
            capture_output=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        r = Result(-1, "", f"timed out after {timeout}s")
    except OSError as exc:
        r = Result(-1, "", str(exc))
    else:
        r = Result(p.returncode, p.stdout or "", p.stderr or "")
    where = "" if cwd in (None, ROOT) else f" (in {cwd})"
    head = f"$ {exe} {' '.join(args)}{where} -> exit {r.code} in {mmss(time.monotonic() - began)}"
    CONSOLE.block(head, "\n".join(s for s in (r.out.rstrip("\n"), r.err.rstrip("\n")) if s.strip()))
    return r


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

    def suite(self, mwl_args):
        """`mwl test <dir>` on this leg. Separate from `run` because a suite takes its own
        subcommand, and separate from calling `self.binary` directly because on the WSL leg that
        path is a Linux one and Windows cannot execute it."""
        return capture(self.binary, mwl_args)


class WslLeg(NativeLeg):
    """Windows only: the same fixtures against a Linux build, through the default WSL distro.

    Building once matters here for the reason it does natively, plus a `wsl.exe` round trip per
    invocation. The 9p mount at `/mnt/<drive>` is *not* what makes it expensive -- the workspace is
    1,412 files, 190 of them `.rs`, and a no-op build across it costs 0.31s. Where the *target*
    directory lives is what matters: see `Goal.wsl_target`, and commands.md for the measurement.
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

    def suite(self, mwl_args):
        return self.bash(f"cd {self.repo} && {self.binary} " + " ".join(mwl_args))


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

# What a memoizable check reads, and so what its verdict is keyed on -- see `Goal.inputs_id`.
# `tests/` and `docs/` are deliberately absent: nothing keyed on this runs a `.mwlt` case.
MEMO_DIRS = ("crates", "examples")
MEMO_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "docs/agent/loop-goal.toml")
MEMO_NOT_INPUTS = {"target", "node_modules", "out", ".vscode-test"}

#: The sweep runs several fixtures at once. It was strictly serial once, and is 42% of an
#: acceptance check -- 21.3 of its 50.3 minutes over the 20260826-142040 run, 58s a session for 20
#: fixtures -- so how wide it runs matters. **How wide is `tools/machine.py`'s decision, not this
#: file's**: it is a property of the box, measured there once and cached, and a constant here was
#: a property of the box it was measured on.
#:
#: Nothing is traded for the concurrency. A leak verdict is per-process and deterministic, so
#: concurrency cannot change one -- unlike the abi-probe cost guards, which are cost-class
#: assertions and must have an idle machine. Those run strictly AFTER this sweep, which is why
#: they still can, and it is why `machine.py` hands out half a box and not all of it.


def rustc_version():
    """The exact compiler, so a toolchain bump invalidates every memoized verdict."""
    p = subprocess.run(["rustc", "-vV"], capture_output=True, encoding="utf-8", errors="replace")
    return (p.stdout or "") + (p.stderr or "")

# Held by whatever background release build is in flight; see `Goal.prebuild`.
PREBUILD_LOCK = threading.Lock()

# A `command` check is an argv run in a directory, exit 0, with `want` as ordered substrings across
# both streams. It exists because the acceptance test grew a leg that is neither `mwl run` stdout nor
# `cargo test`: from M4B, `editors/vscode` is TypeScript and its suites are `npm` scripts. Kept
# deliberately general -- a check kind per external tool is how a driver becomes a build system.
#
# It is not a PROGRAM_KIND, so it runs ONCE between the legs rather than per leg. That is the right
# default and not an accident of implementation: the WSL leg exists because a JIT is where a
# calling-convention divergence hides, and a TextMate grammar has no calling convention.


class Goal:
    """The acceptance test, read from docs/agent/loop-goal.toml. `check()` returns "" when everything
    passes, or the first failure as one line -- with one deliberate exception, a suite that runs
    clean but holds fewer cases than its `min_passing`. That is the loop's stopping condition rather
    than a claim about the language, so it is held back and reported only once everything else has
    run; otherwise a corpus still being grown short-circuits the memory-safety sweep behind it for
    as many sessions as the growing takes.

    Two things are memoized, and neither of them skips a check:

    * **Within one run**, an identical `args` list runs cargo once. The list holds `mwl-runtime`
      twice on purpose -- stage 0 and stage 5 name different guard tests on it -- and running the
      crate's suite a second time cannot answer differently.
    * **Across runs**, the three checks in `EXPENSIVE` are remembered against a content hash of
      *the files they read* -- `crates/`, `examples/`, the manifests, the toolchain and the goal
      file (`inputs_id`). Those inputs being bit-identical is the whole argument: a deterministic
      check over identical bytes cannot reach a different verdict, which is `verify.py`'s rule for
      its own green cache. It used to key on the tree instead, HEAD included, and every session
      commits -- so the memo never once fired inside a run. `tests/` and `docs/` are not inputs to
      any of the three, and 8 of 22 sessions of one measured run touched nothing else.
    """

    def __init__(self, spec):
        self.files = spec.get("files", [])
        self.checks = spec.get("check", [])
        self.valgrind_skip = set(spec.get("valgrind", {}).get("skip", []))
        self.wsl_target = spec.get("wsl", {}).get("target_dir", "/var/tmp/mwl-target-wsl")
        self.program_checks = [c for c in self.checks if c["kind"] in PROGRAM_KINDS]
        self.ran = []  # (label, seconds) for every check this run actually paid for
        self.short = []  # `min_passing` thresholds not met; see `check()`
        self.verbose = False  # narrate each check as it starts and what it cost
        self._begun = 0.0  # monotonic start of the current check(), for the elapsed stamp
        self._cargo = {}  # args tuple -> Result, within one check() call
        self._tree = ""
        self._inputs = None  # content hash of what the memoizable checks read; see `inputs_id`
        self._green = {}  # check name -> the `inputs_id` it was last green over
        self._memoized = {c["name"] for c in self.checks if c.get("memoize") and "name" in c}
        self._prebuild = None  # the thread warming the release profile
        self._prebuilt = ()  # the args it is warming, so `cargo()` knows to wait for it
        cargo = [c for c in self.checks if c["kind"] not in PROGRAM_KINDS]
        # Stage 0 is catch-up: work a later ADR reopened inside a milestone that
        # was already reported done. It runs before everything else so the
        # ledger names it while it is unfinished -- a Stage 3 fixture failing is
        # not the thing the loop should be told about first. See loop-goal.md.
        catch_up = [str(c.get("stage", "")).startswith("0") for c in cargo]
        # A `--release` check is held back to the end of the sweep whatever stage it is labelled
        # with -- the one place the stage order above is not the run order, and `check()` says why.
        self.release_checks = [c for c in cargo if "--release" in c.get("args", [])]
        held = {id(c) for c in self.release_checks}
        self.catch_up_checks = [c for c, first in zip(cargo, catch_up)
                                if first and id(c) not in held]
        self.cargo_checks = [c for c, first in zip(cargo, catch_up)
                             if not first and id(c) not in held]

    # -- measuring, and the two memos --------------------------------------------------

    def trace(self, msg):
        """Name the check about to run, stamped with how long the whole sweep has been going.
        Silent unless the caller asked for it; see `step()` for why it exists at all.

        The status line gets it either way: `verbose` decides whether the console keeps a scrolling
        record of every check, not whether a watcher can see which one is running."""
        TICKER.set(detail=msg)
        if self.verbose:
            say(f"   .. +{mmss(time.monotonic() - self._begun):>6}  {msg}", C.GRAY)

    def timed(self, label, thunk):
        """Run `thunk`, recording what it cost. `check()` reports the total and the three
        slowest, because an acceptance test nobody has ever timed is one nobody can tune.

        Anything that took a second or more also says so on the spot: the summary at the end
        is no help while you are watching a run and wondering whether it is still moving."""
        started = time.monotonic()
        try:
            return thunk()
        finally:
            spent = time.monotonic() - started
            self.ran.append((label, spent))
            TICKER.advance()
            if self.verbose and spent >= 1:
                say(f"   .. {'':>7}  {label} took {mmss(spent)}", C.GRAY)

    def cargo(self, args):
        """`cargo` with the result shared by every check that asks for the same argument list."""
        key = tuple(args)
        if key == self._prebuilt and self._prebuild:
            self.trace("waiting for the release build started at the top of the sweep")
            self.timed("release prebuild (overlapped)", self._prebuild.join)
            self._prebuild = None
        if key not in self._cargo:
            self._cargo[key] = capture("cargo", args)
        return self._cargo[key]

    # -- the release build, moved off the critical path ---------------------------------

    def release_args(self):
        """The one check in the goal whose cost is a BUILD and not a test.

        `--release` is a different profile from everything else here, so nothing it needs is on
        disk when the sweep starts, and this workspace's release profile is `lto = "thin"` with
        `codegen-units = 1` -- measured at 133s after a one-line change to `mwl-runtime`, against
        the ~130s the whole rest of the sweep costs. Found by its `--release` rather than named,
        so a goal that moves the guard to another crate does not have to come back here.

        `None` when there is no such check, or when its verdict is already remembered against this
        tree: `prebuild` would then be warming a profile nothing is going to ask about."""
        for c in self.checks:
            if c["kind"] in PROGRAM_KINDS or "--release" not in c.get("args", []):
                continue
            return None if self.remembered(c["name"]) else c["args"]
        return None

    def prebuild(self):
        """Start that build now, in the background, and let the rest of the sweep run beside it.

        The sweep was strictly serial, so the release build was ~60% of an acceptance check that a
        session waits out before the next one can start. Nothing else in the sweep touches the
        release profile, and two cargos on one `target/` were measured NOT to block each other --
        a debug build finished in its usual 4.5s beside a release build that took its usual 129s.

        `--no-run` on purpose, and this is the part that must not be traded away: the guards are
        cost-class assertions, and a cost measured on a machine that is simultaneously linking is
        not the cost. So the BUILD overlaps and the RUN does not -- `cargo()` joins this thread
        before it starts the real invocation, which by then is a no-op build and a 3s test run."""
        args = self.release_args()
        if not args:
            return
        def build():
            # One at a time across the whole run. A sweep that fails before it reaches the guard
            # returns with this thread still building -- correctly, since the work is wanted either
            # way -- and `load_goal()` hands the next session a fresh `Goal` that knows nothing
            # about it. Without the lock those two cargos would build the same units at once.
            with PREBUILD_LOCK:
                capture("cargo", [*args, "--no-run"])

        self._prebuilt = tuple(args)
        self._prebuild = threading.Thread(target=build, daemon=True)
        self._prebuild.start()
        self.trace("release build started in the background")

    def tree_id(self):
        """HEAD, plus a hash of everything not committed. Two runs with the same id are two runs
        over the same bytes, so a deterministic check cannot answer them differently.

        Kept for reporting. It is NOT what the memos key on any more, and `inputs_id` says why."""
        head = git("rev-parse", "HEAD") or "no-head"
        dirty = git("status", "--porcelain") + "\n" + git("diff", "HEAD")
        return head + ":" + hashlib.blake2b(dirty.encode("utf-8", "replace"),
                                            digest_size=8).hexdigest()

    def inputs_id(self):
        """A content hash of every file the memoizable checks READ, and the compiler that builds it.

        This is the whole fix for a memo that never fired. It used to key on `tree_id` -- HEAD plus
        the dirty tree -- and every session commits, so HEAD moved every session and all three
        expensive verdicts were thrown away whatever had changed. Measured over the 21-session run
        in `.loop/logs/20260826-142040-*`: 8 of 22 sessions changed nothing any of these three
        checks reads, and NO session in the whole run touched `examples/` at all, yet the valgrind
        sweep ran 22 times out of 22 at 58 seconds a time.

        What they actually read is below, and it is deliberately a SUPERSET of what they need --
        the whole of `crates/` for the binary they run, the whole of `examples/` for the programs,
        every manifest, the toolchain, and the goal file whole (its `files`, `[valgrind] skip` and
        `[[check]]` lists all steer the sweep, and hashing the whole file rather than those three
        sections means a section added later cannot be missed). `tests/` and `docs/` are absent on
        purpose: no check keyed on this executes a `.mwlt` case or reads a document.

        Widening this is safe and narrowing it is not, so anything unreadable returns `None` and
        every memo falls through to running for real, which is `verify.py`'s rule as well.
        """
        try:
            h = hashlib.blake2b(digest_size=16)
            h.update(rustc_version().encode("utf-8", "replace"))
            for name in MEMO_FILES:
                p = ROOT / name
                h.update(name.encode("utf-8") + b"\0")
                h.update(p.read_bytes() if p.is_file() else b"")
                h.update(b"\0")
            for top in MEMO_DIRS:
                base = ROOT / top
                if not base.is_dir():
                    continue
                for dirpath, dirnames, filenames in os.walk(base):
                    dirnames[:] = sorted(d for d in dirnames if d not in MEMO_NOT_INPUTS)
                    rel = Path(dirpath).relative_to(ROOT)
                    for f in sorted(filenames):
                        h.update((rel / f).as_posix().encode("utf-8") + b"\0")
                        h.update((ROOT / rel / f).read_bytes())
                        h.update(b"\0")
            return h.hexdigest()
        except OSError:
            return None

    def memoizable(self, name):
        """Whether a green verdict on `name` may be remembered against the tree that produced it.

        Two sources, and both are about cost rather than confidence: `EXPENSIVE` is the three checks
        this driver has always known are minutes long, and `_memoized` is whatever the goal itself
        marked `memoize = true` -- which a `command` check needs, since the goal is the only thing
        that knows an `npm` script downloads an editor."""
        return name in EXPENSIVE or name in self._memoized

    def remembered(self, name):
        """Was this check green over inputs bit-identical to the ones on disk right now?

        `None` from `inputs_id` -- something unreadable -- is not a match, so the check runs."""
        return (self.memoizable(name) and self._inputs is not None
                and self._green.get(name) == self._inputs)

    def remember(self, name):
        if self.memoizable(name) and self._inputs is not None:
            self._green[name] = self._inputs
            try:
                GOALCACHE.parent.mkdir(exist_ok=True)
                GOALCACHE.write_text(
                    json.dumps({"tree": self._tree, "inputs": self._inputs,
                                "green": dict(sorted(self._green.items()))}, indent=1),
                    encoding="utf-8", newline="\n",
                )
            except OSError:
                pass

    def load_green(self):
        self._tree = self.tree_id()
        self._inputs = self.inputs_id()
        self._green = {}
        try:
            entry = json.loads(GOALCACHE.read_text(encoding="utf-8"))
            green = entry.get("green")
            if isinstance(green, dict):
                self._green = {k: v for k, v in green.items() if isinstance(v, str)}
            elif isinstance(green, list) and entry.get("tree") == self._tree:
                # The old shape: a list of names under one whole-tree key. Honour it for this run
                # rather than throwing a green verdict away on the version that changes the format.
                self._green = {name: self._inputs for name in green}
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

        if c["kind"] == "command":
            if self.remembered(c["name"]):
                return ""
            r = self.timed(label, lambda: capture(c["argv"][0], c["argv"][1:],
                                                  cwd=ROOT / c.get("cwd", ".")))
            if r.code != 0:
                return f"{label}: exit {r.code} -- {r.first_err_line}"
            missing = ordered_in(r.out + "\n" + r.err, c.get("want", []))
            if missing:
                return f"{label}: {missing}"
            self.remember(c["name"])
            return ""

        if c["kind"] == "mwl-suite":
            # The suite runner is the CLI the leg already built; `cargo run` here would be one
            # more workspace fingerprint scan to start a binary sitting on disk. Through the leg
            # rather than at the binary, because `--leg-only` runs these on the Linux build, whose
            # path Windows cannot execute.
            r = self.timed(label, lambda: leg.suite(c["args"]))
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
            # `cases` is the `.mwlt` twin of `cargo-named`, and it exists for the same reason: a
            # suite is green when a case was never written, and `min_passing` cannot tell the
            # difference between "the corpus grew" and "the corpus grew somewhere else". A named
            # case must be on disk AND not have been skipped -- an `--ORACLE--` whose probe fails
            # skips silently, and the SKIP line is the only place that shows.
            #
            # Path separators are normalized both ways: `mwl test` prints whatever the platform's
            # `Path::display` gives it, so the same case is `tests/…` here and `tests\…` there.
            flat = both.replace("\\", "/")
            for case in c.get("cases", []):
                want = case.replace("\\", "/")
                if not (ROOT / case).exists():
                    return f"{label}: case {want} is not written yet"
                if f"SKIP {want}" in flat:
                    return f"{label}: case {want} was skipped, so nothing ran it"
            if int(m.group(1)) < c["min_passing"]:
                # Held, not returned. A `min_passing` threshold is the loop's STOPPING condition --
                # "is the corpus big enough yet" -- and not a correctness signal; the two lines
                # above are the correctness half and they have just passed, so every case that
                # exists runs and none of them fails. Returning here would short-circuit the Stage 5
                # guards, the second leg and the valgrind sweep for as long as the corpus is still
                # growing, which is dozens of sessions, and priority 1 does not wait behind
                # priority 4. `check()` reports this after all of them.
                self.short.append(
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
            self.trace("valgrind sweep (green on these inputs already)")
            return ""
        if leg.name == "native" and shutil.which("valgrind") is None:
            self.trace("valgrind sweep skipped -- no valgrind on this platform")
            return ""  # not a failure: this platform simply has no valgrind leg

        targets = [f for f in self.files if f not in self.valgrind_skip]
        if not targets:
            return ""

        def cmd_for(f):
            return (f"cd {leg.repo} && " if leg.name == "wsl" else "") + (
                "valgrind --error-exitcode=1 --leak-check=full "
                f"--errors-for-leak-kinds=definite -q {leg.binary} run {f}"
            )

        def shell(line):
            """One command where this leg runs it."""
            return leg.bash(line) if leg.name == "wsl" else capture("bash", ["-lc", line])

        def probe():
            # Once per machine, and only where the work actually runs -- on Windows that is inside
            # WSL, whose core count and memory come from `.wslconfig` and not from the host. The
            # sample is one real fixture, so the baseline it records is this box's serial cost for
            # exactly the work about to be run wide.
            self.trace("probing this machine (once) -- cores, free memory, one fixture serially")
            def out(line):
                r = shell(line)
                return r.code, r.out + r.err
            return machine.posix_probe(out, sample=cmd_for(targets[0]))

        prof = machine.profile(leg.name, probe=probe)
        jobs = machine.jobs(leg.name, ceiling=len(targets), envs=("MWL_VALGRIND_JOBS",))
        self.trace(f"valgrind sweep: {len(targets)} fixtures, {jobs} at a time "
                   f"({prof.get('cores', '?')} cores on the {leg.name} leg)")

        def sweep(f):
            # Several of these are in flight, so the ticker's detail is "one of the running ones"
            # rather than "the one running". `timed` advances the counter under the ticker's own
            # lock, so the bar itself stays exact.
            self.trace(f"valgrind {f}")
            return f, self.timed(f"valgrind {f}", lambda: shell(cmd_for(f)))

        # `map` keeps input order, so the failure reported is the first fixture in the goal's own
        # list however the workers finished. It does not short-circuit, which is the one behaviour
        # that changes: a red sweep runs all of them and names EVERY leaking fixture instead of
        # stopping at the first. That is worth the seconds -- "one fixture leaks" and "twelve do"
        # are different bugs, and the parallel sweep pays a fraction of what the serial one did to
        # answer both.
        fails, began = [], time.monotonic()
        with ThreadPoolExecutor(max_workers=jobs) as pool:
            for f, r in pool.map(sweep, targets):
                if r.code != 0:
                    fails.append(f"valgrind {f}: exit {r.code} -- {r.first_err_line}")
        # What the width bought, against the serial cost measured on this same box. Recorded rather
        # than printed: it is how a later run says whether the policy is still right here, and it
        # costs nothing to keep.
        spent = time.monotonic() - began
        if prof.get("sample_s") and spent > 0:
            machine.remember(leg.name, sweep_s=round(spent, 1),
                             sweep_speedup=round(prof["sample_s"] * len(targets) / spent, 2))
        if fails:
            return fails[0] + (f"  (and {len(fails) - 1} more: "
                               f"{', '.join(x.split(':')[0] for x in fails[1:])})"
                               if len(fails) > 1 else "")
        self.remember("valgrind sweep")
        return ""

    # -- the whole thing ----------------------------------------------------------------

    def plan_size(self, mode, wsl):
        """How many `timed()` steps the sweep about to start will pay for.

        Countable exactly, and that is the whole reason the status line shows a percentage here and
        nowhere else: the list is fixed on disk, the legs are known before the first build, and the
        two memos say up front what will be skipped rather than discovering it halfway through. Call
        it only after `load_green()`, or every remembered check is counted as work still to do.

        It is an upper bound in one direction only -- a failing check returns early, so a run can
        end at 40% -- and it never undercounts, so the bar cannot reach 100% with work left.
        """
        programs = len(self.program_checks)
        sweep = sum(1 for f in self.files if f not in self.valgrind_skip)
        # Only the wsl leg's valgrind is a given: on a native leg the sweep is skipped outright
        # when the platform has no valgrind, and counting it would strand the bar short of 100%.
        sweepable = not self.remembered("valgrind sweep") and bool(wsl or shutil.which("valgrind"))

        n = 1  # the input fingerprint, already spent by the time this is called
        if mode == "leg":
            n += 1 + programs  # the leg's build, then every fixture on it
            n += sum(1 for c in self.cargo_checks if c["kind"] == "mwl-suite")
            return n + (sweep if sweepable else 0)

        n += 1 + len(self.catch_up_checks) + programs  # native build, catch-up, native fixtures
        n += sum(1 for c in self.cargo_checks if not self.remembered(c["name"]))
        n += sum(1 for c in self.release_checks if not self.remembered(c["name"]))
        # The whole Linux leg -- probe, build and fixtures -- is skipped when its two consumers are
        # both green over these inputs, so none of the three is counted then either.
        if not (self.remembered("wsl leg") and self.remembered("valgrind sweep")):
            n += 1  # the wsl probe
            if wsl:
                n += 1 + (0 if self.remembered("wsl leg") else programs)
        return n + (sweep if sweepable else 0)

    def begin(self, mode="check"):
        """Reset the per-run bookkeeping, and confirm every fixture is still on disk before
        anything is built. Shared by the two entry points below."""
        self.ran = []
        self._cargo = {}
        self.short = []  # thresholds not met yet, judged after everything else
        self._begun = time.monotonic()
        TICKER.set(done=0, total=0)
        # Two hashes: the `git diff HEAD` behind `tree_id`, and the content walk of `crates/` and
        # `examples/` behind `inputs_id`. Both are a visible pause before any check has started, so
        # the ticker is told what is happening rather than appearing to hang on nothing.
        self.trace("fingerprinting what the memoizable checks read")
        self.timed("input fingerprint", self.load_green)
        TICKER.set(total=self.plan_size(mode, wsl_available()))
        for f in self.files:
            if not (ROOT / f).exists():
                return f"{f} is missing -- the acceptance fixtures are fixed, see docs/agent/loop-goal.md"
        return ""

    def check(self, verbose=False):
        self.verbose = verbose
        trace = self.trace

        TICKER.set(phase="acceptance check")
        fail = self.begin("check")
        if fail:
            return fail

        # Before anything else, because it is the longest pole and it is a build: see `prebuild`.
        self.prebuild()

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
                trace(f"cargo {c['name']} (green on these inputs already)")
                continue
            trace(f"cargo {c['name']}")
            fail = self.cargo_check(c, native)
            if fail:
                return fail
            self.remember(c["name"])

        # Windows is green, so now pay for the Linux leg.
        #
        # The build is skipped only when BOTH things that would use the binary are already green
        # over these inputs -- the leg's own fixtures and the valgrind sweep behind it. That pairing
        # is the safety: if either still has to run, the build runs, so nothing ever reaches a
        # missing or stale binary. Skipping the build alone would be worse than useless, because
        # `leg` would stay `native` and the sweep would quietly downgrade to a platform with no
        # valgrind on it.
        leg = native
        trace("asking whether there is a wsl leg")
        if self.remembered("wsl leg") and self.remembered("valgrind sweep"):
            trace("wsl leg and valgrind sweep both green on these inputs -- neither is rebuilt")
        elif self.timed("wsl probe", wsl_available):
            leg = WslLeg(self.wsl_target)
            trace("building the wsl CLI")
            fail = self.timed("wsl build", leg.prepare)
            if fail:
                return fail
            if self.remembered("wsl leg"):
                trace("wsl fixtures (green on these inputs already)")
            else:
                for c in self.program_checks:
                    trace(f"{leg.name} {c['file']}")
                    fail = self.program_check(leg, c)
                    if fail:
                        return fail
                self.remember("wsl leg")

        trace("valgrind sweep")
        fail = self.valgrind(leg)
        if fail:
            return fail

        # The `--release` checks, held back from their stages to here. Two reasons, and the second
        # is the one that must not be traded away:
        #
        # * `prebuild` has been building them since before the native build, and this is the point
        #   at which it has had the whole sweep to finish in. Measured: reached in its stage-0
        #   position, the sweep waited 1m50s for a build with 41s of work in front of it and 1m57s
        #   behind it; from here the wait is nothing and the run is 3s.
        # * They are cost-class guards, and a cost measured while a WSL build and a valgrind sweep
        #   are running is not the cost. Here, everything else has finished and the machine is idle.
        #
        # What it costs is reporting order: a red guard is now named after a red fixture rather than
        # before one. That is the smaller loss -- and a red guard is not reported LATER in wall-clock
        # terms either, because the sweep it now runs behind is shorter than the build it used to
        # wait on.
        for c in self.release_checks:
            if self.remembered(c["name"]):
                trace(f"cargo {c['name']} (green on these inputs already)")
                continue
            trace(f"cargo {c['name']}")
            fail = self.cargo_check(c, native)
            if fail:
                return fail
            self.remember(c["name"])

        # Last, because a corpus that is merely still growing is the one failure that must not hide
        # anything: everything above is a claim about whether the language is correct on both legs
        # and leaks nothing, and all of it has now run. See `cargo_check`'s `min_passing` arm.
        return self.short[0] if self.short else ""

    def leg_check(self, verbose=False):
        """The Linux leg on its own: every fixture, both suites and the valgrind sweep against a
        Linux build. The cargo suites are left out because they do not divide by target -- what
        this leg exists to catch is a calling-convention divergence in the JIT or a leak in the
        refcount protocol, and both of those show up through the CLI.

        This was `tools/wsl-acceptance.sh`, and it is a method rather than a shell script because
        that script carried its own frozen copy of every fixture's expected output. A second copy
        of a frozen list drifts, and that one had: by the time the two were run side by side it was
        seven fixtures and seven valgrind targets behind `loop-goal.toml`, while its own header
        still said it ran "the same commands". `WslLeg` also already solves the quoting the script
        was written as a file to avoid.
        """
        self.verbose = verbose
        trace = self.trace

        TICKER.set(phase="linux leg")
        fail = self.begin("leg")
        if fail:
            return fail

        leg = WslLeg(self.wsl_target) if wsl_available() else NativeLeg()
        trace(f"building the {leg.name} CLI")
        fail = self.timed(f"{leg.name} build", leg.prepare)
        if fail:
            return fail

        for c in self.program_checks:
            trace(f"{leg.name} {c['file']}")
            fail = self.program_check(leg, c)
            if fail:
                return fail

        for c in self.cargo_checks:
            if c["kind"] != "mwl-suite":
                continue
            trace(f"{leg.name} {c['name']}")
            fail = self.cargo_check(c, leg)
            if fail:
                return fail

        trace("valgrind sweep")
        fail = self.valgrind(leg)
        if fail:
            return fail
        return self.short[0] if self.short else ""

    def summary(self):
        """One line: what this run cost, and the three checks that cost the most of it.

        The headline is WALL CLOCK, not the sum of the per-check timers. Those two stopped being
        the same number when the valgrind sweep went four-wide: twenty fixtures each timed at 4-5s
        add up to well over the twenty seconds the sweep actually took, and a cost line that
        reported 311s for a 130s check would send the next person tuning the wrong thing. The sum
        is still what ranks the slowest three, which is what it was always for."""
        if not self.ran:
            return "nothing ran"
        wall = time.monotonic() - self._begun
        worst = sorted(self.ran, key=lambda x: -x[1])[:3]
        slow = ", ".join(f"{label} {s:.0f}s" for label, s in worst if s >= 1)
        return f"{wall:.0f}s over {len(self.ran)} check(s)" + (f"; slowest: {slow}" if slow else "")


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
    say(line, driver=True)


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


# ------------------------------------------------------------------------- the usage wall
#
# A session does not FAIL when the account's usage window closes. It stops, mid-slice, and exits
# non-zero -- which is indistinguishable from a crashed CLI, so the driver counted it against
# `--max-retries`, backed off 60s and then 120s, and ended the run. Three sessions and about three
# minutes to turn a five-hour window into an idle night. Nothing landed was ever lost (every
# session commits its own slices, and the ledger, the goal memo and the machine profile are all on
# disk), but the run had to be noticed and restarted by hand -- which unattended is the whole point
# of not happening.
#
# The signal is exact, and it is already in every session log here. `--output-format stream-json`
# carries a `rate_limit_event` on stdout, emitted unconditionally in print mode and re-emitted
# whenever the status changes, so a window that closes MID-session arrives live rather than only at
# connect:
#
#     {"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1787868600,
#      "rateLimitType":"five_hour","overageStatus":"rejected",...},"uuid":...,"session_id":...}
#
# `status` is `allowed`, `allowed_warning` or `rejected`; `resetsAt` is epoch seconds and says
# exactly when to come back, so the recovery is a sleep and not a guess.
#
# **Parse it, never grep it.** The event above is a healthy one -- and it already contains the
# string `"rejected"`, under `overageStatus`, which is a different question about a different
# account setting. A substring test for a refusal matches every session on this machine.

#: A wall this driver will not sit through: eight in a row is past any unattended run, and the
#: guard exists so a permanently refused account cannot loop forever at zero sessions served.
MAX_WALLS = 8

#: How long to come back in when the CLI reported a limit in text but no event carried a deadline.
#: Short on purpose: the next session's own event carries the real one, so a guess only has to be
#: cheap and roughly right.
BLIND_WAIT = 1800

#: The fallback, for the day the event changes shape or never arrives. Matched ONLY against the
#: single terminal `result` event of a session that exited non-zero -- never against tool results,
#: where a session that merely read this file would supply the words itself.
LIMIT_TEXT = re.compile(r"usage limit reached|rate_limit_error", re.I)

#: Windows measured in days rather than hours. Reaching one is not something to sleep through.
LONG_WINDOWS = {"seven_day", "seven_day_opus", "seven_day_sonnet"}


class RateLimit:
    """One `rate_limit_event`: the last thing a session said about the account's standing.

    `blocked` is the only verdict the driver acts on, and it deliberately wants both halves -- a
    `rejected` status AND a reset still in the future. The CLI leaves the last observed value
    standing rather than clearing it when a window turns over, so a rejection whose `resetsAt` has
    passed is stale, and parking a run behind a wall that is no longer there is the one failure
    mode worse than the one this replaces."""

    __slots__ = ("status", "resets_at", "kind", "utilization")

    def __init__(self, info):
        info = info if isinstance(info, dict) else {}
        self.status = str(info.get("status") or "")
        self.kind = str(info.get("rateLimitType") or "")
        self.utilization = info.get("utilization")
        try:
            self.resets_at = int(info.get("resetsAt") or 0)
        except (TypeError, ValueError):
            self.resets_at = 0

    @property
    def blocked(self):
        return self.status == "rejected" and self.left() > 0

    @property
    def long(self):
        return self.kind in LONG_WINDOWS

    def left(self):
        return self.resets_at - time.time() if self.resets_at else 0

    def when(self):
        return f"{datetime.fromtimestamp(self.resets_at):%Y-%m-%d %H:%M}" if self.resets_at else "?"

    def describe(self):
        state = {"rejected": "is reached", "allowed_warning": "is close"}.get(self.status, "is fine")
        pct = self.utilization
        used = f", {pct * 100:.0f}% used" if isinstance(pct, (int, float)) and 0 <= pct <= 1 else ""
        return (f"the {(self.kind or 'usage').replace('_', '-')} usage window {state}{used}; "
                f"it resets at {self.when()}, {hms(self.left())} from now")


def read_limit(line):
    """One NDJSON line, as a `RateLimit` if that is what it is."""
    try:
        e = json.loads(line)
    except json.JSONDecodeError:
        return None
    return RateLimit(e.get("rate_limit_info")) if e.get("type") == "rate_limit_event" else None


def remember_limit(limit):
    """Leave the deadline on disk, so a driver killed or rebooted during a wall does not walk
    straight back into it on the next start. Best effort: an unwritable `.loop/` costs the next
    run one refused session, not the run."""
    try:
        LIMIT.parent.mkdir(exist_ok=True)
        LIMIT.write_text(
            json.dumps({"resets_at": limit.resets_at, "kind": limit.kind, "status": limit.status,
                        "noted": f"{datetime.now():%Y-%m-%d %H:%M:%S}"}, indent=1),
            encoding="utf-8", newline="\n",
        )
    except OSError:
        pass


def standing_limit():
    """The wall a previous driver was still waiting out, or `None` once it has turned over. A
    file that has gone stale is deleted here rather than left to be re-read every start."""
    try:
        entry = json.loads(LIMIT.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    limit = RateLimit({"status": "rejected", "resetsAt": entry.get("resets_at"),
                       "rateLimitType": entry.get("kind")})
    if limit.blocked:
        return limit
    LIMIT.unlink(missing_ok=True)
    return None


def wait_out_limit(limit, opts):
    """Sleep until the window reopens. Returns "" when the run may go on, or the reason it stops.

    A minute of margin: `resetsAt` is the server's second and the clock here is not it, and a
    session launched a moment early costs a whole session to learn that it was.
    """
    left = limit.left() + 60
    if left <= 0:
        LIMIT.unlink(missing_ok=True)
        return ""
    if left > opts.max_limit_wait:
        return (f"{limit.describe()} -- further out than --max-limit-wait "
                f"({hms(opts.max_limit_wait)}), so the run stops here rather than sleeping "
                f"through it. Nothing is lost: every session committed its own slices, and a "
                f"restart after {limit.when()} picks up from the handoff.")
    remember_limit(limit)
    ledger(f"       usage wall: {limit.describe()}; waiting {hms(left)}")
    TICKER.set(phase="waiting out the usage limit", detail=limit.describe())
    # A request that arrived before this wall existed is not a request about it -- `r` is offered
    # only while one is up, and the file follows the key rather than outliving it.
    RETRY.unlink(missing_ok=True)
    step(f"press r to retry now -- after switching accounts, say -- or s to stop the run. From "
         f"another terminal: create {rel_to_root(RETRY)} or {rel_to_root(STOP)}", C.CYAN)
    # In slices, and every one of them interruptible: a wall is hours long, which is exactly when
    # somebody decides to take the tree back or to carry on under a different account. Both
    # controls therefore have to work while the run is parked, not only between sessions.
    CONTROL.parked = True
    try:
        while left > 0:
            stop = CONTROL.stop_reason()
            if stop:
                return f"{stop}, while waiting out the usage limit"
            if CONTROL.take_retry():
                step("retrying now at your request -- the wall is dropped, and the session it "
                     "refused runs next", C.GREEN)
                LIMIT.unlink(missing_ok=True)
                return ""
            wait(min(30, left), f"usage window reopens {limit.when()}", until=CONTROL.pending)
            left = limit.left() + 60
    finally:
        CONTROL.parked = False
    LIMIT.unlink(missing_ok=True)
    step(f"the usage window has reopened -- {limit.when()} has passed", C.GREEN)
    return ""


def mark_interrupted(index, limit):
    """Record that a session was cut off with work still in the tree. Returns the path count.

    A session stopped mid-slice has committed everything it FINISHED -- one commit per slice is
    what buys that -- but whatever it was in the middle of is still uncommitted, and the handoff
    it never reached does not mention it. Without this the next session finds those files and has
    no way to tell them from the state it was supposed to start in. `orient.py` reads this file
    and says so at the top of the pack; the next session to leave a clean tree deletes it."""
    dirty = [ln for ln in git("status", "--porcelain").split("\n") if ln.strip()]
    if not dirty:
        INTERRUPTED.unlink(missing_ok=True)
        return 0
    try:
        INTERRUPTED.write_text(
            json.dumps({"session": index, "when": f"{datetime.now():%Y-%m-%d %H:%M:%S}",
                        "why": limit.describe() if limit else "the CLI exited non-zero",
                        "head": git("rev-parse", "HEAD"), "files": dirty}, indent=1),
            encoding="utf-8", newline="\n",
        )
    except OSError:
        pass
    return len(dirty)


def run_session(run_id, index, prompt_text, opts, renderer):
    """One `claude -p` session, its NDJSON streamed to the console and to
    .loop/logs/<run>-NNNN.log. The run stamp is in the name because the index restarts at 1
    every run: named by index alone, session 3 of today's run appended to session 3 of last
    week's, and any per-session measurement over the directory silently mixed the two.

    The session id off the `system`/`init` event is kept, not just printed: it is the only
    handle on the harness's own transcript directory, and therefore on any subagent this
    session spawned. Without it a delegated read is invisible to every measurement below.

    The file is opened through `CONSOLE` rather than here, and stays open after this returns: the
    acceptance check that judges this session runs next, and its lines belong in this session's
    log. `drive()` closes it once that verdict is in."""
    log = LOGDIR / f"{run_id}-{index:04d}.log"
    CONSOLE.open_session(log)
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
    # `orient.py` runs a `brief.py`, a `git log` and a plan read before the child is even
    # spawned, and on a cold filesystem cache that is tens of seconds between the "== session"
    # banner and the first token. It is the second half of the gap between two sessions.
    step("building the orientation pack (tools/orient.py)")
    TICKER.set(phase="orienting", detail="tools/orient.py")
    started = time.monotonic()
    pack = orientation_pack()
    spent = mmss(time.monotonic() - started)
    if pack:
        step(f"orientation pack: {len(pack.encode('utf-8')):,} bytes in {spent}")
    else:
        step(f"orientation pack: orient.py failed after {spent} -- "
             "the session will run it itself", C.YELLOW)
    session_id = ""
    limit = None  # the last `rate_limit_event` this session reported; see `RateLimit`
    said_limit = False  # the text fallback, read only off a non-zero exit's `result` event
    # The pack's size, recorded beside the transcript that paid for it. Two sessions with
    # different pack sizes are a two-point regression against their measured `ctx_start`,
    # which is how `loop-stats.py --calibrate` derives bytes-per-token instead of assuming
    # it. Nothing downstream needs this line; every reader skips a `type` it does not know.
    CONSOLE.raw(json.dumps({"type": "loop_pack", "bytes": len(pack.encode("utf-8"))}) + "\n")
    step(f"launching {exe} (--model {opts.model}, --permission-mode {opts.permission_mode})")
    TICKER.set(phase="launching", detail=f"{exe} --model {opts.model}")
    launched = time.monotonic()
    proc = subprocess.Popen(
        cmd,
        cwd=ROOT,
        # Always a pipe, pack or no pack. Inheriting this driver's stdin would hand the console to
        # the child, and the console is where `r` and `s` are typed -- a session started without a
        # pack would silently eat them. Closed immediately when there is nothing to send: the
        # prompt is on argv, so the child never wanted a stdin of its own in the first place.
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=None,
        encoding="utf-8",
        errors="replace",
        bufsize=1,
    )
    if pack:
        threading.Thread(target=feed, args=(proc.stdin, pack), daemon=True).start()
    elif proc.stdin:
        try:
            proc.stdin.close()
        except OSError:
            pass
    assert proc.stdout is not None
    try:
        for line in proc.stdout:
            if launched:
                # The CLI's own start-up -- config, MCP servers, the model handshake -- is
                # dead air on the console, and it is charged to whatever ran just before it.
                step(f"claude answered after {mmss(time.monotonic() - launched)}")
                TICKER.set(phase="working")
                launched = 0
            CONSOLE.raw(line)
            if '"rate_limit_event"' in line:
                fresh = read_limit(line)
                if fresh:
                    # Said once per change, not once per event: the status is re-sent whenever
                    # anything about it moves, and a healthy account sends one at connect.
                    if fresh.status != "allowed" and (not limit or fresh.status != limit.status):
                        step(f"the account reports: {fresh.describe()}", C.YELLOW)
                    limit = fresh
            elif '"type":"result"' in line and LIMIT_TEXT.search(line):
                # The terminal event only. `"type":"tool_result"` does not match this, which is
                # the point: a session that read this very file would otherwise supply the words.
                said_limit = True
            if not session_id and '"session_id"' in line:
                try:
                    e = json.loads(line)
                    if e.get("type") == "system" and e.get("subtype") == "init":
                        session_id = str(e.get("session_id") or "")
                except json.JSONDecodeError:
                    pass
            renderer.event(line)
        # Its stdout is closed but the process is not necessarily gone -- flushing its
        # transcript, tearing down MCP servers. Named, because it is time the console
        # would otherwise attribute to the driver's own work.
        closed = time.monotonic()
        TICKER.set(phase="closing the session", detail="flushing the transcript")
        proc.wait()
        if time.monotonic() - closed >= 1:
            step(f"claude took {mmss(time.monotonic() - closed)} to exit after its last event")
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
    if proc.returncode and said_limit and not (limit and limit.blocked):
        # A limit reported in prose, with no event carrying a deadline. Come back shortly rather
        # than ending the run: the next session's own event will carry the real one.
        step(f"a usage limit was reported in text but no event named a reset -- treating it as a "
             f"wall and coming back in {hms(BLIND_WAIT)}", C.YELLOW)
        limit = RateLimit({"status": "rejected", "resetsAt": int(time.time()) + BLIND_WAIT})
    return proc.returncode, log, session_id, limit


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
    """A thin wrapper so the status line is torn down on every exit path -- a normal return, a
    Ctrl-C, or an exception -- rather than leaving a half-painted bottom row on the console."""
    try:
        return run_cli()
    finally:
        TICKER.stop()
        CONTROL.disable()
        CONSOLE.close()


def run_cli():
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
    ap.add_argument(
        "--max-limit-wait", type=float, default=6 * 3600, metavar="SECONDS",
        help="sleep through a usage limit that reopens within this long, and end the run when it "
             "does not (default 6h: a five-hour window fits inside it, a weekly one does not)"
    )
    ap.add_argument("--delay-seconds", type=int, default=0)
    ap.add_argument("--max-result-lines", type=int, default=60)
    ap.add_argument("--max-input-lines", type=int, default=40)
    ap.add_argument("--max-line-chars", type=int, default=500)
    ap.add_argument("--full-output", action="store_true", help="no truncation anywhere")
    ap.add_argument(
        "--no-status", dest="status", action="store_false",
        help="do not paint the live status line (it is off by itself when stdout is not a terminal)"
    )
    ap.add_argument("--goal-only", action="store_true", help="run the acceptance test and exit")
    ap.add_argument(
        "--leg-only", action="store_true",
        help="run just the Linux leg -- every fixture, both suites and the valgrind sweep against a "
             "Linux build -- and exit"
    )
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
            if c["kind"] == "command":
                memo = "  (memoized against the tree)" if c.get("memoize") else ""
                say(f"  [{c.get('stage', '?')}] {c['kind']:<11} {c['name']}: "
                    f"{' '.join(c['argv'])} in {c.get('cwd', '.')}{memo}")
                continue
            driver = "mwl" if c["kind"] == "mwl-suite" else "cargo"
            say(f"  [{c.get('stage', '?')}] {c['kind']:<11} {c['name']}: "
                f"{driver} {' '.join(c['args'])}")
            # Named cases are the half of a suite check that a session can act on: each is one
            # item's artefact, and the ones not yet on disk are the worklist.
            cases = c.get("cases", [])
            if cases:
                absent = [p for p in cases if not (ROOT / p).exists()]
                say(f"      {len(cases)} named case(s), {len(absent)} not written yet", C.GRAY)
                for path in absent:
                    say(f"        - {path}", C.GRAY)
        skipped = ", ".join(sorted(goal.valgrind_skip)) or "nothing"
        say(f"  valgrind sweep over every fixture except: {skipped}")
        return 0

    if opts.status:
        TICKER.start()

    if opts.goal_only or opts.leg_only:
        # By hand is exactly when the full output is wanted: `--goal-only` is what you run to
        # find out why a check is red, and the console still prints only its first line.
        LOGDIR.mkdir(parents=True, exist_ok=True)
        path = LOGDIR / f"{datetime.now():%Y%m%d-%H%M%S}-console.log"
        CONSOLE.open_run(path)
        say(f"console log: {rel_to_root(path)}", C.GRAY)

    if opts.goal_only:
        say("running the acceptance test ...", C.CYAN)
        fail = goal.check(verbose=True)
        say(f"\ncost: {goal.summary()}", C.GRAY)
        if fail:
            say(f"NOT GREEN: {fail}", C.RED)
            return 1
        say("GOAL REACHED: every acceptance check passes", C.GREEN)
        return 0

    if opts.leg_only:
        say("running the Linux leg ...", C.CYAN)
        fail = goal.leg_check(verbose=True)
        say(f"\ncost: {goal.summary()}", C.GRAY)
        if fail:
            say(f"NOT GREEN: {fail}", C.RED)
            return 1
        say("Linux leg: everything passes", C.GREEN)
        return 0

    LOGDIR.mkdir(parents=True, exist_ok=True)
    if not LEDGER.exists():
        LEDGER.write_text("# Loop ledger\n", encoding="utf-8", newline="\n")

    TICKER.set(phase="making room", detail="pruning earlier runs' logs and scratch")
    if not make_room(opts):
        return 2
    if not claim_run(opts):
        return 2
    CONTROL.enable()
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
    CONSOLE.open_run(LOGDIR / f"{run_id}-console.log")
    ledger("")
    ledger(f"## run started {datetime.now():%Y-%m-%d %H:%M} (max {opts.max_sessions}, logs {run_id}-*)")
    say(f"console log: {rel_to_root(LOGDIR / f'{run_id}-console.log')}", C.GRAY, driver=True)
    # The key row under the status line says this continuously and says it in context, so it is
    # only worth a line when that row is not there -- which is a redirected STDOUT, not a missing
    # stdin. The two are independent: keys are readable whenever stdin is a console, and the row
    # is painted only when stdout is one, so `nohup` gets the files and `loop.py > log` gets both.
    if not (CONTROL.tty and TICKER.enabled):
        say("controls: " + ("press r to end a usage wait early, s to stop after the current "
                            "session" if CONTROL.tty else
                            f"create {rel_to_root(RETRY)} to end a usage wait early, "
                            f"{rel_to_root(STOP)} to stop after the current session"),
            C.GRAY, driver=True)

    # Every acceptance check builds the debug CLI, so from the second session on it is current at
    # the tree the next session starts from -- and orient.py's closing block tells the session so,
    # to stop it spending a call on `ls -la target/debug/mwl.exe` and a defensive `cargo build`
    # (0.6 calls a session, measured). The first session of a run is the one case that promise
    # would be false, because no check has run in front of it yet. So it runs here. It is a no-op
    # against a warm target/ and it is not fatal: a red build is a thing a session may be sent to
    # fix, and the goal check reports it either way.
    warm = NativeLeg().prepare()
    if warm:
        say(f"the debug CLI is not built: {warm}", C.YELLOW, driver=True)

    # Two counters where there was one. `index` names the logs and only ever goes up, so a session
    # the wall cut short never has its transcript overwritten by the one that replaces it; `served`
    # is what `--max-sessions` counts, and a session the account refused is not one of them.
    index = 0
    served = 0
    walls = 0
    wall = standing_limit()  # left standing by a driver killed or rebooted during one
    if wall:
        step(f"{rel_to_root(LIMIT)} says {wall.describe()}", C.YELLOW)

    while served < opts.max_sessions:
        asked = CONTROL.stop_reason()
        if asked:
            reason = asked
            break

        if wall:
            # Neither a failure nor a stall: no retry can help, nothing is wrong with the tree, and
            # the account has already said when it will answer again. Sleep until then, then run
            # the session it refused.
            stop = wait_out_limit(wall, opts)
            if stop:
                reason = stop
                break
            wall = None

        index += 1
        head_before = git("rev-parse", "HEAD")
        STATUS.unlink(missing_ok=True)
        TICKER.set(scope=f"session {served + 1}/{opts.max_sessions}", phase="starting")
        say(f"== session {served + 1}/{opts.max_sessions}  {datetime.now():%H:%M:%S}", C.CYAN)

        # The prompt is re-read for the same reason `load_goal()` is called below: a session that
        # improved it should be improving the next session, not the next run. A read that fails
        # keeps the last good text rather than ending a run nobody is watching.
        try:
            prompt_text = PROMPT.read_text(encoding="utf-8")
        except OSError as e:
            say(f"   {rel_to_root(PROMPT)} did not read, using the last good one -- {e}", C.YELLOW)

        session_started = time.monotonic()
        cli_exit, log, session_id, limit = run_session(run_id, index, prompt_text, opts, renderer)
        step(f"session {index} ended after {mmss(time.monotonic() - session_started)}, "
             f"claude exit {cli_exit}", C.CYAN)

        # The wall is judged before the exit code, because it EXPLAINS the exit code. A refused
        # session exits non-zero exactly like a crashed one, and counting it as a crash is what
        # used to end a run three minutes into a five-hour window.
        if limit and limit.blocked:
            walls += 1
            open_paths = mark_interrupted(index, limit)
            ledger(f"- {index:04d} refused by the usage wall -- {limit.describe()}"
                   + (f"; {open_paths} path(s) left uncommitted" if open_paths
                      else "; the tree is clean")
                   + f" -- see {log.relative_to(ROOT).as_posix()}")
            if walls >= MAX_WALLS:
                reason = f"{walls} sessions in a row were refused by the usage limit"
                break
            wall = limit
            continue

        if cli_exit != 0:
            fails += 1
            mark_interrupted(index, None)
            ledger(
                f"- {index:04d} CLI exit {cli_exit} (attempt {fails}/{opts.max_retries}) -- "
                f"see {log.relative_to(ROOT).as_posix()}"
            )
            if fails >= opts.max_retries:
                reason = f"claude CLI failed {fails} times in a row"
                break
            backoff = min(300, 30 * 2**fails)
            step(f"backing off {mmss(backoff)} before retry {fails + 1}", C.YELLOW)
            TICKER.set(phase=f"backing off before retry {fails + 1}")
            wait(backoff, "claude exited non-zero")
            continue
        fails = 0
        walls = 0
        served += 1

        line = STATUS.read_text(encoding="utf-8").strip() if STATUS.exists() else ""
        head_after = git("rev-parse", "HEAD")
        commits = 0
        if head_after and head_after != head_before:
            commits = int(git("rev-list", "--count", f"{head_before}..{head_after}") or 0)
        # A session that finished and left nothing behind closes any earlier interruption.
        if not git("status", "--porcelain").strip():
            INTERRUPTED.unlink(missing_ok=True)
        step("collecting subagent transcripts")
        TICKER.set(phase="collecting subagent transcripts")
        started = time.monotonic()
        agents, agent_calls = collect_subagents(session_id, run_id, index)
        spent = time.monotonic() - started
        if spent >= 1:
            step(f"subagent transcripts took {mmss(spent)}")
        delegated = f" | {agents} subagent(s), {agent_calls} call(s)" if agents else ""
        ledger(f"- {index:04d} {commits} commit(s){delegated} | {line or '(no status written)'}")

        # The deterministic goal check outranks whatever the session reported -- against the list
        # as the session left it, which is why this is re-read rather than held from start-up. A
        # `loop-goal.toml` that does not parse keeps the last good spec: a run of 300 sessions must
        # not end on one session's typo, and the ledger names it loudly instead.
        try:
            goal = load_goal()
        except (OSError, tomllib.TOMLDecodeError) as e:
            ledger(f"       goal spec: {rel_to_root(GOAL_TOML)} did not parse -- "
                   f"checking against the last good one. {e}")
        # Verbose on purpose, and the one place a run spends minutes without a session running:
        # a native build, every fixture, both suites, the WSL leg and the valgrind sweep. Silent,
        # this read as a driver that had hung after printing the session's status line.
        step("acceptance check: build, fixtures, suites, wsl leg, valgrind", C.CYAN)
        checked = time.monotonic()
        fail = goal.check(verbose=True)
        step(f"acceptance check done in {mmss(time.monotonic() - checked)}", C.CYAN)
        ledger(f"       goal cost: {goal.summary()}")
        # The verdict on session `i` is the last thing that belongs in session `i`'s log.
        CONSOLE.close_session()
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
            step(f"--delay-seconds: waiting {mmss(opts.delay_seconds)} before the next session")
            TICKER.set(phase="waiting")
            wait(opts.delay_seconds, "--delay-seconds")

    ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- {reason}")
    say("")
    say(reason, C.YELLOW)


if __name__ == "__main__":
    sys.exit(main())
