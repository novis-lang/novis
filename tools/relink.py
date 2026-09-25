#!/usr/bin/env python3
"""Freeing `target/release/nvs` when a running copy of it holds the path cargo has to link.

Windows refuses to delete a file that is mapped into a running process, and cargo's last step is to
remove the old executable and put the new one where it was. An editor whose `nvs.path` points into
this tree runs `nvs lsp` for the life of its window, so every release build made while that window
is open fails at the link with `failed to remove file`, and the tree keeps whatever binary was
there before.

That is an annoyance for somebody building by hand and a stop for the unattended loop, whose
acceptance checks judge a proof against the release binary: the check asks cargo for one that is
current, cargo cannot produce it, and a goal fails for a reason that has nothing to do with the
tree. The failure also reads as *the tree does not build in release*, which is the opposite of what
happened.

**Renaming the file is allowed where deleting it is not.** The running process keeps executing the
image it has already mapped, and the name it was loaded from falls free for the linker. So a build
that failed this way is retried once with the old binary moved aside, and a moved copy is deleted
by the next build that finds nobody holding it -- one of them per live editor window, inside
`target/`, which is git-ignored and `disk.py`'s to clean.

Nothing is moved ahead of time. Cargo relinks whenever its output is missing, so moving the binary
aside before every build would add a link step to every no-op one, and a no-op release build is
what most acceptance sweeps do. POSIX needs none of this, since unlinking a running binary is
ordinary there; the code is not guarded by platform regardless, because a failure it does not
recognise falls through untouched.
"""

from __future__ import annotations

import os
from pathlib import Path

#: What cargo prints when the old executable could not be removed. Everything after it is the
#: operating system's own wording and is translated on a localised Windows, so this prefix is the
#: only part worth matching.
REMOVE_FAILED = "failed to remove file"

#: The suffix a moved-aside binary carries.
ASIDE = ".held-"


def release_cli(root: Path) -> Path:
    """`target/release/nvs`, with this platform's extension."""
    return root / "target" / "release" / ("nvs.exe" if os.name == "nt" else "nvs")


def held(stderr: str, exe: Path) -> bool:
    """Whether `stderr` is cargo failing to replace `exe` because something is still running it."""
    return REMOVE_FAILED in stderr and exe.name in stderr


def free(exe: Path) -> Path | None:
    """Move `exe` aside so a linker can write its path, and say where it went.

    `None` when there was nothing to move or the move itself was refused, and then the caller has
    the build failure it already had rather than a second kind of one.
    """
    for n in range(64):
        aside = exe.with_name(f"{exe.name}{ASIDE}{n}")
        if aside.exists():
            continue
        try:
            exe.rename(aside)
        except OSError:
            return None
        return aside
    return None


def sweep(exe: Path) -> int:
    """Delete every copy moved aside whose holder has since exited, and count the ones that went.

    One that is still held stays where it is: this runs after a build rather than as a cleanup of
    its own, so the next build sweeps what this one could not.
    """
    gone = 0
    for aside in exe.parent.glob(f"{exe.name}{ASIDE}*"):
        try:
            aside.unlink()
        except OSError:
            continue
        gone += 1
    return gone
