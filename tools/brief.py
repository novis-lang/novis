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
        python tools/brief.py --where         # topic index of the routing table
        python tools/brief.py --where regex   # the routing rows matching a keyword
"""

import re
import subprocess
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan as planmod  # noqa: E402  -- the plan's one API; never reimplemented here

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
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
TOPIC_EXCERPT = 96  # one routing-table topic cell, in the `--where` index
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
    """``rule:errors/propagation`` -> ``rule:errors/propagation``. A link target is ~50 bytes
    of no value in a digest whose reader has the routing table one call away."""
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
    the next thing to do -- M8 was `done` at goals 4 and 5 and still open at 17, and one bolded id
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
            "no goal in docs/agent/goals/chain.toml matches the H1 of docs/agent/loop-goal.md, so "
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
    emit('say "goal 19", not "in M7". Only the current and next milestones\' opening paragraphs are')
    emit("printed; a milestone's full text is deliberately not in this digest -- `python")
    emit("tools/plan.py --show M8` prints one, `--show M8:verify` its acceptance paragraph alone.")
    if live is not None:
        total = len(planmod.chain_goals())
        where = live["milestone"] or "no milestone"
        emit()
        emit(f"  LIVE: goal {live['num']} of {total} -- {live['name']}, inside {where}")
        emit("        docs/agent/goals/chain.toml is the schedule: every earlier goal has passed,")
        emit("        every later one is not started.")
    emit()

    # The cell is markdown, so `done\*` carries M4's footnote marker escaped. Widened to whatever
    # the longest one needs rather than a constant, because that is a list now and it grows.
    cells = {m["id"]: m["entry"]["carried"].replace("\\*", "*") for m in milestones}
    width = max((len(c) for c in cells.values()), default=0)
    for m in milestones:
        line = f"{m['id']:<4} {m['title']}"
        marker = "  <- current" if m["id"] == current else ("  <- next" if m["id"] == nxt else "")
        emit(f"  {m['entry']['rel']:<20} {cells[m['id']]:<{width}} {line}{marker}")

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


ADR_LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")


def split_table_row(row):
    """Markdown table cells, respecting `\\|` -- a decision cell naming `||` writes it escaped,
    and splitting on a bare `|` truncates that row mid-sentence."""
    cells = re.split(r"(?<!\\)\|", row)
    return [c.replace("\\|", "|").strip() for c in cells]


def compress_adr_rows(rows, linenos):
    """One line per ADR: filename, then the decision. The status column is dropped for the
    Accepted majority and printed only where it differs. A row that does not match the
    expected cell shape is passed through verbatim rather than silently reshaped or dropped."""
    compressed = []
    for row, lineno in zip(rows, linenos):
        cells = split_table_row(row)
        link = ADR_LINK_RE.search(cells[1]) if len(cells) > 1 else None
        # `| a | b | c |` splits to ['', a, b, c, ''] -- any other count means a stray `|`.
        if link is None or len(cells) != 5:
            compressed.append((row, lineno, None))
            continue
        filename = link.group(1).strip()
        decision = cells[2]
        status = cells[3]
        suffix = "" if status == "Accepted" else "   [" + status + "]"
        compressed.append((filename + "  " + decision + suffix, lineno, decision))
    return compressed


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


# --------------------------------------------------------- the routing table


LINK_RE = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")


def rootward(text):
    """``rule:core-classes/regex-two-tiers`` -> ``rule:core-classes/regex-two-tiers` (docs/adr/0056-...)`. The table
    lives in docs/adr/, so its link targets are relative to that; a reader of this output is at
    the repository root and wants a path they can open."""

    def one(m):
        label, target = m.group(1), m.group(2)
        if target.startswith(("http://", "https://", "#")):
            return label
        try:
            resolved = (ADR_README.parent / target).resolve().relative_to(ROOT).as_posix()
        except (ValueError, OSError):
            return label
        return f"{label} ({resolved})"

    return LINK_RE.sub(one, text)


def parse_routing_table():
    """The `## Where to look` table in docs/adr/README.md, as (topic, home, line number)."""
    text = read(ADR_README)
    if text is None:
        return None
    rows = []
    inside = False
    for lineno, line in enumerate(text.split("\n"), start=1):
        if line.startswith("| Doing this | Open this |"):
            inside = True
            continue
        if inside:
            if not line.startswith("|"):
                break
            if line.startswith("|---") or line.startswith("| ---"):
                continue
            cells = split_table_row(line)
            if len(cells) >= 4:
                rows.append((cells[1], cells[2], lineno))
    return rows


def run_where_rulebook(terms):
    """`--where` over the rulebook: the topic list, or every rule whose id or title matches.

    The routing table in docs/adr/README.md was retired by the docs migration's unit C2 -- one
    row per topic, hand-maintained, was the second copy of what `docs/rules/_index.json` and the
    chapters' own titles already state. A keyword now routes to a rule, and the rule's chapter is
    the file that owns the topic. C6 folds the non-rule rows (the plan, the tools) back in.
    """
    import rules as rulebook  # noqa: PLC0415 -- same directory; loaded only for --where

    try:
        book = rulebook.Rulebook()
    except Exception as exc:  # noqa: BLE001
        sys.stdout.write(f"brief.py: the rulebook did not load: {exc}\n")
        return 1
    if not terms:
        sys.stdout.write(
            f"Routing: {len(book.topics)} chapters under docs/rules/, {len(book.by_id)} rules.\n"
            "Re-run with a keyword for the rules that match: python tools/brief.py --where <keyword>\n\n"
        )
        for t in book.topics:
            sys.stdout.write(f"  docs/rules/{t.topic}.md  {t.title} ({len(t.rules)} rules)\n")
        return 0
    needles = [t.lower() for t in terms]
    hits = [r for r in book.by_id.values()
            if all(n in (r.id + " " + r.title).lower() for n in needles)]
    if not hits:
        sys.stdout.write(
            f"brief.py --where: no rule id or title matches {' '.join(terms)!r}. Run `--where` "
            "with no keyword for the chapter list, or grep docs/rules/ for the words.\n"
        )
        return 0
    for r in hits[:40]:
        mark = "" if r.status == "shipped" else "  (designed)"
        sys.stdout.write(f"  docs/rules/{r.topic}.md#{r.anchor}  rule:{r.id}{mark}\n     {r.title}\n")
    if len(hits) > 40:
        sys.stdout.write(f"  ... and {len(hits) - 40} more; narrow the keyword\n")
    return 0


def run_where(terms):
    """`--where` with no term prints the topic index; with terms, the matching rows in full."""
    rows = parse_routing_table()
    if not rows:
        return run_where_rulebook(terms)

    if not terms:
        sys.stdout.write(
            f"Routing table: {len(rows)} topics, from {rel(ADR_README)} section 'Where to look'.\n"
            "Re-run with a keyword for the full row(s): python tools/brief.py --where <keyword>\n\n"
        )
        for topic, _home, lineno in rows:
            plain = strip_links(topic).replace("**", "").replace("*", "")
            if nbytes(plain) > TOPIC_EXCERPT:
                plain = plain.encode("utf-8")[:TOPIC_EXCERPT].decode("utf-8", "ignore")
                plain = plain.rsplit(" ", 1)[0] + " ..."
            sys.stdout.write(f"  {rel(ADR_README)}:{lineno}  {plain}\n")
        return 0

    needles = [t.lower() for t in terms]
    hits = [r for r in rows if all(n in (r[0] + " " + r[1]).lower() for n in needles)]
    if not hits:
        sys.stdout.write(
            f"brief.py --where: nothing matches {' '.join(terms)!r}. "
            "Run `--where` with no keyword for the topic index, or open "
            f"{rel(ADR_README)} section 'Where to look'.\n"
        )
        return 0
    for topic, home, lineno in hits:
        sys.stdout.write(f"\n-- {rel(ADR_README)}:{lineno}\n")
        sys.stdout.write(
            textwrap.fill(
                strip_links(topic), width=96, initial_indent="   ", subsequent_indent="   "
            )
            + "\n"
        )
        sys.stdout.write(
            textwrap.fill(
                rootward(home), width=96, initial_indent="   -> ", subsequent_indent="      "
            )
            + "\n"
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
