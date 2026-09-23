#!/usr/bin/env python3
"""One call, whole orientation. Prints the parts of this repository an agent reads at the
start of nearly every session: where the plan stands, the map of every milestone plus the
lead of the current one, one line per source module and the anchors sessions hunt for, what
the guard tests hold, and what actually exists on disk.

It stores no facts of its own. Every line it prints is sliced out of a file it names, so it
cannot go stale. When a slice comes back empty it says so loudly rather than printing a
plausible nothing.

Size is controlled by one structural rule:

    this digest may only print text whose length is bounded by a COUNT OF ENTITIES,
    never by a LENGTH OF PROSE.

So a section is a *projection* -- one line per milestone, per module, per guard test, per named
status field. Adding a module, a milestone or a paragraph of prose grows it by a line, never by
a page. Where a legitimately-long *prose* paragraph is only wanted at the head, it is cut by
[`excerpt`] and marked inline with the file and line to open: that is the current/next
milestone's lead, and every status field but the two that steer a session.

The decision index is a *count* by default rather than a line per record. docs/ground-rules.md,
generated from the rulebook, already carries one line per rule with its link, and `--where`
answers "which file owns this topic" far better than 143 title lines -- so printing them all was
a second copy of both. `--adrs` still prints one line per record, read off docs/decisions/, and
any record whose status is not accepted is always printed, because that is the part no other
file states.

**This script measures nothing and enforces nothing.** The length guidance for a status field,
a milestone heading or an ADR decision cell is in AGENTS.md, addressed to the author, and is
deliberately not a check here or in CI -- an agent burning a session trimming bytes to satisfy
a tripwire costs far more than the bytes ever saved.

Usage:  python tools/brief.py                 # the digest
        python tools/brief.py --no-git        # skip the working-tree section
        python tools/brief.py --no-map        # skip the module map
        python tools/brief.py --adrs          # + one line per ADR, as it used to print
        python tools/brief.py --where         # the rulebook's chapters, and the homes that are not rules
        python tools/brief.py --where words   # the rules and homes matching every word
"""

import ast
import re
import subprocess
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan as planmod  # noqa: E402  -- the plan's one API; never reimplemented here
import goals as goalsmod  # noqa: E402  -- side goals have one reader and it is there

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
#: Read for one section only -- 'Decisions taken at project start', the decisions that never had a
#: numbered record. Its routing and index tables were retired by the docs migration's unit C2;
#: `--where` reads the rulebook, and the decision index reads the records.
ADR_README = ROOT / "docs" / "adr" / "README.md"
ADR_DIR = ROOT / "docs" / "decisions"  # the frozen records, `NNNN.md`, since migration unit C1
PROBE = ROOT / "benches" / "abi-probe"
CRATES = ROOT / "crates"
DIAGNOSTICS = CRATES / "nvs-diagnostics" / "src" / "lib.rs"

# --------------------------------------------------------------- display excerpts
#
# Not authoring limits. These bound how much of one legitimately-long *prose* paragraph this
# digest reproduces before pointing at the file; the full text is always one open away.

LEAD_EXCERPT = 700  # current milestone's opening paragraph
VERIFY_EXCERPT = 600  # current milestone's `**Verify:**` paragraph
NEXT_LEAD_EXCERPT = 350  # next milestone's opening paragraph
STATUS_EXCERPT = 600  # one status field, except the two in STATUS_FULL
MODULE_EXCERPT = 104  # one module's line in the map

# The plan's status block has a fixed field set, so it is overwritten in place rather than
# appended to. A missing one is reported below; nothing here rejects an extra one.
STATUS_FIELDS = [
    "Status",
    "Done",
    "On disk",
    "Toolchain",
    "ADR slices landed",
    "Open now",
    "Blocking",
]

# The two fields that answer "what do I do next" print whole; the rest are history and
# inventory, wanted at the head. Both are one open of the plan away either way.
STATUS_FULL = {"Open now", "Blocking"}

# Symbols sessions re-derive with a grep nearly every iteration, resolved live against the
# tree. This list is a judgment about what is worth pointing at -- like STATUS_FIELDS, it is
# the one kind of fact this script holds. The file:line beside each is always sliced, never
# stored, and a pattern that stops matching is reported rather than quietly dropped.
ANCHORS = [
    ("the helper-body macro", r"^macro_rules! nvs_helper\b"),
    ("the roster of `Core` classes", r"^pub const CLASSES\b"),
    ("one class's member rows (one per stdlib module)", r"^pub const CLASS: CoreClass\b"),
    ("what a member's row may say", r"^pub struct CoreMethod\b"),
    ("a `Core` signature's type", r"^pub enum CoreTy\b"),
    ("a helper symbol -> address arm (one per stdlib module)",
     r"^pub\(crate\) fn address\("),
    ("every IR instruction", r"^pub enum InstKind\b"),
    ("a class as the runtime sees it", r"^pub struct ClassDesc\b"),
    ("the checker's entry point", r"^pub fn check_program\b"),
]

GIT_CHANGED_LINE_CAP = 40  # a display cap on `git status` output, not on anything an author writes

out = []


def emit(line=""):
    out.append(line)


def warn(msg):
    emit("")
    emit(f"!! brief.py: {msg}")


def section(title, source):
    emit()
    emit()
    emit(f"== {title}")
    emit(f"-- source: {source}")
    emit()


def nbytes(text):
    return len(text.encode("utf-8"))


def excerpt(text, limit, source_hint):
    """Deliberate shortening of legitimately-long prose, always naming where the rest is."""
    if nbytes(text) <= limit:
        return text
    kept = text.encode("utf-8")[:limit].decode("utf-8", "ignore")
    kept = kept.rsplit(" ", 1)[0]
    return f"{kept} ... [continues at {source_hint}]"


def read(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def rel(path):
    """Relative path with forward slashes, regardless of host OS."""
    return path.relative_to(ROOT).as_posix()


def strip_links(text):
    """``[0002](../decisions/0002.md)`` -> ``0002``. A link target is ~50 bytes of no value in a
    digest whose reader has the rulebook one `--where` call away."""
    return re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)


def wrap(label, text):
    # break_on_hyphens=False so a path like `benches/abi-probe` is never split at its hyphen
    # into something that no longer greps.
    return textwrap.fill(
        f"{label}: {text}",
        width=100,
        subsequent_indent="    ",
        break_long_words=False,
        break_on_hyphens=False,
    )


# ---------------------------------------------------------------- plan status


FIELD_RE = re.compile(r"^\*\*([^*:]+):\*\*\s*(.*)$")


def parse_status_fields(plan_text):
    """The leading blockquote, as (field name, text, line number). A line opening with
    `**Name:**` starts a field; continuation lines fold into it."""
    lines = plan_text.split("\n")
    fields = []
    cur = None
    started = False
    for lineno, raw in enumerate(lines[1:], start=2):  # skip the H1
        if raw.startswith(">"):
            started = True
            body = re.sub(r"^> ?", "", raw).strip()
            m = FIELD_RE.match(body)
            if m:
                cur = [m.group(1).strip(), [m.group(2).strip()], lineno]
                fields.append(cur)
            elif cur is not None and body:
                cur[1].append(body)
        elif started:
            break
    return [(name, " ".join(p).strip(), ln) for name, p, ln in fields]


def run_status(fields):
    section("WHERE THE PLAN STANDS", f"{rel(PLAN)} (leading status block)")
    if not fields:
        warn(
            f"no `> **Field:**` status block at the top of {rel(PLAN)} -- read it directly. "
            f"The expected fields are: {', '.join(STATUS_FIELDS)}."
        )
        return

    seen = set()
    for name, text, lineno in fields:
        seen.add(name)
        body = strip_links(text)
        if name not in STATUS_FULL:
            body = excerpt(body, STATUS_EXCERPT, f"{rel(PLAN)}:{lineno}")
        emit(wrap(name, body))

    missing = [f for f in STATUS_FIELDS if f not in seen]
    if missing:
        warn(f"status block is missing the field(s): {', '.join(missing)}")


# ------------------------------------------------------- milestone map + leads


def parse_milestones(_plan_text=None):
    """Every milestone, from the index table -- `plan.py` owns how one is found on disk."""
    found = []
    for m in planmod.milestones():
        found.append(
            {
                "id": m["id"],
                "title": strip_links(m["title"]).replace("**", "").strip(),
                "line": m["line"],
                "entry": m,
            }
        )
    return found


def pick_current_next(status_text, milestones):
    """The milestone the live chain goal lands in, the one the next goal does, and that goal.

    The chain is the schedule and a milestone is an identity tag one or more goals carry
    (AGENTS.md's *The schedule is the chain* bullet), so **which milestone is current is only
    the chain can answer**. It used to be answered by regexing `**Mn**` out of the Status prose,
    which is how a milestone two thirds landed kept being projected into every session's pack as
    the next thing to do -- M8 was `done` at goals `core-part-ii` and `database` and still open at 17, and one bolded id
    in a paragraph could not say that. That regex survives only as the fallback for a tree with no
    chain on disk."""
    goals = planmod.chain_goals()
    live = planmod.live_goal() if goals else None
    if live is not None:
        ids = {m["id"] for m in milestones}
        current = live["milestone"] if live["milestone"] in ids else None
        following = None
        for g in goals[live["pos"]:]:
            if g["milestone"] in ids and g["milestone"] != current:
                following = g["milestone"]
                break
        return current, following, live
    if goals:
        warn(
            "no goal in docs/agent/goals/ matches the H1 of docs/agent/loop-goal.md, so "
            "the schedule cannot say which milestone is current and the Status field is being "
            "read instead. A live goal is a copy of its chain entry: the two H1s should be equal."
        )
    cur = re.search(r"[Cc]urrent:\s*\*\*(M\d+[A-Z]?)\*\*", status_text or "")
    nxt = re.search(r"[Nn]ext:\s*\*\*(M\d+[A-Z]?)\*\*", status_text or "")
    if cur:
        current = cur.group(1)
    else:
        mentioned = re.findall(r"\*\*(M\d+[A-Z]?)\*\*", status_text or "")
        if not mentioned:
            # Silence here means the map prints with no `<- current` marker and no lead at all,
            # which reads as "the plan has no current milestone" rather than as a defect. It went
            # unnoticed for as long as the Status field happened not to bold a bare `**Mn**`.
            warn(
                "the Status field says neither `Current: **Mn**` nor any bare `**Mn**`, so no "
                "milestone is marked current and no lead is printed below. Spell it out: the "
                "field is the one home for which milestone the work is inside."
            )
            return None, None, None
        current = mentioned[-1]
        warn(
            f'the Status field does not say `Current: **Mn**`; guessed {current} from the last '
            "milestone it names. Spell it out so this is not a guess."
        )
    if nxt:
        return current, nxt.group(1), None
    ids = [m["id"] for m in milestones]
    if current in ids:
        i = ids.index(current)
        return current, (ids[i + 1] if i + 1 < len(ids) else None), None
    return current, None, None


def run_milestones(status_text, plan_text):
    milestones = parse_milestones(plan_text)
    if not milestones:
        section("MILESTONE MAP", f"{rel(PLAN)} (## Milestones)")
        warn(f"found no `### Mn --` milestone headings in {rel(PLAN)}")
        return

    current, nxt, live = pick_current_next(status_text, milestones)
    section(
        "MILESTONE MAP, AND THE LEAD OF THE CURRENT ONE",
        f"{rel(PLAN)} (the milestone table) -- one file each, under docs/plan/",
    )
    emit("Every milestone, one line each, with the file that holds it and the chain goals that")
    emit("carry its work. The goal is the unit of schedule and the milestone the unit of identity:")
    emit('say "goal `parses`", not "in M7". Only the current and next milestones\' opening paragraphs are')
    emit("printed; a milestone's full text is deliberately not in this digest -- `python")
    emit("tools/plan.py --show M8` prints one, `--show M8:verify` its acceptance paragraph alone.")
    if live is not None:
        total = len(planmod.chain_goals())
        where = live["milestone"] or "no milestone"
        emit()
        emit(f"  LIVE: goal `{live['slug']}`, {live['num']} of {total}, inside {where}")
        emit("        docs/agent/goals/ is the schedule: every earlier goal has passed,")
        emit("        every later one is not started.")
    side = [g.slug for g in goalsmod.load_side() if not g.retired]
    if side:
        emit(f"  SIDE: {', '.join(f'`{s}`' for s in side)} -- off the chain; each runs only under")
        emit("        `loop.py --side <slug>`, and `python tools/side.py --status` says where it stands.")
    emit()

    # The cell is markdown, so `done\*` carries M4's footnote marker escaped. The goals in it are
    # named, not numbered, so the cell is a sentence rather than a column: M8's is eleven slugs and
    # padding every row to it made the map 250 characters wide. So the milestone leads and the
    # goals that carry it follow on their own indented line, wrapped, and only when there are any.
    cells = {m["id"]: m["entry"]["carried"].replace("\\*", "*") for m in milestones}
    for m in milestones:
        marker = "  <- current" if m["id"] == current else ("  <- next" if m["id"] == nxt else "")
        emit(f"  {m['entry']['rel']:<20} {m['id']:<4} {m['title']}{marker}")
        cell = cells[m["id"]]
        if cell:
            # A slug is hyphenated, and a break inside one reads as two goals.
            for line in textwrap.wrap(cell, width=76, break_on_hyphens=False):
                emit(f"  {'':<20} {line}")

    by_id = {m["id"]: m for m in milestones}

    def lead_of(mid, limit):
        m = by_id.get(mid)
        if not m or not m["entry"]["path"].exists():
            return
        para = strip_links(planmod.lead_paragraph(m["entry"]))
        if not para:
            warn(f"milestone {mid} has no opening paragraph to slice")
            return
        emit()
        emit(f"-- {mid} lead ({m['entry']['rel']})")
        emit(textwrap.fill(excerpt(para, limit, m["entry"]["rel"]), width=100))

    lead_of(current, LEAD_EXCERPT)

    m = by_id.get(current)
    if m and m["entry"]["path"].exists():
        ver = planmod.verify_paragraph(m["entry"])
        if ver:
            # the "-- Mn acceptance" label already says what this is
            ver = re.sub(r"^\*\*Verif(y|ied):\*\*\s*", "", strip_links(ver))
            emit()
            emit(f"-- {current} acceptance ({m['entry']['rel']})")
            emit(textwrap.fill(excerpt(ver, VERIFY_EXCERPT, m["entry"]["rel"]), width=100))

    if nxt:
        lead_of(nxt, NEXT_LEAD_EXCERPT)


# ------------------------------------------------------------------ decisions


def decision_records():
    """(number, title, status) for every frozen record in docs/decisions/, off its YAML block
    and H1. The index table that used to carry this in docs/adr/README.md was retired by the
    docs migration's unit C2; the files are the only home now."""
    out = []
    for path in sorted(ADR_DIR.glob("[0-9][0-9][0-9][0-9].md")):
        text = read(path) or ""
        status, title = "?", ""
        for line in text.split("\n")[:60]:
            if line.startswith("status:"):
                status = line[len("status:"):].strip()
            elif line.startswith("# "):
                title = line[2:].split("—", 1)[-1].strip()
                break
        out.append((path.name[:4], title, status))
    return out


def run_adr_index(full):
    section(
        "DECISIONS WITH A RECORD (number, decision, status)",
        f"{rel(ADR_DIR)}/NNNN.md (the YAML block and the title)",
    )
    records = decision_records()
    if not records:
        warn(f"no records under {rel(ADR_DIR)}/")
        return
    if full:
        for num, title, status in records:
            suffix = "" if status == "accepted" else f"   [{status}]"
            emit(f"{num}  {title}{suffix}")
        emit()
        emit("Every record not marked otherwise is accepted -- the status is printed only for")
        emit("the exceptions.")
    else:
        # The exceptions are the rows no other file in the tree states, so they print whatever
        # the flag says; the accepted majority is one line each in docs/ground-rules.md.
        exceptions = [(n, t, s) for n, t, s in records if s != "accepted"]
        emit(f"{len(records)} records, all accepted except the {len(exceptions)} listed here.")
        for num, title, status in exceptions:
            emit(f"  {num}  {title}   [{status}]")
        emit()
        emit("One line per rule, with its link, is in docs/ground-rules.md -- printing every")
        emit("record here too was a second copy of it. For one line per record:")
        emit("python tools/brief.py --adrs")
    emit()
    emit("A record never holds the current rule -- docs/rules/ does. Open the chapter a rule id")
    emit("names, or run `python tools/brief.py --where <keyword>` to route a topic to its owner.")


def run_no_adr_decisions():
    section(
        "DECISIONS WITH NO ADR (titles only)",
        f"{rel(ADR_README)} section 'Decisions taken at project start'",
    )
    text = read(ADR_README)
    if text is None:
        warn(f"could not read {rel(ADR_README)} at all")
        return
    inside = False
    bullets = []
    for lineno, line in enumerate(text.split("\n"), start=1):
        if line.startswith("## Decisions taken at project start"):
            inside = True
            continue
        if inside and line.startswith("## "):
            break
        if inside and line.startswith("**"):
            m = re.match(r"\*\*(.*?)\*\*", line)
            if m:
                bullets.append((m.group(1), lineno))
    if not bullets:
        warn(f"could not slice the project-start decisions out of {rel(ADR_README)}")
        return
    for title, _lineno in bullets:
        emit(f"- {title}")
    emit()
    emit("(each is one paragraph in that section: open it for the reasoning)")


# ------------------------------------------------------------ where to look


#: The homes that are not rules. `--where` routes a keyword to the rulebook -- a chapter, or a
#: rule whose id or title matches -- and everything a language rule can be is there. These are the
#: topics the retired routing table also carried that no rule owns, because they are about the
#: repository rather than the language: the schedule, the tooling, the prose, the measurements.
#: Each row is (the words that hit it, the home, one line saying what is there). Keep it this
#: short: a topic that grows a rule moves to the rulebook and comes off this list.
HOMES = (
    ("plan milestone schedule goal", "docs/plan/ + docs/implementation-plan.md",
     "the status block and the milestone table; one file per milestone; the chain is the schedule"),
    ("tools commands scripts", "docs/agent/commands.md",
     "how the repo is driven: peek.py, verify.py, session.py, splice.py, plan.py, disk.py"),
    ("benches benchmark", "benches/ + docs/perf/",
     "the benchmark programs, and the figures with the methodology that took them"),
    ("perf performance latency throughput", "docs/perf/",
     "the measured figures, the methodology and the regressions"),
    ("comment comments changelog history prose dates docstring",
     "docs/agent/conventions.md, 'A code comment'",
     "a comment says what the code does now: no date, no volatile count, rewritten whole"),
    ("unowned owner owners ownership gap gaps register",
     "python tools/owners.py --registers",
     "who owns each `# Known gaps` item, derived: a milestone still ahead, and nothing else"),
    ("order ordering position band chapters",
     "docs/agent/conventions.md, 'Where a rule sits in the order'",
     "the rulebook reads ground-up, not by date: the five bands, and where a new rule is inserted"),
)

WHERE_CAP = 40  # a display cap on one `--where` answer, not on anything an author writes

#: The one home in `HOMES` whose line is measured rather than written. It is matched by its path,
#: so the row above stays the same shape as its neighbours.
OWNERS_HOME = "python tools/owners.py --registers"


def ownership_line():
    """How big the ownership register is, counted now rather than written down here.

    Routing to the tool is half an answer: what a reader wants next is how much is open and whether
    any of it names an owner the gate refuses, and both numbers move every time a gap is written or
    closed. So they come from `tools/owners.py`'s own classification of every `# Known gaps` item --
    imported rather than shelled out to, and never a figure typed into this file, which is the copy
    that would go stale first. The scan is a third of a second, and only a keyword that routes here
    pays it.
    """
    try:
        import owners  # noqa: PLC0415 -- same directory; loaded only for this one answer
        kinds = owners.classify(owners.collect())
    except Exception as exc:  # noqa: BLE001
        return f"(tools/owners.py could not count them: {exc})"
    refused = sum(len(kinds[kind]) for kind in owners.REFUSED)
    tagged = sum(len(v) for k, v in kinds.items() if k != "untagged")
    return (f"{len(kinds['milestone'])} of {tagged} tagged item(s) name a milestone still ahead "
            f"today, and {refused} name an owner the gate refuses; `python tools/owners.py --check` "
            f"lists those with their anchors")


def run_where(terms):
    """`--where` over the rulebook: the chapter list, or every rule whose id or title matches,
    plus whichever of `HOMES` the same words hit.

    The routing table in docs/adr/README.md was retired by the docs migration's unit C2 -- one
    row per topic, hand-maintained, was the second copy of what `docs/rules/_index.json` and the
    chapters' own titles already state. A keyword routes to a rule, and the rule's chapter is the
    file that owns the topic; the rows that never were rules are `HOMES` above.
    """
    import rules as rulebook  # noqa: PLC0415 -- same directory; loaded only for --where

    try:
        book = rulebook.Rulebook()
    except Exception as exc:  # noqa: BLE001
        sys.stdout.write(f"brief.py: the rulebook did not load: {exc}\n")
        return 1
    if not terms:
        sys.stdout.write(
            f"Routing: {len(book.topics)} chapters under docs/rules/, {len(book.by_id)} rules, "
            f"and {len(HOMES)} homes that are not rules.\n"
            "Re-run with a keyword for what matches: python tools/brief.py --where <keyword>\n\n"
        )
        for t in book.topics:
            sys.stdout.write(f"  docs/rules/{t.topic}.md  {t.title} ({len(t.rules)} rules)\n")
        sys.stdout.write("\n")
        for _words, home, what in HOMES:
            sys.stdout.write(f"  {home}  {what}\n")
        return 0
    needles = [t.lower() for t in terms]
    # A home is hit through its words and its path, never its description: "plan" should not
    # route to commands.md because that line happens to name plan.py.
    homes = [(home, what) for words, home, what in HOMES
             if all(n in (words + " " + home).lower() for n in needles)]
    hits = [r for r in book.by_id.values()
            if all(n in (r.id + " " + r.title).lower() for n in needles)]
    if not hits and not homes:
        sys.stdout.write(
            f"brief.py --where: no rule id, rule title or home matches {' '.join(terms)!r}. Run "
            "`--where` with no keyword for the chapter list, or grep docs/rules/ for the words.\n"
        )
        return 0
    for home, what in homes:
        sys.stdout.write(f"  {home}\n     {what}\n")
        if home == OWNERS_HOME:
            sys.stdout.write(f"     {ownership_line()}\n")
    for r in hits[:WHERE_CAP]:
        mark = "" if r.status == "shipped" else "  (designed)"
        sys.stdout.write(f"  docs/rules/{r.topic}.md#{r.anchor}  rule:{r.id}{mark}\n     {r.title}\n")
    if len(hits) > WHERE_CAP:
        sys.stdout.write(f"  ... and {len(hits) - WHERE_CAP} more; narrow the keyword\n")
    if hits:
        # The path above is the generated chapter, which is the reader's link and the wrong thing
        # to fetch: a chapter runs to 87 KB. The token beside it is a `peek.py` target, so the
        # next call is the answer rather than another routing step -- and several tokens go in
        # one call, which is the shape this answer usually wants.
        sys.stdout.write(
            "\n  Read one -- or several -- in one call:  python tools/peek.py "
            + " ".join(f"rule:{r.id}" for r in hits[:2])
            + "\n  (the chapter path is the link; the `rule:` token is the target)\n"
        )
    return 0


# --------------------------------------------------------------- guard tests


def parse_guard_tests(rs_text):
    """Test name, its [MAX/MIN const bounds], its feature gate (from a
    `#[cfg(feature = "...")]` on the mod or on the test itself), with line numbers."""
    order = []
    bound = {}
    gate = {}
    at_line = {}
    pending_feat = None
    mod_feat = None
    pending = False
    cur = None

    cfg_re = re.compile(r'^\s*#\[cfg\(feature\s*=\s*("[^"]+")\)\]')
    mod_re = re.compile(r"^\s*mod\s")
    test_attr_re = re.compile(r"^\s*#\[test\]")
    fn_re = re.compile(r"^\s*fn\s+([a-z_0-9]+)\s*\(\)")
    const_re = re.compile(r"^\s*const\s+((?:MAX|MIN)[A-Z_]*:.*?);.*$")

    for lineno, line in enumerate(rs_text.split("\n"), start=1):
        cfg_m = cfg_re.match(line)
        if cfg_m:
            pending_feat = cfg_m.group(1)
            continue
        if pending_feat is not None and mod_re.match(line):
            mod_feat = pending_feat
            pending_feat = None
            continue
        if test_attr_re.match(line):
            pending = True
            continue
        if pending:
            fn_m = fn_re.match(line)
            if fn_m:
                name = fn_m.group(1)
                order.append(name)
                at_line[name] = lineno
                pending = False
                feat = pending_feat or mod_feat
                pending_feat = None
                if feat:
                    gate[name] = feat
                cur = name
                continue
        if cur:
            const_m = const_re.match(line)
            if const_m:
                entry = const_m.group(1).strip()
                bound[cur] = bound.get(cur, "") + (", " if cur in bound else "") + entry
        if line.startswith("}"):
            cur = None
            mod_feat = None

    result = []
    for name in order:
        parts = name
        if name in bound:
            parts += f"  [{bound[name]}]"
        if name in gate:
            parts += f"  (feature {gate[name]})"
        result.append((parts, at_line[name]))
    return result


def run_guard_tests():
    section(
        "WHAT IS ACTUALLY GUARDED",
        f"{rel(PROBE)}/tests/*.rs -- authoritative for every measured number",
    )
    emit(
        "A test name is the claim; a bracketed threshold is the bound it holds. If one of these fails,"
    )
    emit("the ADR naming it needs revisiting -- not the threshold.")

    tests_dir = PROBE / "tests"
    rs_files = sorted(tests_dir.glob("*.rs")) if tests_dir.is_dir() else []
    count = 0
    for rs_file in rs_files:
        text = read(rs_file)
        if text is None:
            continue
        entries = parse_guard_tests(text)
        if not entries:
            continue
        emit()
        emit(rel(rs_file))
        for line, _lineno in entries:
            emit(f"  {line}")
            count += 1
    if count == 0:
        warn(
            f"found no guard tests under {rel(PROBE)}/tests -- that directory is the source of "
            "truth, check it"
        )
        return

    benches_dir = PROBE / "benches"
    if benches_dir.is_dir():
        bench_files = sorted(benches_dir.glob("*.rs"))
        if bench_files:
            emit()
            emit("benchmarks (unguarded, for tracking figures by hand):")
            for f in bench_files:
                emit(f"  {rel(f)}")


# --------------------------------------------------------------- the map
#
# A session's wall clock is very nearly its turn count times a constant, and two fifths of
# every loop session's tool calls were read-only probes asking where something lives -- the
# same files, the same symbols, every iteration, because nothing in its context survived the
# last one. This section is that answer, paid once and sliced live.


DOC_LINE_RE = re.compile(r"^\s*//!\s?(.*)$")
SKIP_BEFORE_DOC_RE = re.compile(r"^\s*(#!\[|//[^!]|//$|$)")
INTRA_DOC_RE = re.compile(r"\[(`[^`\]]+`)\]")


def module_doc(text):
    """The first paragraph of a module's own `//!` block, or "" if it has none."""
    para = []
    for line in text.split("\n"):
        m = DOC_LINE_RE.match(line)
        if m:
            body = m.group(1).strip()
            if not body:
                if para:
                    break  # a blank `//!` ends the opening paragraph
                continue
            para.append(body)
        elif para:
            break
        elif not SKIP_BEFORE_DOC_RE.match(line):
            break  # real code before any `//!` -- this module has no doc comment
    return " ".join(para)


def first_sentence(text):
    """`Foo the bar. Then baz.` -> `Foo the bar`. A `.` inside backticks or followed by a
    non-space never ends a sentence, so `nvs_ir::ir` and `0.1.0` stay whole."""
    text = INTRA_DOC_RE.sub(r"\1", strip_links(text))
    depth_safe = re.split(r"(?<=[a-z\)`])\.\s+(?=[A-Z\[`])", text, maxsplit=1)
    return depth_safe[0].strip().rstrip(".")


def py_doc(text):
    """The opening paragraph of a Python file's module docstring, or "" if it has none.

    Parsed rather than pattern-matched. A tool's docstring opens with a usage block full of quotes
    and backslashes, and every regex that tried to find where it ended got one of them wrong; `ast`
    already knows, and the file is read once.

    The paragraph, not the whole docstring, because the line after it is a `python tools/...`
    usage block and `first_sentence` cannot end a sentence inside one: no `.` there is followed by
    a space and a capital, so `chain.py` handed the map its entire twelve-line synopsis. PEP 257
    puts the summary in the first paragraph, and every tool here follows it."""
    try:
        doc = ast.get_docstring(ast.parse(text)) or ""
    except (SyntaxError, ValueError):
        return ""
    return doc.split("\n\n", 1)[0].strip()


def md_doc(text):
    """A markdown file's first `#` heading and the opening sentence beneath it, as one line.

    The heading alone is usually a noun -- `# Carried gaps` -- and says what the file is called
    rather than what it holds, so the sentence under it does the work. Either half may be missing,
    and what is there is what comes back."""
    head, para = "", []
    for raw in text.split("\n"):
        line = raw.strip()
        if not head:
            if line.startswith("# "):
                head = line[2:].strip()
            continue
        if line.startswith("#"):
            break  # the next heading, with nothing but headings between: no prose to quote
        if line:
            para.append(line)
        elif para:
            break
    body = first_sentence(" ".join(para)) if para else ""
    return f"{head} -- {body}" if head and body else (head or body)


#: How a file that is not a crate module says what it is, keyed by suffix. A shape absent here has
#: no header this tool can read -- a `.nvst` case, a workflow, a fixture -- and the map prints its
#: path alone rather than guessing, which is the honest answer and still resolves the selector.
#: How long a derived summary may be before the map cuts it. A `[context] modules` line is a
#: reminder of what a file is, not the file's own introduction.
SUMMARY_MAX = 200

PATH_SUMMARY = {
    ".py": lambda text: first_sentence(py_doc(text)),
    ".md": md_doc,
    ".rs": lambda text: first_sentence(module_doc(text)),
    ".ts": lambda text: first_sentence(header_doc(text)),
    ".tsx": lambda text: first_sentence(header_doc(text)),
}


def path_summary(path):
    """One line describing any file in the tree, or "" for a shape that carries no header.

    This exists because a goal's `[context] modules` names whatever files its slices touch, and
    `crates/` plus `editors/` is not that set: goals name `tools/*.py`, `docs/agent/*.md`, a
    conformance case, a workflow. Those used to resolve to nothing, print nothing, and warn -- and
    an optimization pass then deleted the selector for being unresolvable, which is backwards. The
    file the goal named is the fact; what this repository can say about it is the variable."""
    text = read(path)
    if text is None:
        return ""
    fn = PATH_SUMMARY.get(Path(path).suffix)
    if not fn:
        return ""
    # One line, and a bounded one. A crate module's `//!` is written to be read here and is trusted
    # at whatever length it is; these files were not, and a header that runs long is a header that
    # was written for its own file rather than for a digest. The cut is on the summary this tool
    # derives, never on the file.
    line = " ".join(fn(text).split())
    return line if len(line) <= SUMMARY_MAX else line[:SUMMARY_MAX].rstrip(" ,;:-") + "..."


def crate_modules():
    """Every `crates/*/src/**/*.rs`, grouped by crate, as (crate, relative path, summary)."""
    if not CRATES.is_dir():
        return {}
    groups = {}
    for crate_dir in sorted(p for p in CRATES.iterdir() if (p / "src").is_dir()):
        entries = []
        for rs in sorted((crate_dir / "src").rglob("*.rs")):
            if "snapshots" in rs.parts:
                continue
            text = read(rs)
            if text is None:
                continue
            summary = first_sentence(module_doc(text))
            within = rs.relative_to(crate_dir).as_posix()
            # lib.rs is the crate's own doc and sorts first; the rest alphabetically.
            entries.append((within != "src/lib.rs", within, summary))
        if entries:
            groups[crate_dir.name] = [(w, s) for _, w, s in sorted(entries)]
    return groups


def editor_modules():
    """Every `editors/*/src/**/*.{ts,tsx}`, grouped by package, as (relative path, summary).

    The same shape `crate_modules` returns, so every consumer -- this file's map and `orient.py`'s
    scoped one -- treats an editor package exactly like a crate. It exists because from M4B a
    session's file set is not always Rust: `editors/vscode` is TypeScript, and a session working
    there would otherwise orient on nothing at all.

    The key is the package's ROOT-relative path (`editors/vscode`) rather than a bare name, so the
    map's group heading is already the thing a `[context] modules` glob is written against.
    """
    base = ROOT / "editors"
    if not base.is_dir():
        return {}
    groups = {}
    for pkg in sorted(p for p in base.iterdir() if (p / "src").is_dir()):
        entries = []
        for src in sorted((pkg / "src").rglob("*.ts")) + sorted((pkg / "src").rglob("*.tsx")):
            if "node_modules" in src.parts:
                continue
            text = read(src)
            if text is None:
                continue
            within = src.relative_to(pkg).as_posix()
            # `extension.ts` is the activation entry point and sorts first, the way `lib.rs` does.
            entries.append((within != "src/extension.ts", within, first_sentence(header_doc(text))))
        if entries:
            groups[pkg.relative_to(ROOT).as_posix()] = [(w, s) for _, w, s in sorted(entries)]
    return groups


def header_doc(text):
    """The first paragraph of a TypeScript file's own leading comment, or "" if it has none.

    `//!` has no TypeScript equivalent, so the convention is the same one every other TS project
    uses: a `/** ... */` or `//` block at the top of the file, before any import. Anything else at
    the top means this file has no header doc, which the map says out loud rather than guessing.
    """
    para = []
    for raw in text.split("\n"):
        line = raw.strip()
        if line.startswith("/**") or line.startswith("/*"):
            line = line.lstrip("/*").strip()
        elif line.startswith("*/"):
            break
        elif line.startswith("*"):
            line = line[1:].strip()
        elif line.startswith("//"):
            line = line[2:].strip()
        elif para or line:
            break  # real code, or a blank line after the paragraph
        if not line:
            if para:
                break
            continue
        para.append(line)
    return " ".join(para)


def run_map():
    section(
        "THE MAP -- ONE LINE PER MODULE",
        "each module's own `//!` first sentence, sliced live",
    )
    groups = {**crate_modules(), **editor_modules()}
    if not groups:
        warn(f"no `crates/*/src/**/*.rs` under {rel(CRATES)} -- that directory is the source")
        return
    emit("A module's full doc comment is authoritative for how it works, and for its own known")
    emit("gaps; this is only the sentence that says which one to open.")
    undocumented = []
    for crate, entries in groups.items():
        emit()
        emit(crate)
        width = max(len(w) for w, _ in entries)
        for within, summary in entries:
            if not summary:
                undocumented.append(f"{crate}/{within}")
                summary = "(no `//!` doc comment)"
            # A bare ellipsis, not excerpt()'s "[continues at ...]" -- the file this sentence
            # continues in is the line's own label, so naming it again is pure width.
            if nbytes(summary) > MODULE_EXCERPT:
                kept = summary.encode("utf-8")[:MODULE_EXCERPT].decode("utf-8", "ignore")
                summary = kept.rsplit(" ", 1)[0] + " ..."
            emit(f"  {within:<{width}}  {summary}")

    emit()
    emit("anchors -- the definitions a session most often greps for:")
    text_by_file = {}
    for crate_dir in sorted(p for p in CRATES.iterdir() if (p / "src").is_dir()):
        for rs in sorted((crate_dir / "src").rglob("*.rs")):
            if "snapshots" not in rs.parts:
                text_by_file[rs] = read(rs) or ""

    missing = []
    for label, pattern in ANCHORS:
        rx = re.compile(pattern, re.MULTILINE)
        hits = []
        for path, body in text_by_file.items():
            for m in rx.finditer(body):
                hits.append(f"{rel(path)}:{body[: m.start()].count(chr(10)) + 1}")
        if not hits:
            missing.append(label)
            continue
        shown = ", ".join(hits[:3]) + (f", +{len(hits) - 3} more" if len(hits) > 3 else "")
        emit(f"  {label}")
        emit(f"      {shown}")

    if missing:
        warn(
            "these anchor patterns matched nothing -- the symbol was renamed or moved, so "
            "brief.py's ANCHORS list needs the new spelling: " + ", ".join(missing)
        )
    if undocumented:
        warn(
            f"{len(undocumented)} module(s) have no `//!` doc comment, so the map cannot say "
            "what they are for: " + ", ".join(undocumented[:8])
            + (f", +{len(undocumented) - 8} more" if len(undocumented) > 8 else "")
        )


# ------------------------------------------------------- the next free number
#
# Two numbers a session has to look up before it can write anything, and both were being
# derived with a grep every time. Worse, the ADR one is a race: two agents that both grep for
# the highest number pick the same next one.


CODE_DECL_RE = re.compile(r'Code::new\("(E(\d{2})\d{2})"\)')
# A retired code is never reused, and its `Code::new` line is gone -- so the
# comment that replaced it is the only thing left holding the band's ceiling up.
# Read both spellings the registry uses (the code before the word and after it),
# within one comment's worth of text and never across a second code.
RETIRED_BEFORE_RE = re.compile(r"`(E(\d{2})\d{2})`[^`]{0,80}?\bretired\b")
RETIRED_AFTER_RE = re.compile(r"\bretired\b[^`]{0,80}?`(E(\d{2})\d{2})`")
CODE_LEGEND_RE = re.compile(r"^///\s*\|\s*`E(\d{2})xx`\s*\|\s*([^|]+?)\s*\|")
ADR_FILE_RE = re.compile(r"^(\d{4})\.md$")


def run_numbers():
    section(
        "THE NEXT FREE NUMBER",
        f"{rel(DIAGNOSTICS)} (every `Code::new`) and {rel(ADR_DIR)}/ (the filenames)",
    )

    text = read(DIAGNOSTICS)
    if text is None:
        warn(f"could not read {rel(DIAGNOSTICS)} -- it is the diagnostic-code registry")
    else:
        legend = {}
        highest = {}
        for line in text.split("\n"):
            m = CODE_LEGEND_RE.match(line)
            if m:
                legend[m.group(1)] = m.group(2).strip()
        for regex in (CODE_DECL_RE, RETIRED_BEFORE_RE, RETIRED_AFTER_RE):
            for m in regex.finditer(text):
                band = m.group(2)
                highest[band] = max(highest.get(band, 0), int(m.group(1)[1:]))
        if not highest:
            warn(f"no `Code::new(\"Ennnn\")` declarations in {rel(DIAGNOSTICS)}")
        else:
            emit("diagnostic codes -- next free in each band (max + 1; a retired code is never")
            emit("reused, so this is deliberately not the lowest hole):")
            for band in sorted(highest):
                meaning = legend.get(band, "(no row for this band in that file's legend table)")
                # Max-plus-one leaves the band when it lands on `Enn00`, and a number in
                # the next band's digits is not free -- it reads as that band's stage.
                # So a filled band says so and the legend table's continuation row is
                # where the next code comes from: E0499 did not make E0500 the next types
                # code, it made the types band need E07xx.
                if (highest[band] + 1) % 100 == 0:
                    emit(f"  E{band}xx  {meaning:<48} FULL at E{highest[band]:04d}")
                else:
                    emit(f"  E{band}xx  {meaning:<48} next: E{highest[band] + 1:04d}")
            unlisted = sorted(set(legend) - set(highest))
            if unlisted:
                emit(f"  bands with a legend row but no code yet: "
                     + ", ".join(f"E{b}xx" for b in unlisted))

    if not ADR_DIR.is_dir():
        warn(f"no {rel(ADR_DIR)}/ directory")
        return
    numbers = [
        int(m.group(1))
        for p in ADR_DIR.iterdir()
        for m in [ADR_FILE_RE.match(p.name)]
        if m
    ]
    emit()
    if not numbers:
        warn(f"no `NNNN.md` files in {rel(ADR_DIR)}/")
        return
    emit(f"ADRs: {len(numbers)} on disk, highest {max(numbers):04d} -- "
         f"next free is {max(numbers) + 1:04d}")
    emit("Claim it by creating the file, and re-check this immediately before you do: another")
    emit("agent working the same tree derives the same answer from the same directory.")


# -------------------------------------------------------------- what exists


def run_disk():
    section("WHAT EXISTS ON DISK", "the filesystem, not the plan")

    def listing(dirname):
        d = ROOT / dirname
        if not d.is_dir():
            return ""
        return " ".join(sorted(p.name for p in d.iterdir()))

    emit(f"crates:  {listing('crates')}")
    emit(f"benches: {listing('benches')}")
    emit(f"docs:    {listing('docs')}")
    emit(f"tools:   {listing('tools')}")
    spec_dir = ROOT / "docs" / "spec"
    if spec_dir.is_dir() and any(spec_dir.iterdir()):
        emit(f"spec:    {listing('docs/spec')}")
    else:
        emit("spec:    unwritten -- say so rather than inferring language semantics")


# ------------------------------------------------------------- working tree


def run_git():
    if not (ROOT / ".git").is_dir():
        return
    section("WORKING TREE", "git")

    def git(*args):
        # rstrip only -- git status --porcelain's leading " M"/"??" column is meaningful,
        # and a plain .strip() would eat it off the first line.
        try:
            return subprocess.run(
                ["git", *args],
                cwd=ROOT,
                capture_output=True,
                encoding="utf-8",
                errors="replace",
                check=True,
            ).stdout.rstrip("\n")
        except (subprocess.CalledProcessError, OSError):
            return None

    emit(f"branch:  {git('rev-parse', '--abbrev-ref', 'HEAD') or '(unknown)'}")
    emit(f"HEAD:    {git('log', '-1', '--format=%h %s') or '(unknown)'}")

    # `.loop/running` exists for exactly as long as tools/loop.py is driving this tree. Two agents on
    # one working tree race on every file, and the loop's sessions edit the same few files on nearly
    # every iteration -- so this is the first thing a session needs to know, before it edits anything.
    running = ROOT / ".loop" / "running"
    paused = ROOT / ".loop" / "pause"
    if running.exists():
        emit()
        emit("!! A WORK LOOP IS RUNNING ON THIS TREE (.loop/running):")
        for line in (read(running) or "").rstrip("\n").split("\n"):
            emit(f"     {line}")
        # The way to work on this tree anyway is named right here rather than only in the doc,
        # because this warning is the part of it that gets read at the moment it matters. And
        # `held:`, not the file's existence, is the promise: coordinator.md § Holding the tree.
        if paused.exists():
            text = read(paused) or ""
            queued = "held:" not in text or "(not yet)" in text
            emit(f"!! A HOLD IS {'QUEUED' if queued else 'IN PLACE'} (.loop/pause):")
            for line in text.rstrip("\n").split("\n"):
                if line.strip():
                    emit(f"     {line}")
            if queued:
                emit("!! Queued is not held: the session in flight is still committing to this tree.")
                emit("!! Wait for the `held:` line above to name a time before you touch anything.")
            else:
                emit("!! Held: no session is running and none starts until this file goes. Work, then")
                emit("!! delete .loop/pause and the loop carries on where it left off.")
            emit("!! A hold marked `by: user` was taken at the console, and only `p` there lifts it.")
        else:
            emit("!! Its sessions commit to this same tree. Do not edit files alongside it: tell the user,")
            emit("!! and stop unless they say otherwise. `tail -f .loop/log.md` shows what it is doing.")
            emit("!! To take the tree without ending the run: create .loop/pause, wait for a `held:`")
            emit("!! line to appear in it, work, then delete it. The loop carries on where it was.")
    elif paused.exists():
        emit()
        emit("!! .loop/pause is present and no loop is running: the next one will hold before its")
        emit("!! first session. Delete it unless that is what you meant.")

    changed = git("status", "--porcelain")
    if changed:
        changed_lines = changed.split("\n")
        emit("changed:")
        for line in changed_lines[:GIT_CHANGED_LINE_CAP]:
            emit(f"  {line}")
        if len(changed_lines) > GIT_CHANGED_LINE_CAP:
            emit(
                f"  ... and {len(changed_lines) - GIT_CHANGED_LINE_CAP} more -- "
                "run `git status` directly"
            )
    else:
        emit("changed: nothing")


# ------------------------------------------------------------------ drivers


def build(opts):
    plan_text = read(PLAN)
    if plan_text is None:
        warn(f"could not read {rel(PLAN)} at all")
        fields = []
    else:
        fields = parse_status_fields(plan_text)

    run_status(fields)
    if plan_text is not None:
        status_text = next((t for n, t, _ in fields if n == "Status"), "")
        run_milestones(status_text, plan_text)
    run_adr_index(full="--adrs" in opts)
    run_no_adr_decisions()
    run_guard_tests()
    if "--no-map" not in opts:
        run_map()
    run_numbers()
    run_disk()
    if "--no-git" not in opts:
        run_git()


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass
    argv = sys.argv[1:]

    if "--where" in argv:
        i = argv.index("--where")
        return run_where([a for a in argv[i + 1 :] if not a.startswith("--")])

    known = {"--no-git", "--no-map", "--adrs"}
    unknown = [a for a in argv if a.startswith("--") and a not in known]
    if unknown:
        sys.stdout.write(
            f"brief.py: unknown option(s) {' '.join(unknown)}. "
            f"Known: {' '.join(sorted(known))} --where\n"
        )
        return 2

    build(argv)
    emit()
    emit("Route a topic to the one file that owns it: python tools/brief.py --where <keyword>")
    sys.stdout.write("\n".join(out).lstrip("\n") + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
