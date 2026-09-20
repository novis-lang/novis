#!/usr/bin/env python3
"""Starts `tools/loop.py`, and starts it again every time it asks. Nothing else.

This is the one process of a run that lives as long as the run does, so it is the one process
whose code cannot change under it -- and the whole of what it is for is that this does not
matter. It holds no loop logic: it prints nothing while a run is healthy, reads no key, opens no
file under `.loop/`, parses no flag. Every decision -- whether there is another session, a hold,
a checkpoint, an optimization pass, why a run ends -- is made by the `loop.py` it started, which
is a fresh interpreter reading the file as it stands on disk. An edit to `loop.py` is therefore
live at the next session, whoever made it.

It is never run by hand. `python tools/loop.py ...` calls `run` below when it finds it was not
started from here, so the command is the one it always was.

The protocol is an exit code and an environment variable:

* `AGAIN` from the child means *start me again*. Any other code ends the run and is passed
  through as this process's own -- a crash, a refusal at the door and a finished run all end it,
  so a `loop.py` that no longer starts is a run that stops, never one that spins.
* `ENV` in the child's environment names the run. It is how a `loop.py` tells "I am the next
  turn of the run that holds `.loop/running`" from "somebody else's run holds it".

Stdio is inherited and never touched, so the console -- the keys and the status rows -- belongs
to the one child alive at any moment. A Ctrl-C reaches both processes at once; the child's
handler commits what the session left, and this waits for it rather than racing it.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time

#: The child's "start me again". 75 is `EX_TEMPFAIL`: not a success, not a failure, and not a
#: code Python, argparse or a signal produces by accident.
AGAIN = 75

#: Names the run in every turn's environment; see the module doc.
ENV = "NOVIS_LOOP_RUN"

#: How long a child is given to finish its Ctrl-C handler -- a sweep and a commit -- before it
#: is killed.
INTERRUPT_GRACE = 20

#: The guard against a child that asks to be started again without having done anything: this
#: many turns in a row, each shorter than `QUICK` seconds, ends the run. A real turn is a
#: session and an acceptance check, which is minutes.
QUICK = 5.0
MAX_QUICK = 5


def run(script, argv, cwd=None):
    """Drive `script` until it exits with anything but `AGAIN`, and return that exit code."""
    env = {**os.environ, ENV: f"{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}"}
    quick = 0
    while True:
        started = time.monotonic()
        proc = subprocess.Popen([sys.executable, str(script), *argv], cwd=cwd, env=env)
        try:
            code = proc.wait()
        except KeyboardInterrupt:
            return interrupted(proc)
        if code != AGAIN:
            return code
        quick = quick + 1 if time.monotonic() - started < QUICK else 0
        if quick >= MAX_QUICK:
            sys.stderr.write(
                f"respawn: {os.path.basename(str(script))} asked to be started again {quick} "
                f"times in a row within {QUICK:.0f}s each, so the run ends here rather than "
                f"spinning. If `.loop/running` is still there, delete it before the next run.\n")
            return 1


def interrupted(proc):
    """Ctrl-C: let the child finish its own handler, and end the run with its exit code."""
    deadline = time.monotonic() + INTERRUPT_GRACE
    while True:
        try:
            return proc.wait(timeout=max(0.1, deadline - time.monotonic()))
        except subprocess.TimeoutExpired:
            proc.kill()
            return proc.wait()
        except KeyboardInterrupt:
            continue  # a second Ctrl-C while waiting is the same request, not a new one


if __name__ == "__main__":
    # `--help` answers and exits 0 because the optimization pass's load check asks every changed
    # tool exactly that (`loop.tools_still_load`). Anything else is somebody running this by hand.
    if {"-h", "--help"} & set(sys.argv[1:]):
        print(__doc__)
        raise SystemExit(0)
    sys.stderr.write("respawn.py is not run by hand -- `python tools/loop.py` starts it.\n")
    raise SystemExit(2)
