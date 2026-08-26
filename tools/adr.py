#!/usr/bin/env python3
"""Audit and route the ADR set.

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
import glob
import io
import os
import re
import sys
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
    def __init__(self, path: str) -> None:
        self.path = path
        self.file = os.path.basename(path)
        self.num = self.file[:4]
        self.text = open(path, encoding="utf-8").read()
        self.lines = self.text.split("\n")
        self.title = self.lines[0].lstrip("# ").strip() if self.lines else ""
        self.fields: dict[str, str] = {}
        self.field_line: dict[str, int] = {}
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
                    cur = name
                else:
                    self.unknown.append((i, name))
                    cur = None
                continue
            if cur and line.startswith("  ") and not line.startswith("  -"):
                self.fields[cur] += " " + line.strip()
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


def check_links(adrs):
    out = []
    for a in adrs.values():
        for i, line in enumerate(a.lines, 1):
            for m in re.finditer(r"\]\((\d{4})-([a-z0-9-]+)\.md(?:#[^)]*)?\)", line):
                target = f"{m.group(1)}-{m.group(2)}.md"
                if not os.path.exists(os.path.join(ADR_DIR, target)):
                    out.append((a.file, i, f"broken ADR link -> {target}"))
            for m in re.finditer(r"\]\((\.\./[^)#]+)(?:#[^)]*)?\)", line):
                rel = os.path.normpath(os.path.join(ADR_DIR, m.group(1)))
                if not os.path.exists(rel):
                    out.append((a.file, i, f"broken relative link -> {m.group(1)}"))
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


def check_index_table(adrs):
    readme = open(os.path.join(ADR_DIR, "README.md"), encoding="utf-8").read()
    want = "\n".join(index_rows(adrs))
    m = re.search(r"^\| # \| Decision \| Status \|\n(?:\|.*\n)+", readme, re.M)
    if not m:
        return [("README.md", 1, "no index table -- `python tools/adr.py --index` prints one")]
    if m.group(0).strip() != want.strip():
        return [("README.md", readme[: m.start()].count("\n") + 1,
                 "index table is stale -- regenerate with `python tools/adr.py --index`")]
    return []


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


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--stats", action="store_true", help="size and section shape, one line per ADR")
    p.add_argument("--graph", metavar="NNNN", help="one ADR's amend/cite graph")
    p.add_argument("--orphans", action="store_true", help="unlinked and unindexed ADRs")
    p.add_argument("--index", action="store_true", help="print README.md's index table, derived")
    p.add_argument("--residue", action="store_true", help="changelog prose only")
    p.add_argument("--check", action="store_true", help="quiet on success; exit non-zero on a finding")
    p.add_argument("--only", metavar="CHECK", action="append", help="run one named check")
    args = p.parse_args()

    adrs = load()
    if args.graph:
        return graph(adrs, args.graph)
    if args.stats:
        stats(adrs)
        return 0
    if args.orphans:
        orphans(adrs)
        return 0
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
