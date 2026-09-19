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
| `perf` | `benches/members/` + `docs/perf/members.ndjson` | four counts and one clock per feature: what the program did, the same on any machine, and how long it took on this one -- so a change is re-measured against **us**, never against PHP |
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

    python tools/dossier.py --partition --group G    cut one group into worker briefs, or refuse. See below
    python tools/dossier.py --brief '<feature>'      one feature's brief, as a worker is handed it
    python tools/dossier.py --findings [--clear]     what the workers hit, collated for one batch fix

    python tools/dossier.py --no-perf …              drop the perf proof entirely, for any command above
    python tools/dossier.py --emit-goals             append the roster to docs/agent/goals/,
                                                     with the goal files under docs/agent/goals/dossier/
    python tools/dossier.py --emit-goals --dry-run   ... and say what that would change, writing nothing

## The description, and the switch that makes it owed

Beside the four proofs every feature gets `about.md` in its example directory: the short plain
prose the website shows first when somebody looks the feature up. `docs/examples/README.md` §
*The description* owns what it is. The emitted goals write it -- it is the first thing in a
worker's brief for any feature that lacks one, and the examples are then written to deliver what
it promises -- and `--id` prints whether it is there.

**It is owed only where the policy says so**, and `POLICY` says no: `[all] about = true` in
`tools/data/dossier-policy.toml` is the switch. While it is off, no audit, gate or emission counts
a feature as incomplete for lacking one -- every owed feature already sits in a goal that writes
it, and counting it would re-open the complete ones for a file their own goal never asked for.
`--emit-goals` appends one closing goal after every goal it writes, and that goal is what turns
the switch on and closes whatever is left; its check is the whole roster's `--gate`, which every
goal after it carries as floor. From there a feature without its description is a red check, the
same as one without its test.

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
* **Perf is re-measured only where the implementation moved.** The `impl_hash` currency rule
  below is the whole mechanism: an untouched member is never re-timed, so a sweep after a change to
  one file measures that file's features. `--record-perf` measures only what has no current figure
  unless `--force`, so it is safe to run at the end of every slice.
* **A figure from any machine satisfies the gate.** A fresh clone on a new box owes nothing it
  already has a current record for -- the ledger travels with the repository, and re-taking a
  number to learn what the last machine already recorded proves nothing about the language. Only
  `--perf-report` insists on this machine's own records, because only a delta needs them.
* **`--run` remembers a green verdict against the bytes that produced it** — the program's own hash
  and the binary's — in `.loop/dossier-green.json`. Identical bytes into a deterministic run cannot
  reach a different verdict, which is the argument `verify.py` and `loop.py` both already make for
  their own caches. So a re-run with an unchanged binary costs the walk; a re-run after a rebuild
  costs the programs. `--no-cache` forces the long way, and a failure is never cached.

## Running one group's features at once

Three of the four proofs are attributed by **position** -- the example, attack and bench trees all
use the same relative path, derived from the feature's own id -- so two features' proofs cannot name
the same file. That, and not a hope, is why this work runs wide: `--partition` cuts a group into
worker briefs and **refuses** if any two workers would write the same path.

    python tools/dossier.py --partition --group 'Core\\Str'

writes one brief per worker under `.loop/dossier-fanout/` and prints the lanes. A brief is
self-contained: hand a worker its *path* and it reads it in its own window, so the parent's window
holds the table and nothing else. What a worker may not touch is in every brief and is the whole of
the safety argument -- **no `crates/`, no `git`, no `cargo`, no ledger, no policy file, no
`--record-perf`, no `--run hostile`** -- because each of those is either shared by the goal's whole
batch or has exactly one writer, and a second writer arriving in parallel fails silently.

The two things a worker hands back rather than writing are the Rust `#[test]` half of the `tests`
proof, which lands in the `mod tests` of an implementing file the goal's other features share,
and any bug a proof found. The parent splices the first in one `tools/splice.py --patch`,
collects the second with `--findings`, and fixes them as one batch. Then, and only after every
worker has stopped, it runs `--run all`, `--record-perf` -- a figure measured while eight workers
are running is not a measurement -- and `tools/verify.py`, and commits.

`FANOUT_WORKERS` below is the width and carries how it was derived. It is not `machine.jobs()`: a
worker waits on an API, not on a core.

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

## What a perf record holds, and which half of it travels

A record is **four counts and one clock**, taken in one sweep over the same bench program.

The counts -- statements executed, calls made, allocations, bytes -- come from `nvs run --count`,
which reads `rule:testing/debug-probes`'s probe sites in `rule:testing/bench-counters`'s counting
mode plus the allocator's own per-thread counters. They are the same on every machine and every
day for the same program and binary, so the report diffs them across any two records, and a bench
may **declare what it expects** -- `// bench: allocations 0`, `// bench: calls 1` -- which
`--record-perf` checks on the very first run, with no history to compare against. A count sees
what the program did and not how long it took: a `Core` member is one helper call however much
work it does inside, so a member that got slower without allocating is invisible to every count.

The clock is wall clock on the machine that took it, and **not comparable across machines**, which
is `rule:testing/perf-two-mechanisms`'s whole finding. Every record carries a machine fingerprint,
the fastest and the median of its reps, and a `ratio` against a calibration program measured in the
same sweep. Same fingerprint: the nanoseconds are the honest number, read against the spread the
median shows. Different machines: only the ratio travels, and only to about a tenth. The report
refuses to diff a clock across fingerprints rather than quietly printing a delta that means nothing.
The one wall-clock check that does travel is a **ratio within one run**: a bench that declares
`// bench: complexity constant` and has a sibling `<name>.scale.nvs` declaring `// bench: scale K`
is timed at both sizes seconds apart, and a constant-time member that costs K times more on K times
the input is wrong on any machine.

## Why a figure is current against the implementing file's text, not a commit

Measurement needs an idle machine and the acceptance test runs after every session, so a sweep that
re-times 500 features per session would measure the driver's own build more than the language. Every
record therefore carries `impl_hash` -- the implementing file's text with its trailing `mod tests`
cut off, hashed -- and the gate passes while that value still matches. Change the implementation and
its figure goes stale on the spot; add a test to the same file, or change anything else, and nothing
is re-measured. The text and not the commit, because the fan-out splices a goal's Rust tests into
the implementing file *before* the parent measures, and a figure keyed on the commit would go stale
at the wrap that commits them, every session re-owing what it had just taken. `impl_commit` is still
recorded, as where to look, and `binary` is the hash of the `nvs` that ran.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import fnmatch
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

import goals as goalsmod  # noqa: E402  (the chain's one reader; tools/ is not a package)
import machine  # noqa: E402  (tools/ is not a package; this is how every tool here imports a sibling)

BS = chr(92)  # a literal backslash, spelled so no layer of quoting can eat it

EXAMPLES = ROOT / "docs" / "examples"
HOSTILE = ROOT / "tests" / "hostile"
BENCHES = ROOT / "benches" / "members"
#: A feature's website description, inside its example directory. `collect()` globs that directory
#: for `*.nvs` alone, so this file never counts as an example.
ABOUT = "about.md"
LEDGER = ROOT / "docs" / "perf" / "members.ndjson"
PERF_REPORT = ROOT / "docs" / "perf" / "members.md"
CONFORMANCE = ROOT / "tests" / "conformance"
DIFFERENTIAL = ROOT / "tests" / "differential"
CRATES = ROOT / "crates"
LANG = ROOT / "docs" / "reference" / "lang"
TOOLCHAPTERS = ROOT / "docs" / "reference" / "tools"
POLICY_FILE = TOOLS / "data" / "dossier-policy.toml"
GOALS_OUT = ROOT / "docs" / "agent" / "goals" / "dossier"
#: The chain is the goals directory itself and `GOALS_OUT` is a subdirectory of it, which is what
#: makes an emission land *on* the chain rather than beside it: `tools/goals.py` walks the tree
#: under `docs/agent/goals/`, so a goal written here is a goal the driver walks.
CALIBRATION = BENCHES / "_calibration"
#: Green verdicts from `--run`, keyed on the bytes that produced them. Under `.loop/` with every
#: other run-time artefact, and gitignored with it.
GREEN = ROOT / ".loop" / "dossier-green.json"

#: Where `--partition` writes a worker's brief, and where a worker drops a finding. Under `.loop/`
#: beside `dossier-green.json` for the same reason: a brief restates what the roster already says
#: and is worthless the moment the roster moves, and a finding is state that lives until the batch
#: fix lands. Neither is ever committed.
FANOUT = ROOT / ".loop" / "dossier-fanout"
FINDINGS = ROOT / ".loop" / "dossier-findings"

#: No worker writes under one of these, and `partition()` refuses a lane that would. Every entry is
#: either shared by a goal's whole batch -- `crates/` holds the `mod tests` its features append
#: to -- or is a ledger with exactly one writer. A second writer arriving in
#: parallel is how both of those fail silently instead of loudly.
#:
#: `docs/decisions/` is the ledger a goal's one record lands in. `docs/adr/` holds only README.md's
#: project-start decisions and tooling-parity.md since the records moved (docs migration, unit C1),
#: and no lane has business writing either -- it stays reserved so a worker that still spells the
#: old path is refused rather than left writing a record where nothing reads it.
RESERVED = ("crates/", "tools/", "docs/perf/", "docs/agent/", "docs/adr/", "docs/decisions/", "fuzz/", ".loop/")

#: How many workers a fan-out runs. Deliberately not `machine.jobs()`: a worker is an agent waiting
#: on an API, not a process waiting on a core, and the only machine-bound thing it does is bless an
#: example's `.out` in milliseconds. The number comes from the **serial tail** instead -- what a
#: goal spends whatever its width -- which is 14.9 minutes per goal and 24 hours over the 98 of
#: them. Its parts, all but the last measured on 2026-09-05: 7.9 min of a session's 34 fixed head
#: and tail calls, 2.1 min of `verify.py`, 1.9 min launching the workers, 0.9 min of this tool's
#: own commands, 0.7 min of `--record-perf`, and the batch fix. The sweeps do not appear because
#: they are free (`--gate` 2.6s, seventeen examples in 0.1s).
#:
#: Against a tail that size, over the emitter's real goal sizes and a feature costing 16 calls,
#: width buys: 2 lanes 2.17x the serial program, 3 -> 2.60x, 4 -> 2.79x, 6 -> 3.09x, 8 -> 3.13x,
#: 12 -> 3.19x, 18 -> 3.14x. **Six is 79% of everything width can give and eight is 80%**, and 18
#: is slower than 12 because each lane costs the parent a launch call whether or not it shortens a
#: wave. Eight, and the tail is what to attack next -- more than half of it is the session's own
#: fixed cost, which only fewer, larger goals would touch, and `--per-goal` at 27 already puts the
#: parent's window over the 200k ceiling to save three hours. One input is an estimate and it is
#: the 16: no dossier goal has run yet. `--workers N` and `NVS_DOSSIER_WORKERS` override this.
FANOUT_WORKERS = 8

#: What each kind of feature owes. `tests` counts proofs from either side -- a `.nvst` case or a
#: Rust `#[test]` -- and `rust` is how many of them must be the Rust half; `examples` and `hostile`
#: are file counts; `perf` is a bench program plus a current ledger record; `about` is the
#: website description, off for every kind here and switched on for all of them at once by the
#: policy file's `[all] about = true`, which the emitter's closing goal writes. `comments` is a
#: key no kind carries here, which reads as off: `[all] comments = true` makes every program a
#: feature's proofs are made of owe the bounds `comment_problems` judges, and goal
#: `plain-comments` writes it after bringing the landed programs inside them.
POLICY = {
    "member":    {"tests": 2, "rust": 1, "examples": 3, "perf": True,  "hostile": 1, "about": False},
    "lang":      {"tests": 2, "rust": 0, "examples": 3, "perf": True,  "hostile": 1, "about": False},
    "exception": {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 1, "about": False},
    "enum":      {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 0, "about": False},
    "interface": {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 0, "about": False},
    "tool":      {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 1, "about": False},
    "directive": {"tests": 1, "rust": 0, "examples": 1, "perf": False, "hostile": 1, "about": False},
}

PROOFS = ("tests", "examples", "perf", "hostile", "about")

#: The words a description may run to. `docs/examples/README.md` § *The description* is where the
#: band is explained; this is where it is enforced once the description is owed.
ABOUT_WORDS = (40, 160)

#: `// covers: A, B` -- in a `.nvst`, a `.nvs`, or above a Rust `#[test]`. `#` is accepted so the
#: marker can sit in a TOML or a shell fixture too.
COVERS_RE = re.compile(r"(?://|#)\s*covers:\s*(.+)")
#: `// requires: unimplemented` -- the website's own skip marker, honoured unchanged.
UNIMPL_RE = re.compile(r"^(?://|#)\s*requires:\s*unimplemented", re.M)
#: `// bench: iterations 200000` inside a bench program.
ITER_RE = re.compile(r"(?://|#)\s*bench:\s*iterations\s+([0-9_]+)")
#: `// bench: allocations 0`, `// bench: calls 1`, `// bench: statements 3`, `// bench: bytes 0` --
#: what the bench declares it expects per operation, in `rule:testing/bench-counters`'s counts.
#: `--record-perf` checks each on every run, the first included: a count needs no history to be
#: judged. Met to within a hundredth per operation, so a one-off set-up allocation over hundreds of
#: thousands of iterations rounds away and a per-call one does not.
EXPECT_RE = re.compile(r"(?://|#)\s*bench:\s*(allocations|calls|statements|bytes)\s+([0-9_]+)")
#: `// bench: complexity constant` in a bench, and `// bench: scale 10` in its sibling
#: `<name>.scale.nvs`, whose input is that many times the bench's. The two are timed in the same
#: sweep and the ratio of their per-operation figures is the one wall-clock check that holds on any
#: machine, because both numbers came from the same one seconds apart.
COMPLEXITY_RE = re.compile(r"(?://|#)\s*bench:\s*complexity\s+(constant|linear)")
SCALE_RE = re.compile(r"(?://|#)\s*bench:\s*scale\s+([0-9._]+)")
#: The one line `nvs run --count` prints on stderr at exit.
COUNT_LINE_RE = re.compile(r"^count: statements=(\d+) calls=(\d+) allocations=(\d+) bytes=(\d+)",
                           re.M)
COUNTS = ("statements", "calls", "allocations", "bytes")
#: How far a scaling ratio may sit from what its declared complexity predicts before the record is
#: refused: order-of-magnitude, like `rule:testing/perf-two-mechanisms`'s per-PR guards, because a
#: same-run wall-clock ratio is honest to a factor and not to a percent.
SCALE_TOLERANCE = 3.0
#: The least a calibration iteration may cost before `calibrate` calls the measurement failed. Every
#: `units` figure in the ledger is a division by it, so a unit that did not measure anything does not
#: produce a bad record -- it produces one whose scale is meaningless, appended to a file nothing
#: rewrites. Both calibration runs are the *fastest* of their reps, so on a machine busy enough the
#: empty program's fastest run can land above the unit program's and the difference goes negative.
#: A tenth of a nanosecond is well under one clock cycle of any machine this runs on, so nothing that
#: really measured an iteration is ever refused by it.
MIN_UNIT_NS = 0.1
#: `[report]` in `tools/data/dossier-policy.toml`, and its defaults. `outlier_factor` is how far
#: above its group's median a member's `units` figure sits before the report lists it as a
#: candidate; `ceiling` is an absolute `units` figure per declared complexity, past which a member
#: is listed whatever its neighbours cost. Both are advisory lines in the report and never a gate.
REPORT = {"outlier_factor": 5.0, "ceiling": {}}
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


#: Every subprocess this file runs, decoded the same way and started with nothing to read. Not the
#: platform default decoding: `nvs meta --json` carries the em dashes its own reference cards are
#: written with, and cp1252 refuses them -- which arrives as a `NoneType` where the JSON was,
#: several frames from the cause.
#:
#: Input is closed for the reason `nvs-test`'s runner closes it: a prompt reads the controlling
#: terminal, and a process keeping one inherited from whoever started the sweep is interactive.
#: The example that asks a question would then reach one answer under the loop driver and another
#: under a person's shell -- where it would also stop the sweep dead, waiting five minutes to be
#: answered by somebody who is reading a progress line rather than a question.
CAPTURE = {"capture_output": True, "text": True, "encoding": "utf-8", "errors": "replace",
           "stdin": subprocess.DEVNULL}


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
    """One spelling for a path in this tree: repo-relative and posix, whatever the caller held.

    A path typed on the command line arrives relative, and `relative_to` refuses that against an
    absolute root -- which used to fall through to the Windows spelling, so the same file was
    printed two ways and, worse, *named* to the binary two ways by the runners below."""
    try:
        return path.resolve().relative_to(ROOT).as_posix()
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


def load_report_policy() -> dict:
    """`REPORT` with the `[report]` table of the policy file applied."""
    out = {"outlier_factor": REPORT["outlier_factor"], "ceiling": dict(REPORT["ceiling"])}
    if POLICY_FILE.exists():
        table = tomllib.loads(read(POLICY_FILE)).get("report", {})
        if "outlier_factor" in table:
            out["outlier_factor"] = float(table["outlier_factor"])
        out["ceiling"].update({k: float(v) for k, v in table.get("ceiling", {}).items()})
    return out


def shown_proofs(policy: dict) -> tuple[str, ...]:
    """The proofs any kind still owes -- the audit's columns, so a switched-off proof leaves no
    column reading as complete when nothing was ever asked of it."""
    return tuple(p for p in PROOFS
                 if p not in ("perf", "about") or any(k[p] for k in policy.values()))


def about_problem(path: Path) -> str:
    """What is wrong with a description's shape, or "" -- only what a script can judge. Whether it
    reads well is `docs/examples/README.md`'s to say and a person's to check."""
    text = read(path).strip()
    if text.startswith(("#", "---")):
        return "opens with a heading or front matter, and the page supplies the title"
    if "```" in text:
        return "carries a code block, and the examples are where code goes"
    words = len(text.split())
    if not ABOUT_WORDS[0] <= words <= ABOUT_WORDS[1]:
        return f"{words} words, outside {ABOUT_WORDS[0]}-{ABOUT_WORDS[1]}"
    return ""


#: What a script can judge of `docs/examples/README.md` § *How a comment is written*. That section
#: is the rule and says why; these are the bounds it states, and the words it names that a script
#: can match without catching an everyday use of them.
COMMENT_TOP_LINES = 4
COMMENT_STEP_LINES = 2
COMMENT_SENTENCE_WORDS = 25
COMMENT_DIRECTIVE_RE = re.compile(r"^//\s*(?:bench|hostile|covers|dossier|requires):")
COMMENT_INTERNAL_RE = re.compile(
    r"\b(?:shards?|refcounts?|reference counts?|lowering|the registry|allocators?|optimi[sz]ers?|"
    r"hoist(?:s|ed)?|longest[- ]match|single[- ]filler|stack frames?|ADR ?\d+|nvs[-_]\w+)\b|rule:\w",
    re.I)
#: The *Not* column of that section's table, and the idioms it names: this repository's own voice,
#: which a reader who looked a feature up has never met. `connection refused` is the one everyday
#: use any of them has.
COMMENT_HOUSE_RE = re.compile(
    r"\b(?:spellings?|answers|answered|hands?(?: \w+)? back|hands|handed|(?<!connection )refus\w+|"
    r"members?|bindings?|holds|rather than|further down|on purpose|at the edge|goes through|"
    r"earns?)\b", re.I)


def comment_problems(path: Path) -> list[str]:
    """What is wrong with the shape of a proof program's comments, as `line: what` -- only what a
    script can judge. Whether a comment reads plainly is a person's to check; a sentence too long
    to be plain, a block too long to be skimmed and a word only the implementation uses are not.

    A directive line is a tool's and is skipped, and so is an indented line, which is how a
    configuration example shows the block it is about."""
    blocks: list[tuple[int, list[str]]] = []
    current: list[str] = []
    for number, line in enumerate(read(path).splitlines(), 1):
        text = line.strip()
        if text.startswith("//") and not COMMENT_DIRECTIVE_RE.match(text):
            body = text[2:]
            if body.startswith("     ") or not body.strip():
                continue
            if not current:
                blocks.append((number, current))
            current.append(body.strip())
        elif not text.startswith("//"):
            current = []
    problems = []
    for index, (number, lines) in enumerate(blocks):
        top = index == 0 and number <= 3
        bound = COMMENT_TOP_LINES if top else COMMENT_STEP_LINES
        if len(lines) > bound:
            where = "the top comment" if top else "a comment above a step"
            problems.append(f"{number}: {where} is {len(lines)} lines, and {bound} is the bound")
        prose = " ".join(lines)
        if "—" in prose or " -- " in prose:
            problems.append(f"{number}: a dash joins two sentences; write two")
        for sentence in re.split(r"(?<=[.!?:])\s+", prose):
            words = len(sentence.split())
            if words > COMMENT_SENTENCE_WORDS:
                problems.append(f"{number}: a {words}-word sentence opening "
                                f"`{' '.join(sentence.split()[:5])} ...`; "
                                f"{COMMENT_SENTENCE_WORDS} is the bound")
        # What sits in backticks is a name the reader types, and is never judged as a word.
        prose = re.sub(r"`[^`]*`", "", prose)
        for word in sorted({m.group(0).lower() for m in COMMENT_INTERNAL_RE.finditer(prose)}):
            problems.append(f"{number}: `{word}` is the implementation's word, not the reader's")
        for word in sorted({m.group(0).lower() for m in COMMENT_HOUSE_RE.finditer(prose)}):
            problems.append(f"{number}: `{word}` is this repository's word; the rule's table has "
                            f"the plain one")
    return problems


def check_comments(targets: list[Path]) -> int:
    """`--comments`: judge the named programs, or every `.nvs` under a named directory. It reads
    and never runs anything, so a worker may call it on its own files during the fan-out."""
    files = sorted({f for t in targets for f in (t.rglob("*.nvs") if t.is_dir() else [t])})
    bad = 0
    for path in files:
        problems = comment_problems(path)
        if problems:
            bad += 1
            print(f"  {rel(path)}")
            for problem in problems:
                print(f"      {problem}")
    rule = "`How a comment is written` in docs/examples/README.md"
    if bad:
        print(f"dossier comments: {bad} of {len(files)} program(s) miss {rule}.")
        return 1
    print(f"dossier comments: {len(files)} program(s), each inside the bounds of {rule}.")
    return 0


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
    def about_file(self) -> Path:
        return self.examples_dir / ABOUT

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


#: `("Throwable", None)` in `nvs_hir::errors::TREE` and `("Comparable", &[...])` in
#: `nvs_hir::interfaces::RESERVED` -- the two rosters `nvs meta --json` reads off a tuple table.
#: A namespaced row is spelled raw or with its backslashes doubled, like a name const.
TUPLE_ROW_RE = re.compile(r'^\s*\((?:r"([^"]+)"|"((?:[^"\\]|\\.)+)"),', re.M)
#: `pub const FINISH_MARKER: &str = "Core\\Script\\Finished";` -- an exception `errors.rs` declares
#: as a const beside the tree rather than as a row of it.
CONST_ROW_RE = re.compile(r'const\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*&str\s*=\s*'
                          r'(?:r"([^"]+)"|"((?:[^"\\]|\\.)+)")\s*;')
#: `Directive { key: "cache.local", ... }` in `nvs_config::DIRECTIVES`.
DIRECTIVE_ROW_RE = re.compile(r'Directive\s*\{\s*key:\s*"([^"]+)"')
#: A `CoreEnum` literal, named inline or through a const `gaps.class_consts` resolves.
ENUM_RE = re.compile(r'CoreEnum\s*\{\s*name:\s*(?:r"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))')

TABLES = (
    ("exception", CRATES / "nvs-hir" / "src" / "errors.rs", TUPLE_ROW_RE),
    ("exception", CRATES / "nvs-hir" / "src" / "errors.rs", CONST_ROW_RE),
    ("interface", CRATES / "nvs-hir" / "src" / "interfaces.rs", TUPLE_ROW_RE),
    ("directive", CRATES / "nvs-config" / "src" / "directive.rs", DIRECTIVE_ROW_RE),
)


def table_anchors() -> dict[str, dict[str, str]]:
    """kind -> {name: `crates/…/file.rs:NN`} for every feature that is not a class member.

    An exception, an interface and a directive each come from one table the registry document
    is built off, so the anchor is that table's row; an enum is a `CoreEnum` literal in the
    stdlib file that owns it. Like `registry_anchors`, a convenience the roster stands without:
    the anchor seeds a goal's `[context] modules`, and a goal whose features resolve none opens
    the crate by hand in every session.
    """
    out: dict[str, dict[str, str]] = {kind: {} for kind, _, _ in TABLES}
    out["enum"] = {}
    try:
        for kind, path, pattern in TABLES:
            text = read(path)
            for m in pattern.finditer(text):
                name = m.group(1) if m.group(1) is not None else \
                    (m.group(2) or "").replace(BS + BS, BS)
                out[kind].setdefault(name, f"{rel(path)}:{text.count(chr(10), 0, m.start()) + 1}")
        import gaps  # noqa: PLC0415  (optional, as in `registry_anchors`)

        for path in sorted((CRATES / "nvs-stdlib" / "src").rglob("*.rs")):
            text = read(path)
            consts = gaps.class_consts(path, text)
            for m in ENUM_RE.finditer(text):
                name = m.group(1) or consts.get(m.group(2), "")
                if name:
                    out["enum"].setdefault(name, f"{rel(path)}:{text.count(chr(10), 0, m.start()) + 1}")
    except Exception:
        pass
    return out


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

    A fenced block is skipped whole, because a sample program is written in the language the chapter
    documents rather than in Markdown: `# also a line comment` inside one is a comment Novis accepts,
    and reading it as a heading invents a feature nothing ships and then owes it four proofs.
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
        fence = ""
        for line in body.split("\n"):
            line_no += 1
            marker = line.strip()[:3]
            if fence:
                if marker == fence:
                    fence = ""
                continue
            if marker in ("```", "~~~"):
                fence = marker
                continue
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
    tables = table_anchors()
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
                anchor=tables[kind].get(name, ""),
                summary=((item.get("doc") or {}).get("short", "") if isinstance(item, dict) else ""),
            ))

    for d in doc.get("directives", []):
        key = d["key"] if isinstance(d, dict) else str(d)
        out.append(Entry(
            id=f"directive:{key}",
            kind="directive",
            group="config:directives",
            path=f"config/{key.replace('.', '-')}",
            anchor=tables["directive"].get(key, ""),
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
    about: str = ""                 # the website description, when it is on disk
    about_problem: str = ""         # what is wrong with its shape, when something is
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

    **The gate reads records from any machine and the report's clock columns read only this
    one's**, and that split is the whole point. A figure taken on a colleague's Linux box against
    the same `impl_hash` is a measurement of the same code: the feature is documented, and re-taking
    it here would prove nothing about the language. Its counts are the same ones this box would
    take; its *clock* is not, so `--perf-report` never crosses a fingerprint on a clock column.
    Without this split a fresh clone owes 759 figures it already has, and the first thing anyone
    would do is turn the proof off.
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
        if e.about_file.exists():
            p.about = rel(e.about_file)
            p.about_problem = about_problem(e.about_file)
        for d in (e.examples_dir, e.hostile_dir):
            if d.is_dir():
                p.gaps += [rel(f) for f in sorted(d.glob("*.nvs")) if known_gap(read(f))]
        if e.bench_file.exists():
            p.bench = rel(e.bench_file)
        records = perf.get(e.id, [])
        mine = [r for r in records if r.get("machine") == me]
        p.perf = mine[-1] if mine else None
        current = e.impl_file and impl_hash(e.impl_file)
        fresh = [r for r in records if not current or r.get("impl_hash") == current]
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
    if want.get("about") and "about" not in skip:
        if not proofs.about:
            out["about"] = f"no description at {rel(entry.about_file)}"
        elif proofs.about_problem:
            out["about"] = f"{proofs.about}: {proofs.about_problem}"
    if want.get("comments") and "comments" not in skip:
        # Not a sixth proof and so not in `PROOFS`: it judges the programs the other proofs already
        # are. Off in `POLICY`, and `[all] comments = true` is what goal `plain-comments` writes
        # once every landed program is inside the bounds.
        programs = [*proofs.examples, *proofs.hostile]
        if proofs.bench:
            scale = proofs.bench.removesuffix(".nvs") + ".scale.nvs"
            programs += [proofs.bench, *([scale] if (ROOT / scale).exists() else [])]
        missed = [(f, comment_problems(ROOT / f)) for f in programs]
        missed = [(f, problems) for f, problems in missed if problems]
        if missed:
            first, problems = missed[0]
            out["comments"] = (f"{len(missed)} program(s) outside the plain-comment bounds, "
                               f"first {first} line {problems[0]}")
    if want["perf"] and "perf" not in skip:
        # `perf_any` and not `perf`: a figure taken on another machine against this same
        # implementation text documents the feature just as well, and a fresh clone that owed every
        # figure it already has is a proof nobody would keep switched on. `--record-perf` is how a
        # machine gets its own clock, and `--perf-report` is the only thing that insists on it.
        if not proofs.bench:
            out["perf"] = f"no bench at {rel(entry.bench_file)}"
        elif not proofs.perf_any:
            out["perf"] = ("never measured" if not entry.impl_file else
                           f"stale: {entry.impl_file} changed since it was last measured")
    return out


# --------------------------------------------------------------------------------- running


def jobs_for(count: int) -> int:
    """How many proof programs run at once. `machine.py` § *The policy* owns the number.

    **The memory cap that policy describes does not apply here yet, and an attack is the one kind
    of program it was written for.** `machine.width()` caps on free memory only when the profile
    carries both `mem_kb` and a `sample_rss_kb`, and the `local` context has neither on Windows:
    `local_probe()` reads them out of `/proc/meminfo`, and it takes no timed sample because its
    first caller's unit of work was a snippet that holds nothing. So a hostile sweep runs at half
    the cores with nothing bounding what those workers allocate -- and `tests/hostile/README.md`
    asks for input "sized to the memory of the machine rather than to the example".

    Harmless while the attack tree is a handful of files, and the thing to fix before it is 800:
    probe free memory on this context too, and give the sweep a `worker_kb` measured from the
    attacks themselves. `NVS_DOSSIER_JOBS` is the way out in the meantime. A fan-out worker never
    reaches this -- its brief forbids running an attack for exactly this reason -- so the parent
    running the sweep is the only caller that can hit it.
    """
    try:
        return machine.jobs("local", ceiling=count, envs=("NVS_DOSSIER_JOBS",))
    except Exception:
        return min(4, max(1, count))


def normalise(text: str) -> str:
    return text.replace("\r\n", "\n").rstrip()


def run_one_example(nvs: Path, path: Path) -> tuple[str, str]:
    """An example is named to the binary the way `bless` named it: repo-relative, posix, from the
    repository root. A program can print the path it was started with -- a log record carries the
    file it was written in -- so a sweep that passed an absolute Windows path would never match an
    output frozen from a relative one, and the same file would freeze differently on each host."""
    source = read(path)
    if UNIMPL_RE.search(source):
        return "skip", "marked `requires: unimplemented`"
    try:
        out = subprocess.run([str(nvs), "run", rel(path)], timeout=60, cwd=ROOT, **CAPTURE)
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

    **A suite with no files reports the same counts, all zero.** `--emit-goals` writes one `want`
    per group asking both suites for `0 failed`, and a group of enums owes no hostile program at
    all, so a sentence here instead of the counts would make that group's check unsatisfiable. Zero
    failures out of zero programs is what happened; a proof that *is* owed and missing is the gate's
    to refuse, not this suite's.
    """
    if not files:
        print(f"dossier {what}: 0 ok, 0 skipped, 0 known-gap, 0 failed "
              f"(no {what} on disk yet -- nothing to run)")
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


def time_program(nvs: Path, path: Path, reps: int) -> tuple[float, float]:
    """(the fastest, the median) of `reps` runs, in nanoseconds. The fastest is the figure: the
    floor is the signal and everything above it is the machine doing something else. The median
    rides beside it so a later delta can be read against the spread it was taken in."""
    spent: list[int] = []
    for _ in range(reps):
        started = time.perf_counter_ns()
        out = subprocess.run([str(nvs), "run", str(path)], timeout=600, **CAPTURE)
        spent.append(time.perf_counter_ns() - started)
        if out.returncode != 0:
            raise RuntimeError(f"{rel(path)} exited {out.returncode}: "
                               f"{(out.stderr.strip().splitlines() or [''])[0]}")
    spent.sort()
    return float(spent[0]), float(spent[len(spent) // 2])


def count_program(nvs: Path, path: Path) -> dict[str, int]:
    """`rule:testing/bench-counters`'s four totals for one run of `path`, off the line `nvs run
    --count` prints. One run, because the answer is the same every time."""
    out = subprocess.run([str(nvs), "run", "--count", str(path)], timeout=600, **CAPTURE)
    if out.returncode != 0:
        raise RuntimeError(f"{rel(path)} exited {out.returncode} under --count: "
                           f"{(out.stderr.strip().splitlines() or [''])[0]}")
    m = COUNT_LINE_RE.search(out.stderr)
    if not m:
        raise RuntimeError(f"{rel(path)}: `nvs run --count` printed no count line -- "
                           f"is {rel(nvs)} built from this tree?")
    return dict(zip(COUNTS, (int(g) for g in m.groups())))


def iterations_of(path: Path) -> int:
    m = ITER_RE.search(read(path))
    if not m:
        raise RuntimeError(f"{rel(path)} declares no `// bench: iterations N`")
    return int(m.group(1).replace("_", ""))


def expectations_of(path: Path) -> dict[str, int]:
    """What a bench declares per operation -- `{"allocations": 0, "calls": 1}` -- or nothing."""
    return {k: int(v.replace("_", "")) for k, v in EXPECT_RE.findall(read(path))}


def scale_sibling(path: Path) -> Path:
    """`benches/members/core/Str/length.scale.nvs` for `.../length.nvs`: the same bench over an
    input `// bench: scale K` times larger, with its own `// bench: iterations`."""
    return path.with_name(path.stem + ".scale.nvs")


def impl_hash(path: str) -> str:
    """What a perf figure is current against: the implementing file's text with its trailing
    `mod tests` cut off, hashed. The module doc § *Why a figure is current against the
    implementing file's text* is why a text and not a commit. Empty when there is no such file."""
    p = ROOT / path
    if not p.is_file():
        return ""
    text = read(p)
    m = re.search(r"#\[cfg\(test\)\]\s*mod tests\b", text)
    if m:
        text = text[:m.start()]
    return hashlib.sha1(text.encode("utf-8")).hexdigest()[:12]


def binary_hash(nvs: Path) -> str:
    """The `nvs` that ran, by content. Once per sweep, so the 80 MB read is nothing."""
    return hashlib.sha1(nvs.read_bytes()).hexdigest()[:12]


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
    floor, _ = time_program(nvs, baseline, reps)
    unit_total, _ = time_program(nvs, unit, reps)
    unit_ns = (unit_total - floor) / iterations_of(unit)
    if unit_ns < MIN_UNIT_NS:
        raise RuntimeError(
            f"the calibration did not measure anything -- the unit program's fastest run "
            f"({unit_total / 1e6:.1f} ms) is not enough above the empty program's "
            f"({floor / 1e6:.1f} ms) to price one iteration at {MIN_UNIT_NS} ns. "
            f"Something else on this machine is taking the CPU; re-run --record-perf when it is idle")
    return floor, unit_ns


def measure_one(nvs: Path, e: Entry, reps: int, floor: float, base_counts: dict[str, int]
                ) -> tuple[dict, list[str]]:
    """One feature's figures -- the clock, the four counts and the scaling ratio -- and every way
    they fall short of what its bench declared. A shortfall is a finding, not a number to record:
    `rule:testing/a-failing-proof-is-fixed-or-recorded` names the two answers, and a `known-gap`
    marker on the bench is the second."""
    iters = iterations_of(e.bench_file)
    total, median = time_program(nvs, e.bench_file, reps)
    ns_per_op = max(0.0, (total - floor) / iters)
    counts = count_program(nvs, e.bench_file)
    per_op = {k: round(max(0, counts[k] - base_counts[k]) / iters, 3) for k in COUNTS}
    fig = {
        "iterations": iters,
        "ns_per_op": round(ns_per_op, 3),
        "median_ns_per_op": round(max(0.0, (median - floor) / iters), 3),
        **per_op,
    }
    findings: list[str] = []
    expected = expectations_of(e.bench_file)
    if expected:
        fig["expected"] = expected
    for k, want in expected.items():
        if abs(per_op[k] - want) > 0.01:
            findings.append(f"declares `{k} {want}` per op and did {per_op[k]:.3f}")
    source = read(e.bench_file)
    m = COMPLEXITY_RE.search(source)
    if m:
        fig["complexity"] = m.group(1)
    sibling = scale_sibling(e.bench_file)
    if sibling.exists():
        k = SCALE_RE.search(read(sibling))
        if not k:
            raise RuntimeError(f"{rel(sibling)} declares no `// bench: scale K`")
        scale = float(k.group(1).replace("_", ""))
        scaled_total, _ = time_program(nvs, sibling, reps)
        scaled = max(0.0, (scaled_total - floor) / iterations_of(sibling))
        ratio = scaled / ns_per_op if ns_per_op > 0 else float("inf")
        fig.update({"scale": scale, "scale_ns_per_op": round(scaled, 3),
                    "scale_ratio": round(ratio, 3)})
        # An upper bound only. A linear member over a small input is dominated by its fixed
        # per-call cost and looks nearly constant, which is not a bug; growing *faster* than
        # declared is the finding.
        predicted = {"constant": 1.0, "linear": scale}.get(fig.get("complexity", ""))
        if predicted is not None and ratio > predicted * SCALE_TOLERANCE:
            findings.append(f"declares `complexity {fig['complexity']}` and costs {ratio:.1f}x "
                            f"per op on {scale:g}x the input, past the "
                            f"{predicted * SCALE_TOLERANCE:g}x that allows")
    return fig, findings


def record_perf(nvs: Path, entries: list[Entry], reps: int, note: str, proofs: dict[str, Proofs],
                policy: dict, skips: dict, force: bool) -> int:
    """Measure and append. **By default only what has no current figure**, which is what makes this
    safe to put at the end of a slice: a session that edited one file re-measures that file's
    features and nothing else, and running it twice costs a walk. `--force` re-measures everything
    in scope, for when the question is the machine rather than the code.

    A bench whose figures miss what it declared gets no record and fails the sweep, exactly as a
    failing example does -- unless it carries a `known-gap` marker, in which case the record is
    written with the findings in it and the sweep says so."""
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
    binary = binary_hash(nvs)
    try:
        floor, unit_ns = calibrate(nvs, reps)
        base_counts = count_program(nvs, CALIBRATION / "baseline.nvs")
    except RuntimeError as exc:
        print(f"dossier: {exc}")
        return 1
    print(f"dossier perf: {len(todo)} features, {reps} reps, unit = {unit_ns:.1f} ns/iteration "
          f"on {fp['cpu']} ({fp['id']}), binary {binary}")
    print(f"  {'feature':44} {'ns/op':>10} {'units':>9}  {'stmts':>7} {'calls':>7} {'allocs':>7} "
          f"{'bytes':>9}")
    lines = []
    failed = 0
    for e in sorted(todo, key=lambda x: x.id):
        try:
            fig, findings = measure_one(nvs, e, reps, floor, base_counts)
        except (RuntimeError, subprocess.SubprocessError) as exc:
            print(f"  FAIL  {e.id}: {exc}")
            return 1
        gap = known_gap(read(e.bench_file))
        if findings and not gap:
            failed += 1
            print(f"  FAIL  {e.id}: " + "; ".join(findings))
            print("        fix it, or mark the bench `// dossier: known-gap <module> -- why`; "
                  "the figure is not recorded until one of those lands.")
            continue
        rec = {
            "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "id": e.id,
            "kind": e.kind,
            "group": e.group,
            "commit": commit,
            "dirty": dirty,
            "binary": binary,
            "impl_commit": last_commit(e.impl_file) if e.impl_file else "",
            "impl_hash": impl_hash(e.impl_file) if e.impl_file else "",
            "machine": fp["id"],
            "cpu": fp["cpu"],
            "os": fp["os"],
            "cores": fp["cores"],
            "reps": reps,
            **fig,
            "unit_ns": round(unit_ns, 3),
            "ratio": round(fig["ns_per_op"] / unit_ns, 4),
            "note": note,
        }
        if findings:
            rec["known_gap"] = {"module": gap[0], "findings": findings}
        lines.append(json.dumps(rec))
        print(f"  {e.id:44} {fig['ns_per_op']:10.1f} {rec['ratio']:9.3f}  "
              f"{fig['statements']:7.2f} {fig['calls']:7.2f} {fig['allocations']:7.2f} "
              f"{fig['bytes']:9.1f}"
              + (f"   scale x{fig['scale']:g}: {fig['scale_ratio']:.2f}x" if "scale" in fig else "")
              + (f"   known-gap: {'; '.join(findings)}" if findings else ""))
    if lines:
        LEDGER.parent.mkdir(parents=True, exist_ok=True)
        with LEDGER.open("a", encoding="utf-8", newline="\n") as fh:
            fh.write("\n".join(lines) + "\n")
    print(f"dossier perf: {len(lines)} records appended to {rel(LEDGER)}"
          + (f", {failed} bench(es) missed what they declared and were not recorded" if failed
             else ""))
    return 1 if failed else 0


def count_delta(last: dict, prev: dict | None, key: str) -> str:
    """A count's change against the record before it, as a signed per-operation difference, or
    blank when there is no earlier count or it did not move. Any machine's record qualifies as the
    earlier one: a count is the same everywhere."""
    if prev is None or key not in prev or key not in last:
        return ""
    change = last[key] - prev[key]
    return "" if abs(change) < 0.0005 else f"{change:+.3f}"


def perf_report() -> int:
    """Regenerate `docs/perf/members.md` -- the ledger's front page, overwritten every time.

    Three parts, in the order they are worth reading. **What the program did**, one row per
    feature, machine-independent, with each count diffed against the record before it wherever
    that was taken. **Candidates**: the members whose `units` figure sits far above their group's,
    or above the ceiling their declared complexity carries in the policy file, or whose scaling
    ratio the tool refused -- advisory, a shortlist for a person with a profiler. Then **the
    clock**, per machine, newest reading against the one before it on that same machine, with the
    spread the median shows. A clock delta across fingerprints is never printed, because there is
    no honest one to print.
    """
    if not LEDGER.exists():
        print(f"dossier: {rel(LEDGER)} does not exist yet -- run --record-perf first.")
        return 1
    by_feature = ledger_records()
    if not by_feature:
        print(f"dossier: {rel(LEDGER)} holds no records -- run --record-perf first.")
        return 1
    history: dict[tuple[str, str], list[dict]] = {}
    for fid, recs in by_feature.items():
        for rec in recs:
            history.setdefault((rec.get("machine", ""), fid), []).append(rec)
    machines: dict[str, dict] = {}
    for (mid, _fid), recs in history.items():
        machines.setdefault(mid, recs[-1])
    report = load_report_policy()

    out = [
        "# Measured cost, feature by feature",
        "",
        "**Generated by `python tools/dossier.py --perf-report` — never edited.**",
        f"[`{rel(LEDGER)}`]({LEDGER.name}) is the append-only ledger this is the front page of;",
        "`tools/dossier.py` owns how a figure is taken and `benches/members/README.md` owns what a",
        "bench program is. `rule:testing/member-perf-ledger` is what the columns mean.",
        "",
        "Every figure is Novis against **itself**: there is no PHP column here and there never will",
        "be — [`benches/userland/`](../../benches/userland/README.md) owns the cross-engine",
        "comparison. The **counts** — statements, calls, allocations, bytes, each per operation",
        "with the empty program's share subtracted — are what the program did, and are the same on",
        "every machine for the same commit, so their `Δ` is against the previous record wherever it",
        "was taken. A `Core` member is one call however much it does inside, so a count sees what",
        "the program asked for and not what the member cost. The **clock** — `ns/op`, the fastest",
        "of the reps, with `median` beside it — is wall clock on the machine named in the heading,",
        "and its `Δ` is against the previous reading **on that machine** only; `units` divides it by",
        "the calibration program measured in the same sweep and travels to about a tenth",
        "(`rule:testing/perf-two-mechanisms`). A clock `Δ` inside the spread the median shows is",
        "marked `~`: it is the machine, not the code.",
        "",
        "## What the program did, per operation",
        "",
        "| Feature | statements | calls | allocations | bytes | Δ allocations | Δ bytes | Declares | Commit |",
        "|---|---:|---:|---:|---:|---:|---:|---|---|",
    ]
    counted = {fid: [r for r in recs if "allocations" in r] for fid, recs in by_feature.items()}
    for fid in sorted(counted):
        recs = counted[fid]
        if not recs:
            continue
        last, prev = recs[-1], (recs[-2] if len(recs) > 1 else None)
        declares = ", ".join(f"{k} {v}" for k, v in last.get("expected", {}).items())
        if last.get("complexity"):
            declares = ", ".join(x for x in (f"complexity {last['complexity']}", declares) if x)
        out.append(f"| `{fid}` | {last['statements']:.2f} | {last['calls']:.2f} | "
                   f"{last['allocations']:.2f} | {last['bytes']:.1f} | "
                   f"{count_delta(last, prev, 'allocations')} | {count_delta(last, prev, 'bytes')} "
                   f"| {declares} | {last.get('commit', '')} |")
    out.append("")

    # Candidates: read off this machine's latest clock per feature, group by group.
    me = fingerprint()["id"]
    latest: dict[str, dict] = {}
    for (mid, fid), recs in history.items():
        if mid == me:
            latest[fid] = recs[-1]
    candidates: list[str] = []
    groups: dict[str, list[dict]] = {}
    for rec in latest.values():
        groups.setdefault(rec.get("group", ""), []).append(rec)
    for group, recs in sorted(groups.items()):
        ratios = sorted(r.get("ratio", 0.0) for r in recs)
        median = ratios[len(ratios) // 2] if ratios else 0.0
        for rec in sorted(recs, key=lambda r: r.get("id", "")):
            why = []
            ratio = rec.get("ratio", 0.0)
            if len(recs) >= 3 and median > 0 and ratio > report["outlier_factor"] * median:
                why.append(f"{ratio / median:.1f}x its group's median of {median:.1f} units")
            ceiling = report["ceiling"].get(rec.get("complexity", ""))
            if ceiling is not None and ratio > ceiling:
                why.append(f"{ratio:.1f} units against a `{rec['complexity']}` ceiling of "
                           f"{ceiling:g}")
            if rec.get("known_gap"):
                why.append("recorded as known-gap: " + "; ".join(rec["known_gap"]["findings"]))
            if why:
                candidates.append(f"| `{rec['id']}` | {group} | {' — '.join(why)} |")
    out += [
        "## Candidates",
        "",
        f"Advisory, never a gate: a `units` figure more than {report['outlier_factor']:g}x its",
        "group's median on this machine, a figure past the ceiling `tools/data/dossier-policy.toml`",
        "sets for its declared complexity, or a bench recorded as `known-gap`. A row here is where",
        "a person with a profiler looks first; it is not a verdict.",
        "",
    ]
    if candidates:
        out += ["| Feature | Group | Why |", "|---|---|---|", *candidates]
    else:
        out.append("*(none on this machine's latest readings)*")
    out.append("")

    for mid, sample in sorted(machines.items()):
        rows = sorted(((fid, recs) for (m, fid), recs in history.items() if m == mid),
                      key=lambda r: r[0])
        out += [
            f"## {sample.get('cpu', 'unknown CPU')} · {sample.get('os', '?')} · "
            f"{sample.get('cores', '?')} cores  (`{mid}`)",
            "",
            "| Feature | ns/op | median | units | Δ | Scaling | Measured at | Implementation |",
            "|---|---:|---:|---:|---:|---|---|---|",
        ]
        for fid, recs in rows:
            last = recs[-1]
            prev = recs[-2] if len(recs) > 1 else None
            delta = ""
            if prev and prev.get("ns_per_op"):
                change = (last["ns_per_op"] - prev["ns_per_op"]) / prev["ns_per_op"] * 100
                delta = f"{change:+.1f}%"
                median = last.get("median_ns_per_op")
                if median and last["ns_per_op"] > 0:
                    spread = (median - last["ns_per_op"]) / last["ns_per_op"] * 100
                    if abs(change) <= spread:
                        delta += " ~"
            median_cell = (f"{last['median_ns_per_op']:.1f}" if "median_ns_per_op" in last
                           else "")
            scaling = (f"x{last['scale']:g} → {last['scale_ratio']:.2f}x" if "scale" in last
                       else "")
            out.append(f"| `{fid}` | {last.get('ns_per_op', 0):.1f} | {median_cell} | "
                       f"{last.get('ratio', 0):.3f} | {delta} | {scaling} | "
                       f"{last.get('commit', '')} | "
                       f"{last.get('impl_hash') or last.get('impl_commit', '')} |")
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


HEADINGS = {"tests": "tests", "examples": "exmpl", "perf": "perf", "hostile": "hostl",
            "about": "about"}


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
    print(f"   about     {p.about or 'none at ' + rel(match.about_file)}")
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
    counted = p.perf_any if p.perf_any and "allocations" in p.perf_any else \
        (p.perf if p.perf and "allocations" in p.perf else None)
    if counted:
        print(f"     per op: {counted['statements']:.2f} statements, {counted['calls']:.2f} calls, "
              f"{counted['allocations']:.2f} allocations, {counted['bytes']:.1f} bytes"
              + (f"  (declares {', '.join(f'{k} {v}' for k, v in counted['expected'].items())})"
                 if counted.get("expected") else ""))
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


# ------------------------------------------------------------------------------ fanning out


def owned_paths(entry: Entry) -> list[str]:
    """The paths one worker holding `entry` may write. There are no others.

    Three of the four proofs are attributed **by position**, so this is derived rather than
    declared, and two workers holding different features cannot name the same path. That is the
    entire argument for running this work wide, and `partition()` asserts it on every run instead
    of trusting this comment.

    The Rust half of the `tests` proof is deliberately absent. It lands in the `#[cfg(test)] mod
    tests` of the *implementing* file, and a goal is batched by implementing file, so its
    features want one file or a few of one crate -- a string-class goal wants
    `crates/nvs-stdlib/src/str.rs` and nothing else. It comes back as text and one hand splices
    the lot.
    """
    return [rel(entry.examples_dir), rel(entry.hostile_dir), rel(entry.bench_file)]


def worker_owed(missing: dict[str, str]) -> dict[str, str]:
    """The subset of what a feature owes that a *worker* can close.

    There is one exception and it is the perf proof. A figure goes stale when the implementing file
    moves, and what closes that is `--record-perf` in the parent on a quiet machine -- no file
    anybody writes. A feature owing nothing else therefore gets no lane: handing a worker a brief
    with no work in it also prices the split wrong, because the lane looks full."""
    out = dict(missing)
    if "perf" in out and not out["perf"].startswith("no bench"):
        del out["perf"]
    return out


def case_home(p: Proofs) -> str:
    """Where a new `.nvst` for this feature most likely belongs.

    The corpus is one case per file in a flat per-subsystem directory, so a new case is a new
    *file* and collides with nothing -- but its name is a sentence the worker writes, and two
    workers cannot check each other's unwritten name. Naming the directory its existing credited
    cases already sit in is what keeps the sentences apart in practice; the parent's `--gate` is
    the backstop, and this one race is the only thing the scheme does not close by construction.
    """
    dirs = [f.rsplit("/", 1)[0] for f in p.nvst if f.endswith(".nvst")]
    return max(set(dirs), key=dirs.count) if dirs else "tests/conformance/core"


WORKER_RULES = """\
## Stay in your lane

Other workers are writing proofs for other features in this same working tree **right now**. These
are not style rules: breaking one silently destroys their work or the parent's.

- **Create only the paths this brief names, for your own features.** Every one is derived from the
  feature's id, so no other worker can name it.
- **Never edit a file under `crates/`** -- not to add the Rust `#[test]`, not to fix a bug. Up to
  68 features share one implementing file and your neighbour is holding it.
- **Never run `git`, `cargo`, `tools/verify.py`, `--record-perf`, `--run`, or anything that writes
  `docs/perf/members.ndjson`, `tools/data/dossier-policy.toml` or `.loop/`.** The parent runs every
  one of those, once, after every worker has stopped. A benchmark measured while eight workers are
  running is not a measurement.
- **Do not commit.** The parent commits.
- **Do not run your hostile program.** It is written to exhaust the machine, and there are seven
  other workers on it. The parent runs the whole attack tree at a width it controls.
- **You may run your own example programs**, and only to create their `.out`:
  `python tools/dossier.py --bless <the .nvs you just wrote>`. Read what it prints -- a blessed
  output is a claim. Nothing else of yours executes.

## What each proof is

- **about** -- the feature's website description, and **the first thing you write**: working out
  how to say what the feature does in plain words is the understanding every other proof spends.
  One lead sentence, one or two short paragraphs, 40 to 160 words, no code. An `**In plain
  words:**` picture only where the honest explanation is technical. Where the prose is hard to
  follow without code, a closing `**The examples below**` sentence naming what they show -- and
  then your examples show exactly that, in that order. `docs/examples/README.md` § *The
  description* is the shape and carries two models.
- **example** -- three small, self-contained programs a reader learns from, each printing, each a
  *different* use, and the third the thing somebody actually does at work. No framework, no
  database, no socket. `docs/examples/README.md` is the shape.
- **every comment you write in a `.nvs` file** -- example, hostile and perf alike -- is read by
  somebody who looked the feature up, often not in their first language. **Do not write it in the
  voice of this brief or of any document in this repository**; write it the way a good manual
  does. Say what the line does, then what the result is, with the real value: "`as ?int` converts
  the value to a whole number. If that is not possible, the result is `null`." The subject is the
  code or "you", and code *returns, gives, prints, throws* -- it never *answers, hands back,
  refuses, asks* or *holds*. Say what happens, not what does not: no "rather than". Use the word a
  programmer knows -- cast, syntax, method, variable, returns -- and never this repository's word
  for it (spelling, member, binding). No idiom, no figure of speech, no ADR number, no crate name.
  Up to four lines at the top, one or two above a step. `docs/examples/README.md` § *How a comment
  is written* is the rule, with the word table and three before-and-after pairs; read it before
  your first file. `about.md` is written in the same plain English. **Then check every `.nvs` you wrote**,
  all of them in one call: `python tools/dossier.py --comments <file> <file> ...`. It reads and
  never runs, so it is safe here. It judges only length and vocabulary, so passing it is the
  floor and not the goal: a file is handed back when it passes *and* a beginner would follow it.
  Rewrite the sentence as two; never trim a word to get under a bound.
- **hostile** -- the file you write when you are trying to make the runtime come apart. It has no
  expected output: it passes if nothing panicked, aborted, hung or leaked. Throwing is a pass; a
  limit stopping it cleanly is a pass. Unbounded input, deep nesting, one element either side of a
  documented limit. `tests/hostile/README.md` is the contract.
- **perf** -- one program that measures this feature and nothing else, chaining its inputs so no
  optimiser can hoist the loop, declaring `// bench: iterations N`.
  `benches/members/README.md` is the shape. You write it; you do **not** measure it.
- **tests** -- a `.nvst` case carrying `// covers: <the feature id>` in its `--FILE--` block, in a
  NEW file under the directory this brief names, plus (for a `Core` member) a Rust `#[test]` you
  **hand back as text and do not write**.

## When a proof finds a bug

It will, and that is the program working. **Do not fix it and do not weaken the proof** -- not a
softened attack, not an `.out` re-blessed to whatever the binary now prints. Write one file to
`.loop/dossier-findings/<worker>-<feature-slug>.md` saying which feature, which proof, what you
expected, what happened, and where you think it lives. One file per finding, so no two workers ever
append to the same one. Then carry on with your next feature. The parent collects them with
`python tools/dossier.py --findings` and fixes them as one batch, because the fix lands in a crate
file your neighbour is waiting on.

## What to hand back

One block per feature, and nothing else -- no excerpts, no restated file contents:

    <feature id>
      wrote: <every path you created, one per line>
      rust:  <the whole `#[test]` fn with its `// covers:` line, or `none owed`>
      found: <the finding file you wrote, or `nothing`>
"""


def feature_block(i: int, e: Entry, missing: dict[str, str], p: Proofs, policy: dict) -> str:
    want = policy[e.kind]
    where = "documented at" if e.anchor.startswith("docs/") else "implemented at"
    lines = [f"### {i}. `{e.id}`   [{e.kind}]"]
    if e.summary:
        lines.append(f"{e.summary.strip()}")
    if e.anchor:
        lines.append(f"{where} `{e.anchor}`")
    if e.twin:
        lines.append(f"replaces PHP `{'`, `'.join(e.twin[:4])}` -- its oracle case goes in "
                     f"`tests/differential/` and needs no frozen output")
    lines.append("")
    if not p.about:
        lines.append(f"- **about** -- write `{rel(e.about_file)}` first; the examples build on it.")
    elif "about" in missing:
        lines.append(f"- **about** -- rewrite `{p.about}`: {p.about_problem}.")
    if "comments" in missing:
        lines.append(f"- **comments** -- {missing['comments']}. Rewrite the comments and no line "
                     f"of code; `--comments <file>` names every line.")
    if "examples" in missing:
        n = want["examples"] - len(p.examples)
        lines.append(f"- **examples** -- write {n}: "
                     f"`{rel(e.examples_dir)}/NN-slug.nvs`, and `--bless` each one.")
        if p.examples:
            lines.append(f"  Already there: {', '.join(f.rsplit('/', 1)[-1] for f in p.examples)}")
    if "hostile" in missing:
        n = want["hostile"] - len(p.hostile)
        lines.append(f"- **hostile** -- write {n}: `{rel(e.hostile_dir)}/NN-slug.nvs`. No `.out`.")
    if "perf" in missing:
        lines.append(f"- **perf** -- write `{rel(e.bench_file)}`. Write it only; the parent")
        lines.append("  measures the whole group at once, on a machine with nothing else on it.")
        lines.append("  Declare what it should count where you know it -- `// bench: allocations 0`")
        lines.append("  for a member that returns a scalar, `// bench: complexity constant` with a")
        lines.append("  `.scale.nvs` sibling -- so the first measurement can already be judged.")
    if "tests" in missing:
        need = max(0, want["tests"] - len(p.nvst) - len(p.rust))
        if need:
            lines.append(f"- **tests** -- write {need} NEW `.nvst` under `{case_home(p)}/`, named "
                         f"as a sentence, carrying `// covers: {e.id}`.")
        if want.get("rust") and not p.rust:
            lines.append(f"- **tests (Rust)** -- hand back one `#[test]` for the `mod tests` of")
            lines.append(f"  `{e.impl_file or 'its implementing file'}`, carrying "
                         f"`// covers: {e.id}`.")
            lines.append("  **Do not write it** -- other lanes' features want that same file, and "
                         "the parent splices them all in one patch.")
        if p.nvst:
            lines.append(f"  {len(p.nvst)} case(s) already credit it, e.g.")
            lines.append(f"  `{p.nvst[0]}` -- read one before adding")
            lines.append("  another, so yours pins something the corpus does not.")
    if p.gaps:
        lines.append(f"- **known gap** -- {len(p.gaps)} proof here already found a bug nobody has "
                     f"fixed: {', '.join(p.gaps)}")
    return "\n".join(lines) + "\n"


def worker_brief(n: int, total: int, label: str, lane: list[tuple[Entry, dict]],
                 proofs: dict[str, Proofs], policy: dict) -> str:
    head = [
        f"# Fan-out worker {n} of {total} -- {label}",
        "",
        f"You are writing `rule:testing/four-proofs`'s proofs for the {len(lane)} feature(s) below. **One feature at",
        "a time, all of its proofs together** -- never one proof across many features. The expensive",
        "thing is understanding what the feature does at its edges, and the example, the attack, the",
        "bench and the test all spend that same understanding.",
        "",
        f"`python tools/dossier.py --id '<feature>'` re-prints any of this. `--brief '<feature>'`",
        "prints one of the blocks below on its own.",
        "",
        WORKER_RULES,
        "## Your features",
        "",
        "",
    ]
    body = [feature_block(i, e, m, proofs[e.id], policy) for i, (e, m) in enumerate(lane, 1)]
    return "\n".join(head) + "\n".join(body)


def wrapped(ids: list[str], indent: str, width: int = 96) -> list[str]:
    """`ids` as few lines as they fit on. The parent reads this table and nothing else of the
    split, so one line per feature would charge its window forty lines to say what eight say."""
    out, line = [], indent
    for name in ids:
        piece = name + ("," if name != ids[-1] else "")
        if len(line) + len(piece) + 1 > width and line != indent:
            out.append(line)
            line = indent
        line += (" " if line != indent else "") + piece
    return out + ([line] if line.strip() else [])


def lanes_for(todo: list[tuple[Entry, dict]], workers: int) -> list[list[tuple[Entry, dict]]]:
    """Split the worklist into balanced lanes, heaviest feature first.

    Longest-processing-time-first, priced in proofs owed: a feature owing all four is four times a
    feature owing one, and a lane holding the four heaviest is what makes a fan-out wait on one
    worker while seven idle."""
    lanes: list[list[tuple[Entry, dict]]] = [[] for _ in range(workers)]
    load = [0] * workers
    for entry, missing in sorted(todo, key=lambda r: (-len(r[1]), r[0].id)):
        i = load.index(min(load))
        lanes[i].append((entry, missing))
        load[i] += len(missing)
    return [lane for lane in lanes if lane]


def partition(scope: list[Entry], proofs: dict[str, Proofs], policy: dict, skips: dict,
              workers: int, label: str, flags: str) -> int:
    """Cut a scope into worker briefs, or refuse to.

    The refusal is the point. Everything else here is arithmetic; what makes fanning this work out
    safe rather than hopeful is that two lanes are checked for a shared path before a brief is
    written, and that no lane may name anything under `RESERVED`. A partition that cannot be run
    wide writes nothing and says which two features collide."""
    todo: list[tuple[Entry, dict]] = []
    parent_only: list[Entry] = []
    for entry in scope:
        missing = owed(entry, proofs[entry.id], policy, skips)
        if not missing:
            continue
        mine = worker_owed(missing)
        (todo.append((entry, mine)) if mine else parent_only.append(entry))
    if not todo:
        print(f"dossier: nothing for a worker in {label}"
              + (f" -- {len(parent_only)} feature(s) owe a re-measurement only, which is "
                 f"`--record-perf` in the parent." if parent_only else " -- nothing owed."))
        return 0

    forced = workers or machine.override("NVS_DOSSIER_WORKERS") or FANOUT_WORKERS
    lanes = lanes_for(todo, max(1, min(forced, len(todo))))

    owner: dict[str, int] = {}
    refusals: list[str] = []
    for i, lane in enumerate(lanes):
        for entry, _ in lane:
            for path in owned_paths(entry):
                if path.startswith(RESERVED):
                    refusals.append(f"{entry.id} would write {path}, which is shared by the batch "
                                    f"or has exactly one writer")
                if owner.get(path, i) != i:
                    refusals.append(f"worker {owner[path] + 1} and worker {i + 1} would both write "
                                    f"{path} ({entry.id})")
                owner[path] = i
    if refusals:
        print(f"dossier: REFUSED -- this partition is not safe to run wide. "
              f"{len(refusals)} collision(s), nothing written:")
        for line in refusals[:20]:
            print(f"  {line}")
        if len(refusals) > 20:
            print(f"  ... and {len(refusals) - 20} more")
        print("  A collision here is a defect in `owned_paths()` or in the roster's paths, never "
              "in the lane split -- run these features serially until it is fixed.")
        return 1

    out = FANOUT / slugify(label)
    out.mkdir(parents=True, exist_ok=True)
    for stale in out.glob("w*.md"):
        stale.unlink()
    written = []
    for i, lane in enumerate(lanes, 1):
        path = out / f"w{i:02d}.md"
        path.write_text(worker_brief(i, len(lanes), label, lane, proofs, policy),
                        encoding="utf-8", newline="\n")
        written.append((path, lane))

    owed_total = sum(len(m) for _, m in todo)
    print(f"== FAN-OUT  {label}: {len(todo)} feature(s), {owed_total} proof(s) owed, "
          f"{len(lanes)} worker(s)")
    print(f"-- {len(owner)} owned path(s), no two workers share one. Hand each worker the BRIEF")
    print("-- path below and launch them in one message; each reads its own in its own window.")
    print()
    for path, lane in written:
        proofs_owed = sum(len(m) for _, m in lane)
        print(f"  {rel(path):40} {len(lane):>2} feature(s), {proofs_owed:>2} proof(s)")
        for line in wrapped([e.id for e, _ in lane], " " * 8):
            print(safe(line))
    if parent_only:
        print()
        print(f"  {len(parent_only)} feature(s) have no lane -- they owe a stale figure and nothing")
        print(f"  a worker writes. `--record-perf` in step 4 closes them: "
              f"{', '.join(safe(e.id) for e in parent_only[:4])}"
              + (f" and {len(parent_only) - 4} more" if len(parent_only) > 4 else ""))
    print()
    print("Then, in the parent and only after every worker has stopped, in this order:")
    print("  1. python tools/dossier.py --findings          # fix what they hit, as one batch")
    print("  2. python tools/splice.py --patch <file>       # every handed-back #[test], one call")
    print(safe(f"  3. python tools/dossier.py --run all {flags}".rstrip()))
    # Only where a figure is actually owed: `types:enum` and every other kind `POLICY` excuses
    # would send a session to measure a scope with no bench in it, and a step that does nothing is
    # a step the next session learns to skip.
    if any("perf" in m for _, m in todo) or parent_only:
        print(safe(f"  4. python tools/dossier.py --record-perf {flags}".rstrip()
                   + "   # nothing else running"))
    print("  5. python tools/verify.py, then the wrap. One commit per feature still.")
    return 0


def print_brief(fid: str, entries: list[Entry], proofs: dict[str, Proofs], policy: dict,
                skips: dict) -> int:
    match = next((e for e in entries if e.id == fid), None)
    if match is None:
        near = [e.id for e in entries if fid.lower() in e.id.lower()][:8]
        print(f"dossier: no feature {fid!r}." + (f" Near: {', '.join(near)}" if near else ""))
        return 1
    missing = owed(match, proofs[match.id], policy, skips)
    if not missing:
        print(f"dossier: {match.id} is complete -- nothing to brief.")
        return 0
    mine = worker_owed(missing)
    if not mine:
        print(f"dossier: {match.id} owes only a re-measurement "
              f"({missing['perf']}), which is `--record-perf` in the parent. Nothing to brief.")
        return 0
    print(safe(WORKER_RULES))
    print(safe(feature_block(1, match, mine, proofs[match.id], policy)))
    return 0


def print_findings(clear: bool) -> int:
    """What the workers hit, collated. One file per finding is what makes this safe to write while
    eight of them are running; collating is what makes it one batch to fix."""
    files = sorted(FINDINGS.glob("*.md")) if FINDINGS.is_dir() else []
    if not files:
        print(f"dossier: no findings -- {rel(FINDINGS)} is empty. Either every proof the workers "
              f"wrote passed, or none has been run yet.")
        return 0
    print(f"== WHAT THE WORKERS FOUND  ({len(files)} finding(s))")
    print("-- fix these as ONE batch: they cluster in the implementing files a goal's batch shares,")
    print("-- which is the reason no worker was allowed to touch them. Fixing, and then the")
    print("-- proof that found it, land in the same commit -- or the bug goes in that crate's")
    print("-- `# Known gaps` with a `// dossier: known-gap <file> -- <what breaks>` on the proof.")
    print()
    for path in files:
        print(f"-- {rel(path)}")
        for line in read(path).rstrip().split("\n"):
            print(f"   {safe(line)}")
        print()
    if clear:
        done = FINDINGS / "applied"
        done.mkdir(parents=True, exist_ok=True)
        for path in files:
            path.replace(done / path.name)
        print(f"dossier: moved {len(files)} finding(s) into {rel(done)}. Archived, not deleted -- "
              f"a fix that turns out to be wrong needs what was written.")
    return 0


# ------------------------------------------------------------------------------ goal writing


#: What every generated goal's `[context] rules` names. A rule id prints that rule's body whole and
#: a record number prints one title per rule the record created, so the two ids are the rules a
#: session writing proofs reads in full, and 0134 is the record that created them and the six
#: beside them. 0079 and 0063 are deliberately absent: each created twenty-four rules, and naming
#: them printed forty-eight titles about the test runner and the `Core` shape rules into every
#: session of every generated goal -- the one section of 0079 a bench needs is `adrs` below.
GOAL_RULES = ["testing/four-proofs", "testing/a-failing-proof-is-fixed-or-recorded",
              "0134", "0026", "0117", "0004", "0088"]
GOAL_SHAPES = [
    "A `.nvst` test case",
    "A feature's four proofs — an example, an attack, a bench, a `covers:` marker",
    "A commit message",
]
GOAL_PLAYBOOK = [
    "Tooling > the end",
    "Tooling > one call reads",
    "Tooling > a commit",
    "Tooling > a [context]*",
    "Tooling > another agent",
    "Writing a test case > a rule*",
    "Writing a test case > a -p*",
    "Running things > target/release/nvs.exe is",
]


#: How small a class must be to share a goal with a class of a different implementing file. A small
#: `Core` class nearly always has a file of its own, so merging only by shared file left it a goal
#: of one to three features: a session's whole fixed cost and the fan-out's serial tail, spent on a
#: width of two lanes. What a merge spends is the parent's window -- its manifest, its batch fix and
#: its spliced Rust tests reach several files of one crate rather than one -- which is why only
#: small classes merge, and only inside one crate under `crates/`.
MERGE_SMALL = 6


def goal_batches(entries: list[Entry], size: int) -> list[tuple[str, list[Entry]]]:
    """Cut the roster into goals, **by file set and never across a group**.

    A goal is a finite contained group of work whose `[context]` manifest is what keeps a session
    under the ceiling, so the split follows the files its sessions open: one class, a run of
    classes sharing an implementing file, or a run of classes of at most `MERGE_SMALL` features
    each from one crate. A class larger than `size` becomes several goals of even size numbered
    `(1/3)`, `(2/3)`, so no part is a remainder of a feature or two -- the file set is identical,
    so the manifests are too.
    """
    groups: dict[str, list[Entry]] = {}
    for e in entries:
        groups.setdefault(e.group, []).append(e)

    def file_set(members: list[Entry]) -> str:
        """The implementing file most of a group's features live in. Two adjacent groups merge
        when they share it -- ordering by name instead would put classes together because A
        precedes B, and a session would open two file sets to save one goal -- or when both are
        small and `crate()` puts them in one crate."""
        files = [e.impl_file for e in members if e.impl_file]
        return max(set(files), key=files.count) if files else ""

    def crate(members: list[Entry]) -> str:
        """The crate a group's implementing file sits in, or "" when that file is outside
        `crates/` -- a reference chapter, or no file at all -- which never merges across files."""
        parts = file_set(members).replace(BS, "/").split("/")
        return "/".join(parts[:2]) if parts[0] == "crates" and len(parts) > 2 else ""

    batches: list[tuple[str, list[Entry]]] = []
    pending: list[Entry] = []
    pending_names: list[str] = []
    pending_small = True

    def flush() -> None:
        nonlocal pending, pending_names, pending_small
        if pending:
            label = pending_names[0] if len(pending_names) == 1 else \
                f"{pending_names[0]} and {len(pending_names) - 1} more"
            batches.append((label, pending))
            pending, pending_names, pending_small = [], [], True

    ordered = sorted(groups.items(), key=lambda kv: (kv[1][0].kind, file_set(kv[1]), kv[0]))
    for name, members in ordered:
        members.sort(key=lambda e: e.id)
        if len(members) > size:
            flush()
            parts = -(-len(members) // size)
            step = -(-len(members) // parts)
            chunks = [members[i:i + step] for i in range(0, len(members), step)]
            for n, chunk in enumerate(chunks, 1):
                batches.append((f"{name} ({n}/{len(chunks)})", chunk))
            continue
        small = len(members) <= MERGE_SMALL
        joins = file_set(pending) == file_set(members) or (
            small and pending_small and crate(pending) != "" and crate(pending) == crate(members))
        if pending and (len(pending) + len(members) > size or not joins):
            flush()
        pending += members
        pending_names.append(name)
        pending_small = pending_small and small
    flush()
    return batches


def emit_goals(entries: list[Entry], proofs: dict[str, Proofs], policy: dict, skips: dict,
               out_dir: Path, size: int, skip_complete: bool, no_perf: bool,
               dry_run: bool = False) -> int:
    """Write one goal triple per batch into the goals directory, which is the chain.

    **Onto the one chain, always.** This used to be a flag, and without it the emitter wrote a
    standalone chain under `--out` -- one `tools/loop.py` never walks, since it walks
    `docs/agent/goals/` and nothing else. The whole point of this program is that a goal *on* the
    chain queues its workset behind itself and `Chain.refresh()` walks into it without a restart,
    and a second chain in a directory could only ever be a proposal somebody had to copy by hand.

    Regenerating is safe and is how the roster grows: a batch whose features are all complete is
    left out entirely (`--all-groups` keeps them), so a second run after a hundred sessions emits
    the goals that are *left*, not the ones that were. Two properties make that safe, and both are
    about a reference that has to stay good for the hundreds of sessions between emission and
    arrival:

    * **A goal's identity is its slug, never its position in this emission.** A later run -- with
      complete groups dropped -- puts the same group at a different batch index, so numbering off
      that index would hand an existing goal a number some other group already answers to.
      `chain_numbers()` is what keeps a slug on the number it was first given, and re-emitting then
      rewrites that goal's own three files in place.
    * **A goal already on the chain is never appended twice**, and the ones that are get numbered
      on from the chain's last goal, because the number is the position and a person says it out
      loud.
    * **A feature some goal on disk already gates on is never given a second goal**, whatever
      label the batch it falls into carries today -- `claimed_features` is the whole of it. A
      goal the run has walked is left as it is: its checks are the floor's now, and rewriting its
      files would hand a retired goal its acceptance list back.
    * **A goal that says `position: last` stays last.** `goals.pinned_tail` is the hand-written
      goals closing the chain on purpose; what is appended takes their numbers and they are
      renamed behind it first, by `chain.apply_renumber`, so no two goals ever share a number on
      disk. One the run has already reached is not moved: appending behind the live goal is the
      ordinary case again.

    `--dry-run` writes nothing at all and says what the emission would change, which is the form the
    emitting goal's own acceptance check takes: a check that appended the hundred goals itself
    would pass by doing the work it exists to judge.
    """
    todo = entries
    if skip_complete:
        todo = [e for e in entries if owed(e, proofs[e.id], policy, skips)]
    out_dir = out_dir.resolve()
    where = rel(out_dir)                      # posix, and relative to the repository if it is inside
    known, last = chain_numbers()
    # The closing goal is appended once, by the emission that finds the description still switched
    # off, and after every batch so it is the last thing that emission queues. A later emission
    # finds its slug on the chain and appends what it has after it, which is the right order too:
    # by then the switch is on and those goals gate on the description themselves.
    closing = goal_slug(CLOSING_LABEL)
    wants_closing = closing not in known and not any(k["about"] for k in policy.values())
    if not todo and not wants_closing:
        print(f"dossier: nothing owed -- {NOTHING_APPENDED}.")
        return 0
    batches = goal_batches(todo, size) if todo else []
    if not dry_run:
        out_dir.mkdir(parents=True, exist_ok=True)
    env = inherited_env()
    live = goalsmod.live()
    taken_groups, taken_ids = claimed_features(out_dir)

    # Which batches are written, before any is numbered: how many are new decides where the pinned
    # tail lands, and it has to be there before a new goal takes the number it held.
    planned = []
    for label, members in batches:
        slug = goal_slug(label)
        if slug in known:
            # A goal the run has walked is somebody's floor already: its `.toml` may be gone, and
            # writing it back would un-retire it. What its features still owe is the floor's to
            # report, and a re-owed feature that no goal on disk claims is appended below.
            if known[slug].retired or known[slug].num <= live:
                continue
        else:
            members = [e for e in members
                       if e.group not in taken_groups and e.id not in taken_ids]
            if not members:
                continue
        planned.append((label, slug, members))
    fresh = sum(1 for _, slug, _ in planned if slug not in known) + (1 if wants_closing else 0)

    chain = goalsmod.load()
    tail = [g for g in goalsmod.pinned_tail(chain) if g.num > live]
    last -= len(tail)
    if tail and fresh and not dry_run:
        import chain as chainmod  # here, not at the top: it imports `loop`, which no other path needs
        chainmod.apply_renumber(chain, {g.num: g.num + fresh for g in tail}, dry_run=False)
        known = chain_numbers()[0]

    written, owed_count = 0, 0
    for label, slug, members in planned:
        if slug in known:
            n = known[slug].num
        else:
            last += 1
            n = last
        written += 1
        owed_count += len(members)
        anchors = sorted({e.impl_file for e in members if e.impl_file})
        groups = sorted({e.group for e in members})
        # A group split across several goals gates on its own features and not on the class, or
        # every part of the split would wait for all of them and only the last could ever pass.
        split = bool(re.search(r"\(\d+/\d+\)$", label))
        if not dry_run:
            (out_dir / f"{n}-{slug}.md").write_text(
                goal_prose(n, label, members, proofs, policy, skips,
                           [e.id for e in members] if split else None),
                encoding="utf-8", newline="\n")
            (out_dir / f"{n}-{slug}.toml").write_text(
                goal_toml(n, label, groups, anchors, no_perf,
                          [e.id for e in members] if split else None, env),
                encoding="utf-8", newline="\n")
            (out_dir / f"{n}-{slug}.handoff.md").write_text(
                goal_handoff(label, members, proofs, policy, skips),
                encoding="utf-8", newline="\n")

    if wants_closing:
        written += 1
        last += 1
        if not dry_run:
            (out_dir / f"{last}-{closing}.md").write_text(
                closing_prose(last), encoding="utf-8", newline="\n")
            (out_dir / f"{last}-{closing}.toml").write_text(
                goal_toml(last, CLOSING_LABEL, [], CLOSING_MODULES, no_perf, None, env,
                          closing=True),
                encoding="utf-8", newline="\n")
            (out_dir / f"{last}-{closing}.handoff.md").write_text(
                closing_handoff(), encoding="utf-8", newline="\n")

    print(f"dossier: {'would write' if dry_run else 'wrote'} {written} goal(s) into "
          f"{where}/ ({owed_count} owed feature(s) across them, up to {size} per goal)")
    if not fresh:
        print(f"dossier: {NOTHING_APPENDED}.")
        return 0
    print(f"dossier: {'would append' if dry_run else 'appended'} {fresh} goal(s) as goals "
          f"{last - fresh + 1}-{last}"
          + ("." if dry_run else ". The running driver picks them up at its next switch."))
    for g in tail:
        print(f"dossier: goal `{g.slug}` says `position: last` and "
              f"{'would stay' if dry_run else 'stays'} behind them.")
    return 0


#: The line `--emit-goals` ends on when it has nothing to add, in both of the ways that happens --
#: nothing owed at all, or every owed feature already some goal's job. Goal `dossier`'s own
#: acceptance check reads this exact sentence off a `--dry-run`, so it is one string, here.
NOTHING_APPENDED = "the chain already names every one of them -- nothing appended"


def goal_slug(label: str) -> str:
    """A batch's slug, which is its identity on the chain: the filename stem after the number."""
    return slugify(label)[:60]


def claimed_features(out_dir: Path) -> tuple[set[str], set[str]]:
    """The groups and feature ids the generated goals already on disk gate on.

    Read off each goal's own `[[check]]` argv -- `--group G` claims the whole group, present and
    future members alike, and `--only` claims the ids it lists -- because that check is what makes
    a feature some goal's job. A batch is cut from what is owed *today*, and that moves with every
    session: one class in a merged batch going complete changes the batch's label, and the label
    is the slug, so a re-emission would otherwise append a second goal for features a goal on disk
    already owns. Filtering an unknown slug's members through this is what keeps `--dry-run`
    answering *nothing appended* for as long as that is true, which is what goal `dossier`'s check
    -- carried as the floor of every generated goal after it -- asks every session.
    """
    groups: set[str] = set()
    ids: set[str] = set()
    for path in out_dir.glob("*.toml"):
        try:
            spec = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError):
            continue
        for check in spec.get("check", []):
            argv = [str(a) for a in check.get("argv", [])]
            for i, arg in enumerate(argv):
                if arg == "--group" and i + 1 < len(argv):
                    groups.add(argv[i + 1])
                elif arg == "--only":
                    for value in argv[i + 1:]:
                        if value.startswith("--"):
                            break
                        ids.add(value)
    return groups, ids


def check_goals(out_dir: Path) -> int:
    """Every generated goal is one the driver can actually walk. Exit 1 naming what is not.

    A defect here is multiplied by however many goals the emission wrote, and the two that bite are
    both silent: a `[context]` selector `orient.py` cannot resolve costs a warning in every session
    of that goal and prints nothing, and a missing `[valgrind] skip` makes `goal-switch.py` refuse
    the switch outright -- which stops the run rather than degrading it.

    A `modules` pattern resolves the way `orient.py`'s map resolves it: against `git ls-files`, so
    a reference chapter -- where a `lang:` or `tools:` feature lives -- is a line on the map like a
    crate module is, and the one thing that warns is a pattern nothing in the repository matches.

    This is deliberately a *file* check and executes nothing, so it stays cheap enough to be the
    acceptance check of the goal that writes these.
    """
    tomls = sorted(out_dir.glob("*.toml"))
    if not tomls:
        print(f"dossier: no generated goal under {rel(out_dir)} -- nothing to check.")
        return 1
    tracked = [p for p in git("ls-files").split("\n") if p]
    findings: list[str] = []
    for path in tomls:
        def bad(what: str) -> None:
            findings.append(f"  {rel(path)}: {what}")
        text = path.read_text(encoding="utf-8")
        try:
            spec = tomllib.loads(text)
        except tomllib.TOMLDecodeError as e:
            bad(f"does not parse -- {e}")
            continue
        if GOAL_SWITCH_MARKER not in text:
            bad("has no goal-switch marker line, so the floor cannot be spliced into it")
        if "files" not in spec:
            bad("has no `files = [...]` for the previous goal's fixtures to be carried into")
        if "skip" not in (spec.get("valgrind") or {}):
            bad("has no `[valgrind] skip = [...]`, which goal-switch.py refuses outright")
        for pattern in (spec.get("context") or {}).get("modules", []):
            pat = str(pattern)
            if not any(fnmatch.fnmatch(p, pat) or fnmatch.fnmatch(p, pat.rstrip("/") + "/**")
                       for p in tracked):
                bad(f"[context] modules names {pat!r}, which matches no tracked file -- orient.py "
                    f"warns once a session and prints nothing for it")
        for suffix in (".md", ".handoff.md"):
            if not (path.parent / (path.stem + suffix)).is_file():
                bad(f"names no sibling {suffix}, which the chain entry points at")

    if findings:
        print(f"dossier: {len(findings)} finding(s) across {len(tomls)} generated goal(s). Each is "
              f"a defect in the emitter, not in the file -- fix `goal_toml` and re-emit:")
        for f in findings[:40]:
            print(f)
        if len(findings) > 40:
            print(f"  ... and {len(findings) - 40} more")
        return 1
    print(f"dossier: {len(tomls)} generated goal(s) checked -- every manifest resolves, every one "
          f"carries the keys goal-switch.py needs.")
    return 0


#: The tables a goal carries about the *environment* its checks run in rather than about its own
#: work. `goal-switch.py` unions `[valgrind] skip` and leaves the rest exactly as written, so a
#: generated goal that omits them does not inherit them -- it silently drops the containers the
#: floor's checks need and refuses the switch outright for want of a `skip` key.
ENV_TABLES = ("valgrind", "wsl", "docker")

#: `goal-switch.py`'s own marker, restated here because a generated goal that lacks it stops the run.
GOAL_SWITCH_MARKER = "# <<< goal-switch: floor checks are inserted below this line >>>"

#: What a goal gets when the chain's last entry declares no environment of its own: the key
#: `goal-switch.py` insists on, and nothing invented.
DEFAULT_ENV = "[valgrind]\nskip = []\n"


def render_env(spec: dict) -> str:
    """`[valgrind]`, `[wsl]` and `[docker]` as text, from a parsed goal TOML.

    Re-rendered rather than sliced out verbatim because a generated file has no comments of its own
    to preserve; the one comment that matters is the banner saying where these came from.
    """
    out = []
    for table in ENV_TABLES:
        body = spec.get(table)
        if not isinstance(body, dict) or not body:
            continue
        out.append(f"[{table}]")
        for key, value in body.items():
            if isinstance(value, list):
                out.append(f"{key} = [" + ", ".join(json.dumps(str(v)) for v in value) + "]")
            else:
                out.append(f"{key} = {json.dumps(str(value))}")
        out.append("")
    return "\n".join(out)


def inherited_env() -> str:
    """The environment tables of the last goal already on the chain.

    A generated goal runs the whole floor of every goal before it, so it needs the containers and
    the WSL target that floor was written against. Inheriting them from the goal it is appended
    behind is the only answer that stays right when the chain is emitted onto more than once: that
    goal is either the hand-written one that queued the emission or a generated one that inherited
    the same tables from it.
    """
    chain = goalsmod.load()
    chain = chain[:len(chain) - len(goalsmod.pinned_tail(chain))]
    try:
        last = chain[-1]
        prev = tomllib.loads(last.toml.read_text(encoding="utf-8"))
    except (OSError, IndexError, tomllib.TOMLDecodeError):
        return DEFAULT_ENV
    rendered = render_env(prev)
    if not rendered.strip():
        return DEFAULT_ENV
    return (f"# The environment tables of `{goalsmod.rel(last.toml)}`, the goal this one was\n"
            f"# appended behind: a generated goal runs that goal's whole acceptance list as its\n"
            f"# floor, so it needs the same containers and the same WSL target. Regenerated,\n"
            f"# never hand-edited.\n"
            + rendered)


def chain_numbers() -> tuple[dict[str, goalsmod.Goal], int]:
    """`{slug: goal}` for every goal on the chain, and the highest number on it.

    **A generated goal keeps the number it was first given.** A re-emission drops the groups that
    have gone complete, so a batch's position in *this* emission is not its position in the last
    one -- numbering off that would hand an existing goal somebody else's number and move a file
    hundreds of sessions of prose already cite. The slug is the identity; a slug already on the
    chain keeps its number and everything new is appended from the end.
    """
    chain = goalsmod.load()
    return {g.slug: g for g in chain}, max((g.num for g in chain), default=0)


def scope_flags(group: str | None, only: list[str] | None) -> str:
    """The `--group` and `--only` arguments naming one scope, so a command line printed for a
    session reaches exactly the features the command that printed it did."""
    flags = [f"--group '{group}'"] if group else []
    if only:
        flags.append("--only " + " ".join(f"'{i}'" for i in only))
    return " ".join(flags)


def partition_command(members: list[Entry], only: list[str] | None) -> str:
    """The `--partition` line for one goal, scoped exactly the way its own check is.

    `--group` alone reaches the wrong features in two shapes of goal, and both name theirs by
    `--only`. A part of a class larger than `--per-goal` keeps `--group` for the lane directory's
    name and narrows it with `--only`, or every part hands its workers the whole class. A goal
    holding several classes names no group at all: `--group` narrows before `--only` is read, so
    the first class's name would refuse every feature of the others."""
    if len({e.group for e in members}) > 1:
        return "python tools/dossier.py --partition " + scope_flags(None, [e.id for e in members])
    return "python tools/dossier.py --partition " + scope_flags(
        members[0].group if members else None, only)


def goal_prose(n: int, label: str, members: list[Entry], proofs: dict[str, Proofs], policy: dict,
               skips: dict, only: list[str] | None = None) -> str:
    # The front matter is the one fact a goal's files cannot derive: `plan.py --check` derives
    # every `Carried by` cell from it and refuses a goal without one, and `dossier` is a label
    # rather than a milestone id, which is the shape that tool accepts for work in no milestone.
    # The H1 is the one shape `chain.py` renumbers and `goals.py` reads a title off.
    lines = [
        "---",
        "milestone: dossier",
        "---",
        f"# Loop goal {n} — {label}",
        "",
        "**Generated by `python tools/dossier.py --emit-goals`.** The checks are the sibling",
        "`.toml`; this half is the target and the standing decisions. Regenerating overwrites both.",
        "",
        "## The target",
        "",
        f"Every feature listed below owes the four proofs of",
        "`rule:testing/four-proofs`: the behaviour tested",
        "Novis *and* from Rust, three small real-world examples, one measured performance figure,",
        "and one file written to break it. `python tools/dossier.py --id '<feature>'` prints what",
        "one feature has and what it still owes, with the path each proof belongs at.",
        "",
    ]
    if any(k["about"] for k in policy.values()):
        lines += [
            "**Every feature here also owes its website description**, `about.md` in its example",
            "directory, written before its examples. The check counts it like any other proof.",
            "",
        ]
    else:
        lines += [
            "**Every feature here also gets its website description**, `about.md` in its example",
            "directory, written before its examples. The check does not count it yet -- goal",
            f"`{goal_slug(CLOSING_LABEL)}` is where that is switched on -- so nothing but this",
            "paragraph and the worker's brief asks for it: a feature is not done until it has one.",
            "",
        ]
    lines += [
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
        "## Running this goal wide",
        "",
        "**This is one of the few goals where a session may hand *writing* to subagents.** The",
        "standing rule in `docs/agent/session-prompt.md` — a subagent searches and never writes —",
        "holds everywhere else, and the carve-out is this program and no other, because dossier work",
        "is the one shape that earns it: three of the four proofs are attributed by a path derived",
        "from the feature's own id, so two workers cannot name the same file; nothing here is a",
        "design decision; and `dossier.py --verify --group` judges the result mechanically.",
        "",
        "    " + partition_command(members, only),
        "",
        "writes one brief per worker under `.loop/dossier-fanout/` and **refuses** if any two would",
        "write the same path. Hand each worker its brief *path* — it reads it in its own window, so",
        "yours holds the table and nothing else — and launch them in one message so they run at",
        "once. A worker's own brief carries what it may not touch; you do not repeat it.",
        "",
        "Then, in this session and only after every worker has stopped, in this order:",
        "",
        "1. `python tools/dossier.py --findings` — what they hit. Fix it as **one batch**, because",
        "   the fixes cluster in the implementing files this goal shares.",
        "2. Splice every Rust `#[test]` they handed back into its `mod tests`, in one",
        "   `python tools/splice.py --patch`. No worker writes under `crates/` for exactly this",
        "   reason: this goal's features share their implementing files.",
        "3. `--run all` and then `--record-perf`, over that same scope — the `--partition` run",
        "   prints both lines back with the scope already in them. A figure taken while eight",
        "   workers are running is not a measurement, so nothing else may be in flight.",
        "5. `python tools/verify.py`, then the wrap. One commit per feature still.",
        "",
        "Nothing in that list is optional, and none of it may overlap the fan-out.",
        "",
        "## Standing decisions",
        "",
        "Settled before the run; a session decides and records, and never reports `BLOCKED` for any",
        "of these.",
        "",
        "- **The description comes first, and the examples answer it.** `about.md` is what the",
        "  website shows first when somebody looks the feature up: plain prose a beginner and an",
        "  expert read the same way, 40 to 160 words, no code, an `**In plain words:**` picture only",
        "  where the explanation is technical. When it is hard to follow without code it closes by",
        "  naming what the examples show, and the examples then show exactly that.",
        "  `docs/examples/README.md` § *The description* is the shape.",
        "- **An example is written for a reader, not for a test.** Small, self-contained, three",
        "  per member, each a *different* use, and the third is the one that earns its place: make",
        "  it the thing somebody actually does with this feature at work.",
        "- **A comment in any proof's `.nvs` file is written like a good manual, not like this",
        "  file.** Example, attack and bench alike are read by somebody who looked the feature up,",
        "  often not in their first language. Say what the line does and then what the result is;",
        "  the subject is the code or \"you\"; code *returns* and *throws*, it never *answers*,",
        "  *hands back* or *refuses*; say what happens and not what does not; use the word a",
        "  programmer knows (cast, syntax, method, variable) and not this repository's word for it",
        "  (spelling, member, binding); no idiom, no ADR number, no crate name. Up to four lines at",
        "  the top, one or two above a step. `about.md` is written in the same plain English.",
        "  `docs/examples/README.md` § *How a comment is written* is the rule, and",
        "  `python tools/dossier.py --comments <paths>` judges the half of it a script can: run it",
        "  over every `.nvs` this session wrote before the wrap, and a file it names is rewritten,",
        "  not trimmed. A file you touch for another reason is brought up to it; the landed ones",
        "  are goal `plain-comments`'s to sweep, behind every generated goal, and that goal is",
        "  also where the check becomes a gate.",
        "- **A `.out` file is created with `--bless` and then read.** Blessing is how the expected",
        "  output is *created*; a red example is never made green by re-blessing it.",
        "- **A hostile case has no expected output.** Its whole assertion is that the runtime",
        "  survived: no panic, no abort, no hang, no definite leak. Write the attack an attacker",
        "  would write — unbounded input, deep nesting, an allocation the program does not free, a",
        "  boundary crossed by one, a value at the far end of a range.",
        "- **A bench program measures the feature and nothing else**, chains its inputs so no",
        "  optimiser can hoist the loop, declares `// bench: iterations N`, and declares what it",
        "  expects to count where that is known — `// bench: allocations 0` for a member that",
        "  returns a scalar — so its first measurement is judged rather than merely recorded.",
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


#: The goal `--emit-goals` appends after every goal it writes. Its label is its slug, and the slug
#: is what keeps it from being appended twice.
CLOSING_LABEL = "the description is owed"
CLOSING_MODULES = ["docs/examples/README.md", "tools/data/dossier-policy.toml", "tools/dossier.py"]


def closing_prose(n: int) -> str:
    low, high = ABOUT_WORDS
    lines = [
        "---",
        "milestone: dossier",
        "---",
        f"# Loop goal {n} — {CLOSING_LABEL}",
        "",
        "**Generated by `python tools/dossier.py --emit-goals`.** The checks are the sibling",
        "`.toml`; this half is the target and the standing decisions. Regenerating overwrites both.",
        "",
        "## The target",
        "",
        "Every goal before this one wrote each feature's website description -- `about.md` in its",
        "example directory -- without any check counting it. **This goal makes it owed**, for every",
        "kind of feature, and closes whatever the goals before it left. When it is reached, a",
        "feature without its description is a red check for every goal that follows, the same as",
        "a feature without its test, and `rule:testing/four-proofs`'s five artefacts are what",
        "finished means in this repository.",
        "",
        "## The item list",
        "",
        "One file set -- the policy file and the example tree -- so this is one group.",
        "",
        "- [ ] **Switch it on.** Add `about = true` under `[all]` in",
        "      `tools/data/dossier-policy.toml`, creating the table if the file has none. Nothing",
        "      in `tools/dossier.py` changes: `POLICY` stays the default and the file is the",
        "      repository's durable answer, exactly as it is for `perf`.",
        "- [ ] **Read what is left.** `python tools/dossier.py --owed` now lists every feature",
        f"      with no description, and every description outside {low} to {high} words, opening",
        "      with a heading, or carrying a code block.",
        "- [ ] **Close it, one feature at a time.** Read the feature's examples first -- they are",
        "      already on disk here, so the description is written to fit them, and where it",
        "      closes with `**The examples below**` it names what they really show, in their",
        "      order. `python tools/dossier.py --partition --group <G>` fans a large group out.",
        "",
        "## Standing decisions",
        "",
        "- **`docs/examples/README.md` § *The description* is the standard**, and it is not",
        "  reopened here: plain prose a beginner and an expert read the same way, no code, an",
        "  `**In plain words:**` picture only where the explanation is technical.",
        "- **A description that fails the shape check is rewritten, never padded or trimmed to",
        "  fit.** Too long means it is explaining edge cases the tests own; too short means the",
        "  lead sentence is carrying the whole page.",
        "- **A feature that cannot carry a description does not exist.** Every feature has a page",
        "  on the website, so there is no `[skip]` entry for `about`.",
        "- **No numbered ADR is opened by this goal.** The decision is `rule:testing/four-proofs`.",
        "",
    ]
    return "\n".join(lines) + "\n"


def closing_handoff() -> str:
    lines = [
        "# Handoff",
        "",
        f"**Generated by `python tools/dossier.py --emit-goals` for goal `{goal_slug(CLOSING_LABEL)}`.",
        "The first session of this goal overwrites it like any other handoff.**",
        "",
        "## State",
        "",
        "This goal has just been installed. The description is not owed yet.",
        "",
        "## Next group",
        "",
        "- [ ] Add `about = true` under `[all]` in `tools/data/dossier-policy.toml`.",
        "- [ ] `python tools/dossier.py --owed`, and write or rewrite each description it names.",
        "",
        "## Backlog",
        "",
        "- (nothing yet)",
        "",
    ]
    return "\n".join(lines) + "\n"


def goal_toml(n: int, label: str, groups: list[str], anchors: list[str], no_perf: bool,
              only: list[str] | None, env: str = "", closing: bool = False) -> str:
    def toml_str(s: str) -> str:
        return "'" + s + "'" if BS in s else '"' + s + '"'

    #: The gate the loop runs carries the same switch the sweep that wrote these goals did.
    #: Otherwise a run started with the perf proof off would be judged by a gate that wants it.
    perf_flag = ', "--no-perf"' if no_perf else ""

    lines = [
        f"# Goal {n} -- {label}. The acceptance test, as data.",
        "#",
        "# GENERATED by `python tools/dossier.py --emit-goals` -- do not hand-edit; re-run it.",
        "# The sibling `.md` is the prose. One home each.",
        "",
        "files = [",
        "  # The fixtures this goal's own checks name. goal-switch.py unions the previous goal's",
        "  # list into this one, so an empty list is fine -- the key is what has to be here, and",
        "  # written across lines so the carried entries land in it rather than beside it.",
        "]",
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
    ]
    lines += env.splitlines()
    lines += [
        "",
        "# <<< goal-switch: floor checks are inserted below this line >>>",
        "",
        "",
    ]
    if closing:
        # The proofs the gate names appear in `PROOFS` order, so `hostile, about` is in its answer
        # exactly when the description is owed -- with the perf proof on or off. A roster that is
        # merely complete with the switch still off does not pass.
        lines += [
            "# ---------------------------------------------------------------------------------------",
            "# Stage 2 -- the description is owed by every kind of feature, and nothing on the whole",
            "# roster owes anything. The gate executes nothing, so carrying this as floor costs a walk.",
            "# ---------------------------------------------------------------------------------------",
            "",
            "[[check]]",
            'kind = "command"',
            'stage = "2 the description is owed"',
            'name = "dossier: the description is owed and the whole roster owes nothing"',
            f'argv = ["python", "tools/dossier.py", "--gate"{perf_flag}]',
            'want = ["nothing owed in the whole roster", "hostile, about"]',
            "",
        ]
        return "\n".join(lines) + "\n"
    lines += [
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


def goal_handoff(label: str, members: list[Entry], proofs: dict[str, Proofs], policy: dict,
                 skips: dict) -> str:
    # The three headings are the ones `orient.py` reads the pack's item off and `session.py --wrap`
    # requires of the handoff a session writes back; the goal names itself by slug, never by
    # number, because the number is a position and this file is copied out of the chain's
    # directory when the goal is installed.
    first = [e for e in members if owed(e, proofs[e.id], policy, skips)][:3]
    lines = [
        "# Handoff",
        "",
        f"**Generated by `python tools/dossier.py --emit-goals` for goal `{goal_slug(label)}` —",
        f"{label}. The first session of this goal overwrites it like any other handoff.**",
        "",
        "## State",
        "",
        f"This goal has just been installed. Nothing in {label} has been taken yet.",
        "",
        "## Next group",
        "",
        "One slice is one feature with all four proofs. Take them in this order — the list runs in",
        "file order, so neighbours share an implementing file and the second and third cost a",
        "fraction of the first:",
        "",
    ]
    for e in first:
        missing = owed(e, proofs[e.id], policy, skips)
        lines.append(f"- [ ] **`{e.id}`** — owes {', '.join(sorted(missing))}."
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
    plain = ", plain comments" if any(k.get("comments") for k in policy.values()) else ""
    print(f"dossier gate: nothing owed in {where} "
          f"({len(scope)} features, each owing {', '.join(shown_proofs(policy))}{plain}).")
    return 0


def run_scoped(nvs: Path, what: str, scope: list[Entry], args) -> int:
    """Every case in scope, and nothing that is not a case.

    A case is a `*.nvs` sitting directly in a feature's own directory -- the same set
    `owed` counts, so the gate and the suite never disagree about what exists. A `.nvs`
    below that, in a subdirectory, is material one of those cases `require`s or autoloads;
    it does not stand on its own, has no `.out` and does not compile alone. Walking the
    whole tree instead runs a fragment as though it were a case and fails it for being
    one -- `docs/examples/README.md` is where a supporting file's place is decided.
    """
    dirs = [e.examples_dir if what == "examples" else e.hostile_dir for e in scope]
    files = sorted({f for d in dirs if d.is_dir() for f in d.glob("*.nvs")})
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
        out = subprocess.run([str(nvs), "run", rel(path)], timeout=120, cwd=ROOT, **CAPTURE)
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
    ap.add_argument("--comments", nargs="+", metavar="PATH",
                    help="judge the comments of these .nvs programs, or of every one under a "
                         "directory, against the plain-comment bounds. Reads, never runs")
    ap.add_argument("--partition", action="store_true",
                    help="cut the scope into worker briefs under .loop/dossier-fanout/, or refuse "
                         "naming the two workers that would write the same path")
    ap.add_argument("--workers", type=int, default=0,
                    help="with --partition: how many lanes (default FANOUT_WORKERS, see --help)")
    ap.add_argument("--brief", metavar="ID",
                    help="one feature's brief and the worker rules, as a worker is handed them")
    ap.add_argument("--findings", action="store_true",
                    help="what the workers hit, collated for one batch fix")
    ap.add_argument("--clear", action="store_true",
                    help="with --findings: archive them under applied/ once the fix has landed")
    ap.add_argument("--emit-goals", action="store_true",
                    help="write one goal per group and append them to docs/agent/goals/, "
                         "so a run already walking it continues into them. Idempotent -- an entry "
                         "the chain already names is left alone")
    ap.add_argument("--out", default=str(GOALS_OUT),
                    help="where --emit-goals writes the goal files the chain then points at")
    ap.add_argument("--dry-run", action="store_true",
                    help="with --emit-goals: write nothing, and say what the emission would change")
    ap.add_argument("--check-goals", action="store_true",
                    help="every goal under --out is one the driver can walk: the manifest resolves, "
                         "the marker is there, the keys goal-switch.py needs are there")
    ap.add_argument("--per-goal", type=int, default=18, help="features per emitted goal")
    ap.add_argument("--all-groups", action="store_true",
                    help="with --emit-goals: include groups that owe nothing")
    ap.add_argument("--nvs", help="the binary to use (default: target/release, then target/debug)")
    args = ap.parse_args()

    # Before the binary is resolved, because this one reads files and executes nothing -- a tree
    # with no build still owes an answer about whether its generated goals are walkable.
    if args.check_goals:
        return check_goals(Path(args.out).resolve())
    # Reads one directory and executes nothing, like --check-goals: the parent asks this in the
    # middle of a fan-out, when a build may not have happened for an hour.
    if args.findings:
        return print_findings(args.clear)
    if args.comments:
        return check_comments([ROOT / p if not Path(p).is_absolute() else Path(p)
                               for p in args.comments])

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

    if args.brief:
        return print_brief(args.brief, entries, proofs, policy, skips)
    if args.partition:
        return partition(scope, proofs, policy, skips, args.workers,
                         scope_label(args) or "the whole roster",
                         scope_flags(args.group, args.only))

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
                          not args.all_groups, args.no_perf, args.dry_run)

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
