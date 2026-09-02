#!/usr/bin/env python3
"""Every feature Novis ships owes four proofs. This says which are missing, runs the ones on disk,
and writes the loop goals that produce the rest.

`gaps.py` answers *which claim the corpus does not ask*. `holes.py` answers *which shape the language
does not have*. This answers the third question, and the three never overlap: **what does each
shipped feature still owe?** The four proofs, one row each in every table this prints:

| Proof | Lives in | What it is |
|---|---|---|
| `tests` | `tests/conformance/`, `tests/differential/`, `crates/**/src/**.rs` | the behaviour pinned from Novis *and* from Rust, every relevant path |
| `examples` | `docs/examples/` | small, self-contained, plainly commented programs a reader learns from -- synced to the website |
| `perf` | `benches/members/` + `docs/perf/members.ndjson` | one measured figure per feature, so a change can be re-measured against **us**, never against PHP |
| `hostile` | `tests/hostile/` | the file written to break it -- panic, leak, unbounded growth, whatever an attacker would reach for |

The roster is **derived, never listed**. `nvs meta --json` names every registered class, member,
exception, enum, interface and `nvs.toml` directive; `docs/reference/lang/*.md` and
`docs/reference/tools/*.md` name every language and tool feature by their own `#` headings. A
feature that ships appears here on the sweep after it lands, and one that is deleted stops being
owed, with nobody editing a list.

    python tools/dossier.py                         the audit: every group, four columns, thinnest first
    python tools/dossier.py --group 'Core\\Str'       one group, feature by feature
    python tools/dossier.py --id 'Core\\Str::length'  one feature: what it has, what it owes, where each goes
    python tools/dossier.py --owed                   only what is missing, as a worklist
    python tools/dossier.py --json                   the same, for a tool

    python tools/dossier.py --gate [--group G]       exit 0 when nothing in scope is owed. Judges, never measures
    python tools/dossier.py --run examples           run every example, diff against its `.out`
    python tools/dossier.py --run hostile            run every attack; survive it or fail naming the file
    python tools/dossier.py --record-perf [--group G]  measure and append to the ledger
    python tools/dossier.py --perf-report            regenerate docs/perf/members.md from the ledger

    python tools/dossier.py --no-perf …              drop the perf proof entirely, for any command above
    python tools/dossier.py --emit-goals             write the whole loop chain under docs/agent/goals/dossier/

## Turning the perf proof off

`--no-perf` on any command drops it from what is owed, from the audit's columns, and from every goal
`--emit-goals` writes (the emitted gate carries the flag, so the loop stays consistent with the
sweep you ran). `NVS_DOSSIER_NO_PERF=1` does the same for a whole shell, and `[all] perf = false` in
`tools/data/dossier-policy.toml` does it durably for the repository. Nothing is deleted by any of
them: the ledger, the bench programs and `--record-perf` all keep working, so turning it back on
picks up where it stopped rather than starting again.

## The second sweep is a fraction of the first

Once a feature is complete, keeping it complete is cheap — deliberately, because a check nobody can
afford to run is a check nobody runs:

* **The audit and the gate never execute anything.** One `nvs meta --json`, one walk of the four
  trees, one `git log -1` per implementing file. Seconds, whatever the roster's size.
* **Perf is re-measured only where the implementation moved.** The `impl_commit` currency rule
  above is the whole mechanism: an untouched member is never re-timed, so a sweep after a change to
  one crate measures that crate. `--record-perf` measures only what has no current figure unless
  `--force`, so it is safe to run at the end of every slice.
* **A figure from any machine satisfies the gate.** A fresh clone on a new box owes nothing it
  already has a current record for -- the ledger travels with the repository, and re-taking a
  number to learn what the last machine already recorded proves nothing about the language. Only
  `--perf-report` insists on this machine's own records, because only a delta needs them.
* **`--run` remembers a green verdict against the bytes that produced it** — the program's own hash
  and the binary's — in `.loop/dossier-green.json`. Identical bytes into a deterministic run cannot
  reach a different verdict, which is the argument `verify.py` and `loop.py` both already make for
  their own caches. So a re-run with an unchanged binary costs the walk; a re-run after a rebuild
  costs the programs. `--no-cache` forces the long way, and a failure is never cached.

## How a proof is attributed to a feature

**An example, an attack and a bench are attributed by where they sit**: every one of the four trees
uses the same relative path for a feature, so `docs/examples/core/Str/length/`,
`tests/hostile/core/Str/length/` and `benches/members/core/Str/length.nvs` need no registration and
no marker. Put the file there and it counts; `--id` prints the three paths.

**A test is attributed by a marker**, because a case lives where its suite wants it and one case
often pins several features:

    // covers: Core\\Str::length, lang:expressions/precedence-and-associativity

It goes in the `--FILE--` block of a `.nvst` case, or above a Rust `#[test]`, where it attributes to
the `fn` beneath it. For a `Core` member the scan additionally credits a case that plainly calls it
(`Core\\Str::length(`), so the 1,678 cases that existed before this file counts without being
rewritten -- `--id` says which of the two found each one. Nothing else is inferred.

## When a proof fails

It has found something, and there are two honest answers: **fix it**, or **record it**. Recording is
a `# Known gaps` entry in the owning crate's module doc -- this repository's existing home for
exactly this fact -- plus a marker on the proof that found it:

    // dossier: known-gap crates/nvs-stdlib/src/str.rs -- one sentence saying what breaks

The sweep then counts that file as `known-gap` rather than a failure, so an unattended run continues
past a bug too large for the slice that found it, and `--gaps` keeps the list in front of anyone who
asks. Two things stop this from becoming a way to make anything green: the marker must name a file
that really carries a `# Known gaps` section, and **a marked proof that passes fails the sweep** --
so removing the marker is part of whatever fix eventually lands. `--run … --strict` fails on them
outright, which is what a person runs to see the real debt.

**Weakening the proof is not one of the two answers** -- not softening an attack until it survives,
not re-blessing an example to whatever the binary now prints, not `[skip]`ping the feature. Those
turn a finding into a green check, which is the one outcome this file exists to prevent.

## What is owed, per kind

A directive is not benchmarked and an interface is not attacked -- the four proofs apply where they
mean something, and the table is data, not a rule in prose: `POLICY` below, overridable per feature
in `tools/data/dossier-policy.toml` (`[skip]` for a proof that cannot exist, with the reason). A
skip carries its reason into the audit, so "this cannot be benchmarked" and "nobody wrote one" never
look the same.

## Why perf is gated on the implementation's commit rather than re-measured

Measurement needs an idle machine and the acceptance test runs after every session, so a sweep that
re-times 500 features per session would measure the driver's own build more than the language. Every
record therefore carries `impl_commit` -- the last commit that touched the file implementing that
feature -- and the gate passes while that value still matches. Change the implementation and its
figure goes stale on the spot; change something else and nothing is re-measured.

**Wall clock is not comparable across machines**, which is [ADR 0026](../docs/adr/0026-performance-measurement-methodology.md)'s
whole finding, so every record carries a machine fingerprint and a `ratio` against a calibration
program measured in the same sweep. Same fingerprint, same commit: the nanoseconds are the honest
number. Different machines: only the ratio travels, and only to about a tenth. The tool refuses to
diff across fingerprints rather than quietly printing a delta that means nothing.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))

import machine  # noqa: E402  (tools/ is not a package; this is how every tool here imports a sibling)

BS = chr(92)  # a literal backslash, spelled so no layer of quoting can eat it

EXAMPLES = ROOT / "docs" / "examples"
HOSTILE = ROOT / "tests" / "hostile"
BENCHES = ROOT / "benches" / "members"
LEDGER = ROOT / "docs" / "perf" / "members.ndjson"
PERF_REPORT = ROOT / "docs" / "perf" / "members.md"
CONFORMANCE = ROOT / "tests" / "conformance"
DIFFERENTIAL = ROOT / "tests" / "differential"
CRATES = ROOT / "crates"
LANG = ROOT / "docs" / "reference" / "lang"
TOOLCHAPTERS = ROOT / "docs" / "reference" / "tools"
POLICY_FILE = TOOLS / "data" / "dossier-policy.toml"
GOALS_OUT = ROOT / "docs" / "agent" / "goals" / "dossier"
CALIBRATION = BENCHES / "_calibration"
#: Green verdicts from `--run`, keyed on the bytes that produced them. Under `.loop/` with every
#: other run-time artefact, and gitignored with it.
GREEN = ROOT / ".loop" / "dossier-green.json"

#: What each kind of feature owes. `tests` counts proofs from either side -- a `.nvst` case or a
#: Rust `#[test]` -- and `rust` is how many of them must be the Rust half; `examples` and `hostile`
#: are file counts; `perf` is a bench program plus a current ledger record.
POLICY = {
    "member":    {"tests": 2, "rust": 1, "examples": 3, "perf": True,  "hostile": 1},
    "lang":      {"tests": 2, "rust": 0, "examples": 3, "perf": True,  "hostile": 1},
    "exception": {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 1},
    "enum":      {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 0},
    "interface": {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 0},
    "tool":      {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 1},
    "directive": {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 1},
}

PROOFS = ("tests", "examples", "perf", "hostile")

#: `// covers: A, B` -- in a `.nvst`, a `.nvs`, or above a Rust `#[test]`. `#` is accepted so the
#: marker can sit in a TOML or a shell fixture too.
COVERS_RE = re.compile(r"(?://|#)\s*covers:\s*(.+)")
#: `// requires: unimplemented` -- the website's own skip marker, honoured unchanged.
UNIMPL_RE = re.compile(r"^(?://|#)\s*requires:\s*unimplemented", re.M)
#: `// bench: iterations 200000` inside a bench program.
ITER_RE = re.compile(r"(?://|#)\s*bench:\s*iterations\s+([0-9_]+)")
#: `// hostile: timeout-ms 4000`
TIMEOUT_RE = re.compile(r"(?://|#)\s*hostile:\s*timeout-ms\s+([0-9]+)")
#: `// hostile: expect-refusal` -- this attack's whole point is that the compiler says no.
REFUSAL_EXPECTED_RE = re.compile(r"(?://|#)\s*hostile:\s*expect-refusal")
#: `// dossier: known-gap crates/nvs-stdlib/src/str.rs -- one sentence`. A proof that found a real
#: bug too large for the slice that found it. The path names the module doc whose `# Known gaps`
#: section carries the entry, which is where this repository already keeps exactly this fact.
KNOWN_GAP_RE = re.compile(r"(?://|#)\s*dossier:\s*known-gap\s+(\S+)\s*(.*)")
#: A Novis compile diagnostic. A runtime failure does not look like this -- an uncaught throw is a
#: structured log line -- so this distinguishes "the attack was refused before it ran" from "the
#: attack ran and the runtime handled it", which are opposite verdicts.
REFUSED_RE = re.compile(r"^error\[E[0-9]+\]", re.M)

#: What a hostile run must never produce on stderr, whatever else it does. A Novis program is
#: allowed to fail; the runtime under it is not allowed to come apart.
CRASH_MARKERS = (
    "panicked at",
    "internal error",
    "RUST_BACKTRACE",
    "note: run with",
    "double free",
    "Assertion failed",
    "AddressSanitizer",
    "SIGSEGV",
    "stack overflow",
)


# ------------------------------------------------------------------------------ small helpers


#: Every subprocess this file runs, decoded the same way. Not the platform default: `nvs meta
#: --json` carries the em dashes its own reference cards are written with, and cp1252 refuses them
#: -- which arrives as a `NoneType` where the JSON was, several frames from the cause.
CAPTURE = {"capture_output": True, "text": True, "encoding": "utf-8", "errors": "replace"}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def safe(text: str) -> str:
    """Printable on a console that is not UTF-8. A reference card is written with em dashes and a
    Windows console is cp1252; the audit is not the place to lose a run over one."""
    return text.encode(sys.stdout.encoding or "utf-8", "replace").decode(
        sys.stdout.encoding or "utf-8", "replace")


def rel(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def binary(explicit: str | None = None) -> Path | None:
    """The `nvs` this repository just built: release first, debug second."""
    if explicit:
        p = Path(explicit)
        return p if p.exists() else None
    exe = "nvs.exe" if os.name == "nt" else "nvs"
    for profile in ("release", "debug"):
        p = ROOT / "target" / profile / exe
        if p.exists():
            return p
    return None


def git(*args: str) -> str:
    try:
        out = subprocess.run(["git", *args], cwd=ROOT, timeout=30, **CAPTURE)
        return out.stdout.strip() if out.returncode == 0 else ""
    except (OSError, subprocess.SubprocessError):
        return ""


_last_commit: dict[str, str] = {}


def last_commit(path: str) -> str:
    """The commit that last touched `path` -- what a perf record's currency is keyed on."""
    if path not in _last_commit:
        _last_commit[path] = git("log", "-1", "--format=%H", "--", path)[:12]
    return _last_commit[path]


def slugify(text: str) -> str:
    out = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
    return out or "unnamed"


def load_policy(no_perf: bool = False) -> tuple[dict, dict]:
    """`POLICY` with the overrides in `tools/data/dossier-policy.toml` applied, and the skip map.

    The file is optional and every key in it is optional:

        [all]                 # every kind at once -- this is where `perf = false` lives durably
        perf = false

        [member]              # raise or lower what one kind owes
        examples = 4

        [skip."Core\\Process::exit"]
        perf = "the program is gone before a second iteration"

    `--no-perf` and `NVS_DOSSIER_NO_PERF=1` are the same switch for one command and one shell. All
    three only stop the proof from being *owed*: the ledger, the benches and `--record-perf` are
    untouched, so turning it back on resumes rather than restarts.
    """
    policy = {k: dict(v) for k, v in POLICY.items()}
    skips: dict[str, dict[str, str]] = {}
    if POLICY_FILE.exists():
        doc = tomllib.loads(read(POLICY_FILE))
        for kind, fields in doc.items():
            if kind == "skip":
                for fid, reasons in fields.items():
                    skips[fid] = dict(reasons)
            elif kind == "all":
                for k in policy:
                    policy[k].update(fields)
            elif kind in policy:
                policy[kind].update(fields)
    if no_perf or os.environ.get("NVS_DOSSIER_NO_PERF", "") not in ("", "0"):
        for k in policy:
            policy[k]["perf"] = False
    return policy, skips


def shown_proofs(policy: dict) -> tuple[str, ...]:
    """The proofs any kind still owes -- the audit's columns, so a switched-off proof leaves no
    column reading as complete when nothing was ever asked of it."""
    return tuple(p for p in PROOFS if p != "perf" or any(k["perf"] for k in policy.values()))


# ------------------------------------------------------------------------------- the roster


@dataclass
class Entry:
    """One shipped feature, and where each of its four proofs belongs."""

    id: str
    kind: str
    group: str
    path: str                       # the shared relative path all four trees use
    anchor: str = ""                # `crates/…/file.rs:NN`, when the roster knows one
    twin: list[str] = field(default_factory=list)   # PHP built-ins it replaces, when known
    summary: str = ""

    @property
    def examples_dir(self) -> Path:
        return EXAMPLES / self.path

    @property
    def hostile_dir(self) -> Path:
        return HOSTILE / self.path

    @property
    def bench_file(self) -> Path:
        return BENCHES / (self.path + ".nvs")

    @property
    def impl_file(self) -> str:
        return self.anchor.split(":")[0] if self.anchor else ""


def meta_json(nvs: Path) -> dict:
    out = subprocess.run([str(nvs), "meta", "--json"], timeout=120, **CAPTURE)
    if out.returncode != 0:
        raise SystemExit(f"dossier: `{rel(nvs)} meta --json` failed:\n{out.stderr.strip()}")
    return json.loads(out.stdout)


def registry_anchors() -> dict[tuple[str, str], str]:
    """(class, member) -> `crates/…/file.rs:NN`, borrowed from `gaps.py`'s registry reader.

    Anchors are a convenience -- they seed a goal's `[context]` and key the perf ledger's currency.
    A roster that cannot resolve one is still a roster, so this never raises.
    """
    try:
        import gaps  # noqa: PLC0415  (optional: the roster stands without it)

        return {k: f"{rel(v[0])}:{v[1]}" for k, v in gaps.registry().items()}
    except Exception:
        return {}


def php_twins() -> dict[tuple[str, str], list[str]]:
    """(class, member) -> the PHP built-ins the spec's *Replaces* column names."""
    try:
        import gaps  # noqa: PLC0415

        out: dict[tuple[str, str], list[str]] = {}
        for owners, member, replaces, _q in gaps.spec_rows():
            for owner in owners:
                names = gaps.php_twins(replaces)
                if names:
                    out[(owner, member)] = names
        return out
    except Exception:
        return {}


def class_tail(name: str) -> str:
    """`Core\\Db\\Row` -> `Db-Row`, the spelling every tree and the website already use."""
    tail = name[5:] if name.startswith("Core" + BS) else name
    return tail.replace(BS, "-")


def chapter_features(directory: Path, area: str) -> list[Entry]:
    """Every `#` heading in a reference chapter is one feature of that chapter's topic.

    The chapters are the hand-written half of `docs/novis.md` and each `#` inside one is exactly a
    thing the language or a tool *has* -- which makes them a live roster rather than a list somebody
    maintains. The frontmatter's `id` is the chapter, so a heading moving between chapters renames
    its feature and its four trees move with it.
    """
    out: list[Entry] = []
    for path in sorted(directory.glob("*.md")):
        text = read(path)
        m = re.search(r"^---\n(.*?)\n---\n", text, re.S)
        front = m.group(1) if m else ""
        cid = re.search(r"^id:\s*(\S+)", front, re.M)
        chapter = cid.group(1) if cid else path.stem
        body = text[m.end():] if m else text
        line_no = text[: m.end()].count("\n") + 1 if m else 0
        for line in body.split("\n"):
            line_no += 1
            if line.startswith("# "):
                title = line[2:].strip()
                slug = slugify(title)
                out.append(Entry(
                    id=f"{area}:{chapter}/{slug}",
                    kind="lang" if area == "lang" else "tool",
                    group=f"{area}:{chapter}",
                    path=f"{area}/{chapter}/{slug}",
                    anchor=f"{rel(path)}:{line_no}",
                    summary=title,
                ))
    return out


def roster(nvs: Path) -> list[Entry]:
    """Every feature Novis ships, from the four live sources. Nothing here is a list."""
    doc = meta_json(nvs)
    anchors = registry_anchors()
    twins = php_twins()
    out: list[Entry] = []

    for klass in doc.get("classes", []):
        cname = klass["name"]
        tail = class_tail(cname)
        for member in klass.get("members", []):
            mname = member["name"]
            out.append(Entry(
                id=f"{cname}::{mname}",
                kind="member",
                group=cname,
                path=f"core/{tail}/{mname}",
                anchor=anchors.get((cname, mname), ""),
                twin=twins.get((cname, mname), []),
                summary=(member.get("doc") or {}).get("short", "") or member.get("signature", ""),
            ))

    for kind, key in (("exception", "exceptions"), ("enum", "enums"), ("interface", "interfaces")):
        for item in doc.get(key, []):
            name = item["name"] if isinstance(item, dict) else str(item)
            out.append(Entry(
                id=name,
                kind=kind,
                group=f"types:{kind}",
                path=f"types/{class_tail(name)}",
                summary=((item.get("doc") or {}).get("short", "") if isinstance(item, dict) else ""),
            ))

    for d in doc.get("directives", []):
        key = d["key"] if isinstance(d, dict) else str(d)
        out.append(Entry(
            id=f"directive:{key}",
            kind="directive",
            group="config:directives",
            path=f"config/{key.replace('.', '-')}",
            summary=(f"{d.get('class', '')} directive, applied at {d.get('apply', '')}"
                     if isinstance(d, dict) else ""),
        ))

    out += chapter_features(LANG, "lang")
    out += chapter_features(TOOLCHAPTERS, "tools")
    return out


# ------------------------------------------------------------------- what is already on disk


def covers_in(text: str) -> set[str]:
    ids: set[str] = set()
    for m in COVERS_RE.finditer(text):
        for part in m.group(1).split(","):
            name = part.strip().strip("`\"'")
            if name:
                ids.add(name)
    return ids


@dataclass
class Proofs:
    """What one feature actually has, and how each proof was attributed."""

    nvst: list[str] = field(default_factory=list)
    rust: list[str] = field(default_factory=list)
    examples: list[str] = field(default_factory=list)
    hostile: list[str] = field(default_factory=list)
    bench: str = ""
    perf: dict | None = None        # the newest record taken on THIS machine, for the report
    perf_any: dict | None = None    # the newest current record from ANY machine, for the gate
    inferred: int = 0               # tests credited by a call rather than by a marker
    gaps: list[str] = field(default_factory=list)   # proofs that found a bug nobody has fixed yet


def scan_markers() -> dict[str, list[str]]:
    """Every `covers:` marker on the tree -> the files carrying it.

    One walk over the two case trees and `crates/`, because a per-feature search over 750 features
    is 750 walks of the same directories. The example, attack and bench trees are deliberately not
    walked: those three are attributed by path, so a marker in one would be a second mechanism
    answering a question the directory already answers.
    """
    found: dict[str, list[str]] = {}
    roots = [
        (CONFORMANCE, ("*.nvst",)),
        (DIFFERENTIAL, ("*.nvst",)),
        (CRATES, ("*.rs",)),
    ]
    for base, globs in roots:
        if not base.is_dir():
            continue
        for pattern in globs:
            for path in base.rglob(pattern):
                if "target" in path.parts:
                    continue
                text = read(path)
                if "covers:" not in text:
                    continue
                is_rust = path.suffix == ".rs"
                for m in COVERS_RE.finditer(text):
                    label = rel(path)
                    if is_rust:
                        # The marker sits above the `#[test] fn` it belongs to, so the report can
                        # name the test rather than the 4,000-line file it lives in.
                        fn = re.search(r"\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)", text[m.end():m.end() + 400])
                        if fn:
                            label = f"{label}::{fn.group(1)}"
                    for part in m.group(1).split(","):
                        name = part.strip().strip("`\"'")
                        if name:
                            found.setdefault(name, []).append(label)
    return found


def scan_calls() -> dict[str, list[str]]:
    """Which conformance/differential cases plainly call which member.

    The advisory half of attribution, and the reason the 1,678 cases that existed before this file
    count without being rewritten: `Core\\Str::length(` names its member outright, and 123 cases
    call that one.

    **Only the written `::` spelling is credited.** The obvious extension -- credit `->length(` to
    every `Core` class the case happens to name -- is unsound rather than merely loose: a case
    naming three classes and calling one reader would credit the reader to all three, and no amount
    of tightening fixes it without a type checker. An instance member is attributed by its
    `covers:` marker instead, which is a one-line edit to a case that already exists and is the
    whole reason the marker is the authority and this is the fallback.
    """
    static_re = re.compile(r"(?:Core" + re.escape(BS) + r")?([A-Za-z_][A-Za-z0-9_" + re.escape(BS)
                           + r"]*)::([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
    out: dict[str, list[str]] = {}
    for base in (CONFORMANCE, DIFFERENTIAL):
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.nvst")):
            text = read(path)
            name = rel(path)
            for owner, member in static_re.findall(text):
                tail = owner.split(BS)[-1]
                out.setdefault(f"static:{tail}::{member}", []).append(name)
    return out


def is_rust(label: str) -> bool:
    return ".rs" in label


def ledger_records() -> dict[str, list[dict]]:
    """Every perf record, grouped by feature, oldest first. An append-only ledger is the history;
    this is it, and the callers pick what they need out of each list.

    **The gate reads records from any machine and the report reads only this one's**, and that split
    is the whole point. A figure taken on a colleague's Linux box at the same `impl_commit` is a
    measurement of the same code: the feature is documented, and re-taking it here would prove
    nothing about the language. But a *delta* between the two boxes is meaningless, so
    `--perf-report` never crosses a fingerprint. Without this split a fresh clone owes 759 figures
    it already has, and the first thing anyone would do is turn the proof off.
    """
    if not LEDGER.exists():
        return {}
    out: dict[str, list[dict]] = {}
    for line in read(LEDGER).splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        out.setdefault(rec.get("id", ""), []).append(rec)
    return out


def collect(entries: list[Entry]) -> dict[str, Proofs]:
    markers = scan_markers()
    calls = scan_calls()
    perf = ledger_records()
    me = fingerprint()["id"]
    out: dict[str, Proofs] = {}
    for e in entries:
        p = Proofs()
        marked = markers.get(e.id, [])
        p.nvst = sorted({f for f in marked if f.endswith(".nvst")})
        p.rust = sorted({f for f in marked if is_rust(f)})
        if e.kind == "member":
            owner, member = e.id.split("::", 1)
            inferred = calls.get(f"static:{owner.split(BS)[-1]}::{member}", [])
            fresh = sorted(set(inferred) - set(p.nvst))
            p.inferred = len(fresh)
            p.nvst = sorted(set(p.nvst) | set(fresh))
        if e.examples_dir.is_dir():
            p.examples = sorted(rel(f) for f in e.examples_dir.glob("*.nvs"))
        if e.hostile_dir.is_dir():
            p.hostile = sorted(rel(f) for f in e.hostile_dir.glob("*.nvs"))
        for d in (e.examples_dir, e.hostile_dir):
            if d.is_dir():
                p.gaps += [rel(f) for f in sorted(d.glob("*.nvs")) if known_gap(read(f))]
        if e.bench_file.exists():
            p.bench = rel(e.bench_file)
        records = perf.get(e.id, [])
        mine = [r for r in records if r.get("machine") == me]
        p.perf = mine[-1] if mine else None
        current = e.impl_file and last_commit(e.impl_file)
        fresh = [r for r in records if not current or r.get("impl_commit") == current]
        p.perf_any = fresh[-1] if fresh else None
        out[e.id] = p
    return out


# ------------------------------------------------------------------------------ the verdict


def owed(entry: Entry, proofs: Proofs, policy: dict, skips: dict) -> dict[str, str]:
    """What this feature still owes, proof -> one sentence. Empty means complete."""
    want = policy[entry.kind]
    skip = skips.get(entry.id, {})
    out: dict[str, str] = {}

    if "tests" not in skip:
        have = len(proofs.nvst) + len(proofs.rust)
        if have < want["tests"]:
            out["tests"] = f"{have} of {want['tests']} cases"
        elif want.get("rust") and not proofs.rust:
            out["tests"] = "no Rust-side test carries its `covers:` marker"
    if "examples" not in skip and len(proofs.examples) < want["examples"]:
        out["examples"] = f"{len(proofs.examples)} of {want['examples']} in {rel(entry.examples_dir)}"
    if "hostile" not in skip and len(proofs.hostile) < want["hostile"]:
        out["hostile"] = f"{len(proofs.hostile)} of {want['hostile']} in {rel(entry.hostile_dir)}"
    if want["perf"] and "perf" not in skip:
        # `perf_any` and not `perf`: a figure taken on another machine at this same implementation
        # commit documents the feature just as well, and a fresh clone that owed every figure it
        # already has is a proof nobody would keep switched on. `--record-perf` is how a machine
        # gets its own numbers, and `--perf-report` is the only thing that insists on them.
        if not proofs.bench:
            out["perf"] = f"no bench at {rel(entry.bench_file)}"
        elif not proofs.perf_any:
            out["perf"] = ("never measured" if not entry.impl_file else
                           f"stale: {entry.impl_file} changed since it was last measured")
    return out


# --------------------------------------------------------------------------------- running


def jobs_for(count: int) -> int:
    try:
        return machine.jobs("local", ceiling=count, envs=("NVS_DOSSIER_JOBS",))
    except Exception:
        return min(4, max(1, count))


def normalise(text: str) -> str:
    return text.replace("\r\n", "\n").rstrip()


def run_one_example(nvs: Path, path: Path) -> tuple[str, str]:
    source = read(path)
    if UNIMPL_RE.search(source):
        return "skip", "marked `requires: unimplemented`"
    try:
        out = subprocess.run([str(nvs), "run", str(path)], timeout=60, **CAPTURE)
    except subprocess.TimeoutExpired:
        return "fail", "timed out after 60s"
    if out.returncode != 0:
        first = (out.stderr.strip().splitlines() or [""])[0]
        return "fail", f"exit {out.returncode}: {first}"
    expected_file = path.with_suffix(".out")
    if not expected_file.exists():
        return "fail", f"no {expected_file.name} beside it"
    if normalise(out.stdout) != normalise(read(expected_file)):
        return "fail", "stdout differs from its .out"
    return "ok", ""


def run_one_hostile(nvs: Path, path: Path, valgrind: bool) -> tuple[str, str]:
    """An attack passes when the *runtime* survives it, whatever the program's own fate.

    A hostile case has no frozen output on purpose -- freezing one would make it a fixture, and the
    question here is not what it printed. It is: did anything come apart? A thrown exception, a
    limit stopping it, a clean fatal and a successful run are all passes. A panic, an abort, a hang,
    a crash-shaped exit status or a definite leak are not.

    **A compile diagnostic is the one failure that looks like a pass**, and it is checked for
    explicitly. An attack that does not compile was never delivered -- a typo would otherwise
    "survive" every sweep for the rest of the repository's life, which is the exact shape of
    `loop-authoring.md` § 3's "a green suite is not a run guard". Where the refusal *is* the
    assertion -- a sink handed a tainted value, a capability used without being granted -- the case
    says `// hostile: expect-refusal`, and then compiling cleanly is what fails it.
    """
    source = read(path)
    if UNIMPL_RE.search(source):
        return "skip", "marked `requires: unimplemented`"
    m = TIMEOUT_RE.search(source)
    limit = int(m.group(1)) / 1000 if m else 10.0
    argv = [str(nvs), "run", str(path)]
    if valgrind:
        argv = ["valgrind", "--quiet", "--error-exitcode=97", "--leak-check=full",
                "--errors-for-leak-kinds=definite", *argv]
        limit *= 20
    try:
        out = subprocess.run(argv, timeout=limit, **CAPTURE)
    except subprocess.TimeoutExpired:
        return "fail", f"still running after {limit:.0f}s -- unbounded"
    except OSError as exc:
        return "fail", f"could not run: {exc}"
    blob = out.stderr + out.stdout
    for marker in CRASH_MARKERS:
        if marker in blob:
            return "fail", f"stderr carries {marker!r}"
    refused = bool(REFUSED_RE.search(out.stderr))
    if REFUSAL_EXPECTED_RE.search(source):
        if not refused:
            return "fail", "declares `expect-refusal`, but the compiler accepted it"
        return "ok", ""
    if refused:
        first = next((ln for ln in out.stderr.splitlines() if ln.startswith("error[")), "")
        return "fail", f"never ran -- it does not compile: {first}"
    if valgrind and out.returncode == 97:
        return "fail", "valgrind reports a definite leak or an invalid access"
    # A negative status is a signal on POSIX; anything past 255 on Windows is a structured
    # exception (0xC0000005 and friends) rather than an exit code a program chose.
    if out.returncode < 0 or out.returncode > 255:
        return "fail", f"crash-shaped exit status {out.returncode}"
    return "ok", ""


def known_gap(source: str) -> tuple[str, str] | None:
    """The `known-gap` marker in a proof file, as (module doc path, reason), or None."""
    m = KNOWN_GAP_RE.search(source)
    return (m.group(1), m.group(2).strip(" -\t")) if m else None


def judge_gap(path: Path, verdict: str, why: str) -> tuple[str, str]:
    """Re-judge one result against the file's `known-gap` marker, if it carries one.

    **A proof that fails has found something, and the only two honest answers are to fix it or to
    record it.** Weakening the proof is neither, and it is the cheapest thing an unattended session
    could do, so this makes the third path a real one: mark the file, and the failure becomes a
    counted, printed `known-gap` rather than a red check the run cannot get past.

    Two things keep that from becoming a way to make anything green. The marker must name a module
    doc that actually carries a `# Known gaps` section -- this repository's existing home for
    exactly this fact -- so recording a bug means writing it where the crate's own readers will
    find it. And a marked file that *passes* fails: the gap it names is fixed, and the marker has
    to go with it.
    """
    marker = known_gap(read(path))
    if not marker:
        return verdict, why
    doc, reason = marker
    target = ROOT / doc
    if not target.exists():
        return "fail", f"`known-gap` names {doc}, which does not exist"
    if "# Known gaps" not in read(target):
        return "fail", f"`known-gap` names {doc}, which has no `# Known gaps` section to hold it"
    if verdict == "ok":
        return "fail", f"passes, but is still marked `known-gap` against {doc} -- remove the marker"
    if verdict == "skip":
        return verdict, why
    return "known", f"{reason or why} (recorded in {doc})"


def binary_key(nvs: Path) -> str:
    """The binary, cheaply: size and modification time. Hashing 80 MB per invocation to learn what
    a `stat` already answered is the sort of cost this cache exists to avoid."""
    st = nvs.stat()
    return f"{st.st_size}-{st.st_mtime_ns}"


def load_green() -> dict:
    if not GREEN.exists():
        return {}
    try:
        return json.loads(read(GREEN))
    except json.JSONDecodeError:
        return {}


def save_green(doc: dict) -> None:
    try:
        GREEN.parent.mkdir(parents=True, exist_ok=True)
        GREEN.write_text(json.dumps(doc, indent=0, sort_keys=True), encoding="utf-8", newline="\n")
    except OSError:
        pass  # a cache that cannot be written is a slow run, never a failed one


def run_suite(nvs: Path, what: str, files: list[Path], valgrind: bool, quiet: bool,
              use_cache: bool = True, strict: bool = False) -> int:
    """Run one suite, remembering what was green.

    **A green verdict is keyed on the bytes that produced it** -- the program's own hash, the
    binary's identity, and whether valgrind was in the loop -- so an unchanged program against an
    unchanged binary is skipped rather than re-run. That is the same argument `verify.py` makes for
    its green cache and `loop.py` for `.loop/goal-green.json`: a deterministic run over identical
    bytes cannot reach a different verdict. Only `ok` is remembered. A failure is re-run and
    re-reported every time, because the one thing worse than a slow check is a cached red one that
    stops being mentioned.
    """
    if not files:
        print(f"dossier: no {what} on disk yet -- nothing to run.")
        return 0
    green = load_green() if use_cache else {}
    bkey = binary_key(nvs)
    tag = f"{what}:{'valgrind' if valgrind else 'plain'}"
    todo, cached = [], 0
    for path in files:
        digest = hashlib.sha1(path.read_bytes()).hexdigest()[:16]
        expected = green.get(f"{tag}:{rel(path)}")
        out_file = path.with_suffix(".out")
        if what == "examples" and out_file.exists():
            digest += hashlib.sha1(out_file.read_bytes()).hexdigest()[:16]
        if expected == [digest, bkey]:
            cached += 1
        else:
            todo.append((path, digest))

    width = jobs_for(len(todo)) if todo else 1
    started = time.time()
    results: list[tuple[Path, str, str]] = []
    runner = run_one_example if what == "examples" else (
        lambda n, p: run_one_hostile(n, p, valgrind))
    if todo:
        with concurrent.futures.ThreadPoolExecutor(max_workers=width) as pool:
            futures = {pool.submit(runner, nvs, p): (p, d) for p, d in todo}
            for fut in concurrent.futures.as_completed(futures):
                path, digest = futures[fut]
                verdict, why = judge_gap(path, *fut.result())
                results.append((path, verdict, why))
                if verdict == "ok":
                    green[f"{tag}:{rel(path)}"] = [digest, bkey]
                else:
                    green.pop(f"{tag}:{rel(path)}", None)
    if use_cache:
        save_green(green)

    ok = sum(1 for _, v, _ in results if v == "ok") + cached
    skipped = sum(1 for _, v, _ in results if v == "skip")
    gaps = [(p, w) for p, v, w in results if v == "known"]
    bad = [(p, w) for p, v, w in results if v == "fail"]
    if strict:
        bad += gaps
        gaps = []
    for path, why in sorted(bad, key=lambda r: str(r[0])):
        print(f"  FAIL  {rel(path)}: {why}")
    for path, why in sorted(gaps, key=lambda r: str(r[0])):
        print(f"  gap   {rel(path)}: {why}")
    if not quiet:
        for path, verdict, why in sorted(results, key=lambda r: str(r[0])):
            if verdict == "skip":
                print(f"  skip  {rel(path)}: {why}")
    print(f"dossier {what}: {ok} ok, {skipped} skipped, {len(gaps)} known-gap, {len(bad)} failed "
          f"({len(files)} files, {cached} unchanged since they last passed, "
          f"{width} at a time, {time.time() - started:.1f}s)")
    return 1 if bad else 0


# ------------------------------------------------------------------------------ measurement


def fingerprint() -> dict:
    """This machine, as the ledger records it. Two records compare only when these agree."""
    cpu = platform.processor() or platform.machine()
    try:
        prof = machine.profile("local")
        cores = prof.get("cores")
    except Exception:
        cores = os.cpu_count()
    fields = {
        "cpu": cpu,
        "os": platform.system().lower(),
        "release": platform.machine(),
        "cores": cores,
    }
    blob = json.dumps(fields, sort_keys=True)
    fields["id"] = hashlib.sha1(blob.encode()).hexdigest()[:12]
    return fields


def time_program(nvs: Path, path: Path, reps: int) -> float:
    """The fastest of `reps` runs, in nanoseconds. Fastest, not mean: the floor is the signal and
    everything above it is the machine doing something else."""
    best = float("inf")
    for _ in range(reps):
        started = time.perf_counter_ns()
        out = subprocess.run([str(nvs), "run", str(path)], timeout=600, **CAPTURE)
        spent = time.perf_counter_ns() - started
        if out.returncode != 0:
            raise RuntimeError(f"{rel(path)} exited {out.returncode}: "
                               f"{(out.stderr.strip().splitlines() or [''])[0]}")
        best = min(best, spent)
    return best


def iterations_of(path: Path) -> int:
    m = ITER_RE.search(read(path))
    if not m:
        raise RuntimeError(f"{rel(path)} declares no `// bench: iterations N`")
    return int(m.group(1).replace("_", ""))


def calibrate(nvs: Path, reps: int) -> tuple[float, float]:
    """(the empty program's floor in ns, the calibration unit's ns per iteration).

    Everything the ledger records is measured against these two, taken in the same sweep on the same
    machine: the floor is subtracted so a figure is the work rather than the CLI's start-up, and the
    unit is what a ratio is expressed in so the number means something on somebody else's box.
    """
    baseline = CALIBRATION / "baseline.nvs"
    unit = CALIBRATION / "unit.nvs"
    for p in (baseline, unit):
        if not p.exists():
            raise RuntimeError(f"the calibration program {rel(p)} is missing")
    floor = time_program(nvs, baseline, reps)
    unit_total = time_program(nvs, unit, reps)
    unit_ns = max(1e-9, (unit_total - floor) / iterations_of(unit))
    return floor, unit_ns


def record_perf(nvs: Path, entries: list[Entry], reps: int, note: str, proofs: dict[str, Proofs],
                policy: dict, skips: dict, force: bool) -> int:
    """Measure and append. **By default only what has no current figure**, which is what makes this
    safe to put at the end of a slice: a session that edited one file re-measures that file's
    features and nothing else, and running it twice costs a walk. `--force` re-measures everything
    in scope, for when the question is the machine rather than the code."""
    todo = [e for e in entries if e.bench_file.exists()]
    if not force:
        todo = [e for e in todo if "perf" in owed(e, proofs[e.id], policy, skips)]
        if not todo:
            print("dossier: every bench in scope already has a current figure "
                  "(--force re-measures anyway).")
            return 0
    if not todo:
        print("dossier: no bench programs in scope -- nothing to measure.")
        return 0
    if "release" not in str(nvs):
        print(f"dossier: refusing to measure with {rel(nvs)} -- build a release binary "
              f"(`cargo build --release -p nvs-cli`).")
        return 1
    fp = fingerprint()
    commit = git("rev-parse", "HEAD")[:12]
    dirty = bool(git("status", "--porcelain"))
    try:
        floor, unit_ns = calibrate(nvs, reps)
    except RuntimeError as exc:
        print(f"dossier: {exc}")
        return 1
    print(f"dossier perf: {len(todo)} features, {reps} reps, unit = {unit_ns:.1f} ns/iteration "
          f"on {fp['cpu']} ({fp['id']})")
    lines = []
    for e in sorted(todo, key=lambda x: x.id):
        try:
            iters = iterations_of(e.bench_file)
            total = time_program(nvs, e.bench_file, reps)
        except (RuntimeError, subprocess.SubprocessError) as exc:
            print(f"  FAIL  {e.id}: {exc}")
            return 1
        ns_per_op = max(0.0, (total - floor) / iters)
        rec = {
            "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "id": e.id,
            "kind": e.kind,
            "group": e.group,
            "commit": commit,
            "dirty": dirty,
            "impl_commit": last_commit(e.impl_file) if e.impl_file else "",
            "machine": fp["id"],
            "cpu": fp["cpu"],
            "os": fp["os"],
            "cores": fp["cores"],
            "reps": reps,
            "iterations": iters,
            "ns_per_op": round(ns_per_op, 3),
            "unit_ns": round(unit_ns, 3),
            "ratio": round(ns_per_op / unit_ns, 4),
            "note": note,
        }
        lines.append(json.dumps(rec))
        print(f"  {e.id:44} {ns_per_op:10.1f} ns/op   {rec['ratio']:8.3f} units")
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    with LEDGER.open("a", encoding="utf-8", newline="\n") as fh:
        fh.write("\n".join(lines) + "\n")
    print(f"dossier perf: {len(lines)} records appended to {rel(LEDGER)}")
    return 0


def perf_report() -> int:
    """Regenerate `docs/perf/members.md` -- the ledger's front page, overwritten every time.

    One row per feature per machine, newest reading against the one before it on that same machine.
    A delta across fingerprints is never printed, because there is no honest one to print.
    """
    if not LEDGER.exists():
        print(f"dossier: {rel(LEDGER)} does not exist yet -- run --record-perf first.")
        return 1
    history: dict[tuple[str, str], list[dict]] = {}
    for line in read(LEDGER).splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        history.setdefault((rec.get("machine", ""), rec.get("id", "")), []).append(rec)

    machines: dict[str, dict] = {}
    for (mid, _fid), recs in history.items():
        machines.setdefault(mid, recs[-1])

    out = [
        "# Measured cost, feature by feature",
        "",
        "**Generated by `python tools/dossier.py --perf-report` — never edited.**",
        f"[`{rel(LEDGER)}`]({LEDGER.name}) is the append-only ledger this is the front page of;",
        "`tools/dossier.py` owns how a figure is taken and `benches/members/README.md` owns what a",
        "bench program is.",
        "",
        "Every figure is Novis against **itself**: there is no PHP column here and there never will",
        "be — [`benches/userland/`](../../benches/userland/README.md) owns the cross-engine",
        "comparison. `ns/op` is wall clock on the machine named in the heading, with the empty",
        "program's start-up floor subtracted; `units` is that figure divided by the calibration",
        "program measured in the same sweep, and it is the only column that means anything on a",
        "different machine — to about a tenth ([ADR 0026](../adr/0026-performance-measurement-methodology.md)).",
        "`Δ` compares against the previous reading **on the same machine** and is blank when there",
        "is not one.",
        "",
    ]
    for mid, sample in sorted(machines.items()):
        rows = sorted(((fid, recs) for (m, fid), recs in history.items() if m == mid),
                      key=lambda r: r[0])
        out += [
            f"## {sample.get('cpu', 'unknown CPU')} · {sample.get('os', '?')} · "
            f"{sample.get('cores', '?')} cores  (`{mid}`)",
            "",
            "| Feature | ns/op | units | Δ | Measured at | Implementation |",
            "|---|---:|---:|---:|---|---|",
        ]
        for fid, recs in rows:
            last = recs[-1]
            prev = recs[-2] if len(recs) > 1 else None
            delta = ""
            if prev and prev.get("ns_per_op"):
                change = (last["ns_per_op"] - prev["ns_per_op"]) / prev["ns_per_op"] * 100
                delta = f"{change:+.1f}%"
            out.append(f"| `{fid}` | {last.get('ns_per_op', 0):.1f} | "
                       f"{last.get('ratio', 0):.3f} | {delta} | {last.get('commit', '')} | "
                       f"{last.get('impl_commit', '')} |")
        out.append("")
    PERF_REPORT.write_text("\n".join(out), encoding="utf-8", newline="\n")
    print(f"dossier: wrote {rel(PERF_REPORT)} "
          f"({sum(len(v) for v in history.values())} records, {len(machines)} machines)")
    return 0


# ------------------------------------------------------------------------------- reporting


def group_rows(entries: list[Entry], proofs: dict[str, Proofs], policy: dict,
               skips: dict) -> list[dict]:
    groups: dict[str, list[Entry]] = {}
    for e in entries:
        groups.setdefault(e.group, []).append(e)
    rows = []
    for name, members in sorted(groups.items()):
        counts = {p: 0 for p in PROOFS}
        complete = 0
        for e in members:
            missing = owed(e, proofs[e.id], policy, skips)
            for p in PROOFS:
                if p not in missing:
                    counts[p] += 1
            if not missing:
                complete += 1
        rows.append({
            "group": name,
            "kind": members[0].kind,
            "features": len(members),
            "complete": complete,
            "gaps": sum(len(proofs[e.id].gaps) for e in members),
            **{p: counts[p] for p in PROOFS},
        })
    rows.sort(key=lambda r: (r["complete"] / r["features"], -r["features"], r["group"]))
    return rows


HEADINGS = {"tests": "tests", "examples": "exmpl", "perf": "perf", "hostile": "hostl"}


def print_status(rows: list[dict], columns: tuple[str, ...]) -> None:
    total = sum(r["features"] for r in rows)
    done = sum(r["complete"] for r in rows)
    print("== WHAT EACH FEATURE STILL OWES  "
          f"({total} features in {len(rows)} groups, {done} complete)")
    print("-- a column counts the features of that group whose proof is on disk and current.")
    print("-- `python tools/dossier.py --group <name>` opens one; --owed is the worklist.")
    if "perf" not in columns:
        print("-- the perf proof is switched off, so it is neither owed nor shown.")
    print()
    print(f"  {'DONE':>9}  " + " ".join(f"{HEADINGS[c]:>6}" for c in columns) + "  group")
    for r in rows:
        print(f"  {r['complete']:4}/{r['features']:<4}  "
              + " ".join(f"{r[c]:6}" for c in columns) + f"  {r['group']}")
    print()
    print(f"  {done}/{total} features complete "
          f"({done / total * 100:.1f}%)" if total else "  nothing on the roster")
    gaps = sum(r["gaps"] for r in rows)
    if gaps:
        print(f"  {gaps} proof(s) carry a `known-gap` marker: a bug the proof found and nobody has")
        print("  fixed yet, recorded in the owning crate's `# Known gaps`. `--run … --strict` fails")
        print("  on them; `--owed --gaps` lists them. This number going up is the point of the")
        print("  hostile tree, and it going down is the point of the rest of the repository.")


def print_group(name: str, entries: list[Entry], proofs: dict[str, Proofs], policy: dict,
                skips: dict, columns: tuple[str, ...]) -> None:
    members = [e for e in entries if e.group == name]
    if not members:
        near = sorted({e.group for e in entries if name.lower() in e.group.lower()})
        print(f"dossier: no group {name!r}." + (f" Did you mean: {', '.join(near[:6])}?"
                                               if near else ""))
        return
    print(f"== {name}  ({len(members)} features)")
    print("  " + " ".join(f"{HEADINGS[c]:>5}" for c in columns) + "  feature")
    for e in sorted(members, key=lambda x: x.id):
        p = proofs[e.id]
        missing = owed(e, p, policy, skips)
        have = {"tests": len(p.nvst) + len(p.rust), "examples": len(p.examples),
                "perf": 1 if p.perf else 0, "hostile": len(p.hostile)}
        cells = []
        for c in columns:
            cell = "-" if c in skips.get(e.id, {}) else \
                f"{have[c]}" + ("!" if c in missing else "")
            cells.append(f"{cell:>5}")
        print("  " + " ".join(cells) + f"  {e.id}")
    print("\n  `!` marks a proof still owed, `-` one this feature is excused from.")


def print_entry(fid: str, entries: list[Entry], proofs: dict[str, Proofs], policy: dict,
                skips: dict) -> int:
    match = next((e for e in entries if e.id == fid), None)
    if match is None:
        near = [e.id for e in entries if fid.lower() in e.id.lower()][:8]
        print(f"dossier: no feature {fid!r}." + (f" Near: {', '.join(near)}" if near else ""))
        return 1
    p = proofs[match.id]
    missing = owed(match, p, policy, skips)
    print(f"== {match.id}   [{match.kind}]")
    if match.summary:
        print(f"   {safe(match.summary[:100])}")
    if match.anchor:
        print(f"   implemented at {match.anchor}")
    if match.twin:
        print(f"   replaces PHP: {', '.join(match.twin)}  (a differential case needs no frozen output)")
    print()
    print(f"   tests     {len(p.nvst)} case(s), {len(p.rust)} Rust")
    for f in p.nvst[:6]:
        print(f"     {f}")
    if p.inferred:
        print(f"     ({p.inferred} credited by a call rather than a `covers:` marker)")
    for f in p.rust[:6]:
        print(f"     {f}")
    print(f"   examples  {len(p.examples)} in {rel(match.examples_dir)}")
    for f in p.examples:
        print(f"     {f}")
    print(f"   perf      {p.bench or 'no bench at ' + rel(match.bench_file)}")
    if p.perf:
        print(f"     {p.perf['ns_per_op']} ns/op, {p.perf['ratio']} units, "
              f"measured at {p.perf['commit']} on {p.perf['machine']}")
    print(f"   hostile   {len(p.hostile)} in {rel(match.hostile_dir)}")
    for f in p.hostile:
        print(f"     {f}")
    if p.gaps:
        print(f"   known-gap {len(p.gaps)} proof(s) found a bug nobody has fixed:")
        for f in p.gaps:
            gap = known_gap(read(ROOT / f))
            print(f"     {f} -> {gap[0] if gap else '?'}: {gap[1] if gap else ''}")
    print()
    if missing:
        print("   OWED:")
        for proof, why in missing.items():
            print(f"     {proof:9} {why}")
    else:
        print("   complete.")
    return 0


def print_owed(entries: list[Entry], proofs: dict[str, Proofs], policy: dict, skips: dict,
               limit: int) -> None:
    rows = []
    for e in entries:
        missing = owed(e, proofs[e.id], policy, skips)
        if missing:
            rows.append((e, missing))
    print(f"== STILL OWED  ({len(rows)} features)")
    print("-- one line per feature; a session takes a feature, not a column, because all four "
          "proofs\n-- spend the same understanding of what the feature does at its edges.")
    print()
    for e, missing in rows[:limit]:
        print(f"  {e.id:46} {', '.join(sorted(missing))}")
    if len(rows) > limit:
        print(f"  ... and {len(rows) - limit} more (--limit 0 for all)")


# ------------------------------------------------------------------------------ goal writing


GOAL_RULES = ["0079", "0063", "0026", "0117", "0004", "0088"]
GOAL_SHAPES = [
    "A `.nvst` test case",
    "A feature's four proofs — an example, an attack, a bench, a `covers:` marker",
    "A commit message",
]
GOAL_PLAYBOOK = [
    "Tooling > the end",
    "Tooling > one call reads",
    "Tooling > a commit",
    "Tooling > a [context]",
    "Tooling > another session",
    "Writing a test case > a rule",
    "Writing a test case > a -p",
    "Running things > target/release/nvs.exe is",
]


def goal_batches(entries: list[Entry], size: int) -> list[tuple[str, list[Entry]]]:
    """Cut the roster into goals, **by file set and never across a group**.

    A goal is a finite contained group of work whose `[context]` manifest is what keeps a session
    under the ceiling, so the split follows the files its sessions open: one class, or a run of
    small classes sharing an implementing module. A class larger than `size` becomes several goals
    numbered `(1/3)`, `(2/3)` — the file set is identical, so the manifests are too.
    """
    groups: dict[str, list[Entry]] = {}
    for e in entries:
        groups.setdefault(e.group, []).append(e)

    def file_set(members: list[Entry]) -> str:
        """The implementing file most of a group's features live in -- what two adjacent groups
        have to share before merging them into one goal is worth anything. Merging by name would
        put `Core\\Ast` beside `Core\\Bytes` because A precedes B, and their sessions would open
        two file sets to save one goal."""
        files = [e.impl_file for e in members if e.impl_file]
        return max(set(files), key=files.count) if files else ""

    batches: list[tuple[str, list[Entry]]] = []
    pending: list[Entry] = []
    pending_names: list[str] = []

    def flush() -> None:
        nonlocal pending, pending_names
        if pending:
            label = pending_names[0] if len(pending_names) == 1 else \
                f"{pending_names[0]} and {len(pending_names) - 1} more"
            batches.append((label, pending))
            pending, pending_names = [], []

    ordered = sorted(groups.items(), key=lambda kv: (kv[1][0].kind, file_set(kv[1]), kv[0]))
    for name, members in ordered:
        members.sort(key=lambda e: e.id)
        if len(members) > size:
            flush()
            chunks = [members[i:i + size] for i in range(0, len(members), size)]
            for n, chunk in enumerate(chunks, 1):
                batches.append((f"{name} ({n}/{len(chunks)})", chunk))
            continue
        if pending and (len(pending) + len(members) > size
                        or file_set(pending) != file_set(members)):
            flush()
        pending += members
        pending_names.append(name)
    flush()
    return batches


def emit_goals(entries: list[Entry], proofs: dict[str, Proofs], policy: dict, skips: dict,
               out_dir: Path, size: int, skip_complete: bool, no_perf: bool) -> int:
    """Write one goal triple per batch, plus the `chain.toml` that walks them.

    Regenerating is safe and is how the roster grows: a batch whose features are all complete is
    left out entirely (`--all-groups` keeps them), so a second run after a hundred sessions writes
    the chain that is *left*, not the one that was.
    """
    todo = entries
    if skip_complete:
        todo = [e for e in entries if owed(e, proofs[e.id], policy, skips)]
    if not todo:
        print("dossier: nothing owed -- no goals to write.")
        return 0
    batches = goal_batches(todo, size)
    out_dir = out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    written: list[tuple[int, str, str]] = []
    where = rel(out_dir)                      # posix, and relative to the repository if it is inside

    for n, (label, members) in enumerate(batches, 1):
        slug = f"{n:03d}-{slugify(label)}"[:60]
        anchors = sorted({e.impl_file for e in members if e.impl_file})
        groups = sorted({e.group for e in members})
        # A group split across several goals gates on its own features and not on the class, or
        # every part of the split would wait for all of them and only the last could ever pass.
        split = bool(re.search(r"\(\d+/\d+\)$", label))
        (out_dir / f"{slug}.md").write_text(goal_prose(n, label, members, proofs, policy, skips),
                                            encoding="utf-8", newline="\n")
        (out_dir / f"{slug}.toml").write_text(
            goal_toml(n, label, groups, anchors, no_perf,
                      [e.id for e in members] if split else None),
            encoding="utf-8", newline="\n")
        (out_dir / f"{slug}.handoff.md").write_text(
            goal_handoff(n, label, members, proofs, policy, skips), encoding="utf-8", newline="\n")
        written.append((n, label, slug))

    chain = [
        "# The dossier program's goal order, read by",
        f"# `python tools/loop.py --chain {where}/chain.toml`.",
        "#",
        "# GENERATED by `python tools/dossier.py --emit-goals` -- do not hand-edit; re-run it.",
        "# `docs/adr/0134-every-shipped-feature-owes-four-proofs.md` is why this program exists and",
        "# `tools/dossier.py --help` is how it is driven. One entry per goal, in roster order:",
        "# a goal is one group of features sharing an implementing file set, which is what keeps",
        "# each `[context]` manifest small.",
        "",
    ]
    for n, label, slug in written:
        chain += [
            "[[goal]]",
            # A single-quoted TOML literal: a group name carries a backslash, and a backslash in a
            # basic string is an escape sequence the parser then refuses.
            f"name = '{n} {label}'",
            f'md = "{where}/{slug}.md"',
            f'toml = "{where}/{slug}.toml"',
            f'handoff = "{where}/{slug}.handoff.md"',
            'milestone = "dossier"',
            "",
        ]
    (out_dir / "chain.toml").write_text("\n".join(chain), encoding="utf-8", newline="\n")

    print(f"dossier: wrote {len(written)} goals into {where}/ "
          f"({sum(len(m) for _, m in batches)} features owed, up to {size} per goal)")
    print()
    print("Start the run with:")
    print(f"    python tools/loop.py --chain {where}/chain.toml --max-sessions 300")
    return 0


def goal_prose(n: int, label: str, members: list[Entry], proofs: dict[str, Proofs], policy: dict,
               skips: dict) -> str:
    lines = [
        f"# Dossier goal {n} — {label}",
        "",
        "**Generated by `python tools/dossier.py --emit-goals`.** The checks are the sibling",
        "`.toml`; this half is the target and the standing decisions. Regenerating overwrites both.",
        "",
        "## The target",
        "",
        f"Every feature listed below owes the four proofs of",
        "[ADR 0134](../../adr/0134-every-shipped-feature-owes-four-proofs.md): the behaviour tested",
        "Novis *and* from Rust, three small real-world examples, one measured performance figure,",
        "and one file written to break it. `python tools/dossier.py --id '<feature>'` prints what",
        "one feature has and what it still owes, with the path each proof belongs at.",
        "",
        "## The item list, grouped by file set",
        "",
        "**One slice is one feature, all four proofs together** — never one proof across many",
        "features. The expensive thing a session buys is understanding what the feature does at its",
        "edges, and the test, the examples, the bench and the attack all spend that same",
        "understanding; split across four sessions it is bought four times.",
        "",
    ]
    for i, e in enumerate(members, 1):
        missing = owed(e, proofs[e.id], policy, skips)
        if not missing:
            continue
        lines.append(f"{i}. **`{e.id}`** — owes {', '.join(sorted(missing))}."
                     + (f" `{e.anchor}`" if e.anchor else ""))
        if e.twin:
            lines.append(f"   Replaces PHP `{'`, `'.join(e.twin[:4])}` — so its oracle case goes in")
            lines.append("   `tests/differential/` and needs no frozen output.")
    lines += [
        "",
        "## Standing decisions",
        "",
        "Settled before the run; a session decides and records, and never reports `BLOCKED` for any",
        "of these.",
        "",
        "- **An example is written for a reader, not for a test.** Small, self-contained, one",
        "  sentence of plain comment saying what it shows — no ADR numbers, no internal vocabulary.",
        "  Three per member, each a *different* use, and the third is the one that earns its place:",
        "  make it the thing somebody actually does with this feature at work.",
        "- **A `.out` file is created with `--bless` and then read.** Blessing is how the expected",
        "  output is *created*; a red example is never made green by re-blessing it.",
        "- **A hostile case has no expected output.** Its whole assertion is that the runtime",
        "  survived: no panic, no abort, no hang, no definite leak. Write the attack an attacker",
        "  would write — unbounded input, deep nesting, an allocation the program does not free, a",
        "  boundary crossed by one, a value at the far end of a range.",
        "- **A bench program measures the feature and nothing else**, chains its inputs so no",
        "  optimiser can hoist the loop, and declares `// bench: iterations N`.",
        "  `benches/members/README.md` is the shape.",
        "- **Every proof carries a `covers:` marker naming its feature.** That is the only thing",
        "  attributing it, and for a Rust `#[test]` it is the only thing.",
        "- **A feature that genuinely cannot carry a proof** — a member whose program exits, a",
        "  directive with no runtime cost — goes in `tools/data/dossier-policy.toml`'s `[skip]`",
        "  with the reason in one sentence, and the session says so in its commit. That is a",
        "  recorded decision, not a gap. It is **not** the answer to a proof that fails; see below.",
        "",
        "### When a proof finds a bug",
        "",
        "It will. An attack written to break a member sometimes does, and an example written",
        "against the documented behaviour sometimes disagrees with the binary. **That is the",
        "program working, not a problem with the slice**, and there are exactly two honest",
        "answers — in this order:",
        "",
        "1. **Fix it.** This is the default and it is in scope: the fix, a `.nvst` case pinning the",
        "   corrected behaviour, and the proof that found it, in the same slice. Most will be small.",
        "2. **Record it**, when the fix is genuinely larger than a slice — a representation change, a",
        "   design question, a refusal that needs an ADR. Add the entry to the owning crate's module",
        "   doc `# Known gaps` (this repository's existing home for exactly this fact), and mark the",
        "   proof with the file that carries it:",
        "",
        "       // dossier: known-gap crates/nvs-stdlib/src/str.rs -- one sentence saying what breaks",
        "",
        "   The sweep then counts it as `known-gap` rather than a failure, so the run continues and",
        "   the bug stays visible in `python tools/dossier.py --gaps`. A marked proof that *passes*",
        "   fails the sweep, so removing the marker is part of whatever fix eventually lands.",
        "",
        "**Weakening the proof is not one of the two.** Do not soften an attack until it stops",
        "failing, do not bless an example's `.out` to whatever the binary currently prints, and do",
        "not `[skip]` the feature. Those all turn a finding into a green check, which is the one",
        "outcome this whole program exists to prevent. If you are unsure whether the binary or the",
        "proof is right, the ADR that owns the member decides; say which one you read in the commit.",
        "- **No numbered ADR is opened by this goal.** Nothing here is a design decision; it is",
        "  proof for designs that already landed. A finding that contradicts an ADR goes in the",
        "  handoff's `## Backlog`.",
        "",
    ]
    return "\n".join(lines) + "\n"


def goal_toml(n: int, label: str, groups: list[str], anchors: list[str], no_perf: bool,
              only: list[str] | None) -> str:
    def toml_str(s: str) -> str:
        return "'" + s + "'" if BS in s else '"' + s + '"'

    #: The gate the loop runs carries the same switch the sweep that wrote these goals did.
    #: Otherwise a run started with the perf proof off would be judged by a gate that wants it.
    perf_flag = ', "--no-perf"' if no_perf else ""

    lines = [
        f"# Dossier goal {n} -- {label}. The acceptance test, as data.",
        "#",
        "# GENERATED by `python tools/dossier.py --emit-goals` -- do not hand-edit; re-run it.",
        "# The sibling `.md` is the prose. One home each.",
        "",
        "files = []",
        "",
        "[context]",
        "",
        "modules = [",
    ]
    lines += [f'  "{a}",' for a in anchors[:14]] or ['  # no implementing file resolved']
    lines += [
        "]",
        "",
        f"rules = [{', '.join(chr(34) + r + chr(34) for r in GOAL_RULES)}]",
        "",
        # 0134 whole -- it is what the goal exists to satisfy. 0079 § 15 for what a benchmark of
        # Novis code is. NOT 0026 whole: it is 5k about callgrind, and the rule a session writing a
        # bench needs is 0134 § 4 plus benches/members/README.md.
        'adrs = ["0134", "0079 §15"]',
        "",
        "shapes = [",
    ]
    lines += [f'  "{s}",' for s in GOAL_SHAPES]
    lines += ["]", "", "playbook = ["]
    lines += [f"  '{p}'," for p in GOAL_PLAYBOOK]
    lines += [
        "]",
        "",
        'plan = ["Open now", "Blocking"]',
        "",
        "",
        "# <<< goal-switch: floor checks are inserted below this line >>>",
        "",
        "",
        "# ---------------------------------------------------------------------------------------",
        "# Stage 2 -- every feature in this goal owes nothing AND the proofs hold: the four",
        "# artefacts are on disk, the perf figure is current for the implementation as it stands,",
        "# every example prints what its `.out` says, and every attack leaves the runtime standing.",
        "#",
        "# ONE check per group, not one per proof, and that is a cost decision rather than a",
        "# stylistic one: `goal-switch.py` copies a goal's whole check list into the next goal as",
        "# its floor, so a check written here is re-run by every session of every goal after this",
        "# one. `--verify` does the gate and both suites in a single process, and the suites",
        "# remember what was green, so a floor fifty goals deep is fifty short walks rather than",
        "# a hundred and fifty sweeps.",
        "# ---------------------------------------------------------------------------------------",
        "",
    ]
    if only:
        # This goal is one part of a class too large for a single goal, so it gates on its own
        # features by name. Gating on the class would make every part wait for all of them and
        # only the last part could ever pass.
        ids = ", ".join(toml_str(i) for i in only)
        lines += [
            "[[check]]",
            'kind = "command"',
            'stage = "2 the dossier"',
            f"name = {toml_str('dossier: ' + label)}",
            f'argv = ["python", "tools/dossier.py", "--verify"{perf_flag}, "--only", {ids}]',
            'want = ["nothing owed", "0 failed", "0 failed"]',
            "",
        ]
    else:
        for group in groups:
            lines += [
                "[[check]]",
                'kind = "command"',
                'stage = "2 the dossier"',
                f"name = {toml_str('dossier: ' + group)}",
                f"argv = [\"python\", \"tools/dossier.py\", \"--verify\", \"--group\", "
                f"{toml_str(group)}{perf_flag}]",
                'want = ["nothing owed", "0 failed", "0 failed"]',
                "",
            ]
    return "\n".join(lines) + "\n"


def goal_handoff(n: int, label: str, members: list[Entry], proofs: dict[str, Proofs], policy: dict,
                 skips: dict) -> str:
    first = [e for e in members if owed(e, proofs[e.id], policy, skips)][:3]
    lines = [
        f"# Handoff — dossier goal {n}, {label}",
        "",
        "**Generated by `python tools/dossier.py --emit-goals`; the first session of this goal",
        "overwrites it like any other handoff.**",
        "",
        "## Where the work stands",
        "",
        f"This goal has just been installed. Nothing in {label} has been taken yet.",
        "",
        "## The next group",
        "",
        "One slice is one feature with all four proofs. Take them in this order — they share an",
        "implementing file, so the second and third cost a fraction of the first:",
        "",
    ]
    for e in first:
        missing = owed(e, proofs[e.id], policy, skips)
        lines.append(f"- **`{e.id}`** — owes {', '.join(sorted(missing))}."
                     + (f" `{e.anchor}`" if e.anchor else ""))
    lines += [
        "",
        "`python tools/dossier.py --id '<feature>'` prints the paths each proof belongs at.",
        "",
        "## Backlog",
        "",
        "- (nothing yet)",
        "",
    ]
    return "\n".join(lines) + "\n"


# ------------------------------------------------------------------------------------- cli


def scope_label(args) -> str | None:
    if args.group:
        return args.group
    if args.only:
        return f"{len(args.only)} named feature(s)"
    return None


def gate(scope: list[Entry], proofs: dict[str, Proofs], policy: dict, skips: dict,
         group: str | None) -> int:
    """Exit 0 when nothing in scope is owed. **This judges and never executes** -- it reads the
    trees, the markers and the ledger, so it costs a walk however large the roster is."""
    missing = [(e, owed(e, proofs[e.id], policy, skips)) for e in scope]
    missing = [(e, m) for e, m in missing if m]
    where = group or "the whole roster"
    if missing:
        print(f"dossier gate: {len(missing)} of {len(scope)} features in {where} still owe a proof.")
        for e, m in missing[:30]:
            print(f"  {e.id:46} {', '.join(f'{k}: {v}' for k, v in sorted(m.items()))}")
        if len(missing) > 30:
            print(f"  ... and {len(missing) - 30} more")
        return 1
    print(f"dossier gate: nothing owed in {where} ({len(scope)} features, four proofs each).")
    return 0


def run_scoped(nvs: Path, what: str, scope: list[Entry], args) -> int:
    base, prefix = (EXAMPLES, "docs/examples/") if what == "examples" else (HOSTILE, "tests/hostile/")
    files = sorted(base.rglob("*.nvs")) if base.is_dir() else []
    if args.group or args.only:
        paths = {e.path for e in scope}
        files = [f for f in files if any(rel(f).startswith(f"{prefix}{p}/") for p in paths)]
    return run_suite(nvs, what, files, what == "hostile" and args.valgrind, args.quiet,
                     not args.no_cache, args.strict)


def bless(nvs: Path, targets: list[Path]) -> int:
    """Write an example's `.out` from what it actually prints.

    This is how an expected output is **created**. It is never how a red example is made green:
    that is the discipline `loop-authoring.md` § 6 states for fixtures, and the reason this prints
    the output it wrote rather than doing it silently.
    """
    failed = 0
    for path in targets:
        out = subprocess.run([str(nvs), "run", str(path)], timeout=120, **CAPTURE)
        if out.returncode != 0:
            print(f"  FAIL  {rel(path)} exited {out.returncode}:")
            print("        " + safe(out.stderr.strip().replace("\n", "\n        ")))
            failed += 1
            continue
        dest = path.with_suffix(".out")
        existed = dest.exists()
        dest.write_text(out.stdout.replace("\r\n", "\n"), encoding="utf-8", newline="\n")
        print(f"  {'rewrote' if existed else 'wrote'}  {rel(dest)}")
        for line in out.stdout.replace("\r\n", "\n").rstrip().split("\n"):
            print(f"      | {safe(line)}")
    if failed:
        return 1
    print("dossier: read what was written -- a blessed output is a claim, not a formality.")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--group", help="one group: a class, a chapter, `types:enum`, `config:directives`")
    ap.add_argument("--only", nargs="+", metavar="ID",
                    help="scope to these features by id -- what a goal covering part of a large "
                         "class gates on")
    ap.add_argument("--id", dest="feature", help="one feature, by its full id")
    ap.add_argument("--owed", action="store_true", help="only what is missing, as a worklist")
    ap.add_argument("--gaps", action="store_true",
                    help="every proof carrying a `known-gap` marker: the bugs the proofs found")
    ap.add_argument("--limit", type=int, default=40, help="rows before a summary line (0 = all)")
    ap.add_argument("--json", action="store_true", help="the audit as JSON")
    ap.add_argument("--gate", action="store_true",
                    help="exit 0 when nothing in scope is owed; name what is otherwise")
    ap.add_argument("--verify", action="store_true",
                    help="--gate, then --run examples and --run hostile over the same scope, in "
                         "one process. What the emitted loop goals call")
    ap.add_argument("--run", choices=("examples", "hostile", "all"),
                    help="run the proofs on disk and report what failed")
    ap.add_argument("--valgrind", action="store_true",
                    help="with --run hostile: under valgrind, so a definite leak fails it too")
    ap.add_argument("--quiet", action="store_true", help="failures only, no per-file lines")
    ap.add_argument("--no-perf", action="store_true",
                    help="drop the perf proof: not owed, not shown, not gated on. The ledger and "
                         "the benches are untouched, so switching it back on resumes")
    ap.add_argument("--no-cache", action="store_true",
                    help="with --run: re-run every program, even one unchanged since it passed")
    ap.add_argument("--strict", action="store_true",
                    help="with --run: a `known-gap` proof fails rather than being counted. What a "
                         "person runs to see the language's real debt; the loop does not")
    ap.add_argument("--record-perf", action="store_true", help="measure and append to the ledger")
    ap.add_argument("--reps", type=int, default=5, help="timed runs per program; the fastest wins")
    ap.add_argument("--force", action="store_true",
                    help="with --record-perf: re-measure everything in scope, not only what has "
                         "no current figure")
    ap.add_argument("--note", default="", help="a word recorded with each measurement")
    ap.add_argument("--perf-report", action="store_true", help="regenerate docs/perf/members.md")
    ap.add_argument("--bless", nargs="+", metavar="FILE",
                    help="write an example's .out from what it prints, and show it")
    ap.add_argument("--emit-goals", action="store_true", help="write the loop chain")
    ap.add_argument("--out", default=str(GOALS_OUT), help="where --emit-goals writes")
    ap.add_argument("--per-goal", type=int, default=18, help="features per emitted goal")
    ap.add_argument("--all-groups", action="store_true",
                    help="with --emit-goals: include groups that owe nothing")
    ap.add_argument("--nvs", help="the binary to use (default: target/release, then target/debug)")
    args = ap.parse_args()

    nvs = binary(args.nvs)
    if nvs is None:
        print("dossier: no `nvs` binary. Build one (`cargo build --release -p nvs-cli`) or "
              "pass --nvs.")
        return 1

    if args.bless:
        return bless(nvs, [Path(p) for p in args.bless])
    if args.perf_report and not (args.gate or args.run or args.record_perf):
        return perf_report()

    policy, skips = load_policy(args.no_perf)
    columns = shown_proofs(policy)
    entries = roster(nvs)
    scope = entries
    if args.group:
        scope = [e for e in entries if e.group == args.group]
        if not scope:
            print(f"dossier: no group {args.group!r}.")
            return 1
    if args.only:
        wanted = set(args.only)
        scope = [e for e in scope if e.id in wanted]
        # A named feature that is no longer on the roster is a stale goal, not an empty scope: the
        # member was renamed or removed, and a check that silently narrowed to nothing would pass.
        unknown = sorted(wanted - {e.id for e in scope})
        if unknown:
            print(f"dossier: --only names {len(unknown)} feature(s) that are not on the roster -- "
                  f"re-run --emit-goals: {', '.join(unknown[:5])}")
            return 1
    proofs = collect(entries)

    if args.record_perf:
        rc = record_perf(nvs, scope, args.reps, args.note, proofs, policy, skips, args.force)
        return rc or (perf_report() if args.perf_report else 0)

    if args.verify:
        # One process for the gate and both suites, because every check written into a generated
        # goal is re-run by every session of every goal after it -- `goal_toml`'s comment has the
        # arithmetic. Short-circuiting is deliberate too: an owed proof is not a suite failure and
        # running the suites to say so again costs a sweep to learn nothing.
        rc = gate(scope, proofs, policy, skips, scope_label(args))
        if rc:
            return rc
        rc |= run_scoped(nvs, "examples", scope, args)
        rc |= run_scoped(nvs, "hostile", scope, args)
        return rc

    if args.run:
        rc = 0
        if args.run in ("examples", "all"):
            rc |= run_scoped(nvs, "examples", scope, args)
        if args.run in ("hostile", "all"):
            rc |= run_scoped(nvs, "hostile", scope, args)
        return rc

    if args.emit_goals:
        return emit_goals(entries, proofs, policy, skips, Path(args.out), args.per_goal,
                          not args.all_groups, args.no_perf)

    if args.gate:
        return gate(scope, proofs, policy, skips, scope_label(args))

    if args.json:
        print(json.dumps([{
            "id": e.id, "kind": e.kind, "group": e.group, "path": e.path, "anchor": e.anchor,
            "owed": owed(e, proofs[e.id], policy, skips),
            "have": {"tests": len(proofs[e.id].nvst) + len(proofs[e.id].rust),
                     "examples": len(proofs[e.id].examples),
                     "hostile": len(proofs[e.id].hostile),
                     "perf": bool(proofs[e.id].perf)},
        } for e in scope], indent=2))
        return 0

    if args.gaps:
        rows = [(e, f) for e in scope for f in proofs[e.id].gaps]
        print(f"== BUGS THE PROOFS FOUND  ({len(rows)} marked proof(s))")
        print("-- each is a real failure a session could not fix in the slice that found it, and is")
        print("-- recorded in the named crate's `# Known gaps`. Removing the marker is part of the")
        print("-- fix: a marked proof that passes fails the sweep.")
        print()
        for e, f in rows:
            gap = known_gap(read(ROOT / f))
            print(f"  {e.id:44} {f}")
            print(f"  {'':44}   -> {gap[0] if gap else '?'}: {gap[1] if gap else ''}")
        return 0

    if args.feature:
        return print_entry(args.feature, entries, proofs, policy, skips)
    if args.owed:
        print_owed(scope, proofs, policy, skips, args.limit or 10**9)
        return 0
    if args.group:
        print_group(args.group, entries, proofs, policy, skips, columns)
        return 0
    print_status(group_rows(entries, proofs, policy, skips), columns)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
