#!/usr/bin/env python3
"""One call, whole orientation. Prints the parts of this repository an agent reads at the
start of nearly every session: where the plan stands, the title and status of every ADR (not
its content), what the guard tests hold, and what actually exists on disk.

It stores no facts of its own. Every line it prints is sliced out of a file it names, so it
cannot go stale. When a slice comes back empty it says so loudly rather than printing a
plausible nothing.

Unlike the old `brief.sh`, every section here also carries a hard byte budget. If the source
doc it slices grows past that budget, the section is truncated with a loud note naming the
file to open directly -- so this digest cannot silently balloon back to tens of KB just
because a doc grew between DOC_CLEANUP_PROMPT.md passes. A budget getting hit routinely is a
signal that doc, not this script, needs trimming.

Usage:  python .claude/brief.py            # the digest
        python .claude/brief.py --no-git   # skip the working-tree section
"""

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
ADR_README = ROOT / "docs" / "adr" / "README.md"
PROBE = ROOT / "benches" / "abi-probe"

# Per-section byte budgets. Deliberately generous relative to today's content -- these exist
# to catch runaway growth, not to squeeze the digest further than the doc trims already have.
STATUS_BUDGET = 4_000
MILESTONE_BUDGET = 7_000
ADR_TABLE_BUDGET = 10_000
NO_ADR_BUDGET = 3_000
GUARD_BUDGET = 6_000
DISK_BUDGET = 2_000
GIT_CHANGED_LINE_CAP = 40
TOTAL_BUDGET = STATUS_BUDGET + MILESTONE_BUDGET + ADR_TABLE_BUDGET + NO_ADR_BUDGET + GUARD_BUDGET + DISK_BUDGET + 2_000

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


def cap_text(text, budget, source_hint):
    """Truncate on a line boundary at `budget` bytes, with a loud note if it had to."""
    encoded = text.encode("utf-8")
    if len(encoded) <= budget:
        return text
    lines = text.split("\n")
    kept = []
    total = 0
    for line in lines:
        line_len = len(line.encode("utf-8")) + 1
        if total + line_len > budget:
            break
        kept.append(line)
        total += line_len
    kept_text = "\n".join(kept)
    return (
        kept_text
        + f"\n\n!! brief.py: this section exceeds its {budget}-byte budget and was cut here "
        f"-- read {source_hint} directly for the rest. If this keeps happening, "
        "DOC_CLEANUP_PROMPT.md's trim pass is overdue.\n"
    )


def read(path):
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def rel(path):
    """Relative path with forward slashes, regardless of host OS."""
    return path.relative_to(ROOT).as_posix()


# ---------------------------------------------------------------- plan status


def plan_status_block(plan_text):
    lines = plan_text.split("\n")
    collected = []
    found = False
    for line in lines[1:]:  # skip the H1 title line, same as the old awk NR==1 { next }
        if line.startswith(">"):
            found = True
            collected.append(re.sub(r"^> ?", "", line))
        elif found:
            break
    return "\n".join(collected)


def run_status():
    section("WHERE THE PLAN STANDS", f"{rel(PLAN)} (leading status block)")
    plan_text = read(PLAN)
    if plan_text is None:
        warn(f"could not read {PLAN} at all")
        return None
    status = plan_status_block(plan_text)
    if status.strip():
        emit(cap_text(status, STATUS_BUDGET, rel(PLAN)))
    else:
        warn(f"no status block at the top of {PLAN} -- read it directly")
    return status, plan_text


# ------------------------------------------------------- current + next milestone


def run_milestones(status, plan_text):
    # last match wins, same as the old `sed -n '...' | head -1` with sed's greedy `.*` --
    # the status line names M1 as done and M2 as in-progress, in that order, and "current"
    # means the *last*-named one.
    matches = re.findall(r"Milestone \*\*(M\d+)\*\*", status or "")
    if not matches:
        warn(f"could not read the current milestone out of the status block -- read {PLAN} section Milestones")
        return
    current = matches[-1]
    section(f"MILESTONE {current}, AND THE ONE AFTER IT", f"{rel(PLAN)} (## Milestones)")

    lines = plan_text.split("\n")
    heading_re = re.compile(r"^### (M\d+) ")
    printing = False
    seen_others = 0
    collected = []
    for line in lines:
        hm = heading_re.match(line)
        if hm:
            if hm.group(1) == current:
                printing = True
            elif printing:
                seen_others += 1
                if seen_others > 1:
                    break
        if printing:
            collected.append(line)
    text = "\n".join(collected)
    if text.strip():
        emit(cap_text(text, MILESTONE_BUDGET, rel(PLAN)))
    else:
        warn(f"could not slice milestone {current} out of {PLAN}")


# ------------------------------------------------------------------ decisions


def run_adr_index():
    section("DECISIONS WITH AN ADR (number, decision, status)", f"{rel(ADR_README)} (the index table)")
    text = read(ADR_README)
    if text is None:
        warn(f"could not read {ADR_README} at all")
        return
    rows = [line for line in text.split("\n") if line.startswith("| [")]
    if rows:
        emit(cap_text("\n".join(rows), ADR_TABLE_BUDGET, rel(ADR_README)))
        emit()
        emit("This table intentionally omits the full rule and reasoning -- open the file it names for those.")
        emit('CLAUDE.md, section "Where to look", maps a topic to the same file.')
    else:
        warn(f"could not slice the ADR index table out of {ADR_README}")


def run_no_adr_decisions():
    section(
        "DECISIONS WITH NO ADR (titles only)",
        f"{rel(ADR_README)} section 'Decisions taken at project start'",
    )
    text = read(ADR_README)
    if text is None:
        warn(f"could not read {ADR_README} at all")
        return
    inside = False
    bullets = []
    for line in text.split("\n"):
        if line.startswith("## Decisions taken at project start"):
            inside = True
            continue
        if inside and line.startswith("## "):
            break
        if inside and line.startswith("**"):
            m = re.match(r"\*\*(.*?)\*\*", line)
            if m:
                bullets.append(f"- {m.group(1)}")
    if bullets:
        emit(cap_text("\n".join(bullets), NO_ADR_BUDGET, rel(ADR_README)))
        emit()
        emit("(each is one paragraph in that section: open it for the reasoning)")
    else:
        warn(f"could not slice the project-start decisions out of {ADR_README}")


# --------------------------------------------------------------- guard tests


def parse_guard_tests(rs_text):
    """Port of brief.sh's awk state machine: test name, its [MAX/MIN const bounds], its
    feature gate (from a #[cfg(feature = "...")] on the mod or the test itself)."""
    order = []
    bound = {}
    gate = {}
    pending_feat = None
    mod_feat = None
    pending = False
    cur = None

    cfg_re = re.compile(r'^\s*#\[cfg\(feature\s*=\s*("[^"]+")\)\]')
    mod_re = re.compile(r"^\s*mod\s")
    test_attr_re = re.compile(r"^\s*#\[test\]")
    fn_re = re.compile(r"^\s*fn\s+([a-z_0-9]+)\s*\(\)")
    const_re = re.compile(r"^\s*const\s+((?:MAX|MIN)[A-Z_]*:.*?);.*$")

    for line in rs_text.split("\n"):
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
                bound[cur] = (bound.get(cur, "") + (", " if cur in bound else "") + entry)
        if line.startswith("}"):
            cur = None
            mod_feat = None

    lines = []
    for name in order:
        parts = name
        if name in bound:
            parts += f"  [{bound[name]}]"
        if name in gate:
            parts += f"  (feature {gate[name]})"
        lines.append(f"  {parts}")
    return lines


def run_guard_tests():
    section("WHAT IS ACTUALLY GUARDED", f"{rel(PROBE)}/tests/*.rs -- authoritative for every measured number")
    emit("A test name is the claim; a bracketed threshold is the bound it holds. If one of these fails,")
    emit("the ADR naming it needs revisiting -- not the threshold.")

    tests_dir = PROBE / "tests"
    rs_files = sorted(tests_dir.glob("*.rs")) if tests_dir.is_dir() else []
    blocks = []
    for rs_file in rs_files:
        text = read(rs_file)
        if text is None:
            continue
        lines = parse_guard_tests(text)
        if lines:
            blocks.append(f"\n{rel(rs_file)}\n" + "\n".join(lines))
    if blocks:
        emit(cap_text("\n".join(blocks), GUARD_BUDGET, f"{rel(PROBE)}/tests/"))
    else:
        warn(f"found no guard tests under {PROBE}/tests -- that directory is the source of truth, check it")

    benches_dir = PROBE / "benches"
    if benches_dir.is_dir():
        bench_files = sorted(benches_dir.glob("*.rs"))
        if bench_files:
            emit()
            emit("benchmarks (unguarded, for tracking figures by hand):")
            for f in bench_files:
                emit(f"  {rel(f)}")


# -------------------------------------------------------------- what exists


def run_disk():
    section("WHAT EXISTS ON DISK", "the filesystem, not the plan")

    def listing(dirname):
        d = ROOT / dirname
        if not d.is_dir():
            return ""
        return " ".join(sorted(p.name for p in d.iterdir()))

    lines = [
        f"crates:  {listing('crates')}",
        f"benches: {listing('benches')}",
        f"docs:    {listing('docs')}",
    ]
    spec_dir = ROOT / "docs" / "spec"
    if spec_dir.is_dir() and any(spec_dir.iterdir()):
        lines.append(f"spec:    {listing('docs/spec')}")
    else:
        lines.append("spec:    unwritten -- say so rather than inferring language semantics")
    emit(cap_text("\n".join(lines), DISK_BUDGET, "the filesystem directly (ls crates/ benches/ docs/)"))


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

    branch = git("rev-parse", "--abbrev-ref", "HEAD")
    emit(f"branch:  {branch or '(unknown)'}")

    head = git("log", "-1", "--format=%h %s")
    emit(f"HEAD:    {head or '(unknown)'}")

    changed = git("status", "--porcelain")
    if changed:
        changed_lines = changed.split("\n")
        shown = changed_lines[:GIT_CHANGED_LINE_CAP]
        emit("changed:")
        for line in shown:
            emit(f"  {line}")
        if len(changed_lines) > GIT_CHANGED_LINE_CAP:
            emit(f"  ... and {len(changed_lines) - GIT_CHANGED_LINE_CAP} more -- run `git status` directly")
    else:
        emit("changed: nothing")


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass
    no_git = "--no-git" in sys.argv[1:]

    status_result = run_status()
    if status_result is not None:
        status, plan_text = status_result
        run_milestones(status, plan_text)

    run_adr_index()
    run_no_adr_decisions()
    run_guard_tests()
    run_disk()
    if not no_git:
        run_git()

    text = "\n".join(out).lstrip("\n") + "\n"
    sys.stdout.write(text)

    actual = len(text.encode("utf-8"))
    if actual > TOTAL_BUDGET:
        sys.stdout.write(
            f"\n!! brief.py: total digest is {actual} bytes, over its {TOTAL_BUDGET}-byte design budget "
            "even after per-section caps -- a budget above needs tightening, or a source doc needs trimming.\n"
        )


if __name__ == "__main__":
    main()
