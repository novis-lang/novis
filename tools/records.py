#!/usr/bin/env python3
"""Audit the decision records, and answer questions about the set.

A decision record, `docs/decisions/NNNN.md`, is the reasoning behind a rule: what was asked, what was
considered, why this answer, and what it costs. The rule itself lives in `docs/rules/`, so a record is
where a reader goes to *change* a rule and never to learn one. A record is **frozen on acceptance**:
nothing in it is maintained afterwards except its `status:` line. There is no `Amends:`, no `Amended
by:` and no folding -- a later decision that changes a rule edits the rule's fragment and names the
earlier record only through the rule's `because` list, which the rulebook holds and `tools/rules.py`
validates. This tool is what keeps the records mechanically honest against that shape, and it
answers the same way every time the questions a cleanup pass used to answer by reading 143 files.

    python tools/records.py                 the full audit, grouped by check, exit non-zero on a finding
    python tools/records.py --check         the same, quiet on success -- the CI shape
    python tools/records.py --only links    one named check, by the name the audit prints
    python tools/records.py --stats         one line per record: size, section shape, rationale share
    python tools/records.py --graph 0066    the rules a record creates and modifies, and who else shaped them
    python tools/records.py --orphans       records no other record links to, and the most cited
    python tools/records.py --residue       changelog prose a frozen body should not carry

THE SHAPE A RECORD IS HELD TO

    ---
    date: 2026-08-23
    status: accepted                    accepted | retired | superseded-by NNNN
    changes:
      creates:
        - expressions/nullable-conversion
      modifies:
        - types/conversion
    ---
    # ADR 0066 — the decision as a statement, not a topic

    - **Scope:** ...                    then `Depends on` and `Validated by`, where the record has them

    > **In short:** ...

    ## Context ... ## Decision ... ## Consequences ... ## Alternatives rejected ... ## Verification

`changes:` is the record's one machine field. It names, by rule id, what the decision created and what
it modified; every rule's `because` names the records back, and its first entry is the creator. That
relation is never maintained by hand in either direction beyond writing the two lists once: `--graph`
derives from it the history `Amends:`/`Amended by:` used to carry, and the `changes` check reports the
two lists disagreeing. `python tools/rules.py --show <id>` is the other side of the same question.

WHAT IT CHECKS, AND WHY EACH ONE IS HERE RATHER THAN IN A REVIEWER'S HEAD

  metadata   The YAML block opens the file and closes; `date` is ISO; `status` is one of the allowed
             values and nothing more, because a status carrying a paragraph is a status nobody can
             filter on; the metadata bullets after the title are `Scope`, `Depends on` and
             `Validated by` and nothing else, since a typo'd field name reads as "this record has
             none"; the `In short` block is there, because it is the record's front page.

  changes    Every id under `changes:` is `<topic>/<slug>`, resolves in the rulebook, and that rule's
             `because` names the record back -- and every `because` entry is a record whose `changes:`
             names the rule. The reverse index is how a reader of a rule finds the decisions behind
             it, so a one-directional entry is the one-directional fold this replaced amend symmetry
             to catch, and it is a check because the reader of the rule is the one who never finds out.

  structure  The heading set is closed and its order is fixed, so a reader who has found
             *Alternatives rejected* once knows where it is in every other record. The corpus had
             four different placements for `## Verification` before this ran. No section is empty.

  links      Every link to a record must resolve, from a record or from the two index documents in
             `docs/adr/` that link to records without being one, and every `../` link from any of
             them must name a file that is on disk.

  section    Every `§ N` citing another record must name a section that record actually has. A
  refs       section number is a public identifier -- cited from other records, from `crates/` and
             from the goal manifests -- and one that was renumbered is the failure this set is most
             exposed to, because nothing else notices.

             A `§ N` counts as citing *another* record only when that record is named directly in
             front of it: `[0067] § 13`, `[0104]'s § 3`. Prose in between means the section belongs
             to the record doing the writing, which is how 0044 and 0084 cite their own § 7 and § 8 a
             clause after naming someone else. Reading across that clause finds 1,031 more citations
             than matching the link text alone, and four of them are wrong; refusing to costs three
             citations of a shape nobody writes twice. A gate that cries wolf gets ignored, so this
             takes the narrow rule and 1,288 checked citations.

  residue    A frozen body states the decision as it was made -- git holds the history. Prose like
             "previously said", "is withdrawn", "used to" is an overlay from the folding era, when a
             body was rewritten in place, and a reader would still have to apply it in their head.

  counters   A count restated in more than one file goes stale; one such total had been wrong in
             seven places before this ran. Any spelled-out running total in a body is reported.

Nothing here measures prose against a length. doc-style.md § *Length targets* is explicit that
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
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rules as rulebook  # noqa: E402  -- the rulebook library; `changes:` resolves against it

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
#: The frozen decision records, one file per number.
ADR_DIR = os.path.join(ROOT, "docs", "decisions")
#: The two documents that link *to* records without being one -- the set's README and the tooling
#: parity table. They sit in `docs/adr/` for historical reasons and are checked alongside the set,
#: because they are where a link to a record is written last and looked at least.
INDEX_DIR = os.path.join(ROOT, "docs", "adr")
INDEX_DOCS = ["README.md", "tooling-parity.md"]
#: A link to a record, from a record (`(0066.md)`) or from an index document (`(../decisions/0066.md)`).
#: The group is the number.
RECORD_LINK_RE = re.compile(r"\]\((?:\.\./decisions/)?(\d{4})\.md(?:#[^)]*)?\)")

#: The metadata bullets a record may carry between its title and its `In short` block. A name outside
#: this list is a typo, per the module doc; `Status` and `Date` live in the YAML block and are
#: parsed into the same `fields` dict so a caller reads one shape.
FIELDS = ["Scope", "Depends on", "Validated by"]
STATUS_RE = re.compile(r"^(?:accepted|retired|superseded-by (\d{4}))$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")

# The closed heading set, in canonical order. Front-loading is the `In short` block's job, not the
# section order's -- a reader who only needs the rule has the rulebook -- so `Context` leading is the
# house shape and what this enforces is that every record uses the same one.
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
# Sections a reader skips unless they intend to overturn the decision -- what `--stats` sizes.
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
# *Alternatives rejected* and *Verification* name rejected and withdrawn things for a living, so
# residue is looked for everywhere in the body but there.
RESIDUE_SKIP = {"Alternatives rejected", "Verification"}
COUNTERS = [
    r"\bthe (?:tenth|eleventh|twelfth|thirteenth|fourteenth) divergence\b",
    r"\bnow holds (?:three|four|five|six|seven|eight|nine|ten) names\b",
    r"\bexactly those (?:three|four|five|six|seven) names\b",
    r"\bthe (?:third|fourth|fifth|sixth|seventh) forcing case\b",
]

#: A markdown link's target, as `](0007.md)` or `](../decisions/0007.md)`. `section_refs` collapses it
#: so a citation reads as `[0007] § 4`; the `.md` in it is otherwise a sentence break to any scan.
LINK_TARGET_RE = re.compile(r"\]\((?:\.\./decisions/)?\d{4}\.md(?:#[^)]*)?\)")
SECTION_CITE_RE = re.compile(r"§§?\s*(\d+[a-z]?)")
#: A `§ N` cites *another* record only when that record is named right in front of it -- `[0067] § 13`,
#: `[0104]'s § 3`. Anything else between the two is prose, and prose means the `§` belongs to the
#: record doing the writing. A window wide enough to reach across a clause reads every one of those as
#: a cross-reference and reports it as dangling. `E0122` is a diagnostic code, so a digit run preceded
#: by a letter never counts; `2026` is a year, so a record number always leads with a zero.
CROSS_CITE_RE = re.compile(r"(?<![A-Za-z0-9])(0\d{3})\]?(?:'s)?[\s,]*$")


class Adr:
    """One record, parsed. `fields` carries `Status` and `Date` from the YAML block -- `Status`
    capitalised, so `decisions.py`'s filter on `Accepted` reads the same value it always has -- and
    the metadata bullets by name; `status`, `date` and `changes` are the block's own values."""

    def __init__(self, path: str, text: str | None = None) -> None:
        self.path = path
        self.file = os.path.basename(path)
        self.num = self.file[:4]
        self.text = open(path, encoding="utf-8").read() if text is None else text
        self.lines = self.text.split("\n")
        self.title = next((l.lstrip("# ").strip() for l in self.lines if l.startswith("# ")), "")
        self.front_matter = False
        self.status = ""
        self.date = ""
        #: `{"creates": [rule ids], "modifies": [rule ids]}`, exactly the lists written; a list the
        #: block does not carry is absent, which `check_changes` reports.
        self.changes: dict[str, list[str]] = {}
        self.fields: dict[str, str] = {}
        self.field_line: dict[str, int] = {}
        #: Lines the shape does not admit: an unknown bullet name, an unknown YAML key.
        self.unknown: list[tuple[int, str]] = []
        self.headings: list[tuple[int, str]] = []
        self.sections: dict[str, tuple[int, int]] = {}
        self.subsections: set[str] = set()
        self.in_short = False
        self._parse()

    def _parse_front(self) -> int:
        """The YAML block, by hand: three keys and two lists is not a parser's worth of shape, and
        pulling one in would make a check-only tool the one script in `tools/` with a dependency.
        Returns the line the body starts after, 0 when there is no block."""
        if not self.lines or self.lines[0] != "---":
            return 0
        key = None
        for i, line in enumerate(self.lines[1:], 2):
            if line == "---":
                self.front_matter = True
                return i
            m = re.match(r"^(\w+):\s*(.*?)\s*$", line)
            if m:
                name, value = m.group(1), m.group(2)
                key = None
                if name in ("date", "status"):
                    setattr(self, name, value)
                    self.fields[name.capitalize()] = value.capitalize() if name == "status" else value
                    self.field_line[name.capitalize()] = i
                elif name == "changes":
                    self.field_line["changes"] = i
                else:
                    self.unknown.append((i, name))
                continue
            m = re.match(r"^  (creates|modifies):\s*(.*?)\s*$", line)
            if m:
                key, inline = m.group(1), m.group(2)
                # `modifies: []` is the inline spelling of an empty list; `[a, b]` is the same
                # shape with members, which nothing writes today but the migration doc shows.
                self.changes[key] = [x.strip() for x in inline.strip("[]").split(",") if x.strip()]
                continue
            m = re.match(r"^    - (\S+)\s*$", line)
            if m and key:
                self.changes[key].append(m.group(1))
                continue
            self.unknown.append((i, line.strip()[:40]))
        return 0

    def _parse(self) -> None:
        body_from = self._parse_front()
        cur = None
        for i, line in enumerate(self.lines, 1):
            if i <= body_from:
                continue
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
        """Every record this one links to, by number."""
        return set(RECORD_LINK_RE.findall(self.text))

    def rules(self) -> list[str]:
        """Every rule id under `changes:`, created first, in the order written."""
        return self.changes.get("creates", []) + self.changes.get("modifies", [])

    def section_refs(self) -> list[tuple[int, str, str]]:
        """(line, target record number, section number) for every `0007 § 3`-shaped citation.

        Read backwards from the `§`, not forwards from the number. A citation names its target
        immediately before the section mark, so the target is the *last* record number in the run of
        text ahead of it; scanning forward from every number instead makes
        `[0007](...) and [0009](...) § 2` a claim about 0007 as well, which it is not."""
        out = []
        for i, line in enumerate(self.lines, 1):
            # `[0007](0007.md) § 4` is how a citation is nearly always written, and the `.md` in the
            # link target used to stop the scan dead -- so the one form in common use was the one
            # form never checked, and nine dangling `§ N` had accumulated behind it. Collapsing the
            # target to `]` leaves `[0007] § 4` and moves no column onto another line.
            plain = LINK_TARGET_RE.sub("]", line)
            for m in SECTION_CITE_RE.finditer(plain):
                cite = CROSS_CITE_RE.search(plain[max(0, m.start() - 40):m.start()])
                if cite and cite.group(1) != self.num:
                    out.append((i, cite.group(1), m.group(1)))
        return out


def load() -> dict[str, Adr]:
    return {a.num: a for a in (Adr(p) for p in sorted(glob.glob(os.path.join(ADR_DIR, "0*.md"))))}


# ---------------------------------------------------------------- checks


def check_metadata(adrs):
    out = []
    for a in adrs.values():
        if not a.front_matter:
            out.append((a.file, 1, "no YAML block -- a record opens with `---` and closes it"))
        for line, name in a.unknown:
            out.append((a.file, line, f"unknown metadata field `{name}` -- the bullets are "
                                      f"{', '.join(FIELDS)}; the block is date, status, changes"))
        for req in ("Status", "Date", "Scope"):
            if req not in a.fields:
                out.append((a.file, 1, f"missing `{req.lower() if req != 'Scope' else req}:`"))
        if a.date and not DATE_RE.match(a.date):
            out.append((a.file, a.field_line["Date"], f"`date:` is not ISO: {a.date}"))
        m = STATUS_RE.match(a.status) if a.status else None
        if a.status and not m:
            short = a.status if len(a.status) < 60 else a.status[:57] + "..."
            out.append((a.file, a.field_line["Status"],
                        f"`status:` is not accepted | retired | superseded-by NNNN: {short}"))
        elif m and m.group(1) and m.group(1) not in adrs:
            out.append((a.file, a.field_line["Status"],
                        f"superseded by {m.group(1)}, which does not exist"))
        if not a.in_short:
            out.append((a.file, 1, "no `> **In short:**` block"))
    return out


def check_changes(adrs):
    out = []
    book = rulebook.Rulebook()
    for a in adrs.values():
        at = a.field_line.get("changes", 1)
        if "changes" not in a.field_line:
            out.append((a.file, 1, "no `changes:` block -- name the rules this creates and modifies"))
            continue
        for key in ("creates", "modifies"):
            if key not in a.changes:
                out.append((a.file, at,
                            f"`changes:` has no `{key}:` list -- write `{key}: []` for none"))
        seen: set[str] = set()
        for rid in a.rules():
            if not rulebook.ID_RE.match(rid):
                out.append((a.file, at, f"`changes:` id {rid!r} is not `<topic>/<slug>`"))
            elif rid in seen:
                out.append((a.file, at, f"`changes:` names {rid} twice"))
            elif book.by_id and rid not in book.by_id:
                out.append((a.file, at, f"`changes:` names {rid}, which the rulebook does not define"))
            elif book.by_id and a.num not in book.by_id[rid].because:
                out.append((a.file, at,
                            f"`changes:` names {rid}, whose `because` does not name {a.num}"))
            seen.add(rid)
    for rule in book.by_id.values():
        for i, num in enumerate(rule.because):
            a = adrs.get(num)
            if a is None:
                out.append((f"docs/rules/{rule.topic}.json", 1,
                            f"{rule.id}'s `because` names {num}, which is not a record"))
            elif rule.id not in a.rules():
                out.append((a.file, a.field_line.get("changes", 1),
                            f"{rule.id}'s `because` names {num}, "
                            "but its `changes:` does not name the rule"))
            elif i == 0 and rule.id not in a.changes.get("creates", []):
                out.append((a.file, a.field_line.get("changes", 1),
                            f"{rule.id}'s `because` puts {num} first, "
                            "but its `changes:` does not create it"))
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
                at = next(line for line, name in a.headings if name == n)
                out.append((a.file, at, f"non-canonical heading `## {n}`"))
        ordered = [n for n in names if n in CANONICAL]
        rank = [CANONICAL.index(n) for n in ordered]
        if rank != sorted(rank):
            out.append((a.file, 1, f"sections out of canonical order: {' -> '.join(ordered)}"))
        for n, (s, e) in a.sections.items():
            if not "\n".join(a.lines[s:e]).strip():
                out.append((a.file, s, f"`## {n}` is empty"))
    return out


def _link_sources(adrs):
    """(file, lines, the directory its relative links resolve from)."""
    for a in adrs.values():
        yield a.file, a.lines, ADR_DIR
    for name in INDEX_DOCS:
        path = os.path.join(INDEX_DIR, name)
        if os.path.exists(path):
            yield name, open(path, encoding="utf-8").read().split("\n"), INDEX_DIR


def check_links(adrs):
    out = []
    for file, lines, base in _link_sources(adrs):
        for i, line in enumerate(lines, 1):
            for m in RECORD_LINK_RE.finditer(line):
                if m.group(1) not in adrs:
                    out.append((file, i, f"broken record link -> {m.group(1)}"))
            for m in re.finditer(r"\]\((\.\./[^)#]+)(?:#[^)]*)?\)", line):
                rel = os.path.normpath(os.path.join(base, m.group(1)))
                if not os.path.exists(rel):
                    out.append((file, i, f"broken relative link -> {m.group(1)}"))
    return out


def check_section_refs(adrs):
    out = []
    for a in adrs.values():
        for line, num, sec in a.section_refs():
            t = adrs.get(num)
            if t is None:
                out.append((a.file, line, f"cites record {num}, which does not exist"))
            elif t.subsections and sec not in t.subsections:
                have = ", ".join(sorted(t.subsections, key=lambda s: (len(s), s)))
                out.append((a.file, line, f"cites {num} § {sec}; that record has §§ {have}"))
    return out


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
                    out.append((a.file, i, "running count in prose -- keep the total in one home: "
                                           f"{line.strip()[:90]}"))
                    break
    return out


CHECKS = [
    ("metadata", check_metadata),
    ("changes", check_changes),
    ("structure", check_structure),
    ("links", check_links),
    ("section refs", check_section_refs),
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
    print(f"\n{len(adrs)} records, {tot_l} lines, {tot_b} bytes; "
          f"{tot_r} bytes ({100 * tot_r // tot_b}%) in rationale sections")


def graph(adrs, num):
    """One record's place in the set, derived from `changes:` and the rulebook's `because`. This is
    the amend history the retired `Amends:`/`Amended by:` fields carried, read off the relation
    instead of maintained beside it: for each rule the record touches, who created it and who else
    shaped it. `because` is creator-first and otherwise unordered, so the others are listed by
    number, which is the order the decisions were taken in."""
    a = adrs.get(num)
    if a is None:
        print(f"no record {num}")
        return 1
    book = rulebook.Rulebook()
    cited_by = sorted(n for n, o in adrs.items() if num in o.refs() and n != num)
    print(f"{a.file}\n  {a.title}\n")
    print(f"  {'date:':<15}{a.date}\n  {'status:':<15}{a.status}")
    for f in FIELDS:
        if f in a.fields:
            v = a.fields[f]
            print(f"  {f.lower() + ':':<15}{v[:110]}{'...' if len(v) > 110 else ''}")
    for key in ("creates", "modifies"):
        ids = a.changes.get(key, [])
        print(f"\n  {key} ({len(ids)}):")
        for rid in ids:
            rule = book.by_id.get(rid)
            if rule is None:
                print(f"    {rid:<58} not in the rulebook")
                continue
            others = sorted(n for n in rule.because if n != num)
            if key == "creates":
                note = f"also shaped by {', '.join(others)}" if others else "no other record"
            else:
                creator = rule.because[0] if rule.because else "?"
                rest = [n for n in others if n != creator]
                note = f"created by {creator}" + (f"; also {', '.join(rest)}" if rest else "")
            print(f"    {rid:<58} {note}")
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
    print("records no other record links to:")
    for n in sorted(adrs):
        if not inbound[n]:
            print(f"  {n}  {adrs[n].title[:88]}")
    print("\nmost-cited:")
    for n, s in sorted(inbound.items(), key=lambda kv: -len(kv[1]))[:12]:
        print(f"  {n}  {len(s):>3} inbound")


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--stats", action="store_true", help="size and section shape, one line per record")
    p.add_argument("--graph", metavar="NNNN",
                   help="the rules one record creates and modifies, and who else shaped them")
    p.add_argument("--orphans", action="store_true", help="records nothing links to, and the most cited")
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
    if args.residue:
        return 1 if report(adrs, only={"changelog residue"}) else 0
    n = report(adrs, only=set(args.only) if args.only else None, quiet=args.check)
    if n:
        print(f"\n{n} finding(s) across {len(adrs)} records")
    elif not args.check:
        print(f"\nclean: {len(adrs)} records")
    return 1 if n and args.check else 0


if __name__ == "__main__":
    sys.exit(main())
