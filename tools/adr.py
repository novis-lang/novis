#!/usr/bin/env python3
"""Write, amend and audit the ADR set.

The ADRs are the project's one home for every settled decision, and they are read far more
often than they are written -- by an agent that has one orientation call to spend. This tool is
what keeps that set mechanically honest: it answers the questions a human cleanup pass used to
answer by reading ninety files, and it answers them the same way every time.

    python tools/adr.py                 the full audit, grouped by check, exit non-zero on an error
    python tools/adr.py --stats         one line per ADR: size, section shape, trim candidates
    python tools/adr.py --graph 0077    what one ADR amends, is amended by, cites, and is cited by
    python tools/adr.py --orphans       ADRs nothing links to, and ADRs missing from the two indexes
    python tools/adr.py --index         README.md's index table, derived from the ADR titles
    python tools/adr.py --residue       changelog/overlay prose an ADR body should not carry
    python tools/adr.py --check         audit, quiet on success -- the CI shape

WRITING ONE

README.md § *Adding a decision* is seven steps, and six of them are a form: claim the number nobody
else took, derive the slug, date it, fold the back-link into the ADR you amend, add a routing row,
add a ground-rules bullet, regenerate the index, re-audit. The seventh -- the ADR's own prose -- is
the only one worth a turn. So write that, and let this do the rest:

    python tools/adr.py --draft > .agent-tmp/adr.md     a draft to fill in
    python tools/adr.py --new .agent-tmp/adr.md         claim a number and index it everywhere
    python tools/adr.py --new FILE --dry-run            say what it would touch, write nothing

The draft *is* the ADR, in the shape conventions.md § *An ADR* already describes, with `NNNN` where
the number goes -- there is no second format to learn. Four extra fields are consumed and never
written to the file: `Slug:` overrides the derived filename, and `Route:`, `Rule:` and
`Divergence:` are the rows this adds to README.md's routing table, ground-rules.md and
divergences.md. `NNNN` anywhere in the draft expands to the number it claims, which is what lets
the routing row link to the file being created.

    python tools/adr.py --fold 0110 --into "[0033](0033-...md) § 4 — what changed there"
    python tools/adr.py --set-status 0033 Superseded
    python tools/adr.py --next-section 0007

`--fold` writes **both** halves of an amendment -- the clause in the amending ADR and the bare
number in the amended one's `Amended by:` -- because writing one and not the other is what the
`amend symmetry` check reports, and it is a check because a reader of the amended ADR is the one
who never finds out. `--set-status` moves the index cell that quotes the status with it.
`--next-section` says where a new section goes in an ADR whose `§ N` anchors are cited from
`crates/` and from `loop-goal.toml`: appending is free, renumbering breaks all of them silently.

**Every write is a transaction.** The tool applies the lot, re-runs the audit, and if the tree
gained one finding it did not have before, restores every byte and says which. A refusal never
leaves an index row pointing at a body that is not there. What it does *not* do is judge prose:
folding the amended ADR's body so it reads as currently true is the author's, and both commands
say so on the way out.

WHAT IT CHECKS, AND WHY EACH ONE IS HERE RATHER THAN IN A REVIEWER'S HEAD

  metadata   Every ADR opens with the same field block. `Status` must be a bare value, because a
             status carrying a paragraph is a status nobody can filter on. Unknown field names are
             refused for the reason `deny_unknown_fields` refuses an unknown config key (ADR 0064
             SS 3): a typo'd `Amended-by:` reads as "nothing amends this".

  structure  The heading set is closed and its order is fixed, so a reader who has found
             *Alternatives rejected* once knows where it is in every other ADR. The corpus had
             four different placements for `## Verification` before this ran. No section is empty.

  links      Every `[NNNN](NNNN-slug.md)` must resolve, and every `SS N` reference must name a
             section the target ADR actually has. A cross-link into a section that was renumbered
             is the failure mode this set is most exposed to, because sections are cited by number
             from other ADRs, from `loop-goal.toml`, and from code comments.

  symmetry   `Amends: A` in B obliges `Amended by: B` in A. One-directional folds are how an ADR
             ends up describing a rule that a later one already replaced.

  indexes    Every ADR owes one routing row in README.md and one bullet in ground-rules.md. An ADR
             in neither is unreachable by `brief.py --where`, which is the only way anyone finds it.
             README's index table is *derived* from the titles -- `--index` prints it -- because a
             hand-written Decision cell is a second copy of a sentence the ADR already opens with.

  residue    An ADR body states the *current* rule and nothing else -- git holds the history. Prose
             like "previously said", "is withdrawn", "used to" is an overlay a reader must apply in
             their head, which is exactly what folding exists to prevent.

  counters   A count restated in more than one file goes stale. ADR 0102 SS 9 found one that had
             been wrong in seven places. Any spelled-out running total in a body is reported.

Nothing here measures prose against a length. doc-style.md SS *Length targets* is explicit that
nothing in this repository does, and `--stats` prints sizes so a human can judge, never a verdict.
"""

from __future__ import annotations

import argparse
import datetime
import glob
import io
import os
import re
import sys
import textwrap
from collections import defaultdict

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADR_DIR = os.path.join(ROOT, "docs", "adr")

# The closed field set. A field outside this list is a typo, per the module doc. `Relates to:` was
# retired -- 743 numbers across 95 ADRs that `--graph` derives -- and `Supersedes:` was a second
# spelling of `Amends:`.
FIELDS = ["Status", "Date", "Scope", "Depends on", "Amends", "Amended by", "Validated by"]
STATUSES = {"Accepted", "Proposed", "Rejected", "Superseded", "Retired"}

# The closed heading set, in canonical order. Front-loading is the `In short` block's job, not
# the section order's -- README.md says a reader who only needs the rule stops there -- so `Context`
# leading is the house shape and what this enforces is that every ADR uses the same one.
CANONICAL = [
    "Context",
    "Investigation",
    "Options considered",
    "Decision",
    "Diagnostics",
    "Consequences",
    "Alternatives rejected",
    "Revisiting",
    "Verification",
]
# Sections a reader skips unless they intend to overturn the decision -- the trim surface.
RATIONALE = {"Context", "Investigation", "Options considered", "Alternatives rejected", "Revisiting"}

RESIDUE = [
    (r"\bthis body (?:now )?states the current rule\b", "fold boilerplate"),
    (r"\bpreviously (?:said|read|stated|carried)\b", "describes a prior version"),
    (r"\b(?:is|are) (?:now )?withdrawn\b", "tombstone"),
    (r"\bthis section (?:originally|once|used to)\b", "tombstone"),
    (r"\bWithdrawn\b", "tombstone heading"),
    (r"\bthat sentence (?:is|was) (?:now )?(?:corrected|replaced)\b", "describes an edit"),
    (r"\bformerly listed here\b", "tombstone"),
    (r"\bin (?:an|its) earlier draft\b", "describes a prior version"),
    (r"\bthis ADR (?:first|originally) (?:said|admitted|specified)\b", "describes a prior version"),
]
COUNTERS = [
    r"\bthe (?:tenth|eleventh|twelfth|thirteenth|fourteenth) divergence\b",
    r"\bnow holds (?:three|four|five|six|seven|eight|nine|ten) names\b",
    r"\bexactly those (?:three|four|five|six|seven) names\b",
    r"\bthe (?:third|fourth|fifth|sixth|seventh) forcing case\b",
]


class Adr:
    def __init__(self, path: str, text: str | None = None) -> None:
        self.path = path
        self.file = os.path.basename(path)
        self.num = self.file[:4]
        self.text = open(path, encoding="utf-8").read() if text is None else text
        self.lines = self.text.split("\n")
        self.title = self.lines[0].lstrip("# ").strip() if self.lines else ""
        self.fields: dict[str, str] = {}
        self.field_line: dict[str, int] = {}
        #: (first line, last line) of each field, 1-based inclusive, continuations included.
        #: `field_line` says where a finding is; this says what a rewrite has to replace.
        self.field_span: dict[str, tuple[int, int]] = {}
        self.unknown: list[tuple[int, str]] = []
        self.headings: list[tuple[int, str]] = []
        self.sections: dict[str, tuple[int, int]] = {}
        self.subsections: set[str] = set()
        self.in_short = False
        self._parse()

    def _parse(self) -> None:
        cur = None
        for i, line in enumerate(self.lines, 1):
            m = re.match(r"^- \*\*([^:*]+):\*\*\s*(.*)$", line)
            if m and not self.headings:
                name, value = m.group(1).strip(), m.group(2).strip()
                if name in FIELDS:
                    self.fields[name] = value
                    self.field_line[name] = i
                    self.field_span[name] = (i, i)
                    cur = name
                else:
                    self.unknown.append((i, name))
                    cur = None
                continue
            if cur and line.startswith("  ") and not line.startswith("  -"):
                self.fields[cur] += " " + line.strip()
                self.field_span[cur] = (self.field_span[cur][0], i)
                continue
            cur = None
            if line.startswith("> **In short:**"):
                self.in_short = True
            if line.startswith("## "):
                self.headings.append((i, line[3:].strip()))
            m = re.match(r"^#{3,4} (\d+[a-z]?)\.", line)
            if m:
                self.subsections.add(m.group(1))
        for idx, (line_no, name) in enumerate(self.headings):
            end = self.headings[idx + 1][0] - 1 if idx + 1 < len(self.headings) else len(self.lines)
            self.sections[name] = (line_no, end)

    def body_of(self, name: str) -> str:
        if name not in self.sections:
            return ""
        start, end = self.sections[name]
        return "\n".join(self.lines[start:end])

    def refs(self) -> set[str]:
        """Every ADR this one links to, by number."""
        return set(re.findall(r"\((\d{4})-[a-z0-9-]+\.md\)", self.text))

    def field_nums(self, name: str) -> set[str]:
        return set(re.findall(r"\b(\d{4})\b", self.fields.get(name, "")))

    def section_refs(self) -> list[tuple[int, str, str]]:
        """(line, target ADR number, section number) for every `0007 SS 3`-shaped citation."""
        out = []
        for i, line in enumerate(self.lines, 1):
            # `[0007](…)` or a bare `ADR 0007`, then §N within a short span. `E0122` is a
            # diagnostic code, not an ADR, so a digit run preceded by a letter never counts.
            for m in re.finditer(r"(?<![A-Za-z0-9])(\d{4})(?:\]|\)|\b)[^.\n|]{0,60}?§§?\s*(\d+[a-z]?)", line):
                num, sec = m.group(1), m.group(2)
                if num != self.num and re.match(r"^0\d{3}$", num):
                    out.append((i, num, sec))
        return out


def load() -> dict[str, Adr]:
    return {a.num: a for a in (Adr(p) for p in sorted(glob.glob(os.path.join(ADR_DIR, "0*.md"))))}


# ---------------------------------------------------------------- checks


def check_metadata(adrs):
    out = []
    for a in adrs.values():
        for line, name in a.unknown:
            out.append((a.file, line, f"unknown metadata field `{name}` -- one of {', '.join(FIELDS)}"))
        for req in ("Status", "Date", "Scope"):
            if req not in a.fields:
                out.append((a.file, 1, f"missing `{req}:`"))
        st = a.fields.get("Status", "")
        if st and st not in STATUSES:
            short = st if len(st) < 60 else st[:57] + "..."
            out.append((a.file, a.field_line.get("Status", 1), f"`Status:` is prose, not a value: {short}"))
        if not a.in_short:
            out.append((a.file, 1, "no `> **In short:**` block"))
        for f in ("Amends", "Amended by", "Relates to", "Supersedes"):
            if f in a.fields and not a.fields[f].strip():
                out.append((a.file, a.field_line[f], f"`{f}:` is present but empty -- omit the field"))
    return out


def check_structure(adrs):
    out = []
    for a in adrs.values():
        names = [h[1] for h in a.headings]
        if "Decision" not in names:
            out.append((a.file, 1, "no `## Decision`"))
            continue
        for n in names:
            if n not in CANONICAL:
                out.append((a.file, dict((v, k) for k, v in a.headings)[n], f"non-canonical heading `## {n}`"))
        ordered = [n for n in names if n in CANONICAL]
        rank = [CANONICAL.index(n) for n in ordered]
        if rank != sorted(rank):
            out.append((a.file, 1, f"sections out of canonical order: {' -> '.join(ordered)}"))
        for n, (s, e) in a.sections.items():
            if not "\n".join(a.lines[s:e]).strip():
                out.append((a.file, s, f"`## {n}` is empty"))
    return out


#: The three files that carry links *to* ADRs without being one. They are checked alongside the set
#: because they are where an ADR's links are written last and looked at least -- and because
#: `--new` writes into all three, so a link it got wrong has to be a finding for the write to be
#: rolled back. They sit in `docs/adr/`, so a relative link resolves exactly as an ADR's does.
INDEX_DOCS = ["README.md", "ground-rules.md", "divergences.md", "tooling-parity.md"]


def _link_sources(adrs):
    for a in adrs.values():
        yield a.file, a.lines
    for name in INDEX_DOCS:
        path = os.path.join(ADR_DIR, name)
        if os.path.exists(path):
            yield name, open(path, encoding="utf-8").read().split("\n")


def check_links(adrs):
    out = []
    for file, lines in _link_sources(adrs):
        for i, line in enumerate(lines, 1):
            for m in re.finditer(r"\]\((\d{4})-([a-z0-9-]+)\.md(?:#[^)]*)?\)", line):
                target = f"{m.group(1)}-{m.group(2)}.md"
                if not os.path.exists(os.path.join(ADR_DIR, target)):
                    out.append((file, i, f"broken ADR link -> {target}"))
            for m in re.finditer(r"\]\((\.\./[^)#]+)(?:#[^)]*)?\)", line):
                rel = os.path.normpath(os.path.join(ADR_DIR, m.group(1)))
                if not os.path.exists(rel):
                    out.append((file, i, f"broken relative link -> {m.group(1)}"))
    return out


def check_section_refs(adrs):
    out = []
    for a in adrs.values():
        for line, num, sec in a.section_refs():
            t = adrs.get(num)
            if t is None:
                out.append((a.file, line, f"cites ADR {num}, which does not exist"))
            elif t.subsections and sec not in t.subsections:
                have = ", ".join(sorted(t.subsections, key=lambda s: (len(s), s)))
                out.append((a.file, line, f"cites {num} § {sec}; that ADR has §§ {have}"))
    return out


def check_symmetry(adrs):
    out = []
    for a in adrs.values():
        for t in a.field_nums("Amends"):
            if t in adrs and a.num not in adrs[t].field_nums("Amended by"):
                out.append((a.file, a.field_line.get("Amends", 1), f"amends {t}, but {t} does not list `Amended by: {a.num}`"))
        for t in a.field_nums("Amended by"):
            if t in adrs and a.num not in adrs[t].field_nums("Amends"):
                out.append((a.file, a.field_line.get("Amended by", 1), f"claims {t} amends it, but {t} has no `Amends: {a.num}`"))
    return out


def _index_text():
    readme = open(os.path.join(ADR_DIR, "README.md"), encoding="utf-8").read()
    rules = open(os.path.join(ADR_DIR, "ground-rules.md"), encoding="utf-8").read()
    return readme, rules


def index_rows(adrs):
    """The index table, derived. An ADR's title already *is* its decision as a statement
    (conventions.md § *An ADR*), so a hand-written Decision cell is a second copy of it -- and the
    26 cells that had drifted past 200 bytes, one to 836, are what a second copy does."""
    yield "| # | Decision | Status |"
    yield "|---|---|---|"
    for num, a in sorted(adrs.items()):
        decision = a.title.split("—", 1)[-1].strip().replace("|", r"\|")
        yield f"| [{num}]({a.file}) | {decision} | {a.fields.get('Status', '?')} |"


#: Both the check and the write find the table through this one anchor, so there is no way for
#: them to disagree about which block is the index.
INDEX_TABLE_RE = re.compile(r"^\| # \| Decision \| Status \|\n(?:\|.*\n)+", re.M)


def check_index_table(adrs):
    readme = open(os.path.join(ADR_DIR, "README.md"), encoding="utf-8").read()
    want = "\n".join(index_rows(adrs))
    m = INDEX_TABLE_RE.search(readme)
    if not m:
        return [("README.md", 1, "no index table -- `python tools/adr.py --index` prints one")]
    if m.group(0).strip() != want.strip():
        return [("README.md", readme[: m.start()].count("\n") + 1,
                 "index table is stale -- write it with `python tools/adr.py --sync`")]
    return []


def sync_index_table(adrs):
    """Write the derived table into README.md, in place of whatever block is there.

    Every cell of it comes off the ADR files -- the number, the title as its decision, the
    `Status:` field -- so this replaces the block whole rather than merging into it. `--check`
    is what reports the drift and this is what closes it; before, `--index` printed the table
    and a reader pasted it, which is a hand copy of derived data at the one moment the reader
    has least reason to look at it closely."""
    path = os.path.join(ADR_DIR, "README.md")
    readme = open(path, encoding="utf-8").read()
    m = INDEX_TABLE_RE.search(readme)
    if not m:
        print("adr.py: no `| # | Decision | Status |` table in README.md to replace. "
              "`--index` prints one to paste in where it belongs.")
        return 1
    want = "\n".join(index_rows(adrs)) + "\n"
    if m.group(0) == want:
        print(f"adr.py: the index table already states all {len(adrs)} ADRs -- nothing to write")
        return 0
    was = max(m.group(0).count("\n") - 2, 0)
    with open(path, "w", encoding="utf-8", newline="") as fh:
        fh.write(readme[: m.start()] + want + readme[m.end():])
    print(f"adr.py: README.md's index table rewritten from the ADR files, "
          f"{was} -> {len(adrs)} rows")
    return 0


def check_indexes(adrs):
    out = []
    readme, rules = _index_text()
    routed = set(re.findall(r"\((\d{4})-[a-z0-9-]+\.md\)", readme))
    ruled = set(re.findall(r"\((\d{4})-[a-z0-9-]+\.md\)", rules))
    for num, a in adrs.items():
        if num not in routed:
            out.append((a.file, 1, "no row in README.md § *Where to look*"))
        if num not in ruled:
            out.append((a.file, 1, "no bullet in ground-rules.md"))
    for target in sorted(routed | ruled):
        if target not in adrs:
            out.append(("README.md/ground-rules.md", 1, f"index points at ADR {target}, which does not exist"))
    return out


# The metadata block's `Amends:` clause says what changed in the ADR it names -- that is its job.
# *Alternatives rejected* and *Verification* name rejected and withdrawn things for a living. Residue
# is prose in the ADR's own **body** that narrates a prior version of itself.
RESIDUE_SKIP = {"Alternatives rejected", "Verification"}


def check_residue(adrs):
    out = []
    for a in adrs.values():
        body_start = a.headings[0][0] if a.headings else len(a.lines)
        skip = [a.sections[n] for n in RESIDUE_SKIP if n in a.sections]
        for i, line in enumerate(a.lines, 1):
            if i < body_start or any(s <= i <= e for s, e in skip):
                continue
            for pat, why in RESIDUE:
                if re.search(pat, line, re.I):
                    out.append((a.file, i, f"{why}: {line.strip()[:100]}"))
                    break
    return out


def check_counters(adrs):
    out = []
    for a in adrs.values():
        for i, line in enumerate(a.lines, 1):
            for pat in COUNTERS:
                if re.search(pat, line, re.I):
                    out.append((a.file, i, f"running count in prose -- keep the total in one home: {line.strip()[:90]}"))
                    break
    return out


CHECKS = [
    ("metadata", check_metadata),
    ("structure", check_structure),
    ("links", check_links),
    ("section refs", check_section_refs),
    ("amend symmetry", check_symmetry),
    ("index coverage", check_indexes),
    ("index table", check_index_table),
    ("changelog residue", check_residue),
    ("stale counters", check_counters),
]


# ---------------------------------------------------------------- reports


def report(adrs, only=None, quiet=False):
    total = 0
    for name, fn in CHECKS:
        if only and name not in only:
            continue
        found = fn(adrs)
        total += len(found)
        if not found:
            if not quiet:
                print(f"ok   {name}")
            continue
        print(f"\n== {name} ({len(found)})")
        for f, line, msg in sorted(found):
            print(f"  {f}:{line}  {msg}")
    return total


def stats(adrs):
    print(f"{'adr':<6}{'lines':>6}{'bytes':>8}  {'rationale':>9}  sections")
    tot_l = tot_b = tot_r = 0
    for num, a in sorted(adrs.items()):
        b = len(a.text)
        rat = sum(len(a.body_of(n)) for n in RATIONALE if n in a.sections)
        tot_l += len(a.lines)
        tot_b += b
        tot_r += rat
        shape = " ".join(n[:4] for _, n in a.headings)
        pct = f"{100 * rat // b if b else 0}%"
        print(f"{num:<6}{len(a.lines):>6}{b:>8}  {rat:>6} {pct:>2}  {shape}")
    print(f"\n{len(adrs)} ADRs, {tot_l} lines, {tot_b} bytes; "
          f"{tot_r} bytes ({100 * tot_r // tot_b}%) in rationale sections")


def graph(adrs, num):
    a = adrs.get(num)
    if a is None:
        print(f"no ADR {num}")
        return 1
    cited_by = sorted(n for n, o in adrs.items() if num in o.refs() and n != num)
    print(f"{a.file}\n  {a.title}\n")
    for f in FIELDS:
        if f in a.fields:
            v = a.fields[f]
            print(f"  {f + ':':<15}{v[:110]}{'...' if len(v) > 110 else ''}")
    print(f"\n  links to:      {', '.join(sorted(a.refs() - {num})) or '-'}")
    print(f"  cited by:      {', '.join(cited_by) or '-'}")
    print(f"  sections:      {', '.join(sorted(a.subsections, key=lambda s: (len(s), s))) or '-'}")
    return 0


def orphans(adrs):
    inbound = defaultdict(set)
    for n, a in adrs.items():
        for t in a.refs():
            if t != n:
                inbound[t].add(n)
    readme, rules = _index_text()
    routed = set(re.findall(r"\((\d{4})-[a-z0-9-]+\.md\)", readme))
    ruled = set(re.findall(r"\((\d{4})-[a-z0-9-]+\.md\)", rules))
    print("ADRs no other ADR links to:")
    for n in sorted(adrs):
        if not inbound[n]:
            print(f"  {n}  {adrs[n].title[:88]}")
    print("\nmissing from an index:")
    for n in sorted(adrs):
        miss = [w for w, s in (("routing table", routed), ("ground-rules", ruled)) if n not in s]
        if miss:
            print(f"  {n}  {', '.join(miss)}")
    print("\nmost-cited:")
    for n, s in sorted(inbound.items(), key=lambda kv: -len(kv[1]))[:12]:
        print(f"  {n}  {len(s):>3} inbound")


# ---------------------------------------------------------------- writing
#
# Everything above answers questions about the set. Everything below changes it, and it exists
# because the seven-step recipe in README.md § *Adding a decision* is a form, not a decision: pick
# the number nobody else took, derive the slug, date it, fold the back-link into the amended ADR,
# add a routing row, add a ground-rules bullet, regenerate the index, then run the audit. Six of
# the seven are mechanical, three of them are edits to files the author has no other reason to
# open, and the one that is not mechanical -- the ADR's own prose -- is the only one worth a human
# turn. The failure this replaces is not a typo: it is an ADR that lands with a `Amends:` and no
# matching `Amended by:`, or missing from an index, which is `--check`'s whole finding list.
#
# Every write is a transaction. The tool applies the lot, re-runs the audit, and if the tree gained
# a single finding it did not have before, puts every byte back and says which. So a refusal never
# leaves an index row pointing at a body that is not there.


DIRECTIVES = ["Slug", "Route", "Rule", "Divergence"]

FIELD_RE = re.compile(r"^- \*\*([^:*]+):\*\*\s*(.*)$")
WHERE_TABLE_RE = re.compile(r"^\| Doing this \| Open this \|\n\|---\|---\|\n(?:\|.*\n)+", re.M)

DRAFT = """# ADR NNNN — <the decision as a statement, not a topic>

- **Status:** Accepted
- **Scope:** what this decides, then explicitly what it does *not* — with the file that owns each
  excluded thing.
- **Depends on:** <NNNN, only if this has nothing to decide without it — delete otherwise>
- **Amends:** [NNNN](NNNN-slug.md) § N — what changed there, one clause per target. Delete unless
  this changes a prior ADR's rule. Adding it back-links the other ADR for you; editing that ADR's
  body to state the new rule is still yours to do.
- **Validated by:** <the test that holds a claim this makes, by path — delete otherwise>
- **Route:** the keywords and PHP spellings a reader arrives with | the one file that owns it, as
  the routing table's right-hand cell
- **Rule:** <ground-rules section> | **the one sentence**, only if this is a hard invariant
- **Divergence:** <divergences section> | what PHP does | what Novis does

> **In short:** the whole decision, in one blockquote. A reader who needs only the rule stops here,
> so this paragraph is the ADR's front page and is worth more care than any section below it.

## Context

## Decision

### 1. The first rule

## Consequences

## Alternatives rejected

## Verification
"""


#: Words a truncated slug must not end on. `...-the-name-is-the-identity-not-a` is a filename that
#: reads as though it were cut off, because it was; ending a word earlier says the same thing.
SLUG_TAIL = {"a", "an", "and", "as", "at", "but", "by", "for", "from", "in", "is", "it", "its",
             "not", "of", "on", "or", "so", "than", "that", "the", "then", "to", "with"}


def slugify(title: str, limit: int = 62) -> str:
    """The filename half of an ADR, from its title. Whole words only and never past `limit`, so a
    long title truncates where a reader would rather than mid-word; `Slug:` overrides it when the
    derived one reads badly, which is why 0044 is `core-process-argv-only-no-shell`."""
    out: list[str] = []
    n = 0
    for w in re.sub(r"[^a-z0-9]+", " ", title.lower()).split():
        if out and n + 1 + len(w) > limit:
            break
        n += (1 if out else 0) + len(w)
        out.append(w)
    while len(out) > 1 and out[-1] in SLUG_TAIL:
        out.pop()
    return "-".join(out) or "untitled"


def render_field(name: str, value: str) -> str:
    """One metadata field, folded at the width the rest of `docs/` is written to. A link is one
    whitespace-free token, so folding can never land a newline inside `[0033](0033-....md)`."""
    return "\n".join(textwrap.wrap(
        f"- **{name}:** {value}", width=100, subsequent_indent="  ",
        break_long_words=False, break_on_hyphens=False,
    ))


def parse_block(lines) -> list[list]:
    """The metadata block as `[name, value, first line, last line]`, in the order written.

    Shared by the draft and by an ADR already on disk, so the two can never disagree about where
    a field ends -- which is the question every rewrite below has to answer."""
    out: list[list] = []
    cur = None
    for i, line in enumerate(lines, 1):
        if line.startswith("## "):
            break
        m = FIELD_RE.match(line)
        if m:
            out.append([m.group(1).strip(), m.group(2).strip(), i, i])
            cur = out[-1]
            continue
        if cur and line.startswith("  ") and not line.startswith("  -"):
            cur[1] += " " + line.strip()
            cur[3] = i
            continue
        cur = None
    return out


def set_field(text: str, name: str, value: str) -> str:
    """Write one field, replacing it whole if it is there and inserting it in `FIELDS` order if it
    is not. The order matters to nothing but a reader, and a reader is who the block is for."""
    lines = text.split("\n")
    spans = {b[0]: (b[2], b[3]) for b in parse_block(lines) if b[0] in FIELDS}
    new = render_field(name, value).split("\n")
    if name in spans:
        s, e = spans[name]
        return "\n".join(lines[:s - 1] + new + lines[e:])
    rank = FIELDS.index(name)
    above = [f for f in spans if FIELDS.index(f) < rank]
    below = [f for f in spans if FIELDS.index(f) > rank]
    if above:
        at = spans[max(above, key=lambda f: spans[f][1])][1]
    elif below:
        at = spans[min(below, key=lambda f: spans[f][0])][0] - 1
    else:
        raise ValueError("no metadata block to insert a field into")
    return "\n".join(lines[:at] + new + lines[at:])


def add_amended_by(text: str, num: str) -> str:
    """`Amended by:` is bare numbers, sorted, comma-separated -- README.md is explicit that an
    explanation there would be repeating the fold rule. So this is a set union, not an append."""
    block = {b[0]: b[1] for b in parse_block(text.split("\n"))}
    have = set(re.findall(r"\b\d{4}\b", block.get("Amended by", ""))) | {num}
    return set_field(text, "Amended by", ", ".join(sorted(have)))


def _section_bounds(lines, section: str):
    start = next((i for i, l in enumerate(lines) if l.strip() == f"## {section}"), None)
    if start is None:
        return None
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    return start, end


def append_bullet(text: str, section: str, bullet: str):
    """Append a bullet at the end of a `## ` section, above the blank line that separates it from
    the next one. ground-rules.md grows a bullet per ADR and nothing reads it in full, so where in
    the section it lands does not matter; which section does."""
    lines = text.split("\n")
    bounds = _section_bounds(lines, section)
    if bounds is None:
        return None
    _, end = bounds
    while end > 0 and not lines[end - 1].strip():
        end -= 1
    return "\n".join(lines[:end] + bullet.split("\n") + lines[end:])


def append_row(text: str, section: str, row: str):
    """Append a row to the last table in a `## ` section -- divergences.md's shape."""
    lines = text.split("\n")
    bounds = _section_bounds(lines, section)
    if bounds is None:
        return None
    start, end = bounds
    last = max((i for i in range(start + 1, end) if lines[i].startswith("|")), default=None)
    if last is None:
        return None
    return "\n".join(lines[:last + 1] + [row] + lines[last + 1:])


def sections_of(text: str) -> list[str]:
    return [l[3:].strip() for l in text.split("\n") if l.startswith("## ")]


class Tx:
    """All of it or none of it.

    A half-applied ADR is the one failure worth engineering against: an index row pointing at a
    body that is not there, or an `Amends:` whose other half never landed. `rollback` restores
    every touched file byte for byte and deletes anything this run created."""

    def __init__(self) -> None:
        self.before: dict[str, bytes | None] = {}
        self.wrote: list[str] = []

    def write(self, path: str, text: str) -> None:
        if path not in self.before:
            self.before[path] = open(path, "rb").read() if os.path.exists(path) else None
        with open(path, "w", encoding="utf-8", newline="") as fh:
            fh.write(text)
        if path not in self.wrote:
            self.wrote.append(path)

    def rollback(self) -> None:
        for path, data in self.before.items():
            if data is None:
                if os.path.exists(path):
                    os.remove(path)
            else:
                with open(path, "wb") as fh:
                    fh.write(data)


def write_index(tx: Tx) -> bool:
    """Regenerate README.md's index table *inside* the transaction. `--sync` writes it directly,
    which is right for a standalone call and wrong for every caller here: an index rewritten
    outside the transaction survives a rollback, and then the table states a status the ADR does
    not. That is the one inconsistency this whole file is built to make impossible."""
    path = os.path.join(ADR_DIR, "README.md")
    readme = open(path, encoding="utf-8").read()
    m = INDEX_TABLE_RE.search(readme)
    if not m:
        return False
    tx.write(path, readme[:m.start()] + "\n".join(index_rows(load())) + "\n" + readme[m.end():])
    return True


def findings_now() -> set:
    """Every check's findings as `(check, file, message)`. The line number is dropped on purpose:
    appending a row to README.md moves every finding below it, and a moved finding is not a new
    one. What this compares is whether the tree gained a *defect*."""
    adrs = load()
    return {(name, f, msg) for name, fn in CHECKS for f, _, msg in fn(adrs)}


def guard(tx: Tx, baseline: set, what: str) -> int:
    """Re-audit, and undo everything if the tree gained a finding it did not already have."""
    gained = findings_now() - baseline
    if not gained:
        return 0
    tx.rollback()
    print(f"adr.py: rolled back -- {what} would have left {len(gained)} new finding(s):\n")
    for name, f, msg in sorted(gained):
        print(f"  [{name}] {f}  {msg}")
    return 1


def fail(msg: str) -> int:
    print(f"adr.py: {msg}")
    return 1


def validate_body(title: str, lines, problems: list[str]) -> None:
    """The draft against the shape `--check` will hold the landed file to. Caught here, a
    misordered heading costs a sentence; caught after the write, it costs a rollback."""
    if "—" not in title and " - " in title:
        problems.append("the title separator is an em dash `—`, not a hyphen")
    heads = [l[3:].strip() for l in lines if l.startswith("## ")]
    if "Decision" not in heads:
        problems.append("no `## Decision` -- an ADR that decides nothing is not an ADR")
    for h in heads:
        if h not in CANONICAL:
            problems.append(f"non-canonical heading `## {h}` -- the set is {', '.join(CANONICAL)}; "
                            "anything else is a `###` subsection under Decision")
    rank = [CANONICAL.index(h) for h in heads if h in CANONICAL]
    if rank != sorted(rank):
        problems.append("sections are out of canonical order: "
                        + " -> ".join(h for h in heads if h in CANONICAL))
    if not any(l.startswith("> **In short:**") for l in lines):
        problems.append("no `> **In short:**` block -- it is the ADR's front page")


def cmd_new(adrs, path: str, dry_run: bool) -> int:
    """One draft in, an indexed ADR out. See the module doc for what it touches and in what order."""
    if not os.path.exists(path):
        return fail(f"no draft file at {path} -- `--draft` prints one to fill in")
    text = open(path, encoding="utf-8").read()
    lines = text.split("\n")

    m = re.match(r"^# ADR (?:NNNN|\d{4}) [—-] (.+?)\s*$", lines[0] if lines else "")
    if not m:
        return fail("the draft's first line must read `# ADR NNNN — <the decision as a statement>`")
    title = m.group(1).strip()

    block = parse_block(lines)
    fields = {b[0]: b[1] for b in block}
    problems: list[str] = []
    for name in fields:
        if name not in FIELDS and name not in DIRECTIVES:
            problems.append(f"unknown field `{name}:` -- the ADR set is {', '.join(FIELDS)}; "
                            f"this tool also consumes {', '.join(DIRECTIVES)}")
    if not fields.get("Scope"):
        problems.append("no `Scope:` -- it must say what this does *not* decide, and who owns that")
    status = fields.get("Status") or "Accepted"
    if status not in STATUSES:
        problems.append(f"`Status:` must be one of {', '.join(sorted(STATUSES))}, not {status!r}")
    validate_body(title, lines, problems)

    # The three index payloads, each `section | text`-shaped, checked against the file they land in
    # before anything is written -- a section name that does not exist is the likely typo.
    rules_path = os.path.join(ADR_DIR, "ground-rules.md")
    div_path = os.path.join(ADR_DIR, "divergences.md")
    route = rule = diverge = None
    if fields.get("Route"):
        parts = [p.strip() for p in fields["Route"].split("|")]
        if len(parts) != 2 or not all(parts):
            problems.append("`Route:` is `<what a reader is doing> | <the one file that owns it>`")
        else:
            route = parts
    if fields.get("Rule"):
        parts = [p.strip() for p in fields["Rule"].split("|", 1)]
        if len(parts) != 2 or not all(parts):
            problems.append("`Rule:` is `<ground-rules section> | <the one sentence>`")
        elif parts[0] not in sections_of(open(rules_path, encoding="utf-8").read()):
            problems.append(f"ground-rules.md has no `## {parts[0]}` section; it has "
                            + ", ".join(sections_of(open(rules_path, encoding="utf-8").read())))
        else:
            rule = parts
    if fields.get("Divergence"):
        parts = [p.strip() for p in fields["Divergence"].split("|")]
        if len(parts) != 3 or not all(parts):
            problems.append("`Divergence:` is `<section> | <what PHP does> | <what Novis does>`")
        elif parts[0] not in sections_of(open(div_path, encoding="utf-8").read()):
            problems.append(f"divergences.md has no `## {parts[0]}` section")
        else:
            diverge = parts

    for dep in re.findall(r"\b\d{4}\b", fields.get("Depends on", "")):
        if dep not in adrs:
            problems.append(f"`Depends on:` names ADR {dep}, which does not exist")
    amends = sorted(set(re.findall(r"\b\d{4}\b", fields.get("Amends", ""))))
    for t in amends:
        if t not in adrs:
            problems.append(f"`Amends:` names ADR {t}, which does not exist")
    if fields.get("Amends") and not amends:
        problems.append("`Amends:` names no ADR by number -- link the target as `[NNNN](NNNN-slug.md)`")

    if problems:
        print(f"adr.py: {len(problems)} problem(s) in {path} -- nothing was written\n")
        for pr in problems:
            print(f"  {pr}")
        return 1

    # The number is claimed by creating the file, and it is re-derived here rather than taken from
    # the draft: another agent working the same tree derives the same answer from the same
    # directory, and the loser of that race is the one whose `open` finds a file already there.
    num = f"{max(int(n) for n in adrs) + 1:04d}"
    slug = fields.get("Slug") or slugify(title)
    file = f"{num}-{slug}.md"
    dest = os.path.join(ADR_DIR, file)
    if os.path.exists(dest):
        return fail(f"{file} already exists -- another agent claimed {num} first; re-run")

    # Rebuild the metadata block: the directives are consumed, `Status` and `Date` are defaulted,
    # and everything else keeps the author's own wrapping.
    # `NNNN` is how a draft names an ADR that does not have a number yet -- its own. Expanding it
    # here is what lets the `Route:` cell carry a link to the file being created, which is the one
    # link in the whole set the author provably cannot write by hand.
    def expand(s: str) -> str:
        return re.sub(r"NNNN-[a-z0-9-]*\.md", file, s).replace("NNNN", num)

    route = [expand(x) for x in route] if route else None
    rule = [rule[0], expand(rule[1])] if rule else None
    diverge = [diverge[0], expand(diverge[1]), expand(diverge[2])] if diverge else None

    drop: set[int] = set()
    for name, _value, first, last in block:
        if name in DIRECTIVES:
            drop.update(range(first, last + 1))
    body = "\n".join([f"# ADR {num} — {title}"]
                     + [expand(l) for i, l in enumerate(lines[1:], 2) if i not in drop])
    body = set_field(body, "Status", status)
    if not fields.get("Date") or fields["Date"].startswith("<"):
        body = set_field(body, "Date", datetime.date.today().isoformat())
    if not body.endswith("\n"):
        body += "\n"

    touches = [f"create docs/adr/{file}"]
    touches += [f"fold `Amended by: {num}` into {adrs[t].file}" for t in amends]
    if route:
        touches.append("add a row to README.md § *Where to look*")
    touches.append("regenerate README.md's index table")
    if rule:
        touches.append(f"add a bullet to ground-rules.md § *{rule[0]}*")
    if diverge:
        touches.append(f"add a row to divergences.md § *{diverge[0]}*")
    if dry_run:
        print(f"adr.py: would claim ADR {num} as docs/adr/{file}\n")
        for t in touches:
            print(f"  {t}")
        if not route:
            print("\n  no `Route:` -- nothing will route a reader to this ADR by keyword")
        return 0

    baseline = findings_now()
    tx = Tx()
    tx.write(dest, body)
    for t in amends:
        tx.write(adrs[t].path, add_amended_by(open(adrs[t].path, encoding="utf-8").read(), num))

    readme_path = os.path.join(ADR_DIR, "README.md")
    if route:
        readme = open(readme_path, encoding="utf-8").read()
        mt = WHERE_TABLE_RE.search(readme)
        if not mt:
            tx.rollback()
            return fail("no `| Doing this | Open this |` table in README.md to add a routing row to")
        row = f"| {route[0]} | {route[1]} |\n"
        tx.write(readme_path, readme[:mt.end()] + row + readme[mt.end():])
    if rule:
        sentence = rule[1].rstrip().rstrip(".")
        bullet = "\n".join(textwrap.wrap(f"- {sentence} ([{num}]({file})).", width=100,
                                         subsequent_indent="  ", break_long_words=False,
                                         break_on_hyphens=False))
        out = append_bullet(open(rules_path, encoding="utf-8").read(), rule[0], bullet)
        if out is None:
            tx.rollback()
            return fail(f"ground-rules.md has no `## {rule[0]}` section")
        tx.write(rules_path, out)
    if diverge:
        row = f"| {diverge[1]} | {diverge[2]} | [{num}]({file}) |"
        out = append_row(open(div_path, encoding="utf-8").read(), diverge[0], row)
        if out is None:
            tx.rollback()
            return fail(f"divergences.md has no `## {diverge[0]}` section with a table")
        tx.write(div_path, out)

    if not write_index(tx):
        tx.rollback()
        return fail("no index table in README.md -- `--index` prints one to paste in where it belongs")

    if guard(tx, baseline, f"ADR {num}"):
        return 1

    print(f"adr.py: ADR {num} landed as docs/adr/{file}\n")
    for t in touches:
        print(f"  {t}")
    print(f"\n  files: {' '.join(os.path.relpath(p, ROOT).replace(os.sep, '/') for p in tx.wrote)}")
    if amends:
        print(f"\n  STILL YOURS: edit {', '.join(adrs[t].file for t in amends)} so the body states "
              f"the new rule.\n  Folding means the earlier ADR reads as currently true -- not a note "
              f"saying what changed.")
    if not route:
        print("\n  no `Route:` was given, so nothing routes a reader here by keyword.")
    return 0


def cmd_fold(adrs, source: str, clause: str, dry_run: bool) -> int:
    """Both halves of an `Amends:`, in one call: the clause in the amending ADR and the bare number
    in the amended one. Writing one and not the other is what `amend symmetry` reports, and it is
    reported because it is what a reader of the amended ADR never finds out."""
    a = adrs.get(source)
    if a is None:
        return fail(f"no ADR {source}")
    targets = sorted(set(re.findall(r"\b\d{4}\b", clause)) - {source})
    missing = [t for t in targets if t not in adrs]
    if missing:
        return fail(f"the clause names ADR {', '.join(missing)}, which does not exist")
    if not targets:
        return fail("the clause names no ADR -- write the target as `[NNNN](NNNN-slug.md) § N — what "
                    "changed there`")
    if dry_run:
        print(f"adr.py: would add the clause to {a.file}'s `Amends:` and fold "
              f"`Amended by: {source}` into {', '.join(adrs[t].file for t in targets)}")
        return 0

    baseline = findings_now()
    tx = Tx()
    # One clause per target, separated by `; `. The existing clause's own terminator goes, so a
    # second target does not land as `... share a slot.; [0035] ...`.
    have = a.fields.get("Amends", "").strip().rstrip(".;")
    tx.write(a.path, set_field(a.text, "Amends", f"{have}; {clause}" if have else clause))
    for t in targets:
        tx.write(adrs[t].path, add_amended_by(open(adrs[t].path, encoding="utf-8").read(), source))
    if guard(tx, baseline, f"folding {source} into {', '.join(targets)}"):
        return 1
    print(f"adr.py: {a.file} now amends {', '.join(targets)}, and each names {source} back.\n")
    print(f"  files: {' '.join(os.path.relpath(p, ROOT).replace(os.sep, '/') for p in tx.wrote)}")
    print(f"\n  STILL YOURS: edit {', '.join(adrs[t].file for t in targets)} so the body states the "
          f"new rule.\n  A cross-link with an unedited body is the state README.md calls the bug.")
    return 0


def cmd_status(adrs, num: str, status: str) -> int:
    """A status and the index cell that quotes it, which is the copy that goes stale."""
    a = adrs.get(num)
    if a is None:
        return fail(f"no ADR {num}")
    if status not in STATUSES:
        return fail(f"`Status:` must be one of {', '.join(sorted(STATUSES))}, not {status!r}")
    if a.fields.get("Status") == status:
        return fail(f"{a.file} is already {status}")
    was = a.fields.get("Status")
    baseline = findings_now()
    tx = Tx()
    tx.write(a.path, set_field(a.text, "Status", status))
    if not write_index(tx):
        tx.rollback()
        return fail("no index table in README.md -- `--index` prints one to paste in where it belongs")
    if guard(tx, baseline, f"setting {num} to {status}"):
        return 1
    print(f"adr.py: {a.file} is {was} -> {status}, and README.md's index cell with it")
    print(f"\n  files: {' '.join(os.path.relpath(p, ROOT).replace(os.sep, '/') for p in tx.wrote)}")
    return 0


def cmd_next_section(adrs, num: str) -> int:
    """Where a new section goes in an ADR whose sections are cited from `crates/` and from
    `loop-goal.toml`. A section number is a public identifier: appending is free, renumbering
    silently breaks every citation, so this prints the appends and never a renumbering."""
    a = adrs.get(num)
    if a is None:
        return fail(f"no ADR {num}")
    have = sorted(a.subsections, key=lambda s: (int(re.match(r"\d+", s).group()), s))
    if not have:
        print(f"{a.file} has no numbered sections; the first is `### 1.`")
        return 0
    tops = sorted({int(re.match(r"\d+", s).group()) for s in have})
    print(f"{a.file}\n  sections:  {', '.join('§ ' + s for s in have)}")
    print(f"  at the end: § {tops[-1] + 1}")
    print("  in between: " + ", ".join(
        f"§ {t}{chr(ord('a') + sum(1 for s in have if re.match(rf'^{t}[a-z]$', s)))} (after § {t})"
        for t in tops))
    print("\n  Never renumber: `0007 § 3` is cited from other ADRs, from docs/spec/, from")
    print("  docs/agent/loop-goal.toml and from doc comments in crates/.")
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--stats", action="store_true", help="size and section shape, one line per ADR")
    p.add_argument("--graph", metavar="NNNN", help="one ADR's amend/cite graph")
    p.add_argument("--orphans", action="store_true", help="unlinked and unindexed ADRs")
    p.add_argument("--index", action="store_true", help="print README.md's index table, derived")
    p.add_argument("--sync", action="store_true",
                   help="write that derived table into README.md, replacing the block that is there")
    p.add_argument("--residue", action="store_true", help="changelog prose only")
    p.add_argument("--check", action="store_true", help="quiet on success; exit non-zero on a finding")
    p.add_argument("--only", metavar="CHECK", action="append", help="run one named check")

    w = p.add_argument_group("writing")
    w.add_argument("--draft", action="store_true", help="print a draft ADR to fill in")
    w.add_argument("--new", metavar="FILE",
                   help="claim the next number for that draft and index it everywhere")
    w.add_argument("--fold", metavar="NNNN",
                   help="that ADR amends another; --into is the clause. Writes both sides")
    w.add_argument("--into", metavar="CLAUSE",
                   help="`[NNNN](NNNN-slug.md) § N — what changed there`, for --fold")
    w.add_argument("--set-status", metavar=("NNNN", "STATUS"), nargs=2,
                   help="set one ADR's `Status:` and resync the index table")
    w.add_argument("--next-section", metavar="NNNN",
                   help="where a new `### N.` goes in that ADR, appending and never renumbering")
    w.add_argument("--dry-run", action="store_true", help="with --new/--fold: say what it would do")
    args = p.parse_args()

    adrs = load()
    if args.draft:
        print(DRAFT, end="")
        return 0
    if args.new:
        return cmd_new(adrs, args.new, args.dry_run)
    if args.fold:
        if not args.into:
            return fail("--fold needs --into '<clause naming the amended ADR and what changed>'")
        return cmd_fold(adrs, args.fold, args.into, args.dry_run)
    if args.set_status:
        return cmd_status(adrs, args.set_status[0], args.set_status[1])
    if args.next_section:
        return cmd_next_section(adrs, args.next_section)
    if args.graph:
        return graph(adrs, args.graph)
    if args.stats:
        stats(adrs)
        return 0
    if args.orphans:
        orphans(adrs)
        return 0
    if args.sync:
        return sync_index_table(adrs)
    if args.index:
        print("\n".join(index_rows(adrs)))
        return 0
    if args.residue:
        return 1 if report(adrs, only={"changelog residue"}) else 0
    n = report(adrs, only=set(args.only) if args.only else None, quiet=args.check)
    if n:
        print(f"\n{n} finding(s) across {len(adrs)} ADRs")
    elif not args.check:
        print(f"\nclean: {len(adrs)} ADRs")
    return 1 if n and args.check else 0


if __name__ == "__main__":
    sys.exit(main())
