#!/usr/bin/env python3
"""One call, whole orientation. Prints the parts of this repository an agent reads at the
start of nearly every session: where the plan stands, the map of every milestone plus the
lead of the current one, the title and status of every ADR (not its content), what the guard
tests hold, and what actually exists on disk.

It stores no facts of its own. Every line it prints is sliced out of a file it names, so it
cannot go stale. When a slice comes back empty it says so loudly rather than printing a
plausible nothing.

Size is controlled by one rule, and the rule is structural rather than a tripwire:

    this digest may only print text whose length is bounded by a COUNT OF ENTITIES,
    never by a LENGTH OF PROSE.

So a section is a *projection* -- one bounded line per milestone, per ADR, per guard test,
per named status field -- and every section budget is *derived* from that count
(`n_entities * per_entity_cap + slack`). Adding an ADR, a milestone or a paragraph of prose
therefore cannot put anything over budget; the budget moves with the repository.

That leaves exactly one way to overrun, and it is local: writing one over-long entity. Those
are reported as `violations` -- named with the file and line to edit, and *never truncated*,
so the digest stays complete and correct while the complaint stays actionable. `--check`
exits non-zero on them, which is how CI and a pre-commit run catch a bloated line in the same
session that wrote it instead of a trim pass discovering it a week later.

The one deliberate shortening is a `projection`: the current/next milestone's lead paragraph,
which is legitimately long prose that this digest only ever wanted the head of. That is marked
inline with the file to open, and is NOT a violation -- it is normal operation.

Usage:  python .claude/brief.py            # the digest
        python .claude/brief.py --no-git   # skip the working-tree section
        python .claude/brief.py --check    # lint only: per-entity caps, exit 1 on violations
"""

import re
import subprocess
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
ADR_README = ROOT / "docs" / "adr" / "README.md"
PROBE = ROOT / "benches" / "abi-probe"

# ---------------------------------------------------------------- per-entity caps
#
# These are the only numbers a doc author has to keep in mind, and each one governs a single
# line or field that one author wrote in one place. Exceeding one is a violation naming that
# line -- a thirty-second local edit -- not a signal that the doc set needs a trim pass.

STATUS_FIELD_CAP = 400  # one named field of the plan's leading status block
MILESTONE_HEADING_CAP = 120  # one `### Mn -- title` line, links stripped
ADR_DECISION_CAP = 160  # the decision cell of one ADR index row (the part an author writes)
ADR_ROW_OVERHEAD = 70  # filename + separators + an occasional non-Accepted status tag
NO_ADR_BULLET_CAP = 140  # one project-start decision title
GUARD_LINE_CAP = 200  # one guard test's name + bounds

# ------------------------------------------------------------- projection caps
#
# Deliberate shortening of prose this digest only ever wanted the head of. Never a violation.

LEAD_CAP = 700  # current milestone's opening paragraph
VERIFY_CAP = 600  # current milestone's `**Verify:**` paragraph
NEXT_LEAD_CAP = 350  # next milestone's opening paragraph

# The plan's status block has a FIXED field set. This is what stops it drifting back into free
# prose: it used to carry a paragraph per milestone in flight, which is append-shaped and grew
# without bound. A field is overwritten in place; an unrecognised one is a violation, so a new
# paragraph type cannot quietly appear.
STATUS_FIELDS = [
    "Status",
    "Done",
    "On disk",
    "Toolchain",
    "ADR slices landed",
    "Open now",
    "Blocking",
]

DISK_BUDGET = 2_000
GIT_BUDGET = 2_000
GIT_CHANGED_LINE_CAP = 40

out = []

# (file, line, message) -- an over-long entity. `--check` exits 1 on these.
violations = []
# (title, actual_bytes, derived_budget) -- for the --check headroom report.
section_sizes = []


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


def cap_entity(text, cap, path, line, what):
    """Per-entity cap. Over-length is reported and left intact -- the digest never loses
    content to a budget, because a budget overrun here is a doc bug with a named fix."""
    n = nbytes(text)
    if n > cap:
        violations.append(
            (path, line, f"{what} is {n} B against a {cap} B cap -- shorten it by {n - cap} B")
        )
    return text


def project(text, cap, source_hint):
    """Deliberate shortening of legitimately-long prose. Not a violation."""
    if nbytes(text) <= cap:
        return text
    kept = text.encode("utf-8")[:cap].decode("utf-8", "ignore")
    kept = kept.rsplit(" ", 1)[0]
    return f"{kept} ... [lead continues at {source_hint}]"


def read(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def rel(path):
    """Relative path with forward slashes, regardless of host OS."""
    return path.relative_to(ROOT).as_posix()


def strip_links(text):
    """`[ADR 0002](adr/0002-error-propagation.md)` -> `ADR 0002`. A link target is ~50 bytes
    of no value in a digest whose reader has CLAUDE.md's routing table."""
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


def measure(title, fn):
    """Run a section, record what it actually cost against the budget it derived."""
    start = len(out)
    budget = fn()
    body = "\n".join(out[start:])
    if budget is not None:
        section_sizes.append((title, nbytes(body), budget))


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
        return None

    known = set(STATUS_FIELDS)
    seen = set()
    for name, text, lineno in fields:
        seen.add(name)
        if name not in known:
            violations.append(
                (
                    rel(PLAN),
                    lineno,
                    f'unknown status field "{name}" -- this block has a fixed field set '
                    f"({', '.join(STATUS_FIELDS)}). Fold this into one of them rather than "
                    "adding a paragraph; that is how the block stayed bounded.",
                )
            )
        cap_entity(text, STATUS_FIELD_CAP, rel(PLAN), lineno, f'status field "{name}"')
        emit(wrap(name, strip_links(text)))

    missing = [f for f in STATUS_FIELDS if f not in seen]
    if missing:
        warn(f"status block is missing the field(s): {', '.join(missing)}")

    return len(STATUS_FIELDS) * STATUS_FIELD_CAP + 500


# ------------------------------------------------------- milestone map + leads


HEADING_RE = re.compile(r"^### (M\d+[A-Z]?)\s*[—-]\s*(.*)$")


def parse_milestones(plan_text):
    lines = plan_text.split("\n")
    found = []
    for lineno, line in enumerate(lines, start=1):
        m = HEADING_RE.match(line)
        if m:
            title = strip_links(m.group(2)).replace("**", "").strip()
            found.append({"id": m.group(1), "title": title, "line": lineno})
    return found


def paragraph_after(lines, idx):
    """First non-empty paragraph after 0-based line index `idx`."""
    i = idx + 1
    while i < len(lines) and not lines[i].strip():
        i += 1
    buf = []
    while i < len(lines) and lines[i].strip() and not lines[i].startswith("### "):
        buf.append(lines[i].strip())
        i += 1
    return strip_links(" ".join(buf))


def verify_paragraph(lines, start, end):
    """The `**Verify:**` paragraph inside a milestone's span, if it has one."""
    i = start
    while i < end:
        if lines[i].startswith("**Verify:**"):
            buf = []
            while i < end and lines[i].strip():
                buf.append(lines[i].strip())
                i += 1
            # the "-- Mn acceptance" label already says what this is
            return strip_links(" ".join(buf))[len("**Verify:**") :].strip()
        i += 1
    return ""


def pick_current_next(status_text, milestones):
    """`Current: **M3**` / `Next: **M4**` out of the Status field, with a loud fallback."""
    cur = re.search(r"[Cc]urrent:\s*\*\*(M\d+[A-Z]?)\*\*", status_text or "")
    nxt = re.search(r"[Nn]ext:\s*\*\*(M\d+[A-Z]?)\*\*", status_text or "")
    if cur:
        current = cur.group(1)
    else:
        mentioned = re.findall(r"\*\*(M\d+[A-Z]?)\*\*", status_text or "")
        if not mentioned:
            return None, None
        current = mentioned[-1]
        warn(
            f'the Status field does not say `Current: **Mn**`; guessed {current} from the last '
            "milestone it names. Spell it out so this is not a guess."
        )
    if nxt:
        return current, nxt.group(1)
    ids = [m["id"] for m in milestones]
    if current in ids:
        i = ids.index(current)
        return current, (ids[i + 1] if i + 1 < len(ids) else None)
    return current, None


def run_milestones(status_text, plan_text):
    milestones = parse_milestones(plan_text)
    if not milestones:
        section("MILESTONE MAP", f"{rel(PLAN)} (## Milestones)")
        warn(f"found no `### Mn --` milestone headings in {rel(PLAN)}")
        return None

    current, nxt = pick_current_next(status_text, milestones)
    section(
        "MILESTONE MAP, AND THE LEAD OF THE CURRENT ONE",
        f"{rel(PLAN)} (## Milestones) -- open it at the line number for a milestone's full text",
    )
    emit("Every milestone, one line each. Only the current and next milestones' opening")
    emit("paragraphs are printed; a milestone's full text is deliberately not in this digest.")
    emit()

    for m in milestones:
        line = f"{m['id']} -- {m['title']}"
        cap_entity(line, MILESTONE_HEADING_CAP, rel(PLAN), m["line"], f"milestone heading {m['id']}")
        marker = "  <- current" if m["id"] == current else ("  <- next" if m["id"] == nxt else "")
        emit(f"  {rel(PLAN)}:{m['line']}  {line}{marker}")

    lines = plan_text.split("\n")
    by_id = {m["id"]: m for m in milestones}

    def lead_of(mid, cap):
        m = by_id.get(mid)
        if not m:
            return
        idx = m["line"] - 1
        para = paragraph_after(lines, idx)
        if not para:
            warn(f"milestone {mid} has no opening paragraph to slice")
            return
        emit()
        emit(f"-- {mid} lead ({rel(PLAN)}:{m['line']})")
        emit(textwrap.fill(project(para, cap, f"{rel(PLAN)}:{m['line']}"), width=100))

    lead_of(current, LEAD_CAP)

    m = by_id.get(current)
    if m:
        start = m["line"] - 1
        later = [x["line"] - 1 for x in milestones if x["line"] - 1 > start]
        end = later[0] if later else len(lines)
        ver = verify_paragraph(lines, start, end)
        if ver:
            emit()
            emit(f"-- {current} acceptance ({rel(PLAN)}:{m['line']})")
            emit(textwrap.fill(project(ver, VERIFY_CAP, f"{rel(PLAN)}:{m['line']}"), width=100))

    if nxt:
        lead_of(nxt, NEXT_LEAD_CAP)

    return (
        len(milestones) * MILESTONE_HEADING_CAP + LEAD_CAP + VERIFY_CAP + NEXT_LEAD_CAP + 1_200
    )


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
    expected cell shape is passed through verbatim rather than silently reshaped or dropped.

    Yields (printed line, line number, decision cell) -- the cap applies to the decision
    alone, because that is the part an author writes. A long filename is a slug nobody chose
    for its length, and should not eat into the sentence's budget."""
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


def run_adr_index():
    section(
        "DECISIONS WITH AN ADR (number, decision, status)",
        f"{rel(ADR_README)} (the index table)",
    )
    text = read(ADR_README)
    if text is None:
        warn(f"could not read {rel(ADR_README)} at all")
        return None
    rows, linenos = [], []
    for lineno, line in enumerate(text.split("\n"), start=1):
        if line.startswith("| ["):
            rows.append(line)
            linenos.append(lineno)
    if not rows:
        warn(f"could not slice the ADR index table out of {rel(ADR_README)}")
        return None

    for line, lineno, decision in compress_adr_rows(rows, linenos):
        if decision is None:
            violations.append(
                (
                    rel(ADR_README),
                    lineno,
                    "ADR index row does not split into the expected link/decision/status cells "
                    "-- an unescaped `|` inside a cell is the usual cause, and renders the row "
                    "broken on GitHub too. Write it as `\\|`.",
                )
            )
        else:
            cap_entity(decision, ADR_DECISION_CAP, rel(ADR_README), lineno,
                       "ADR index decision cell")
        emit(line)
    emit()
    emit("Every ADR not marked otherwise is Accepted -- the status column is printed only for")
    emit("the exceptions. This table intentionally omits the full rule and reasoning -- open the")
    emit('file it names. CLAUDE.md, section "Where to look", maps a topic to the same file.')
    return len(rows) * (ADR_DECISION_CAP + ADR_ROW_OVERHEAD) + 500


def run_no_adr_decisions():
    section(
        "DECISIONS WITH NO ADR (titles only)",
        f"{rel(ADR_README)} section 'Decisions taken at project start'",
    )
    text = read(ADR_README)
    if text is None:
        warn(f"could not read {rel(ADR_README)} at all")
        return None
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
        return None
    for title, lineno in bullets:
        cap_entity(title, NO_ADR_BULLET_CAP, rel(ADR_README), lineno, "project-start decision title")
        emit(f"- {title}")
    emit()
    emit("(each is one paragraph in that section: open it for the reasoning)")
    return len(bullets) * NO_ADR_BULLET_CAP + 300


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
    emit("A test name is the claim; a bracketed threshold is the bound it holds. If one of these fails,")
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
        for line, lineno in entries:
            cap_entity(line, GUARD_LINE_CAP, rel(rs_file), lineno, "guard test line")
            emit(f"  {line}")
            count += 1
    if count == 0:
        warn(
            f"found no guard tests under {rel(PROBE)}/tests -- that directory is the source of "
            "truth, check it"
        )
        return None

    benches_dir = PROBE / "benches"
    if benches_dir.is_dir():
        bench_files = sorted(benches_dir.glob("*.rs"))
        if bench_files:
            emit()
            emit("benchmarks (unguarded, for tracking figures by hand):")
            for f in bench_files:
                emit(f"  {rel(f)}")
    return count * GUARD_LINE_CAP + 800


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
    spec_dir = ROOT / "docs" / "spec"
    if spec_dir.is_dir() and any(spec_dir.iterdir()):
        emit(f"spec:    {listing('docs/spec')}")
    else:
        emit("spec:    unwritten -- say so rather than inferring language semantics")
    return DISK_BUDGET


# ------------------------------------------------------------- working tree


def run_git():
    if not (ROOT / ".git").is_dir():
        return None
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
    return GIT_BUDGET


# ------------------------------------------------------------------ drivers


def build(no_git):
    plan_text = read(PLAN)
    if plan_text is None:
        warn(f"could not read {rel(PLAN)} at all")
        fields = []
    else:
        fields = parse_status_fields(plan_text)

    measure("WHERE THE PLAN STANDS", lambda: run_status(fields))
    if plan_text is not None:
        status_text = next((t for n, t, _ in fields if n == "Status"), "")
        measure("MILESTONE MAP", lambda: run_milestones(status_text, plan_text))
    measure("ADR INDEX", run_adr_index)
    measure("DECISIONS WITH NO ADR", run_no_adr_decisions)
    measure("GUARD TESTS", run_guard_tests)
    measure("WHAT EXISTS ON DISK", run_disk)
    if not no_git:
        measure("WORKING TREE", run_git)


def violation_report():
    lines = []
    for path, lineno, message in violations:
        lines.append(f"  {path}:{lineno}  {message}")
    return lines


def run_check():
    """Lint only. Exit 1 on an over-long entity, naming the line to edit."""
    build(no_git=True)
    total = sum(size for _, size, _ in section_sizes)
    if violations:
        sys.stdout.write("brief.py --check: FAIL\n\n")
        for line in violation_report():
            sys.stdout.write(line + "\n")
        sys.stdout.write(
            "\nEach line above is one over-long entity with a named fix. Shorten it where it "
            "lives.\nA cap is per-entity on purpose: adding an ADR, a milestone or a paragraph "
            "of prose\ncan never trip this, so there is nothing here to housekeep on a schedule.\n"
        )
        return 1

    sys.stdout.write("brief.py --check: OK\n\n")
    for title, size, budget in section_sizes:
        pct = (size * 100) // budget if budget else 0
        sys.stdout.write(f"  {title:<28} {size:>6} B of {budget:>6} B derived budget  ({pct}%)\n")
    sys.stdout.write(f"\n  {'digest total':<28} {total:>6} B\n")
    sys.stdout.write(
        "\nBudgets are derived from entity counts, so they move with the repository.\n"
    )
    return 0


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass
    argv = sys.argv[1:]
    if "--check" in argv:
        return run_check()

    build(no_git="--no-git" in argv)

    if violations:
        banner = [
            "!! brief.py: the digest below is COMPLETE, but a doc entity is over its cap.",
            "!! Nothing was truncated -- these are lint hits with a named fix, one line each:",
        ]
        banner.extend(violation_report())
        banner.append("!! Run `python .claude/brief.py --check` for this list on its own.")
        out[0:0] = banner + [""]

    sys.stdout.write("\n".join(out).lstrip("\n") + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
