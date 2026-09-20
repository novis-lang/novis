#!/usr/bin/env python3
"""Which CI lanes a push or a pull request needs, from the paths its diff touched.

`.github/workflows/ci.yml`'s `changes` job runs this, and every other job in that file gates on one
of the booleans it emits. The `LANES` table below is that policy's only home -- `rule:testing/ci-lanes` is the
reasoning, and the workflow holds no `paths:` filter of its own.

  python tools/ci-changes.py --base HEAD~1   # what CI would run for the last commit
  python tools/ci-changes.py --base <sha>    # ... for any range ending at HEAD
  python tools/ci-changes.py --full          # every lane, as the nightly and the release run it
  python tools/ci-changes.py --files -       # lanes for a path list on stdin, touching no git
  python tools/ci-changes.py --help          # this text

Inside Actions it takes `BASE` and `FULL` from the environment and appends `name=value` lines to
`$GITHUB_OUTPUT`; run by hand it prints the same lines to stdout, so the two cannot disagree.

**An unknown base turns every lane on.** A first push to a branch, a force push and a shallow clone
all leave this script without a commit it can diff against, and the failure mode of guessing the
other way is a commit that reached `main` with nothing having checked it.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import impact  # noqa: E402  -- same directory; the package graph has one reader and it is there

# One entry per gate a job in ci.yml spells as `needs.changes.outputs.<name> == 'true'`. A prefix
# ending in `/` matches everything beneath it; anything else matches that one path exactly.
#
# `.github/workflows/` is in every lane on purpose: a run that edits the workflows is the one run
# where the lanes themselves are what needs proving.
LANES = {
    # Any input to a cargo build. Gates the Linux build/test/conformance leg and `lint`.
    "code": (
        "crates/", "benches/", "tests/", "examples/", "fuzz/", "tools/",
        "Cargo.toml", "Cargo.lock", "rustfmt.toml", "rust-toolchain.toml", "deny.toml",
        ".github/workflows/",
    ),
    # The crates that emit or execute machine code, plus the probes that measure them. These are
    # where a use-after-free lives, and they are the only inputs a cost baseline has, so this gates
    # ASAN and the release-profile guard run. It never narrows the platform matrix: every platform
    # runs whenever any code changes, because which host a contributor pushes from is unknowable.
    "native": (
        "crates/nvs-runtime/", "crates/nvs-codegen/", "crates/nvs-stdlib/", "crates/nvs-host/",
        "benches/abi-probe/", "Cargo.lock", "rust-toolchain.toml",
        ".github/workflows/",
    ),
    # Where `wasmtime` enters the tree, and so the only thing `extension-sandbox` can observe.
    "probe": ("benches/abi-probe/", ".github/workflows/"),
    # The VS Code client, and every crate with it: the protocol suite spawns the real `nvs lsp` and
    # the host suite points a throwaway profile at the binary the same run built
    # (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`), so a change anywhere in
    # the workspace can change what either of them answers. Gates `extension` and `extension-host`.
    "editor": (
        "editors/", "crates/", "Cargo.toml", "Cargo.lock",
        ".github/workflows/",
    ),
    # What `cargo deny` and the attribution diff read. Their answer cannot differ in a run that
    # changed none of these (ADR 0068 § Verification).
    "deps": (
        "Cargo.toml", "Cargo.lock", "deny.toml", "THIRD-PARTY-LICENSES.txt",
        "tools/gen-attribution.py", ".github/workflows/",
    ),
    # docs/novis.md is generated from the binary's registry and the chapters, so either side moving
    # can make the committed file stale.
    "refdoc": (
        "crates/", "docs/reference/", "docs/novis.md", "tools/reference.py",
        ".github/workflows/",
    ),
    # The five-driver matrix against real servers: the harness that points each driver at a
    # container, the compose file those containers come from, and -- added by `db_lane` below --
    # every package `tools/db-matrix.py` runs the tests of and every package those are compiled
    # against. Gates `database`, the one job that needs a daemon.
    "db": (
        "tests/db/", "tools/db-matrix.py", "Cargo.toml", "Cargo.lock",
        ".github/workflows/",
    ),
}


def db_lane(base):
    """`LANES["db"]` with the packages the matrix builds. They are read rather than listed: the
    list this replaced named four crates and not `nvs-runtime` or `nvs-host`, which `nvs-db` is
    compiled against, so an edit to either ran no database leg. The workspace packages
    `tools/db-matrix.py` names are what it runs; `impact.closure` is what those are compiled
    against. With no readable graph the lane is every crate, which is the wide direction."""
    graph = impact.manifest_graph()
    names = impact.named_in(graph, "tools/db-matrix.py")
    if not names:
        return base + ("crates/", "benches/")
    built = set(names)
    for name in names:
        built |= impact.closure(graph, name)
    return base + tuple(sorted(f"{graph[n]['dir']}/" for n in built))


LANES["db"] = db_lane(LANES["db"])

ZERO = "0" * 40


def git(*args):
    """Run a git command, returning its stdout, or None if it failed for any reason."""
    try:
        done = subprocess.run(("git", *args), capture_output=True, text=True)
    except OSError:
        return None
    return done.stdout if done.returncode == 0 else None


def changed_paths(base):
    """The paths `base..HEAD` touched, or None if `base` is not a commit we can reach."""
    if not base or base == ZERO:
        return None
    if git("cat-file", "-e", f"{base}^{{commit}}") is None:
        return None
    out = git("diff", "--name-only", base, "HEAD")
    if out is None:
        return None
    return [line.strip() for line in out.splitlines() if line.strip()]


def matches(path, prefixes):
    return any(path.startswith(p) if p.endswith("/") else path == p for p in prefixes)


def lanes_for(paths):
    """One boolean per lane. `paths` of None means "we do not know", which means all of them."""
    if paths is None:
        return {name: True for name in LANES}
    return {name: any(matches(p, pre) for p in paths) for name, pre in LANES.items()}


def truthy(value):
    return str(value).strip().lower() in ("1", "true", "yes", "on")


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", default=os.environ.get("BASE", ""),
                    help="the commit HEAD is compared against; empty means every lane")
    ap.add_argument("--full", action="store_true", default=truthy(os.environ.get("FULL", "")),
                    help="the deep lane: every gate true, every platform in the matrix")
    ap.add_argument("--files", metavar="FILE",
                    help="read the changed paths from a file (`-` for stdin) instead of from git")
    opts = ap.parse_args()

    if opts.full:
        paths = None
    elif opts.files:
        text = sys.stdin.read() if opts.files == "-" else open(opts.files, encoding="utf-8").read()
        paths = [line.strip() for line in text.splitlines() if line.strip()]
    else:
        paths = changed_paths(opts.base)

    out = lanes_for(paths)
    out["deep"] = opts.full
    lines = [f"{name}={str(value).lower()}" for name, value in out.items()]

    sink = os.environ.get("GITHUB_OUTPUT")
    if sink:
        with open(sink, "a", encoding="utf-8") as fh:
            fh.write("\n".join(lines) + "\n")
    for line in lines:
        print(line)


if __name__ == "__main__":
    main()
