#!/usr/bin/env python3
"""Whether the latest `ci.yml` run on `main` succeeded, and whether it ran the code `HEAD` holds.

Goal `ci-green`'s acceptance check. `gh run list` alone answers the first half and cannot answer the
second: the newest run is the newest *push*, the loop never pushes, and a green run for a commit a
hundred sessions old proves nothing about the tree the goal is claimed on.

  python tools/ci-green.py          # exit 0 and the sentence below, or exit 1 and what is missing
  python tools/ci-green.py --help   # this text

**The run's commit need not be `HEAD` itself.** The session that sees the green run commits its
handoff, which moves `HEAD` past the commit the run was for, and a check demanding equality could
never be met by the session that has to meet it. So the run's commit must be an ancestor of `HEAD`,
and every path changed since must sit under `BOOKKEEPING` -- the files a session writes to say where
the work stands, which no CI leg builds.
"""

from __future__ import annotations

import json
import subprocess
import sys

#: The line goal `ci-green`'s check reads. One string, here.
GREEN = "the latest ci.yml run on main succeeded, and it ran the code HEAD holds"

# What a session writes after the work is done. A prefix ending in `/` matches everything beneath it.
BOOKKEEPING = ("docs/agent/", "docs/plan/", "docs/implementation-plan.md")


def out(*argv: str) -> tuple[int, str]:
    done = subprocess.run(argv, capture_output=True, text=True, encoding="utf-8")
    return done.returncode, (done.stdout or "").strip() or (done.stderr or "").strip()


def main(argv: list[str]) -> int:
    if any(a in ("-h", "--help") for a in argv):
        print(__doc__)
        return 0
    code, text = out("gh", "run", "list", "--branch", "main", "--workflow", "ci.yml", "--limit", "1",
                     "--json", "conclusion,status,headSha,url")
    if code:
        print(f"ci-green: `gh run list` failed -- {text}")
        return 1
    runs = json.loads(text or "[]")
    if not runs:
        print("ci-green: no ci.yml run on main exists")
        return 1
    run = runs[0]
    sha = run["headSha"]
    if run["status"] != "completed":
        print(f"ci-green: the latest run is still {run['status']} -- {run['url']}")
        return 1
    if run["conclusion"] != "success":
        print(f"ci-green: the latest run ended `{run['conclusion']}` for {sha[:9]} -- {run['url']}")
        return 1

    _, head = out("git", "rev-parse", "HEAD")
    if out("git", "merge-base", "--is-ancestor", sha, "HEAD")[0]:
        print(f"ci-green: the green run is for {sha[:9]}, which is not an ancestor of HEAD {head[:9]}")
        return 1
    _, changed = out("git", "diff", "--name-only", sha, "HEAD")
    unproven = [p for p in changed.splitlines()
                if not any(p == b or (b.endswith("/") and p.startswith(b)) for b in BOOKKEEPING)]
    if unproven:
        print(f"ci-green: the green run is for {sha[:9]}, and {len(unproven)} path(s) it never saw "
              f"have changed since -- push {head[:9]} and wait for its run:")
        for path in unproven[:20]:
            print(f"       {path}")
        if len(unproven) > 20:
            print(f"       ... and {len(unproven) - 20} more")
        return 1
    print(f"ci-green: {GREEN} ({sha[:9]})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
