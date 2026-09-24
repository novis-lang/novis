#!/usr/bin/env python3
"""Where a tool records the tracked files it just wrote, so the loop's sweep can recognise them.

`loop.py` commits whatever an interrupted session left behind in the tree. To do that without
taking anything that is not the session's, it has to know which dirty paths the session wrote --
because the loop shares one working tree with whoever is using the machine, and a sweep that
stages everything commits a person's open work under a `wip(loop)` subject naming a session that
never opened those files.

A session's own tool calls answer most of it: `Write` and `Edit` name their target, and the driver
reads it off the event stream as it goes past. What they cannot answer is a write one of this
repository's own tools performs on the session's behalf -- `nv splice` applies a patch across any
number of files, and `reference.py` regenerates `docs/novis.md` in place. Both are the session's
work, and neither reaches the stream as a path.

So those tools record it here. `record()` appends to the file named by `$NOVIS_LOOP_WRITES`, and
does nothing whatsoever when that variable is unset -- which is every run outside the loop,
including these same tools run by hand while a session happens to be in flight. The driver sets
the variable on the session it spawns and on nothing else, and truncates the file before each one.
"""

from __future__ import annotations

import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

#: The driver names the ledger through this variable, on the spawned session's environment alone.
#: Unset means nothing is watching and `record` is a no-op, which is the common case by far.
ENV = "NOVIS_LOOP_WRITES"


def record(*paths) -> None:
    """Note that this process wrote `paths`, in the spelling `git status` uses.

    Never raises and never reports. Every caller has already finished its real work by the time it
    gets here, and a failure to keep a bookkeeping note must not turn a successful write into a
    non-zero exit."""
    ledger = os.environ.get(ENV)
    if not ledger or not paths:
        return
    lines = []
    for path in paths:
        try:
            lines.append(Path(path).resolve().relative_to(ROOT).as_posix())
        except (ValueError, OSError):
            lines.append(str(path).replace("\\", "/"))
    try:
        with open(ledger, "a", encoding="utf-8", newline="\n") as handle:
            handle.write("".join(f"{line}\n" for line in lines))
    except OSError:
        pass
