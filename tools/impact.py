#!/usr/bin/env python3
"""What a change can reach: one key per test binary, over exactly what that binary reads.

`tools/verify.py` keyed its `test` step on every input file in the tree, so a new case under
`tests/hostile/` and a one-line edit to `nvs-lsp` each ran every test binary in the workspace.
This file is the narrower key: a binary runs again when something *it* reads has moved, and is
answered from the green cache when nothing has.

    python tools/impact.py                     # every test binary: its reach, and why
    python tools/impact.py --explain <path>... # which test binaries a change to each path re-runs
    python tools/impact.py --graph             # the workspace's packages and what each depends on

## What a test binary reads

Three things, and the key is a hash over all of them.

- **What it is compiled from.** Its own package's directory, as bytes: the sources rustc's
  dep-info names are under it, and so is every fixture a test opens beside its own manifest.
  Then every workspace package it depends on -- the normal and build dependencies transitively,
  and the dev-dependencies of the binary's own package -- each at `verify_keys`'s *code* tier,
  without its `tests/` and `benches/` directories, because that is what rustc is handed from a
  dependency. The toolchain, every manifest and lock file, and the files a `build.rs` reads or a
  source embeds are in every key. `cargo metadata` is the graph; nothing here names a crate.
- **What it opens while it runs, outside its own package.** A binary that says so through
  `nvs_repo::path` leaves one line per path in the file `READS_ENV` names, and `recorded` is the
  last such list. Each entry is hashed as bytes: a file, or every file beneath a directory.
- **Nothing else, if it can be shown.** `escapes` reads the binary's own sources -- the list
  rustc's dep-info gives -- for any way out of the package that is not `nvs_repo`, and
  `way_out` is the list of them. A binary with one is **wide**: it keeps the whole-tree key it
  had before this file existed. Comments are not read, since a comment opens no file, and a
  path handed to `include_str!` or `#[path]` is rustc's to open and is in the dep-info already.

So the direction of every doubt is the wide one. A binary whose dep-info cannot be found is wide.
A package `cargo metadata` does not name is wide. A recorded list is trusted only while the
binary's compiled key is the one it was recorded under: a read-set can change only when the code
that does the reading changes, and that code changing runs the binary and records it again.

What the key does not see is what `verify_keys` already says it does not: a line number in a
dependency, and the state of a service a test talks to.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

import verify_keys as keys

ROOT = keys.ROOT
TMP = keys.TMP
#: Each binary's recorded run-time reads: `{job name: {"compiled": key, "reads": [rel, ...]}}`.
READS = TMP / "impact-reads.json"
#: The environment variable naming the file a test binary appends its run-time reads to.
READS_ENV = "NVS_READS_LOG"

_RECORDED = re.compile(r"\bnvs_repo::")
_SPAWN = re.compile(r"Command::new\(")
#: `Command::new(env!("CARGO_BIN_EXE_..."))`, as `verify_keys.scan` leaves it: the package's own
#: binary, which is compiled from what the key already holds.
_OWN_BIN = re.compile(r"Command::new\(env!\(\x02\)\)")
_CLIMBS = re.compile(r"\.parent\(\)|\.ancestors\(\)|\.pop\(\)")
#: The code in front of a literal rustc opens itself.
_RUSTC_OPENS = re.compile(r"(?:include_str!\(|include_bytes!\(|include!\(|path=)$")
_SEGMENT = re.compile(r"[\\/]")
_JOINS = re.compile(r"(?:join|push|from|new)\($")
#: A package's directories that no dependant is compiled from.
_OWN_ONLY = ("tests", "benches", "examples")


def metadata():
    """`{package name: {"dir": rel, "deps": {name: kind}}}` for every workspace package, or None
    when cargo cannot say -- and then every binary is wide."""
    try:
        p = subprocess.run(["cargo", "metadata", "--format-version", "1", "--offline", "--no-deps"],
                           cwd=ROOT, capture_output=True, encoding="utf-8", errors="replace")
        if p.returncode != 0:
            return None
        doc = json.loads(p.stdout)
    except (OSError, ValueError):
        return None
    names = {pkg["name"] for pkg in doc["packages"]}
    out = {}
    for pkg in doc["packages"]:
        try:
            rel = Path(pkg["manifest_path"]).parent.resolve().relative_to(ROOT).as_posix()
        except ValueError:
            return None
        deps = {}
        for d in pkg["dependencies"]:
            if d["name"] in names:
                kind = d.get("kind") or "normal"
                # One package named twice keeps the kind that reaches furthest.
                if deps.get(d["name"]) != "normal":
                    deps[d["name"]] = kind
        out[pkg["name"]] = {"dir": rel, "deps": deps}
    return out


def closure(graph, package):
    """Every workspace package `package`'s test binaries are compiled against: its own
    dependencies of every kind, then theirs without the dev ones, which cargo builds for the
    package under test alone."""
    seen, todo = set(), [(package, True)]
    while todo:
        name, own = todo.pop()
        for dep, kind in graph.get(name, {"deps": {}})["deps"].items():
            if (kind != "dev" or own) and dep not in seen and dep != package:
                seen.add(dep)
                todo.append((dep, False))
    return seen


def dependants(graph, package):
    """The packages whose test binaries are compiled against `package`, itself included."""
    return {name for name in graph if name == package or package in closure(graph, name)}


def dep_info(job):
    """The sources rustc compiled this binary from, ROOT-relative, or None when its dep-info is
    missing or unreadable. The file sits beside the executable under the same stem."""
    try:
        text = Path(job["argv"][0]).with_suffix(".d").read_text(encoding="utf-8")
    except (OSError, KeyError, IndexError):
        return None
    head = text.split("\n\n", 1)[0].replace("\\\n", " ")
    _, _, tail = head.partition(": ")
    out = []
    for raw in re.split(r"(?<!\\) ", tail.strip()):
        path = Path(raw.replace("\\ ", " "))
        try:
            full = path if path.is_absolute() else ROOT / path
            out.append(full.resolve().relative_to(ROOT).as_posix())
        except (ValueError, OSError):
            continue  # a registry crate: `Cargo.lock` is what stands for it
    return out or None


def _body(literal):
    """A string literal's text without its prefix, hashes and quotes."""
    start, end = literal.find('"'), literal.rfind('"')
    return literal[start + 1:end] if 0 <= start < end else ""


def _leaves(path):
    """Does this relative path climb above the directory it starts in? An absolute one names
    nothing in the tree, so it does not."""
    if not path or path[0] in "/\\" or path[1:2] == ":":
        return False
    depth = 0
    for part in _SEGMENT.split(path.replace("\\\\", "/")):
        depth += -1 if part == ".." else (0 if part in ("", ".") else 1)
        if depth < 0:
            return True
    return False


def way_out(text):
    """How this source can reach a file outside its package without `nvs_repo`, or "". The ways:

    - it starts a process that is not its package's own binary, which reads what it likes;
    - it reads or sets its own working directory, which a test binary starts in its package;
    - a string literal climbs out of the directory it starts in, or names `target/debug` -- a
      bare `".."` only where it is joined, pushed or sits beside the manifest directory;
    - it reads `CARGO_MANIFEST_DIR` and also holds a `..` segment in any literal, or climbs with
      `.parent()`, `.ancestors()` or `.pop()`.

    Read off `verify_keys.scan`, so a comment is never matched and a literal never splits."""
    code, literals, _, _ = keys.scan(text)
    for m in _SPAWN.finditer(code):
        index = code.count("\x02", 0, m.start())
        own = _OWN_BIN.match(code, m.start()) and index < len(literals)
        if not (own and "CARGO_BIN_EXE_" in literals[index]):
            return "starts a process with `Command::new`"
    # `Command::current_dir(dir)` takes an argument and moves a child, which was judged above.
    if "set_current_dir" in code or "current_dir()" in code:
        return "reads or sets the working directory"
    rustc_opens, starts = set(), []
    for index, m in enumerate(re.finditer("\x02", code)):
        starts.append(m.start())
        if _RUSTC_OPENS.search(code[:m.start()][-20:]):
            rustc_opens.add(index)
    if len(starts) != len(literals):
        return "its literals could not be placed"
    manifest = any("CARGO_MANIFEST_DIR" in lit for lit in literals)
    for index, lit in enumerate(literals):
        body = _body(lit)
        if index in rustc_opens or not body:
            continue
        if "target/debug" in body or "target\\\\debug" in body:
            return f"names the target directory in {lit[:60]}"
        # A bare `".."` is a segment some code compares against far more often than one it
        # joins, so alone it counts only where it is joined, pushed or listed beside the manifest.
        bare = body == ".." and not manifest and not _JOINS.search(code[:starts[index]][-12:])
        if _leaves(body) and not bare:
            return f"the literal {lit[:60]} climbs out of its directory"
        if manifest and ".." in _SEGMENT.split(body.replace("\\\\", "/")):
            return f"`CARGO_MANIFEST_DIR` beside the literal {lit[:60]}"
    if manifest and _CLIMBS.search(code):
        return "`CARGO_MANIFEST_DIR` beside `.parent()`, `.ancestors()` or `.pop()`"
    return ""


def escapes(sources):
    """The first of these sources with a `way_out`, as `file: how`, or "". A source that cannot
    be read counts as one."""
    for rel in sources:
        if not rel.endswith(".rs"):
            continue
        try:
            text = (ROOT / rel).read_text(encoding="utf-8", errors="replace")
        except OSError:
            return f"{rel}: could not be read"
        how = way_out(text)
        if how:
            return f"{rel}: {how}"
    return ""


class Reach:
    """One reading of the tree, and each test binary's key against it."""

    def __init__(self, tree=None):
        self.tree = tree or keys.Tree()
        self.graph = metadata()
        try:
            self.recorded = json.loads(READS.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            self.recorded = {}
        if not isinstance(self.recorded, dict):
            self.recorded = {}
        self._common = None
        self._packages = {}

    def common(self):
        """What every binary's key holds: the toolchain, the manifests, and what a `build.rs`
        reads or a source embeds."""
        if self._common is None:
            t = self.tree
            self._common = ([("rustc", t.toolchain)] + t.part(keys.is_manifest)
                            + t.part(lambda r: r in keys.BUILD_READS or r in t.embedded))
        return self._common

    def _package(self, name, whole):
        """One package's files: all of them as bytes for the package under test, and for a
        dependency what a dependant is compiled from, at the code tier."""
        if (name, whole) not in self._packages:
            t, base = self.tree, self.graph[name]["dir"]
            inside = [rel for rel in sorted(t.raw) if keys.under(base)(rel)]
            if whole:
                got = [(rel, t.digest(rel, "raw")) for rel in inside]
            else:
                skip = keys.under(*(f"{base}/{d}" for d in _OWN_ONLY))
                got = [(rel, t.digest(rel, "code")) for rel in inside if not skip(rel)]
            self._packages[(name, whole)] = got
        return self._packages[(name, whole)]

    def compiled(self, job):
        """`(parts, why)`: what this binary is compiled from, or `(None, why)` when that cannot
        be shown and the binary is wide."""
        owner = job.get("owner")
        if self.graph is None:
            return None, "cargo metadata could not be read"
        if owner not in self.graph:
            return None, f"no workspace package is named {owner!r}"
        sources = dep_info(job)
        if sources is None:
            return None, "its dep-info could not be read"
        t, own = self.tree, self.graph[owner]["dir"]
        parts = list(self.common()) + self._package(owner, whole=True)
        outside = {s for s in sources if not keys.under(own)(s) and s in t.raw}
        parts += [(rel, t.digest(rel, "raw")) for rel in sorted(outside)]
        for dep in sorted(closure(self.graph, owner)):
            parts += self._package(dep, whole=False)
        how = escapes(sources)
        if how:
            return None, f"leaves its package without `nvs_repo` -- {how}"
        return parts, ""

    def key(self, job):
        """`(key, why)`. `why` is empty for a narrow key and says what made a wide one wide."""
        parts, why = self.compiled(job)
        if parts is None:
            return _digest(job["name"], keys.STEP_READS["test"](self.tree)), why
        compiled = _digest(job["name"], parts)
        seen = self.recorded.get(job["name"], {})
        reads = seen.get("reads", []) if seen.get("compiled") == compiled else None
        if reads is None and _RECORDED.search(self._text(job)):
            return _digest(job["name"], keys.STEP_READS["test"](self.tree)), \
                "it reads through `nvs_repo` and no run has recorded what"
        return _digest(job["name"], parts + self.tree.part(keys.under(*reads)) if reads
                       else parts), ""

    def _text(self, job):
        out = []
        for rel in dep_info(job) or []:
            try:
                out.append((ROOT / rel).read_text(encoding="utf-8", errors="replace"))
            except OSError:
                pass
        return "\n".join(out)

    def record(self, job, log):
        """File what a run of `job` appended to `log`, under the compiled key it ran as."""
        parts, _ = self.compiled(job)
        if parts is None:
            return
        try:
            lines = Path(log).read_text(encoding="utf-8").splitlines()
        except OSError:
            lines = []
        reads = sorted({ln.strip().replace("\\", "/").strip("/") for ln in lines if ln.strip()})
        self.recorded[job["name"]] = {"compiled": _digest(job["name"], parts), "reads": reads}

    def save(self):
        try:
            TMP.mkdir(exist_ok=True)
            READS.write_text(json.dumps(self.recorded, indent=1, sort_keys=True),
                             encoding="utf-8", newline="\n")
        except OSError:
            pass


def _digest(name, parts):
    h = hashlib.blake2b(digest_size=16)
    h.update(name.encode("utf-8") + b"\0")
    for label, digest in parts:
        h.update(label.encode("utf-8") + b"\0" + digest.encode("utf-8", "replace") + b"\0")
    return h.hexdigest()


def last_jobs():
    """The test jobs `verify.py` last built, or an empty list."""
    try:
        built = json.loads((TMP / "verify-test-built.json").read_text(encoding="utf-8"))
        return built.get("jobs") or []
    except (OSError, ValueError, AttributeError):
        return []


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--explain", nargs="+", metavar="PATH",
                    help="which test binaries a change to each path re-runs")
    ap.add_argument("--graph", action="store_true", help="the packages and their dependencies")
    opts = ap.parse_args()
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    reach = Reach()
    if opts.graph:
        if reach.graph is None:
            print("impact: cargo metadata could not be read, so every binary is wide")
            return 1
        for name in sorted(reach.graph):
            node = reach.graph[name]
            deps = " ".join(d + {"dev": "*", "build": "+"}.get(k, "")
                            for d, k in sorted(node["deps"].items()))
            print(f"{name:<18} {node['dir']:<22} {deps}")
            print(f"{'':<18} reached by an edit: "
                  f"{' '.join(sorted(dependants(reach.graph, name) - {name})) or '--'}")
        print("\n`*` a dev-dependency, `+` a build dependency")
        return 0

    jobs = last_jobs()
    if not jobs:
        print("impact: no test build is on record -- `python tools/verify.py` leaves one")
        return 1
    if opts.explain:
        before = {j["name"]: reach.key(j)[0] for j in jobs}
        for raw in opts.explain:
            rel = Path(raw).as_posix().strip("/")
            if rel not in reach.tree.raw:
                print(f"{rel}: not an input of any step -- no test binary reads it")
                continue
            probe = Reach(_Moved(reach.tree, rel))
            probe.graph, probe.recorded = reach.graph, reach.recorded
            hit = [j["name"] for j in jobs if probe.key(j)[0] != before[j["name"]]]
            print(f"{rel}: {len(hit)} of {len(jobs)} test binaries")
            for name in hit:
                print(f"    {name}")
        return 0

    wide = 0
    for j in jobs:
        _, why = reach.key(j)
        wide += bool(why)
        print(f"{'WIDE  ' if why else 'narrow'}  {j['name']:<44} {why}")
    print(f"\n{len(jobs) - wide} of {len(jobs)} test binaries have a narrow key; a wide one runs "
          f"on any change to the tree.")
    return 0


class _Moved:
    """A tree in which one file's digest differs at every tier: `--explain`'s what-if."""

    def __init__(self, tree, rel):
        self._tree, self._rel = tree, rel
        self.raw = tree.raw
        self.toolchain, self.embedded = tree.toolchain, tree.embedded

    def digest(self, rel, tier):
        got = self._tree.digest(rel, tier)
        return got + "!" if rel == self._rel else got

    def part(self, pred, tier="raw"):
        return [(rel, self.digest(rel, tier)) for rel in sorted(self.raw) if pred(rel)]

    def binary(self, tier="code"):
        return keys.Tree.binary(self, tier)


if __name__ == "__main__":
    raise SystemExit(main())
