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
    python tools/loop.py --no-optimize                      # sessions only, never a pass
    python tools/loop.py --optimize-only                    # one optimization pass now, then exit

**A run is a sequence of turns, and a turn is a process.** One turn serves one session -- and whatever
the boundary behind it holds: a hold, a checkpoint, an optimization pass -- and then exits asking to be
started again. `tools/respawn.py` is what starts it again, and is the only process that lives as long
as the run: it holds no logic at all, so every line of this file is read fresh off disk at every
session, and an edit to the driver -- a session's or a person's -- is live at the next one. Nothing
about a run is kept in a process for that reason: what one turn knows that the next one needs is in
`.loop/run.json`. § *the run*, at the foot of this file, is the only home for what a turn decides.

Nothing on the stop path depends on a model's self-assessment: every acceptance item is an exit code plus
an exact or ordered-substring match on real output.
"""

from __future__ import annotations

import _thread
import argparse
import collections
import contextlib
import hashlib
import json
import os
import platform
import re
import shlex
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
import goals as goalsmod  # noqa: E402  -- same directory; the chain has one reader and it is there
import impact  # noqa: E402  -- same directory; what each test binary reads, and its key
import machine  # noqa: E402  -- same directory; how wide anything runs has one home too
import proctree  # noqa: E402  -- same directory; a session's whole process tree, frozen and thawed
import relink  # noqa: E402  -- same directory; freeing the release binary an editor is running
import respawn  # noqa: E402  -- same directory; the process that starts this file again
import side as sidemod  # noqa: E402  -- same directory; what a side run is, and how it lands
import verify_keys  # noqa: E402  -- same directory; how much of a `.rs` file a reader reads
import written  # noqa: E402  -- same directory; how a tool reports what it wrote, and why

ROOT = Path(__file__).resolve().parent.parent
PROMPT = ROOT / "docs" / "agent" / "session-prompt.md"
GOAL_MD = ROOT / "docs" / "agent" / "loop-goal.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"
#: The side goal this run works on, or None for a chain run. A side run reads that goal's own
#: `.md` and `.toml` where a chain run reads the installed pair, so every reader below follows it
#: without knowing; `tools/side.py` is what a side run is and how it lands.
SIDE = goalsmod.side_goal()
if SIDE:
    GOAL_MD, GOAL_TOML = SIDE.md, SIDE.toml
#: The chain, and there is only one: the goals directory itself, walked in numeric order. Not a
#: flag and never was two -- every goal this project has lives there, and a run walking no chain is
#: a run that stops at the first goal to go green and waits for a person, the same program with an
#: extra night in it per goal. `tools/goals.py` reads it, `tools/chain.py` edits it, `Chain` walks
#: it.
GOALS_DIR = goalsmod.GOALS
RUNDIR = ROOT / ".loop"
LOGDIR = RUNDIR / "logs"
LEDGER = RUNDIR / "log.md"
STATUS = RUNDIR / "status.txt"
STOP = RUNDIR / "stop"
PAUSE = RUNDIR / "pause"
RETRY = RUNDIR / "retry"
#: Present while the running session is frozen where it stands; `h` writes and deletes it.
HALT = RUNDIR / "halt"
#: Text for the running session: delivered as a user message, and deleted. `i` without a console.
SAY = RUNDIR / "say"
RUNNING = RUNDIR / "running"
GOALCACHE = RUNDIR / "goal-green.json"
LIMIT = RUNDIR / "limit.json"
INTERRUPTED = RUNDIR / "interrupted.json"
#: Paths the session's own tools reported writing, appended by `written.py` and truncated before
#: every session. Read by `SessionFiles`, which is where what it is for is written down.
WRITTEN = RUNDIR / "written.txt"
CHAINSTATE = RUNDIR / "chain.json"
#: What one turn of a run leaves for the next. `Run` is what it holds and why.
RUNSTATE = RUNDIR / "run.json"
DOCGATE = RUNDIR / "doc-gate.json"
OWNERGATE = RUNDIR / "owner-gate.json"
FLOORGATE = RUNDIR / "floor-gate.json"
LASTFAIL = RUNDIR / "last-fail.json"
#: What each Python gate was last seen to read, by its command line: `tools/observe.py` writes one
#: record per run under `READSDIR`, and `Goal.observed_inputs` is what reads them.
CHECKREADS = RUNDIR / "check-reads.json"
READSDIR = RUNDIR / "reads"
#: Each proofs group's own paths, as `bun nv proofs` last recorded them (`PROOF_READS` in
#: `tools/nv/keys/checks.ts`); `Goal.proof_inputs` keys a proofs check on it.
PROOF_READS = RUNDIR / "proof-reads.json"
#: What every check of every sweep cost, one NDJSON line each; `Goal.write_times` is the writer.
CHECKTIMES = RUNDIR / "check-times.ndjson"
#: Past this many bytes the older half of `CHECKTIMES` is dropped, so the file stays bounded.
CHECKTIMES_CAP = 8 * 1024 * 1024

# The optimization pass's. § *the run* at the foot of this file is what they are for.
OPTDIR = RUNDIR / "optimization"
OPTSTATE = OPTDIR / "state.json"
OPTSTATUS = RUNDIR / "optimize-status.txt"
PACKLOG = RUNDIR / "pack-size.jsonl"
OPT_PROMPT = ROOT / "docs" / "agent" / "optimization-prompt.md"
REPAIR_PROMPT = ROOT / "docs" / "agent" / "repair-prompt.md"

IS_WINDOWS = os.name == "nt"

#: `--max-sessions` when it is not capped, which is the default. A very large number rather than
#: `inf` so that every count, subtraction and comparison over it stays integer arithmetic and
#: prints as one; a run that really served a billion sessions is not a case worth a second code
#: path. A run does not need a cap: it ends when the chain is walked, when something goes wrong,
#: or when it is told to, and none of those is a number anybody could have guessed at the start.
UNCAPPED = 1_000_000_000


def sessions_label(n):
    """`--max-sessions` as a person reads it: the number, or `uncapped`."""
    return "uncapped" if n >= UNCAPPED else str(n)

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

    Five fields, in the order they narrow:

        scope    where the run is       `session 3/12`
        phase    what it is doing now   `acceptance check`, with `31/58 53%` when that is countable
        tokens   the session's context  `ctx 84.2k in / 6.1k out`, while one is running
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
        self.tokens = ""  # `ctx 84.2k in / 6.1k out` while a session runs; see `Renderer.count`
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
        # Cleared, not merely unset: a run stops the ticker for the length of every leg -- the row
        # belongs to whichever process owns the console -- and a `stop()` that left the event set
        # would hand the next `start()` a thread that returned on its first wait, silently.
        self._stop.clear()
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
                self.tokens = ""
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

    def usage(self, text):
        """The running session's context tokens, in and out. Painted by the next tick: this is
        called once per event of a stream that `tool` and `note` already draw for."""
        with self.lock:
            self.tokens = text

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
        session = CONTROL.session
        halted = (f"HALTED {hms(time.monotonic() - session.frozen_at)}"
                  if session and session.frozen_for else "")
        parts = [p for p in (halted, self.scope, head, self.tokens, self.detail, self.verifying)
                 if p]
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
        if CONTROL.typing is not None:
            # The typed text ends the row, because this row is the last one `draw` paints and the
            # terminal's own cursor stands where painting stopped: right after the last character
            # typed. Shown by its tail when it outgrows the row, since the tail is being typed.
            head = f"Enter sends, Esc cancels{self.sep}prompt> "
            room = max(18, self.width() - 3 - len(head))
            return "  " + C.paint(head + CONTROL.typing[-room:], C.YELLOW)
        bits = []
        session = CONTROL.session
        if session and session.frozen_for:
            bits.append(f"[h] HALTED{self.dash}press h to carry on")
        elif session:
            bits.append("[h] halt now")
        if session and session.streaming and not session.frozen_for:
            bits.append("[i] type a prompt")
        if CONTROL.parked:
            bits.append("[r] retry now")
        if CONTROL.stop:
            grace = CONTROL.stop_in()
            bits.append(f"[s] stopping in {grace:.0f}s{self.dash}press s to cancel" if grace > 0
                        else f"[s] stopping after this session{self.dash}press s to cancel")
        else:
            bits.append("[s] stop after this session")
        if CONTROL.held:
            bits.append(f"[p] held{self.dash}press p to carry on")
        elif CONTROL.pause_by:
            bits.append(f"[p] holding after this session{self.dash}press p to cancel")
        else:
            bits.append("[p] hold after this session")
        body = self.sep.join(bits)
        room = max(18, self.width() - 3)
        if len(body) > room:
            body = body[: room - len(self.cut)] + self.cut
        loud = CONTROL.stop or CONTROL.pause_by or (session and session.frozen_for)
        return "  " + C.paint(body, C.YELLOW if loud else C.GRAY)

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
    """`r`, `s`, `p`, `h` and `i`, from the console or from `.loop/`.

    `h` and `i` act on the session in flight and on nothing else: `h` freezes it where it stands
    and thaws it again, `i` sends it a prompt. `Session` is what they do and why; `.loop/halt`
    and `.loop/say` are the same two things from another terminal. The rest of this is the three
    keys that act on the RUN.

    A run parked behind a usage wall is waiting on a clock, and the one thing that clock cannot
    know is that the account behind it has changed. Logging in somewhere else is not something
    this driver takes part in, so the wait is interruptible: **`r` retries now**, and **`s` stops
    the run** after the current session, exactly as `.loop/stop` does. **`p` holds** at that same
    boundary without ending the run, so a person or another agent can have the tree for a while
    and hand it back.

    Four rules the shape follows, each of which is a mistake it would otherwise make:

    * **`r` exists only while a wall is up** -- a usage window that has closed, or an API that
      answered `529 Overloaded`. It has nothing to end at any other time, and a key that silently
      does nothing is a key that gets pressed twice and then distrusted.
    * **`s` is undoable.** Pressed by accident it would otherwise cost the rest of a run, so it
      toggles, and it is acted on `STOP_GRACE` seconds late -- long enough that even a parked run,
      which reads the flag four times a second, can be told to carry on.
    * **A file does everything a key does.** A keypress needs a terminal, and an overnight run is
      often started with its output redirected, where there is none; `.loop/retry`, `.loop/stop`
      and `.loop/pause` work from another terminal, over SSH and under `nohup`.
    * **A hold has an owner, and the console outranks the file.** `.loop/pause` is how another
      agent queues one: create it, wait for the driver to write a `held:` line into it, work, then
      delete it -- and deleting it is what lifts the hold again. `p` at the console arms the same
      hold, but one this process owns: deleting the file does not lift it, the driver writes it
      straight back, and only `p` releases it. The reverse is deliberately allowed -- `p` lifts a
      hold an agent queued and then forgot, because the person watching a stalled run needs one
      key that always works. There is no grace window on `p` the way there is on `s`: a hold
      pressed by accident costs the keypress that undoes it and nothing else.

    Read from the ticker thread, which is awake eight times a second anyway, AND from `wait()` in
    the main thread -- so the keys still work under `--no-status`, where there is no ticker. Never
    a blocking read: `kbhit`/`select` answer whether anything has been typed, and nothing is taken
    off stdin that was not. The child gets its own stdin pipe (see `run_session`) precisely so that
    this console belongs to the driver alone.
    """

    #: How long an armed stop waits before it is acted on. It is the cancel window, so it is
    #: measured in "noticed the wrong key and pressed it again", not in machine time.
    STOP_GRACE = 5.0

    #: How often `.loop/pause` is looked at. The ticker calls `poll` eight times a second and the
    #: file is a request rather than a deadline: half a second late is invisible to whoever made
    #: it, and it keeps a five-hour run from stat'ing the same path 140,000 times.
    PAUSE_POLL = 0.5

    def __init__(self):
        self.lock = threading.RLock()
        self.stop = False
        self.stop_at = 0.0  # monotonic; before this, an armed stop is still cancellable
        self.retry = False
        self.parked = False  # a wall is up, so `r` has something to end
        self.pause_by = ""  # "user", "agent" or "": who owns the hold armed for the next boundary
        self.pause_why = ""  # what the hold is for, when the driver armed it rather than a person
        self.held = False  # the driver is sitting in `hold_pause` right now, not merely armed
        self._pause_seen = 0.0  # monotonic of the last look at the file; see `_sync_pause`
        self.session = None  # the `Session` in flight, which is what `h` and `i` act on
        self.typing = None  # the prompt being typed after `i`, or None when keys are commands
        self._session_seen = 0.0  # monotonic of the last look at `HALT` and `SAY`
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
                if self.typing is not None:
                    self._type(ch)
                    continue
                key = ch.lower()
                if key == "h":
                    self._toggle_halt()
                elif key == "i":
                    self._begin_typing()
                elif key == "s":
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
                elif key == "p":
                    self._toggle_pause()
            # Here rather than in `pause_reason` alone, so the key row and the status line show a
            # hold another agent queued *while a session is still running* -- which is the whole
            # of the interval that agent is waiting through.
            self._sync_pause()
            self._sync_session()

    # -- the running session: halt it, talk to it ---------------------------------------

    def attach(self, session):
        """A session is in flight, so `h` and `i` have something to act on. A halt file left by
        anything earlier is not this session's, and would freeze it on its first breath."""
        with self.lock:
            HALT.unlink(missing_ok=True)
            self.session = session
            self._session_seen = 0.0

    def detach(self):
        with self.lock:
            self.session = None
            self.typing = None
            HALT.unlink(missing_ok=True)

    def _toggle_halt(self):
        """`h`. The file is the state, so the key and `.loop/halt` can never disagree: the key
        writes or deletes it, and `_sync_session` is what freezes and thaws."""
        if not self.session:
            say("   [h] does nothing right now; it halts a running session", C.GRAY)
            return
        if HALT.exists():
            HALT.unlink(missing_ok=True)
        else:
            HALT.parent.mkdir(parents=True, exist_ok=True)
            HALT.write_text(
                "The running session is frozen where it stands, with every process under it.\n"
                "Delete this file, or press h at the console, to let it carry on.\n",
                encoding="utf-8", newline="\n")
        self._session_seen = 0.0
        self._sync_session()

    def _sync_session(self):
        """Make the session match `.loop/halt`, and deliver `.loop/say`. Rate-limited like the
        pause file; a key resets the clock so its own effect is immediate."""
        s = self.session
        now = time.monotonic()
        if not s or now - self._session_seen < self.PAUSE_POLL:
            return
        self._session_seen = now
        want = HALT.exists()
        if want and "halt" not in s.frozen_for:
            count = s.freeze("halt")
            say(f"   [h] HALTED -- {count} process(es) frozen where they stood, nothing lost. "
                f"Press h, or delete {rel_to_root(HALT)}, to carry on", C.YELLOW)
        elif not want and "halt" in s.frozen_for:
            spent = s.thaw("halt")
            say(f"   [h] carrying on after {hms(spent)}", C.GREEN)
        if s.streaming and SAY.exists():
            try:
                text = SAY.read_text(encoding="utf-8", errors="replace").strip()
            except OSError:
                return  # still being written; the next look has it
            SAY.unlink(missing_ok=True)
            if text:
                self._deliver(text, rel_to_root(SAY))

    def _begin_typing(self):
        """`i`. The session is frozen for as long as the prompt is being typed, so nothing
        scrolls under the line and the message lands at the moment it was written for."""
        s = self.session
        if not s:
            say("   [i] does nothing right now; it sends a prompt to a running session", C.GRAY)
            return
        if not s.streaming:
            say("   [i] this session cannot be sent a prompt: it was started with --plain-input, "
                "or its reply is already complete", C.GRAY)
            return
        self.typing = ""
        s.freeze("typing")
        say("   [i] the session is frozen while you type -- Enter sends, Esc cancels", C.YELLOW)

    def _type(self, ch):
        """One key of a prompt. Every key is text here, `s` and `p` included."""
        if ch in ("\r", "\n"):
            text, self.typing = self.typing.strip(), None
            if text and self.session:
                self._deliver(text, "the console")
            else:
                say("   [i] nothing sent", C.GRAY)
            if self.session:
                self.session.thaw("typing")
        elif ch == "\x1b":
            self.typing = None
            say("   [i] cancelled -- nothing sent", C.GRAY)
            if self.session:
                self.session.thaw("typing")
        elif ch in ("\x08", "\x7f"):
            self.typing = self.typing[:-1]
        elif ch.isprintable():
            self.typing += ch

    def _deliver(self, text, via):
        """Send `text` to the session as a user message, and leave a record that it happened:
        a session a person spoke to is not the unattended session the measurements assume."""
        s = self.session
        if not s or not s.send(text):
            say(f"   [i] NOT sent -- the session is no longer taking input: {text}", C.RED)
            return
        CONSOLE.raw(json.dumps({"type": "loop_prompt", "via": via, "text": text}) + "\n")
        say(f"   [i] sent to the session, from {via}: {text}", C.GREEN)

    # -- the hold ----------------------------------------------------------------------

    def _toggle_pause(self):
        """`p`. Arms a hold this console owns, or releases whichever hold stands."""
        if self.pause_by:
            mine = self.pause_by == "user"
            drivers = bool(self.pause_why)
            self.pause_by = ""
            self.pause_why = ""
            self.held = False
            PAUSE.unlink(missing_ok=True)
            say("   [p] carrying on -- the hold is lifted" if mine else
                "   [p] carrying on -- the driver's hold is lifted and the run takes another session"
                if drivers else
                f"   [p] carrying on -- {rel_to_root(PAUSE)} was another agent's hold, and the "
                f"console outranks it", C.GREEN)
            return
        self.pause_by = "user"
        self.pause_why = ""
        self._write_pause()
        say(f"   [p] hold requested -- the run stops between sessions and waits. Press p again "
            f"to carry on; deleting {rel_to_root(PAUSE)} will not, because this hold was taken "
            f"at the console", C.YELLOW)

    def arm_hold(self, why):
        """Hold the run because the driver has decided a person is needed, naming the decision.

        An *agent's* hold rather than a console one, deliberately: this one is lifted by deleting
        `.loop/pause` as well as by `p`, and the person it is waiting for may be reading the tree
        over ssh rather than sitting at the terminal it was printed on. A hold already standing is
        left exactly as it is -- whoever armed it owns it, and this is not a reason to take it
        away from them."""
        with self.lock:
            if self.pause_by:
                return
            self.pause_by = "agent"
            self.pause_why = why
            self._write_pause()

    def _write_pause(self):
        """(Re)write `.loop/pause` to say who owns the hold and whether it has taken effect yet.

        The `held:` line is the handshake, and it is the only part of this file that matters to
        anything but a human. A hold is *queued* the moment the file appears and the session in
        flight can run for another twenty minutes after that, so an agent that starts editing on
        the strength of its own `touch` is editing alongside a live session. It waits for
        `held:`."""
        mine = self.pause_by == "user"
        note = ("This hold was taken at the console with `p`, and is lifted there with `p`.\n"
                "Deleting this file does not lift it -- the driver writes it straight back.\n"
                if mine else
                "Delete this file to let the run carry on.\n"
                "Do not edit this tree until the `held:` line above is there: until then the\n"
                "hold is only queued, and the session in flight is still committing to it.\n")
        PAUSE.parent.mkdir(parents=True, exist_ok=True)
        PAUSE.write_text(
            f"by:      {'user (the console)' if mine else 'agent (this file)'}\n"
            f"held:    {f'{datetime.now():%Y-%m-%d %H:%M:%S}' if self.held else '(not yet)'}\n"
            f"pid:     {os.getpid()}\n"
            + (f"why:     {self.pause_why}\n" if self.pause_why else "")
            + f"\n{note}",
            encoding="utf-8",
            newline="\n",
        )

    def _sync_pause(self):
        """Reconcile the armed hold with the file on disk, at most every `PAUSE_POLL` seconds.

        The file is this process's only input from outside it and the flag is the state, so this
        is the one place the two meet. Three transitions, and the asymmetry between the last two
        is the whole ownership rule: a file that appears arms an agent's hold, a file that goes
        lifts one, and a file that goes while the console owns the hold is simply written back."""
        now = time.monotonic()
        if now - self._pause_seen < self.PAUSE_POLL:
            return
        self._pause_seen = now
        there = PAUSE.exists()
        if self.pause_by == "user":
            if not there:
                self._write_pause()
            return
        if there and not self.pause_by:
            # Whose hold it is comes out of the file, not out of the fact that this process was
            # not the one that armed it. Every turn of a run is a new process, so the process
            # adopting a hold is routinely not the process the key was pressed at -- and adopted
            # as an agent's, a `p` hold would be liftable by deleting a file, which is exactly
            # what `p` promises it is not.
            self.pause_by = self._file_owner()
            say(f"   {rel_to_root(PAUSE)} appeared -- the run holds after the current session, "
                f"and carries on when the file goes" if self.pause_by == "agent" else
                "   the hold taken at the console still stands -- press p to carry on",
                C.YELLOW)
        elif not there and self.pause_by == "agent":
            self.pause_by = ""
            self.held = False
            say(f"   {rel_to_root(PAUSE)} is gone -- carrying on", C.GREEN)

    def _file_owner(self):
        """Who armed the hold `.loop/pause` describes: `"user"` for a console one, `"agent"` for
        anything else. The `by:` line `_write_pause` writes, read back.

        An unreadable or hand-made file is an agent's. That is the safe direction: an agent's hold
        is lifted by deleting the file *and* by `p`, so a misread costs nothing, where the reverse
        would leave a hold only one of the two channels could lift."""
        try:
            for line in PAUSE.read_text(encoding="utf-8").splitlines():
                if line.startswith("by:"):
                    return "user" if "user" in line else "agent"
        except OSError:
            pass
        return "agent"

    def pause_reason(self):
        """Why the run should hold here, or "". Read once at each session boundary and four times
        a second inside a hold, so either channel is answered promptly."""
        with self.lock:
            self._sync_pause()
            if self.pause_by == "user":
                return "p was pressed at the console"
            if self.pause_by == "agent":
                return f"{rel_to_root(PAUSE)} present"
            return ""

    def enter_hold(self):
        """The hold has taken effect: no session is running and none will start. This is what
        writes the `held:` line whoever asked for it is waiting on."""
        with self.lock:
            self.held = True
            self._write_pause()

    def leave_hold(self):
        with self.lock:
            self.held = False
            if self.pause_by == "user":
                self._write_pause()

    def drop_pause(self):
        """Called when the run ends. A hold taken at the console belongs to a console that is
        going away with this process, and leaving its file behind would hand the next run a hold
        nobody armed and nobody is watching.

        A hold `arm_hold` took goes the same way, and `pause_why` is how it is told from another
        agent's: `s` during one of those ends the run without ever lifting the hold, and the file
        left behind would silently hold the *next* run at its first boundary. Another agent's file
        is never touched -- they are waiting on it, and this run ending is not their answer."""
        with self.lock:
            if self.pause_by == "user" or self.pause_why:
                PAUSE.unlink(missing_ok=True)
            self.pause_by = ""
            self.pause_why = ""
            self.held = False

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
        spin `wait` in a loop of instant returns.

        A queued hold is deliberately NOT one of these. This is the predicate that ends a usage
        wall's sleep, and a hold does not end early: it stays true until somebody lifts it, so a
        `wait` that returned on it would return instantly, forever. A wall is a wait with nothing
        running, and the hold is taken at the boundary on the far side of it."""
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


def hold_pause():
    """Hold here for as long as a pause stands. Returns "" when it lifts and the run may go on,
    or the reason it should stop instead.

    Deliberately the same boundary `.loop/stop` acts at -- between two sessions, with the last
    slice committed and nothing in flight. That is the entire value of it: an agent that queued a
    hold and has seen `held:` land in the file is looking at a tree no session is editing, which
    is the one thing `.loop/running` on its own cannot promise it. A hold is what `.loop/stop`
    would be if the run could be started again afterwards, and an agent that only needs the tree
    for twenty minutes should not have to end a three-hundred-session run to get it.

    A stop always wins, whichever arrives first: `s` (or `.loop/stop`) during a hold ends the run
    rather than waiting for somebody to come back and release it."""
    stop = CONTROL.stop_reason()
    why = CONTROL.pause_reason()
    if stop or not why:
        return stop
    began = time.monotonic()
    CONTROL.enter_hold()
    step(f"holding before the next session -- {why}; "
         + ("press p to carry on" if CONTROL.pause_by == "user"
            else f"delete {rel_to_root(PAUSE)}, or press p, to carry on"), C.YELLOW)
    TICKER.set(phase="held", detail=why)
    try:
        while True:
            CONTROL.poll()
            stop = CONTROL.stop_reason()
            if stop or not CONTROL.pause_reason():
                break
            TICKER.set(detail=f"{why}{TICKER.sep}held {hms(time.monotonic() - began)}")
            time.sleep(0.25)
    finally:
        CONTROL.leave_hold()
    spent = hms(time.monotonic() - began)
    step(f"held {spent} -- {'stopping' if stop else 'carrying on'}",
         C.YELLOW if stop else C.GREEN)
    ledger(f"       held {spent} -- {why}")
    return stop


def mmss(seconds):
    """`123` -> `2m03s`. Durations here run from milliseconds to tens of minutes, and a bare
    float of seconds is unreadable at the top of that range."""
    seconds = int(seconds)
    return f"{seconds // 60}m{seconds % 60:02d}s" if seconds >= 60 else f"{seconds}s"


def ktok(tokens):
    """`84213` -> `84.2k`. A context is five or six digits and is read at a glance, against a
    ceiling that is itself quoted in thousands."""
    return f"{tokens / 1000:.1f}k" if tokens >= 1000 else str(tokens)


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


def verdict(hand, text):
    """The one line of a session a person has to read: is the loop still driving itself?

    Everything else printed at a session boundary is a wall -- the session's own summary, then the
    acceptance check's, then the goal row -- and the question that actually decides whether someone
    opens the terminal is nowhere in it. So the driver answers it last, in one line, from what it
    has just decided rather than from what the session claimed about itself: a session that reports
    `CONTINUE` and a driver that is stopping on a stall streak disagree, and the driver is right.

    `hand=True` means the run has stopped and will not move again without a person."""
    say("")
    say(f"   ==> {'YOUR HAND IS NEEDED' if hand else 'nothing for you to do'} -- {text}",
        C.RED if hand else C.GREEN)


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
        #: The `init` event carries no effort, so the one this driver passed is what is shown.
        self.effort = opts.effort
        self.tool_names: dict[str, str] = {}
        #: The last assistant text printed. The `result` event repeats the session's final message
        #: verbatim, and printing both is how a wrap-up summary reached the console twice.
        self.said = ""
        self.begin()

    def begin(self):
        """A session is starting: its token counts start with it. See `count`."""
        #: What the session's window held going into its latest turn, in tokens.
        self.context = 0
        #: Output tokens per assistant message id. A message arrives as one event per content
        #: block, each repeating the message's usage, so a sum over events counts a three-block
        #: message three times; the last figure per id, summed, counts it once.
        self.written: dict[str, int] = {}

    def count(self, e):
        """The session's context tokens, off one `assistant` event, onto the status line.

        **In** is the window, not a running total: the input, cache-write and cache-read tokens
        of the latest turn, which together are everything the model was shown for it. It is the
        number `loop-stats.py` reads as a session's context and the one AGENTS.md's ceiling is
        set in. **Out** is what the session has written so far, summed over its messages.

        A subagent's turns arrive in this stream tagged with the call that spawned them, and
        are not the session's window -- it holds that call's one result -- so they are skipped
        for the reason `loop-stats.read_session` skips them."""
        if e.get("parent_tool_use_id"):
            return
        message = e.get("message") or {}
        usage = message.get("usage") or {}
        context = sum(int(usage.get(k) or 0) for k in
                      ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"))
        if context:
            self.context = context
        if usage.get("output_tokens"):
            self.written[str(message.get("id") or len(self.written))] = int(usage["output_tokens"])
        TICKER.usage(self.tokens())

    def tokens(self):
        """`ctx 84.2k in / 6.1k out`, or "" before the first turn has reported."""
        if not self.context:
            return ""
        return f"ctx {ktok(self.context)} in / {ktok(sum(self.written.values()))} out"

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
                # `--model` takes an alias; this is what it resolved to, which CLI build ran it
                # and at what effort -- the facts a run's behaviour changes with when nothing in
                # the tree did.
                say(
                    f"   [init] claude {e.get('claude_code_version') or '?'}, "
                    f"model {e.get('model') or '?'}, "
                    f"effort {self.effort or 'model default'}",
                    C.CYAN,
                )
                say(f"   [init] cwd={e.get('cwd')} session={e.get('session_id')}", C.GRAY)
                if e.get("tools"):
                    self.wrapped("tools: " + ", ".join(e["tools"]), "   [init] ", C.GRAY, 2)
            else:
                self.wrapped(json.dumps(e), f"   [{e.get('subtype')}] ", C.GRAY, 4)

        elif kind == "assistant":
            self.count(e)
            for b in e.get("message", {}).get("content", []) or []:
                btype = b.get("type")
                if btype == "text":
                    TICKER.note("writing")
                    self.said = str(b.get("text") or "")
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
                    # The other thing this block is evidence of: if the session ends without a
                    # wrap, a write here is how the sweep tells its own edit to an already-dirty
                    # file from a person's.
                    TOUCH.note(b.get("name"), b.get("input"))
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
            # `result` carries the session's final message, which the assistant event above has
            # already printed in full. Reprinting it doubled every session's summary on screen and
            # in the console log, and a summary is the longest thing a session writes. What is left
            # is the case the repeat was there for: a subtype that ends a session with text no
            # assistant event carried, an error most of all.
            body = str(e.get("result") or "")
            if body.strip() and body.strip() != self.said.strip():
                self.wrapped(body, "   ", C.YELLOW, self.max_result_lines)


# ------------------------------------------------------------------------------- capture


class Result:
    __slots__ = ("code", "out", "err")

    def __init__(self, code, out, err):
        self.code = code
        self.out = out
        self.err = err

    @property
    def first_err_line(self):
        """The one line that names the failure. Every verdict in this file quotes this.

        A `cargo test` run says which test failed on stdout (`test <name> ... FAILED`, then the
        panic) and only `error: test failed` on stderr, after every warning the build printed; a
        `cargo build` puts its `error[E...]` after those same warnings. So the pick is the most
        specific line either stream holds, in this order: the line naming a failed test, the panic
        line, the first `error` line, then stderr's first line, then -- for a program that reports
        on stdout and exits non-zero with nothing on stderr -- stdout's last. Without the fallbacks
        a run whose whole cause was printed reaches the ledger as a warning, or as a bare
        `exit 1 --`, and the reader goes to the console log to learn what the ledger threw away."""
        err = self.err.strip().splitlines()
        out = self.out.strip().splitlines()
        both = out + err
        for pick in (
            lambda l: l.startswith("test ") and l.endswith("... FAILED"),
            lambda l: l.startswith("thread '") and "panicked at" in l,
            lambda l: l.startswith("error"),
        ):
            for line in both:
                if pick(line):
                    return line
        return (err or [""])[0] or (out or [""])[-1]


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
    # Whether a CLI built here carries `nvs_host::net`'s Unix-domain half, which is `#[cfg(unix)]`
    # and so a fact about the build rather than about the kernel: Windows 10 has `AF_UNIX` sockets
    # and this binary still has no transport for one. `LEG_NEEDS` is what asks.
    af_unix = os.name != "nt"

    def __init__(self):
        self.binary = None

    def prepare(self):
        # The whole workspace, never `-p nvs-cli`: this is `verify.py`'s own `build` step, so on a
        # tree a session just verified it is a fingerprint scan, and a `-p` build resolves features
        # over one package's graph and writes a second copy of every workspace crate that
        # `disk.py`'s live set cannot name -- AGENTS.md's rule.
        r = capture("cargo", ["build", "--quiet"])
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
    # A Linux build, whatever the machine hosting it is.
    af_unix = True

    def __init__(self, target_dir):
        super().__init__()
        drive = str(ROOT)[0].lower()
        self.repo = "/mnt/" + drive + str(ROOT)[2:].replace("\\", "/")
        self.target_dir = target_dir

    def bash(self, inner, timeout=1800):
        return capture("wsl.exe", ["--", "bash", "-lc", inner], timeout=timeout)

    def prepare(self):
        r = self.bash(
            f"cd {self.repo} && CARGO_TARGET_DIR={self.target_dir} cargo build --quiet"
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
# What a `[[check]] needs` may ask of the leg it is about to run on, as the question each one is.
# One entry, and the shape is here for the second: a name rather than a `cfg`-style expression,
# because a driver that grew a language for this would be answering questions the platform can
# answer itself.
LEG_NEEDS = {"af-unix": lambda leg: leg.af_unix}


def is_floor(check):
    """Is this check the PREVIOUS goal's, carried in as a regression floor?

    `goal-switch.py` relabels the whole of the goal that just passed to stage `"1 floor"`, so the
    stage is the only thing that distinguishes hundreds of carried checks from the dozen the
    current goal is actually working on. Stage 0 is the catch-up stage and is a floor in the same
    sense: work a later ADR reopened inside a milestone already reported done."""
    text = str(check.get("stage", ""))
    return "floor" in text.lower() or text.startswith("0")


def is_carried(check):
    """Is this check one `goal-switch.py` carried in from the goal before -- the `"1 floor"`
    stage and nothing else? Stage 0 is a floor to `is_floor` but is this goal's own reopened
    work, and the frontier a session is handed, so the floor gate never holds it."""
    return "floor" in str(check.get("stage", "")).lower()


def stage_key(check):
    """A check's stage as something sortable: its leading integer, then its text.

    Stages are written `"6 the member"`, so the plain string sorts correctly only while the goal
    has fewer than ten of them -- `"10 x"` sorts before `"2 x"`. Goal `schema` already runs to 7 and the
    floor is stage 1, so this is one goal away from mattering. `"0..."` is the catch-up stage and
    sorts first by the same rule. A stage with no leading integer sorts last, by its text, rather
    than raising: an unlabelled check is a goal-file bug for `validate_spec` to name, not a reason
    for the sweep to die."""
    text = str(check.get("stage", ""))
    lead = re.match(r"\s*(\d+)", text)
    return (int(lead.group(1)) if lead else 10**6, text)
SUMMARY_RE = re.compile(r"(\d+)\s+passed,\s+(\d+)\s+failed")

# -- what a check reads, and so what its green verdict is keyed on ------------------------------
#
# Every check's green verdict is remembered across runs against a content hash of the PARTITIONS
# of the tree its kind reads -- `Goal.remembered` -- and a partition is a set of top-level names.
# A check is skipped only when every byte it can read is identical to the bytes it was last green
# over: a deterministic check over identical inputs cannot reach a different verdict, which is
# `verify.py`'s rule for its own green cache. Nothing is keyed on what a session says it touched,
# because a session's diff is not what a check reads, and the dirty tree counts as much as HEAD.
#
# Each set is a SUPERSET on purpose, and the safe direction is always wider. Narrowing a set is
# a claim to be shown, not a knob to turn, and each partition below names the grep that shows it.
#
# `crates` is WHAT THE BINARY IS BUILT FROM: every crate's sources, build script and manifest, the
# workspace manifests and toolchain, and the files outside `crates/*/src` that a source embeds or
# a build script reads -- `LICENSE` and `THIRD-PARTY-LICENSES.txt` (`nvs-cli/src/info.rs`),
# `tools/data/php-builtins.txt` and `docs/spec/02-php-migration.md` (`nvs-stdlib/build.rs`),
# `docs/reference/` (`nvs-cli/src/agent.rs`) and `crates/nvs-stdlib/tests/vectors/`
# (`crates/nvs-stdlib/src/tests/vectors.rs`). The two greps that derive it, to re-run before a
# source starts embedding something new: `grep -rn 'include_str!\|include_bytes!' crates/*/src`
# and `grep -n rerun-if-changed crates/*/build.rs`. A crate's `tests/` and `benches/` directories
# are `crate-tests`: cargo runs them, the binary is not built from them, so a fixture never keys
# on them -- the split that let a test-only text file stale every fixture, the WSL leg and the
# valgrind sweep before it existed.
#
# `crates-code` is `crates` as the BINARY reads it, and is derived rather than a set of names: the
# same files, with each `.rs` file fed as `tools/verify_keys.py`'s *shipped* tier -- its tokens,
# with comments, layout and the body of every inline `#[cfg(test)] mod` removed -- in place of its
# bytes. A fixture, a `.nvst` suite, an `{nvs}` command and the two whole-leg memos run the binary
# and open no Rust source, and the binary is built without `cfg(test)`. So a comment, a re-wrapped
# line or a `#[test]` added to a source file's test module cannot reach their verdict, and each
# used to re-run every one of them, the valgrind sweep and the WSL leg included. The cargo checks
# are what runs such a test, and they key on `crates`. A `.rs` file some source embeds with `include_str!`
# is data and stays bytes there too, which is `verify_keys.py`'s rule and its scan. A crate's tests
# keep `crates`, the bytes: the policy tests read source as text, so for them a comment is an
# input. So do the editor suites, which nobody has shown not to.
#
# `docs/` is three partitions by what under `crates/` opens it. `docs` is `docs/reference/` and
# `docs/spec/`: embedded in the binary (above) and read again by `crates/nvs-cli/tests/agent.rs`,
# `crates/nvs-stdlib/tests/php_names.rs`, `spec_registry_coverage.rs` and `nvs-lsp`'s
# `extension_reference.rs`. `goals` is `docs/agent/goals/`, which `spec_registry_coverage` walks
# for owner tags and which no source embeds. `prose` is every other file under `docs/` -- the
# plan, the decisions, the rules, the perf notes, the process docs -- which nothing under `crates/`
# opens: `grep -rn 'read_to_string\|read_dir\|include_str' crates/ -B3 | grep docs/` is the
# evidence, and a docs-only session used to re-run every cargo check on it.
#
# `tests/` and `benches/` are cut by WHO WALKS THEM, because the proof trees are where most
# sessions write and a new case is an input only to what reads its tree. `conformance`,
# `differential`, `hostile` and `lsp-cases` are the four case trees under `tests/`, and `tests` is
# what is left: `tests/config/`, `tests/db/` and `tests/fmt/`. `bench-members` is
# `benches/members/`, which the bench tools read and nothing cargo runs does -- but for the one
# member `benches/abi-probe/tests/perf_guards.rs` opens, which is `crate-tests` with the rest of
# `benches/`, the cargo packages there. `grep -rn 'members/' crates benches --include=*.rs` is
# the evidence. `TEST_TREES` is all five `tests/` partitions, for a reader that walks `tests/`
# whole.
#
# A fixture reads the binary, its own file and whatever it opens, so a program's set is
# `crates-code`, `docs`, `examples` and `tests`, and the two whole-leg memos read the same things.
# No case tree is in it: `grep -rn 'tests/' examples/` names `tests/db/` from four `.toml` files
# and a case tree from one comment, and no program check's `file` or `args` is outside
# `examples/`. A `.nvst` suite reads that set plus THE TREE ITS DIRECTORY IS IN (`suite_reads`);
# `grep -rhoE 'tests/(hostile|differential|conformance|lsp)/' tests/<tree>` finds another tree
# named only in comments. An `{nvs}` command names any path it likes in its `argv`, so it keeps
# every `tests/` partition. A crate's tests read the tree at run time (the policy tests grep other
# crates' sources, `nvs-fmt` and `nvs-syntax` walk `tests/` whole, `spec_registry_coverage` walks the goals,
# `extension_reference` reads `editors/vscode/package.json`), so a cargo check's set is a
# program's plus `crates` as bytes, `crate-tests`, `goals`, `editors` and every `tests/`
# partition -- not `tools/` (its one read file is in `crates`), not `bench-members` and not
# `prose`. A Python tool reads whatever it likes -- `chain.py`, `plan.py` and
# `playbook.py` read the handoff -- so a tool command keys on the whole tree, the session's own
# state files included, and is the one kind a wrap invalidates every session.
#
# `CARGO_ARGS_READS` and `COMMAND_READS` are the exceptions by name: the few checks that cost
# minutes and whose inputs are a known, short list. Each row says what was read to show it.
#
# A partition is the WIDEST key a check can have, and three kinds of check are answered more
# narrowly before `reads_of` is asked (`Goal.inputs_for`). A `cargo test -p <crate>` check is
# keyed on that crate's test binaries as `tools/impact.py` keys them (`binary_inputs`): its own
# package, the packages it is compiled against, and what its tests record opening through
# `nvs_repo`. A Python gate is keyed on what `tools/observe.py` last saw it open, list and start
# (`observed_inputs`), which is what took the handoff out of two hundred keys. And a row of
# `CARGO_ARGS_READS` or `COMMAND_READS` has `crates` replaced by the packages that check builds
# (`package_inputs`).
# Each of the three answers `None` when it cannot show what the check reads, and then the
# partitions below stand, exactly as they did. `Goal.audits` is what checks the first two.
#
# What no partition holds is a SERVICE's state -- the database a `queue migrate` check or the
# `examples/queue.nvs` fixture reaches -- so a memo cannot see that drift. It is not a change a
# session makes to the tree, which is what the memo exists to catch, and `--goal-only --full`
# sees it exactly as every sweep used to.
PARTITIONS = {
    "crates": ("crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml",
               "deny.toml", "nvs.toml", "LICENSE", "THIRD-PARTY-LICENSES.txt"),
    "crate-tests": ("benches",),
    "examples": ("examples",),
    "tests": ("tests",),
    "conformance": (),
    "differential": (),
    "hostile": (),
    "lsp-cases": (),
    "bench-members": (),
    "docs": (),
    "goals": (),
    "prose": ("docs",),
    "tools": ("tools",),
    "editors": ("editors",),
}
# The splits below the top level, first match wins: a path under one of these prefixes belongs to
# the named partition whatever its top-level entry says. The `vectors/` row is before the
# `crate-tests` rule in `partition_of` on purpose -- it is under a `tests/` directory and embedded.
SPLITS = (
    ("crates/nvs-stdlib/tests/vectors/", "crates"),
    ("tools/data/php-builtins.txt", "crates"),
    ("docs/reference/", "docs"),
    ("docs/spec/", "docs"),
    ("docs/agent/goals/", "goals"),
    ("tests/conformance/", "conformance"),
    ("tests/differential/", "differential"),
    ("tests/hostile/", "hostile"),
    ("tests/lsp/", "lsp-cases"),
    ("benches/members/lang/types/numbers-bool-int-uint-float-decimal.nvs", "crate-tests"),
    ("benches/members/", "bench-members"),
)
CRATE_TEST_DIRS = ("tests", "benches")
# Every other top-level entry -- `website/`, `fuzz/`, `docker/`, `AGENTS.md`, the dotfiles -- is
# `other`. The files the wrap rewrites every session are `state`: their own partition counts them
# by NAME only, so that a handoff does not stale every fixture, suite and crate test in the tree,
# and only a set that holds `state` sees their bytes.
OTHER, STATE, CODE = "other", "state", "crates-code"
#: Not a partition of bytes: the set of paths in the tree, which `observed_inputs` keys on.
PATHS = "paths"
STATE_FILES = re.compile(r"^docs/agent/(handoff\.md|goals/[^/]+\.handoff\.md)$")
NOT_INPUTS = {".git", "target", ".loop", ".agent-tmp", "node_modules", "out", ".vscode-test",
              "__pycache__"}
EVERYTHING = tuple(PARTITIONS) + (OTHER, STATE)
TEST_TREES = ("tests", "conformance", "differential", "hostile", "lsp-cases")
PROGRAM_READS = (CODE, "docs", "examples", "tests")
NVS_COMMAND_READS = (CODE, "docs", "examples", *TEST_TREES)
CARGO_READS = ("crates", "crate-tests", "docs", "goals", "examples", "editors", *TEST_TREES)
EDITOR_READS = ("crates", "editors")

#: The `cargo-named` checks that do not get `CARGO_READS`, by their whole `args` list or, with a
#: one-element key, by the package a `-p` names. Each is a cost guard -- a release build and a
#: measurement, minutes of a sweep -- whose inputs were read off its source:
#:
#: * the two `by_the_margin_this_test_names` tests in `crates/nvs-cli/src/cache.rs` and
#:   `serve.rs` write their program into a scratch directory of their own and open nothing in
#:   the tree, so what they measure is the binary;
#: * `benches/abi-probe/` compiles its own sources, and its `perf_guards` test target opens
#:   `examples/arith.nvs` and the one bench member `SPLITS` files under `crate-tests`;
#: * `crates/nvs-host/` opens a kernel setting, a certificate path it is handed and a key log it
#:   wrote itself, and nothing in the tree: `grep -rnE 'read_to_string|read_dir|File::open|env!'`
#:   over it is the evidence.
#:
#: `crates` and `docs` are what the binary is built from. A guard that starts opening something
#: else loses its row in the slice that makes it do so.
CARGO_ARGS_READS = {
    ("test", "--release", "-p", "nvs-cli", "--bin", "nvs", "by_the_margin_this_test_names"):
        ("crates", "docs"),
    ("nvs-abi-probe",): ("crates", "crate-tests", "docs", "examples"),
    ("nvs-host",): ("crates", "crate-tests", "docs"),
}

#: The `command` checks that do not get `EVERYTHING`, by a string their `argv` holds. Each costs
#: minutes, builds the workspace itself, and was read to see what else it opens:
#:
#: * the fuzz run builds `fuzz/` -- `other` -- over four crates by path, and its corpus and
#:   artifacts are git-ignored, so in no partition;
#: * `tools/tsan.sh` runs the tests of `nvs-host` and `nvs-runtime`, whose `manifest_policy.rs`
#:   reads every `Cargo.toml` under the root and whose capability tests name `examples/`;
#: * `tools/db-matrix.py` runs `nvs-db`, three `nvs-stdlib` test targets and one `nvs` unit
#:   module against the servers `tests/db/compose.yaml` describes, and none of those opens a
#:   case tree or a fixture.
COMMAND_READS = (
    ("cargo +nightly fuzz run", ("crates", "docs", OTHER)),
    ("tools/tsan.sh", ("crates", "crate-tests", "docs", "examples", "tools", OTHER)),
    ("tools/db-matrix.py", ("crates", "crate-tests", "docs", "tests", "tools")),
)
#: For each `COMMAND_READS` row, the file that says which workspace packages that check builds:
#: the fuzz workspace's manifest, and the two scripts, each of which names its `-p` crates.
BUILDS = (
    ("cargo +nightly fuzz run", "fuzz/Cargo.toml"),
    ("tools/tsan.sh", "tools/tsan.sh"),
    ("tools/db-matrix.py", "tools/db-matrix.py"),
)
TOP_OWNER = {top: name for name, tops in PARTITIONS.items() for top in tops}


def partition_of(rel):
    """The partition a tree path (posix, relative to the root) is hashed into: a `SPLITS` prefix
    first, then a crate's `tests/` or `benches/` directory, then its top-level entry's owner, and
    `other` for a top-level entry no partition names."""
    for prefix, name in SPLITS:
        if rel.startswith(prefix):
            return name
    parts = rel.split("/")
    if parts[0] == "crates" and len(parts) > 3 and parts[2] in CRATE_TEST_DIRS:
        return "crate-tests"
    return TOP_OWNER.get(parts[0], OTHER)
# The two memos that are a whole leg rather than a check: keyed like a program, because that is
# what they run. `Goal.__init__` builds their specs.
LEG_MEMOS = ("wsl leg", "valgrind sweep")


def reads_of(c):
    """The partitions this check's verdict can depend on, from its kind and what it runs."""
    kind = c["kind"]
    if kind in PROGRAM_KINDS or kind in LEG_MEMOS:
        return PROGRAM_READS
    if kind == "nvs-suite":
        return suite_reads(c.get("args", []))
    if kind == "cargo-named":
        args = tuple(c.get("args", []))
        package = args[args.index("-p") + 1:args.index("-p") + 2] if "-p" in args else ()
        return CARGO_ARGS_READS.get(args) or CARGO_ARGS_READS.get(package) or CARGO_READS
    if kind == "command":
        argv = c.get("argv", [])
        if argv and argv[0] == "{nvs}":
            return NVS_COMMAND_READS
        if argv and argv[0] == "npm" and c.get("cwd", ".").startswith("editors/"):
            return EDITOR_READS
        for needle, reads in COMMAND_READS:
            if runs(argv, needle):
                return reads
    return EVERYTHING


def runs(argv, needle):
    """Does this command run what `needle` names? A `git grep` whose pattern merely quotes the
    command does not: it reads the file it searches, which no `COMMAND_READS` row holds."""
    return bool(argv) and argv[0] != "git" and any(needle in a for a in argv)


def suite_reads(args):
    """What a `.nvst` suite reads: a program's set, plus the partition of every path its `args`
    name. A directory `partition_of` files under plain `tests` -- `tests/` itself, or anything
    that is not a case tree -- may hold any of them, so it gets every `tests/` partition, and so
    does a suite that names no path at all."""
    trees = set()
    paths = [a for a in args[1:] if not a.startswith("-")]
    for a in paths:
        name = partition_of(a.rstrip("/") + "/")
        trees.update(TEST_TREES if name == "tests" else (name,))
    if not paths:
        trees.update(TEST_TREES)
    return PROGRAM_READS + tuple(sorted(trees - set(PROGRAM_READS)))

#: The sweep runs several fixtures at once. It was strictly serial once, and is 42% of an
#: acceptance check -- 21.3 of its 50.3 minutes over the 20260826-142040 run, 58s a session for 20
#: fixtures -- so how wide it runs matters. **How wide is `tools/machine.py`'s decision, not this
#: file's**: it is a property of the box, measured there once and cached, and a constant here was
#: a property of the box it was measured on.
#:
#: Nothing is traded for the concurrency. A leak verdict is per-process and deterministic, so
#: concurrency cannot change one -- unlike the abi-probe cost guards, which are cost-class
#: assertions and must have an idle machine. Those run strictly AFTER this sweep, and it is why
#: `machine.py` hands out half a box and not all of it. Finished is not the same as idle, though,
#: and `COST_SETTLES` is what the driver does about the shadow this sweep leaves behind it.

#: How long a red cost-class check waits before it is asked again, in seconds, one entry per ask.
#:
#: A cost is a measurement of a machine, and this machine is not idle the moment the sweep above
#: returns. Measured on the 20260918-110653 run: the last valgrind fixture exited 0.14s before the
#: abi-probe guards started, and the binary that runs in 6s on a quiet box took 26s there. The
#: fan-out ratio guard read its placed half at 22.2ms against 0.8ms alone while its serial half,
#: which needs one core and no wake-ups, was unchanged -- so this is the sweep's shadow and not a
#: regression, and a ratio guard cannot tell the two apart from the inside.
#:
#: So a red one is asked again after each wait here, and only a red at the end of the ladder is a
#: red. A regression fails every ask; a shadow fails the early ones, and the trace and the failure
#: line say which happened. Paid on a red alone, which is why it is generous.
#:
#: Two waits rather than one because the tail of that shadow outlives the first. Measured on the
#: 20260919-023803 run: `nvs-cli`'s warm-start margin read 3.62x as the sweep ended and 3.85x
#: thirty seconds later, against the 4x it names, and the same binary at the same commit read
#: 12.3x once the box had had three minutes. Its cold arm was 57.9ms, 56.9ms and 59.3ms across
#: those three -- the CPU-bound half of the ratio did not move at all, so what recovered was the
#: machine and not the tree, and a second wait of the length that recovery took is what tells them
#: apart. A run that needs it pays two and a half minutes once, against the session a red guard
#: costs.
COST_SETTLES = (30, 150)


#: The tools whose `command` checks run in a pool beside the cargo checks (`Goal.start_pool`), and
#: how wide. A tool belongs here when every form of it a goal runs only READS the tracked tree: no
#: cargo, no built binary, no database, nothing written. That is a property of the tool and is
#: checked by reading it, which is why this is a list of names and not a key a goal file can set --
#: `tools/db-matrix.py` talks to five servers the `nvs-db` tests share, `tools/reference.py` reads
#: the CLI the sweep is building beside it, and neither may run under a crate's test binaries.
#: The width is small on purpose: these run beside libtest, which already takes every core.
POOLED_TOOLS = frozenset(f"tools/{name}.py" for name in (
    "brief", "chain", "check-links", "check-migration", "decisions", "directives", "holes",
    "owners", "plan", "playbook", "records", "rules"))
POOL_WIDTH = 4


def is_pooled(c):
    """Is this a `command` check `Goal.start_pool` may run ahead of its turn? See `POOLED_TOOLS`."""
    argv = c.get("argv", [])
    return (c["kind"] == "command" and len(argv) >= 2 and argv[0] == "python"
            and argv[1] in POOLED_TOOLS and c.get("cwd", ".") == "."
            and not c.get("setup") and not c.get("overlap"))


class Pooled:
    """A pool's future, behind the one method `Goal.command` asks of whatever is in flight."""

    def __init__(self, future):
        self.future = future

    def join(self):
        self.future.result()


#: `cargo test --workspace`, bare: the one other argument list the shared test build answers.
WORKSPACE_TEST = ["test", "--workspace"]


def is_observed(argv, cwd):
    """Is this command one `Goal.run_command` runs under `tools/observe.py`: a script of this
    directory, started from the root by the interpreter the goal file calls `python`?"""
    return (len(argv) >= 2 and argv[0] == "python" and cwd == "."
            and argv[1].startswith("tools/") and argv[1].endswith(".py")
            and argv[1] != "tools/observe.py")


def observed_key(argv, cwd):
    return "\0".join([*argv, cwd])


def load_observed():
    """`CHECKREADS`, or an empty table -- and then every gate keeps `EVERYTHING` until it runs."""
    try:
        got = json.loads(CHECKREADS.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return got if isinstance(got, dict) else {}


def plain_crate_test(args):
    """`(crate, target)` for a check whose `args` is `cargo test -p <crate>`, or that plus one
    `--test <name>` -- `target` is None for the bare form. None for anything else."""
    if len(args) >= 3 and args[0] == "test" and args[1] == "-p":
        if len(args) == 3:
            return args[2], None
        if len(args) == 5 and args[3] == "--test":
            return args[2], args[4]
    return None


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

# Held while `target/release/nvs.exe` is being linked, and while a check that runs it is in
# flight: `runs_release_cli` says which checks, and `Goal.prebuild` is the link.
RELEASE_CLI_LOCK = threading.Lock()

# One lock per `overlap` command, held while it is in flight; see `Goal.start_overlap`.
OVERLAP_LOCKS = collections.defaultdict(threading.Lock)

#: The configuration a fixture runs under in the valgrind sweep, layered over `nvs.toml`, for a
#: fixture whose cost under memcheck is the distance to a ceiling its leak verdict does not depend
#: on. The named file says what it changes and why that is the same kill path. The native and WSL
#: legs never read this: a fixture's pinned output is judged against the repository's own tree.
VALGRIND_CONFIG = {"examples/limits.nvs": "tools/valgrind-limits.toml"}

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


def runs_release_cli(c):
    """Whether this check runs `target/release/nvs.exe` as it stands, without measuring it.

    `tools/dossier.py` runs every proof program against that binary, several at a time, and
    `bun nv proofs --record-perf` builds it by its own key and times every bench on it. The link
    in `Goal.prebuild` replaces the file, and on Windows it cannot while a proof has it open:
    `relink.py` then moves it aside and a proof starting in that window finds no binary at all.
    So such a check and the link hold `RELEASE_CLI_LOCK` in turn. It is not a `measures_release_cli`
    check, because it is not a cost-class guard and is not held to the end of the sweep. A
    `proof_groups` check is not one either: `Goal.proofs_verify` takes the lock itself, and only
    when the gate puts it on this binary.
    """
    if c["kind"] != "command":
        return False
    argv = c["argv"]
    return (any("dossier.py" in a for a in argv)
            or (argv[:3] == ["bun", "nv", "proofs"] and "--record-perf" in argv))


#: What `bun nv proofs` reads for every group, as `proofParts` in `tools/nv/keys/checks.ts` names
#: it: the partitions holding the roster's chapters, the registry, and every `covers:` marker and
#: plain call, then its own code, the policy, the help backlog and the perf ledger.
PROOF_PARTITIONS = ("crates", "crate-tests", "docs", "conformance", "differential")
PROOF_INPUTS = ("tools/nv", "package.json", "bun.lock", "tools/data/dossier-policy.toml",
                "tools/data/help-backlog.toml", "docs/perf/members.ndjson")


def proof_groups(c):
    """The groups a check verifies when it is `bun nv proofs --verify` and one or more
    `--group G` and nothing else, or `None`. Only that shape joins `Goal.proofs_verify`'s one
    run, because any other flag changes what the run answers for every group in it."""
    argv = c.get("argv", [])
    if c["kind"] != "command" or c.get("cwd", ".") != "." or argv[:4] != ["bun", "nv", "proofs", "--verify"]:
        return None
    rest = argv[4:]
    if not rest or len(rest) % 2 or any(flag != "--group" for flag in rest[::2]):
        return None
    return rest[1::2]


def split_proofs(r, groups):
    """One `nv proofs --verify` run over `groups`, as each group's own `Result`. Several groups
    print a `== <group>` section each, closed by `-- <group>: passed` or `failed`, and that line
    is the group's exit status. A section's first failing line goes on its stderr, so the verdict
    quotes the owed feature or the failed suite. A group with no closed section -- one run alone,
    or a run that died before printing it -- is the whole run."""
    if len(groups) == 1:
        return {groups[0]: r}
    sections, current = {}, None
    for line in r.out.splitlines():
        if line.startswith("== ") and line[3:] in groups:
            current = line[3:]
            sections[current] = [line]
        elif current is not None:
            sections[current].append(line)
    got = {}
    for g in groups:
        lines = sections.get(g, [])
        verdict = lines[-1] if lines else ""
        if verdict not in (f"-- {g}: passed", f"-- {g}: failed"):
            got[g] = r
            continue
        failed = verdict.endswith("failed")
        why = next((l for l in lines if "still owe" in l or re.search(r"\b[1-9]\d* failed\b", l)),
                   verdict) if failed else ""
        got[g] = Result(1 if failed else 0, "\n".join(lines) + "\n", why)
    return got


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
#
# `needs` is the one key only a program kind carries, because it is the only kind that runs more
# than once: a fixture whose claim a platform cannot make -- a store over a Unix socket is the case
# that asked for it -- names what it needs and is skipped where that is absent, rather than weakened
# into something every platform can print. It is a written key where `measures_release_cli` below is
# a derivation, and the difference is that a release binary is visible in a check's own `argv` while
# the transport a store was configured with is a fact about a file the check only names.
CHECK_KEYS = {
    #  kind           required                    optional
    "exact":       (("file", "want"),             ("args", "exit", "needs")),
    "ordered":     (("file", "want"),             ("args", "exit", "stream", "needs")),
    "contains":    (("file",),                    ("args", "exit", "stderr_contains",
                                                   "stdout_contains", "needs")),
    "min-bytes":   (("file", "min_bytes"),        ("args", "exit", "stream", "needs")),
    "command":     (("name", "argv"),             ("cwd", "want", "memoize", "exit", "setup",
                                                   "overlap")),
    "nvs-suite":   (("name", "args"),             ("cases", "memoize", "min_passing")),
    "cargo-named": (("name", "args", "tests"),    ("memoize",)),
}
# `memoize` is read in one direction: `false` means `remembered` never answers this check from the
# file, for a check whose inputs are not all in the tree. `true` and absent are the same thing --
# every check is remembered against what it reads now (`reads_of`).
#
# `setup` on a `command` is the one key that moves a check between tiers: it runs ahead of the floor
# fixtures rather than with the other commands, because what it prepares is the environment a
# fixture needs -- `nvs queue migrate` creates the tables `examples/queue.nvs` pushes a row into.
# Position in the goal file cannot express that on its own, since a `command` and a fixture are
# different tiers and the tier decides the order; `Goal.setup_checks` is where that is done. Such a
# check almost always wants `memoize = false` beside it: what it converges is a database or a file
# outside the tree, and a memo keyed on the tree would answer green for a server that was replaced.
#
# `overlap` on a `command` is the other key that moves a check between tiers, and it is for a check
# whose cost is a clock rather than work: a fuzz run told to last five minutes lasts five minutes on
# one core, and in the commands tier the whole sweep stood behind it. Such a check is started in the
# background once the native CLI is built and judged after the valgrind sweep, ahead of the release
# checks, so its verdict and its duration are what they were and only its place in the reporting
# order moves -- the trade `check()` already makes for the release profile. It must be a check whose
# verdict does not depend on what runs beside it, which is why a cost-class guard can never carry
# it, and it cannot also be `setup`, which asks for the opposite end of the sweep.
# `Goal.start_overlap` owns the mechanism.
#
# Allowed on every kind. `stage` is read by this driver for the run order and by `holes.py` for the
# worklist it prints, which is why an unstaged check is still legal but a misspelled one is not.
COMMON_KEYS = ("kind", "stage")
LIST_OF_STR = ("args", "argv", "cases", "stderr_contains", "stdout_contains", "tests", "want")
# Empty, each of these is a check that cannot fail: no argv to run, no test that must have run, no
# substring that must appear in order. `want` on an `exact` is NOT here -- empty there is the one
# meaning that reads right, "this fixture prints nothing", and it is asserted like any other line.
NONEMPTY = {"argv", "tests", ("ordered", "want")}
COUNTS = ("min_bytes", "min_passing")

#: Commands an acceptance list may not run, keyed on the argv tail that makes them wrong, with what
#: to run instead. A goal's checks are the things the goal's own work turns green. A command that
#: reports a REPO-WIDE BACKLOG is not one of those: it answers for a chore somebody else schedules,
#: so a goal gated on it cannot close until that chore is done, however finished the goal is. That
#: is a stop nobody in the run can clear, and it reads exactly like a goal that is not finished.
#: A command belongs here when its findings are a queue rather than a fault -- when the same list
#: would be red over a tree the goal never touched. Add to it with the reason, never silently.
CHORE_ARGV = {
    ("tools/decisions.py", "--check"): (
        '["python", "tools/decisions.py", "--gate"]',
        "`--check` counts every decision not yet summarized, and the summary pass is fired by the "
        "user (docs/agent/decisions-summary.md), never by a goal. A goal that opens an ADR is "
        "`missing` its own summary from the moment it writes the record, so the check is red on "
        "arrival. `--gate` is the same tool's set of findings that are always wrong, which is what "
        "a check named for a record's integrity is asking for, and it is the shape CI runs."),
}


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
        if "needs" in c and c["needs"] not in LEG_NEEDS:
            known = ", ".join(sorted(LEG_NEEDS))
            raise GoalError(f"{where}: `needs` is {c['needs']!r}, which no leg is asked -- one of: "
                            f"{known}")
        if "setup" in c and not isinstance(c["setup"], bool):
            raise GoalError(f"{where}: `setup` is a flag, not {c['setup']!r}")
        if "overlap" in c and not isinstance(c["overlap"], bool):
            raise GoalError(f"{where}: `overlap` is a flag, not {c['overlap']!r}")
        if c.get("overlap") and c.get("setup"):
            raise GoalError(f"{where}: `overlap` and `setup` ask for opposite ends of the sweep")
        if c.get("overlap") and measures_release_cli(c):
            raise GoalError(f"{where}: a check that measures the release CLI is cost-class, and "
                            f"`overlap` runs a check beside everything else")
        if c.get("exit", "nonzero") != "nonzero":
            raise GoalError(f'{where}: `exit` is absent or "nonzero", not {c["exit"]!r}')
        if kind == "contains" and not (c.get("stderr_contains") or c.get("stdout_contains")):
            raise GoalError(f"{where}: a contains check naming no substring cannot fail")
        if "file" in c and c["file"] not in files:
            raise GoalError(f"{where}: {c['file']} is not in `files`, so nothing checks it is on "
                            f"disk and the valgrind sweep never sees it")

        argv = [a for a in c.get("argv", []) if isinstance(a, str)]
        for (tool, flag), (instead, why) in CHORE_ARGV.items():
            if tool in argv and flag in argv:
                raise GoalError(f"{where}: `{tool} {flag}` reports a backlog this goal cannot "
                                f"clear, so it would hold the run on a chore. Use {instead} -- "
                                f"{why}")


def spec_error(path):
    """One line naming why `path` holds no acceptance list this driver can run, or "" when it does.

    The file-shaped wrapper around `validate_spec`, for the callers that hold a path to a goal file
    rather than the live spec: the chain, which reads every entry it has not walked yet, and
    `tools/chain.py --check`, which is the gate that says an entry is walkable and has to mean it.
    A queued goal validates as it sits, before a floor is folded into it -- the fold only ever adds
    checks the goal they came from was already run against."""
    try:
        validate_spec(tomllib.loads(path.read_text(encoding="utf-8")))
    except OSError as e:
        return f"cannot be read: {e}"
    except tomllib.TOMLDecodeError as e:
        return f"did not parse as TOML: {e}"
    except GoalError as e:
        return str(e)
    return ""


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
    * **Across runs**, every check's green verdict is remembered in `.loop/goal-green.json`
      against a content hash of *the partitions of the tree its kind reads* (`reads_of`,
      `partition_ids`) and of its own spec. Those inputs being bit-identical is the whole
      argument: a deterministic check over identical bytes cannot reach a different verdict,
      which is `verify.py`'s rule for its own green cache. The file outlives a session, a run
      and a goal switch, so a verdict stands until a session changes a byte the check reads,
      however long ago it was filed. **That is what scopes the sweep a goal is reached on**: it
      runs with the floor gate open and the memo consulted, so it pays for the checks whose
      inputs the goal's sessions changed and for nothing else. `full` consults no memo and is
      set by `--goal-only --full` alone.
    """

    def __init__(self, spec):
        validate_spec(spec)
        self.files = spec.get("files", [])
        self.checks = spec.get("check", [])
        self.valgrind_skip = set(spec.get("valgrind", {}).get("skip", []))
        self.wsl_target = spec.get("wsl", {}).get("target_dir", "/var/tmp/nvs-target-wsl")
        # Program checks split in two, and the split is by STAGE rather than by kind.
        #
        # All of them used to run before every cargo check -- they need only the native build --
        # and the sweep stops at the first failure, so a fixture belonging to the goal's LAST
        # stage decided what the driver reported about a goal whose middle stages were the work
        # in flight. Measured: goal `schema`'s `examples/schema.nvs [6 the member]` failed for eight
        # consecutive sessions with the identical line while stages 3, 4 and 5 landed underneath
        # it. Every one of those sessions was told, at the top of its pack and in the imperative,
        # to close a check that could not pass until three stages it did not name had landed.
        #
        # The floor's fixtures keep their place: they are the previous goal's whole acceptance
        # list, they are pure regression detection, and catching a regression before paying for a
        # test build is the reason this tier exists at all. The CURRENT goal's fixtures move
        # behind the cargo checks, because a goal's fixture is its acceptance surface and the
        # cargo checks are the stages that build up to it -- running the end-to-end fixture first
        # can only ever report "the thing at the end is not done yet", which no session can act
        # on. Both halves stay sorted by stage, so the first failure in either is the earliest.
        programs = sorted((c for c in self.checks if c["kind"] in PROGRAM_KINDS), key=stage_key)
        self.program_floor = [c for c in programs if is_floor(c)]
        self.program_checks = [c for c in programs if not is_floor(c)]
        # Both halves, in run order. The split above is about WHEN each runs on the native leg;
        # everything that simply needs every fixture -- the WSL leg, `--leg-only`, the progress
        # bar's count -- wants this and must not pick one half by accident.
        self.all_programs = self.program_floor + self.program_checks
        self.ran = []  # (label, seconds) for every check this run actually paid for
        self.short = []  # `min_passing` thresholds not met; see `check()`
        self.verbose = False  # narrate each check as it starts and what it cost
        self._begun = 0.0  # monotonic start of the current check(), for the elapsed stamp
        self._cargo = {}  # args tuple -> Result, within one check() call
        self._suite = {}  # (leg name, args tuple) -> Result, likewise
        self._commands = {}  # (argv tuple, cwd) -> Result, likewise
        self._proofs = {}  # proofs group -> its Result out of `proofs_verify`'s run, likewise
        self._proofs_lock = threading.Lock()
        self._exes = None  # package -> [(target, exe, dir)] off the workspace build; see test_executables
        self._crate_runs = {}  # package -> Result of its binaries, within one check() call
        self._verify_green = None  # `verify.py`'s per-binary green record, read once a sweep
        self._audited = set()  # (crate, target) pairs an audit is running; `verify_green` skips them
        self.reused = 0  # binaries `crate_tests` answered from `verify.py`'s record this sweep
        self._tree = ""
        self._parts = None  # partition name -> content hash, or None if unreadable; see `partition_ids`
        self._tiers = None  # the `verify_keys.Tree` that walk took, which `binary_inputs` keys on
        self._ignored = set()  # what git ignores, as that walk read it
        self._digests = {}  # tree path -> content hash, for `observed_inputs`; one sweep's worth
        self._observed = load_observed()  # "argv\0cwd" -> what `tools/observe.py` saw it read
        self._observed_dirty = False
        self._reach = None  # `impact.Reach` over `_tiers`, built by the first check that asks
        self._green = {}  # memo key -> the inputs hash it was last green over; see `remembered`
        self._green_dirty = False  # `_green` holds a verdict `.loop/goal-green.json` does not yet
        self.full = False  # consult no memo; `--goal-only --full` is the one thing that sets it
        self.skipped = []  # memo keys this run answered from the file rather than by running
        self._ran_green = set()  # memo keys THIS run made green; a later duplicate is not a memo hit
        # The two memos that are a leg rather than a check, as specs so they key like one: the
        # fixtures they run over, and for the valgrind sweep the suppressions it runs under and
        # the configuration any fixture is given there.
        supp = ROOT / "tools" / "valgrind.supp"
        configs = {f: (ROOT / p).read_text(encoding="utf-8") if (ROOT / p).is_file() else ""
                   for f, p in sorted(VALGRIND_CONFIG.items())}
        self.leg_specs = {
            "wsl leg": {"kind": "wsl leg", "name": "wsl leg", "files": list(self.files),
                        "programs": list(self.all_programs)},
            "valgrind sweep": {"kind": "valgrind sweep", "name": "valgrind sweep",
                               "files": list(self.files), "skip": sorted(self.valgrind_skip),
                               "supp": supp.read_text(encoding="utf-8") if supp.is_file() else "",
                               "configs": configs},
        }
        self._overlap = {}  # (argv tuple, cwd) -> the thread or `Pooled` running that command
        self._pool = None  # the executor behind `start_pool`, for `check()` to shut down
        self._wsl_build = None  # (leg, thread, verdict box) while `start_wsl_build`'s build runs
        self._prebuild = None  # the thread warming the release profile
        self._prebuilt = frozenset()  # the arg lists it warms, so `cargo()` knows to wait for it
        self.floor_gate = True  # do the carried floor and the release profile run this sweep?
        self.settle_only = False  # `--settle`: the carried floor and nothing else
        self.collect = False  # run past a red check and report every red; see `_check`
        self.reds = []  # the red checks a collecting sweep ran past, in sweep order
        self._claimed = set()  # memo keys `skip` ran anyway this sweep; `audits` says why
        self.held = []  # what a shut gate did not run; non-empty means "green" is not "reached"
        self.fast_path = ""  # a check name to try before the sweep; see `fast_fail`
        self.failed_name = ""  # the `cargo-named` check this run died on, for the next one
        self._widen = suite_widening(self.checks)
        cargo = [c for c in self.checks if c["kind"] not in PROGRAM_KINDS]
        # The environment the fixtures run against, prepared before they run: a `setup` command is
        # lifted out of the commands tier, which is behind every program leg, and run ahead of the
        # floor. `CHECK_KEYS`' note owns what the key means; what is decided here is that it is a
        # tier of its own rather than a sort key, because a stage label is what `goal-switch.py`
        # rewrites when it carries a goal's list forward and an ordering that survives that has to
        # be written on the check itself.
        self.setup_checks = [c for c in cargo if c.get("setup")]
        cargo = [c for c in cargo if not c.get("setup")]
        # A tier of its own for the reason `setup` is one, at the other end: `start_overlap` starts
        # these beside the sweep and `check()` judges them once the valgrind sweep is behind it.
        self.overlap_checks = [c for c in cargo if c.get("overlap")]
        cargo = [c for c in cargo if not c.get("overlap")]
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
        # By stage, for the reason the program checks are: the sweep stops at the first failure,
        # so file order decides which of a goal's stages the driver reports and a session acts on.
        # Stable within a stage, so a stage's own checks keep the order its author wrote them in.
        self.cargo_checks = sorted(
            (c for c, first in zip(cargo, catch_up) if not first and id(c) not in held),
            key=stage_key,
        )

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
        if key in self._prebuilt:
            self.join_prebuild()
        if key not in self._cargo:
            self._cargo[key] = capture("cargo", args)
        return self._cargo[key]

    def command(self, argv, cwd):
        """A `command` check's process, with the result shared by every check naming the same argv
        in the same directory, exactly as `cargo()` shares a cargo run. A floor folds in the same
        tool gate -- `reference.py --check`, `rules.py --check` -- once per goal that named it, and
        the tree cannot change inside one check() call, so every run after the first answers the
        same thing again. Each check still judges its own exit and `want` against the result.

        A command `start_overlap` has in flight is waited for rather than started a second time."""
        key = (tuple(argv), cwd)
        running = self._overlap.pop(key, None)
        if running:
            running.join()
        if key not in self._commands:
            self._commands[key] = self.run_command(argv, cwd)
        return self._commands[key]

    def proofs_verify(self, c):
        """A `proof_groups` check's result, out of ONE `bun nv proofs --verify` run over every group
        the sweep owes: the first such check to arrive starts it, over its own groups and those of
        every proofs check in the cargo tier the memo does not answer, and each check is then
        handed its own groups' sections. The roster is read once and every program runs in one
        pool, where a run per check read it once per group. A check whose groups are not all in
        that run -- one the memo answered, asked anyway -- starts a run of its own.

        With the floor gate open the run is on the release binary, which `release_cli` builds in
        `prebuild`'s thread: this joins that build, names the binary with `--nvs`, and holds
        `RELEASE_CLI_LOCK` so no link replaces it mid-run. With the gate shut it is on the proof
        binary `nv proofs` builds for itself, because that is a scoped sweep and the release build
        is the gate's to pay for."""
        groups = proof_groups(c)
        with self._proofs_lock:
            if any(g not in self._proofs for g in groups):
                owed = [g for g in groups if g not in self._proofs]
                for other in self.swept(self.cargo_checks):
                    theirs = proof_groups(other)
                    if theirs is None or not self.owes_proofs(other):
                        continue
                    owed += [g for g in theirs if g not in self._proofs and g not in owed]
                argv = ["nv", "proofs", "--verify", *(a for g in owed for a in ("--group", g))]
                label = f"proofs: {len(owed)} group(s) in one run"
                if self.floor_gate:
                    self.join_prebuild()
                    argv += ["--nvs", str(relink.release_cli(ROOT))]
                    label += " on the release binary"
                self.trace(label)
                with RELEASE_CLI_LOCK if self.floor_gate else contextlib.nullcontext():
                    r = self.timed(label, lambda: capture("bun", argv, timeout=7200, cwd=ROOT))
                self._proofs.update(split_proofs(r, owed))
            parts = [self._proofs[g] for g in groups]
        if len(parts) == 1:
            return parts[0]
        return Result(max(p.code for p in parts), "\n".join(p.out for p in parts),
                      "\n".join(p.err for p in parts if p.err))

    def owes_proofs(self, c):
        """Whether this sweep runs the `proof_groups` check `c`: the memo does not answer it, or
        it is audited anyway. `proofs_verify` batches every such check, and `release_cli` builds
        the release binary for them when the gate is open."""
        return not self.remembered(c) or self.audits(c)

    def run_command(self, argv, cwd):
        """One `command` check's process. A Python gate (`is_observed`) runs under
        `tools/observe.py`, which is the same script in the same interpreter with what it read
        written down beside it; `observed_inputs` keys the check on that from the next sweep on.
        A record is kept only from a run that left one, so a gate that died early keeps the key
        it had."""
        if not is_observed(argv, cwd):
            return capture(argv[0], argv[1:], cwd=ROOT / cwd)
        name = hashlib.blake2b("\0".join(argv).encode("utf-8"), digest_size=8).hexdigest()
        out = READSDIR / f"{name}.json"
        with contextlib.suppress(OSError):
            out.unlink()
        r = capture(argv[0], ["tools/observe.py", "--out", str(out), "--", *argv[1:]], cwd=ROOT)
        try:
            seen = json.loads(out.read_text(encoding="utf-8"))
            self._observed[observed_key(argv, cwd)] = {k: seen[k] for k in ("files", "dirs", "spawns")}
            self._observed_dirty = True
        except (OSError, ValueError, KeyError):
            pass
        return r

    def start_overlap(self, leg):
        """Start every `overlap` command this sweep owes, in the background, and return at once.

        `CHECK_KEYS`' note owns what the key means. The mechanism is `command()`'s own cache: the
        thread files its result where `command()` looks, and `command()` joins a thread still in
        flight, so `cargo_check` judges an overlapped check exactly as it judges any other and what
        it times is the wait that was left rather than the run. The run's own cost goes on `ran`
        under a label that says it was overlapped, because the three slowest are ranked by what a
        check cost and a fuzz build that doubled should still show there.

        One run of any one command at a time across the whole run, for `prebuild`'s reason: a
        sweep that fails early returns with the thread still running, and the next sweep's `Goal`
        knows nothing about it. Two different commands run side by side, which is the point of
        the key. Nothing with the gate shut or over remembered inputs -- the same two questions `check()`
        asks at the judging end, asked here so nothing is started that nothing will read."""
        for c in self.swept(self.overlap_checks):
            if self.remembered(c):
                continue
            argv = [(leg.binary if a == "{nvs}" else a) for a in c["argv"]]
            cwd = c.get("cwd", ".")
            key = (tuple(argv), cwd)
            if key in self._overlap or key in self._commands:
                continue
            label = f"{c['name']} [{c.get('stage', '?')}] (overlapped)"

            # This sweep's own two containers, bound now: `begin()` replaces both, and a thread
            # that outlived its sweep must not file a verdict in the next one's cache.
            def run(argv=argv, cwd=cwd, key=key, label=label,
                    commands=self._commands, ran=self.ran):
                with OVERLAP_LOCKS[key]:
                    started = time.monotonic()
                    commands[key] = self.run_command(argv, cwd)
                    ran.append((label, time.monotonic() - started))

            self._overlap[key] = threading.Thread(target=run, daemon=True)
            self._overlap[key].start()
            self.trace(f"{c['name']} started in the background")

    def start_wsl_build(self):
        """Start the Linux leg's BUILD beside the cargo tier, with the floor gate open.

        The build and nothing after it. It is a compile under a target directory of its own, so
        it shares nothing with the tier it runs beside but cores. The leg's fixtures and the
        valgrind sweep stay behind the tier, and must: they reach the same database servers the
        `nvs-db` tests and `tools/db-matrix.py` do, and those have only ever run one after the
        other. Nothing with the gate shut, where a sweep is seconds long and may not want a leg at
        all, and nothing when the two things that use the binary are green on these inputs --
        the question `check()` asks where it joins this."""
        if not self.floor_gate or not self.swept_programs() or not wsl_available():
            return
        if (self.remembered(self.leg_specs["valgrind sweep"])
                and self.remembered(self.leg_specs["wsl leg"])):
            return
        leg, box = WslLeg(self.wsl_target), {}
        thread = threading.Thread(target=lambda: box.update(fail=leg.prepare()), daemon=True)
        thread.start()
        self._wsl_build = (leg, thread, box)
        self.trace("wsl build started in the background")

    def wsl_built(self):
        """The Linux leg and its build's verdict: the build `start_wsl_build` has in flight, waited
        for, or one made here. Timed as `wsl build` either way, so overlapped it costs the wait."""
        if self._wsl_build:
            leg, thread, box = self._wsl_build
            self._wsl_build = None
            self.timed("wsl build", thread.join)
            return leg, box.get("fail", "the wsl build ended without a verdict")
        leg = WslLeg(self.wsl_target)
        return leg, self.timed("wsl build", leg.prepare)

    def start_pool(self):
        """Start every `is_pooled` command the cargo tier owes, `POOL_WIDTH` at a time, and return.

        The tier is judged one check at a time in stage order, and that does not change: this
        only runs the read-only tool gates ahead of their turn, through the cache `command()`
        reads and the join it already does for an `overlap` command. So the first red check named
        is the one a serial tier would have named. What is given up is stopping early -- a red
        tier has by then paid for gates behind the red check -- and `fast_fail` has already taken
        the common case of that, the frontier check, before any of this starts.

        A run that cost a second or more goes on `ran` under a label that says it was pooled,
        for the reason an overlapped one does."""
        owed, seen = [], set()
        for c in self.swept(self.cargo_checks):
            key = (tuple(c["argv"]), ".") if is_pooled(c) else None
            if key is None or key in seen or key in self._commands:
                continue
            if self.remembered(c) and not self.audits(c):
                continue
            seen.add(key)
            owed.append((key, f"{c['name']} [{c.get('stage', '?')}] (pooled)"))
        if not owed:
            return
        self._pool = ThreadPoolExecutor(max_workers=POOL_WIDTH)

        def run(key, label, commands=self._commands, ran=self.ran):
            started = time.monotonic()
            commands[key] = self.run_command(list(key[0]), ".")
            spent = time.monotonic() - started
            if spent >= 1:
                ran.append((label, spent))

        for key, label in owed:
            self._overlap[key] = Pooled(self._pool.submit(run, key, label))
        self.trace(f"{len(owed)} read-only tool gate(s) started in a pool of {POOL_WIDTH}")

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

    def crate_tests(self, crate, target=None):
        """`cargo test -p <crate>`'s verdict off the shared build: each of the crate's test
        executables -- or the one `--test <target>` names -- run in turn, where cargo would run it
        -- the package's own directory, with `CARGO_MANIFEST_DIR` set -- and the outputs joined so
        a check reads them as it read the one cargo output. Shared by every check naming the same
        crate and target, like `cargo()`.

        The exit code is the first non-zero one, and the run stops there as cargo's does. libtest
        reports a failure on STDOUT, so the stderr the ledger's line is read from is given one
        naming the failing tests: without it the line would end at `exit 101 --`."""
        key = (crate, target)
        if key in self._crate_runs:
            return self._crate_runs[key]
        exes, fail = self.test_executables()
        chosen = [t for t in exes.get(crate, []) if target is None or t[0] == target] if exes else []
        if exes is None:
            r = Result(-1, "", fail)
        elif not chosen:
            r = Result(-1, "", f"no test executable in the workspace build belongs to {crate!r}"
                       + (f" under the target {target!r}" if target else ""))
        else:
            code, outs, errs = 0, [], []
            for target, exe, cwd in chosen:
                held = self.verify_green(exe) if key not in self._audited else None
                if held is not None:
                    self.trace(f"{crate} ({target}) green in verify.py's run over these inputs")
                    self.reused += 1
                    outs.append(held)
                    errs.append("")
                    continue
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
        self._crate_runs[key] = r
        return r

    def verify_green(self, exe):
        """The output `verify.py`'s `test` step recorded for the binary `exe` when it last ran green,
        or `None` when that record does not stand for the bytes on disk now.

        The record is `.agent-tmp/verify-test-green.json`, keyed per binary by `tools/impact.py`
        over what the binary reads -- the same key `binary_inputs` files this sweep's own memo
        under. A session runs `verify.py` over its work, and the sweep behind it would otherwise run
        every binary that work reached a second time over the same inputs, which for the server's
        end-to-end binaries is minutes a session. So a binary with a matching key is answered from
        the record and not run. A binary `impact` calls wide is keyed over the whole tree, so its
        record stands only while nothing at all has changed, which is the trust `verify.py` itself
        gives it. Never under `--full`, never for a check `audits` is running to test the memo, and
        never for a record written before the record carried its test lines, since a `cargo-named`
        check reads those lines for the names it wants."""
        if self.full or self._tiers is None:
            return None
        reach = self.reach()
        if self._verify_green is None:
            try:
                got = json.loads((ROOT / ".agent-tmp" / "verify-test-green.json")
                                 .read_text(encoding="utf-8"))
            except (OSError, ValueError):
                got = {}
            self._verify_green = got if isinstance(got, dict) else {}
        want = os.path.normcase(os.path.normpath(exe))
        job = next((j for j in reach.jobs if j.get("argv")
                    and os.path.normcase(os.path.normpath(j["argv"][0])) == want), None)
        if job is None:
            return None
        entry = self._verify_green.get(job["name"])
        if not isinstance(entry, dict) or not isinstance(entry.get("tests"), list):
            return None
        if entry.get("key") != reach.key(job)[0]:
            return None
        # A record carries one line per test its result line counts, or it is missing some: a
        # line shape `verify.py`'s pattern did not match, and a check naming that test would read
        # it as one that did not run.
        counted = sum(int(p) + int(i) for p, i in
                      re.findall(r"(\d+) passed; \d+ failed; (\d+) ignored", entry.get("result", "")))
        if len(entry["tests"]) < counted:
            return None
        return "\n".join(entry["tests"] + [entry.get("result", "")])

    def workspace_tests(self):
        """`cargo test --workspace`'s verdict off the shared build: `crate_tests` for every
        package the build produced a test executable for, the outputs joined, stopping at the
        first package that fails as cargo does.

        A check names the whole workspace when its tests sit in two crates, since a check carries
        one argument list. Handed to cargo, that ran every test binary in the tree a second time
        behind the per-crate runs that had just run them; here it is those same runs, each paid
        for once whichever check asks first. Doctests are not run, as they are not for a
        `-p <crate>` check: the shared build holds no doctest executable."""
        exes, fail = self.test_executables()
        if exes is None:
            return Result(-1, "", fail)
        code, outs, errs = 0, [], []
        for crate in sorted(exes):
            one = self.crate_tests(crate)
            outs.append(one.out)
            errs.append(one.err)
            if one.code != 0:
                code = one.code
                break
        return Result(code, "\n".join(outs), "\n".join(errs))

    # -- the release build, moved off the critical path ---------------------------------

    def release_builds(self):
        """Every check in the goal whose cost is a BUILD and not a test, one argument list each.

        `--release` is a different profile from everything else here, so nothing they need is on
        disk when the sweep starts, and this workspace's release profile is `lto = "thin"` with
        `codegen-units = 1` -- measured at 133s after a one-line change to `nvs-runtime`, against
        the ~130s the whole rest of the sweep costs. Found by their `--release` rather than named,
        so a goal that moves a guard to another crate does not have to come back here.

        **All of them, not the first.** Each names its own `-p <crate>`, which is a compilation of
        its own, so a list that stopped at one left the rest to compile inside their own checks --
        and a cost-class guard that starts 0.6s after a 2m31s link measures the link, which is how
        `nvs-cli`'s warm-start margin read 1.8x on a tree that prints 13x idle.

        Deduplicated, because several stages name the same probe; and a check whose verdict is
        already remembered against this tree is left out, since `prebuild` would then be warming a
        profile nothing is going to ask about."""
        builds = []
        for c in self.checks:
            if c["kind"] in PROGRAM_KINDS or "--release" not in c.get("args", []):
                continue
            if self.remembered(c) or c["args"] in builds:
                continue
            builds.append(c["args"])
        return builds

    def release_cli(self):
        """The other release build: `target/release/nvs.exe`, when a check measures it or a
        `proof_groups` check runs on it.

        `tools/bench.py` measures that binary and deliberately builds nothing — its own
        `warn_if_stale` says the numbers are about the build on disk rather than the tree — and no
        other leg here produces it, since `NativeLeg` builds the debug CLI. So the warm-start check
        was measuring whatever had last been built by hand, which twice meant a binary too old to
        read the tree's own `nvs.toml` and reported a *configuration* error as the start figure.
        `proofs_verify` names the same binary with `--nvs` whenever the gate is open, which is the
        only time this build runs, so a proofs check the sweep owes asks for it too.

        Built in `prebuild`'s thread rather than before the check: it is the same profile, the same
        lock and the same argument as the release test build above — the build overlaps the sweep
        and the measurement does not. The argument is `releaseBinary`'s in `tools/nv/proofs/run.ts`,
        so both build into one set of artefacts.
        """
        build = ["build", "--release", "-p", "nvs-cli"]
        for c in self.checks:
            if measures_release_cli(c) and not self.remembered(c):
                return build
        for c in self.swept(self.cargo_checks):
            if proof_groups(c) is not None and self.owes_proofs(c):
                return build
        return None

    def prebuild(self):
        """Start those builds now, in the background, and let the rest of the sweep run beside them.

        The sweep was strictly serial, so the release build was ~60% of an acceptance check that a
        session waits out before the next one can start. Nothing else in the sweep touches the
        release profile, and two cargos on one `target/` were measured NOT to block each other --
        a debug build finished in its usual 4.5s beside a release build that took its usual 129s.

        `--no-run` on purpose, and this is the part that must not be traded away: the guards are
        cost-class assertions, and a cost measured on a machine that is simultaneously linking is
        not the cost. So the BUILD overlaps and the RUN does not -- `cargo()` joins this thread
        before it starts the real invocation, which by then is a no-op build and a 3s test run.

        Nothing at all when the floor gate is shut: that is the whole point of the gate, since
        these builds ARE the sweep's critical path and not merely a step in it. Measured on the
        20260904-143054 run, the sweep waited **84s** at `join_prebuild` for a build that had been
        running since t=0, and everything else fitted underneath it."""
        if not self.floor_gate:
            return
        builds = self.release_builds()
        cli = self.release_cli()
        if not builds and not cli:
            return
        def build():
            # One at a time across the whole run. A sweep that fails before it reaches the guard
            # returns with this thread still building -- correctly, since the work is wanted either
            # way -- and `load_goal()` hands the next session a fresh `Goal` that knows nothing
            # about it. Without the lock those two cargos would build the same units at once.
            with PREBUILD_LOCK:
                for args in builds:
                    capture("cargo", [*args, "--no-run"])
                if cli:
                    # The link replaces `target/release/nvs`, which an editor running `nvs lsp`
                    # out of this tree holds open for the life of its window. `relink.py` moves
                    # the copy it is running aside so the retry can land; without that the build
                    # fails, the measurement is taken on whatever binary was there before, and
                    # this docstring's own complaint is back.
                    # `RELEASE_CLI_LOCK` keeps a proof run by `runs_release_cli` off the binary
                    # while it is replaced.
                    exe = relink.release_cli(ROOT)
                    with RELEASE_CLI_LOCK:
                        if relink.held(capture("cargo", cli).err, exe) and relink.free(exe):
                            capture("cargo", cli)
                        relink.sweep(exe)

        # Every warmed argument list, and a `cargo()` that finds its own in here waits out the
        # WHOLE thread: the builds are serial, so a guard released as soon as its own finished
        # would be measuring the next one. A CLI-only prebuild leaves this empty and is joined by
        # the bench check instead.
        self._prebuilt = frozenset(tuple(args) for args in builds)
        self._prebuild = threading.Thread(target=build, daemon=True)
        self._prebuild.start()
        self.trace("release build started in the background")

    def tree_id(self):
        """HEAD, plus a hash of everything not committed. Two runs with the same id are two runs
        over the same bytes, so a deterministic check cannot answer them differently.

        Kept for reporting. It is NOT what the memos key on, and `partition_ids` says why."""
        head = git("rev-parse", "HEAD") or "no-head"
        dirty = git("status", "--porcelain") + "\n" + git("diff", "HEAD")
        return head + ":" + hashlib.blake2b(dirty.encode("utf-8", "replace"),
                                            digest_size=8).hexdigest()

    def partition_ids(self):
        """One content hash per partition of the tree (`PARTITIONS`, `OTHER`, `STATE`), the
        compiler folded into `crates`, or `None` when anything was unreadable -- and then no memo
        fires and every check runs, which is `verify.py`'s rule as well: the safe direction is
        doing the work.

        One walk of the tree, every file's path and bytes fed to the hasher of the partition its
        top-level entry belongs to. The dirty tree is what is hashed, not HEAD: a session's
        uncommitted edit stales exactly what it would stale committed. What git IGNORES is not
        hashed: by `.gitignore`'s own comments every rule there is build output, a cache or
        machine-local state, and two of them are written by the sweep itself -- the packaged
        `.vsix` and the SQLite queue's database -- so hashing them made every run a new tree and
        the memo never fired. A session's work lands only in tracked or untracked-unignored files.
        `NOT_INPUTS` is the same idea as a prune list, so the walk never enters `target/`. A
        `STATE_FILES` match is fed to `state` whole and to its own partition by name alone, so a
        rewritten handoff changes `state` and nothing else. `CODE` is fed beside `crates`, file for
        file, with a `.rs` file's shipped tier where `crates` takes its bytes -- the comment over
        `PARTITIONS` says who reads which.
        """
        hashers = {name: hashlib.blake2b(digest_size=16) for name in EVERYTHING + (CODE, PATHS)}
        self._digests = {}
        for name in ("crates", CODE):
            hashers[name].update(rustc_version().encode("utf-8", "replace"))
        # A file path, or a wholly ignored directory with a trailing `/`. Empty when git cannot
        # answer, and then everything is hashed, which is the wide direction.
        ignored = {p for p in git("ls-files", "--others", "--ignored", "--exclude-standard",
                                  "--directory").split("\n") if p}

        self._ignored = ignored

        def feed(h, rel, path):
            hashers[PATHS].update(rel.encode("utf-8") + b"\0")
            h.update(rel.encode("utf-8") + b"\0")
            h.update(path.read_bytes())
            h.update(b"\0")
            if h is hashers["crates"]:
                code = hashers[CODE]
                code.update(rel.encode("utf-8") + b"\0")
                if rel in tiers.tier and rel not in tiers.embedded:
                    code.update(tiers.digest(rel, "shipped").encode("utf-8"))
                else:
                    code.update(path.read_bytes())
                code.update(b"\0")

        self._tiers = self._reach = None
        try:
            tiers = self._tiers = verify_keys.Tree()
            for top in sorted(os.listdir(ROOT)):
                if top in NOT_INPUTS or top in ignored or f"{top}/" in ignored:
                    continue
                base = ROOT / top
                if base.is_file():
                    feed(hashers[partition_of(top)], top, base)
                    continue
                if not base.is_dir():
                    continue
                for dirpath, dirnames, filenames in os.walk(base):
                    rel = Path(dirpath).relative_to(ROOT).as_posix()
                    dirnames[:] = sorted(d for d in dirnames
                                         if d not in NOT_INPUTS and f"{rel}/{d}/" not in ignored)
                    for f in sorted(filenames):
                        relf = f"{rel}/{f}"
                        if relf in ignored:
                            continue
                        name = partition_of(relf)
                        if STATE_FILES.match(relf):
                            feed(hashers[STATE], relf, ROOT / relf)
                            hashers[name].update(relf.encode("utf-8") + b"\0")
                            continue
                        feed(hashers[name], relf, ROOT / relf)
        except OSError:
            return None
        return {name: h.hexdigest() for name, h in hashers.items()}

    def memo_key(self, c, leg=""):
        """What a check's verdict is filed under: its name or fixture, the leg when it runs on one,
        and a digest of its spec -- so a `want` rewritten in the goal file is a different key, and
        two checks that are the same check share one. `stage` is left out of the digest: it says
        when a check runs and takes no part in what it answers, and `goal-switch.py` relabels it
        when a goal's list becomes the next goal's floor, which is the same check over the same
        inputs and keeps its verdict."""
        spec = json.dumps({k: v for k, v in c.items() if k != "stage"}, sort_keys=True)
        digest = hashlib.blake2b(spec.encode("utf-8"), digest_size=6).hexdigest()
        name = c.get("name") or c.get("file") or c["kind"]
        return f"{leg + ' ' if leg else ''}{name} #{digest}"

    def inputs_for(self, c):
        """The hash of everything this check can read, over the partitions `reads_of` names, or
        `None` when the tree could not be hashed."""
        if self._parts is None:
            return None
        narrow = self.binary_inputs(c) or self.observed_inputs(c) or self.proof_inputs(c)
        if narrow is not None:
            return narrow
        h = hashlib.blake2b(digest_size=16)
        built = self.package_inputs(c)
        for name in reads_of(c):
            if built is not None and name == "crates":
                h.update(b"packages\0" + built.encode("utf-8") + b"\0")
                continue
            h.update(name.encode("utf-8") + b"\0" + self._parts[name].encode("utf-8") + b"\0")
        return h.hexdigest()

    def package_inputs(self, c):
        """What stands in for the `crates` partition in a check whose row in `CARGO_ARGS_READS`
        or `COMMAND_READS` was read off its source: the packages that check builds, and what
        they are compiled against, as `impact.Reach.packages` keys them -- or `None`, and then
        `crates` stands, which is every crate in the workspace.

        Those rows exist because each check costs minutes, and `crates` made every one of them
        stale on an edit to a crate it never compiles: the sanitizer script runs `nvs-host` and
        `nvs-runtime`, and an edit to `nvs-lsp` ran it. A `cargo-named` row's package is its
        `-p`; a `command` row's are the workspace packages named in the file `BUILDS` gives for
        it. Only a check with such a row is answered: any other `cargo-named` check may hold a
        test that reads another crate's sources."""
        if self._tiers is None:
            return None
        names = None
        if c["kind"] == "cargo-named":
            args = tuple(c.get("args", []))
            package = args[args.index("-p") + 1:args.index("-p") + 2] if "-p" in args else ()
            if package and (args in CARGO_ARGS_READS or package in CARGO_ARGS_READS):
                names = list(package)
        elif c["kind"] == "command":
            for needle, source in BUILDS:
                if runs(c.get("argv", []), needle):
                    names = self.reach().named_in(source)
        if not names:
            return None
        parts = self.reach().packages(names)
        if parts is None:
            return None
        # The two root files `crates` holds that no package's key does.
        parts = parts + [(rel, self.digest_of(rel)) for rel in ("nvs.toml", "deny.toml")]
        return impact._digest("packages", parts)

    def reach(self):
        """`impact.Reach` over the tree this sweep hashed: one reading a sweep, with the jobs on
        record and every answer `binary_inputs` has given so far."""
        if self._reach is None:
            self._reach = impact.Reach(self._tiers)
            self._reach.jobs, self._reach.answers = impact.last_jobs(), {}
        return self._reach

    def observed_inputs(self, c):
        """A Python gate's inputs as `tools/observe.py` last saw them, or `None`, and then
        `EVERYTHING` stands -- which holds the handoff, so the check is stale at every wrap.

        The key is the bytes of every file the gate opened or asked the size of, the names in
        every directory it listed, and the set of paths in the tree. The last is there because a
        test for whether a path exists opens nothing: `check-links.py` asks that of every link
        target, and a file that goes away has to move its key. A file git ignores or
        `NOT_INPUTS` prunes is left out, as it is everywhere here; one under `target/` stands for
        the binary and is keyed as `CODE`.

        Why the last run's reads are enough: a gate reads the same files again unless one of them,
        a listing or the path set has changed, and any of those moves this key and runs it, which
        records what it reads now. `None` for a gate that has never run here, and for one that
        starts a process this cannot answer for: `git ls-files` is the path set, the `nvs`
        binary is `CODE` and the paths in its arguments, and anything else reads what it likes."""
        if c["kind"] != "command" or not is_observed(c.get("argv", []), c.get("cwd", ".")):
            return None
        seen = self._observed.get(observed_key(c["argv"], c.get("cwd", ".")))
        if not seen or c.get("setup"):
            return None
        files, dirs, code = set(seen["files"]), set(seen["dirs"]), False
        for argv in seen["spawns"]:
            if len(argv) == 1:  # a command handed over as one string
                argv = shlex.split(argv[0], posix=False)
            program = Path(argv[0]).name.lower() if argv else ""
            if program in ("git", "git.exe") and argv[1:2] == ["ls-files"]:
                continue
            if program not in ("nvs", "nvs.exe"):
                return None
            code = True
            for a in argv[1:]:
                rel = Path(a).as_posix().strip("/")
                if not Path(a).is_absolute() and (ROOT / rel).exists():
                    (dirs if (ROOT / rel).is_dir() else files).add(rel)
        h = hashlib.blake2b(digest_size=16)
        h.update(b"observed\0" + self._parts[PATHS].encode("utf-8") + b"\0")
        for rel in sorted(files):
            if rel.split("/")[0] == "target":
                code = True
            elif not self.is_ignored(rel):
                h.update(rel.encode("utf-8") + b"\0" + self.digest_of(rel).encode("utf-8") + b"\0")
        for rel in sorted(dirs):
            if self.is_ignored(rel):
                continue
            try:
                names = sorted(n for n in os.listdir(ROOT / rel)
                               if not self.is_ignored(f"{rel}/{n}" if rel != "." else n))
            except OSError:
                names = ["\0absent"]
            h.update(rel.encode("utf-8") + b"\0" + "\0".join(names).encode("utf-8") + b"\0\0")
        if code:
            h.update(b"code\0" + self._parts[CODE].encode("utf-8"))
        return h.hexdigest()

    def proof_inputs(self, c):
        """A `proof_groups` check's inputs, one `proofs: <group>` unit per group as `nv why` prints
        it: the proof binary's `crates` and the other `PROOF_PARTITIONS`, `PROOF_INPUTS`, and the
        example, attack and bench paths `nv proofs` last recorded for each group in `PROOF_READS`.
        A check on its own key rather than the run's is what lets an edit to one group's example
        re-run that group alone. `None` for any other check, and for a group with no record yet,
        and then `EVERYTHING` stands until it has run once."""
        groups = proof_groups(c)
        if groups is None or self._parts is None:
            return None
        try:
            reads = json.loads(PROOF_READS.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return None
        if any(g not in reads for g in groups):
            return None
        h = hashlib.blake2b(digest_size=16)
        h.update(b"proofs\0")
        for name in PROOF_PARTITIONS:
            h.update(name.encode("utf-8") + b"\0" + self._parts[name].encode("utf-8") + b"\0")
        own = sorted(set(PROOF_INPUTS).union(*(reads[g] for g in groups)))
        for rel in own:
            h.update(rel.encode("utf-8") + b"\0" + self.path_digest(rel).encode("utf-8") + b"\0")
        return h.hexdigest()

    def path_digest(self, rel):
        """A tree path's content hash: a file's bytes, or a directory's every file that is not
        `is_ignored`, by path; `absent` for a path that is not there."""
        top = ROOT / rel
        if not top.is_dir():
            return self.digest_of(rel)
        if f"{rel}/" in self._digests:
            return self._digests[f"{rel}/"]
        h = hashlib.blake2b(digest_size=16)
        for dirpath, dirnames, filenames in os.walk(top):
            here = Path(dirpath).relative_to(ROOT).as_posix()
            dirnames[:] = sorted(d for d in dirnames if not self.is_ignored(f"{here}/{d}"))
            for name in sorted(filenames):
                sub = f"{here}/{name}"
                if not self.is_ignored(sub):
                    h.update(sub.encode("utf-8") + b"\0" + self.digest_of(sub).encode("utf-8") + b"\0")
        self._digests[f"{rel}/"] = h.hexdigest()
        return self._digests[f"{rel}/"]

    def is_ignored(self, rel):
        """Is this tree path one the memo does not hash -- ignored by git, or under a directory
        `NOT_INPUTS` prunes?"""
        parts = rel.split("/")
        if set(parts) & NOT_INPUTS or rel in self._ignored or f"{rel}/" in self._ignored:
            return True
        return any("/".join(parts[:i]) + "/" in self._ignored for i in range(1, len(parts)))

    def digest_of(self, rel):
        """One file's content hash, read once a sweep; `absent` for a file that is not there."""
        if rel not in self._digests:
            try:
                data = (ROOT / rel).read_bytes()
                self._digests[rel] = hashlib.blake2b(data, digest_size=16).hexdigest()
            except OSError:
                self._digests[rel] = "absent"
        return self._digests[rel]

    def binary_inputs(self, c):
        """A `cargo test -p <crate>` check's inputs as `tools/impact.py` keys them -- one key per
        test binary of that crate, over what that binary reads -- or `None`, and then
        `CARGO_READS` stands, which is every crate and every case tree.

        `CARGO_READS` holds all of `crates/` because a partition cannot say which crate a binary
        is compiled from, so an edit to `nvs-lsp` staled every check naming `nvs-syntax`. The
        binaries are the ones `verify.py` last built (`impact.last_jobs`), and what each opens
        while it runs is what `verify.py` last recorded for it. `None` whenever that cannot be
        shown: a check that is not the plain `-p <crate>` form, a crate with no binary on
        record, or a binary `impact` calls wide."""
        if c["kind"] != "cargo-named" or self._tiers is None:
            return None
        plain = plain_crate_test(c.get("args", []))
        if plain is None:
            return None
        self.reach()
        if plain in self._reach.answers:
            return self._reach.answers[plain]
        self._reach.answers[plain] = None
        crate, target = plain
        jobs = [j for j in self._reach.jobs if j.get("owner") == crate
                and (target is None or j.get("name") == f"{crate} test {target}")]
        if not jobs:
            return None
        h = hashlib.blake2b(digest_size=16)
        for job in sorted(jobs, key=lambda j: j["name"]):
            key, wide = self._reach.key(job)
            if wide:
                return None
            h.update(job["name"].encode("utf-8") + b"\0" + key.encode("utf-8") + b"\0")
        self._reach.answers[plain] = h.hexdigest()
        return self._reach.answers[plain]

    def remembered(self, c, leg=""):
        """Was this check green over inputs bit-identical to the ones on disk right now?

        Never with `full` set, never when the tree could not be hashed, and never for a check that
        says `memoize = false`. That key is for a check whose inputs are not all in the tree: the
        extension-host run reads a downloaded editor build under `.vscode-test/` and an installed
        package tree, both of which `NOT_INPUTS` prunes, so a remembered verdict there would report
        a green editor run on a machine where no editor started (docs/decisions/0185.md section 3).
        Pure: `plan_size` asks this before the sweep to count what it will pay for, and `skip` is
        what records an answer the sweep actually took."""
        if self.full or c.get("memoize") is False:
            return False
        want = self.inputs_for(c)
        return want is not None and self._green.get(self.memo_key(c, leg)) == want

    def skip(self, c, leg="", what=""):
        """`remembered`, taken: the key goes on `skipped` for the cost line, and the status line
        says what was not run.

        Unless this run made the key green itself. The list names one check many times -- the
        conformance tree once per stage that leans on it -- and `remember` files the first pass
        before the duplicates are reached, so they answer from the memo too. That is not a verdict
        taken from the file: the check ran, this run, over these inputs, and it does not go on
        `skipped`, so the cost line counts only what the file answered."""
        if not self.remembered(c, leg):
            return False
        key = self.memo_key(c, leg)
        label = what or f"{leg + ' ' if leg else ''}{c.get('name') or c.get('file')}"
        if key not in self._ran_green and self.audits(c):
            self._claimed.add(key)
            plain = plain_crate_test(c.get("args", []))
            if plain is not None:
                self._audited.add(plain)
            self.trace(f"{label} (green on these inputs already -- run again, as the audit of that)")
            return False
        if key in self._ran_green:
            self.trace(f"{label} (ran above)")
            return True
        if key not in self.skipped:
            self.skipped.append(key)
        self.trace(f"{label} (green on these inputs already)")
        return True

    def audits(self, c):
        """Is this a check the sweep runs even though the memo answers it, to find out whether
        the memo was right?

        A key over partitions holds every file of the trees a check could read. A key from
        `binary_inputs` or `observed_inputs` holds what the check was SEEN or SHOWN to read,
        which is narrower and rests on more: that a test leaves its package only through
        `nvs_repo`, that a script reads again what it read last time. So whenever the floor gate
        is open -- one sweep in `FLOOR_GATE_EVERY`, the sweep a goal is reached on, landing and
        `--settle` -- a sample of the checks with such a key runs whatever the memo says, and
        `run_cargo_check` names a red one a SELECTOR MISS: the key lacked something the check
        reads, and that is a bug in the key rather than in the tree.

        A sample, one check in `AUDIT_EVERY`, and not every one of them. Auditing all of them made
        every gate-open sweep run over a thousand checks the memo had already answered, and it was
        most of the wait between a goal's last session and its landing, while no audit found a miss.
        The sample is drawn from the check's key and the tree's fingerprint, so a sweep over the same
        tree audits the same checks and a new tree audits different ones: a key that is missing
        something is found over a few trees rather than on the first one. A `package_inputs` key is
        not audited -- it stands for the checks that cost minutes, and what it rests on is the cargo
        graph."""
        if not self.floor_gate or self._parts is None:
            return False
        if (self.binary_inputs(c) is None and self.observed_inputs(c) is None
                and self.proof_inputs(c) is None):
            return False
        draw = hashlib.blake2b(f"{self.memo_key(c)}\0{self._tree}".encode("utf-8"), digest_size=8)
        return int.from_bytes(draw.digest(), "big") % AUDIT_EVERY == 0

    def remember(self, c, leg=""):
        want = self.inputs_for(c)
        if want is not None:
            key = self.memo_key(c, leg)
            self._green[key] = want
            self._ran_green.add(key)
            self._green_dirty = True

    def save_green(self):
        """Write the memo out, keeping only keys this goal can ask about again -- a check struck
        from the goal file, or a floor folded away, leaves nothing behind. Once per sweep rather
        than per verdict: seven hundred rewrites of one file a run is what that would be. What
        the Python gates were seen to read is written beside it, on the same terms."""
        if getattr(self, "_observed_dirty", False):
            self._observed_dirty = False
            with contextlib.suppress(OSError):
                RUNDIR.mkdir(parents=True, exist_ok=True)
                CHECKREADS.write_text(json.dumps(self._observed, sort_keys=True),
                                      encoding="utf-8", newline="\n")
        if not self._green_dirty:
            return
        keys = {self.memo_key(c) for c in self.checks if c["kind"] not in PROGRAM_KINDS}
        keys |= {self.memo_key(c, "native") for c in self.all_programs}
        keys |= {self.memo_key(spec) for spec in self.leg_specs.values()}
        green = {k: v for k, v in sorted(self._green.items()) if k in keys}
        try:
            GOALCACHE.parent.mkdir(exist_ok=True)
            GOALCACHE.write_text(
                json.dumps({"tree": self._tree, "partitions": self._parts, "green": green},
                           indent=1),
                encoding="utf-8", newline="\n",
            )
            self._green_dirty = False
        except OSError:
            pass

    def load_green(self):
        self._tree = self.tree_id()
        self._parts = self.partition_ids()
        self._green = {}
        try:
            entry = json.loads(GOALCACHE.read_text(encoding="utf-8"))
            green = entry.get("green")
            if isinstance(green, dict):
                self._green = {k: v for k, v in green.items() if isinstance(v, str)}
        except (OSError, ValueError):
            pass

    # -- one program check on one leg -------------------------------------------------

    def program_check(self, leg, c):
        # `needs`, per `CHECK_KEYS`: a fixture whose claim one platform cannot make runs on the legs
        # that can. Here rather than at the three sweeps that call this, so none of them can forget
        # it and the legs cannot come to disagree about what they skip.
        #
        # What it costs is that such a claim is guarded only where a leg answering the question
        # actually runs, and the WSL leg is conditional on `wsl_available` -- so on a Windows
        # machine with no WSL an `af-unix` check passes the sweep without having asked anything.
        # That is the same trade the valgrind sweep already makes, and the reason `needs` is for a
        # claim a platform cannot make rather than for one that is merely slow or awkward there.
        need = c.get("needs")
        if need and not LEG_NEEDS[need](leg):
            self.trace(f"{leg.name} {c['file']} -- skipped: this leg has no {need}")
            return ""

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
            if proof_groups(c) is not None:
                r = self.timed(label, lambda: self.proofs_verify(c))
            elif runs_release_cli(c):
                with RELEASE_CLI_LOCK:
                    r = self.timed(label, lambda: self.command(argv, c.get("cwd", ".")))
            else:
                r = self.timed(label, lambda: self.command(argv, c.get("cwd", ".")))
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
            return ""

        if c["kind"] == "nvs-suite":
            # The suite runner is the CLI the leg already built; `cargo run` here would be one
            # more workspace fingerprint scan to start a binary sitting on disk. Through the leg
            # rather than at the binary, because `--leg-only` runs these on the Linux build, whose
            # path Windows cannot execute. Shared by every check naming the same args on the same
            # leg, exactly as `cargo()` shares a cargo run -- see the class doc.
            r = self.timed(label, lambda: self.suite(
                leg, c["args"], exact=c.get("min_passing") is not None))
        elif plain := plain_crate_test(c["args"]):
            # `cargo test -p <crate>`, bare or with one `--test <name>`: the crate's binaries off
            # the shared workspace build rather than a cargo run of its own, which would resolve
            # features over that one package and write a second copy of every workspace crate --
            # the class doc, and AGENTS.md's rule. Anything else -- `--release`, a feature -- is a
            # different build and keeps its own invocation.
            r = self.timed(label, lambda: self.crate_tests(*plain))
        elif c["args"] == WORKSPACE_TEST:
            # The same shared build, asked for whole: see `workspace_tests`.
            r = self.timed(label, self.workspace_tests)
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
        if self.skip(self.leg_specs["valgrind sweep"], what="valgrind sweep"):
            return ""
        if leg.name == "native" and shutil.which("valgrind") is None:
            self.trace("valgrind sweep skipped -- no valgrind on this platform")
            return ""  # not a failure: this platform simply has no valgrind leg

        targets = [f for f in self.files if f not in self.valgrind_skip and f in self.swept_files()]
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

        def config_for(f):
            # `VALGRIND_CONFIG` owns which fixture and why. Naming any `--config` turns off the
            # search for `./nvs.toml`, so the repository's own file is named first and the
            # fixture's is layered over it, later-wins. Pathed as `supp` is, for its reason.
            if f not in VALGRIND_CONFIG:
                return ""
            base, over = "nvs.toml", VALGRIND_CONFIG[f]
            if leg.name != "wsl":
                base, over = str(ROOT / base), str(ROOT / over)
            return f"--config {base} --config {over} "

        def cmd_for(f):
            return (f"cd {leg.repo} && " if leg.name == "wsl" else "") + (
                f"valgrind --error-exitcode={vg_error} --leak-check=full "
                f"--errors-for-leak-kinds=definite --suppressions={supp} -q "
                f"{leg.binary} {config_for(f)}run {f}"
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
            started = time.monotonic()
            r = self.timed(f"valgrind {f}", lambda: shell(cmd_for(f)))
            return r, time.monotonic() - started

        # Submitted longest first, by each fixture's cost on this leg the last time it swept: a
        # fixture that walks to a resource ceiling (`examples/limits.nvs`) runs far longer under
        # memcheck than the rest, and one that starts mid-pool is the sweep's whole tail. A fixture
        # with no recorded cost goes first, because it may be the long one. The verdicts are read
        # back in the goal's own order, so the failure reported first is the first fixture in the
        # list however the workers finished. Nothing short-circuits: a red sweep runs all of them
        # and names EVERY leaking fixture, because "one fixture leaks" and "twelve do" are
        # different bugs, and the parallel sweep pays a fraction of what a serial one would to
        # answer both.
        last = prof.get("fixture_s") or {}
        order = sorted(targets, key=lambda f: -last.get(f, float("inf")))
        fails, began = [], time.monotonic()
        with ThreadPoolExecutor(max_workers=jobs) as pool:
            running = {f: pool.submit(sweep, f) for f in order}
            done = {f: running[f].result() for f in targets}
        for f in targets:
            r, _ = done[f]
            if r.code == vg_error:
                fails.append(f"valgrind {f}: exit {r.code} -- {r.first_err_line}")
        # What the width bought, against the serial cost measured on this same box, and what each
        # fixture cost, which orders the next sweep's submission. Recorded rather than printed: it
        # is how a later run says whether the policy is still right here, and it costs nothing to
        # keep.
        spent = time.monotonic() - began
        if prof.get("sample_s") and spent > 0:
            machine.remember(leg.name, sweep_s=round(spent, 1),
                             sweep_speedup=round(prof["sample_s"] * len(targets) / spent, 2))
        machine.remember(leg.name, fixture_s={f: round(s, 1) for f, (_, s) in done.items()})
        if fails:
            return fails[0] + (f"  (and {len(fails) - 1} more: "
                               f"{', '.join(x.split(':')[0] for x in fails[1:])})"
                               if len(fails) > 1 else "")
        # A sweep over the goal's own fixtures alone is not the sweep this memo names.
        if self.floor_gate:
            self.remember(self.leg_specs["valgrind sweep"])
        return ""

    # -- the whole thing ----------------------------------------------------------------

    def report_program_fails(self, fails):
        """One line for the driver's ledger out of every failing program check.

        The EARLIEST-stage failure leads, because that is the one a session can close: a later
        stage's fixture is usually red *because* of it, and reporting the later one sends the
        session at a member three stages of unwritten work away. The rest are named after it by
        stage, without their output -- a session that needs one reads it out of the goal file or
        runs the fixture, and it is the shape of the goal's red edge that this line exists to
        carry, not five error messages.
        """
        fails = sorted(fails, key=lambda f: f[0])
        (_, _, first) = fails[0]
        if len(fails) == 1:
            return first
        others = []
        for _key, c, _msg in fails[1:]:
            label = f"{c.get('file', c.get('name', '?'))} [{c.get('stage', '?')}]"
            if label not in others:
                others.append(label)
        return (f"{first}\n"
                f"       (and {len(fails) - 1} later fixture(s) red, in stage order: "
                f"{', '.join(others[:6])}"
                + (f", +{len(others) - 6} more" if len(others) > 6 else "") + ")")

    def plan_size(self, mode, wsl):
        """How many `timed()` steps the sweep about to start will pay for.

        Countable exactly, and that is the whole reason the status line shows a percentage here and
        nowhere else: the list is fixed on disk, the legs are known before the first build, and the
        two memos say up front what will be skipped rather than discovering it halfway through. Call
        it only after `load_green()`, or every remembered check is counted as work still to do.

        It is an upper bound in one direction only -- a failing check returns early, so a run can
        end at 40% -- and it never undercounts, so the bar cannot reach 100% with work left.
        """
        programs = len(self.swept_programs())
        wsl_leg, vg = self.leg_specs["wsl leg"], self.leg_specs["valgrind sweep"]
        swept = self.swept_files()
        sweep = sum(1 for f in self.files if f not in self.valgrind_skip and f in swept)
        # Only the wsl leg's valgrind is a given: on a native leg the sweep is skipped outright
        # when the platform has no valgrind, and counting it would strand the bar short of 100%.
        sweepable = not self.remembered(vg) and bool(wsl or shutil.which("valgrind"))

        n = 1  # the input fingerprint, already spent by the time this is called
        if mode == "leg":
            n += 1 + programs  # the leg's build, then every fixture on it
            n += sum(1 for c in self.cargo_checks if c["kind"] == "nvs-suite")
            return n + (sweep if sweepable else 0)

        if self.fast_check() is not None:
            n += 1  # last session's failing check, tried before anything is built
        n += 1  # the native build
        # `audits` runs a remembered check anyway, so it is work the bar has to count.
        def owed(c):
            return not self.remembered(c) or self.audits(c)

        n += sum(1 for c in self.catch_up_checks if owed(c))
        n += sum(1 for c in self.setup_checks if owed(c))
        n += sum(1 for c in self.swept_programs() if not self.remembered(c, "native"))
        # The shared workspace test build, paid once by the first plain `cargo test -p` check.
        if any(plain_crate_test(c.get("args", [])) or c.get("args") == WORKSPACE_TEST
               for c in self.catch_up_checks + self.cargo_checks):
            n += 1
        n += sum(1 for c in self.swept(self.cargo_checks) if owed(c))
        n += sum(1 for c in self.swept(self.overlap_checks) if not self.remembered(c))
        if self.floor_gate:
            n += sum(1 for c in self.release_checks if not self.remembered(c))
        # The whole Linux leg -- probe, build and fixtures -- is skipped when its two consumers are
        # both green over these inputs, so none of the three is counted then either.
        if not (self.remembered(wsl_leg) and self.remembered(vg)):
            n += 1  # the wsl probe
            if wsl:
                n += 1 + (0 if self.remembered(wsl_leg) else programs)
        return n + (sweep if sweepable else 0)

    # -- the floor gate ----------------------------------------------------------------

    def swept(self, checks):
        """The checks of `checks` this sweep runs: all of them with the gate open, and with it
        shut everything but the carried floor. `_check` says what the gate is for."""
        if self.settle_only:
            return [c for c in checks if is_carried(c)]
        if self.floor_gate:
            return list(checks)
        return [c for c in checks if not is_carried(c)]

    def swept_programs(self):
        """`all_programs`, less the carried floor when the gate is shut -- what a leg runs."""
        return self.swept(self.all_programs)

    def swept_files(self):
        """The fixture files a leg runs this sweep, for the valgrind sweep to run the same set."""
        return {c["file"] for c in self.swept_programs()}

    def hold(self, checks, what):
        """Note what a shut gate is not running, once per tier, so the cost line says so and the
        driver knows this sweep's green is not the goal's."""
        kept = [c for c in checks if is_carried(c)]
        if kept and not self.floor_gate:
            self.held += [c.get("name") or c.get("file") for c in kept]
            self.trace(f"floor gate shut -- {len(kept)} carried {what} held until it opens")

    def begin(self, mode="check"):
        """Reset the per-run bookkeeping, and confirm every fixture is still on disk before
        anything is built. Shared by the two entry points below."""
        self.ran = []
        self._times_written = 0
        self._claimed = set()
        self.held = []
        self._cargo = {}
        self._suite = {}
        self._commands = {}
        self._proofs = {}
        self._overlap = {}
        self._wsl_build = None
        self._exes = None
        self._crate_runs = {}
        self._verify_green = None
        self._audited = set()
        self.reused = 0
        self.reds = []
        self.short = []  # thresholds not met yet, judged after everything else
        self.skipped = []
        self._ran_green = set()
        self._begun = time.monotonic()
        TICKER.set(done=0, total=0)
        # Two hashes: the `git diff HEAD` behind `tree_id`, and the content walk of the tree behind
        # `partition_ids`. Both are a visible pause before any check has started, so the ticker is
        # told what is happening rather than appearing to hang on nothing.
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
            try:
                return self._check(verbose)
            finally:
                # A red sweep returns with gates still queued; nothing will read them.
                if self._pool:
                    self._pool.shutdown(wait=False, cancel_futures=True)
                    self._pool = None
                self.save_green()

    def _check(self, verbose=False):
        self.verbose = verbose
        trace = self.trace

        TICKER.set(phase="acceptance check")
        fail = self.begin("check")
        if fail:
            return fail

        # Before ANY build, because the whole value of it is not paying for one: see `fast_fail`.
        # Not in a collecting sweep, which is paying for every check to name every red one, and
        # runs this one in its place in the list.
        fail = "" if self.collect else self.fast_fail()
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
            if self.skip(c, what=f"cargo {c['name']} (catch-up)"):
                continue
            trace(f"cargo {c['name']} (catch-up)")
            fail = self.run_cargo_check(c, native)
            if fail:
                return fail
            self.remember(c)

        # The fixtures' environment, before the first of them and whatever the floor gate is doing:
        # a queue fixture claims a row out of a table `nvs queue migrate` creates, and a sweep that
        # ran the migration afterwards reported a fixture that could never have passed. `setup` in
        # `CHECK_KEYS` is the key, and it is the goal file's only way to say "before the floor".
        for c in self.setup_checks:
            if self.skip(c, what=f"setup {c['name']}"):
                continue
            trace(f"setup {c['name']}")
            fail = self.run_cargo_check(c, native)
            if fail:
                return fail
            self.remember(c)

        # Behind the setup tier, which may be preparing what one of these reads, and ahead of
        # everything else, which is the whole of what they overlap.
        self.start_overlap(native)

        # Held for the whole sweep and not just for the fixtures: on a Linux host `leg` stays
        # `native`, so this is also the origin the valgrind sweep's own run of `examples/http.nvs`
        # reaches.
        self.origins.enter_context(local_origin(native))
        # The FLOOR's fixtures, before the test build: a regression here outranks everything the
        # current goal is doing, and catching it without paying for a test build is why this tier
        # runs first. Every one of them, not the first failing one -- the same trade the valgrind
        # sweep makes below, and for the same reason: "one floor fixture broke" and "eleven did"
        # are different bugs. A sweep that is not collecting stops HERE, so a red floor never goes
        # on to pay for the cargo checks, the WSL leg, the valgrind sweep or the release checks; a
        # collecting one carries on, and `ran_past` says which sweeps those are.
        #
        # Unless the floor gate is shut, when the carried floor -- here, in `cargo_checks`, on the
        # WSL leg and under valgrind -- is held, and the sweep is the current goal's own list: its
        # catch-up, its stages, its fixtures. `FLOOR_GATE_EVERY` owns why and what it risks; the
        # short form is that the carried floor is nearly all of a sweep's cost and almost none of
        # what a session is told, since the pack names the goal's earliest red check and a floor
        # regression is one session in twenty-five. A goal is never reached on a held floor:
        # `held` is non-empty, and the driver runs the list again, gate open, before it declares
        # anything -- with the memo consulted, so that sweep pays for what the goal changed.
        fails = []
        self.hold(self.program_floor, "fixture(s)")
        for c in self.swept(self.program_floor):
            if self.skip(c, native.name):
                continue
            trace(f"{native.name} {c['file']}")
            fail = self.program_check(native, c)
            if fail:
                fails.append((stage_key(c), c, fail))
            else:
                self.remember(c, native.name)
        if fails:
            fail = self.report_program_fails(fails)
            if not self.ran_past(fail):
                return fail

        self.hold(self.cargo_checks, "cargo check(s)")
        self.start_wsl_build()
        self.start_pool()
        for c in self.swept(self.cargo_checks):
            if self.skip(c, what=f"cargo {c['name']}"):
                continue
            trace(f"cargo {c['name']}")
            fail = self.run_cargo_check(c, native)
            if fail:
                if self.ran_past(fail):
                    continue
                return fail
            self.remember(c)

        # NOW the current goal's own fixtures, behind the cargo checks of the stages they sit on
        # top of. `Goal.__init__` owns why this is the order; the short version is that a goal's
        # fixture is its acceptance surface, so reporting it ahead of the unit checks underneath
        # tells a session only that the end is not built yet.
        fails = []
        for c in self.program_checks:
            if self.skip(c, native.name):
                continue
            trace(f"{native.name} {c['file']}")
            fail = self.program_check(native, c)
            if fail:
                fails.append((stage_key(c), c, fail))
            else:
                self.remember(c, native.name)
        if fails:
            fail = self.report_program_fails(fails)
            if not self.ran_past(fail):
                return fail

        # Windows is green, so now pay for the Linux leg.
        #
        # The build is skipped only when BOTH things that would use the binary are already green
        # over these inputs -- the leg's own fixtures and the valgrind sweep behind it. That pairing
        # is the safety: if either still has to run, the build runs, so nothing ever reaches a
        # missing or stale binary. Skipping the build alone would be worse than useless, because
        # `leg` would stay `native` and the sweep would quietly downgrade to a platform with no
        # valgrind on it.
        leg = native
        wsl_leg = self.leg_specs["wsl leg"]
        trace("asking whether there is a wsl leg")
        if not self.swept_programs():
            # Every fixture is carried and the gate is shut: nothing for a second leg to run, and
            # the valgrind sweep below finds the same empty list.
            trace("wsl leg skipped -- the floor gate holds every fixture this list has")
        elif self.remembered(self.leg_specs["valgrind sweep"]) and self.skip(
                wsl_leg, what="wsl leg and valgrind sweep both green on these inputs -- "
                              "neither is rebuilt"):
            pass  # `valgrind()` below notes its own skip
        elif self.timed("wsl probe", wsl_available):
            trace("building the wsl CLI")
            leg, fail = self.wsl_built()
            if fail:
                return fail
            # Before the branch below rather than inside it: the sweep runs on this leg even when
            # its fixtures are already green on these inputs, and it runs `examples/http.nvs` too.
            self.origins.enter_context(local_origin(leg))
            if not self.skip(wsl_leg, what="wsl fixtures"):
                # Same sweep-then-report as the native leg above: a Linux-only divergence is
                # worth knowing the extent of, not just the first instance of.
                leg_fails = []
                for c in self.swept_programs():
                    trace(f"{leg.name} {c['file']}")
                    fail = self.program_check(leg, c)
                    if fail:
                        leg_fails.append((stage_key(c), c, fail))
                if leg_fails:
                    fail = self.report_program_fails(leg_fails)
                    if not self.ran_past(fail):
                        return fail
                # A leg over the goal's own fixtures alone is not the leg this memo names.
                if self.floor_gate and not leg_fails:
                    self.remember(wsl_leg)

        trace("valgrind sweep")
        fail = self.valgrind(leg)
        if fail and not self.ran_past(fail):
            return fail

        # The `overlap` commands, running since the setup tier. Judged here because this is the
        # last point ahead of the release checks, which must not start while one is still running:
        # from here the wait is whatever of the command's own clock the sweep did not cover.
        self.hold(self.overlap_checks, "overlapped command(s)")
        for c in self.swept(self.overlap_checks):
            if self.skip(c, what=f"command {c['name']}"):
                continue
            trace(f"command {c['name']} (overlapped since the setup tier)")
            fail = self.run_cargo_check(c, native)
            if fail:
                if self.ran_past(fail):
                    continue
                return fail
            self.remember(c)

        # The `--release` checks, held back from their stages to here. Two reasons, and the second
        # is the one that must not be traded away:
        #
        # * `prebuild` has been building them since before the native build, and this is the point
        #   at which it has had the whole sweep to finish in. Measured: reached in its stage-0
        #   position, the sweep waited 1m50s for a build with 41s of work in front of it and 1m57s
        #   behind it; from here the wait is nothing and the run is 3s.
        # * They are cost-class guards, and a cost measured while a WSL build and a valgrind sweep
        #   are running is not the cost. Here, everything else has finished, which is necessary and
        #   not sufficient -- `asked_again` is what a guard red from the sweep's shadow costs.
        #
        # What it costs is reporting order: a red guard is now named after a red fixture rather than
        # before one. That is the smaller loss -- and a red guard is not reported LATER in wall-clock
        # terms either, because the sweep it now runs behind is shorter than the build it used to
        # wait on.
        for c in self.release_checks:
            if self.skip(c, what=f"cargo {c['name']}"):
                continue
            if not self.floor_gate:
                self.held.append(c["name"])
                trace(f"cargo {c['name']} (floor gate shut -- the release profile is held with it)")
                continue
            trace(f"cargo {c['name']}")
            fail = self.run_cargo_check(c, native)
            if fail:
                fail = self.asked_again(c, native, fail)
            if fail:
                if self.ran_past(fail):
                    continue
                return fail
            self.remember(c)

        if self.reds:
            return self.all_reds()
        # Last, because a corpus that is merely still growing is the one failure that must not hide
        # anything: everything above is a claim about whether the language is correct on both legs
        # and leaks nothing, and all of it has now run. See `cargo_check`'s `min_passing` arm.
        return self.short[0] if self.short else ""

    def ran_past(self, fail):
        """Whether a collecting sweep carries on past the red check `fail`, which it then keeps
        for `all_reds`. A sweep that is not collecting stops at its first red, and this says no.

        A sweep stops at its first red while a goal is in progress, because its own later stages
        are red for the ordinary reason that nobody has built them yet, and the pack names the
        earliest red check. The sweeps that collect are the ones where every red is a finding: the
        gate-open sweep that would reach a goal, landing a side goal, and `--settle`. There a sweep
        that stopped at its first red paid for a whole session and a whole sweep per red check,
        one after another, when one sweep could have named them all. A build that fails still
        stops every sweep, since nothing behind it can run."""
        if not self.collect:
            return False
        self.reds.append(fail)
        return True

    def all_reds(self):
        """A collecting sweep's verdict: the first red check in sweep order, whole, and then each
        other one as an `also red:` line of its own, which `orient.py` prints with it. The first
        line is the one `check_of` reads, so the DONE-claim retry keys on the same check as before."""
        first, rest = self.reds[0], self.reds[1:]
        lines = [first] + [f"       also red: {r.split(chr(10))[0].strip()}" for r in rest]
        return "\n".join(lines)

    # -- the two things that let a sweep cost less than all of it ------------------------

    def run_cargo_check(self, c, leg=None):
        """`cargo_check`, remembering WHICH check a sweep died on so the next one can try it
        first. Only a `cargo-named` check is remembered, because only that kind is cheap enough
        to be worth trying alone -- `fast_fail` says what that costs and what it buys."""
        fail = self.cargo_check(c, leg)
        # The first red of a collecting sweep, which is the one its verdict leads with.
        if (fail and c["kind"] == "cargo-named" and "--release" not in c.get("args", [])
                and not (self.collect and self.reds)):
            self.failed_name = c["name"]
        if fail and self.memo_key(c) in self._claimed:
            ledger(f"       SELECTOR MISS: {c['name']} -- green in the memo, red when run")
            fail = (f"SELECTOR MISS -- the memo called this green over these inputs and it is "
                    f"red, so its key is missing something it reads (`audits`): {fail}")
        return fail

    def asked_again(self, c, leg, first):
        """A red cost-class check, asked again on a machine given each of `COST_SETTLES` to come back.

        Only the `--release` checks reach this, and only once everything else in the sweep has
        finished -- the constant's own note has the measurements that say finishing is not enough,
        and that the shadow's tail outlives the first wait. `first` is the verdict that opened the
        ladder, kept so a pass says which red it was that recovered.

        The cached run is dropped before each ask, or `cargo()` hands back the very verdict being
        asked again. That one cache is the only one to drop: `plain_crate_test` wants `test -p`, so a
        `--release` check never takes the shared-build path."""
        waited, last = 0, first
        for settle in COST_SETTLES:
            self.trace(f"cargo {c['name']} came back red -- {settle}s for the machine, then again")
            time.sleep(settle)
            waited += settle
            self._cargo.pop(tuple(c.get("args", [])), None)
            again = self.cargo_check(c, leg)
            if not again:
                self.trace(f"cargo {c['name']} passed after {waited}s: {first}")
                return ""
            last = again
        return f"{last} (asked {len(COST_SETTLES) + 1} times, over {waited}s)"

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


    def leg_check(self, verbose=False):
        """The Linux leg, with the origin its network fixtures need held open around it -- the same
        shape as `check` above, and for the same reason."""
        with contextlib.ExitStack() as origins:
            self.origins = origins
            try:
                return self._leg_check(verbose)
            finally:
                self.save_green()

    def _leg_check(self, verbose=False):
        """The Linux leg on its own: every fixture, both suites and the valgrind sweep against a
        Linux build. The cargo suites are left out because they do not divide by target -- what
        this leg exists to catch is a calling-convention divergence in the JIT or a leak in the
        refcount protocol, and both of those show up through the CLI.

        This was `tools/wsl-acceptance.sh` (check-links:retired), and it is a method rather than
        a shell script because that script carried its own frozen copy of every fixture's expected
        output. A second copy
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

        for c in self.all_programs:
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
        self.write_times()
        wall = time.monotonic() - self._begun
        worst = sorted(self.ran, key=lambda x: -x[1])[:3]
        slow = ", ".join(f"{label} {s:.0f}s" for label, s in worst if s >= 1)
        skipped = f", {len(self.skipped)} remembered" if self.skipped else ""
        reused = f", {self.reused} test binaries from verify.py's run" if self.reused else ""
        audited = f", {len(self._claimed)} audited" if self._claimed else ""
        held = f", {len(self.held)} held (floor gate shut)" if self.held else ""
        reds = f", {len(self.reds)} red" if self.reds else ""
        return (f"{wall:.0f}s over {len(self.ran)} check(s){skipped}{reused}{audited}{held}{reds}"
                + (f"; slowest: {slow}" if slow else ""))

    def write_times(self):
        """Append what each check of this sweep cost to `CHECKTIMES`, one line per check:
        `{"at", "label", "seconds", "gate", "full"}`. `summary` names the three slowest and the
        ledger keeps only that line, so which check is worth narrowing could not be read off any
        file. Each entry of `ran` is written once, however often `summary` is asked. A file that
        cannot be written costs the measurement and nothing else."""
        fresh = self.ran[getattr(self, "_times_written", 0):]
        self._times_written = len(self.ran)
        if not fresh:
            return
        at = round(time.time())
        lines = "".join(json.dumps({"at": at, "label": label, "seconds": round(spent, 2),
                                    "gate": bool(self.floor_gate), "full": bool(self.full)},
                                   ensure_ascii=False) + "\n" for label, spent in fresh)
        try:
            RUNDIR.mkdir(parents=True, exist_ok=True)
            if CHECKTIMES.is_file() and CHECKTIMES.stat().st_size > CHECKTIMES_CAP:
                kept = CHECKTIMES.read_text(encoding="utf-8").splitlines(keepends=True)
                CHECKTIMES.write_text("".join(kept[len(kept) // 2:]), encoding="utf-8",
                                      newline="\n")
            with CHECKTIMES.open("a", encoding="utf-8", newline="\n") as out:
                out.write(lines)
        except OSError:
            pass


def owed(goal):
    """`--owed`: the carried checks no memo answers for the tree as it stands, and exit status 1
    if there is one. Nothing is run.

    A change made outside a run -- by hand, or by an interactive session -- stales exactly the
    checks that read what it touched, and nothing runs them until the floor gate next opens.
    This is that debt, read off the same memo and the same keys a sweep uses, so what it names
    is what `--settle` would pay for. `tools/git-hooks/pre-push` asks it, which is what keeps a
    commit nothing has checked from leaving the machine. The goal's own checks are not counted:
    they are red until the goal is reached, which is the goal's business and not a debt. Nor is
    a check that says `memoize = false`, which no memo can answer and every open gate runs."""
    goal.load_green()
    if goal._parts is None:
        say("owed: the tree could not be hashed, so every carried check is owed", C.RED)
        return 1
    names = []
    for c in goal.checks:
        if not is_carried(c) or c.get("memoize") is False:
            continue
        leg = "native" if c["kind"] in PROGRAM_KINDS else ""
        if not goal.remembered(c, leg):
            names.append(c.get("name") or c.get("file"))
    if not names:
        say("owed: nothing -- every carried check is green over this tree", C.GREEN)
        return 0
    shown = ", ".join(names[:6]) + (f", +{len(names) - 6} more" if len(names) > 6 else "")
    say(f"owed: {len(names)} carried check(s) are not green over this tree: {shown}", C.YELLOW)
    say("      `python tools/loop.py --settle` runs them, and only them", C.GRAY)
    return 1


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

    A side run's list is its own `.toml` with main's carried floor inserted at the marker, and its
    own WSL target; `tools/side.py` § *What a side run is checked against* says why.
    """
    if not SIDE:
        return Goal(tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")))
    try:
        text = sidemod.effective_spec(GOAL_TOML, ROOT / sidemod.LIVE_GOAL)
    except Exception as e:  # goal-switch's CarryError, loaded by path and so not nameable here
        raise GoalError(f"main's floor could not be carried into it -- {e}") from e
    goal = Goal(tomllib.loads(text))
    goal.wsl_target = sidemod.wsl_target(goal.wsl_target, SIDE.slug)
    return goal


def goal_title(chain=None):
    """The loop-goal half of the status line's goal row: the H1 of `docs/agent/loop-goal.md`,
    behind `goal <slug> 29/43` when a chain is driving. Read from disk each time, for the reason
    `load_goal()` is: a chain switch rewrites the file, and the row has to follow it. A file that
    cannot be read, or has no heading, leaves the row saying so rather than ending the run.

    The H1's own `Loop goal N — ` prefix is cut. That header is the one place a goal states its
    number, and it belongs there -- but the row already names the goal to the left of it, so
    leaving it in printed the number twice and the name not at all.
    """
    title = ""
    try:
        for line in GOAL_MD.read_text(encoding="utf-8").splitlines():
            if line.startswith("# "):
                title = re.sub(r"^(Loop goal \d+|Side goal)\s*[—-]\s*", "", line[2:].strip())
                break
    except OSError:
        pass
    title = title or f"{rel_to_root(GOAL_MD)} has no heading"
    if SIDE:
        return f"side goal {SIDE.slug}{TICKER.sep}{title}"
    if chain is not None and chain.current is not None:
        return (f"goal {chain.current.slug} {chain.current.num}/{len(chain.goals)}"
                f"{TICKER.sep}{title}")
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


#: Where the driver leaves the commit a session opened on, for the tools that run *inside* that
#: session and cannot otherwise know. Only `session.py`'s link gate reads it today. It is one field
#: rather than a shape worth versioning: anything else a session wants to know about its own start
#: is derivable from the sha.
SESSION_BASE = RUNDIR / "session-start.json"


def write_session_base(base):
    """Record the sha a session is opening on, or clear the record when there is no sha.

    Cleared rather than left stale on purpose. A reader that finds a sha trusts it, so a file
    surviving from a previous run would hand the next session a baseline older than its own work
    and hide exactly the breakage it exists to catch. Absent means "ask HEAD", which is what every
    interactive session gets and is the behaviour this file changes nothing about."""
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        if base:
            SESSION_BASE.write_text(json.dumps({"base": base}) + "\n", encoding="utf-8")
        else:
            SESSION_BASE.unlink(missing_ok=True)
    except OSError:
        pass  # the ledger is a convenience; a run never fails over one


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
        minute the row used to name a slice that had already been committed and handed off.

        The base is also written to disk, because this object holds the only record of where a
        session began and `session.py`'s link gate needs it: that gate reads a finding as inherited
        when it is already in HEAD, and a session that commits a slice by hand before wrapping
        moves HEAD under itself and has its own breakage read as somebody else's. That is not the
        rare case its docstring assumed -- it is how a dead link reached `main` and turned CI's
        `docs` job red, from a session whose transcript shows a hand-rolled `git commit -F`."""
        with self.lock:
            self.base = self.head = base or ""
            self.item = "orienting -- no item picked yet"
            self.commits = 0
            self.subject = ""
            self._next_slow = 0.0
        write_session_base(base)
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


def dirty_entries():
    """`(status, path)` for every path `git status` calls dirty.

    `-z` rather than the default spelling: a path holding a space or a byte outside ASCII comes
    back quoted otherwise, and `git add` on the quoted form stages nothing. A rename contributes
    both of its paths, because staging one and not the other leaves the tree dirty either way.

    Not through `git()`, which strips its output. A record is `XY<space><path>` and `Y` alone is
    set for a change that is not staged, so the first record of an ordinary dirty tree begins
    with a space -- stripping it shifts that one record by a character and the path comes back
    with its first letter missing."""
    try:
        out = subprocess.run(["git", "status", "--porcelain", "-z"], cwd=ROOT,
                             capture_output=True, encoding="utf-8", check=True).stdout
    except (subprocess.CalledProcessError, OSError):
        return []
    fields = [f for f in out.split("\0") if f]
    entries, i = [], 0
    while i < len(fields):
        field, i = fields[i], i + 1
        if len(field) < 4:
            continue
        code, path = field[:2], field[3:]
        entries.append((code, path))
        if ("R" in code or "C" in code) and i < len(fields):
            entries.append((code, fields[i]))
            i += 1
    return entries


def repo_path(text):
    """A tool call's `file_path` in the spelling `git status` uses: repo-relative, forward slashes.

    Sessions are handed absolute paths and post them back that way -- `D:\\mwl\\crates\\...` on
    this machine -- so comparing one against `git status` output raw matches nothing at all."""
    try:
        return Path(text).resolve().relative_to(ROOT).as_posix()
    except (ValueError, OSError):
        return str(text).replace("\\", "/").lstrip("./")


class SessionFiles:
    """Which dirty paths the SESSION wrote, so the sweep can leave the rest of the tree alone.

    `mark_interrupted` commits what a session left behind, and this decides what it may stage.
    Staging the whole dirty tree is right for a tree only the loop touches and wrong for the one
    this repository has: a person edits `docs/` while the loop runs, and their open files then go
    into a `wip(loop)` commit naming a session that never opened them.

    **A path is the session's only if something watched it being written.** Not "it is dirty and
    nobody else claimed it" -- the sweep takes what it can name and leaves everything else, so the
    failure mode is a stray file still sitting in the tree, which is visible, rather than a
    person's afternoon inside a commit addressed to a machine.

    Two things do the watching, and between them they cover every write this repository permits:

    - **The event stream.** `note` reads the target off every write tool as the block goes past.
    - **`written.py`.** `splice.py` and `reference.py` write files nothing on the stream names --
      a patch reaches any number of targets behind one `Bash`, and `docs/novis.md` is regenerated
      under `verify.py` with no tool call mentioning it at all. Both report what they wrote.

    A shell that carries content into the tree by itself is outside both, and is also the first
    rule in `AGENTS.md`: an edit that Write and Edit cannot express goes through `splice.py`. So
    the uncovered case is a broken rule, and the sweep leaving that file dirty is how it surfaces.

    Read tools are deliberately not recorded. Sessions grep the goals directory and read the plan
    constantly, and a set counting reads would re-claim every file a person has open.

    A subagent's writes are the one real gap: the parent stream carries its prompt, not its edits,
    and `collect_subagents` reads its transcript only after the sweep has run. Sessions almost
    never spawn one, and what it leaves behind is reported as left rather than lost."""

    #: Tools that write. `Bash` is deliberately absent: its command can write anything at all, and
    #: `written.py` is how the tools it runs answer for themselves instead of being guessed at.
    WRITES = ("Write", "Edit", "MultiEdit", "NotebookEdit")

    def __init__(self):
        self.lock = threading.RLock()
        self.written: set[str] = set()

    def start(self, carry=False):
        """A session is about to launch: forget the last one's paths and empty the tool ledger.

        `carry` keeps both across the one launch that is not a new session -- a dropped stream
        rejoined with `claude --resume`, whose first half wrote files that are sitting dirty in
        the tree right now. Clearing there would hand the session its own work back as a
        stranger's, and the sweep at the end of the resumed half would refuse to take it."""
        with self.lock:
            if carry:
                return
            self.written = set()
        try:
            WRITTEN.write_text("", encoding="utf-8", newline="\n")
        except OSError:
            pass

    def note(self, name, args):
        """One `tool_use` block, as it streams past the renderer."""
        if str(name) not in self.WRITES or not isinstance(args, dict):
            return
        for key in ("file_path", "notebook_path"):
            value = args.get(key)
            if isinstance(value, str) and value.strip():
                with self.lock:
                    self.written.add(repo_path(value))

    def paths(self):
        """Everything either watcher saw, the stream's set unioned with the tools' own ledger."""
        with self.lock:
            seen = set(self.written)
        try:
            seen |= {ln.strip() for ln in WRITTEN.read_text(encoding="utf-8").split("\n")
                     if ln.strip()}
        except OSError:
            pass
        return seen

    def split(self, entries):
        """`(ours, theirs)` over `dirty_entries()` output, each side in the order it arrived."""
        seen = self.paths()
        ours, theirs = [], []
        for code, path in entries:
            (ours if path in seen else theirs).append((code, path))
        return ours, theirs


TOUCH = SessionFiles()


# --------------------------------------------------------------------------------- chain


class ChainError(Exception):
    """A chain file that cannot be walked. Raised at start-up, before a session is launched, for
    the same reason `GoalError` is: a run of hundreds of sessions must not discover on its fourth
    day that entry five names a file nobody wrote."""


class Chain:
    """The staged goals under `docs/agent/goals/`, walked in numeric order by the driver itself.

    **The directory is the schedule.** A goal is `N-<slug>.md` plus, until it is retired, a sibling
    `.toml` and `.handoff.md`; the number is the position and the numbers run `1..N`. When the live
    goal's acceptance list goes green the driver **advances**: it runs `tools/goal-switch.py`
    against the next goal -- which folds the goal that just passed into it as its floor -- copies
    the three files into place, commits that switch, and starts the next session. Without this a
    six-goal program stops five times and waits for a human, which is the same program with five
    extra nights in it.

    `tools/goals.py` is the reader and this holds no second parser. Two things are deliberately not
    automated, and both are in the class of "expensive to get wrong and cheap to do once":

    * **The floor is carried by `goal-switch.py`, as a subprocess.** Re-implementing that text
      splice here would be a second copy of the one operation whose failure mode is silent -- a
      missing floor looks exactly like a passing one.
    * **`goal-switch.py` is not idempotent**: it inserts at a marker it leaves in place, so running
      it twice inserts the floor twice. `.loop/chain.json` is what makes "exactly once per goal"
      a fact rather than an intention, and it survives a driver that is killed mid-run.
    """

    def __init__(self):
        self.goals = self._load()
        self.index = self._restore()
        fail = self._retired_error(self.goals, self.index)
        if fail:
            raise ChainError(fail)

    def _load(self):
        """Every goal on disk, validated. `ChainError` on anything unwalkable."""
        chain = goalsmod.load()
        if not chain:
            raise ChainError(f"{rel_to_root(GOALS_DIR)} holds no goal")
        fail = goalsmod.numbering_error(chain)
        if fail:
            raise ChainError(fail)
        for g in chain:
            # A **retired** goal is one the run has already left. `chain.py --retire` proved its
            # whole acceptance list had been folded forward and deleted the two files that held it,
            # leaving the `.md` so nothing citing the prose broke. So there is one file to validate
            # and no list to run: it is history with a number, and `install_next` never reaches
            # back for it. `_retired_error` is the other half -- that it really is behind the run,
            # and not something about to be installed.
            for path in g.files:
                if not path.is_file():
                    raise ChainError(f"goal {g.num} ({g.slug}) names {rel_to_root(path)}, which "
                                     f"does not exist")
            if g.retired:
                continue
            # Existing is not the same as walkable. A misspelled key in a queued goal's list is an
            # authoring mistake with a three-day fuse: nothing reads that file until the switch
            # into it, which is hours of sessions after the goal before it went green, and the
            # driver's answer there is to stop the run. Read at start-up, it is one line before a
            # single session is launched -- exactly what this class refuses a missing file for.
            fail = spec_error(g.toml)
            if fail:
                raise ChainError(f"goal {g.num} ({g.slug}) names {rel_to_root(g.toml)}, whose "
                                 f"acceptance list this driver cannot run -- {fail}")
        return chain

    @staticmethod
    def _retired_error(goals, index):
        """A retired goal the run has not already left, as one line, or `""`.

        Retirement is only ever true *behind* the run: it deletes the acceptance list a goal was
        walked on, so a goal at or after the live one is one the driver would be asked to install
        off files that are gone. `-1` -- a tree with no `.loop/chain.json` -- has left nothing
        behind it at all, and a retired prefix there is refused rather than inferred into a
        position: which goals have been walked is exactly what that file is for, and guessing it
        from what somebody deleted is how a floor gets folded in twice.
        """
        for i, g in enumerate(goals):
            if not g.retired or i < index:
                continue
            if index < 0:
                return (f"goal {g.num} ({g.slug}) is retired -- the acceptance list it was walked "
                        f"on has been deleted -- but {rel_to_root(CHAINSTATE)} records no "
                        f"installed goal, so this run would start by trying to install it. Restore "
                        f"that file, or start the run against a chain whose first goal still has "
                        f"one.")
            return (f"goal {g.num} ({g.slug}) is retired but the run stands on goal "
                    f"{goals[index].num}, so it is the live goal or ahead of it. Retiring is what "
                    f"says a goal's checks are already somebody's floor; this one's are not.")
        return ""

    def refresh(self):
        """Re-read the directory, so a chain edited under the run is walked as it now stands.

        `dossier.py --emit-goals` is the reason this exists: a goal whose whole job
        is to write the next hundred cannot hand them to a driver that read the chain once at
        start-up, and stopping the run for a human to restart is the thing the chain exists to
        avoid. A goal *inserted* in front of a later one is the same need arriving from the other
        side, and it is adopted for the same reason.

        **What is protected is the walked prefix, not the whole list.** `goal-switch.py` folds each
        walked goal's checks into the one after it at switch time, so a goal at or before the live
        one that changed is not something to follow -- the floor those switches built no longer
        matches what is on disk. Nothing has been folded into a goal the run has not reached, so
        those may be inserted, edited or appended freely; the guard is the invariant and never more
        than it. That rewrite, and a goal file that stops parsing mid-run, both leave the snapshot
        in place and the run continues on it: every goal it is walking is still on disk, so there
        is nothing here worth ending three hundred sessions over.

        Returns a one-line note for the console, or "" when nothing changed.
        """
        where = rel_to_root(GOALS_DIR)
        try:
            fresh = self._load()
        except ChainError as e:
            return f"chain: {where} changed and is not walkable -- {e}"
        fail = self._retired_error(fresh, self.index)
        if fail:
            return f"chain: {where} changed and is not walkable -- {fail}"
        # `-1` is "nothing installed yet", which protects nothing: no switch has folded a floor
        # into anything, so every goal is still free to move.
        walked = max(self.index + 1, 0)
        if [g.slug for g in fresh[:walked]] != [g.slug for g in self.goals[:walked]]:
            return (f"chain: {where} was rewritten across the {walked} goal(s) this run has "
                    f"already walked -- walking the {len(self.goals)} goals it started with")
        was, now = len(self.goals), len(fresh)
        if [g.slug for g in fresh] == [g.slug for g in self.goals]:
            return ""
        self.goals = fresh
        if now != was:
            return (f"chain: {where} is {now} goal(s) where it was {was} -- the run walks it as it "
                    f"stands, without a restart")
        return (f"chain: {where} changed ahead of the live goal -- the run walks it as it stands, "
                f"without a restart")

    def _restore(self):
        """Where the chain stands: the index of the goal `.loop/chain.json` names, or -1.

        **The file holds the goal's NUMBER**, which is the number a person says out loud and also
        its position, so there is no second spelling to keep in step. It held a 0-based index for
        as long as the order lived in a file whose entries could be reordered under a running
        driver; a goal inserted ahead of the live one now changes no number at all. `tools/goals.py`
        is the one reader and writer of it."""
        num = goalsmod.live()
        return next((i for i, g in enumerate(self.goals) if g.num == num), -1) if num else -1

    def _save(self):
        goalsmod.write_live(self.goals[self.index].num)

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
        step(f"chain: switching to goal `{nxt.slug}`, {nxt.num} of {len(self.goals)}", C.CYAN)

        fail = preflight(nxt.preflight)
        if fail:
            return fail

        # The goal as it was authored, before anything on disk moves. `_load` reads the same file
        # at start-up, which is where an authoring error should surface and cost nothing; this is
        # that read again against what is on disk now, since a session may have edited a queued
        # goal since the run began. What the fold then does to it is judged separately below.
        fail = spec_error(nxt.toml)
        if fail:
            return f"chain: goal `{nxt.slug}`'s acceptance list is not runnable -- {fail}"

        # The floor. `goal-switch.py` reads the LIVE goal, so this has to happen before the copy.
        # On the first goal there is no previous chain goal and the live one is whatever the run
        # started against -- which is exactly the floor that goal wants.
        #
        # The pre-fold bytes are held because the fold is the one step here that cannot be repeated:
        # `goal-switch.py` inserts at a marker it leaves in place, so a goal left folded by a
        # refusal below gets the same floor a second time on the next run. Putting these back is the
        # whole of that repair, and it is only the whole of it while nothing else has moved yet.
        goal_path = nxt.toml
        unfolded = goal_path.read_bytes()
        r = capture(sys.executable, [str(ROOT / "tools" / "goal-switch.py"),
                                     rel_to_root(goal_path)])
        if r.code != 0:
            return f"chain: goal-switch failed for `{nxt.slug}` -- {r.first_err_line}"
        for line in stdout_lines(r.out):
            say(f"  {line}", C.GRAY)

        # The folded list is what a session is actually spent against, and folding is what makes a
        # list unrunnable that read fine unfolded: the floor arrives naming fixtures, and whether
        # the entry's own `files` came to hold them is decided by the union above. Judged here, one
        # `write_bytes` undoes everything this method has done.
        try:
            Goal(tomllib.loads(goal_path.read_text(encoding="utf-8")))
        except (tomllib.TOMLDecodeError, GoalError) as e:
            goal_path.write_bytes(unfolded)
            return (f"chain: goal `{nxt.slug}`'s acceptance list is not runnable once the floor is "
                    f"folded into it -- {e}")

        shutil.copyfile(goal_path, GOAL_TOML)
        # The prose and the handoff are markdown full of relative links, and installing them moves
        # them one directory up -- out of `docs/agent/goals/` and into `docs/agent/`. Copying the
        # bytes verbatim breaks every one of them, which `check-links.py` reports and nothing else
        # notices, because orient.py prints link *text* and a session never follows one to find out.
        GOAL_MD.write_text(
            relocate_links(read_text(nxt.md), nxt.md.parent.name),
            encoding="utf-8", newline="\n",
        )
        (ROOT / "docs" / "agent" / "handoff.md").write_text(
            relocate_links(read_text(nxt.handoff), nxt.handoff.parent.name),
            encoding="utf-8", newline="\n",
        )

        self.index += 1
        self._save()

        # `.loop/goal-green.json` is carried across the switch. A verdict there is filed under the
        # check's own spec and the bytes it read (`Goal.memo_key`, `Goal.inputs_for`), not under
        # the goal that asked, so the floor this goal inherits arrives already answered for every
        # check whose inputs the goal has not touched yet.

        # The goal the run has just LEFT. Every one of its checks is in the file above -- that is
        # what the fold did four calls ago -- so this is the one moment its own copy is provably
        # redundant, and `chain.py --retire` re-proves it before unlinking anything. Without this
        # the goals directory keeps a full floor per walked goal forever: six of them were 830K of
        # text no tool reads, and `dossier.py` is about to append 93 more.
        #
        # It is hygiene, so a refusal is printed and the run goes on. Nothing downstream needs the
        # file to be gone, and stopping a three-hundred-session run over a deleted file that is
        # still there would be the tail wagging the dog.
        retired = []
        prev = self.goals[self.index - 1] if self.index >= 1 else None
        if prev is not None and not prev.retired:
            gone = [rel_to_root(prev.toml), rel_to_root(prev.handoff)]
            r = capture(sys.executable, [str(ROOT / "tools" / "chain.py"), "--retire",
                                         str(prev.num)])
            for line in stdout_lines(r.out):
                say(f"  {line}", C.GRAY)
            if r.code == 0:
                # `retired` is derived from the `.toml` being gone, so the snapshot follows the
                # disk with no flag to set: what has to be staged is the two deletions.
                retired = gone
            else:
                say(f"  chain: goal `{prev.slug}` was not retired -- {r.first_err_line}", C.GRAY)

        # The plan's derived cells, re-derived now that a carrier has left the chain. A milestone
        # whose every carrier has walked reads `done` in its `Carried by` cell, and `plan.py
        # --check` -- on every goal's floor -- refuses the cell that still names the goals. The
        # moment that cell changes is this one, so the sync rides in the switch commit; left to a
        # session, the stale cell was found by the floor of the goal after, as a DONE claim held
        # for a hand. Hygiene like the retirement above: a refusal is printed and the run goes on.
        synced = []
        if prev is not None:
            r = capture(sys.executable, [str(ROOT / "tools" / "plan.py"), "--sync"])
            for line in stdout_lines(r.out):
                say(f"  {line}", C.GRAY)
            if r.code == 0:
                synced = ["docs/implementation-plan.md"]
            else:
                say(f"  chain: the plan's cells were not synced -- {r.first_err_line}", C.GRAY)

        # The one line here that outlives the run. It names the goals and never their numbers: `git
        # log` is read long after a later insert has moved every number, and a subject line saying
        # `advances to 29 xml-tree` would by then name a different goal with no way to tell. The
        # subject is the whole message -- why a switch is mechanical is docs/agent/goals/README.md's.
        came = f" from `{prev.slug}`" if prev is not None else ""
        dropped = f", and `{prev.slug}` is retired" if retired else ""
        message = f"docs(loop): the chain advances{came} to `{nxt.slug}`{dropped}\n"
        msg_file = ROOT / ".agent-tmp" / "chain-switch.txt"
        msg_file.parent.mkdir(parents=True, exist_ok=True)
        msg_file.write_text(message, encoding="utf-8", newline="\n")
        # `git add` on a path that is gone stages the deletion, so the retirement rides in this
        # commit rather than sitting in the tree for whichever session commits next.
        git("add", rel_to_root(nxt.toml), "docs/agent/loop-goal.toml", "docs/agent/loop-goal.md",
            "docs/agent/handoff.md", *retired, *synced)
        git("commit", "-F", str(msg_file))
        say(f"chain: goal `{nxt.slug}` is live, {nxt.num} of {len(self.goals)}", C.GREEN)
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
            return (f"chain: `docker compose -f {compose} up` failed -- {compose_error(r)}. "
                    f"The live goal's checks need those services.")
        # `[docker.copy]`: a file a check needs on disk that only exists inside a container. The CA
        # the compose `certs` service issues is the case it was written for -- `nvs.toml`'s
        # `[db.main] tls_ca_file` names it, it belongs to a Docker volume rather than to git, and a
        # fixture that reached the server without it would be one verifying nothing.
        for dest, source in (docker.get("copy") or {}).items():
            r = capture("docker", ["compose", "-f", compose, "cp", source, dest], timeout=120)
            if r.code != 0:
                return (f"chain: `docker compose cp {source} {dest}` failed -- {compose_error(r)}. "
                        f"The live goal's checks need that file on disk.")
        return ""


class SideChain(Chain):
    """What a side run walks in place of the chain: its one goal, already installed.

    A side goal is run where it sits, so there is nothing to switch into and nothing to refresh,
    and reaching it ends in `land_side` rather than `install_next`. `bring_up_services` is the
    chain's own, reading the side goal's `[docker]` through the redirected `GOAL_TOML`."""

    def __init__(self):
        fail = spec_error(SIDE.toml)
        if fail:
            raise ChainError(f"side goal `{SIDE.slug}` names {rel_to_root(SIDE.toml)}, whose "
                             f"acceptance list this driver cannot run -- {fail}")
        self.goals, self.index = [SIDE], 0

    def refresh(self):
        return ""

    def install_next(self):
        return f"side goal `{SIDE.slug}` is the whole of a side run; there is no goal to switch to"


def compose_error(r):
    """Why a `docker compose` command failed, quoted from its stderr.

    Not `first_err_line`: compose writes its progress to stderr too, one indented
    ` Container novis-db-certs-1 Recreate` line per step, so the first stderr line is a step that
    happened and never the reason the command died. The reason is the first line that is not a
    step -- compose's own are unindented, `Error response from daemon: ...` and `dependency failed
    to start: ...` -- plus the tail, because the run has not opened its console log yet when this
    runs and the verdict is the only record the failure leaves. When every line is a step, the last
    one is the step it was on."""
    lines = [line.rstrip() for line in r.err.splitlines() if line.strip()]
    if not lines:
        return r.first_err_line
    reason = next((line.strip() for line in lines if not line.startswith(" ")), lines[-1].strip())
    tail = "\n".join(f"       | {line.strip()}" for line in lines[-6:])
    return f"{reason}\n{tail}"


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
    * `N-<name>.toml` and `README.md` are siblings in `goals/`, and from `docs/agent/` they are
      `goals/N-<name>.toml` and `goals/README.md`.

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

    Only one exists: `preflight = "docker"`, for the goal whose drivers `rule:core-classes/db-one-api` verifies against
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


#: How `drive` ended, as a *kind* rather than a sentence. `reason` is written for a person and is
#: reworded whenever the wording improves; this is what `turn` branches on, and only `served` and
#: `rejudge` are followed by another turn without a person. Adding a kind here is free; changing
#: one renames an API.
#:
#:   served          the turn's session was served and nothing below happened: `SERVED`
#:   rejudge         the session changed driver code this process had imported, so the next
#:                   turn judges it with no session of its own: `driver_changed`
#:   budget          the run has served its --max-sessions
#:   chain-complete  the last goal in the chain is green
#:   chain-error     a chain switch could not be made
#:   asked           `.loop/stop`, or `s` at the console
#:   wall            MAX_WALLS sessions in a row refused by the usage limit
#:   wall-timeout    a usage window that does not reopen inside --max-limit-wait
#:   cli-failed      --max-retries consecutive non-zero exits from the CLI
#:   done-claim      a session claimed DONE that the acceptance test does not agree with, and
#:                   the retry session failed the same check it was handed, or the goal is out
#:                   of `DONE_RETRIES`
#:   blocked         a session wrote BLOCKED
#:   stalled         --max-stalls sessions in a row produced no commit
#:   interrupted     Ctrl-C
SERVED = "served"


class Run:
    """What one turn of a run leaves for the next: `.loop/run.json`.

    A turn is a process and serves one session, so everything the driver counts ACROSS sessions
    lives here and not in a local. It is deliberately short. A streak that a served session
    resets -- CLI failures, usage walls, overloads, resumes of one dropped stream -- never crosses
    a turn, because the turn does not end until a session is served or the run does, and stays a
    local in `drive`. What is left:

        run           the name `tools/respawn.py` gave this run; a file naming another is a dead
                      run's, and is ignored whole
        run_id        the stamp every log of the run is named by, so `loop-stats.py --run` and the
                      log retention in `disk.py` both read the run as one unit
        served        sessions that count against `--max-sessions`
        index         the last log index used, which only ever goes up; see `drive`
        stalls        consecutive served sessions that committed nothing
        done_retries  DONE-claim retries the live goal has spent
        retry_check   the check the next session is handed as one, or ""
        last_hand     the verdict the run last held on, so the same one twice ends it
        judge         a served session no sweep has judged yet, because it changed the driver's
                      own code: its log index, status line, commit count and base sha, or {}
        repair        the verdict the next session repairs instead of doing goal work, or ""
        repairs       repair sessions the live goal has spent: `REPAIRS_PER_GOAL`

    The sessions since the last optimization pass are not here: `.loop/optimization/state.json`
    has always held that count across runs, and `credit` is its one writer outside a checkpoint.

    Written through a rename, because a turn can be cut off anywhere and a half-written file
    would read as a run that had served nothing."""

    DEFAULTS = {"run": "", "run_id": "", "served": 0, "index": 0, "stalls": 0,
                "done_retries": 0, "retry_check": "", "last_hand": [], "judge": {},
                "repair": "", "repairs": 0}

    def __init__(self, stamp):
        state = read_json(RUNSTATE, default={}) or {}
        if not isinstance(state, dict) or state.get("run") != stamp:
            state = {}
        #: The first turn of the run: nothing has been said, written or counted yet.
        self.fresh = not state
        for name, default in self.DEFAULTS.items():
            value = state.get(name, default)
            setattr(self, name, value if isinstance(value, type(default)) else default)
        self.run = stamp
        if self.fresh:
            self.run_id = run_stamp()

    def save(self):
        body = json.dumps({name: getattr(self, name) for name in self.DEFAULTS}, indent=2) + "\n"
        try:
            RUNDIR.mkdir(parents=True, exist_ok=True)
            scratch = RUNSTATE.with_suffix(".json.tmp")
            scratch.write_text(body, encoding="utf-8", newline="\n")
            os.replace(scratch, RUNSTATE)
        except OSError:
            pass  # an unwritable `.loop` costs the next turn its counts, never this one its session


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


# --------------------------------------------------------------- a stream that dropped
#
# The third exit that is not a crash, and the only one the driver answers by CONTINUING rather
# than starting over.
#
# A refused session never ran: the account said no, or the server did, and there is nothing on
# disk but the refusal. A DROPPED session is the opposite -- it ran, it was healthy, and what
# died was the connection carrying the answer back. Its transcript is complete in the harness's
# own project directory, and `claude --resume <id>` replays it.
#
# What that is worth is measured: on 2026-09-07, run 20260907-193447, session 0002 lost its
# stream nine minutes and 24 turns into a group, at $2.89, with 786 lines of `nvs-lsp` in the
# tree. The driver swept the tree, started a FRESH session, and that session spent its opening
# minutes rebuilding a 78 KB pack and reading back the wip commit to work out what its
# predecessor had been doing. The work survived; the reasoning did not, and it did not have to
# go.

#: What a session's terminal event calls the end when its stream died rather than the server
#: answering. Paired with a null `api_error_status`, because that pairing is the whole signature:
#: a refusal carries a status -- 529, 500 -- and is a verdict the driver already classifies
#: above, while a drop carries none at all, since nothing answered to supply one.
DROP_REASON = "api_error"

#: How many times in a row one dropped session is resumed before the driver gives up on the
#: transcript and starts fresh. Three, because a resume that fails three times is no longer a
#: network blip, and the fresh path -- sweep, re-orient, read the wip commit -- is the one that
#: recovers from a transcript the harness will not replay.
MAX_RESUMES = 3

#: What the driver waits before resuming a dropped session, by consecutive attempt; the last
#: entry repeats. Short, and far shorter than `OVERLOAD_BACKOFF`, because a dropped connection is
#: not a busy server: there is nothing to wait out, and every second spent waiting is spent
#: holding a context that is only worth resuming while the run still wants it.
RESUME_BACKOFF = (15, 30, 60)

#: What a resumed session is told, in place of the session prompt it already has. It answers the
#: three questions a session cut off mid-answer actually has -- whether anything moved under it,
#: whether its last call landed, and what it still owes. Everything else it needs, the goal and
#: the pack and the rules, is in the transcript being replayed and is not worth a second copy.
RESUME_PROMPT = (
    "The connection to the API dropped mid-response and this session was rejoined with "
    "`--resume`, so the conversation above is yours and you are continuing it.\n"
    "\n"
    "Nothing moved under you. The working tree is exactly as you left it and the driver "
    "committed nothing on your behalf. Your last tool call may or may not have landed -- read "
    "back whatever it touched rather than assuming either way.\n"
    "\n"
    "Pick up where you stopped and finish the session the way the prompt at the top told you "
    "to, ending with the wrap. Do not re-orient, and do not restart the group."
)


def stream_dropped(line):
    """True when this session's terminal `result` event blames a connection that died mid-answer.

    Three fields off that one event, never the words: `is_error` is set, so a session that merely
    quoted the phrase cannot supply it; `terminal_reason` is `api_error`; and `api_error_status`
    is null, because nothing answered with a status. The last is what separates a drop from a
    refusal, and the two need opposite recoveries."""
    try:
        e = json.loads(line)
    except json.JSONDecodeError:
        return False
    if e.get("type") != "result" or not e.get("is_error"):
        return False
    return e.get("terminal_reason") == DROP_REASON and not e.get("api_error_status")


def resume_wait(n):
    """How long to wait before the nth consecutive resume of a dropped session."""
    return RESUME_BACKOFF[min(max(n, 1), len(RESUME_BACKOFF)) - 1]


#: The subject of the commit a swept session leaves behind. Matched by `orient.py` and by
#: `holes.py`, so it is spelled once here rather than three times in prose.
SWEEP_SUBJECT = "wip(loop): the unfinished slice of"


def mark_interrupted(index, why=None):
    """End a session with a CLEAN TREE, whatever cut it off. Returns the path count it swept.

    A session stopped mid-slice has committed everything it FINISHED -- one commit per slice is
    what buys that -- but whatever it was in the middle of is still uncommitted, and the handoff
    it never reached does not mention it.

    **So the driver commits it.** Leaving it dirty was the old behaviour and it fails in the one
    way that matters: the next session finds those files and cannot tell them from the state it
    was supposed to start in, and if the RUN ends there -- `s` at the console, the last session of
    a `--max-sessions` batch -- nothing ever picks them up. That is not a hypothetical; it is how
    1,200 lines of goal `schema` stage 6 sat uncommitted across a stopped run on 2026-09-06, including a
    `docs/novis.md` that `verify.py` had regenerated under a session that then never wrapped.

    An unverified commit is the right trade here and the asymmetry is not close. The work is on a
    branch the loop owns, the acceptance sweep runs against it immediately afterwards, and the
    next session is told in the pack. Against that: work that only exists in a working tree is one
    `git checkout` from gone, and nothing in this repository is allowed to depend on a person
    noticing. A commit is recoverable and reviewable; a dirty tree is neither.

    `INTERRUPTED` is still written, and now carries the sweep's own hash -- the next session reads
    it out of the pack and continues from a commit rather than from a diff. The next session to
    end clean deletes it.

    **It sweeps the session's own paths and nothing else.** `TOUCH` splits the dirty tree into what
    something watched this session write and what it did not, and the second half is left exactly
    where it is: the loop shares this working tree with a person, and staging everything dirty
    commits whatever that person has open, however little of it the session is responsible for. A
    sweep with nothing of its own to take is the same as a clean exit -- no commit at all, and the
    interruption cleared.

    `why` is a `RateLimit`, a sentence, or nothing at all -- the things that cut a session off,
    in the order the driver can explain them."""
    entries = dirty_entries()
    ours, theirs = TOUCH.split(entries)
    dirty = [f"{code} {path}" for code, path in ours]
    left = [f"{code} {path}" for code, path in theirs]
    if not ours:
        INTERRUPTED.unlink(missing_ok=True)
        if left:
            say(f"   the tree holds {len(left)} path(s) this session never wrote -- left alone",
                C.GRAY)
        return 0

    said = (why.describe() if isinstance(why, RateLimit)
            else why or "the CLI exited non-zero")
    # Index 0 is the Ctrl-C handler, which is outside any session's scope and says so.
    whose = f"session {index:04d}" if index else "the interrupted run"
    body = (f"{SWEEP_SUBJECT} {whose}\n"
            f"\n"
            f"The session ended before it wrapped -- {said} -- so this is what it had in the\n"
            f"tree at that moment, committed by the driver rather than left for the next one to\n"
            f"find as an unexplained diff. It has NOT been through `verify.py`.\n"
            f"\n"
            + (f"{len(left)} other path(s) were dirty before this session launched and are NOT\n"
               f"in this commit -- they are somebody else's work and are still in the tree.\n"
               f"\n" if left else "")
            + f"`.loop/interrupted.json` names this commit; `orient.py` puts it at the top of the\n"
            f"next session's pack. Continue it, amend it or revert it -- but read it first.\n")
    before = git("rev-parse", "HEAD")
    # By path, never `-A`: everything this session did not write belongs to whoever is working in
    # the tree beside it. Untracked paths are named the same way -- a new module or `.nvst` case
    # is exactly what a mid-slice session has, and `git add` on the path stages it.
    paths = [path for _, path in ours]
    git("add", "--", *paths)
    msg = RUNDIR / "sweep-msg.txt"
    try:
        msg.write_text(body, encoding="utf-8", newline="\n")
        # `--only`, so the commit is these paths whatever else the index holds: a person working
        # in the tree beside the loop may have staged their own edit, and a bare `git commit`
        # would take it. The `add` above is still needed -- `--only` reaches a path git is not
        # yet tracking only once something has put it in the index.
        #
        # `--no-verify` is never used here: the commit-msg hook's rule applies to this message
        # like any other, and this one has no trailer for it to catch.
        git("commit", "-F", str(msg), "--only", "--", *paths)
    except OSError:
        pass
    finally:
        msg.unlink(missing_ok=True)
    swept = git("rev-parse", "HEAD")
    # HEAD, not an exception: `git()` swallows a non-zero exit and answers "", so whether the
    # commit happened is a question only the hash can settle.
    committed = bool(swept) and swept != before
    if not committed:
        # Leave the tree as it was found rather than staged-but-uncommitted, which is the same
        # bug this function exists to remove, one level down. By path again: an unqualified
        # `git reset` here would unstage a person's own staged work along with it.
        git("reset", "--", *paths)
        say("   the unfinished slice could NOT be committed -- it is still in the tree, "
            "uncommitted, and .loop/interrupted.json says so", C.YELLOW)

    try:
        INTERRUPTED.write_text(
            json.dumps({"session": index, "when": f"{datetime.now():%Y-%m-%d %H:%M:%S}",
                        "why": said, "head": swept, "swept": committed, "files": dirty,
                        # What the sweep declined to take, so a tree that is still dirty after one
                        # is explained on disk rather than looking like a half-finished sweep.
                        "left": left}, indent=1),
            encoding="utf-8", newline="\n",
        )
    except OSError:
        pass
    return len(dirty)


class Session:
    """The `claude` child in flight, as the console sees it: something to halt, and something to
    talk to. `Control` holds the one that is running, and `h`, `i`, `.loop/halt` and `.loop/say`
    all end up here.

    **A halt is a freeze, not a kill.** Every process under the session is suspended where it
    stands (`proctree`) and resumed from the same instruction, so nothing is lost and nothing is
    re-sent: no transcript replayed, no tool call cut off half way, no build to start again. The
    one thing a freeze can cost is the API stream, if it is held long enough for the far end to
    give up -- and then the session exits as a dropped stream and `drive` rejoins its transcript,
    exactly as it does for a drop nobody caused.

    It is frozen *for* reasons and thawed when the last one goes, because there are two: a
    person halted it, or a person is typing a prompt for it. Typing under a halt, or halting
    while typing, must not thaw it early.

    **Talking to it** is a user message on its stdin. The session is started with
    `--input-format stream-json`, which is what keeps stdin open as a channel: the first message
    is the session prompt and the pack, and any later one is read by the agent at its next step,
    inside the same run. The price of that mode is that the CLI no longer exits when its reply
    is complete -- it waits for stdin to close -- so `close_input` is called on the `result`
    event, and that is what ends the session."""

    def __init__(self, proc, streaming):
        self.proc = proc
        self.tree = proctree.Tree(proc)
        #: Can it be sent a message? False for `--plain-input`, and from `close_input` on.
        self.streaming = streaming
        self.frozen_for: set[str] = set()
        self.frozen_at = 0.0  # monotonic, when the current freeze began
        self.halts = 0
        self.halted = 0.0  # seconds spent frozen, over every freeze
        self.prompts = 0
        self._stdin = threading.Lock()  # the first message is written from its own thread

    def freeze(self, why):
        """Freeze for `why`. Returns how many processes are stopped."""
        if not self.frozen_for:
            self.frozen_at = time.monotonic()
            self.halts += 1
        self.frozen_for.add(why)
        return self.tree.freeze()

    def thaw(self, why):
        """`why` no longer holds. Thaws when nothing else does; returns seconds spent frozen."""
        if why not in self.frozen_for:
            return 0.0
        self.frozen_for.discard(why)
        if self.frozen_for:
            return 0.0
        spent = time.monotonic() - self.frozen_at
        self.halted += spent
        self.tree.thaw()
        return spent

    def send(self, text, operator=True):
        """One user message down stdin. False when the session is no longer taking any."""
        if not self.streaming or not self.proc.stdin:
            return False
        message = {"type": "user",
                   "message": {"role": "user", "content": [{"type": "text", "text": text}]}}
        try:
            with self._stdin:
                self.proc.stdin.write(json.dumps(message) + "\n")
                self.proc.stdin.flush()
        except (OSError, ValueError):
            return False
        if operator:
            self.prompts += 1
        return True

    def close_input(self):
        """The reply is complete: closing stdin is what lets a streaming session exit."""
        self.streaming = False
        try:
            with self._stdin:
                if self.proc.stdin:
                    self.proc.stdin.close()
        except (OSError, ValueError):
            pass

    def record(self):
        """What a person did to this session, for its ledger line; "" when nobody did anything."""
        bits = []
        if self.halts:
            bits.append(f"halted {self.halts}x for {hms(self.halted)}")
        if self.prompts:
            bits.append(f"{self.prompts} operator prompt(s)")
        return ", ".join(bits)

    def finish(self):
        self.frozen_for.clear()
        self.tree.close()


def session_env():
    """The environment a `claude` child gets: this one, less the run's name.

    `respawn.ENV` is how a turn knows it belongs to the run that holds `.loop/running`. Inherited
    by a session, it would say the same of a `python tools/loop.py` that session typed, and the
    marker that exists to refuse a second driver on this tree would wave it through."""
    env = dict(os.environ)
    env.pop(respawn.ENV, None)
    return env


def run_session(run_id, index, prompt_text, opts, renderer, resume=""):
    """One `claude -p` session, its NDJSON streamed to the console and to
    .loop/logs/<run>-NNNN.log. The run stamp is in the name because the index restarts at 1
    every run: named by index alone, session 3 of today's run appended to session 3 of last
    week's, and any per-session measurement over the directory silently mixed the two.

    The session id off the `system`/`init` event is kept, not just printed: it is the only
    handle on the harness's own transcript directory, and therefore on any subagent this
    session spawned. Without it a delegated read is invisible to every measurement below.

    The file is opened through `CONSOLE` rather than here, and stays open after this returns: the
    acceptance check that judges this session runs next, and its lines belong in this session's
    log. `drive()` closes it once that verdict is in.

    `resume` is a session id whose transcript this launch rejoins instead of starting a new
    conversation: no pack is built, `RESUME_PROMPT` goes on argv in place of the session prompt,
    and the log still gets its own index, so a resumed session is a separate transcript on disk
    and mixes with nothing. It gets its own log line, its own subagent sweep and its own row in
    `loop-stats.py` -- everything except a `loop_pack` line, which it must not have. Returns the
    exit code, that log, the session id, any rate limit, the API status, whether the terminal
    event blamed a dropped stream, and what a person did to the session (`Session.record`)."""
    log = LOGDIR / f"{run_id}-{index:04d}.log"
    CONSOLE.open_session(log)
    renderer.begin()
    exe = shutil.which("claude") or "claude"
    # A resumed session is handed the CONTINUATION, not the session prompt. That prompt is
    # already the first turn of the transcript being replayed, and sending it a second time asks
    # a session standing in the middle of a slice to orient from the top.
    # The prompt goes down stdin as the first message, with the pack behind it -- the order the
    # CLI itself puts an argv prompt and a piped stdin in -- so that stdin stays open as the
    # channel `Session.send` uses. `--plain-input` is the way back to a prompt on argv, for a
    # CLI that one day stops taking streamed input: the run carries on, and `i` does nothing.
    streaming = not opts.plain_input
    opening = RESUME_PROMPT if resume else prompt_text
    cmd = [
        exe,
        "-p",
        *([] if streaming else [opening]),
        *(["--input-format", "stream-json"] if streaming else []),
        "--model",
        opts.model,
        "--permission-mode",
        opts.permission_mode,
        "--output-format",
        "stream-json",
        "--verbose",
    ]
    if resume:
        cmd += ["--resume", resume]
    # Only when it was asked for: the flag and the model's own default are not the same thing to
    # the harness, and passing `--effort high` would record a choice where none was made.
    if opts.effort:
        cmd += ["--effort", opts.effort]
    # `orient.py` runs a `brief.py`, a `git log` and a plan read before the child is even
    # spawned, and on a cold filesystem cache that is tens of seconds between the "== session"
    # banner and the first token. It is the second half of the gap between two sessions.
    #
    # A resume skips all of it. The session being rejoined read a pack already, it is in the
    # transcript, and building a second would spend that minute to hand a session standing
    # mid-slice the map it opened on.
    if resume:
        pack = ""
        step(f"resuming session {resume} -- no pack, its transcript already carries one", C.CYAN)
        SLICES.pick("resumed after a dropped connection")
    else:
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
    dropped = False  # did that same event blame a lost stream? see `stream_dropped`
    # The pack's size, recorded beside the transcript that paid for it. Two sessions with
    # different pack sizes are a two-point regression against their measured `ctx_start`,
    # which is how `loop-stats.py --calibrate` derives bytes-per-token instead of assuming
    # it. Nothing downstream needs this line; every reader skips a `type` it does not know.
    #
    # A resumed session writes NO such line, and the omission is what keeps it out of that
    # regression: `loop-stats.py` skips any session whose `pack_bytes` is falsy, and a session
    # opening on a whole replayed conversation with no pack at all is a point that would tilt
    # the fit by itself.
    #
    # The live goal's slug rides along because a chain switch installs a new manifest: the pack
    # can double across one without any session having written a byte of it, and `loop-stats.py`
    # fits its drift slope inside a goal for exactly that reason.
    if not resume:
        CONSOLE.raw(json.dumps({"type": "loop_pack", "bytes": len(pack.encode("utf-8")),
                                "goal": goalsmod.live_slug()}) + "\n")
    effort = f", --effort {opts.effort}" if opts.effort else ""
    rejoin = f", --resume {resume}" if resume else ""
    step(f"launching {exe} (--model {opts.model}{effort}, "
         f"--permission-mode {opts.permission_mode}{rejoin})")
    TICKER.set(phase="launching", detail=f"{exe} --model {opts.model}{effort}")
    launched = time.monotonic()
    VERIFY.arm()
    proc = subprocess.Popen(
        cmd,
        cwd=ROOT,
        # The one thing this environment carries that the driver's does not: where a tool the
        # session runs reports the files it wrote. Set here and nowhere else, so the same tool
        # run by hand in another terminal -- while this session is in flight -- reports nothing
        # and its edits stay the person's own. `written.py` owns the convention.
        env={**session_env(), written.ENV: str(WRITTEN)},
        **proctree.popen_kwargs(),
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
    session = Session(proc, streaming)
    if streaming:
        first = opening + (f"\n{pack}" if pack else "")
        threading.Thread(target=session.send, args=(first, False), daemon=True).start()
    elif pack:
        threading.Thread(target=feed, args=(proc.stdin, pack), daemon=True).start()
    elif proc.stdin:
        try:
            proc.stdin.close()
        except OSError:
            pass
    CONTROL.attach(session)

    def watch():
        # The keys and the two files are read from the ticker, and a run with its stdout
        # redirected has no ticker: this is what reads them then. Beside the ticker it is a
        # second reader of a non-blocking read under one lock, which costs nothing.
        while proc.poll() is None:
            CONTROL.poll()
            time.sleep(0.25)

    threading.Thread(target=watch, daemon=True).start()
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
                dropped = stream_dropped(line) or dropped
                session.close_input()
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
        # The child does not outlive the process watching it. It is an autonomous agent writing
        # this tree with permissions bypassed, and when the driver died on an encoding
        # error its child kept going unwatched -- committing work the next session then
        # found beside its own, which is what a `BLOCKED two writers` ledger line is
        # made of. The whole tree, frozen or not: a `cargo` the session started is the
        # session's, and on POSIX the child is in a session of its own that a Ctrl-C at
        # this terminal never reaches.
        session.tree.kill()
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            pass
        raise
    finally:
        VERIFY.disarm()
        CONTROL.detach()
        session.finish()
    if proc.returncode and said_limit and not (limit and limit.blocked):
        # A limit reported in prose, with no event carrying a deadline. Come back shortly rather
        # than ending the run: the next session's own event will carry the real one.
        step(f"a usage limit was reported in text but no event named a reset -- treating it as a "
             f"wall and coming back in {hms(BLIND_WAIT)}", C.YELLOW)
        limit = RateLimit({"status": "rejected", "resetsAt": int(time.time()) + BLIND_WAIT})
    return proc.returncode, log, session_id, limit, api_error, dropped, session.record()


# ------------------------------------------------------------------- subagent transcripts
#
# A subagent's turns arrive in the parent's `stream-json` interleaved with the parent's own,
# each tagged with the `parent_tool_use_id` of the call that spawned it -- so a reader taking the
# stream at face value charges a session for calls it never made and reads the step back down to
# the subagent's small context as a compaction. `loop-stats.py`'s `read_session` drops them for
# that reason. What the parent's own window actually holds is the tool call and the report it
# returned. The harness writes each subagent's full transcript next to the parent session's, and
# copying it under .loop/logs/ is what lets loop-stats.py price a delegated read on its own terms
# rather than reporting a session that mysteriously did a lot with very few calls.
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


def enough_disk(opts):
    """Refuse to start a run with too little disk.

    The refusal is the point. A run that fills the disk does not stop cleanly -- it dies inside
    a session with the tree half-edited and the next session inheriting the mess, which is
    exactly what happened on 2026-08-25. Failing at the door instead costs one line. Nothing is
    reclaimed here: `clean_disk` does that at the end of every goal, and at the door, before the
    claim on `.loop/running`, there is no telling whether another run is mid-build.
    `tools/disk.py` owns the policy, the numbers and the explanation."""
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


def clean_disk(opts):
    """`python tools/disk.py --clean`, run in-process after every session's acceptance check.

    In-process because the command refuses while `.loop/running` exists -- a person cannot see
    whether a session is mid-build, and the driver can. The check runs between sessions, so
    nothing is building and it has just left the build warm, which makes the cargo queries
    `disk.clean` asks no-op builds. Every session rather than every goal because a goal is days of
    sessions, and a day of builds is what filled the disk. A release build the check started may
    still be running under `PREBUILD_LOCK`; then this session's sweep is skipped rather than
    waited for, since the next one is a session away. Hygiene, so a failure is printed and the
    run goes on."""
    TICKER.set(phase="cleaning disk", detail="tools/disk.py --clean")
    before = disk.free_gb(ROOT)
    if not PREBUILD_LOCK.acquire(blocking=False):
        say("disk: a release build is still running; the sweep waits for the next session", C.GRAY)
        return
    try:
        freed = disk.clean(keep_runs=opts.keep_runs)
    except OSError as e:
        say(f"disk: the sweep failed -- {e}", C.YELLOW)
        return
    finally:
        PREBUILD_LOCK.release()
    after = disk.free_gb(ROOT)
    summary = f"disk: freed {disk.human(disk.total(freed))}, {before:.1f}G -> {after:.1f}G free"
    say(summary, C.GRAY)
    for line in disk.freed_lines(freed):
        say(f"  {line}", C.GRAY)
    if freed["target/deps"] is None:
        summary += "; deps/ left alone, cargo could not name the live set"
    ledger(f"       {summary}")


def marker_run():
    """The run `.loop/running` was written by, or "" when there is no marker or it names none."""
    try:
        for line in RUNNING.read_text(encoding="utf-8").splitlines():
            if line.startswith("run:"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return ""


def claim_run(opts, stamp):
    """Write the marker, or explain who already holds it. Returns True when the run may start.

    The marker is held for the whole run and not per turn: it is what `brief.py` and `disk.py`
    ask, and one dropped and retaken at every session would tell them no loop was running at the
    moments between two, which is where a checkpoint edits this tree. So the first turn writes
    it, every later turn finds its own run named in it and carries on, and the turn that ends the
    run removes it."""
    if marker_run() == stamp:
        return True
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
        # The parent's: `tools/respawn.py` is the process that lives as long as the run does.
        f"pid:      {os.getppid()}\n"
        f"run:      {stamp}\n"
        f"host:     {platform.node()}\n"
        f"started:  {datetime.now():%Y-%m-%d %H:%M:%S}\n"
        f"sessions: {sessions_label(opts.max_sessions)}, model {opts.model}"
        f"{f', effort {opts.effort}' if opts.effort else ''}\n",
        encoding="utf-8",
        newline="\n",
    )
    return True


def release_run():
    RUNNING.unlink(missing_ok=True)
    CONTROL.drop_pause()


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
    ap.add_argument(
        "--max-sessions", type=int, default=UNCAPPED, metavar="N",
        help="stop after N sessions. Uncapped by default, and 0 means the same thing: a run ends "
             "when the chain is walked, when something goes wrong, or when it is told to -- `s`, "
             ".loop/stop or Ctrl-C -- and a count guessed at the start is none of those"
    )
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
    ap.add_argument(
        "--no-hold", dest="hold_on_hand", action="store_false",
        help="end the run on a verdict that needs a person, instead of holding for one. The "
             "default holds: the answer is usually one edit away and a held run carries on with "
             "`p` where an ended one has to be relaunched. Pass this for a run nobody is watching"
    )
    ap.add_argument("--max-result-lines", type=int, default=60)
    ap.add_argument("--max-input-lines", type=int, default=40)
    ap.add_argument("--max-line-chars", type=int, default=500)
    ap.add_argument("--full-output", action="store_true", help="no truncation anywhere")
    ap.add_argument(
        "--no-status", dest="status", action="store_false",
        help="do not paint the live status line (it is off by itself when stdout is not a terminal)"
    )
    ap.add_argument(
        "--plain-input", action="store_true",
        help="start each session with its prompt on argv instead of streaming it down stdin. "
             "The session then cannot be sent a prompt with `i` or .loop/say; `h` still halts it"
    )
    ap.add_argument("--goal-only", action="store_true", help="run the acceptance test and exit")
    ap.add_argument("--full", action="store_true",
                    help="with --goal-only: consult no memo and run every check, which is also "
                         "what sees a service's state drift that no tree hash can")
    ap.add_argument(
        "--leg-only", action="store_true",
        help="run just the Linux leg -- every fixture, both suites and the valgrind sweep against a "
             "Linux build -- and exit"
    )
    ap.add_argument("--list", action="store_true", help="print the acceptance plan and exit")
    ap.add_argument("--owed", action="store_true",
                    help="which carried checks are not green over the tree as it stands; exits 1 "
                         "if any is, and runs nothing")
    ap.add_argument("--settle", action="store_true",
                    help="run the carried floor alone, memo consulted: what --owed names")
    ap.add_argument(
        "--chain-install", action="store_true",
        help="install the next staged goal from docs/agent/goals/ and exit, without "
             "running a session. The switch on its own -- run it when you want the new goal live "
             "before deciding when to start the run, and a run then resumes at it rather than "
             "installing it again."
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
        help="how many runs' session logs survive the disk sweep at the end of every goal. "
             "Whole runs, because loop-stats.py reads a run as a unit"
    )
    # The run's own. § *the run* at the foot of this file owns what they mean and the numbers.
    ap.add_argument(
        "--optimize-every", type=int, default=OPTIMIZE_EVERY, metavar="N",
        help="sessions between checkpoints that may spend a session on the loop itself. A "
             "checkpoint with no signal costs four subprocesses and not a session, which is what "
             "makes this number safe to be wrong about"
    )
    ap.add_argument(
        "--probe-every", type=int, default=PROBE_EVERY, metavar="N",
        help="sessions between two looks for drift. A look is where drift, rather than the "
             "count, can bring an optimization pass forward"
    )
    ap.add_argument(
        "--min-pass-gap", type=int, default=MIN_PASS_GAP, metavar="N",
        help="never run two optimization passes closer together than this, whatever fired"
    )
    ap.add_argument("--no-optimize", action="store_true",
                    help="sessions only; never look for drift or spend a session on the loop itself")
    ap.add_argument("--optimize-only", action="store_true",
                    help="run one optimization pass now, against the tree as it stands, and exit")
    ap.add_argument("--side", metavar="SLUG",
                    help="run the side goal named SLUG under docs/agent/goals/side/ and nothing "
                         "else, in its own "
                         "worktree, and land it on main when it is green (tools/side.py). Typed in "
                         "the main tree; never walks the chain and never runs an optimization pass")
    ap.add_argument("--land", action="store_true",
                    help="with --side: start no session -- verify the side branch over main and "
                         "land it if green, then end")
    opts = ap.parse_args()

    if opts.full_output:
        opts.max_result_lines = opts.max_input_lines = opts.max_line_chars = 0
    if opts.max_sessions <= 0:
        opts.max_sessions = UNCAPPED

    enable_ansi()

    if opts.land and not (opts.side or SIDE):
        say("--land lands a side goal, and needs --side <slug>", C.RED)
        return 2
    if opts.side and not SIDE:
        # Typed in the main tree: `launch_side` makes the worktree and runs every turn inside it,
        # where `goals.SIDE_ENV` is set and this branch is never taken again.
        return launch_side(opts)
    if SIDE:
        if opts.optimize_only or opts.chain_install:
            say("a side run never runs an optimization pass or installs a chain goal", C.RED)
            return 2
        # A pass edits `tools/` for the chain run's sake, and a side branch that carried one would
        # land it on main as a side effect.
        opts.no_optimize = True

    if opts.optimize_only:
        # Before the goal is loaded, and deliberately so: a pass is what fixes a loop whose own
        # files have drifted, and refusing to run one because one of them has drifted is backwards.
        return optimize_only(opts)

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
        for c in (goal.catch_up_checks + goal.setup_checks + goal.program_floor
                  + goal.cargo_checks + goal.program_checks + goal.overlap_checks):
            if "file" in c:
                # The argv `program_check` actually builds, in its order and behind its subcommand:
                # a check's own `args` go *ahead* of the fixture, so a line printed the other way
                # round names a command nothing runs -- and this is the line a session reads to
                # decide whether a `want` is producible at all.
                argv = " ".join(["run", *c.get("args", []), c["file"]])
                say(f"  [{c.get('stage', '?')}] {c['kind']:<10} nvs {argv}")
                continue
            if c["kind"] == "command":
                memo = "  (memoized against the tree)" if c.get("memoize") else ""
                note = ("  (setup: runs before the fixtures)" if c.get("setup")
                        else "  (overlap: runs beside the sweep)" if c.get("overlap") else memo)
                say(f"  [{c.get('stage', '?')}] {c['kind']:<11} {c['name']}: "
                    f"{' '.join(c['argv'])} in {c.get('cwd', '.')}{note}")
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

    if opts.owed:
        return owed(goal)

    if opts.settle:
        opts.goal_only, goal.settle_only, goal.collect = True, True, True

    if not (opts.goal_only or opts.leg_only or opts.chain_install) and respawn.ENV not in os.environ:
        # A run, typed by hand: `tools/respawn.py` takes it from here, and starts this file again
        # -- with the same flags, exactly as typed -- for every turn of it. This process never
        # gets past this line; the ones it starts find `respawn.ENV` set and carry on below.
        # Before the status line and the key reader on purpose: the console belongs to the turn.
        return respawn.run(Path(__file__).resolve(), sys.argv[1:], cwd=ROOT)

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
        goal.full = opts.full
        what = "the carried floor" if goal.settle_only else "the acceptance test"
        say(f"running {what} ..." + (" (full: no memo)" if opts.full else ""), C.CYAN)
        fail = goal.check(verbose=True)
        say(f"\ncost: {goal.summary()}", C.GRAY)
        if fail:
            say(f"NOT GREEN: {fail}", C.RED)
            return 1
        if goal.settle_only:
            say("SETTLED: every carried check is green over this tree", C.GREEN)
            return 0
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

    # The chain. Always `GOALS_DIR` and never a flag: there is one, every goal is in it, and a run
    # that walked none would stop at the first goal to go green. Built before `claim_run` so a goal
    # file with a typo in it costs a line and cannot leave `.loop/running` behind on a refusal.
    try:
        chain = SideChain() if SIDE else Chain()
    except ChainError as e:
        say(str(e), C.RED)
        return 2
    if opts.chain_install:
        # The switch on its own. Deliberately separate from starting a run: installing a goal is a
        # change to the repository that gets committed, and deciding when to spend three hundred
        # sessions against it is a different decision made at a different moment.
        if chain.finished:
            say(f"chain: goal `{chain.current.slug}` is the last in "
                f"{rel_to_root(GOALS_DIR)} and is already installed", C.YELLOW)
            return 0
        fail = chain.install_next()
        if fail:
            say(fail, C.RED)
            return 2
        say("")
        say("chain: `python tools/loop.py` resumes here", C.CYAN)
        return 0
    # One turn of the run. Everything above was the door, and every turn goes through it: a
    # goal file or a chain that stopped loading between two sessions ends the run on one line
    # here, where the previous turn's process would have carried on with what it had in memory.
    if not opts.no_optimize and not OPT_PROMPT.exists():
        say(f"missing {rel_to_root(OPT_PROMPT)} -- restore it, or run with --no-optimize", C.RED)
        return 2
    stamp = os.environ[respawn.ENV]
    if marker_run() != stamp:
        # The first turn only: a run is refused for want of disk at the door, never half way.
        TICKER.set(phase="checking disk", detail="free space against --min-free-gb")
        if not enough_disk(opts):
            return 2
    if not claim_run(opts, stamp):
        return 2
    # Pessimistic until `turn` says otherwise, so that a traceback, a `SystemExit` and a refusal
    # below all release the marker, and only a turn that asked for another one keeps it.
    code = 1
    try:
        run = Run(stamp)
        if run.fresh:
            fail, goal = first_turn(chain, goal)
            if fail:
                say(fail, C.RED)
                code = 2
                return code
        # Set again here rather than only above: a chain that just installed its first goal
        # rewrote `loop-goal.md` after the first read, and this is the first moment the row can
        # name it.
        TICKER.set(loop_goal=goal_title(chain))
        code = turn(opts, goal, chain, run)
        return code
    except KeyboardInterrupt:
        say("")
        # Ctrl-C kills the session where it stands, so the driver never reaches the per-session
        # sweep and this is the one place that can. A session that WRAPS commits before it exits;
        # the one being interrupted is the only kind this handler ever sees, and it has not.
        # Between two sessions there is nothing of a session's to sweep, and this finds nothing.
        swept = mark_interrupted(0, "the run was interrupted with Ctrl-C")
        say(
            f"interrupted -- {swept} uncommitted path(s) swept into a wip commit; "
            f"the working tree is clean" if swept else
            "interrupted -- the working tree was already clean",
            C.YELLOW,
        )
        ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- interrupted (Ctrl-C)"
               + (f"; swept {swept} path(s) into a wip commit" if swept else ""))
        code = 0
        return code
    finally:
        if code != respawn.AGAIN:
            release_run()


def first_turn(chain, goal):
    """What a run does once, before its first session: the goal's environment.

    Returns `(why it cannot start, the goal to run against)`. Once per run and not once per turn
    because none of it is state a process holds -- an installed goal is files, a Docker daemon
    and the containers `bring_up_services` starts outlive the turn that asked for them -- and
    `drive` does the same for a goal it switches to half way through a run."""
    if chain.index < 0:
        # Nothing installed yet: the first goal takes its floor from whatever goal the repository
        # is currently running, which is what the first switch is for. Everything after it takes
        # its floor from the goal before.
        fail = chain.install_next()
        if fail:
            return fail, goal
        try:
            goal = load_goal()
        except (tomllib.TOMLDecodeError, GoalError) as e:
            return f"{rel_to_root(GOAL_TOML)}: {e}", goal
    elif SIDE:
        say(f"side goal `{SIDE.slug}`: running in {ROOT.as_posix()} on branch "
            f"{sidemod.branch(SIDE.slug)}", C.CYAN)
        fail = preflight(chain.current.preflight)
        if fail:
            return fail, goal
    else:
        say(f"chain: resuming at goal `{chain.current.slug}`, "
            f"{chain.current.num} of {len(chain.goals)}", C.CYAN)
        fail = preflight(chain.current.preflight)
        if fail:
            return fail, goal
    return chain.bring_up_services(), goal


#: How many sessions run between two runs of the carried floor and the release-profile checks,
#: and the only home for that number. The release profile is `lto = "thin"` with
#: `codegen-units = 1` and it is the
#: acceptance check's critical path, not a step in it: measured on the 20260904-143054 run, the
#: sweep spent **84s of 288s** waiting at `join_prebuild` while everything else fitted underneath
#: the build. Its only consumers are cost-class assertions -- the abi-probe perf guards and the
#: CLI warm-start bench -- so a stale verdict is a latency regression (priority 3) and never a
#: wrong answer, and it does not compound the way a leak or a semantics bug does.
#:
#: The same gate holds the CARRIED FLOOR: the previous goal's whole list, some seven hundred
#: checks relabelled `1 floor`, which is nearly all of what a sweep costs and almost none of what
#: a session is told. Replayed over the ledger's 1,240 sessions before this gate existed: 94% of
#: sweeps ended red, and 1,126 of those were the frontier -- a named test not yet written, a
#: fixture not yet on disk, the goal's own stage still open -- which the goal's own checks report
#: without the floor. The floor itself caught 44 regressions, one session in twenty-five, and
#: 25 of them were doc-gate commands `session.py --wrap` now refuses before the commit; the ten
#: that were code, and the six valgrind findings, all sat at positions a ten-session window still
#: reaches inside the goal that made them. The blind window is at most this many sessions of
#: three commits each, `git log` reads a slice at a time, and a goal is never declared reached on
#: a held floor: `Goal.held` is non-empty, and `drive` runs the list again, gate open, first.
#: An open gate still consults the memo, so what it pays for is the carried checks whose inputs
#: some session has changed since each was last green -- nothing, over a tree the floor does not
#: read. Every sweep in between is the goal's own list, so the pack still names the earliest red
#: check.
#: Measured before the gate: 128s an ordinary sweep and 691s a gated one; after it an ordinary
#: sweep is the goal's own dozen checks. Dropping this to 5 halves the window for about a minute
#: a session more.
FLOOR_GATE_EVERY = 10

#: One narrow-keyed check in this many is run again on a gate-open sweep although the memo answers
#: it, to test the memo. `Goal.audits` owns why it is a sample and how the sample is drawn.
AUDIT_EVERY = 50


def read_counter(path, every):
    """Sessions since a periodic gate last fired. An unreadable file fires it rather than skipping
    it, which is `partition_ids`'s rule and `verify.py`'s: the safe direction is doing the work."""
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


def write_doc_gate(failed=None, session=""):
    """`.loop/doc-gate.json`: the rustdoc gate's standing verdict, and the session it was taken
    after.

    A file rather than a ledger line because `orient.py` needs the *current* state -- a ledger
    holds every verdict a run ever wrote, and the newest `doc gate:` line in it stays red forever
    once one has been written."""
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        DOCGATE.write_text(
            json.dumps({"when": time.time(), "failed": failed or "", "session": session},
                       indent=1),
            encoding="utf-8", newline="\n")
    except OSError:
        pass


def context_sweep(base, slug):
    """Let the goal's `[context] modules` learn what the session actually edited.

    `tools/context-sync.py` is the whole of it and holds the reasoning; this is where it runs,
    beside `doc_gate` for the same reason -- between sessions, over the tree the session left, in
    the driver's seconds rather than a session's context ceiling.

    A session that finds the manifest missing a file it is working in writes that into the handoff,
    which no one acts on, and the next session pays the same search. The evidence is on disk by
    then -- the commits the session just made -- so the driver reads it rather than asking anyone.
    Widening the list cannot break a build or a check: at worst a session reads one map line it did
    not need, which is why this is allowed to run unattended at all.

    The commit's subject names the goal and the added files, and its body is their paths: the
    reasoning is `context-sync.py`'s module doc, and a copy of it in every commit is noise.

    Returns a one-line note for the console, or "" when nothing changed."""
    if not base:
        return ""
    listing = RUNDIR / "context-added.txt"
    listing.unlink(missing_ok=True)
    r = capture(sys.executable, [str(ROOT / "tools" / "context-sync.py"), "--since", base,
                                 "--goal", str(GOAL_TOML), "--added", str(listing)])
    note = (r.out or "").strip()
    try:
        added = listing.read_text(encoding="utf-8").split()
    except OSError:
        added = []
    listing.unlink(missing_ok=True)
    if r.code != 0 or not added:
        return note  # it refused and said why, or found nothing -- there is nothing to commit
    names = [path.rsplit("/", 1)[-1] for path in added]
    files = names[0] if len(names) == 1 else f"{', '.join(names[:-1])} and {names[-1]}"
    body = f"docs(loop): goal `{slug}` maps {files}\n\n" + "".join(f"{path}\n" for path in added)
    git("add", "--", str(GOAL_TOML))
    msg = RUNDIR / "context-msg.txt"
    try:
        msg.write_text(body, encoding="utf-8", newline="\n")
        git("commit", "-F", str(msg))
    except OSError:
        pass
    finally:
        msg.unlink(missing_ok=True)
    return note


def doc_gate(index):
    """`verify.py --doc`, on a sweep that would reach the goal, and a goal is not reached while it
    is red. Returns the finding, or "" when green.

    A broken intra-doc link stops no build and changes no behaviour, so a goal in progress may
    carry some: an item is renamed, the comment naming it goes stale, and a later session of the
    same goal fixes it. What has to be clean is the tree a goal leaves behind, so this runs where
    the floor gate opens for good -- on the green sweep that would declare the goal reached -- and
    not after every session. Paying it per session bought nothing that matters, at a price set by the
    graph rather than the edit: rustdoc re-documents the edited crate and every workspace crate
    above it, one after another.

    Red, it holds the goal open without stopping the run. `orient.py` prints the finding to the
    next session off the file this writes, that session fixes it, and the next green sweep asks
    again."""
    step("rustdoc gate: every link in a doc comment, resolved (the acceptance list is green)",
         C.CYAN)
    began = time.monotonic()
    r = capture(sys.executable, ["tools/verify.py", "--doc"], timeout=900)
    spent = mmss(time.monotonic() - began)
    if r.code == 0:
        step(f"rustdoc gate green in {spent}", C.CYAN)
        write_doc_gate()
        return ""
    text = ((r.out or "") + "\n" + (r.err or "")).replace("\r\n", "\n")
    first = next((ln.strip() for ln in text.split("\n") if ln.strip().startswith("error")), "")
    why = first or f"`python tools/verify.py --doc` exited {r.code}"
    step(f"rustdoc gate FAILED in {spent} -- {why}", C.RED)
    write_doc_gate(failed=why, session=f"{index:04d}")
    ledger(f"       doc gate: {why}")
    return why


def write_owner_gate(failed=None, session=""):
    """`.loop/owner-gate.json`: the owner gate's standing verdict, for `orient.py`, in the shape
    `write_doc_gate` writes and for the same reason."""
    try:
        RUNDIR.mkdir(parents=True, exist_ok=True)
        OWNERGATE.write_text(
            json.dumps({"when": time.time(), "failed": failed or "", "session": session},
                       indent=1),
            encoding="utf-8", newline="\n")
    except OSError:
        pass


def owner_gate(index, slug):
    """`owners.py --closes <slug>` and `playbook.py --closes <slug>` on a sweep that would reach
    the goal, and a goal is not reached while either names a gap. Returns the finding, or "" when
    green.

    The floor carries `no module-doc gap names a goal that walked without closing it`, and it
    cannot fire at the one moment it matters. A goal reads as retired off its `.toml` being gone,
    which `chain.py --retire` does *after* the goal is reached, so an item tagged to the reaching
    goal counts as goal-owned on the sweep that reaches it and as retired-owner on the next goal's
    floor, one goal late. Goal `unowned-closures` walked that way with forty items tagged to it and
    none built: its own gate was `unowned: 0`, and tagging the items to the goal is what made the
    count zero. So the question is asked here, of the goal by name, whether or not its own list
    asks it, over both places a goal's tag can sit in -- the module docs, and the § *Owned* table
    of an index rebuilt where the deleted one stood. A tag is not a build.

    Red, it holds the goal open without stopping the run, as `doc_gate` does: `orient.py` prints
    the finding off the file this writes, the next session builds, strikes or re-owners each item,
    and the next green sweep asks again."""
    step(f"owner gate: no gap names goal `{slug}` (the acceptance list is green)", C.CYAN)
    began = time.monotonic()
    found = []
    for tool in ("owners.py", "playbook.py"):
        r = capture(sys.executable, [str(ROOT / "tools" / tool), "--closes", slug], timeout=300)
        if r.code == 0:
            continue
        text = ((r.out or "") + "\n" + (r.err or "")).replace("\r\n", "\n")
        last = next((ln.strip() for ln in reversed(text.split("\n")) if ln.strip()), "")
        found.append(last or f"`python tools/{tool} --closes {slug}` exited {r.code}")
    spent = mmss(time.monotonic() - began)
    if not found:
        step(f"owner gate green in {spent}", C.CYAN)
        write_owner_gate()
        return ""
    why = "; ".join(found)
    step(f"owner gate FAILED in {spent} -- {why}", C.RED)
    write_owner_gate(failed=why, session=f"{index:04d}")
    ledger(f"       owner gate: {why}")
    return why


#: How many failed DONE claims on one goal get a fresh session before a hand is asked.
#:
#: A DONE claim the sweep refuses used to hold the run at once, and the person's hand then did
#: what a session does: read the `goal check:` line, fix the check, claim DONE again. Measured
#: over the 2026-09-14 ledger, four holds in a day, each with a different cause -- a test named
#: by a near miss, a stale plan cell, a flake under the sanitizer, a host-tier timeout -- and
#: three of the four were one session's work. The sweep's failing line is already in the next
#: pack (`orient.py` reads it from the ledger), so a retry costs one session and no one's hand.
#:
#: Bounded twice, and both bounds are needed. A retry whose own DONE fails on the check it was
#: handed holds the run: the same question asked twice is the coordinator's rule for a hand, and
#: a session that could not fix it with the line in front of it is not going to on a third read.
#: A retry that closed its check and fell to a *different* one is not that: the sweep stops at
#: the first red check, so a goal several checks short of green meets them one per sweep, and
#: each is a new question a fresh session answers as well as a hand would. And a goal gets this
#: many in total, because a DONE claimed early and refused, then CONTINUEd, then claimed and
#: refused again is a session pair per cycle with no bound but this one -- and so is a chain of
#: sweeps that each fall to a new check.
DONE_RETRIES = 3


def check_of(fail):
    """The check a `goal check:` line names -- its `name [stage]` label, which every failure line
    opens with -- so two sweeps that died on the same check compare equal whatever the detail
    after the label said: a different needle, a different test name, a timeout."""
    head, sep, _ = fail.partition("]: ")
    return head + "]" if sep else fail


def driver_files():
    """Every `.py` file under `tools/` this process has imported, with the digest of its bytes.
    This is the code a sweep run by this process judges with: `loop.py` itself and the modules
    it imports. A tool the sweep starts as a subprocess is read off disk when it starts, and is
    never stale."""
    tools = ROOT / "tools"
    out = {}
    for mod in list(sys.modules.values()):
        path = getattr(mod, "__file__", None)
        if not path:
            continue
        p = Path(path).resolve()
        if p.suffix != ".py" or tools not in p.parents:
            continue
        with contextlib.suppress(OSError):
            out[p] = hashlib.sha256(p.read_bytes()).hexdigest()
    return out


def driver_changed(before):
    """The files in `before`, a `driver_files()` snapshot, whose bytes differ now, as paths from
    the root.

    A non-empty answer after a session means this process would judge the session with the code
    the session replaced. A session that fixes the driver's own verdict then fails the same check
    again, and a retry that fixed its check is held as if it had fixed nothing. So `drive` ends the
    turn without a sweep, and the next turn, a fresh process, judges that session."""
    changed = []
    for p, digest in before.items():
        try:
            now = hashlib.sha256(p.read_bytes()).hexdigest()
        except OSError:
            now = ""
        if now != digest:
            changed.append(rel_to_root(p))
    return sorted(changed)


def drive(opts, goal, chain, run):
    """One turn's session: launch it, judge it, and say how it ended as `(kind, reason)`.

    It is a loop only because a session can be refused before it is served -- a usage wall, an
    overloaded API, a dropped stream, a CLI that exits non-zero -- and each of those is waited out
    and tried again HERE, in the turn that met it. The streaks those retries count are locals for
    that reason: a served session resets every one of them, and a turn does not end until a
    session is served or the run does. What is counted across served sessions comes out of `run`
    at the top and goes back into it at the bottom; `Run` lists it."""
    prompt_text = PROMPT.read_text(encoding="utf-8")
    renderer = Renderer(opts)

    stalls = run.stalls
    fails = 0
    # The DONE-claim retries this goal has spent, and the check the session that just ended was
    # handed as one -- "" when it was not a retry.
    done_retries = run.done_retries
    retry_check = run.retry_check
    reason = "the session was served"
    kind = SERVED
    run_id = run.run_id

    # Two counters where there was one. `index` names the logs and only ever goes up, so a session
    # the wall cut short never has its transcript overwritten by the one that replaces it; `served`
    # is what `--max-sessions` counts, and a session the account refused is not one of them.
    index = run.index
    served = 0
    number = (f"{run.served + 1}/{opts.max_sessions}" if opts.max_sessions < UNCAPPED
              else f"{run.served + 1}")
    walls = 0
    overloads = 0  # consecutive sessions the API refused as overloaded; see `OVERLOAD_STATUS`
    overloaded_since = 0.0  # monotonic, when the current overload streak began
    resumes = 0  # consecutive resumes of one dropped session; see `MAX_RESUMES`
    resume_from = ""  # the transcript the next launch rejoins, set by the drop branch below
    wall = standing_limit()  # left standing by a driver killed or rebooted during one
    if wall:
        step(f"{rel_to_root(LIMIT)} says {wall.describe()}", C.YELLOW)

    # The code this turn judges with, taken before any session can change it: `driver_changed`.
    imported = driver_files()
    # A session the last turn served and could not judge, because it changed that code. This turn
    # serves no session: it writes the sweep into that session's log and judges it below.
    judge, run.judge = run.judge, {}
    if judge:
        index = int(judge.get("index", index))
        line = str(judge.get("line", ""))
        commits = int(judge.get("commits", 0))
        # Where the session began, which `context_sweep` reads its edits from.
        SLICES.base = str(judge.get("base", ""))
        served = 1
        CONSOLE.open_session(LOGDIR / f"{run_id}-{index:04d}.log")
        step(f"judging session {index} with the driver code it committed -- no session this turn",
             C.CYAN)

    while not served:
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

        # After the wall rather than before it, because this is the last instant before a session
        # starts and a hold is a promise about the tree, not about the clock. A hold queued during
        # a five-hour usage wait is therefore honoured when the window reopens, not slept through.
        held = hold_pause() or merge_yield()
        if held:
            reason, kind = held, "asked"
            break

        index += 1
        SLICES.start(git("rev-parse", "HEAD"))
        STATUS.unlink(missing_ok=True)
        TICKER.set(scope=f"session {number}", phase="starting")
        say(f"== session {number}  {datetime.now():%H:%M:%S}", C.CYAN)

        # The prompt is re-read for the same reason `load_goal()` is called below: a session that
        # improved it should be improving the next session, not the next run. A read that fails
        # keeps the last good text rather than ending a run nobody is watching. A run repairing a
        # verdict hands this session the repair prompt and the verdict instead: `REPAIR_KINDS`.
        opening = REPAIR_PROMPT if run.repair else PROMPT
        try:
            prompt_text = opening.read_text(encoding="utf-8")
            if run.repair:
                prompt_text = prompt_text.rstrip("\n") + f"\n\n```\n{run.repair}\n```\n"
                step("a repair session: it gets the verdict the run stopped on, not a goal item",
                     C.CYAN)
        except OSError as e:
            say(f"   {rel_to_root(opening)} did not read, using the last good one -- {e}",
                C.YELLOW)

        session_started = time.monotonic()
        # Consumed here whatever happens below, so a launch can only ever rejoin a transcript the
        # branch that set it chose: every other path leaves this empty and starts a fresh session.
        rejoined, resume_from = resume_from, ""
        # Here rather than beside `SLICES.start` above, because this is the line that knows
        # whether a new session is starting or a dropped one is being picked back up.
        TOUCH.start(carry=bool(rejoined))
        cli_exit, log, session_id, limit, api_error, dropped, operator = run_session(
            run_id, index, side_preamble() + prompt_text, opts, renderer, resume=rejoined)
        step(f"session {index} ended after {mmss(time.monotonic() - session_started)}, "
             f"claude exit {cli_exit}"
             + (f", {renderer.tokens()}" if renderer.tokens() else ""), C.CYAN)

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

        # A dropped stream is the third exit that is not a crash, judged before the exit code for
        # the same reason the other two are: it explains it. The session was healthy and its
        # transcript is whole on disk, so it is handed back to `claude --resume` and picks up
        # mid-slice, instead of being swept and replaced by a stranger that has to read the wip
        # commit to find out what it was doing.
        #
        # **The tree is deliberately NOT swept here.** `mark_interrupted` exists because the NEXT
        # session cannot tell an unfinished slice from its own starting state -- and the next
        # session here is the SAME session, which can. Sweeping would also take the wrap out from
        # under it: `session.py --wrap` stages a slice's files and commits them, and a sweep that
        # has already committed them leaves it staging nothing. The exposure is one backoff plus
        # one session, and the moment the resumes run out the branch below sweeps as it always
        # did.
        if cli_exit != 0 and dropped and session_id and resumes < MAX_RESUMES:
            resumes += 1
            back = resume_wait(resumes)
            resume_from = session_id
            ledger(f"- {index:04d} the connection dropped mid-response -- rejoining its "
                   f"transcript, attempt {resumes}/{MAX_RESUMES} in {mmss(back)}; the tree is "
                   f"left as the session had it -- see {log.relative_to(ROOT).as_posix()}")
            step(f"the connection dropped mid-response -- the session is intact on disk, so it "
                 f"is resumed rather than restarted. Attempt {resumes}/{MAX_RESUMES} in "
                 f"{mmss(back)}, and the tree is left exactly as it was", C.YELLOW)
            TICKER.set(phase="resuming a dropped session",
                       detail=f"attempt {resumes}/{MAX_RESUMES}")
            wait(back, "the connection dropped, resuming")
            continue

        if cli_exit != 0:
            fails += 1
            # The streak is over either way: this exit was not a drop, or it was one resumed
            # `MAX_RESUMES` times without sticking. A later drop in a later session gets its own
            # three, which is the point -- the cap is per drop, not per run.
            resumes = 0
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
        resumes = 0
        served += 1
        # Counted the moment it is true, and not at the bottom with the rest: everything below
        # this line can be cut off by a Ctrl-C, and a session that was served was served.
        run.served += 1
        run.index = index
        # Spent on the session that was served, so a repair the usage wall refused is still owed.
        repaired, run.repair = bool(run.repair), ""
        run.save()
        credit()

        line = STATUS.read_text(encoding="utf-8").strip() if STATUS.exists() else ""
        # The count the goal row has been showing all session, from the same range: the watch
        # takes one last look here, because the final slice is committed by the session's last
        # tool call and nothing after it would have polled.
        commits = SLICES.finish()
        # A session that finished and left nothing behind closes any earlier interruption; one
        # that left something behind gets it committed here.
        #
        # This path used to be the `if not dirty: unlink` half alone, and that is precisely the
        # hole: a session that exits ZERO without wrapping -- interrupted at the console, cut off
        # by the harness, or simply out of turns -- reaches here, is counted as served, and its
        # unfinished slice was neither committed NOR recorded. Only the failure paths below swept,
        # so the one shutdown a person actually performs was the one that leaked.
        swept = mark_interrupted(index, "it exited without wrapping")
        if swept:
            commits += 1
            step(f"swept {swept} uncommitted path(s) into a wip commit -- "
                 f"the session ended without wrapping", C.YELLOW)
        step("collecting subagent transcripts")
        TICKER.set(phase="collecting subagent transcripts")
        started = time.monotonic()
        agents, agent_calls = collect_subagents(session_id, run_id, index)
        spent = time.monotonic() - started
        if spent >= 1:
            step(f"subagent transcripts took {mmss(spent)}")
        delegated = f" | {agents} subagent(s), {agent_calls} call(s)" if agents else ""
        wip = f" | swept {swept} path(s) into a wip commit" if swept else ""
        # Said in the ledger because it changes what the line means: a session a person halted
        # or spoke to is not the unattended session every measurement over this file assumes.
        attended = f" | {operator}" if operator else ""
        if repaired:
            attended += " | repair session"
        ledger(f"- {index:04d} {commits} commit(s){delegated}{wip}{attended} | "
               f"{line or '(no status written)'}")

        stale = driver_changed(imported)
        if stale:
            run.judge = {"index": index, "line": line, "commits": commits, "base": SLICES.base}
            reason = (f"session {index} changed the driver's own code ({', '.join(stale)}), "
                      f"so a fresh turn judges it")
            kind = "rejudge"
            ledger(f"       {reason}")
            CONSOLE.close_session()
            break

    # The verdict on the session, which is one pass: every way out of it is a `break`. A loop
    # above that ended without a session to judge skips it.
    while kind == SERVED:
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
        # carried floor and the release profile run at all this session. Both are read here rather
        # than inside `Goal` because both are counted in SESSIONS, and a `Goal` is loaded fresh
        # every one.
        goal.fast_path = read_last_fail()
        floor_since = read_counter(FLOORGATE, FLOOR_GATE_EVERY) + 1
        goal.floor_gate = floor_since >= FLOOR_GATE_EVERY
        held = ("" if goal.floor_gate
                else f" (carried floor and release profile held, 1 session in {FLOOR_GATE_EVERY})")
        step(f"acceptance check: build, fixtures, suites, wsl leg, valgrind{held}", C.CYAN)
        checked = time.monotonic()
        fail = goal.check(verbose=True)
        # A green sweep is the end of a goal, and a goal is not reached on a sweep that held
        # anything behind the floor gate: the list runs again, once, gate open. The memo is still
        # consulted there, and it is what scopes that sweep to the goal's own changes: a held
        # check's verdict was filed over the bytes it read when it last ran, which may be many
        # sessions or a goal ago, so it runs now exactly when some session since has changed one
        # of those bytes, and is answered from the file when none has. `Goal.remembered` owns why
        # a check over identical inputs cannot answer differently; `--goal-only --full` is the
        # sweep that consults nothing, by hand.
        #
        # That second sweep runs past a red check and names every one (`Goal.ran_past`). The goal's
        # own list has just passed, so each red there is a finding for the next session, and a
        # sweep that stopped at the first cost a session and a sweep for every red check, one after
        # another.
        #
        # The goal-end gates run after it, red or green, for the same reason. They ask about the
        # goal's own work, which the scoped sweep has just passed, so a red floor check says nothing
        # about them, and a gate first asked once the floor is green is one more round. Any other
        # sweep asks them only when it is green, as it always did.
        reaching = not fail
        if not fail and goal.held:
            step(f"scoped sweep green with {len(goal.held)} check(s) held -- opening the floor "
                 f"gate over what this goal changed before the goal is reached", C.CYAN)
            ledger(f"       goal cost: {goal.summary()} (scoped; opening the floor gate)")
            goal.floor_gate = True
            goal.collect = True
            reaching = True
            fail = goal.check(verbose=True)
        write_counter(FLOORGATE, 0 if goal.floor_gate else floor_since)
        write_last_fail(goal.failed_name)
        step(f"acceptance check done in {mmss(time.monotonic() - checked)}", C.CYAN)
        ledger(f"       goal cost: {goal.summary()}")
        # Every session, not only one that reaches the goal: the check has just left the build
        # warm and nothing is building, and a day of sessions is what fills `target/`.
        clean_disk(opts)
        # Beside the acceptance check because it is the same kind of thing: a gate the driver runs
        # between sessions, over the tree the session left, reported through a file a pack reads.
        widened = context_sweep(SLICES.base, chain.current.slug)
        if widened:
            step(widened, C.CYAN)
            ledger(f"       {widened}")
        # Only a sweep that would reach the goal pays for the two goal-end gates; `doc_gate` and
        # `owner_gate` each say why. Both run when the list is green, so one session sees both.
        docs_red = doc_gate(index) if reaching else ""
        owner_red = owner_gate(index, chain.current.slug) if reaching else ""
        # The verdict on session `i` is the last thing that belongs in session `i`'s log.
        CONSOLE.close_session()
        if not fail and not docs_red and not owner_red:
            done = chain.current.slug
            ledger(f"## goal reached: {done} -- every check in its acceptance list passes")
            say(f"GOAL REACHED: {done}", C.GREEN)
            if SIDE:
                # A side run has no next goal: it lands, and `turn` hands that to `land_side`.
                reason, kind = f"side goal `{done}` is green -- landing it on main", "side-land"
                break
            # The goal that just passed may have been the one that writes the rest of the chain.
            grew = chain.refresh()
            if grew:
                say(grew, C.CYAN)
                ledger(f"## {grew}")
            if chain.finished:
                reason = (f"CHAIN COMPLETE: `{done}` was the last goal in "
                          f"{rel_to_root(GOALS_DIR)}, and every one of them is green")
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
            ledger(f"## run continues on goal `{chain.current.slug}` "
                   f"({chain.current.num} of {len(chain.goals)})")
            # A new goal is a new worklist, so a stall streak from the old one says nothing about
            # it -- and the first session of any goal is the one most likely to spend itself
            # reading rather than committing. Its DONE-claim retries and its repairs start over for
            # the same reason.
            stalls = 0
            done_retries = 0
            retry_check = ""
            run.repairs = 0
            try:
                goal = load_goal()
            except (tomllib.TOMLDecodeError, GoalError) as e:
                reason = f"chain: {rel_to_root(GOAL_TOML)} did not load after the switch -- {e}"
                kind = "chain-error"
                break
            TICKER.set(loop_goal=goal_title(chain))
            verdict(False, f"goal reached -- the run carries on with `{chain.current.slug}`")
            break
        if fail:
            ledger(f"       goal check: {fail}")

        # A `DONE` held only by a goal-end gate -- rustdoc, or a gap still naming the goal -- is not
        # a wrong claim, just an unfinished one: the next session gets the finding in its pack and
        # fixes it, with no one to wake.
        #
        # A `DONE` the sweep refuses gets the same treatment: the failing check is in the next
        # pack, so a fresh session is given it before a hand is asked. The hand is asked when the
        # retry's own DONE fails on the check it was handed -- the same question asked twice --
        # or when the goal has spent `DONE_RETRIES`. A retry that closed its check and fell to a
        # different one made progress, and the new check is a new question for a new session.
        handed = retry_check
        retry_check = ""
        if line.startswith("DONE") and fail:
            failing = check_of(fail)
            if failing == handed:
                reason = (f"session reported DONE but the acceptance test does not pass yet, and "
                          f"the retry failed the check it was handed, `{failing}`: {line}")
                kind = "done-claim"
                break
            if done_retries >= DONE_RETRIES:
                reason = (f"session reported DONE but the acceptance test does not pass yet, and "
                          f"the goal has spent its {DONE_RETRIES} retries: {line}")
                kind = "done-claim"
                break
            done_retries += 1
            retry_check = failing
            progress = (f"; the retry closed `{handed}` and fell to a different check, "
                        f"so it is a new question" if handed else "")
            retried = (f"done-claim retried: the DONE above failed its sweep, and one session "
                       f"gets the failing check before a hand is asked "
                       f"({done_retries} of {DONE_RETRIES} on this goal){progress}")
            step(retried, C.YELLOW)
            ledger(f"       {retried}")
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

        verdict(False, line or "the session wrote no status line, and the loop carries on")

        if opts.delay_seconds:
            step(f"--delay-seconds: waiting {mmss(opts.delay_seconds)} before the next session")
            TICKER.set(phase="waiting")
            wait(opts.delay_seconds, "--delay-seconds")
        break

    # Whatever ended it, and in one place: a verdict reached with a `break` leaves through here
    # exactly as a served session does. `turn` says the verdict; this only hands it over.
    run.index = index
    run.stalls = stalls
    run.done_retries = done_retries
    run.retry_check = retry_check
    run.save()
    return kind, reason


# ------------------------------------------------------------------------------------ the run
#
# Everything above serves ONE SESSION: a fresh `claude`, the acceptance check that judges it, and
# a verdict the moment any of a dozen fires. Everything below is THE RUN, which is a sequence of
# turns -- and **a turn is a process**. `turn` is one of them; `tools/respawn.py` starts the next.
#
# A turn is a process so that nothing in this file can go stale. `orient.py` is a subprocess,
# `session-prompt.md` is re-read and `loop-goal.toml` is re-loaded every session, so a session
# that improves any of those improves the *next* session -- and since a fresh interpreter reads
# this file off disk for every session too, the same is true of the driver. `respawn.py` is the
# one process that outlives a session, and it is kept free of anything worth improving: it holds
# no logic, reads no key and opens no file of the run's.
#
# That the long-lived process runs NOTHING is the constraint the rest follows from, and both
# ways of breaking it cost something real. Work done up there -- a checkpoint, a hold, a count --
# is code read once per run and never again, and it is work done by a process that does not read
# the keys, so `s`, `p` and `r` are dead for its length; a checkpoint can be a full `verify.py`
# and then a whole optimization session. A flag parsed up there is a second argparse to keep
# equal to this one's by hand. So every flag reaches every turn exactly as it was typed, the
# console belongs to the one turn alive, and `.loop/running` is held across all of them
# (`claim_run`) because it is a file and not a process's.
#
# The boundary behind a session is also the only moment quiet enough to ask a second question:
# has the loop drifted? A loop changes the shape of its own input and nothing announces it. The
# pack grew 59 KB -> 118 KB at +907 B a session once, re-billed on all ~81 calls of every session
# after it, and the projected slice cap fell to one on the strength of that alone.
# `docs/agent/optimization-prompt.md` is the pass that reverses it; `checkpoint` decides when to
# spend a session on one, and `settle` decides whether what it committed may stay.
#
# Nothing down here judges the work. The stop path is still exit codes and exact output matching
# up in `Goal`, and the run adds no verdict of its own to it.

#: The verdicts a person can answer, after which the run is worth carrying on: the goal is not
#: finished, the tree is where the session left it, and everything the next turn reads -- the
#: chain, the goal, the handoff, `loop.py` itself -- is read from disk. So the run **holds** on
#: these rather than ending, and a lifted hold starts a fresh turn where a relaunch used to be
#: needed. Every other kind but `SERVED` ends the run, including the ones that look recoverable:
#: a usage window that never reopened is a reason a *human* should look, and a run that retried
#: it would turn one bad hour into eight.
#:
#: The two verdicts deliberately outside it are the two that a hold would insult. `asked` is `s` or
#: `.loop/stop`: someone said end the run, and holding would be arguing with them. `chain-complete`
#: has nothing left to walk, so there is no session to start when the hold lifts.
HOLD_KINDS = frozenset({"blocked", "stalled", "done-claim", "cli-failed", "chain-error", "wall",
                        "side-conflict"})

#: The held verdicts a session can answer without the user, so the run gives each one a repair
#: session before it holds: the next turn's session gets `docs/agent/repair-prompt.md` and the
#: verdict in place of the session prompt, and is judged like any other. This is what the user
#: did by hand at every hold: paste the verdict into a new session, let it fix the cause, release.
#: The two left out need the user. `blocked` is a session asking for a decision, and `wall` is the
#: usage limit, which nothing in the tree can change and which would refuse the session anyway.
REPAIR_KINDS = frozenset({"stalled", "done-claim", "cli-failed", "chain-error", "side-conflict"})

#: Repair sessions one goal gets before a repairable verdict holds after all. It is a bound and not
#: a count of anything: a verdict that three sessions with the error in front of them could not
#: clear needs the person, and without a bound a goal that never goes green would spend sessions
#: forever with nobody asked.
REPAIRS_PER_GOAL = 3

#: What an optimization pass may commit. Everything outside this is reverted, unread: the pass is
#: the loop working on itself, and `crates/`, `tests/` and `examples/` are the work, not the loop.
#: An allowlist rather than a denylist because a new top-level directory must default to refused.
ALLOWED = ("tools/", "docs/", "AGENTS.md", ".claude/CLAUDE.md", "README.md")

#: A pass only runs when a signal fires, so these are when the run *looks*, not how often it
#: spends a session. Looking is four subprocesses; the pass is a session.
PROBE_EVERY = 10
OPTIMIZE_EVERY = 25
MIN_PASS_GAP = 15
PACK_GROWTH = 20 * 1024

#: How the pack-growth signal opens. It is the one signal that can bring a pass *forward*, so the
#: check for it reads this rather than sniffing for a word: a reworded signal would otherwise
#: disable the early trigger silently, which is the failure this whole section exists to catch.
PACK_SIGNAL = "the orientation pack grew"


def run_stamp():
    return f"{datetime.now():%Y%m%d-%H%M%S}"


def clip(text, head=40, tail=140):
    """Bound a probe's output without losing either end.

    Head *and* tail, because these tools put the table first and the conclusion last:
    `loop-stats.py` opens with a row per session and closes with the constants and the projection,
    which is the half a pass actually reads. A plain head-truncation would drop exactly that."""
    lines = text.rstrip().split("\n")
    if len(lines) <= head + tail:
        return "\n".join(lines)
    hidden = len(lines) - head - tail
    return "\n".join(lines[:head] + [f"   ... [{hidden} lines omitted] ...", ""] + lines[-tail:])


def probe(*argv, timeout=300):
    """Run one of the loop's own measuring tools and return `(exit code, output)`.

    Never raises. A probe that cannot run is a missing section in the evidence pack and a line in
    the report; it is never the reason a run of 300 sessions stops."""
    try:
        done = subprocess.run(
            [sys.executable, str(ROOT / "tools" / argv[0]), *argv[1:]],
            cwd=ROOT, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=timeout,
        )
    except (OSError, subprocess.SubprocessError) as e:
        return 127, f"(did not run: {e})"
    return done.returncode, (done.stdout or "") + (done.stderr or "")


def git_ok(*args):
    """Did this git command succeed? `git` returns stdout and swallows the status, which is right
    for reading a rev and useless for the one place here that must know whether a `revert`
    actually applied."""
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, encoding="utf-8"
        ).returncode == 0
    except OSError:
        return False


def read_json(path, default=None):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return default


def write_json(path, obj):
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj, indent=2) + "\n", encoding="utf-8", newline="\n")
    except OSError:
        pass


def pack_bytes():
    """The size of the orientation pack as it stands, out of the log `session.py --wrap` keeps.

    Read from the log rather than measured here because measuring means running `orient.py`, and
    the number wanted is the one the *sessions* were charged, not the one a probe would be."""
    last = 0
    for line in (PACKLOG.read_text(encoding="utf-8").splitlines()
                 if PACKLOG.exists() else []):
        if line.strip():
            try:
                last = int(json.loads(line).get("bytes") or last)
            except ValueError:
                pass
    return last


def verify_state():
    """`(green, the step it failed at)`.

    A gate that demanded green outright would roll back every pass whenever the *language* was red
    -- and the language is red most of the time, because a red acceptance check is what the loop is
    working on. So a pass is judged against the tree as it was handed over, not against an ideal.
    `verify.py` prints `verify: FAILED at <step>`, which is the whole of what a comparison needs."""
    code, out = probe("verify.py", timeout=3600)
    if code == 0:
        return True, ""
    for line in out.split("\n"):
        if line.startswith("verify: FAILED at "):
            return False, line[len("verify: FAILED at "):].split()[0]
    return False, f"exit {code}"


def tools_still_load(changed):
    """Every changed `tools/*.py` parses and answers `--help`. Empty string when they all do.

    The cheap half of the code gate, and the half that catches what actually goes wrong: a syntax
    error or an argparse mistake in a script the driver shells out to. It costs a second and it
    does not care what state the Rust tree is in, so unlike `verify.py` it is conclusive."""
    for path in [p for p in changed if p.startswith("tools/") and p.endswith(".py")]:
        if not (ROOT / path).exists():  # the pass deleted it; that is the allowlist's business
            continue
        try:
            done = subprocess.run(
                [sys.executable, "-m", "py_compile", str(ROOT / path)],
                cwd=ROOT, capture_output=True, text=True, timeout=60,
            )
        except (OSError, subprocess.SubprocessError) as e:
            return f"{path} could not be compiled: {e}"
        if done.returncode != 0:
            return f"{path} does not parse: {(done.stderr or '').strip()[:200]}"
        code, out = probe(Path(path).name, "--help", timeout=120)
        if code != 0:
            return f"`python {path} --help` exits {code}: {out.strip()[:200]}"
    return ""


# ------------------------------------------------------------------------------- the signals


def gather_signals(state):
    """What has drifted since the last pass, and the evidence for saying so.

    Returns the signals that fired and the probe output behind them. **No signal, no pass** --
    that is what makes the cadence cheap enough to be wrong about: looking costs four
    subprocesses, and a pass that would have found nothing is a session not spent."""
    fired = []
    ev = {}

    grown = pack_bytes() - int(state.get("pack_bytes") or 0)
    # A pack that grew across a chain switch is the new goal's manifest, which only whoever writes
    # the goal may narrow. Firing on it buys a pass whose one possible finding is that it is not a
    # leak, so the boundary is named in the evidence and the signal stays shut.
    was, goal = state.get("pack_goal") or "", goalsmod.live_slug()
    crossed = bool(was and goal and was != goal)
    ev["pack"] = (f"pack now {pack_bytes():,} B, {grown:+,} B since the last pass "
                  f"({state.get('pack_bytes') or 'never taken'})"
                  + (f", across a chain switch: `{was}` -> `{goal}`" if crossed else ""))
    if state.get("pack_bytes") and grown >= PACK_GROWTH and not crossed:
        fired.append(f"{PACK_SIGNAL} {grown:,} B since the last pass")

    TICKER.set(detail="orient.py --audit")
    code, out = probe("orient.py", "--audit")
    warnings = [ln for ln in out.split("\n") if ln.startswith("!! orient.py:")]
    tail = out.split("== WHAT THIS PACK COST", 1)
    ev["audit"] = ("== WHAT THIS PACK COST" + tail[1]) if len(tail) > 1 else "(no audit section)"
    ev["warnings"] = "\n".join(warnings) or "  none -- every selector resolves"
    if code != 0:
        fired.append(f"orient.py exits {code}: the pack the loop runs on does not build")
        ev["warnings"] += f"\n  orient.py exited {code}"
    if warnings:
        fired.append(f"{len(warnings)} dead selector(s) or stale anchor(s) in the pack")

    TICKER.set(detail="playbook.py --dupes")
    _, out = probe("playbook.py", "--dupes")
    ev["dupes"] = clip(out, 8, 60)
    if "none at this threshold" not in out:
        fired.append("the playbook says the same thing twice")

    TICKER.set(detail="playbook.py --check")
    _, out = probe("playbook.py", "--check")
    ev["playbook_check"] = clip(out, 8, 60)
    if "none -- every path any bullet names still exists" not in out:
        fired.append("a playbook bullet names a path that is no longer in the tree")

    TICKER.set(detail="check-links.py")
    code, out = probe("check-links.py")
    ev["links"] = clip(out, 8, 60)
    if code != 0:
        fired.append("check-links.py reports a dead link")

    # Not a signal of its own -- it fires nothing and cannot. It is what tells the pass *where* the
    # fix for the signals above goes, once the loop is walking goals a tool wrote.
    ev["generated"] = generated_by()

    TICKER.set(detail="")
    return fired, ev


#: A goal file's own banner, when a tool wrote it rather than a person. What a pass needs is not
#: the fact that it is generated but the *command that regenerates it*, because that command is
#: where the fix goes -- see the optimization prompt's menu item 8.
GENERATED_RE = re.compile(r"^#\s*GENERATED by `([^`]+)`", re.M)


def generated_by():
    """The command that wrote the live goal, or "" when a person did."""
    try:
        found = GENERATED_RE.search(GOAL_TOML.read_text(encoding="utf-8"))
    except OSError:
        return ""
    return found.group(1) if found else ""


def ledger_since(n):
    """The ledger's last few lines: what the sessions since the last pass actually reported.

    Bounded by the pass count rather than by the file, because `.loop/log.md` is the whole run
    history and only the recent end of it says anything about the drift being looked at."""
    try:
        lines = LEDGER.read_text(encoding="utf-8").rstrip().split("\n")
    except OSError:
        return "  (no ledger)"
    return "\n".join(lines[-(n * 3 + 12):])


def evidence_pack(fired, ev, since, report_path, baseline):
    """Everything a pass would otherwise spend ten calls fetching, piped in on its stdin.

    The same trick, and for the same measured reason, as a session being handed `orient.py`'s
    output: a result this size comes back through a tool call as a spill notice and a readback,
    which costs more than the text. It also makes the pass *deterministic* -- it chooses among
    findings it was handed rather than deciding what to go and look at."""
    _, stats = probe("loop-stats.py", timeout=600)
    _, attrib = probe("loop-stats.py", "--attribute", timeout=600)
    free = disk.free_gb(ROOT)

    parts = [
        "You are the loop optimization pass. Your evidence follows; the prompt after it says what",
        "you may do with it. Do not re-run any of these to start with -- that is why they are here.",
        "",
        f"== WRITE YOUR REPORT TO: {rel_to_root(report_path)}",
        "",
        "== SIGNALS THAT FIRED",
        *(f"  - {s}" for s in fired),
        "",
        f"== THE {since} SESSION(S) SINCE THE LAST PASS",
        clip(ledger_since(since), 0, 80),
        "",
        "== WHAT THE SESSIONS COST  (python tools/loop-stats.py)",
        clip(stats),
        "",
        "== WHERE THE CONTEXT WENT  (python tools/loop-stats.py --attribute)",
        clip(attrib),
        "",
        "== WHAT THE PACK COST  (python tools/orient.py --audit)",
        ev.get("audit", ""),
        "",
        "== THE PACK'S SLOPE",
        f"  {ev.get('pack', '')}",
        "",
        "== SELECTOR WARNINGS  (python tools/orient.py)",
        ev.get("warnings", ""),
        "",
        "== THE LIVE GOAL",
        (f"  GENERATED by `{ev['generated']}`. Menu item 8 applies: a finding in it is a defect in\n"
         f"  the emitter, fixed there and re-emitted. A hand-edit is discarded by the next emission."
         if ev.get("generated") else
         "  hand-written -- a finding in it is fixed in it, the ordinary case"),
        "",
        "== DUPLICATE PLAYBOOK BULLETS  (python tools/playbook.py --dupes)",
        ev.get("dupes", ""),
        "",
        "== STALE PLAYBOOK PATHS  (python tools/playbook.py --check)",
        ev.get("playbook_check", ""),
        "",
        "== BROKEN LINKS  (python tools/check-links.py)",
        ev.get("links", ""),
        "",
        "== DISK",
        f"  {free:.1f}G free; a run refuses to start below {disk.MIN_FREE_GB}G",
        "",
        "== THE TREE'S VERIFICATION STATE GOING IN",
        ("  green -- tools/verify.py passes, so any red after your pass is yours"
         if baseline[0] else
         f"  RED, at the `{baseline[1]}` step, before you touched anything. That is the loop's own\n"
         "  worklist and it is NOT yours to fix -- do not take a checklist item. It does mean your\n"
         "  pass is judged on whether it made this worse, not on whether it is green."),
        "",
        "",
    ]
    return "\n".join(parts)


# ---------------------------------------------------------------------------------- the pass


def run_pass(opts, fired, ev, since, baseline):
    """One `claude` session against `docs/agent/optimization-prompt.md`, then the smoke test.

    Returns a one-line verdict for the ledger. This is the one place in the whole loop where an
    agent edits the machinery that will drive the next several hours unattended, so what follows
    it is not a review -- it is a set of exit codes, and a `git revert` on any of them."""
    OPTDIR.mkdir(parents=True, exist_ok=True)
    run = run_stamp()
    report = OPTDIR / f"{run}-report.md"
    log = LOGDIR / f"{run}-optimize.log"
    OPTSTATUS.unlink(missing_ok=True)

    base = git("rev-parse", "HEAD")
    dirty = git("status", "--porcelain").strip()
    if dirty:
        say("the tree is not clean going into the pass -- skipping it rather than mixing "
            "somebody's edits into a revert range", C.YELLOW)
        return "SKIPPED (tree not clean)"

    pack = evidence_pack(fired, ev, since, report, baseline)
    (OPTDIR / f"{run}-evidence.md").write_text(pack, encoding="utf-8", newline="\n")
    prompt = OPT_PROMPT.read_text(encoding="utf-8")

    exe = shutil.which("claude") or "claude"
    cmd = [exe, "-p", prompt, "--model", opts.model,
           "--permission-mode", opts.permission_mode,
           "--output-format", "stream-json", "--verbose"]
    if opts.effort:
        cmd += ["--effort", opts.effort]

    say(f"optimization pass {run}: {len(fired)} signal(s), evidence {len(pack):,} B", C.CYAN)
    for s in fired:
        say(f"   - {s}", C.GRAY)
    CONSOLE.open_session(log)
    TICKER.set(phase="optimization pass", detail=run, calls=0)
    renderer = Renderer(opts)
    started = time.monotonic()
    # Its stdin is a pipe and not this console: the evidence goes down it, and the keys stay here.
    # That is what lets `s` and `p` be answered during a pass at all.
    proc = subprocess.Popen(
        cmd, cwd=ROOT, env=session_env(), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=None, encoding="utf-8", errors="replace", bufsize=1,
    )
    threading.Thread(target=feed, args=(proc.stdin, pack), daemon=True).start()
    warned = False
    try:
        assert proc.stdout is not None
        for line in proc.stdout:
            renderer.event(line)
            # Polled here as well as from the ticker, because a run with its stdout redirected has
            # no ticker at all, and `.loop/stop` is then the only channel there is.
            CONTROL.poll()
            if not warned and CONTROL.stop_reason():
                warned = True
                say("   a stop is armed -- a pass is a session like any other, so it finishes and "
                    "settles first, and the run ends after it", C.YELLOW)
        proc.wait()
    except BaseException:
        # The same rule as a work session: an agent with permissions bypassed does not outlive
        # the process watching it.
        proc.kill()
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            pass
        raise
    finally:
        CONSOLE.close_session()
    say(f"pass ended after {mmss(time.monotonic() - started)}, claude exit {proc.returncode}",
        C.CYAN)

    status = ""
    if OPTSTATUS.exists():
        status = OPTSTATUS.read_text(encoding="utf-8").strip()
    if proc.returncode != 0:
        # A refused or crashed pass has usually committed nothing, and a usage wall is the likeliest
        # cause of both. Judge the tree, not the exit code, then hand the wall back to the leg,
        # which is the only thing here that knows how to wait one out.
        say(f"the pass exited {proc.returncode} -- checking what it left behind", C.YELLOW)

    TICKER.set(phase="settling the pass", detail=run)
    verdict = settle(base, run, report, status, baseline)
    write_json(OPTSTATE, {
        "last_pass": run,
        "last_pass_head": git("rev-parse", "HEAD"),
        "pack_bytes": pack_bytes(),
        "pack_goal": goalsmod.live_slug(),
        "verdict": verdict,
        "signals": fired,
    })
    return verdict


def settle(base, run, report, status, baseline):
    """Decide whether what the pass committed may stay, and undo it if not.

    Five gates, in the order a failure is cheapest to find in: what it touched, whether it left
    anything behind, whether the loop still orients and reads its goal, whether every script it
    edited still loads, and -- only when it edited code the loop executes -- whether the tree got
    worse. Nothing here reads the diff. The point is that the decision is mechanical, because the
    thing being judged is an agent's edit to the judge."""
    head = git("rev-parse", "HEAD")
    if head == base:
        left = git("status", "--porcelain").strip()
        if left:
            git("stash", "push", "-u", "-m", f"optimization pass {run}: uncommitted")
            say("the pass left uncommitted edits and no commit -- stashed them", C.YELLOW)
            return "STASHED (edits without a commit)"
        return status or "CLEAN (nothing committed)"

    shas = [s for s in git("rev-list", f"{base}..{head}").split("\n") if s]
    changed = [p for p in git("diff", "--name-only", base, head).split("\n") if p]
    say(f"the pass committed {len(shas)} change(s) over {len(changed)} file(s)", C.GRAY)

    stray = [p for p in changed if not p.startswith(ALLOWED)]
    if stray:
        return rollback(base, shas, f"it touched {', '.join(stray[:4])}"
                        + (f" and {len(stray) - 4} more" if len(stray) > 4 else ""))

    leftover = git("status", "--porcelain").strip()
    if leftover:
        git("stash", "push", "-u", "-m", f"optimization pass {run}: uncommitted tail")
        say("stashed an uncommitted tail the pass left behind", C.YELLOW)

    code, out = probe("orient.py")
    if code != 0 or len(out) < 500:
        return rollback(base, shas, f"orient.py exits {code} with {len(out)} bytes of pack")
    code, _ = probe("loop.py", "--list")
    if code != 0:
        return rollback(base, shas, f"loop.py --list exits {code}: the acceptance list is unreadable")

    broken = tools_still_load(changed)
    if broken:
        return rollback(base, shas, broken)

    if any(p.startswith("tools/") for p in changed):
        say("the pass edited tools/ -- verifying before handing the loop back", C.CYAN)
        ok, failed_at = verify_state()
        if not ok and baseline[0]:
            return rollback(base, shas, f"verify.py was green going in and now FAILS at {failed_at}")
        if not ok and failed_at != baseline[1]:
            return rollback(base, shas, f"verify.py failed at {baseline[1]} going in and now "
                                        f"fails at {failed_at} instead")
        if not ok:
            say(f"verify.py still fails at {failed_at}, exactly as it did before the pass -- not "
                "the pass's doing, so it stands", C.YELLOW)

    say(f"the pass stands: {status or '(no status line)'}", C.GREEN)
    if report.exists():
        say(f"report: {rel_to_root(report)}", C.GRAY)
    return status or f"APPLIED {len(shas)}"


def rollback(base, shas, why):
    """Put the tree back, loudly.

    `revert` rather than `reset`, so the run's history still shows the pass and what was undone --
    and a `reset` only as the fallback for a revert that conflicts, which it can only do against
    commits the pass itself made, since no other writer runs between two sessions."""
    say("", C.RED)
    say(f"ROLLING BACK the optimization pass: {why}", C.RED)
    ok = bool(shas)
    for sha in shas:  # rev-list is newest first, which is the order a revert must take
        if not git_ok("revert", "--no-edit", "--no-commit", sha):
            ok = False
            break
    if ok:
        ok = git_ok("commit", "-m",
                    f"revert: the optimization pass was rolled back -- {why}")
    if not ok:
        git("revert", "--abort")
        git("reset", "--hard", base)
        say("the revert did not apply cleanly; reset to the pre-pass commit instead", C.YELLOW)
    say("the loop continues on the code it had before the pass", C.YELLOW)
    return f"REVERTED ({why.splitlines()[0]})"


# ---------------------------------------------------------------------------------- driving


def checkpoint(opts, since):
    """A look for drift, between two sessions; spend a session on it only if something is
    actually wrong.

    Returns the session count to carry forward -- 0 when the counter was spent, `since` when it
    was not, because a look that found nothing must not reset the clock on the next look. This
    owns every write to `state.json`, so the count and the pack size it is measured against can
    never be written from two different reads of the same file."""
    if opts.no_optimize:
        return since
    state = read_json(OPTSTATE, default={}) or {}
    say("")
    say(f"== checkpoint: {since} session(s) since the last pass", C.MAGENTA)
    TICKER.set(scope="checkpoint", phase="looking for drift", detail="")
    fired, ev = gather_signals(state)
    due = since >= opts.optimize_every
    early = since >= opts.min_pass_gap and any(s.startswith(PACK_SIGNAL) for s in fired)
    if not fired:
        say("nothing has drifted -- no pass", C.GREEN)
        ledger(f"## run checkpoint {datetime.now():%Y-%m-%d %H:%M} -- "
               f"clean after {since} session(s), no pass")
        # A clean look still spends the clock: the next one is a full cadence away, and the pack
        # it will compare against is this one, not the one the last *pass* left.
        write_json(OPTSTATE, {**state, "since": 0 if due else since,
                              "pack_bytes": pack_bytes() if due else state.get("pack_bytes"),
                              "pack_goal": goalsmod.live_slug() if due else state.get("pack_goal")})
        return 0 if due else since
    if not (due or early):
        say(f"{len(fired)} signal(s), but only {since} of {opts.optimize_every} sessions in -- "
            "carrying them to the next checkpoint", C.GRAY)
        for s in fired:
            say(f"   - {s}", C.GRAY)
        # Ledgered like the other two outcomes. This branch used to be silent, and a run whose
        # every checkpoint took it left no trace that the cadence had fired at all.
        ledger(f"## run checkpoint {datetime.now():%Y-%m-%d %H:%M} -- "
               f"{len(fired)} signal(s) after {since} session(s), carried: " + "; ".join(fired))
        write_json(OPTSTATE, {**state, "since": since})
        return since

    dirty = git("status", "--porcelain").strip()
    if dirty:
        # Somebody is editing this tree by hand, which the loop allows. A pass over a dirty tree
        # would mix their edits into its revert range, so it waits -- and the counter waits with
        # it, so this is asked again straight after the next session (`look_due`). It does
        # not reset: a pass deferred is not a pass taken. Checked before the baseline measurement
        # below, which is a full `verify.py` and worth nothing if the pass is not going to run.
        say(f"{len(fired)} signal(s) and the pass is due, but the tree is not clean -- "
            "deferring it until the next session is over", C.YELLOW)
        for s in fired:
            say(f"   - {s}", C.GRAY)
        ledger(f"## run checkpoint {datetime.now():%Y-%m-%d %H:%M} -- "
               f"{len(fired)} signal(s) after {since} session(s), pass DEFERRED: "
               f"the tree is not clean ({len(dirty.splitlines())} path(s))")
        write_json(OPTSTATE, {**state, "since": since})
        return since

    # A pass is a `claude` session that edits this tree exactly the way a work session does, so the
    # two things that stop a session from starting stop this one too. Asked here and not only in
    # `turn` before the look because gathering the signals is minutes of probes, and a hold queued
    # during them would otherwise be answered by a session starting anyway. `hold_pause` blocks
    # for as long as the hold stands and returns why to stop, or "" to carry on.
    if hold_pause() or CONTROL.stop_reason():
        say("a stop or a hold arrived while the signals were being gathered -- not starting the "
            "pass; the signals carry to the next boundary", C.YELLOW)
        write_json(OPTSTATE, {**state, "since": since})
        return since

    say("measuring the tree before the pass, so it is judged on what it changed", C.GRAY)
    TICKER.set(phase="baseline verify", detail="")
    baseline = verify_state()
    say(f"   verify.py going in: {'green' if baseline[0] else 'RED at ' + baseline[1]}",
        C.GREEN if baseline[0] else C.YELLOW)
    verdict = run_pass(opts, fired, ev, since, baseline)
    ledger(f"## optimization pass {datetime.now():%Y-%m-%d %H:%M} after {since} session(s) "
           f"-- {verdict}")
    if verdict.startswith("BROKEN"):
        raise SystemExit(f"the optimization pass reported {verdict}")
    if verdict.startswith("SKIPPED"):
        return since  # not taken, so not spent -- the same deferral as the dirty-tree branch
    return 0


def sessions_since():
    """Served sessions since the last optimization pass, as `.loop/optimization/state.json` has it."""
    return int((read_json(OPTSTATE, default={}) or {}).get("since") or 0)


def credit(served=1):
    """Count a served session towards the next checkpoint, and persist it.

    The one write to `since` that is not a checkpoint's, so a checkpoint that finds nothing and
    a run that ends -- either way -- agree on the number the next checkpoint reads."""
    state = read_json(OPTSTATE, default={}) or {}
    write_json(OPTSTATE, {**state, "since": int(state.get("since") or 0) + served})


def look_due(opts, since):
    """Is this boundary one where the run looks for drift?

    Every `--probe-every` sessions, and at every boundary once `--optimize-every` is reached. The
    second half is what makes a deferred pass -- the tree was not clean -- get asked again after
    the very next session instead of a whole probe interval later: a deferral keeps the count,
    and the count stays past the cadence until a pass is taken."""
    if opts.no_optimize or since <= 0:
        return False
    return since >= opts.optimize_every or since % max(1, opts.probe_every) == 0


def settle_stop():
    """Wait out an `s` that is still inside its cancel window.

    The flag lives in this process and this process is about to end, so a stop pressed in the
    last seconds of a turn would otherwise be neither acted on nor cancellable: the next turn
    never heard it. The wait is at most `Control.STOP_GRACE`, and only after the key."""
    while CONTROL.stop_in() > 0:
        CONTROL.poll()
        time.sleep(0.1)


def open_run(opts, run):
    """What a run says and does once, on its first turn, before its first session."""
    ledger("")
    # The effort is in the header, not on each session line: it is a property of the run, and this
    # is the row that maps a run stamp onto a setting -- which is the whole of what an A/B between
    # two settings needs, since `loop-stats.py --run <stamp>` prices a run.
    effort = f", effort {opts.effort}" if opts.effort else ""
    ledger(f"## run started {datetime.now():%Y-%m-%d %H:%M} "
           f"(max {sessions_label(opts.max_sessions)}{effort}, logs {run.run_id}-*)")
    say(f"console log: {rel_to_root(LOGDIR / f'{run.run_id}-console.log')}", C.GRAY, driver=True)
    # The key row under the status line says this continuously and says it in context, so it is
    # only worth a line when that row is not there -- which is a redirected STDOUT, not a missing
    # stdin. The two are independent: keys are readable whenever stdin is a console, and the row
    # is painted only when stdout is one, so `nohup` gets the files and `loop.py > log` gets both.
    if not (CONTROL.tty and TICKER.enabled):
        say("controls: " + ("press r to end a usage wait early, s to stop after the current "
                            "session, p to hold after it without ending the run, h to halt "
                            "the running session at once and again to carry on, i to type it "
                            "a prompt" if CONTROL.tty
                            else
                            f"create {rel_to_root(RETRY)} to end a usage wait early, "
                            f"{rel_to_root(STOP)} to stop after the current session, "
                            f"{rel_to_root(PAUSE)} to hold after it until the file goes, "
                            f"{rel_to_root(HALT)} to freeze the running session until the file "
                            f"goes, {rel_to_root(SAY)} with a prompt in it to send it one"),
            C.GRAY, driver=True)
    # Every acceptance check builds the debug CLI, so from the second session on it is current at
    # the tree the next session starts from -- and orient.py's closing block tells the session so,
    # to stop it spending a call on `ls -la target/debug/nvs.exe` and a defensive `cargo build`.
    # The first session of a run is the one case that promise would be false, because no check has
    # run in front of it yet. So it runs here. It is a no-op against a warm target/ and it is not
    # fatal: a red build is a thing a session may be sent to fix, and the goal check reports it
    # either way.
    warm = NativeLeg().prepare()
    if warm:
        say(f"the debug CLI is not built: {warm}", C.YELLOW, driver=True)
    run.save()


def finish(run, kind, reason):
    """The run ends here. Says why, once, in the ledger and on the console; returns the exit code."""
    ledger(f"## run ended {datetime.now():%Y-%m-%d %H:%M} -- {reason}")
    done = kind in ("budget", "side-landed")
    verdict(not done, reason if done else f"the run ended for good: {reason}")
    say("")
    say(f"run done: {run.served} session(s)", C.CYAN)
    return 0


def again(run):
    """The turn is over and the run is not: ask `tools/respawn.py` for the next one.

    The last look at the console this process gets, so a stop or a hold that arrived during the
    boundary -- a checkpoint can be a whole optimization session long -- is answered here, by
    the process that heard it.

    A release build the acceptance check started in the background may still be running, since
    a sweep that fails early returns without joining it. The work is wanted either way, so the
    turn waits for it: the thread dies with this process, and the `cargo` under it would be left
    building with nobody to free the release binary an editor holds (`relink`)."""
    settle_stop()
    stop = hold_pause()
    if stop:
        return finish(run, "asked", stop)
    if not PREBUILD_LOCK.acquire(blocking=False):
        step("waiting for the release build the acceptance check left running")
        TICKER.set(phase="waiting for the release build", detail="")
        PREBUILD_LOCK.acquire()
    PREBUILD_LOCK.release()
    run.save()
    return respawn.AGAIN


def merge_yield():
    """Wait here while a side run lands on `main`. Returns "" when it has, or the reason to stop.

    Called at the boundary where a chain run touches nothing -- before a session starts, beside
    `hold_pause` -- and answered by writing `.loop/yielded`, which is what lets the side run move
    `main` (`tools/side.py` § *The handshake*). A side run never yields: it does not touch `main`
    until it lands, and two side runs take `merge.lock` in turn."""
    if SIDE:
        return ""
    asked = sidemod.merge_requested(ROOT)
    if not asked:
        return ""
    slug = asked[1] or "?"
    sidemod.write_marker(ROOT / sidemod.YIELDED, slug=slug)
    step(f"side goal `{slug}` is landing on main -- waiting before the next session", C.YELLOW)
    TICKER.set(phase="yielded", detail=f"side goal {slug} is landing on main")
    began = time.monotonic()
    try:
        while sidemod.merge_requested(ROOT):
            CONTROL.poll()
            stop = CONTROL.stop_reason()
            if stop:
                return stop
            time.sleep(1)
    finally:
        (ROOT / sidemod.YIELDED).unlink(missing_ok=True)
    spent = hms(time.monotonic() - began)
    step(f"yielded {spent} -- side goal `{slug}` is done with main", C.GREEN)
    ledger(f"       yielded {spent} to side goal `{slug}` landing on main")
    return ""


def side_preamble():
    """What a side session is told ahead of `session-prompt.md`, or "" in a chain run.

    The prompt and the orientation pack speak of `loop-goal.*` and `handoff.md`, and every tool a
    session runs already reads the side goal's files instead (`goals.SIDE_ENV`). What only a
    sentence can carry is what not to touch, and why a landing did not happen last time."""
    if not SIDE:
        return ""
    rel = f"docs/agent/goals/side/{SIDE.slug}"
    note = ""
    landing = ROOT / sidemod.LANDING
    if landing.is_file():
        note = ("\n**The last landing did not land, and this session fixes that first:** "
                + landing.read_text(encoding="utf-8").strip() + "\n")
    return (f"# Side goal `{SIDE.slug}`\n\n"
            f"This session belongs to a **side run**: one goal, in its own worktree on branch "
            f"`{sidemod.branch(SIDE.slug)}`. Your goal is `{rel}.md`, its acceptance list "
            f"`{rel}.toml`, and your handoff `{rel}.handoff.md` -- `session.py --wrap` writes it "
            f"there, and `orient.py`, `brief.py` and `loop.py --goal-only` already read these "
            f"three wherever the prompt below says `loop-goal.*` or `handoff.md`. Every rule "
            f"below applies unchanged, with four more:\n\n"
            f"- Do not edit `docs/agent/loop-goal.md`, `docs/agent/loop-goal.toml`, "
            f"`docs/agent/handoff.md`, the plan's status block, or any other goal's files. They "
            f"belong to the chain run on `main`.\n"
            f"- Never rebase, merge, push or switch branch. The driver lands this branch on "
            f"`main` itself once the goal is green, and retires the goal as it does.\n"
            f"- The goal's list runs over main's carried floor too, so a check you did not write "
            f"can hold it red. Fix the code, never the floor check.\n"
            f"- A decision record takes the next free number on `main` "
            f"(`git -C {sidemod.main_root().as_posix()} ls-tree main docs/decisions/`), which "
            f"may be ahead of this branch.\n"
            f"{note}\n---\n\n")


def landed(run, kind, reason):
    """The exit code for what `land_side` answered: the run ends when it landed or was told to
    stop, and carries on with another session when `main` does not pass over the branch."""
    if kind == "side-red":
        ledger(f"## landing held -- {reason}")
        verdict(False, f"{reason}\n       The run carries on: the next session is told why.")
        return again(run)
    return finish(run, kind, reason)


def land_side(run):
    """Land the side goal on `main`, in `tools/side.py` § *Landing*'s order. Returns `(kind,
    reason)`: `side-landed`, `side-red` when main's tree does not pass over the rebased branch,
    `side-conflict` when the rebase does not apply, or `asked` for a stop that came while waiting.

    The verify runs with no lock held, since it takes minutes and the chain run keeps working
    meanwhile. So the lock is taken after it and `main` is compared with the commit the verify
    ran over: moved, and the rebase and verify run again, so only a verified tree ever lands."""
    slug, wt = SIDE.slug, ROOT
    main = sidemod.main_root()
    landing = ROOT / sidemod.LANDING
    while True:
        stop = CONTROL.stop_reason()
        if stop:
            return "asked", stop
        dirty = sidemod.main_dirty(wt)
        if dirty:
            return "side-red", (f"the worktree has uncommitted changes to {len(dirty)} tracked "
                                f"file(s) ({', '.join(dirty[:5])}); commit or revert them")
        base = sidemod.git(main, "rev-parse", "main")
        step(f"landing: rebasing {sidemod.branch(slug)} onto main at {base[:10]}", C.CYAN)
        TICKER.set(phase="landing", detail="rebasing onto main")
        try:
            sidemod.git(wt, "rebase", base)
        except RuntimeError as e:
            sidemod.git(wt, "rebase", "--abort", check=False)
            return "side-conflict", (f"{sidemod.branch(slug)} does not rebase onto main cleanly. "
                                     f"Rebase it by hand in {wt.as_posix()} and resolve it; the "
                                     f"next green sweep lands it. {e}")
        fail = landing_verify()
        if fail:
            landing.write_text(fail + "\n", encoding="utf-8", newline="\n")
            return "side-red", fail
        landing.unlink(missing_ok=True)

        step("landing: verified over main -- asking the chain run for main", C.CYAN)
        TICKER.set(phase="landing", detail="waiting for main")
        waited = time.monotonic()
        while not sidemod.write_marker(main / sidemod.MERGE_LOCK, slug=slug):
            CONTROL.poll()
            stop = CONTROL.stop_reason()
            if stop:
                return "asked", stop
            time.sleep(sidemod.POLL_SECONDS)  # another side run is landing
        try:
            said = ""
            while True:
                quiet = sidemod.main_quiet(main)
                dirty = [] if not quiet else sidemod.main_dirty(main)
                if quiet and not dirty:
                    break
                why = (f"main has uncommitted changes to {', '.join(dirty[:3])}" if dirty
                       else "the chain run has not reached a boundary yet")
                if why != said:
                    step(f"landing: waiting -- {why}", C.YELLOW)
                    said = why
                TICKER.set(detail=f"{why}{TICKER.sep}{hms(time.monotonic() - waited)}")
                for _ in range(sidemod.POLL_SECONDS * 4):
                    CONTROL.poll()
                    stop = CONTROL.stop_reason()
                    if stop:
                        return "asked", stop
                    time.sleep(0.25)
            if sidemod.git(main, "rev-parse", "main") != base:
                step("landing: main moved during the verify -- rebasing and verifying again",
                     C.YELLOW)
                continue
            # Read before the retire deletes the `.toml` it comes from; the launcher removes it.
            try:
                wsl = load_goal().wsl_target
            except (OSError, tomllib.TOMLDecodeError, GoalError):
                wsl = ""
            before = sidemod.git(wt, "rev-parse", "HEAD")
            try:
                carried = sidemod.carry_into_live(SIDE.toml, wt / sidemod.LIVE_GOAL)
                sidemod.git(wt, "rm", "-q", "--", str(SIDE.toml), str(SIDE.handoff))
                sidemod.git(wt, "add", "--", str(wt / sidemod.LIVE_GOAL))
                message = wt / ".agent-tmp" / "side-landing-commit.txt"
                message.parent.mkdir(parents=True, exist_ok=True)
                message.write_text(
                    f"docs(loop): side goal `{slug}` lands on main, and its {carried} check(s) "
                    f"join the floor\n\nThe side goal's `.toml` and `.handoff.md` are retired; its "
                    f"`.md` stays under docs/agent/goals/side/.\n", encoding="utf-8", newline="\n")
                sidemod.git(wt, "commit", "-q", "-F", str(message))
                sidemod.git(main, "merge", "--ff-only", "-q", sidemod.branch(slug))
            except (OSError, RuntimeError) as e:
                # Nothing reached `main`: put the branch back as the verify saw it, and ask.
                sidemod.git(wt, "reset", "-q", "--hard", before, check=False)
                return "side-conflict", (f"landing failed after the verify, and main is "
                                         f"unchanged: {e}")
            head = sidemod.git(main, "rev-parse", "--short", "main")
        finally:
            (main / sidemod.MERGE_LOCK).unlink(missing_ok=True)
        sidemod.write_marker(ROOT / sidemod.LANDED, slug=slug, head=head, wsl=wsl)
        return "side-landed", (f"side goal `{slug}` landed on main at {head}, with {carried} "
                               f"check(s) added to the floor")


def landing_verify():
    """`verify.py`, then the whole acceptance list with the floor gate open, over the rebased
    branch. Returns "" when both are green, or one line saying which was red and where."""
    step("landing: verify.py over the rebased branch", C.CYAN)
    TICKER.set(detail="verify.py")
    r = capture(sys.executable, [str(ROOT / "tools" / "verify.py")], timeout=7200)
    if r.code != 0:
        text = ((r.out or "") + "\n" + (r.err or "")).replace("\r\n", "\n")
        last = next((ln.strip() for ln in reversed(text.split("\n")) if ln.strip()), "")
        return f"verify.py is red over the branch rebased onto main: {last}"
    dirty = sidemod.main_dirty(ROOT)
    if dirty:
        return (f"verify.py changed {len(dirty)} tracked file(s) ({', '.join(dirty[:5])}); "
                f"commit what it wrote")
    try:
        goal = load_goal()
    except (OSError, tomllib.TOMLDecodeError, GoalError) as e:
        return f"the acceptance list does not load after the rebase: {e}"
    goal.floor_gate = True
    goal.collect = True
    step("landing: the acceptance list, floor gate open", C.CYAN)
    fail = goal.check(verbose=True)
    ledger(f"       landing cost: {goal.summary()}")
    return f"the acceptance list is red over the branch rebased onto main: {fail}" if fail else ""


def launch_side(opts):
    """`--side <slug>` typed in the main tree: prepare the worktree, run the side run in it, and
    remove it once it has landed. `tools/side.py` § *The run* says why the launcher is a separate
    process from every turn."""
    slug = opts.side
    main = sidemod.main_root()
    if main.resolve() != ROOT.resolve():
        say(f"--side runs from the main tree ({main.as_posix()}), not from a worktree", C.RED)
        return 2
    goal = goalsmod.SideGoal(slug)
    if goal.retired and goal.md.is_file():
        say(f"side goal `{slug}` is retired -- it has landed already", C.YELLOW)
        return 2
    wt, why = sidemod.prepare(slug, main, disk.free_gb(ROOT), opts.min_free_gb)
    if not wt:
        say(f"side goal `{slug}`: {why}", C.RED)
        return 2
    say(f"side goal `{slug}`: running in {wt.relative_to(main).as_posix()} on branch "
        f"{sidemod.branch(slug)}", C.CYAN)
    os.environ[goalsmod.SIDE_ENV] = slug
    code = respawn.run(wt / "tools" / "loop.py", sys.argv[1:], cwd=wt)
    marker = wt / sidemod.LANDED
    if not marker.is_file():
        return code
    fields = dict(line.split(":", 1) for line in marker.read_text(encoding="utf-8").splitlines()
                  if ":" in line)
    left = sidemod.cleanup(slug, main, fields.get("wsl", "").strip())
    for what in left:
        say(f"side goal `{slug}` landed, but could not remove {what}", C.YELLOW)
    if not left:
        say(f"side goal `{slug}` landed; its worktree, branch and WSL target are removed", C.GREEN)
    return code


def turn(opts, goal, chain, run):
    """One turn of the run: a session, and the boundary behind it. Returns the process's exit
    code, which is `respawn.AGAIN` exactly when the run goes on.

    The console belongs to this process for the whole of it, so `s`, `p` and `r` mean the same
    thing through a checkpoint -- which can be a full `verify.py` and then an optimization
    session -- as they do inside a work session."""
    LOGDIR.mkdir(parents=True, exist_ok=True)
    CONSOLE.open_run(LOGDIR / f"{run.run_id}-console.log")
    CONTROL.enable()
    if SIDE and opts.land:
        # By hand, with no session to send a red result to: whatever the answer, it is the end.
        # Ahead of `open_run`, whose warm-up build exists for a session that is not coming.
        kind, reason = land_side(run)
        finish(run, kind, reason)
        return 0 if kind == "side-landed" else 1
    if run.fresh:
        open_run(opts, run)
    if run.served >= opts.max_sessions and not run.judge:
        return finish(run, "budget", f"hit --max-sessions ({opts.max_sessions})")

    kind, reason = drive(opts, goal, chain, run)

    if kind == "rejudge":
        # Straight to the next turn: no checkpoint and no budget check stand between a session
        # and its verdict.
        say(reason, C.CYAN)
        return again(run)

    if kind == "side-land":
        kind, reason = land_side(run)
        if kind in ("side-landed", "side-red", "asked"):
            return landed(run, kind, reason)

    if kind == SERVED:
        run.last_hand = []
        # A boundary is a moment no session is running, and it is not an idle one: the checkpoint
        # below may spend it on an optimization session that edits this tree exactly the way a
        # work session does. So a stop or a hold is answered before it as well as after it.
        settle_stop()
        stop = hold_pause()
        if stop:
            return finish(run, "asked", stop)
        if run.served >= opts.max_sessions:
            # No checkpoint on the way out. A pass exists to make the *next* sessions cheaper,
            # and there are none.
            return finish(run, "budget", f"hit --max-sessions ({opts.max_sessions})")
        since = sessions_since()
        if look_due(opts, since):
            after = checkpoint(opts, since)
            state = read_json(OPTSTATE, default={}) or {}
            write_json(OPTSTATE, {**state, "since": after})
        return again(run)

    if kind in REPAIR_KINDS and not run.repair and run.repairs < REPAIRS_PER_GOAL:
        # Not held: the next turn's session is given the verdict to fix, as the user would give
        # it. Whatever `--no-hold` says, since nobody is asked. A repair still owed is one whose
        # session was never served -- `claude` itself failing, say -- and a second one would not
        # be served either, so that verdict holds, as does one past the goal's repairs.
        run.repairs += 1
        run.repair = f"{kind}: {reason}"
        note = (f"not holding on {kind} -- a repair session gets it first "
                f"({run.repairs} of {REPAIRS_PER_GOAL} on this goal): {reason}")
        ledger(f"## {note}")
        verdict(False, note)
        return again(run)

    hand = [kind, reason]
    if opts.hold_on_hand and kind in HOLD_KINDS and hand != run.last_hand:
        # The verdict the hold is taken on is kept: a turn that comes back with the same one was
        # not answered -- the hold was lifted and nothing changed -- and holding again would spend
        # another session to ask the identical question, so the second one ends the run.
        run.last_hand = hand
        run.save()
        verdict(True, f"{reason}\n       The run is HOLDING, not ending. Answer it, then "
                      f"press p -- or delete {rel_to_root(PAUSE)} -- to carry on with a "
                      f"fresh session; s ends the run.")
        CONTROL.arm_hold(reason)
        stop = hold_pause()
        if stop:
            return finish(run, kind, f"{reason}, and then: {stop}")
        say("the hold is lifted -- another session, on the tree as it stands", C.GREEN)
        ledger(f"## run held on {kind} and carried on -- {reason}")
        return again(run)
    if hand == run.last_hand:
        return finish(run, kind, f"{reason} -- twice, with a hold in between, so the run ends "
                                 f"rather than asking the same question again")
    return finish(run, kind, reason)


def optimize_only(opts):
    """`--optimize-only`: one pass now, against the tree as it stands, and out.

    It claims nothing and prunes nothing. This is a person asking for the pass by hand, usually
    with no run in flight at all, and a marker taken here would be one more thing to clean up."""
    if not OPT_PROMPT.exists():
        say(f"missing {rel_to_root(OPT_PROMPT)}", C.RED)
        return 2
    if opts.status:
        TICKER.start()
    CONTROL.enable()
    state = read_json(OPTSTATE, default={}) or {}
    fired, ev = gather_signals(state)
    if not fired:
        say("no signal fired -- running the pass anyway, because you asked for it", C.YELLOW)
        fired = ["--optimize-only: run by hand, against whatever the evidence says"]
    say(run_pass(opts, fired, ev, int(state.get("since") or 0), verify_state()), C.CYAN)
    return 0


if __name__ == "__main__":
    sys.exit(main())
