#!/usr/bin/env python3
"""Cuts the two icons `crates/nvs-cli/build.rs` links into `nvs.exe`.

    python tools/exe-icons.py

`website/media/novis-logo.png` becomes `crates/nvs-cli/assets/nvs.ico`, the
release binary's icon, and `website/media/novis-logo-file-icon-light-theme-nvst.svg`
becomes `nvs-debug.ico`, the debug binary's. Both are committed, so a build
needs neither this script nor ImageMagick; run it after either drawing changes
and commit what it writes. `rule:packaging/the-windows-binary-says-what-it-is`
owns which binary carries which.

It needs ImageMagick's `magick` on `PATH`, the tool `website/README.md`
§ *Logo and favicon* already cuts the favicon with, and for the same two
reasons: each size is trimmed of the export's transparent margin and sharpened
for the size it is, which `-define icon:auto-resize` cannot do.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MEDIA = ROOT / "website" / "media"
ASSETS = ROOT / "crates" / "nvs-cli" / "assets"
SCRATCH = ROOT / ".agent-tmp" / "exe-icons"

ICONS = {
    "nvs.ico": MEDIA / "novis-logo.png",
    "nvs-debug.ico": MEDIA / "novis-logo-file-icon-light-theme-nvst.svg",
}

# The sizes Explorer, the taskbar and Alt-Tab pick from at 100% to 250% scaling,
# each with the unsharp amount that keeps the mark's counters open at that size.
SIZES = {
    16: "0x0.6+0.8+0.02",
    20: "0x0.6+0.7+0.02",
    24: "0x0.6+0.7+0.02",
    32: "0x0.6+0.6+0.02",
    40: "0x0.6+0.5+0.02",
    48: "0x0.6+0.5+0.02",
    64: "0x0.6+0.4+0.02",
    256: None,
}


def magick(*args: str | Path) -> None:
    subprocess.run(["magick", *map(str, args)], check=True)


def cut(source: Path, target: Path) -> None:
    stem = SCRATCH / target.stem
    mark = Path(f"{stem}-mark.png")
    # A drawing is rasterised well above the largest size it is cut to; the
    # density is ignored for the PNG, which is already pixels.
    magick("-background", "none", "-density", "384", source, "-trim", "+repage", mark)
    frames = []
    for size, unsharp in SIZES.items():
        frame = Path(f"{stem}-{size}.png")
        sharpen = ["-unsharp", unsharp] if unsharp else []
        magick(
            mark, "-filter", "Lanczos", "-resize", f"{size}x{size}", *sharpen,
            "-background", "none", "-gravity", "center", "-extent", f"{size}x{size}",
            frame,
        )
        frames.append(frame)
    magick(*frames, target)


def main() -> int:
    if shutil.which("magick") is None:
        print("error: ImageMagick's `magick` is not on PATH", file=sys.stderr)
        return 1
    SCRATCH.mkdir(parents=True, exist_ok=True)
    ASSETS.mkdir(parents=True, exist_ok=True)
    for name, source in ICONS.items():
        cut(source, ASSETS / name)
        print(f"wrote {(ASSETS / name).relative_to(ROOT).as_posix()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
