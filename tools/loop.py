#!/usr/bin/env python3
"""Unattended Novis work loop. Starts one fresh `claude` session per iteration, so the per-session
context cost is constant no matter how many sessions run. See docs/agent/coordinator.md for the design
and docs/agent/loop-goal.md for the goal; the acceptance list itself is docs/agent/loop-goal.toml.

Runs on Windows, Linux and macOS. On Windows the acceptance test gets a second leg through WSL, because
a JIT is exactly where a calling-convention divergence between two targets hides; on Linux the native leg
already is that target, so there is one leg plus the valgrind sweep.

    python tools/loop.py --max-sessions 300 --effort medium
    python tools/loop.py --max-sessions 300 --full-output   # no truncation anywhere
    python tools/loop.py --goal-only                        # run the acceptance test and exit
    python tools/loop.py --leg-only                         # just the Linux leg, and exit

Nothing on the stop path depends on a model's self-assessment: every acceptance item is an exit code plus
an exact or ordered-substring match on real output.
"""

from __future__ import annotations

import _thread
import argparse
import contextlib
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
CHAINSTATE = RUNDIR / "chain.json"
RUNEND = RUNDIR / "run-end.json"
DOCGATE = RUNDIR / "doc-gate.json"
RELEASEGATE = RUNDIR / "release-gate.json"
LASTFAIL = RUNDIR / "last-fail.json"

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
        detail   the current item       `wsl fixtures/closures.nvs`, `Edit tools/loop.py`
        elapsed  how long this phase has been going

    Above them, on its own row, **what the work is for** -- the moving half first: what the
    running session has landed so far (`SliceWatch` -- `2 commits · <the last one's subject>`,
    and until the first one lands, the item `orient.py` handed it), and then the loop goal the
    run is driving toward -- the title of `docs/agent/loop-goal.md`, behind `goal 2/6` when a
    chain is driving (`goal_title`). Neither is something the scrolling output says: the tool
    calls name files, the pack scrolled off minutes ago, and these two are the only lines that
    say what all of it is *for*.

    The session half leads because it is the only half that changes. The loop goal's title is one
    string for a whole run, and while it was in front it was also all a glance ever saw: the row
    overflows a terminal, and the window below takes some thirteen seconds to slide far enough to
    reach what came after it.

    The **window title** carries the same three fields, for the same reason -- see `title()`.

    The row is usually longer than a terminal, so it **scrolls**: the ticker slides a window over
    it a character at a time, holds at each end, and comes back -- a bounce rather than a wrap,
    so the start and the end are both read in full and nothing is ever cut mid-word for good.

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
        self.loop_goal = ""  # the run's goal, from `goal_title`
        self.goal = ""  # what the running session has landed, from `SliceWatch`
        self.scope = ""
        self.phase = ""
        self.detail = ""
        self.done = 0
        self.total = 0
        self.calls = 0
        self.verifying = ""  # `verify test 3/7 12s` while verify.py runs; see `VerifyWatch`
        self.since = time.monotonic()
        self._frame = 0
        self._rows = 0  # rows the live block owns right now; 0 when it is not on screen
        self._title = ""  # the last window title written; see `title_seq`
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
        self._title = ""  # whatever the window says now, this run did not put it there
        self.since = time.monotonic()
        self._thread = threading.Thread(target=self._tick, daemon=True)
        self._thread.start()

    def stop(self):
        """Erase the line and leave the cursor where the next print expects it -- and give the
        window title back, since the run it described is over."""
        self._stop.set()
        with self.lock:
            self.erase()
            if self.enabled:
                # An empty title is not a blank one: a terminal falls back to whatever it would
                # have shown had nothing ever set it -- the shell's own title, or the profile's.
                sys.stdout.write("\033]0;\007")
                sys.stdout.flush()
                self._title = ""
            self.enabled = False

    def _tick(self):
        while not self._stop.wait(self.INTERVAL):
            # Outside the lock, and it has to stay outside: `poll` says things, and saying
            # anything takes this lock. Every path in this file therefore takes CONTROL's lock
            # before the ticker's, which is the whole of the argument that neither waits on the
            # other.
            CONTROL.poll()
            VERIFY.poll()
            with self.lock:
                self._frame += 1
                self.draw()

    # -- what it says ------------------------------------------------------------------

    def set(self, phase=None, detail=None, scope=None, total=None, done=None, calls=None,
            goal=None, loop_goal=None):
        """Update any subset of the fields. A new `phase` restarts the elapsed clock and clears
        the detail and the counters, because those belonged to the phase that just ended."""
        with self.lock:
            if loop_goal is not None:
                self.loop_goal = loop_goal
            if goal is not None:
                self.goal = goal
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

    def verify(self, text):
        """What `verify.py` is doing right now, or "" when it is not running. Painted by the
        next tick rather than here: the ticker is the only caller, and it draws right after."""
        with self.lock:
            self.verifying = text

    # -- painting ----------------------------------------------------------------------

    def compose(self):
        head = self.phase
        if self.total > 0:
            done = min(self.done, self.total)
            head = f"{head} {done}/{self.total} {done * 100 // self.total}%".lstrip()
        parts = [p for p in (self.scope, head, self.detail, self.verifying) if p]
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

    # The scroll's tempo, in ticker frames of `INTERVAL`: one character every `SCROLL_STEP`
    # frames, and `SCROLL_HOLD` frames of rest at either end before it turns around.
    SCROLL_STEP = 2
    SCROLL_HOLD = 16

    def goal_row(self):
        """The row between the rule and the status line: what the session has landed, then the
        loop goal it is landing it for.

        Grey and indented like the key row, so the one white line in the block is still the
        status. Never truncated: a row longer than the terminal is shown through a window that
        the ticker slides along it, `scroll_offset` says how far. Cut short the way `compose()`
        cuts, the loop goal's title would eat the whole width and the session half -- the one
        that changes -- would never be seen at all, which is also why it is no longer second."""
        parts = [self.goal or "(no session has been oriented yet)", self.loop_goal]
        body = self.sep.join(p for p in parts if p).replace("\n", " ")
        room = max(18, self.width() - 3)
        if len(body) > room:
            start = self.scroll_offset(len(body) - room)
            body = body[start: start + room]
        return "  " + C.paint(body, C.GRAY)

    def scroll_offset(self, overflow):
        """Where the window over an overflowing row starts on this frame: 0 for `SCROLL_HOLD`
        frames, then one character further every `SCROLL_STEP` frames until the end is in view,
        a hold there, and the same walk back. A bounce, not a wrap -- text that loops around
        never shows either end for long, and the ends are the two things worth reading."""
        walk = overflow * self.SCROLL_STEP
        cycle = 2 * (walk + self.SCROLL_HOLD)
        t = self._frame % cycle
        if t < self.SCROLL_HOLD:
            return 0
        t -= self.SCROLL_HOLD
        if t < walk:
            return t // self.SCROLL_STEP
        t -= walk
        if t < self.SCROLL_HOLD:
            return overflow
        return overflow - (t - self.SCROLL_HOLD) // self.SCROLL_STEP

    def keys(self):
        """The row under the status line: what a keypress would do *right now*.

        Contextual on purpose, and that is the whole design. `[r]` appears only while the run is
        parked -- behind a usage wall or an API overload -- because a wait is the only thing it
        ends, and offered at any other time it is a key that silently does nothing. `[s]` is
        always there, and once armed the row says so and says
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

    # -- the terminal's own title bar --------------------------------------------------

    #: How much of a title a taskbar button or a tab will actually show is anyone's guess, so the
    #: fields are ordered widest-context-first and the tail is what gets cut.
    TITLE_ROOM = 120

    def title(self):
        """The status line, squeezed into the window title: where the run is, what it is doing,
        and what the session has landed so far.

        A terminal running a five-hour loop is usually not the window in front, and a title is
        what a taskbar button, a tab and an alt-tab preview all show -- the only place this run
        reports anything at all when it is behind something else.

        It takes the three fields that will still be true in a minute and leaves out the two that
        will not: the per-tool `detail`, and the elapsed clock. Both change several times a
        second, and a title that flickers that fast is read as noise by a person and as a new
        window by some taskbars -- while saying nothing a look at the terminal would not say
        better."""
        head = self.phase
        if self.total > 0:
            done = min(self.done, self.total)
            head = f"{head} {done}/{self.total} {done * 100 // self.total}%".lstrip()
        body = self.sep.join(p for p in (self.scope, head, self.goal) if p)
        # A title is a control sequence's payload, and it ends at the first BEL or ESC: a commit
        # subject with a control character in it would otherwise leave the rest of this line
        # being interpreted by the terminal rather than shown by it.
        body = re.sub(r"[\x00-\x1f\x7f]", " ", body)
        if len(body) > self.TITLE_ROOM:
            body = body[: self.TITLE_ROOM - len(self.cut)] + self.cut
        return f"nvs loop{self.sep}{body}" if body else "nvs loop"

    def title_seq(self):
        """The escape sequence `draw` prepends to a repaint, and "" whenever the title already
        says this. Rewriting it on every frame would put eight identical sequences a second into
        the console log and into any recording of this terminal, to no effect on screen.

        OSC 0 rather than 2: it sets the icon name and the window title together, which is what
        conhost, Windows Terminal and every xterm-alike agree on, and it is BEL-terminated
        because conhost takes only that form."""
        text = self.title()
        if text == self._title:
            return ""
        self._title = text
        return f"\033]0;{text}\007"

    def rows(self):
        """How tall the live block is: the rule, the goal row and the status line always, the key
        row only when there is a console to type at. Both `draw` and `erase` read this, and a
        height that changes mid-run is handled by giving the old block back before claiming the
        new one."""
        return 4 if CONTROL.tty else 3

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
        out = self.title_seq()  # invisible, and only when it changed
        out += "\n" * (want - 1) if not self._rows else ""  # claim the rows under this one, once
        out += f"\033[{want - 1}A\r\033[2K" + self.divider()  # up to the rule
        for line in [self.goal_row(), self.compose()] + ([self.keys()] if want > 3 else []):
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

    * **`r` exists only while a wall is up** -- a usage window that has closed, or an API that
      answered `529 Overloaded`. It has nothing to end at any other time, and a key that silently
      does nothing is a key that gets pressed twice and then distrusted.
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
                        say("   [r] does nothing right now; it ends a usage or overload wait",
                            C.GRAY)

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


class VerifyWatch:
    """What `tools/verify.py` is doing inside a session's tool call, on the status line.

    The harness hands the driver a tool call's output only when the call returns, so a session's
    verification -- one call, a minute and a half at the end of every session -- was a spinner
    behind `Bash python tools/verify.py` with nothing moving for the whole of it. `verify.py`
    therefore writes `.agent-tmp/verify-progress.json` at every step boundary (its module doc is
    the format's home), and this reads it: the step in flight and its clock go on the status line
    behind the tool call, and each boundary gets one grey line in the scrollback and the session
    log, so the timeline is still there once the call has returned. A `--start` run shows the
    same way while the session writes its wrap file beside it.

    Armed for the life of a session and nothing else. A file older than the arming is a previous
    session's -- `verify.py` rewrites it `finished` at the end, but a run killed mid-step leaves
    its last entry behind forever -- and so is a step "in flight" for longer than `STALE`, which
    is `verify.py`'s own `--wait` timeout."""

    FILE = ROOT / ".agent-tmp" / "verify-progress.json"
    EVERY = 0.5  # seconds between reads; the ticker fires eight times a second
    STALE = 600.0

    def __init__(self):
        self.armed = 0.0  # wall clock of the arming; 0 while no session is running
        self.seen = None  # (index, step) last announced, so a boundary is said once
        self._next = 0.0

    def arm(self):
        self.armed = time.time()
        self.seen = None
        self._next = 0.0
        TICKER.verify("")

    def disarm(self):
        self.armed = 0.0
        TICKER.verify("")

    def poll(self):
        """Called by the ticker, outside its lock -- this says things."""
        if not self.armed:
            return
        now = time.monotonic()
        if now < self._next:
            return
        self._next = now + self.EVERY
        entry = self.read()
        if entry is None or "finished" in entry or "step" not in entry:
            TICKER.verify("")
            return
        key = (entry.get("index"), entry.get("step"))
        if key != self.seen:
            self.seen = key
            done = entry.get("done") or []
            before = f"{done[-1]['name']} ok {mmss(done[-1]['seconds'])} -> " if done else ""
            say(f"     ~ verify: {before}{entry['step']} ({entry['index']}/{entry['total']})",
                C.GRAY)
        TICKER.verify(f"verify {entry['step']} {entry['index']}/{entry['total']} "
                      f"{mmss(time.time() - float(entry.get('at') or 0))}")

    def read(self):
        try:
            if self.FILE.stat().st_mtime < self.armed:
                return None
            entry = json.loads(self.FILE.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return None
        if not isinstance(entry, dict):
            return None
        if time.time() - float(entry.get("at") or 0) > self.STALE:
            return None
        return entry


TICKER = StatusLine()
CONTROL = Control()
VERIFY = VerifyWatch()


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
                    # Two small file reads, on the one thread that already knows the session did
                    # something. A slice committed since the last call moves the goal row.
                    SLICES.poll()
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


def capture(exe, args, timeout=1800, cwd=None, env=None, log_stdout=True):
    """Run a program with stdout and stderr captured SEPARATELY -- the acceptance list distinguishes
    them (a backtrace and FATAL go to stderr, program output to stdout).

    `env` replaces the environment whole when given -- `Goal.crate_tests` runs a test executable
    where cargo would, with cargo's variables. `log_stdout=False` keeps stdout out of the console
    log and is for exactly one caller: a `--message-format=json` build, whose stdout is a line per
    artifact for every crate in the dependency graph and nothing a person reads back.

    `cwd` defaults to the repository root, which is what every cargo and `nvs` invocation wants. A
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
            env=env,
        )
    except subprocess.TimeoutExpired:
        r = Result(-1, "", f"timed out after {timeout}s")
    except OSError as exc:
        r = Result(-1, "", str(exc))
    else:
        r = Result(p.returncode, p.stdout or "", p.stderr or "")
    where = "" if cwd in (None, ROOT) else f" (in {cwd})"
    head = f"$ {exe} {' '.join(args)}{where} -> exit {r.code} in {mmss(time.monotonic() - began)}"
    shown = (r.out.rstrip("\n") if log_stdout else "", r.err.rstrip("\n"))
    CONSOLE.block(head, "\n".join(s for s in shown if s.strip()))
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
        r = capture("cargo", ["build", "--quiet", "-p", "nvs-cli"])
        if r.code != 0:
            return f"the {self.name} build failed -- {r.first_err_line}"
        self.binary = str(ROOT / "target" / "debug" / ("nvs.exe" if IS_WINDOWS else "nvs"))
        return ""

    def run(self, nvs_args):
        return capture(self.binary, ["run", *nvs_args])

    def origin_command(self):
        """`tools/origin.py` where this leg's fixtures run -- see `local_origin` below."""
        return [sys.executable, "-u", str(ROOT / "tools" / "origin.py")]

    def suite(self, nvs_args):
        """`nvs test <dir>` on this leg. Separate from `run` because a suite takes its own
        subcommand, and separate from calling `self.binary` directly because on the WSL leg that
        path is a Linux one and Windows cannot execute it."""
        return capture(self.binary, nvs_args)


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
            f"cd {self.repo} && CARGO_TARGET_DIR={self.target_dir} cargo build --quiet -p nvs-cli"
        )
        if r.code != 0:
            return f"the {self.name} build failed -- {r.first_err_line}"
        self.binary = f"{self.target_dir}/debug/nvs"
        return ""

    def run(self, nvs_args):
        return self.bash(f"cd {self.repo} && {self.binary} run " + " ".join(nvs_args))

    def origin_command(self):
        """Inside the distro, because a WSL loopback is its own: a listener bound on the Windows
        side of 127.0.0.1 is not the one `examples/http.nvs` reaches from here. The mirrored-
        networking case, where the two loopbacks *are* the same one, is handled by `origin.py`
        itself -- it leaves an existing listener alone and idles as a lifetime handle."""
        return ["wsl.exe", "--", "bash", "-lc",
                f"cd {self.repo} && exec python3 -u tools/origin.py"]

    def suite(self, nvs_args):
        return self.bash(f"cd {self.repo} && {self.binary} " + " ".join(nvs_args))


@contextlib.contextmanager
def local_origin(leg):
    """`tools/origin.py` on this leg, up for as long as the caller's fixtures run.

    `examples/http.nvs` names `http://127.0.0.1:8099` rather than discovering it, so the harness is
    what has to be serving there -- that decision, and why it is not the example spawning its own,
    is recorded beside the stage 5 check in `docs/agent/loop-goal.toml`. One per leg: the native
    fixtures reach the Windows listener, and the WSL leg's fixtures and valgrind sweep reach one
    inside the distro.

    Readiness is the origin's own line on stdout, not a sleep and not a probe from here -- the WSL
    one cannot be probed from Windows at all. The reader thread stays for the process's life so a
    full pipe buffer can never stall it, and closing stdin is the whole shutdown protocol.

    A leg whose origin does not come up is loud but not fatal: the fixture that needed it then
    fails on its own line with the connection error, which says more than a harness failure would.
    """
    proc = subprocess.Popen(leg.origin_command(), cwd=str(ROOT), stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    ready, up = [], threading.Event()

    def watch():
        for line in proc.stdout:
            if line.startswith("origin:") and not ready:
                ready.append(line.strip())
                up.set()
        up.set()  # it ended without ever saying it -- do not wait the timeout out for nothing

    threading.Thread(target=watch, daemon=True).start()
    up.wait(60)
    if ready:
        say(f"{leg.name} {ready[0]}", C.GRAY)
    else:
        why = (proc.stderr.read() or "").strip() if proc.poll() is not None else "it never answered"
        say(f"the {leg.name} leg has no origin on 8099 -- {why.splitlines()[-1] if why else '?'}",
            C.RED)
    try:
        yield
    finally:
        try:
            proc.stdin.close()
        except OSError:
            pass
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            proc.kill()


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
# `tests/` and `docs/` are deliberately absent: nothing keyed on this runs a `.nvst` case.
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


def plain_crate_test(args):
    """The crate a check's `args` is exactly `cargo test -p <crate>` for, else None."""
    return args[2] if len(args) == 3 and args[0] == "test" and args[1] == "-p" else None


def suite_widening(checks):
    """For every `nvs-suite` directory in the list, the widest directory the SAME list asks about
    that contains it -- `tests/conformance/core/` -> `tests/conformance/`, and nothing for a
    directory no other check encloses.

    A suite over a directory runs every case a suite over a directory inside it would, so the
    narrower run is cases re-run for an answer already on the desk. Measured on the 20260904-143054
    run: `conformance (the digest roster)` is `nvs test tests/conformance/core/` and cost **52s of
    a 288s sweep** re-running 908 of the 1503 cases `conformance (Core by name)` had already run
    over `tests/conformance/`.

    Widening the REQUEST rather than reusing a result after the fact is what makes this
    order-independent: both checks then ask `suite()` the same question, and the memo it has always
    had answers the second one. A rule that reused the wider run only if it happened to have gone
    first would be silently worth nothing the day the stages are reordered.

    Keyed on the argument as the goal file spells it, so two checks naming the same directory the
    same way share a key and one spelled differently simply does not widen. A `min_passing` check
    is never widened -- see `Goal.suite`.
    """
    dirs = [(tuple(c["args"][:-1]), c["args"][-1]) for c in checks
            if c["kind"] == "nvs-suite" and len(c.get("args", [])) >= 2]
    widen = {}
    for head, spelled in dirs:
        inner = spelled.replace("\\", "/").rstrip("/")
        best, best_len = None, len(inner)
        for other_head, other_spelled in dirs:
            outer = other_spelled.replace("\\", "/").rstrip("/")
            if other_head == head and inner.startswith(outer + "/") and len(outer) < best_len:
                best, best_len = other_spelled, len(outer)
        if best is not None:
            widen[(head, spelled)] = best
    return widen


def rustc_version():
    """The exact compiler, so a toolchain bump invalidates every memoized verdict."""
    p = subprocess.run(["rustc", "-vV"], capture_output=True, encoding="utf-8", errors="replace")
    return (p.stdout or "") + (p.stderr or "")

# Held by whatever background release build is in flight; see `Goal.prebuild`.
PREBUILD_LOCK = threading.Lock()

# A `command` check is an argv run in a directory, exit 0, with `want` as ordered substrings across
# both streams. It exists because the acceptance test grew a leg that is neither `nvs run` stdout nor
# `cargo test`: from M4B, `editors/vscode` is TypeScript and its suites are `npm` scripts. Kept
# deliberately general -- a check kind per external tool is how a driver becomes a build system.
#
# It is not a PROGRAM_KIND, so it runs ONCE between the legs rather than per leg. That is the right
# default and not an accident of implementation: the WSL leg exists because a JIT is where a
# calling-convention divergence hides, and a TextMate grammar has no calling convention.


def measures_release_cli(c):
    """Whether this check reads `target/release/nvs.exe`, which nothing else here builds.

    Recognized by the tool rather than by a field on the check, because `tools/bench.py` is the
    only thing in the repository that measures that binary and a `needs = "release-cli"` key would
    be a second place to keep one fact. `Goal.release_cli` owns what is done about it.
    """
    return c["kind"] == "command" and any("bench.py" in a for a in c["argv"])


class GoalError(ValueError):
    """`loop-goal.toml` parsed as TOML but is not an acceptance list this driver can run.

    Deliberately the same class of event as a TOML syntax error, and handled in the same two
    places: at start-up it refuses to begin, and mid-run it keeps the last good spec and names
    itself in the ledger. A run of 300 sessions must not end on one session's typo."""


# Every key the driver reads off a check, per kind: the ones it reads without a default, then the
# ones it reads with one. `validate_spec` refuses anything outside both, and both halves are worth
# refusing at load. A missing required key is a `KeyError` raised from the middle of a sweep --
# hours into a run, after the build and every check before it has been paid for, and as a traceback
# rather than a sentence. An unrecognized one is the quieter failure: `min_passsing` on a suite is a
# stopping condition that silently is not there, and the check stays green while it guards nothing.
CHECK_KEYS = {
    #  kind           required                    optional
    "exact":       (("file", "want"),             ("args", "exit")),
    "ordered":     (("file", "want"),             ("args", "exit", "stream")),
    "contains":    (("file",),                    ("args", "exit", "stderr_contains",
                                                   "stdout_contains")),
    "min-bytes":   (("file", "min_bytes"),        ("args", "exit", "stream")),
    "command":     (("name", "argv"),             ("cwd", "want", "memoize", "exit")),
    "nvs-suite":   (("name", "args"),             ("cases", "memoize", "min_passing")),
    "cargo-named": (("name", "args", "tests"),    ("memoize",)),
}
# Allowed on every kind. `stage` is read by this driver for the run order and by `holes.py` for the
# worklist it prints, which is why an unstaged check is still legal but a misspelled one is not.
COMMON_KEYS = ("kind", "stage")
LIST_OF_STR = ("args", "argv", "cases", "stderr_contains", "stdout_contains", "tests", "want")
# Empty, each of these is a check that cannot fail: no argv to run, no test that must have run, no
# substring that must appear in order. `want` on an `exact` is NOT here -- empty there is the one
# meaning that reads right, "this fixture prints nothing", and it is asserted like any other line.
NONEMPTY = {"argv", "tests", ("ordered", "want")}
COUNTS = ("min_bytes", "min_passing")


def validate_spec(spec):
    """Everything about an acceptance list that can be known before a single check is run.

    Raises `GoalError` naming the offending check and what it is missing; returns nothing. This is
    the whole of the driver's schema: the checks it can run, the keys each kind carries, and the
    three shapes -- a vacuous `contains`, a fixture outside `files`, a memo name used twice -- that
    parse fine and then do not guard what they were written to guard."""
    files = spec.get("files", [])
    if not isinstance(files, list) or not all(isinstance(f, str) for f in files):
        raise GoalError("`files` must be a list of fixture paths")
    checks = spec.get("check", [])
    if not isinstance(checks, list):
        raise GoalError("`check` must be a list of `[[check]]` tables")

    memo_names = {}
    for i, c in enumerate(checks, 1):
        named = c.get("name") or c.get("file") or "unnamed"
        where = f"check {i}, {named} [{c.get('stage', '?')}]"
        kind = c.get("kind")
        if kind not in CHECK_KEYS:
            known = ", ".join(sorted(CHECK_KEYS))
            raise GoalError(f"{where}: unknown kind {kind!r} -- one of: {known}")
        required, optional = CHECK_KEYS[kind]
        for key in required:
            if key not in c:
                raise GoalError(f"{where}: a {kind} check needs `{key}`")
        for key in sorted(set(c) - set(required) - set(optional) - set(COMMON_KEYS)):
            raise GoalError(f"{where}: a {kind} check has no `{key}`, so nothing would read it")

        for key in sorted(set(c) & set(LIST_OF_STR)):
            if not isinstance(c[key], list) or not all(isinstance(x, str) for x in c[key]):
                raise GoalError(f"{where}: `{key}` must be a list of strings")
            if not c[key] and (key in NONEMPTY or (kind, key) in NONEMPTY):
                raise GoalError(f"{where}: `{key}` is empty, so the check cannot fail")
        for key in sorted(set(c) & set(COUNTS)):
            if not isinstance(c[key], int) or isinstance(c[key], bool) or c[key] < 0:
                raise GoalError(f"{where}: `{key}` must be a count, not {c[key]!r}")
        if c.get("exit", "nonzero") != "nonzero":
            raise GoalError(f'{where}: `exit` is absent or "nonzero", not {c["exit"]!r}')
        if kind == "contains" and not (c.get("stderr_contains") or c.get("stdout_contains")):
            raise GoalError(f"{where}: a contains check naming no substring cannot fail")
        if "file" in c and c["file"] not in files:
            raise GoalError(f"{where}: {c['file']} is not in `files`, so nothing checks it is on "
                            f"disk and the valgrind sweep never sees it")
        # A memo is keyed on the name alone, so two memoizable checks sharing one means the first
        # to go green skips the second for the rest of the run. Unmemoized duplicates are fine and
        # the goal file has a pair on purpose: two stages naming the same suite over the same cases.
        if c.get("memoize") or named in EXPENSIVE:
            if named in memo_names:
                raise GoalError(f"{where}: a memoized check is already named {named!r} at "
                                f"check {memo_names[named]}, and the memo is keyed on the name")
            memo_names[named] = i


class Goal:
    """The acceptance test, read from docs/agent/loop-goal.toml. `check()` returns "" when everything
    passes, or the first failure as one line -- with one deliberate exception, a suite that runs
    clean but holds fewer cases than its `min_passing`. That is the loop's stopping condition rather
    than a claim about the language, so it is held back and reported only once everything else has
    run; otherwise a corpus still being grown short-circuits the memory-safety sweep behind it for
    as many sessions as the growing takes.

    Two things are memoized, and neither of them skips a check:

    * **Within one run**, an identical `args` list runs cargo once, and an identical `nvs-suite`
      `args` list runs the suite once per leg. The list holds `nvs-runtime` twice on purpose --
      stage 0 and stage 5 name different guard tests on it -- and running the crate's suite a
      second time cannot answer differently. The suite half is the same argument and was the
      larger omission: `tests/conformance/` is named by five checks across four stages, and a
      sweep before this memo paid 16-19s for each of them -- 57s of its 173s re-running a suite
      whose verdict it already held. What a check reads off the shared result -- its own `cases`,
      its own `min_passing` -- is still judged per check.
    * **A plain `cargo test -p <crate>` check runs no cargo of its own.** One warm
      `cargo test --no-run` over the workspace -- the build `verify.py` already made -- names every
      test executable with its package (`test_executables`), and the check runs the crate's own
      binaries directly (`crate_tests`). Measured: `cargo test -p X` straight after a workspace
      build RECOMPILES X, because a package selected alone unifies its dependencies' features
      differently from the workspace, so the `-p` artifact is a second one that every source edit
      stales. A sweep paid that seven times over -- 28s of rebuilds in front of 25s of tests --
      plus a cargo start per check. What this path does not run is doc-tests, which no `tests`
      list can name anyway (a doc-test is `path.rs - Item (line N)`); `verify.py` runs them.
    * **Across runs**, the three checks in `EXPENSIVE` are remembered against a content hash of
      *the files they read* -- `crates/`, `examples/`, the manifests, the toolchain and the goal
      file (`inputs_id`). Those inputs being bit-identical is the whole argument: a deterministic
      check over identical bytes cannot reach a different verdict, which is `verify.py`'s rule for
      its own green cache. It used to key on the tree instead, HEAD included, and every session
      commits -- so the memo never once fired inside a run. `tests/` and `docs/` are not inputs to
      any of the three, and 8 of 22 sessions of one measured run touched nothing else.
    """

    def __init__(self, spec):
        validate_spec(spec)
        self.files = spec.get("files", [])
        self.checks = spec.get("check", [])
        self.valgrind_skip = set(spec.get("valgrind", {}).get("skip", []))
        self.wsl_target = spec.get("wsl", {}).get("target_dir", "/var/tmp/nvs-target-wsl")
        self.program_checks = [c for c in self.checks if c["kind"] in PROGRAM_KINDS]
        self.ran = []  # (label, seconds) for every check this run actually paid for
        self.short = []  # `min_passing` thresholds not met; see `check()`
        self.verbose = False  # narrate each check as it starts and what it cost
        self._begun = 0.0  # monotonic start of the current check(), for the elapsed stamp
        self._cargo = {}  # args tuple -> Result, within one check() call
        self._suite = {}  # (leg name, args tuple) -> Result, likewise
        self._exes = None  # package -> [(target, exe, dir)] off the workspace build; see test_executables
        self._crate_runs = {}  # package -> Result of its binaries, within one check() call
        self._tree = ""
        self._inputs = None  # content hash of what the memoizable checks read; see `inputs_id`
        self._green = {}  # check name -> the `inputs_id` it was last green over
        self._memoized = {c["name"] for c in self.checks if c.get("memoize") and "name" in c}
        self._prebuild = None  # the thread warming the release profile
        self._prebuilt = ()  # the args it is warming, so `cargo()` knows to wait for it
        self.release_gate = True  # may the release profile be built, and its cost guards run?
        self.release_owed = []  # what the gate held back, for `release_catch_up` to settle
        self.fast_path = ""  # a check name to try before the sweep; see `fast_fail`
        self.failed_name = ""  # the `cargo-named` check this run died on, for the next one
        self._widen = suite_widening(self.checks)
        cargo = [c for c in self.checks if c["kind"] not in PROGRAM_KINDS]
        # Stage 0 is catch-up: work a later ADR reopened inside a milestone that
        # was already reported done. It runs before everything else so the
        # ledger names it while it is unfinished -- a Stage 3 fixture failing is
        # not the thing the loop should be told about first. See loop-goal.md.
        catch_up = [str(c.get("stage", "")).startswith("0") for c in cargo]
        # A check that BUILDS or MEASURES the release profile is held to the end of the sweep
        # whatever stage it is labelled with -- the one place the stage order above is not the run
        # order, and `check()` says why.
        #
        # Selected by what the check NEEDS rather than by `--release` in its argument list.
        # `tools/bench.py` reads `target/release/nvs.exe` and takes no such flag, so the warm-start
        # guard used to run in the middle of `cargo_checks` -- precisely where `check()` says a
        # cost-class assertion must not run, because the machine is not idle there. It measured
        # 8.3-9.1 ms across some forty sweeps and then 11.0 ms against a 10 ms budget once a second
        # agent session shared the box, which is the guard reporting the machine and not the tree.
        self.release_checks = [c for c in cargo
                               if "--release" in c.get("args", []) or measures_release_cli(c)]
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

    def join_prebuild(self):
        """Waits out the background release build, if one is still in flight.

        Called from the two places that must not read a half-built release profile: the `cargo`
        invocation the prebuild was warming, and a `command` check that measures the release CLI.
        """
        if not self._prebuild:
            return
        self.trace("waiting for the release build started at the top of the sweep")
        self.timed("release prebuild (overlapped)", self._prebuild.join)
        self._prebuild = None

    def cargo(self, args):
        """`cargo` with the result shared by every check that asks for the same argument list."""
        key = tuple(args)
        if key == self._prebuilt:
            self.join_prebuild()
        if key not in self._cargo:
            self._cargo[key] = capture("cargo", args)
        return self._cargo[key]

    def suite(self, leg, args, exact=False):
        """`nvs test` on a leg, with the result shared by every check that asks for the same
        argument list there. Keyed on the leg as well as the args because `leg_check()` runs the
        same lists through the Linux build, and two binaries are two answers.

        The request is first WIDENED to the largest directory the goal asks about that contains
        it, which is what makes a suite over `tests/conformance/core/` share the run a suite over
        `tests/conformance/` was going to pay for anyway -- `suite_widening` owns why, and what it
        was measured to cost unwidened.

        `exact=True` turns that off, and there is exactly one thing it is for: `min_passing` counts
        the cases that RAN, so a floor over the narrow directory would be satisfied by the wide
        one's larger count. That is the difference between "this corpus is big enough" and "some
        corpus is", and it is the loop's stopping condition -- so the floor keeps its own run."""
        if not exact:
            args = self.widen_suite(args)
        key = (leg.name, tuple(args))
        if key not in self._suite:
            self._suite[key] = leg.suite(args)
        return self._suite[key]

    def widen_suite(self, args):
        """`args` with its directory replaced by the widest one that contains it, or unchanged."""
        if len(args) < 2:
            return args
        wider = self._widen.get((tuple(args[:-1]), args[-1]))
        return [*args[:-1], wider] if wider else args

    # -- the workspace test build, and a crate's binaries run off it ---------------------

    def test_executables(self):
        """Every test executable in the workspace, by package: `{name: [(target, exe, dir)]}`,
        and "" -- or `None` and the failure line when the build itself fails, so a tree that
        does not compile is reported by the first check that asks as a build failure rather
        than as a test that did not run.

        One `cargo test --no-run --message-format=json`: `verify.py`'s own `cargo test` without
        the run, so on the tree a session just verified it is a fingerprint scan, and on any
        other it is the one build every crate then shares. The artifact messages carry the
        package (`path+file:///…/crates/nvs-types#0.0.1`, or `…/benches/abi-probe#nvs-abi-probe@0.0.1`
        when directory and package differ), the target, its kind and the executable, and they
        are emitted for `fresh` units too, which is what makes the warm case free."""
        if self._exes is not None:
            return self._exes, ""
        self.trace("building the workspace's test executables (shared by every cargo check)")
        r = self.timed("workspace test build",
                       lambda: capture("cargo", ["test", "--no-run", "--message-format=json"],
                                       log_stdout=False))
        if r.code != 0:
            return None, f"the workspace test build failed -- {r.first_err_line}"
        exes = {}
        for line in r.out.splitlines():
            try:
                m = json.loads(line)
            except ValueError:
                continue
            if m.get("reason") != "compiler-artifact" or not m.get("executable"):
                continue
            if not m.get("profile", {}).get("test"):
                continue
            if not set(m.get("target", {}).get("kind", [])) & {"lib", "bin", "test"}:
                continue
            pid = m.get("package_id", "")
            source, _, tail = pid.rpartition("#")
            name = tail.split("@", 1)[0] if "@" in tail else source.rstrip("/").rsplit("/", 1)[-1]
            exes.setdefault(name, []).append(
                (m["target"]["name"], m["executable"], str(Path(m["manifest_path"]).parent)))
        self._exes = exes
        return exes, ""

    def crate_tests(self, crate):
        """`cargo test -p <crate>`'s verdict off the shared build: each of the crate's test
        executables run in turn, where cargo would run it -- the package's own directory, with
        `CARGO_MANIFEST_DIR` set -- and the outputs joined so a check reads them as it read the
        one cargo output. Shared by every check naming the crate, like `cargo()`.

        The exit code is the first non-zero one, and the run stops there as cargo's does. libtest
        reports a failure on STDOUT, so the stderr the ledger's line is read from is given one
        naming the failing tests: without it the line would end at `exit 101 --`."""
        if crate in self._crate_runs:
            return self._crate_runs[crate]
        exes, fail = self.test_executables()
        if exes is None:
            r = Result(-1, "", fail)
        elif crate not in exes:
            r = Result(-1, "", f"no test executable in the workspace build belongs to {crate!r}")
        else:
            code, outs, errs = 0, [], []
            for target, exe, cwd in exes[crate]:
                TICKER.set(detail=f"{crate}: {target}")
                one = capture(exe, [], cwd=cwd, env=dict(os.environ, CARGO_MANIFEST_DIR=cwd))
                outs.append(one.out)
                errs.append(one.err)
                if one.code != 0:
                    code = one.code
                    failed = [ln.split()[1] for ln in one.out.splitlines()
                              if ln.startswith("test ") and ln.rstrip().endswith("FAILED")]
                    errs.insert(0, f"{crate} ({target}): " + (
                        f"{len(failed)} test(s) failed: {', '.join(failed)}" if failed
                        else f"exit {one.code} -- {one.first_err_line}"))
                    break
            r = Result(code, "\n".join(outs), "\n".join(errs))
        self._crate_runs[crate] = r
        return r

    # -- the release build, moved off the critical path ---------------------------------

    def release_args(self):
        """The one check in the goal whose cost is a BUILD and not a test.

        `--release` is a different profile from everything else here, so nothing it needs is on
        disk when the sweep starts, and this workspace's release profile is `lto = "thin"` with
        `codegen-units = 1` -- measured at 133s after a one-line change to `nvs-runtime`, against
        the ~130s the whole rest of the sweep costs. Found by its `--release` rather than named,
        so a goal that moves the guard to another crate does not have to come back here.

        `None` when there is no such check, or when its verdict is already remembered against this
        tree: `prebuild` would then be warming a profile nothing is going to ask about."""
        for c in self.checks:
            if c["kind"] in PROGRAM_KINDS or "--release" not in c.get("args", []):
                continue
            return None if self.remembered(c["name"]) else c["args"]
        return None

    def release_cli(self):
        """The other release build: `target/release/nvs.exe`, when a check measures it.

        `tools/bench.py` measures that binary and deliberately builds nothing — its own
        `warn_if_stale` says the numbers are about the build on disk rather than the tree — and no
        other leg here produces it, since `NativeLeg` builds the debug CLI. So the warm-start check
        was measuring whatever had last been built by hand, which twice meant a binary too old to
        read the tree's own `nvs.toml` and reported a *configuration* error as the start figure.

        Built in `prebuild`'s thread rather than before the check: it is the same profile, the same
        lock and the same argument as the release test build above — the build overlaps the sweep
        and the measurement does not.
        """
        for c in self.checks:
            if not measures_release_cli(c):
                continue
            return None if self.remembered(c["name"]) else ["build", "--release", "-p", "nvs-cli"]
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
        before it starts the real invocation, which by then is a no-op build and a 3s test run.

        Nothing at all when the release gate is shut: that is the whole point of the gate, since
        this build IS the sweep's critical path and not merely a step in it. Measured on the
        20260904-143054 run, the sweep waited **84s** at `join_prebuild` for a build that had been
        running since t=0, and everything else fitted underneath it."""
        if not self.release_gate:
            return
        args = self.release_args()
        cli = self.release_cli()
        if not args and not cli:
            return
        def build():
            # One at a time across the whole run. A sweep that fails before it reaches the guard
            # returns with this thread still building -- correctly, since the work is wanted either
            # way -- and `load_goal()` hands the next session a fresh `Goal` that knows nothing
            # about it. Without the lock those two cargos would build the same units at once.
            with PREBUILD_LOCK:
                if args:
                    capture("cargo", [*args, "--no-run"])
                if cli:
                    capture("cargo", cli)

        # The test profile's args, or nothing to match: a key is a tuple, so a `cargo()` looking
        # for one never matches the `None` a CLI-only prebuild leaves here, and the thread is
        # joined by the bench check instead.
        self._prebuilt = tuple(args) if args else None
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
        purpose: no check keyed on this executes a `.nvst` case or reads a document.

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
            # `{nvs}` is the CLI the leg already built. A check that wants to run a subcommand --
            # `nvs config check`, `nvs build --openapi`, `nvs queue migrate` -- would otherwise
            # either hard-code a profile-dependent path or pay for a second `cargo run`, which is
            # one workspace fingerprint scan to start a binary already sitting on disk.
            argv = [(leg.binary if a == "{nvs}" else a) for a in c["argv"]]
            # A check that measures the release CLI waits for the build of it that started at the
            # top of the sweep -- `release_cli` owns why that build is this driver's job at all.
            # It cannot arrive here with the gate shut: `release_checks` holds every check that
            # reads that binary, and that loop is the one place the gate skips one.
            if measures_release_cli(c):
                self.join_prebuild()
            r = self.timed(label, lambda: capture(argv[0], argv[1:],
                                                  cwd=ROOT / c.get("cwd", ".")))
            # Some commands fail by design -- `nvs config check` over a file that must be refused
            # is one, and its exit code is the assertion. `exit = "nonzero"` inverts the
            # expectation exactly as it does on a fixture.
            wanted_nonzero = c.get("exit") == "nonzero"
            if wanted_nonzero and r.code == 0:
                return f"{label}: exit 0, and this check asserts a non-zero exit"
            if not wanted_nonzero and r.code != 0:
                return f"{label}: exit {r.code} -- {r.first_err_line}"
            # `ordered_in` is a predicate, not a description of what is absent -- reading it as
            # one inverts the check and reports the bare word `True` as the failure.
            want = c.get("want", [])
            if not ordered_in(r.out + "\n" + r.err, want):
                return f"{label}: output lacks, in order: {' -> '.join(want)}"
            self.remember(c["name"])
            return ""

        if c["kind"] == "nvs-suite":
            # The suite runner is the CLI the leg already built; `cargo run` here would be one
            # more workspace fingerprint scan to start a binary sitting on disk. Through the leg
            # rather than at the binary, because `--leg-only` runs these on the Linux build, whose
            # path Windows cannot execute. Shared by every check naming the same args on the same
            # leg, exactly as `cargo()` shares a cargo run -- see the class doc.
            r = self.timed(label, lambda: self.suite(
                leg, c["args"], exact=c.get("min_passing") is not None))
        elif crate := plain_crate_test(c["args"]):
            # `cargo test -p <crate>` and nothing else: the crate's binaries off the shared
            # workspace build rather than a cargo run of its own -- the class doc has the rebuild
            # a `-p` run pays. Anything more than that -- `--release`, `--test`, a feature -- is
            # a different build and keeps its own invocation.
            r = self.timed(label, lambda: self.crate_tests(crate))
        else:
            r = self.timed(label, lambda: self.cargo(c["args"]))
        if r.code != 0:
            return f"{label}: exit {r.code} -- {r.first_err_line}"
        both = r.out + "\n" + r.err

        if c["kind"] == "nvs-suite":
            m = SUMMARY_RE.search(both)
            if not m:
                return f"{label}: no 'N passed, M failed' summary line in the output"
            if int(m.group(2)) != 0:
                return f"{label}: {m.group(2)} case(s) failed"
            # `cases` is the `.nvst` twin of `cargo-named`, and it exists for the same reason: a
            # suite is green when a case was never written, and `min_passing` cannot tell the
            # difference between "the corpus grew" and "the corpus grew somewhere else". A named
            # case must be on disk AND not have been skipped -- an `--ORACLE--` whose probe fails
            # skips silently, and the SKIP line is the only place that shows.
            #
            # Path separators are normalized both ways: `nvs test` prints whatever the platform's
            # `Path::display` gives it, so the same case is `tests/…` here and `tests\…` there.
            flat = both.replace("\\", "/")
            for case in c.get("cases", []):
                want = case.replace("\\", "/")
                if not (ROOT / case).exists():
                    return f"{label}: case {want} is not written yet"
                if f"SKIP {want}" in flat:
                    return f"{label}: case {want} was skipped, so nothing ran it"
            # `min_passing` is optional, and a suite check that omits it is not a mistake: a stage
            # whose worklist is its `cases` list -- Stage 00 names the shapes a first page reaches
            # for -- has no corpus size to reach, and a threshold there would be a second number
            # saying what the case list already says. Absent means "no stopping condition here".
            floor = c.get("min_passing")
            if floor is not None and int(m.group(1)) < floor:
                # Held, not returned. A `min_passing` threshold is the loop's STOPPING condition --
                # "is the corpus big enough yet" -- and not a correctness signal; the two lines
                # above are the correctness half and they have just passed, so every case that
                # exists runs and none of them fails. Returning here would short-circuit the Stage 5
                # guards, the second leg and the valgrind sweep for as long as the corpus is still
                # growing, which is dozens of sessions, and priority 1 does not wait behind
                # priority 4. `check()` reports this after all of them.
                self.short.append(
                    f"{label}: only {m.group(1)} passing case(s), wanted at least {floor}"
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

        # Relative on the wsl leg because the line already `cd`s into the repo there; absolute
        # natively, where the only guarantee is this process's own cwd. The file itself says what
        # it hides and why -- two contexts inside `ring`'s AEAD assembly, which memcheck reports
        # on every TLS connection a fixture's queue worker opens.
        supp = ("tools/valgrind.supp" if leg.name == "wsl"
                else str(ROOT / "tools" / "valgrind.supp"))

        # 97 rather than 1, and `tools/leak-check.sh` owns why: a fixture's own exit status passes
        # straight through valgrind, so under `--error-exitcode=1` a fixture that ends in a FATAL
        # by design is indistinguishable from one that leaked. `examples/limits.nvs` is that
        # fixture -- it exists to cross the memory ceiling -- and it read as a leak for as long as
        # this said 1.
        vg_error = 97

        def cmd_for(f):
            return (f"cd {leg.repo} && " if leg.name == "wsl" else "") + (
                f"valgrind --error-exitcode={vg_error} --leak-check=full "
                f"--errors-for-leak-kinds=definite --suppressions={supp} -q {leg.binary} run {f}"
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
        jobs = machine.jobs(leg.name, ceiling=len(targets), envs=("NVS_VALGRIND_JOBS",))
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
                if r.code == vg_error:
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
            n += sum(1 for c in self.cargo_checks if c["kind"] == "nvs-suite")
            return n + (sweep if sweepable else 0)

        if self.fast_check() is not None:
            n += 1  # last session's failing check, tried before anything is built
        n += 1 + len(self.catch_up_checks) + programs  # native build, catch-up, native fixtures
        # The shared workspace test build, paid once by the first plain `cargo test -p` check.
        if any(plain_crate_test(c.get("args", [])) for c in self.catch_up_checks + self.cargo_checks):
            n += 1
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
        self._suite = {}
        self._exes = None
        self._crate_runs = {}
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
        """The acceptance sweep, with the local origins its network fixtures need held open around
        it. An `ExitStack` rather than a `with` per leg because `_check` returns from a dozen places
        and the second origin's lifetime begins in the middle of it."""
        with contextlib.ExitStack() as origins:
            self.origins = origins
            return self._check(verbose)

    def _check(self, verbose=False):
        self.verbose = verbose
        trace = self.trace

        TICKER.set(phase="acceptance check")
        fail = self.begin("check")
        if fail:
            return fail

        # Before ANY build, because the whole value of it is not paying for one: see `fast_fail`.
        fail = self.fast_fail()
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
            fail = self.run_cargo_check(c, native)
            if fail:
                return fail

        # Held for the whole sweep and not just for the fixtures: on a Linux host `leg` stays
        # `native`, so this is also the origin the valgrind sweep's own run of `examples/http.nvs`
        # reaches.
        self.origins.enter_context(local_origin(native))
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
            fail = self.run_cargo_check(c, native)
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
            # Before the branch below rather than inside it: the sweep runs on this leg even when
            # its fixtures are already green on these inputs, and it runs `examples/http.nvs` too.
            self.origins.enter_context(local_origin(leg))
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
            if not self.release_gate:
                self.release_owed.append(c["name"])
                trace(f"cargo {c['name']} (release gate shut -- owed until the sweep goes green)")
                continue
            trace(f"cargo {c['name']}")
            fail = self.run_cargo_check(c, native)
            if fail:
                return fail
            self.remember(c["name"])

        # Last, because a corpus that is merely still growing is the one failure that must not hide
        # anything: everything above is a claim about whether the language is correct on both legs
        # and leaks nothing, and all of it has now run. See `cargo_check`'s `min_passing` arm.
        return self.short[0] if self.short else ""

    # -- the two things that let a sweep cost less than all of it ------------------------

    def run_cargo_check(self, c, leg=None):
        """`cargo_check`, remembering WHICH check a sweep died on so the next one can try it
        first. Only a `cargo-named` check is remembered, because only that kind is cheap enough
        to be worth trying alone -- `fast_fail` says what that costs and what it buys."""
        fail = self.cargo_check(c, leg)
        if fail and c["kind"] == "cargo-named" and "--release" not in c.get("args", []):
            self.failed_name = c["name"]
        return fail

    def fast_check(self):
        """The check `fast_path` names, if the goal still holds one of that name and kind."""
        if not self.fast_path:
            return None
        return next((c for c in self.checks
                     if c.get("name") == self.fast_path and c["kind"] == "cargo-named"
                     and "--release" not in c.get("args", [])), None)

    def fast_fail(self):
        """Last session's failing check, run first and alone, before a single build is started.

        The acceptance list is a frontier: a session is handed the check its work has to turn
        green, and the sweep dies at that same check for as long as the work is unfinished. Over
        the 20260904 runs that was every session but one -- nine ledger lines in a row ending at a
        `cargo-named` check whose test was not written yet -- and each of them paid the full sweep
        to find it out. A `cargo-named` check costs the shared workspace test build and one crate's
        binaries, so asking it first turns a **4m45s** answer into roughly **15s**.

        What it defers, and the argument that this is safe: a sweep that stops here has not run the
        fixtures, the suites, the second leg or the valgrind sweep over this session's work. But the
        session itself has just run `verify.py` -- build, fmt, test, both `.nvst` trees, clippy --
        and, decisively, **a goal cannot be declared reached without a full green sweep**: `drive`
        advances the chain only on an empty `fail`, and `fail` is empty only when everything below
        has run. So the full sweep runs on exactly the sessions that move the frontier, and a
        regression hidden behind a red check is found by the session that turns it green, with one
        commit per slice in `git log` to localize it.

        Only `cargo-named`, and never a `--release` one: a fixture or a suite is not cheap enough
        for the fast path to be worth anything, and a release check would build the profile this
        run is trying not to build."""
        c = self.fast_check()
        if c is None:
            return ""
        self.trace(f"fast path: {c['name']}, the check the last sweep died at")
        fail = self.run_cargo_check(c)
        if fail:
            return fail
        self.trace("fast path green -- the full sweep runs")
        return ""

    def release_catch_up(self, verbose=False):
        """Run the release-profile checks the gate held back, now that everything else is green.

        This is the gate's hole, closed at the one place it matters. `release_gate` skips a cost
        guard on four sessions in five, which is right while the sweep is red -- but a green sweep
        is the end of a goal, and a goal must not be declared reached on a sweep that skipped one.
        Reaching here with an empty `fail` IS that moment, for the last goal in the chain as much
        as for any other, so nothing here has to know what a chain or a milestone is.

        Cheap because it is not a second sweep: `_cargo`, `_crate_runs` and `_exes` still hold this
        run's answers, so what is paid is the release build the gate declined and the two or three
        checks that read it."""
        self.verbose = verbose
        self.release_gate = True
        owed = set(self.release_owed)
        self.release_owed = []
        if not owed:
            return ""
        self.trace(f"release gate: settling {len(owed)} check(s) held back, the rest being green")
        self.prebuild()
        native = NativeLeg()
        fail = self.timed("native build (release catch-up)", native.prepare)
        if fail:
            return fail
        for c in self.checks:
            if c["kind"] in PROGRAM_KINDS or c.get("name") not in owed:
                continue
            self.trace(f"cargo {c['name']} (release gate)")
            fail = self.run_cargo_check(c, native)
            if fail:
                return fail
            self.remember(c["name"])
        return ""

    def leg_check(self, verbose=False):
        """The Linux leg, with the origin its network fixtures need held open around it -- the same
        shape as `check` above, and for the same reason."""
        with contextlib.ExitStack() as origins:
            self.origins = origins
            return self._leg_check(verbose)

    def _leg_check(self, verbose=False):
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
        self.origins.enter_context(local_origin(leg))

        for c in self.program_checks:
            trace(f"{leg.name} {c['file']}")
            fail = self.program_check(leg, c)
            if fail:
                return fail

        for c in self.cargo_checks:
            if c["kind"] != "nvs-suite":
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

    Raises `GoalError` for a list that parses but cannot be run; both callers handle it exactly as
    they handle a `TOMLDecodeError`, and `validate_spec` says why the checking happens here.
    """
    return Goal(tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")))


def goal_title(chain=None):
    """The loop-goal half of the status line's goal row: the H1 of `docs/agent/loop-goal.md`,
    behind `goal 2/6` when a chain is driving. Read from disk each time, for the reason
    `load_goal()` is: a chain switch rewrites the file, and the row has to follow it. A file that
    cannot be read, or has no heading, leaves the row saying so rather than ending the run."""
    title = ""
    try:
        for line in GOAL_MD.read_text(encoding="utf-8").splitlines():
            if line.startswith("# "):
                title = line[2:].strip()
                break
    except OSError:
        pass
    title = title or f"{rel_to_root(GOAL_MD)} has no heading"
    if chain is not None and chain.current is not None:
        return f"goal {chain.index + 1}/{len(chain.goals)}{TICKER.sep}{title}"
    return title


def session_goal(pack):
    """The checklist item `orient.py` picked for the session about to run, read back out of the
    pack rather than out of the handoff. What the goal row opens on, until `SliceWatch` has a
    commit to show instead.

    The pack is the one place the pick is authoritative -- `--item`, a ticked-out group and a
    handoff with no group at all are all decided in there -- so parsing the handoff again here
    would be a second implementation of `orient.py`'s rule that drifts the first time that rule
    moves. The marker is `-- YOUR ITEM (n of m), in full:` and the item's first line follows it;
    the row keeps the bold title when the line has one, the line itself when it does not.
    An empty pack means orient.py failed and the session picks for itself, and the row says so.

    The marker's `n of m` is deliberately **dropped** rather than shown. It reads like progress
    and cannot move: `orient.py` picks the group's first unticked item, the driver never passes
    `--item`, and every session overwrites the handoff with a fresh group whose items are all
    unticked -- so it was `item 1/3` in every session of every run on record. The item's own
    title says which one it is; what the session then does with it is `SliceWatch`'s to say."""
    if not pack:
        return "orient.py failed -- the session picks its own item"
    lines = pack.splitlines()
    for i, line in enumerate(lines):
        if not re.match(r"^-- YOUR ITEM \(\d+ of \d+\), in full:\s*$", line):
            continue
        first = next((ln.strip() for ln in lines[i + 1:] if ln.strip()), "")
        first = re.sub(r"^- \[[ xX]\]\s*", "", first)
        bold = re.match(r"^\*\*(.+?)\*\*", first)
        return (bold.group(1) if bold else first) or "the pack's item has no text"
    if "Every item in the group is ticked" in pack:
        return "every item in the group is ticked -- the session picks from the handoff"
    return "the pack names no item -- see the handoff's `## Next group`"


class SliceWatch:
    """What the goal row's session half says: what the running session has landed **so far**,
    rather than what it was handed when it started.

    The row used to name one thing -- the checklist item the pack picked -- set once when the
    pack was built and never touched again. Two facts made that a line a run could not be read
    off. The pick never varies, for the reasons `session_goal` records. And AGENTS.md's step 2
    tells a session to keep taking slices from its group while the file set and the context
    ceiling hold, so sessions land two to seven commits: the item a session opened on stops
    describing it within minutes, and sometimes never described it -- session 0004 of the
    2026-09-04 run opened on the group's first item, then committed its second one and a fix the
    group did not name, while the row spent the whole session naming the first.

    So the row follows the one thing that moves and is not a session's opinion of itself: HEAD. A
    commit is a slice finished, its subject is the sentence the session wrote about what it just
    did, and the count is `git rev-list` over the range the ledger reports -- one computation,
    polled live here and read back by `drive()` rather than worked out a second time at the end.

    Polled off the session's own tool calls, never off a timer: the check is two small file reads
    and git is spawned only on the calls where the sha actually moved, which is a handful a
    session. A packed ref, or a `.git` that is a worktree's file, leaves no loose ref to read;
    that falls back to asking git, and no more often than `SLOW_EVERY` seconds."""

    SLOW_EVERY = 15.0

    def __init__(self):
        self.lock = threading.RLock()
        self.base = ""  # HEAD when the session started -- the ledger counts from here too
        self.item = ""  # what the pack handed it, shown until the first slice lands
        self.head = ""  # the newest sha this watch has seen
        self.commits = 0
        self.subject = ""
        self._next_slow = 0.0

    # -- what the driver tells it ------------------------------------------------------

    def start(self, base):
        """A session is about to be oriented. Everything the last one landed is now history, and
        the row must stop asserting its item: `orient.py` is a minute of work away, and for that
        minute the row used to name a slice that had already been committed and handed off."""
        with self.lock:
            self.base = self.head = base or ""
            self.item = "orienting -- no item picked yet"
            self.commits = 0
            self.subject = ""
            self._next_slow = 0.0
        TICKER.set(goal=self.row())

    def pick(self, item):
        """The pack is built and it named an item. Shown until the first commit replaces it."""
        with self.lock:
            self.item = item
        TICKER.set(goal=self.row())

    def poll(self):
        """Look for a slice that landed since the last look, and move the row if one did."""
        sha = self.sha()
        with self.lock:
            if not sha or sha == self.head:
                return
            self.head = sha
            base = self.base
        # Both spawns are outside the lock: the ticker thread paints this row eight times a
        # second and must never be found waiting on a subprocess.
        count = git("rev-list", "--count", f"{base}..{sha}") if base else ""
        subject = git("log", "-1", "--format=%s", sha)
        with self.lock:
            self.commits = int(count) if count.isdigit() else self.commits + 1
            self.subject = subject
        TICKER.set(goal=self.row())

    def finish(self):
        """The session is over. One last look -- its final slice is usually committed by its last
        tool call, with nothing after it to trigger a poll -- and then the count `drive()` puts in
        the ledger, from the same range and the same command the row has been showing all along."""
        with self.lock:
            self._next_slow = 0.0  # the answer is wanted now, not at the next cheap opportunity
        self.poll()
        with self.lock:
            return self.commits

    # -- what it says, and how it looks ------------------------------------------------

    def row(self):
        """The session half of the goal row: the slices landed and the last one's subject, or the
        item the session opened on while that is still all there is to say."""
        with self.lock:
            if not self.commits:
                return self.item
            landed = f"{self.commits} commit{'' if self.commits == 1 else 's'}"
            return f"{landed}{TICKER.sep}{self.subject}" if self.subject else landed

    def sha(self):
        """HEAD's sha without spawning anything, where the repository allows it: `.git/HEAD` names
        a ref, and the loose ref file under it holds the sha. Anything else -- a detached HEAD is
        the sha itself, a packed ref or a worktree `.git` file has no loose ref to read -- falls
        back to asking git, throttled, because this runs on every tool call a session makes."""
        gitdir = ROOT / ".git"
        try:
            if gitdir.is_dir():
                head = (gitdir / "HEAD").read_text(encoding="utf-8").strip()
                if not head.startswith("ref:"):
                    return head
                ref = gitdir / head[4:].strip()
                if ref.is_file():
                    return ref.read_text(encoding="utf-8").strip()
        except OSError:
            pass
        now = time.monotonic()
        with self.lock:
            if now < self._next_slow:
                return ""
            self._next_slow = now + self.SLOW_EVERY
        return git("rev-parse", "HEAD")


SLICES = SliceWatch()


# --------------------------------------------------------------------------------- chain


class ChainError(Exception):
    """A chain file that cannot be walked. Raised at start-up, before a session is launched, for
    the same reason `GoalError` is: a run of hundreds of sessions must not discover on its fourth
    day that entry five names a file nobody wrote."""


class Chain:
    """A sequence of staged goals the driver walks by itself.

    `docs/agent/goals/chain.toml` is the order; each entry names a `.md`, a `.toml` and a
    `.handoff.md`. When the live goal's acceptance list goes green the driver **advances**: it runs
    `tools/goal-switch.py` against the next entry -- which folds the goal that just passed into it
    as its floor -- copies the three files into place, commits that switch, and starts the next
    session. Without this a six-goal program stops five times and waits for a human, which is the
    same program with five extra nights in it.

    Two things are deliberately not automated, and both are in the class of "expensive to get
    wrong and cheap to do once":

    * **The floor is carried by `goal-switch.py`, as a subprocess.** Re-implementing that text
      splice here would be a second copy of the one operation whose failure mode is silent -- a
      missing floor looks exactly like a passing one.
    * **`goal-switch.py` is not idempotent**: it inserts at a marker it leaves in place, so running
      it twice inserts the floor twice. `.loop/chain.json` is what makes "exactly once per entry"
      a fact rather than an intention, and it survives a driver that is killed mid-run.
    """

    def __init__(self, path):
        self.path = Path(path)
        if not self.path.is_file():
            raise ChainError(f"{rel_to_root(self.path)} does not exist")
        self.goals = self._load()
        self.index = self._restore()

    def _load(self):
        """Every `[[goal]]` in the file, validated. `ChainError` on anything unwalkable."""
        try:
            spec = tomllib.loads(self.path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as e:
            raise ChainError(f"{rel_to_root(self.path)} did not parse: {e}") from e
        goals = spec.get("goal", [])
        if not goals:
            raise ChainError(f"{rel_to_root(self.path)} holds no [[goal]] entry")
        for i, g in enumerate(goals, 1):
            for key in ("name", "md", "toml", "handoff"):
                if key not in g:
                    raise ChainError(f"goal {i} in {rel_to_root(self.path)} has no `{key}`")
            for key in ("md", "toml", "handoff"):
                if not (ROOT / g[key]).is_file():
                    raise ChainError(f"goal {i} ({g['name']}) names {g[key]}, which does not exist")
        return goals

    def refresh(self):
        """Re-read the file, so a goal that *appends* entries is walked in the same run.

        `dossier.py --emit-goals --append-chain` is the reason this exists: a goal whose whole job
        is to write the next hundred cannot hand them to a driver that read the chain once at
        start-up, and stopping the run for a human to restart is the thing `--chain` exists to
        avoid.

        **Only growth is adopted.** `.loop/chain.json` is an index into this list and
        `goal-switch.py` has already folded each walked entry's checks into the one after it, so a
        chain whose existing entries moved is not something to follow -- the floor those switches
        built no longer matches the file. That rewrite, and a file that stops parsing mid-run, both
        leave the snapshot in place and the run continues on it: every entry it is walking is still
        on disk, so there is nothing here worth ending three hundred sessions over.

        Returns a one-line note for the console, or "" when nothing changed.
        """
        try:
            fresh = self._load()
        except ChainError as e:
            return f"chain: {rel_to_root(self.path)} changed and is not walkable -- {e}"
        old = [g["md"] for g in self.goals]
        if [g["md"] for g in fresh[:len(old)]] != old:
            return (f"chain: {rel_to_root(self.path)} was rewritten under the run rather than "
                    f"appended to -- walking the {len(old)} entries this run started with")
        if len(fresh) == len(old):
            return ""
        self.goals = fresh
        return (f"chain: {rel_to_root(self.path)} grew by {len(fresh) - len(old)} goal(s) to "
                f"{len(fresh)} -- the run walks them without a restart")

    def _restore(self):
        """Where the chain stands, from `.loop/chain.json`, or -1 for "nothing installed yet".

        A state file naming a different chain is ignored rather than trusted: two chains in one
        repository is not a thing this supports, and silently resuming the wrong one is worse than
        starting over.
        """
        if not CHAINSTATE.exists():
            return -1
        try:
            state = json.loads(CHAINSTATE.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return -1
        if state.get("chain") != self.path.as_posix():
            return -1
        i = state.get("index", -1)
        return i if isinstance(i, int) and -1 <= i < len(self.goals) else -1

    def _save(self):
        CHAINSTATE.write_text(
            json.dumps({"chain": self.path.as_posix(), "index": self.index}, indent=2) + "\n",
            encoding="utf-8",
        )

    @property
    def current(self):
        return self.goals[self.index] if 0 <= self.index < len(self.goals) else None

    @property
    def finished(self):
        return self.index >= len(self.goals) - 1

    def install_next(self):
        """Advance one entry: switch the floor into it, copy it into place, commit. Returns "" on
        success or a one-line reason the run should stop."""
        nxt = self.goals[self.index + 1]
        say("")
        step(f"chain: switching to goal {self.index + 2} of {len(self.goals)} -- {nxt['name']}",
             C.CYAN)

        fail = preflight(nxt.get("preflight"))
        if fail:
            return fail

        # The floor. `goal-switch.py` reads the LIVE goal, so this has to happen before the copy.
        # On the first entry there is no previous chain goal and the live one is whatever the run
        # started against -- which is exactly the floor that entry wants.
        r = capture(sys.executable, [str(ROOT / "tools" / "goal-switch.py"), nxt["toml"]])
        if r.code != 0:
            return f"chain: goal-switch failed for {nxt['name']} -- {r.first_err_line}"
        for line in stdout_lines(r.out):
            say(f"  {line}", C.GRAY)

        shutil.copyfile(ROOT / nxt["toml"], GOAL_TOML)
        # The prose and the handoff are markdown full of relative links, and installing them moves
        # them one directory up -- out of `docs/agent/goals/` and into `docs/agent/`. Copying the
        # bytes verbatim breaks every one of them, which `check-links.py` reports and nothing else
        # notices, because orient.py prints link *text* and a session never follows one to find out.
        GOAL_MD.write_text(
            relocate_links(read_text(ROOT / nxt["md"]), Path(nxt["md"]).parent.name),
            encoding="utf-8", newline="\n",
        )
        (ROOT / "docs" / "agent" / "handoff.md").write_text(
            relocate_links(read_text(ROOT / nxt["handoff"]), Path(nxt["handoff"]).parent.name),
            encoding="utf-8", newline="\n",
        )

        # The spec has to be runnable before a session is spent against it. Same class of failure
        # as a TOML typo in the live goal, caught in the same place: before anything is launched.
        try:
            Goal(tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")))
        except (tomllib.TOMLDecodeError, GoalError) as e:
            return f"chain: {nxt['name']}'s acceptance list is not runnable -- {e}"

        self.index += 1
        self._save()

        # A memoized check's verdict belongs to the goal that asked for it. Carrying it across a
        # switch would let a check the previous goal memoized stand in for one the new goal names.
        GOALCACHE.unlink(missing_ok=True)

        message = (
            f"docs(loop): the chain advances to {nxt['name']}\n\n"
            f"Written by tools/loop.py --chain from {rel_to_root(self.path)}. The previous goal's\n"
            f"whole acceptance list is this one's floor, carried verbatim by goal-switch.py and\n"
            f"relabelled -- see docs/agent/goals/README.md for why that is mechanical.\n"
        )
        msg_file = ROOT / ".agent-tmp" / "chain-switch.txt"
        msg_file.parent.mkdir(parents=True, exist_ok=True)
        msg_file.write_text(message, encoding="utf-8", newline="\n")
        git("add", nxt["toml"], "docs/agent/loop-goal.toml", "docs/agent/loop-goal.md",
            "docs/agent/handoff.md")
        git("commit", "-F", str(msg_file))
        say(f"chain: goal {self.index + 1} of {len(self.goals)} is live -- {nxt['name']}", C.GREEN)
        return ""

    def bring_up_services(self):
        """`[docker]` in the live goal: the containers its checks need, up once per run.

        Once per run and not once per iteration, for the reason the WSL leg and the valgrind sweep
        are memoized: standing up five database servers costs minutes and cannot change a
        deterministic verdict. `--wait` is what makes "up" mean "healthy" rather than "started".
        """
        try:
            spec = tomllib.loads(GOAL_TOML.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError:
            return ""
        docker = spec.get("docker") or {}
        compose = docker.get("compose")
        if not compose:
            return ""
        services = docker.get("services", [])
        step(f"chain: bringing up {len(services) or 'the'} service(s) from {compose}", C.CYAN)
        r = capture("docker", ["compose", "-f", compose, "up", "-d", "--wait", *services],
                    timeout=1800)
        if r.code != 0:
            return (f"chain: `docker compose -f {compose} up` failed -- {r.first_err_line}. "
                    f"The live goal's checks need those services.")
        # `[docker.copy]`: a file a check needs on disk that only exists inside a container. The CA
        # the compose `certs` service issues is the case it was written for -- `nvs.toml`'s
        # `[db.main] tls_ca_file` names it, it belongs to a Docker volume rather than to git, and a
        # fixture that reached the server without it would be one verifying nothing.
        for dest, source in (docker.get("copy") or {}).items():
            r = capture("docker", ["compose", "-f", compose, "cp", source, dest], timeout=120)
            if r.code != 0:
                return (f"chain: `docker compose cp {source} {dest}` failed -- {r.first_err_line}. "
                        f"The live goal's checks need that file on disk.")
        return ""


def read_text(path):
    return Path(path).read_text(encoding="utf-8")


#: A markdown link's target: `](...)`, up to the closing paren. Reference-style links and bare URLs
#: are deliberately not matched -- this repository writes inline links and nothing else, and a
#: rewriter that guessed at other forms would be a second thing to keep right.
LINK_RE = re.compile(r"\]\(([^)]+)\)")


def relocate_links(text, from_dir):
    """Rewrite a goal file's relative links for its new home one directory up.

    A staged goal lives in `docs/agent/goals/` and is installed at `docs/agent/`, so every link in
    it is off by exactly one level -- and in two different directions:

    * `../../adr/0071-…` becomes `../adr/0071-…`: one `../` too many, so one is dropped. This is
      right for `../../../crates/…` too, which loses one of its three and keeps two.
    * `1-core-depth.toml` and `README.md` are siblings in `goals/`, and from `docs/agent/` they are
      `goals/1-core-depth.toml` and `goals/README.md`.

    An anchor, an absolute path and anything with a scheme are left exactly as they are.
    """
    def fix(m):
        target = m.group(1)
        if target.startswith(("#", "/", "http://", "https://", "mailto:")) or "://" in target:
            return m.group(0)
        if target.startswith("../"):
            return f"]({target[3:]})"
        return f"]({from_dir}/{target})"

    return LINK_RE.sub(fix, text)


def preflight(kind):
    """An external precondition a goal names, checked before its first session rather than after
    its first six hours.

    Only one exists: `preflight = "docker"`, for the goal whose drivers ADR 0067 verifies against
    real servers. A run that grinds against a check that cannot pass is worse than one that stops
    in the first minute, and the failure is loud on purpose -- a skipped driver matrix leaves the
    four network drivers unproven while every other check goes green.
    """
    if not kind:
        return ""
    if kind != "docker":
        return f"chain: unknown preflight {kind!r} -- the only one is \"docker\""
    if not shutil.which("docker"):
        return ("chain: this goal needs Docker and the `docker` command is not on PATH. "
                "Install Docker Desktop, or run this goal by hand.")
    r = capture("docker", ["info", "--format", "{{.ServerVersion}}"], timeout=120)
    if r.code != 0:
        return ("chain: this goal needs a reachable Docker daemon and `docker info` failed. "
                "Start Docker Desktop and re-run; nothing has been spent.")
    say(f"chain: docker daemon {r.out.strip()} is up", C.GRAY)
    return ""


# -------------------------------------------------------------------------------- driver


def git(*args):
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, encoding="utf-8", check=True
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return ""


#: Why a run ended, as a *kind* rather than a sentence. `reason` is written for a person and is
#: reworded whenever the wording improves; this is what `tools/loop-supervisor.py` branches on, and
#: only one of these is restartable. Adding a kind here is free; changing one renames an API.
#:
#:   budget          served --max-sessions and stopped. The only kind a supervisor may restart on.
#:   goal            every acceptance check passes and there is no chain
#:   chain-complete  the last goal in the chain is green
#:   chain-error     a chain switch could not be made
#:   asked           `.loop/stop`, or `s` at the console
#:   wall            MAX_WALLS sessions in a row refused by the usage limit
#:   wall-timeout    a usage window that does not reopen inside --max-limit-wait
#:   cli-failed      --max-retries consecutive non-zero exits from the CLI
#:   done-claim      a session claimed DONE that the acceptance test does not agree with
#:   blocked         a session wrote BLOCKED
#:   stalled         --max-stalls sessions in a row produced no commit
#:   interrupted     Ctrl-C
#: How far the current run has got, for the one exit path that cannot see `drive`'s locals: the
#: Ctrl-C handler in `main`. `drive` resets it when a run starts and bumps it per served session.
PROGRESS = {"served": 0, "run_id": ""}


def write_run_end(kind, reason, served=0, run_id=""):
    """Record why this run ended, machine-readably. Best effort: a run that ended for a real
    reason must not also fail on an unwritable `.loop`, so every error here is swallowed. A
    supervisor that finds no file treats the run as terminal, which is the safe direction."""
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        RUNEND.write_text(
            json.dumps({
                "kind": kind,
                "reason": reason,
                "served": served,
                "run_id": run_id,
                "at": f"{datetime.now():%Y-%m-%d %H:%M:%S}",
            }, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )
    except OSError:
        pass


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


# -------------------------------------------------------------------- the overload wall
#
# `529 Overloaded` is the other non-failure that arrives as a non-zero exit, and unlike a usage
# wall it is nothing to do with the account: the server is busy. The CLI fights it first -- ten
# `api_retry` events under its own exponential backoff, about four minutes of them -- and when it
# gives up it says exactly what happened, on the one event that reports how the session ended:
#
#     {"type":"result","is_error":true,"terminal_reason":"api_error","api_error_status":529,
#      "result":"API Error: 529 Overloaded. This is a server-side issue, usually temporary...",..}
#
# Counted as a crash, that was three retries and about three minutes to end a run: measured on
# 2026-09-03, run 20260903-151807, which stopped at session 3 with `cli-failed` and the goal
# untouched. So an overloaded session is re-run instead, and re-run FOREVER -- there is no
# deadline to sleep to the way a closed usage window has one, and nothing about the tree is wrong,
# so the only recovery available is to keep asking. A run that ends itself at 03:00 costs every
# session that would have run before somebody looked at it.
#
# **Parsed, never grepped**, for the same reason the usage wall above is: `api_error_status` is a
# field on the terminal event, while the words "529 Overloaded" appear in any session that reads
# this file, or the log of a session that hit one.

#: The API statuses a session is re-run for rather than counted against `--max-retries`. Just the
#: one, deliberately: a 529 is the server saying "not now", where a 500 can as easily be a request
#: that will fail identically every time it is sent -- and retrying *that* forever is a run that
#: serves no sessions and never stops. Another status is one entry here, once it is known to be
#: transient.
OVERLOAD_STATUS = frozenset({529})

#: What the driver waits before re-running an overloaded session, by consecutive attempt; the last
#: entry repeats for as long as the overload lasts. It opens at a minute because the CLI has
#: already spent about four fighting the same 529, and stops climbing at ten because a busy server
#: is not an escalating problem -- waiting longer past that only makes the run slower to pick up
#: the moment it clears.
OVERLOAD_BACKOFF = (60, 120, 300, 600)


def api_error_status(line):
    """The HTTP status a session's terminal `result` event blames for its exit, or 0.

    Read as a field off that one event, so a session that quoted an error while working -- or read
    the log of a session that hit one -- cannot supply the number itself."""
    try:
        e = json.loads(line)
    except json.JSONDecodeError:
        return 0
    if e.get("type") != "result":
        return 0
    try:
        return int(e.get("api_error_status") or 0)
    except (TypeError, ValueError):
        return 0


def overload_wait(n):
    """How long to wait before the nth consecutive re-run of an overloaded session."""
    return OVERLOAD_BACKOFF[min(max(n, 1), len(OVERLOAD_BACKOFF)) - 1]


def mark_interrupted(index, why=None):
    """Record that a session was cut off with work still in the tree. Returns the path count.

    A session stopped mid-slice has committed everything it FINISHED -- one commit per slice is
    what buys that -- but whatever it was in the middle of is still uncommitted, and the handoff
    it never reached does not mention it. Without this the next session finds those files and has
    no way to tell them from the state it was supposed to start in. `orient.py` reads this file
    and says so at the top of the pack; the next session to leave a clean tree deletes it.

    `why` is a `RateLimit`, a sentence, or nothing at all -- the three things that cut a session
    off, in the order the driver can explain them."""
    dirty = [ln for ln in git("status", "--porcelain").split("\n") if ln.strip()]
    if not dirty:
        INTERRUPTED.unlink(missing_ok=True)
        return 0
    try:
        INTERRUPTED.write_text(
            json.dumps({"session": index, "when": f"{datetime.now():%Y-%m-%d %H:%M:%S}",
                        "why": (why.describe() if isinstance(why, RateLimit)
                                else why or "the CLI exited non-zero"),
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
    # Only when it was asked for: the flag and the model's own default are not the same thing to
    # the harness, and passing `--effort high` would record a choice where none was made.
    if opts.effort:
        cmd += ["--effort", opts.effort]
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
    SLICES.pick(session_goal(pack))
    session_id = ""
    limit = None  # the last `rate_limit_event` this session reported; see `RateLimit`
    said_limit = False  # the text fallback, read only off a non-zero exit's `result` event
    api_error = 0  # the HTTP status its terminal `result` event blamed; see `api_error_status`
    # The pack's size, recorded beside the transcript that paid for it. Two sessions with
    # different pack sizes are a two-point regression against their measured `ctx_start`,
    # which is how `loop-stats.py --calibrate` derives bytes-per-token instead of assuming
    # it. Nothing downstream needs this line; every reader skips a `type` it does not know.
    CONSOLE.raw(json.dumps({"type": "loop_pack", "bytes": len(pack.encode("utf-8"))}) + "\n")
    effort = f", --effort {opts.effort}" if opts.effort else ""
    step(f"launching {exe} (--model {opts.model}{effort}, "
         f"--permission-mode {opts.permission_mode})")
    TICKER.set(phase="launching", detail=f"{exe} --model {opts.model}{effort}")
    launched = time.monotonic()
    VERIFY.arm()
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
            elif '"type":"result"' in line:
                # The terminal event only. `"type":"tool_result"` does not match this, which is
                # the point: a session that read this very file would otherwise supply the words.
                if LIMIT_TEXT.search(line):
                    said_limit = True
                api_error = api_error_status(line) or api_error
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
    finally:
        VERIFY.disarm()
    if proc.returncode and said_limit and not (limit and limit.blocked):
        # A limit reported in prose, with no event carrying a deadline. Come back shortly rather
        # than ending the run: the next session's own event will carry the real one.
        step(f"a usage limit was reported in text but no event named a reset -- treating it as a "
             f"wall and coming back in {hms(BLIND_WAIT)}", C.YELLOW)
        limit = RateLimit({"status": "rejected", "resetsAt": int(time.time()) + BLIND_WAIT})
    return proc.returncode, log, session_id, limit, api_error


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
    freed = disk.prune_logs(opts.keep_runs) + disk.prune_scratch()
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
        f"sessions: up to {opts.max_sessions}, model {opts.model}"
        f"{f', effort {opts.effort}' if opts.effort else ''}\n",
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
    ap.add_argument(
        "--effort", default=None, choices=("low", "medium", "high", "xhigh", "max"),
        help="reasoning effort for every session in the run. Omitted, the harness uses the "
             "model's own default -- `high` for opus-5. The run stamp and the effort are both "
             "in the ledger header, so `loop-stats.py --run <stamp>` prices one setting against "
             "another. Relative token cost on opus-5: low 0.67, medium 0.76, high 1, xhigh 1.6, "
             "max 1.7 -- and roughly a fifth of a session's ending context is thinking, so this "
             "moves the context ceiling as well as the bill"
    )
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
        "--chain", metavar="CHAIN_TOML",
        help="walk a sequence of staged goals: on GOAL REACHED, carry the floor into the next one "
             "with goal-switch.py, install it, and keep going. docs/agent/goals/chain.toml is the "
             "parity program's. Without this the run stops at the first goal that goes green."
    )
    ap.add_argument(
        "--chain-install", action="store_true",
        help="with --chain: install the next staged goal and exit, without running a session. This "
             "is the switch on its own -- run it when you want the new goal live before deciding "
             "when to start the run, and `--chain` then resumes at it rather than installing again."
    )
    ap.add_argument(
        "--force", action="store_true", help="start even if .loop/running says a driver is up"
    )
    ap.add_argument(
        "--min-free-gb", type=float, default=disk.MIN_FREE_GB,
        help="refuse to start below this much free disk; 0 disables the check"
    )
    ap.add_argument(
        "--keep-runs", type=int, default=disk.KEEP_RUNS, metavar="N",
        help="how many runs' session logs survive the prune at start-up. Raise it when a "
             "supervisor is splitting one long run into legs: the default keeps N *runs*, and "
             "legs of 25 sessions would otherwise leave loop-stats.py an eighth of the "
             "transcripts it had"
    )
    opts = ap.parse_args()

    if opts.full_output:
        opts.max_result_lines = opts.max_input_lines = opts.max_line_chars = 0

    enable_ansi()

    for f in (PROMPT, GOAL_MD, GOAL_TOML):
        if not f.exists():
            say(f"missing {f}", C.RED)
            return 2

    # Refusing here is the point of the preflight: a run starts with a build and ends hours later,
    # and a spec the driver cannot run is worth one line now rather than a traceback then.
    try:
        goal = load_goal()
    except (tomllib.TOMLDecodeError, GoalError) as e:
        say(f"{rel_to_root(GOAL_TOML)}: {e}", C.RED)
        return 2

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
            driver = "nvs" if c["kind"] == "nvs-suite" else "cargo"
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
    TICKER.set(loop_goal=goal_title())

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

    # The chain, if there is one. Built before `make_room` so a chain file with a typo in it costs
    # a line rather than a log prune, and before `claim_run` so it cannot leave `.loop/running`
    # behind on a refusal.
    chain = None
    if opts.chain_install and not opts.chain:
        say("--chain-install needs --chain to say which chain to install from", C.RED)
        return 2
    if opts.chain:
        try:
            chain = Chain(opts.chain)
        except ChainError as e:
            say(str(e), C.RED)
            return 2
        if opts.chain_install:
            # The switch on its own. Deliberately separate from starting a run: installing a goal is
            # a change to the repository that gets committed, and deciding when to spend three
            # hundred sessions against it is a different decision made at a different moment.
            if chain.finished:
                say(f"chain: {chain.current['name']} is the last goal in "
                    f"{rel_to_root(chain.path)} and is already installed", C.YELLOW)
                return 0
            fail = chain.install_next()
            if fail:
                say(fail, C.RED)
                return 2
            say("")
            say(f"chain: `python tools/loop.py --chain {opts.chain} --max-sessions <n>` "
                f"resumes here", C.CYAN)
            return 0
        if chain.index < 0:
            # Nothing installed yet: entry 0 takes its floor from whatever goal the repository is
            # currently running, which is what the first switch is for. Everything after it takes
            # its floor from the entry before.
            fail = chain.install_next()
            if fail:
                say(fail, C.RED)
                return 2
            try:
                goal = load_goal()
            except (tomllib.TOMLDecodeError, GoalError) as e:
                say(f"{rel_to_root(GOAL_TOML)}: {e}", C.RED)
                return 2
        else:
            say(f"chain: resuming at goal {chain.index + 1} of {len(chain.goals)} -- "
                f"{chain.current['name']}", C.CYAN)
            fail = preflight(chain.current.get("preflight"))
            if fail:
                say(fail, C.RED)
                return 2
        fail = chain.bring_up_services()
        if fail:
            say(fail, C.RED)
            return 2

    # Set again here rather than only above: a chain that just installed its first goal rewrote
    # `loop-goal.md` after the first read, and this is the first moment the row can name it.
    TICKER.set(loop_goal=goal_title(chain),
               phase="making room", detail="pruning earlier runs' logs and scratch")
    if not make_room(opts):
        return 2
    if not claim_run(opts):
        return 2
    CONTROL.enable()
    try:
        drive(opts, goal, chain)
    except KeyboardInterrupt:
        say("")
        say(
            "interrupted -- the working tree is still consistent, because every session commits "
            "before it exits",
            C.YELLOW,
        )
        ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- interrupted (Ctrl-C)")
        # With the count, not a bare verdict: the supervisor adds a leg's `served` to the sessions
        # since the last optimization pass, and a leg cut short by Ctrl-C still served them.
        write_run_end("interrupted", "interrupted (Ctrl-C)",
                      PROGRESS["served"], PROGRESS["run_id"])
    finally:
        release_run()
    return 0


#: How many sessions run between two `verify.py --doc` gates, and the only home for that number.
#: The rustdoc gate measured 41.8s over the 72 sessions in `.loop/logs` -- 40% of a green
#: verification, ~85s a session, 7% of the loop's whole wall clock -- for a lint whose inputs are
#: doc comments and which `.github/workflows/ci.yml` runs on every push regardless. `verify.py`'s
#: *Why `doc` is a periodic gate* owns that argument. At ten it costs the loop about four seconds
#: a session; raising it trades a longer blind window for very little more.
DOC_GATE_EVERY = 5

#: How many sessions run between two runs of the release-profile checks, and the only home for
#: that number. The release profile is `lto = "thin"` with `codegen-units = 1` and it is the
#: acceptance check's critical path, not a step in it: measured on the 20260904-143054 run, the
#: sweep spent **84s of 288s** waiting at `join_prebuild` while everything else fitted underneath
#: the build. Its only consumers are cost-class assertions -- the abi-probe perf guards and the
#: CLI warm-start bench -- so a stale verdict is a latency regression (priority 3) and never a
#: wrong answer, it does not compound the way a leak or a semantics bug does, and the blind window
#: is at most this many sessions of one commit per slice. `Goal.release_catch_up` closes the one
#: hole that would matter, by refusing to let a goal be declared reached on a gated sweep.
RELEASE_GATE_EVERY = 5


def read_counter(path, every):
    """Sessions since a periodic gate last fired. An unreadable file fires it rather than skipping
    it, which is `inputs_id`'s rule and `verify.py`'s: the safe direction is doing the work."""
    try:
        return int(json.loads(path.read_text(encoding="utf-8")).get("since", 0))
    except (OSError, ValueError, TypeError):
        return every


def write_counter(path, since):
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"since": since, "when": time.time()}, indent=1),
                        encoding="utf-8", newline="\n")
    except OSError:
        pass


def read_last_fail():
    """The name of the `cargo-named` check the last sweep died at, for `Goal.fast_fail`."""
    try:
        name = json.loads(LASTFAIL.read_text(encoding="utf-8")).get("name", "")
        return name if isinstance(name, str) else ""
    except (OSError, ValueError, TypeError):
        return ""


def write_last_fail(name):
    """Record it, or clear it. Clearing on green matters as much as writing on red: a stale name
    would send every later sweep through a fast path that cannot fail."""
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        LASTFAIL.write_text(json.dumps({"name": name or "", "when": time.time()}, indent=1),
                            encoding="utf-8", newline="\n")
    except OSError:
        pass


def write_doc_gate(since, failed=None, session=""):
    """`.loop/doc-gate.json`: sessions since the last gate, and its standing verdict.

    A file rather than a ledger line because `orient.py` needs the *current* state -- a ledger
    holds every verdict a run ever wrote, and the newest `doc gate:` line in it stays red forever
    once one has been written."""
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        DOCGATE.write_text(
            json.dumps({"since": since, "when": time.time(),
                        "failed": failed or "", "session": session}, indent=1),
            encoding="utf-8", newline="\n")
    except OSError:
        pass


def doc_gate(index):
    """`verify.py --doc` every `DOC_GATE_EVERY` sessions, and after every session while it is red.

    Between sessions is where this belongs: the seconds are the driver's, beside the acceptance
    check, rather than inside a session's context ceiling. The counter persists across runs
    because a run that ends at its fourth session would never reach a gate keyed on the session
    index, and runs end early all the time.

    A red gate does not reset the counter, so it runs again after the next session and the one
    after that until it is green. Nothing here fails a run or stops one: a broken intra-doc link
    is not a broken tree, and `verify.py` judges the tree. Reaching a session is `orient.py`'s
    job, off the file this writes."""
    since = read_counter(DOCGATE, DOC_GATE_EVERY) + 1
    if since < DOC_GATE_EVERY:
        write_doc_gate(since)
        return
    step(f"rustdoc gate: every link in a doc comment, resolved "
         f"(1 session in {DOC_GATE_EVERY})", C.CYAN)
    began = time.monotonic()
    r = capture(sys.executable, ["tools/verify.py", "--doc"], timeout=900)
    spent = mmss(time.monotonic() - began)
    if r.code == 0:
        step(f"rustdoc gate green in {spent}", C.CYAN)
        write_doc_gate(0)
        return
    text = ((r.out or "") + "\n" + (r.err or "")).replace("\r\n", "\n")
    first = next((ln.strip() for ln in text.split("\n") if ln.strip().startswith("error")), "")
    why = first or f"`python tools/verify.py --doc` exited {r.code}"
    step(f"rustdoc gate FAILED in {spent} -- {why}", C.RED)
    write_doc_gate(since, failed=why, session=f"{index:04d}")
    ledger(f"       doc gate: {why}")


def drive(opts, goal, chain=None):
    """The session loop itself. Split out so `main` can hold the `.loop/running` marker across it,
    and drop it on any exit -- a normal stop, a Ctrl-C, or an exception."""
    prompt_text = PROMPT.read_text(encoding="utf-8")
    renderer = Renderer(opts)

    stalls = 0
    fails = 0
    reason = f"hit --max-sessions ({opts.max_sessions})"
    kind = "budget"
    run_id = f"{datetime.now():%Y%m%d-%H%M%S}"
    # Cleared at the start, not only written at the end: a driver killed mid-run leaves the last
    # run's verdict on disk, and a supervisor reading that would restart on a stale `budget`.
    RUNEND.unlink(missing_ok=True)
    CONSOLE.open_run(LOGDIR / f"{run_id}-console.log")
    ledger("")
    # The effort is in the header, not on each session line: it is a property of the run, and this
    # is the row that maps a run stamp onto a setting -- which is the whole of what an A/B between
    # two settings needs, since `loop-stats.py --run <stamp>` prices a run.
    effort = f", effort {opts.effort}" if opts.effort else ""
    ledger(f"## run started {datetime.now():%Y-%m-%d %H:%M} "
           f"(max {opts.max_sessions}{effort}, logs {run_id}-*)")
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
    # to stop it spending a call on `ls -la target/debug/nvs.exe` and a defensive `cargo build`
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
    PROGRESS.update(served=0, run_id=run_id)
    walls = 0
    overloads = 0  # consecutive sessions the API refused as overloaded; see `OVERLOAD_STATUS`
    overloaded_since = 0.0  # monotonic, when the current overload streak began
    wall = standing_limit()  # left standing by a driver killed or rebooted during one
    if wall:
        step(f"{rel_to_root(LIMIT)} says {wall.describe()}", C.YELLOW)

    while served < opts.max_sessions:
        asked = CONTROL.stop_reason()
        if asked:
            reason, kind = asked, "asked"
            break

        if wall:
            # Neither a failure nor a stall: no retry can help, nothing is wrong with the tree, and
            # the account has already said when it will answer again. Sleep until then, then run
            # the session it refused.
            stop = wait_out_limit(wall, opts)
            if stop:
                reason, kind = stop, "wall-timeout"
                break
            wall = None

        index += 1
        SLICES.start(git("rev-parse", "HEAD"))
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
        cli_exit, log, session_id, limit, api_error = run_session(
            run_id, index, prompt_text, opts, renderer)
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
                kind = "wall"
                break
            wall = limit
            continue

        # Overload is the other exit that is not a crash, and it is judged before the exit code
        # for the same reason the wall is: it explains it. The server is busy, there is no
        # deadline to sleep to and nothing in the tree to fix, so the session is simply run again
        # -- for as long as it takes, costing neither a `--max-sessions` slot nor a retry.
        if cli_exit != 0 and api_error in OVERLOAD_STATUS:
            overloads += 1
            if overloads == 1:
                overloaded_since = session_started
            back = overload_wait(overloads)
            open_paths = mark_interrupted(index, f"the API answered {api_error} Overloaded")
            ledger(f"- {index:04d} refused by the API -- {api_error} Overloaded"
                   + (f"; {open_paths} path(s) left uncommitted" if open_paths
                      else "; the tree is clean")
                   + f"; overloaded {hms(time.monotonic() - overloaded_since)} so far, retry "
                   f"{overloads + 1} in {hms(back)}"
                   f" -- see {log.relative_to(ROOT).as_posix()}")
            step(f"the API answered {api_error} Overloaded -- the server is busy, not the account, "
                 f"so this session is re-run rather than counted as a crash. Retry {overloads + 1} "
                 f"in {hms(back)}, and there is no cap on how many", C.YELLOW)
            TICKER.set(phase="waiting out an API overload",
                       detail=f"{api_error} Overloaded, {overloads} in a row")
            # Parked, so `r` has something to end: a person watching the status page come back
            # should not have to sit out the rest of a ten-minute backoff to act on it.
            RETRY.unlink(missing_ok=True)
            CONTROL.parked = True
            try:
                wait(back, f"{api_error} Overloaded, retrying", until=CONTROL.pending)
                if CONTROL.take_retry():
                    step("retrying now at your request -- the backoff is dropped", C.GREEN)
            finally:
                CONTROL.parked = False
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
                kind = "cli-failed"
                break
            backoff = min(300, 30 * 2**fails)
            step(f"backing off {mmss(backoff)} before retry {fails + 1}", C.YELLOW)
            TICKER.set(phase=f"backing off before retry {fails + 1}")
            wait(backoff, "claude exited non-zero")
            continue
        fails = 0
        walls = 0
        overloads = 0
        served += 1
        PROGRESS["served"] = served

        line = STATUS.read_text(encoding="utf-8").strip() if STATUS.exists() else ""
        # The count the goal row has been showing all session, from the same range: the watch
        # takes one last look here, because the final slice is committed by the session's last
        # tool call and nothing after it would have polled.
        commits = SLICES.finish()
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
        except (OSError, tomllib.TOMLDecodeError, GoalError) as e:
            ledger(f"       goal spec: {rel_to_root(GOAL_TOML)} did not parse -- "
                   f"checking against the last good one. {e}")
        # Verbose on purpose, and the one place a run spends minutes without a session running:
        # a native build, every fixture, both suites, the WSL leg and the valgrind sweep. Silent,
        # this read as a driver that had hung after printing the session's status line.
        # The two gates the sweep itself does not own: which check to try first, and whether the
        # release profile is built at all this session. Both are read here rather than inside
        # `Goal` because both are counted in SESSIONS, and a `Goal` is loaded fresh every one.
        goal.fast_path = read_last_fail()
        release_since = read_counter(RELEASEGATE, RELEASE_GATE_EVERY) + 1
        goal.release_gate = release_since >= RELEASE_GATE_EVERY
        held = "" if goal.release_gate else f" (release profile held, 1 session in {RELEASE_GATE_EVERY})"
        step(f"acceptance check: build, fixtures, suites, wsl leg, valgrind{held}", C.CYAN)
        checked = time.monotonic()
        fail = goal.check(verbose=True)
        # A green sweep is the end of a goal, and a goal must not be reached on one that skipped a
        # cost guard. `release_catch_up` says why this is the whole of that argument.
        if not fail and goal.release_owed:
            fail = goal.release_catch_up(verbose=True)
        write_counter(RELEASEGATE, 0 if goal.release_gate else release_since)
        write_last_fail(goal.failed_name)
        step(f"acceptance check done in {mmss(time.monotonic() - checked)}", C.CYAN)
        ledger(f"       goal cost: {goal.summary()}")
        # Beside the acceptance check because it is the same kind of thing: a gate the driver runs
        # between sessions, over the tree the session left, reported through a file a pack reads.
        doc_gate(index)
        # The verdict on session `i` is the last thing that belongs in session `i`'s log.
        CONSOLE.close_session()
        if not fail:
            if chain is None:
                reason = "GOAL REACHED: every acceptance check in docs/agent/loop-goal.toml passes"
                kind = "goal"
                break
            done = chain.current["name"]
            ledger(f"## goal reached: {done} -- every check in its acceptance list passes")
            say(f"GOAL REACHED: {done}", C.GREEN)
            # The goal that just passed may have been the one that writes the rest of the chain.
            grew = chain.refresh()
            if grew:
                say(grew, C.CYAN)
                ledger(f"## {grew}")
            if chain.finished:
                reason = (f"CHAIN COMPLETE: {done} was the last goal in "
                          f"{rel_to_root(chain.path)}, and every one of them is green")
                kind = "chain-complete"
                break
            switch = chain.install_next()
            if switch:
                reason, kind = switch, "chain-error"
                break
            switch = chain.bring_up_services()
            if switch:
                reason, kind = switch, "chain-error"
                break
            ledger(f"## run continues on {chain.current['name']} "
                   f"(goal {chain.index + 1} of {len(chain.goals)})")
            # A new goal is a new worklist, so a stall streak from the old one says nothing about
            # it -- and the first session of any goal is the one most likely to spend itself
            # reading rather than committing.
            stalls = 0
            try:
                goal = load_goal()
            except (tomllib.TOMLDecodeError, GoalError) as e:
                reason = f"chain: {rel_to_root(GOAL_TOML)} did not load after the switch -- {e}"
                kind = "chain-error"
                break
            TICKER.set(loop_goal=goal_title(chain))
            continue
        ledger(f"       goal check: {fail}")

        if line.startswith("DONE"):
            reason = f"session reported DONE but the acceptance test does not pass yet: {line}"
            kind = "done-claim"
            break
        if line.startswith("BLOCKED"):
            reason = f"blocked on a user decision: {line}"
            kind = "blocked"
            break

        if commits == 0:
            stalls += 1
            if stalls >= opts.max_stalls:
                reason = f"{stalls} sessions in a row produced no commit"
                kind = "stalled"
                break
        else:
            stalls = 0

        if opts.delay_seconds:
            step(f"--delay-seconds: waiting {mmss(opts.delay_seconds)} before the next session")
            TICKER.set(phase="waiting")
            wait(opts.delay_seconds, "--delay-seconds")

    ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- {reason}")
    write_run_end(kind, reason, served, run_id)
    say("")
    say(reason, C.YELLOW)


if __name__ == "__main__":
    sys.exit(main())
