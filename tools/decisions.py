#!/usr/bin/env python3
"""The decision summary a human reads, and the bookkeeping that keeps it true.

The records under `docs/decisions/` are every settled decision at full length, frozen on acceptance
and written for the agent that has to revisit one: 143 files, each carrying the question, the
options weighed and the cost accepted. For a reader who wants to know *what Novis decided*, that is
a wall. This tool owns the other artifact: one plain-language paragraph per decision, grouped by
topic, in the order the decisions were taken, with no cross-reference in the prose at all.

    python tools/decisions.py              the summary, rendered to the terminal
    python tools/decisions.py --check      what the summary owes; quiet on success
    python tools/decisions.py --gate       only the findings that are always wrong; the CI shape
    python tools/decisions.py --work       the work order for a pass: only what is missing or stale
    python tools/decisions.py --apply FILE merge written entries, transactionally
    python tools/decisions.py --render     rewrite the two derived artifacts from the source
    python tools/decisions.py --json       the website feed, on stdout
    python tools/decisions.py --groups     the closed group list, with what belongs in each

## The one home, and the two derived from it

`docs/decisions.toml` is the source. Every entry in it is prose a person wrote; nothing in it can be  # check-links:retired
derived from an ADR mechanically, which is why it is a file and not a `--render` of the corpus.

`docs/decisions.md` and `website/src/data/decisions.json` are **generated** -- `--render` writes both
and `--check` reports either one being stale. Never edit them; the edit is lost on the next pass and
the file that lost it does not say so.

## Why an entry carries a digest

A pass that re-summarized all 143 decisions every time would cost a session per run and re-word
entries nobody asked it to touch, so the summary would drift while the records stood still. Instead
each entry stamps a digest of the record it summarizes -- the frozen title, and the `changes:` block
naming the rule ids the decision created and modified. `--check` recomputes it: an entry whose
digest still matches is **known current** and is never looked at again, and a pass therefore does
only the records that are new or whose digest moved. That is what makes re-running this cheap
enough to be worth doing, which is the whole point of it being a tool.

Nothing else of a record is in the digest, because nothing else of it moves: a record is frozen on
acceptance and never amended in place. The `changes:` block is the one field that is *derived* --
from every rule's `because` in the rulebook -- so a rule re-homed or re-attributed re-derives it,
and that is exactly when a summary is worth a second look. A digest formula that changes (this one
did, when the records were frozen) marks every entry stale at once; `--check` then reports a
re-pass owed, and `--work` is the order for it. The stamps are never edited by hand.

## The rules `--check` enforces on the prose, and why they are mechanical

The brief for this document is "clear, short, simple words, no cross-references." Three of those
four are judgment and stay the author's. The fourth is not, and neither is length, so they are
checked rather than hoped for:

  refs     No `ADR`, no `§`, no bare four-digit number, no markdown link. A summary that sends the
           reader somewhere else has failed at the one thing it exists to do. The ADR link belongs
           to the *renderer*, which puts one discreet link per entry on the website and none in the
           prose -- so a reader who wants the full decision can reach it without the text being
           about where to reach it.
  size     A headline is one scannable line; a body is two to four sentences. The caps are here
           because 140 entries is a document, and a document whose entries drift to a screen each
           is the corpus again with extra steps.
  jargon   A short blocklist of the words this repository uses correctly and a reader does not
           know. It is a list you edit when it is wrong, not a taste test.

## The groups

A closed list, in reading order, `internal` last. A group is an editorial decision about what a
reader is looking for, not a restatement of the rulebook's chapters -- `docs/rules/` answers a
different question ("which rule is currently true, and where") with a different answer, and
`brief.py --where` routes it. `--groups` prints the list and what belongs in each; `--check`
refuses an entry naming a group that is not on it.

Inside a group, entries run in the order they were decided, with one exception: `pin = true` floats
a single entry to the top. It is for the entry that *frames* the group rather than continuing it —
what the language is for belongs above the decisions that follow from it, and decision order would
bury it in the middle. **At most one per group**, checked, because a second pin is the beginning of
hand-sorting a hundred and thirty-nine entries and there is no honest place to stop after that.
"""

import argparse
import hashlib
import io
import json
import os
import re
import sys
import textwrap

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import records as adrlib  # noqa: E402  -- the Adr parser has one home, and this is it

# `adr` already replaced `sys.stdout` with a utf-8 wrapper on import. Wrapping *that* one's buffer
# a second time leaves the first wrapper unreferenced, and closing it on collection closes the
# buffer under this one -- so reconfigure what is there rather than adding a layer.
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover -- 3.11 is the floor everything else here assumes
    print("decisions.py needs Python 3.11 or newer for tomllib", file=sys.stderr)
    sys.exit(2)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOURCE = os.path.join(ROOT, "docs", "decisions.toml")
RENDER_MD = os.path.join(ROOT, "docs", "decisions.md")
RENDER_JSON = os.path.join(ROOT, "website", "src", "data", "decisions.json")

#: Where a decision record is read on the web. The website links out to it rather than republishing
#: it: `website/config/site.mjs` holds the same two constants for everything on the site's own side.
GITHUB_BLOB = "https://github.com/novis-lang/novis/blob/main"

#: (id, heading, what belongs here). The heading is what a reader sees; the third column is what an
#: author consults to place an entry, and is the only description of a group anywhere.
GROUPS = [
    (
        "foundations",
        "What Novis is",
        "Who the language is for, what it refuses to be, and the ground the rest stands on: "
        "how errors travel, what a running script is, what ships in the box.",
    ),
    (
        "types",
        "Types and values",
        "What a value can be and how its type behaves: the type system's own rules, the scalar "
        "types, conversion, equality, and the shapes a type can take.",
    ),
    (
        "syntax",
        "How code is written",
        "Spelling. What parses and what does not, which PHP forms were kept and which were "
        "rejected, naming, visibility, and the shape of a file.",
    ),
    (
        "language",
        "Language features",
        "What the language gives you to build with: classes and their members, interfaces, "
        "closures, iteration, concurrency, attributes, testing.",
    ),
    (
        "security",
        "Security and isolation",
        "The decisions that exist because the code and the data are not trusted: qualifiers on "
        "values, what a request can reach, what an extension may do, what the doors are.",
    ),
    (
        "core",
        "The standard library and runtime",
        "What is built in and how it behaves: the Core namespace's own conventions, the "
        "components that ship with it, and how the runtime serves a request.",
    ),
    (
        "tooling",
        "Tools, editors and shipping",
        "Everything around the language: the command-line tool, the editor experience, "
        "formatting, packaging, deployment, and what the tools may and may not do for you.",
    ),
    (
        "internal",
        "Engineering decisions",
        "Decisions about how Novis itself is built and measured. Real decisions, kept for the "
        "record, but a reader learning the language can skip the group entirely.",
    ),
]
GROUP_IDS = [g[0] for g in GROUPS]
GROUP_BY_ID = {g[0]: g for g in GROUPS}

HEADLINE_MAX = 78
BODY_MIN_WORDS = 20
BODY_MAX_WORDS = 95

#: Words this repository uses precisely and a reader arriving from PHP does not know. Each is here
#: because it says nothing to the audience, not because it is wrong -- the ADR that owns the topic
#: is where the precise word belongs.
JARGON = [
    "lowering", "lowers to", "safepoint", "monomorph", "arity", "codegen", "ABI", "IR",
    "vtable", "trampoline", "prologue", "epilogue", "call site", "sans-io", "reentrant",
    "idempotent", "invariant", "orthogonal", "canonicalize", "normative",
]

BANNED_REFS = [
    (re.compile(r"\bADRs?\b"), "names an ADR"),
    (re.compile(r"§"), "uses a section mark"),
    (re.compile(r"(?<![\w.])0\d{3}(?![\w.])"), "cites a decision by number"),
    (re.compile(r"\]\("), "carries a markdown link"),
    (re.compile(r"\bamend(s|ed|ment|ments)?\b", re.I), "talks about amendments"),
    (re.compile(r"\bsupersed(e|es|ed|ing)\b", re.I), "talks about superseding"),
]


# ---------------------------------------------------------------- the corpus side


def title_of(a) -> str:
    """The decision itself, without the `ADR NNNN —` the heading carries for the corpus's sake."""
    return re.sub(r"^ADR\s+\d{4}\s*[—-]\s*", "", a.title).strip()


def in_short(a) -> str:
    """The `In short` blockquote, as one line of plain text."""
    out = []
    for line in a.lines:
        if line.startswith("> **In short:**"):
            out.append(line[len("> **In short:**"):])
        elif out and line.startswith(">"):
            out.append(line[1:])
        elif out:
            break
    text = " ".join(" ".join(out).split())
    return re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)


def shape(a) -> list[str]:
    """The Decision section's numbered headings -- what the decision is made of."""
    out = []
    start, end = a.sections.get("Decision", (0, 0))
    for line in a.lines[start:end]:
        m = re.match(r"^#{3,4} (\d+[a-z]?\.\s*)?(.+)$", line)
        if m:
            out.append(m.group(2).strip())
    return out


def frontmatter(a) -> dict:
    """The frozen record's YAML block, read off its own lines: `status`, and under `changes:` the
    `creates` and `modifies` lists of rule ids. Read here rather than through the parser's fields
    so this tool's one dependency on `records.py` stays the record's text and title.

    The shape is fixed by the freeze (docs/agent/conventions.md, *A decision record*): scalars as
    `key: value`, the two lists as `    - id` items under their key."""
    out: dict = {"status": "", "creates": [], "modifies": []}
    if not a.lines or a.lines[0] != "---":
        return out
    current = None
    for line in a.lines[1:]:
        if line == "---":
            break
        m = re.match(r"^(status):\s*(.+?)\s*$", line)
        if m:
            out[m.group(1)] = m.group(2)
            current = None
            continue
        m = re.match(r"^\s+(creates|modifies):\s*$", line)
        if m:
            current = m.group(1)
            continue
        m = re.match(r"^\s+-\s+(\S+)\s*$", line)
        if m and current:
            out[current].append(m.group(1))
    return out


def digest_of(a) -> str:
    """What a summary of this record was written from, hashed: the frozen title and the
    `changes:` block. See the module doc on why nothing else of a record is in here."""
    fm = frontmatter(a)
    material = "\n".join([a.title,
                          "creates: " + " ".join(fm["creates"]),
                          "modifies: " + " ".join(fm["modifies"])])
    return hashlib.sha256(material.encode("utf-8")).hexdigest()[:12]


def corpus() -> dict:
    """Every accepted record, by number. A retired one is not a decision the summary owes."""
    return {n: a for n, a in adrlib.load().items() if frontmatter(a)["status"] == "accepted"}


# ---------------------------------------------------------------- the source file


def load_entries(path: str = SOURCE) -> dict:
    if not os.path.exists(path):
        return {}
    with open(path, "rb") as f:
        data = tomllib.load(f)
    out = {}
    for e in data.get("entry", []):
        e = dict(e)
        e["body"] = " ".join(str(e.get("body", "")).split())
        e["headline"] = " ".join(str(e.get("headline", "")).split())
        e["pin"] = bool(e.get("pin", False))
        out[str(e.get("adr", "")).zfill(4)] = e
    return out


def toml_string(value: str) -> str:
    """A TOML literal string. Single quotes take no escapes, so a value carrying one goes basic."""
    if "'" not in value:
        return f"'{value}'"
    return json.dumps(value)


def dump_entries(entries: dict) -> str:
    out = [
        "# The plain-language decision summary -- the one home for it.",
        "#",
        "# One entry per accepted decision, written for a reader who has never opened an ADR.",
        "# `docs/decisions.md` and `website/src/data/decisions.json` are generated from this file",
        "# and must never be edited; `python tools/decisions.py --render` writes them both.",
        "#",
        "# `digest` is stamped by the tool. Write `adr`, `group`, `headline` and `body`,",
        "# and let `python tools/decisions.py --apply` fill in the rest.",
        "#",
        "# `pin = true` floats one entry to the top of its group, ahead of decision order, for the",
        "# one that frames what the group is about. At most one per group.",
        "",
    ]
    for num in sorted(entries):
        e = entries[num]
        out.append("[[entry]]")
        out.append(f"adr      = '{num}'")
        out.append(f"group    = '{e['group']}'")
        if e.get("pin"):
            out.append("pin      = true")
        out.append(f"digest   = '{e['digest']}'")
        out.append(f"headline = {toml_string(e['headline'])}")
        body = textwrap.fill(e["body"], width=96, break_long_words=False, break_on_hyphens=False)
        out.append("body     = '''")
        out.append(body)
        out.append("'''")
        out.append("")
    return "\n".join(out)


# ---------------------------------------------------------------- checks


def prose_findings(num: str, e: dict) -> list[str]:
    out = []
    head, body = e.get("headline", ""), e.get("body", "")
    if not head:
        out.append("headline is empty")
    if len(head) > HEADLINE_MAX:
        out.append(f"headline is {len(head)} chars, over {HEADLINE_MAX}")
    if head.endswith("."):
        out.append("headline ends in a period")
    words = len(body.split())
    if words < BODY_MIN_WORDS:
        out.append(f"body is {words} words, under {BODY_MIN_WORDS}")
    if words > BODY_MAX_WORDS:
        out.append(f"body is {words} words, over {BODY_MAX_WORDS}")
    for pattern, why in BANNED_REFS:
        for field, text in (("headline", head), ("body", body)):
            m = pattern.search(text)
            if m:
                out.append(f"{field} {why}: {m.group(0)!r}")
    lowered = f"{head} {body}".lower()
    for word in JARGON:
        if re.search(rf"(?<![\w-]){re.escape(word.lower())}(?![\w-])", lowered):
            out.append(f"uses {word!r}, which the audience does not read")
    return [f"{num}: {f}" for f in out]


def check(entries: dict, adrs: dict) -> dict:
    """Every finding, grouped by kind. An empty dict is a clean tree."""
    found: dict[str, list[str]] = {}

    def add(kind: str, msg: str):
        found.setdefault(kind, []).append(msg)

    for num in sorted(adrs):
        if num not in entries:
            add("missing", f"{num}: {title_of(adrs[num])[:70]}")
    for num in sorted(entries):
        e = entries[num]
        if num not in adrs:
            add("orphan", f"{num}: summarized, but no accepted decision has that number")
            continue
        if e.get("group") not in GROUP_BY_ID:
            add("group", f"{num}: unknown group {e.get('group')!r}")
        if e.get("digest") != digest_of(adrs[num]):
            add("stale", f"{num}: the digest moved since this was written")
        for f in prose_findings(num, e):
            add("prose", f)

    for gid in GROUP_IDS:
        pinned = [n for n in sorted(entries)
                  if entries[n].get("group") == gid and entries[n].get("pin")]
        if len(pinned) > 1:
            add("pin", f"{gid}: {', '.join(pinned)} are all pinned — a group is framed once")

    if entries:
        for path, text in ((RENDER_MD, render_md(entries, adrs)),
                           (RENDER_JSON, render_json(entries, adrs))):
            on_disk = open(path, encoding="utf-8").read() if os.path.exists(path) else None
            if on_disk != text:
                rel = os.path.relpath(path, ROOT).replace("\\", "/")
                add("render", f"{rel} is not what the source renders to -- run --render")
    return found


# ---------------------------------------------------------------- rendering


def grouped(entries: dict, adrs: dict) -> list[tuple]:
    """(group tuple, entries in decision order, pinned one first). Empty groups are dropped."""
    out = []
    for gid, heading, blurb in GROUPS:
        nums = [n for n in sorted(entries) if entries[n].get("group") == gid]
        # `sorted` is stable, so the pinned entry moves to the front and everything behind it keeps
        # the order it was decided in.
        nums.sort(key=lambda n: not entries[n].get("pin", False))
        if nums:
            out.append(((gid, heading, blurb), [entries[n] for n in nums]))
    return out


def render_md(entries: dict, adrs: dict) -> str:
    out = [
        "# What Novis has decided",
        "",
        "<!-- Generated by `python tools/decisions.py --render` from docs/decisions.toml.",  # check-links:retired
        "     Do not edit: the next render overwrites it without saying so. -->",
        "",
        "Every decision this project has taken, in plain language, grouped by what it is about and",
        "in the order it was decided. Each entry says what is true now, not how it got there.",
        "",
        "The full reasoning behind any one of them -- the alternatives weighed, the costs accepted,",
        "the exact wording -- lives in the frozen records under `docs/decisions/`, each reached through",
        "the rule it changed in [the rulebook](ground-rules.md).",
        "",
    ]
    for (_, heading, blurb), rows in grouped(entries, adrs):
        out += [f"## {heading}", "", blurb, ""]
        for e in rows:
            out.append(f"**{e['headline']}**")
            out.append("")
            out.append(textwrap.fill(e["body"], width=100, break_long_words=False,
                                     break_on_hyphens=False))
            out.append("")
    return "\n".join(out).rstrip() + "\n"


def render_json(entries: dict, adrs: dict) -> str:
    """The website feed. No timestamp -- a generated file that churns every run is noise in `git

    log` and a conflict for the loop."""
    data = {
        "groups": [
            {
                "id": gid,
                "title": heading,
                "blurb": blurb,
                "entries": [
                    {
                        "adr": e["adr"],
                        "headline": e["headline"],
                        "body": e["body"],
                        # Where the full record is read. The site publishes the rulebook --
                        # what is true now -- and not the frozen rationale behind it, so this
                        # leaves for the repository rather than naming a page.
                        "url": f"{GITHUB_BLOB}/docs/decisions/{e['adr']}.md",
                    }
                    for e in rows
                ],
            }
            for (gid, heading, blurb), rows in grouped(entries, adrs)
        ]
    }
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


# ---------------------------------------------------------------- commands


def cmd_work(entries: dict, adrs: dict, group: str | None, limit: int) -> int:
    """The work order: the raw material for every decision the summary does not yet hold."""
    owed = []
    for num in sorted(adrs):
        e = entries.get(num)
        if e is None:
            owed.append((num, "new"))
        elif e.get("digest") != digest_of(adrs[num]):
            owed.append((num, "changed"))
    if group:
        owed = [(n, w) for n, w in owed if entries.get(n, {}).get("group") == group]
    total = len(owed)
    if not total:
        print(f"nothing owed: all {len(adrs)} decisions are summarized and current")
        return 0
    shown = owed[:limit] if limit else owed
    print(f"# {total} decision(s) owed; {len(shown)} below. Write an entry for each, then apply:")
    print("#   python tools/decisions.py --apply <file>")
    print(f"# Groups: {', '.join(GROUP_IDS)}   (python tools/decisions.py --groups)")
    print()
    for num, why in shown:
        a = adrs[num]
        print("# " + "-" * 94)
        print(f"# {num} ({why}) — {title_of(a)}")
        for line in textwrap.wrap(in_short(a), width=94):
            print(f"#   {line}")
        parts = shape(a)
        if parts:
            print("# the decision has these parts:")
            for p in parts:
                print(f"#   - {p}")
        print()
        print("[[entry]]")
        print(f"adr      = '{num}'")
        print("group    = ''")
        print("headline = ''")
        print("body     = '''\n'''")
        print()
    if total > len(shown):
        print(f"# {total - len(shown)} more owed; re-run --work after applying these.")
    return 0


def cmd_apply(path: str, entries: dict, adrs: dict, dry_run: bool) -> int:
    incoming = load_entries(path)
    if not incoming:
        return fail(f"{path} holds no [[entry]] blocks")
    merged = dict(entries)
    touched = []
    for num, e in incoming.items():
        if num not in adrs:
            return fail(f"{num} is not an accepted decision — nothing to summarize")
        merged[num] = {
            "adr": num,
            "group": e.get("group", ""),
            "pin": bool(e.get("pin", False)),
            "digest": digest_of(adrs[num]),
            "headline": e.get("headline", ""),
            "body": e.get("body", ""),
        }
        touched.append(num)

    findings = check(merged, adrs)
    blocking = {k: v for k, v in findings.items() if k in ("group", "pin", "prose", "orphan")}
    if blocking:
        print(f"refused — {sum(len(v) for v in blocking.values())} finding(s), nothing written:\n")
        report(blocking)
        return 1

    if dry_run:
        noun = "entry" if len(touched) == 1 else "entries"
        print(f"would write {len(touched)} {noun}: {', '.join(touched)}")
        print(f"would render {os.path.relpath(RENDER_MD, ROOT)} and "
              f"{os.path.relpath(RENDER_JSON, ROOT)}")
        return 0

    write(SOURCE, dump_entries(merged))
    write(RENDER_MD, render_md(merged, adrs))
    write(RENDER_JSON, render_json(merged, adrs))
    still = len([n for n in adrs if n not in merged])
    print(f"applied {len(touched)}: {', '.join(touched)}")
    print(f"{len(merged)} of {len(adrs)} decisions summarized; {still} still owed")
    return 0


def write(path: str, text: str) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 1


def report(found: dict) -> None:
    labels = {
        "missing": "not summarized yet",
        "stale": "the record's title or `changes:` moved since the summary was written; "
                 "a re-pass is owed (python tools/decisions.py --work)",
        "orphan": "summarizes a decision that is not there",
        "group": "not in a group the tool knows",
        "pin": "more than one entry pinned to the top of a group",
        "prose": "breaks a rule the summary is held to",
        "render": "a generated file is behind the source",
    }
    for kind in ("missing", "stale", "orphan", "group", "pin", "prose", "render"):
        rows = found.get(kind)
        if not rows:
            continue
        print(f"{kind} — {labels[kind]} ({len(rows)})")
        for row in rows:
            print(f"  {row}")
        print()


def main() -> int:
    p = argparse.ArgumentParser(
        description="the plain-language decision summary, and the bookkeeping behind it")
    p.add_argument("--check", action="store_true", help="what the summary owes; quiet on success")
    p.add_argument("--gate", action="store_true",
                   help="only the findings that are always wrong; the CI shape")
    p.add_argument("--work", action="store_true", help="the work order: what is missing or stale")
    p.add_argument("--group", metavar="ID", help="with --work: only this group's entries")
    p.add_argument("--limit", type=int, default=25, help="with --work: how many to print (0 = all)")
    p.add_argument("--apply", metavar="FILE", help="merge written entries, then render")
    p.add_argument("--dry-run", action="store_true", help="with --apply: say what it would do")
    p.add_argument("--render", action="store_true", help="rewrite the two generated artifacts")
    p.add_argument("--json", action="store_true", help="the website feed, on stdout")
    p.add_argument("--groups", action="store_true", help="the closed group list")
    args = p.parse_args()

    if args.groups:
        for gid, heading, blurb in GROUPS:
            print(f"{gid:<12} {heading}")
            for line in textwrap.wrap(blurb, width=84):
                print(f"{'':<12} {line}")
            print()
        return 0

    adrs = corpus()
    entries = load_entries()
    if args.group and args.group not in GROUP_BY_ID:
        return fail(f"unknown group {args.group!r} — one of {', '.join(GROUP_IDS)}")

    if args.apply:
        return cmd_apply(args.apply, entries, adrs, args.dry_run)
    if args.work:
        return cmd_work(entries, adrs, args.group, args.limit)
    if args.json:
        print(render_json(entries, adrs), end="")
        return 0
    if args.render:
        if not entries:
            return fail("docs/decisions.toml is empty — nothing to render")  # check-links:retired
        write(RENDER_MD, render_md(entries, adrs))
        write(RENDER_JSON, render_json(entries, adrs))
        print(f"rendered {len(entries)} entries to {os.path.relpath(RENDER_MD, ROOT)} "
              f"and {os.path.relpath(RENDER_JSON, ROOT)}")
        return 0

    found = check(entries, adrs)
    n = sum(len(v) for v in found.values())
    if args.gate:
        # A decision not summarized yet is a pass that has not run, which is a schedule and not a
        # fault -- gating on it would mean every new ADR breaks the build until somebody writes
        # prose. The other four kinds are always a mistake, and none of them can arrive except
        # through an edit to a generated file or a hand-edit that skipped `--apply`.
        bad = {k: v for k, v in found.items() if k not in ("missing", "stale")}
        m = sum(len(v) for v in bad.values())
        if m:
            report(bad)
            print(f"{m} finding(s) that are always wrong")
        return 1 if m else 0
    if args.check:
        if n:
            report(found)
            print(f"{n} finding(s); {len(entries)} of {len(adrs)} decisions summarized")
        return 1 if n else 0

    if not entries:
        print("the summary is empty — start with: python tools/decisions.py --work")
        return 0
    print(render_md(entries, adrs), end="")
    if n:
        print(f"\n[{n} finding(s) — python tools/decisions.py --check]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
