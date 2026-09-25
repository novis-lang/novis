#!/usr/bin/env python3
"""Run one of this directory's scripts, and write down what it read.

    python tools/observe.py --out <file> -- tools/<script>.py <args>

`tools/loop.py` remembers a green check against a hash of what the check can read. For a Python
gate that was the whole tree, the handoff included, because a script opens what it likes: two
hundred checks went stale at every wrap, whatever the session had touched. This runs the script
in-process under an audit hook and leaves what it actually did in `--out`, as JSON:

- `files` -- every file under the repository it opened, or asked the size or the time of;
- `dirs` -- every directory under the repository it listed, which is how a glob or a walk shows;
- `spawns` -- the argument list of every process it started;
- `exit` -- its exit status, which is this process's own as well.

`loop.py`'s `observed_inputs` is the reader and says what each of those becomes in a key. The
script's own source and the source of every module it imported from the repository are added to
`files` when it ends, because a module loaded from cached bytecode is never opened as source.

**What this cannot see**, and what is done about each. A test for whether a path exists opens
nothing and is not audited, so the reader keys every observed check on the set of paths in the
tree as well: a file that appears or goes away moves every such key. `os.stat` and `os.lstat`
are replaced for the run so that a size or a time read without an open counts as a read of that
file. The size a directory listing hands back with each entry is not seen, and no script here
uses one. A process the script starts reads what it likes, so the reader treats any start as
"reads everything" unless it can say what that program reads.
"""

from __future__ import annotations

import argparse
import json
import os
import runpy
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
_ROOT = os.path.normcase(str(ROOT))


def inside(path):
    """`path` as the repository names it, or None when it is not under the root."""
    try:
        full = os.path.normcase(os.path.abspath(os.fsdecode(path)))
    except (TypeError, ValueError):
        return None
    if full == _ROOT:
        return "."
    if not full.startswith(_ROOT + os.sep):
        return None
    rel = os.path.abspath(os.fsdecode(path))[len(_ROOT) + 1:].replace("\\", "/")
    return None if "__pycache__" in rel else rel


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True, help="where the JSON record is written")
    ap.add_argument("script", help="the script to run, relative to the repository root")
    ap.add_argument("args", nargs=argparse.REMAINDER)
    opts = ap.parse_args()

    files, dirs, spawns = set(), set(), []

    def hook(event, args):
        if event == "open" and args and isinstance(args[0], (str, bytes, os.PathLike)):
            rel = inside(args[0])
            if rel:
                files.add(rel)
        elif event in ("os.listdir", "os.scandir") and args:
            rel = inside(args[0] if args[0] is not None else ".")
            if rel:
                dirs.add(rel)
        elif event == "subprocess.Popen" and len(args) > 1:
            argv = args[1]
            spawns.append([str(a) for a in argv] if isinstance(argv, (list, tuple))
                          else [str(argv)])
        elif event in ("os.system", "os.exec", "os.spawn", "os.posix_spawn", "os.startfile"):
            spawns.append([event, *[str(a) for a in args[:2]]])

    real_stat, real_lstat = os.stat, os.lstat

    def seen(path):
        if isinstance(path, (str, bytes, os.PathLike)):
            rel = inside(path)
            if rel and rel != ".":
                files.add(rel)

    def stat(path, *a, **kw):
        seen(path)
        return real_stat(path, *a, **kw)

    def lstat(path, *a, **kw):
        seen(path)
        return real_lstat(path, *a, **kw)

    os.stat, os.lstat = stat, lstat
    sys.addaudithook(hook)

    script = str(ROOT / opts.script)
    sys.argv = [script, *[a for a in opts.args if a != "--"]]
    sys.path.insert(0, os.path.dirname(script))
    code = 0
    try:
        runpy.run_path(script, run_name="__main__")
    except SystemExit as exc:
        code = exc.code if isinstance(exc.code, int) else (0 if exc.code is None else 1)
        if exc.code is not None and not isinstance(exc.code, int):
            print(exc.code, file=sys.stderr)
    except BaseException:
        code = 1
        raise
    finally:
        os.stat, os.lstat = real_stat, real_lstat
        # A module imported from its cached bytecode is never opened as source, and the importer
        # asks its time through a `stat` bound before this file ran. Its source decides what the
        # script does all the same.
        files.add(opts.script.replace("\\", "/"))
        for module in list(sys.modules.values()):
            rel = inside(getattr(module, "__file__", None) or "")
            if rel and rel != ".":
                files.add(rel)
        record = {"files": sorted(files), "dirs": sorted(dirs), "spawns": spawns, "exit": code}
        out = Path(opts.out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(record), encoding="utf-8", newline="\n")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
