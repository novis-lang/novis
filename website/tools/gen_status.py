"""Extract the project's live status from the implementation plan.

`docs/implementation-plan.md` opens with a status block whose field set is fixed
by AGENTS.md — `Status`, `Done`, `On disk`, `Toolchain`, `ADR slices landed`,
`Open now`, `Blocking` — and is overwritten in place every session. That block is
the one honest answer to "can I use this yet", and it is already maintained, so
this tool reads it rather than asking anyone to keep a second copy current.

Output is `website/src/generated/status.json`, consumed by:

* `astro.config.ts`, for the site-wide pre-alpha banner, and
* `<StatusTable>`, on the authored status page.

Deliberately *not* output: a whole generated status page. The page is authored,
so a human can explain the state of the project in their own words, and only the
facts inside it come from here. That is the split this site uses everywhere:
generated facts, authored prose.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from common import REPO, WEB, ok, warn

PLAN = REPO / "docs" / "implementation-plan.md"
PLAN_DIR = REPO / "docs" / "plan"
OUT = WEB / "src" / "generated" / "status.json"

FIELD = re.compile(r"\*\*([A-Z][A-Za-z ]+):\*\*\s*(.*)", re.DOTALL)


def _strip_markdown(s: str) -> str:
    s = re.sub(r"`([^`]*)`", r"\1", s)
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
    s = re.sub(r"\*\*([^*]*)\*\*", r"\1", s)
    s = re.sub(r"\s+", " ", s)
    return s.strip()


def _status_block(text: str) -> str:
    """The leading `>` blockquote, with its quote markers removed."""
    lines: list[str] = []
    started = False
    for line in text.split("\n"):
        if line.startswith(">"):
            started = True
            lines.append(re.sub(r"^>\s?", "", line))
        elif started and not line.strip():
            lines.append("")
        elif started:
            break
    return "\n".join(lines)


def _fields(block: str) -> dict[str, str]:
    """Split the block on its `**Field:**` markers, keeping each field's prose."""
    marks = list(re.finditer(r"\*\*([A-Z][A-Za-z ]+):\*\*", block))
    out: dict[str, str] = {}
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(block)
        out[m.group(1)] = _strip_markdown(block[m.end() : end])
    return out


def _milestones() -> list[dict[str, str]]:
    if not PLAN_DIR.is_dir():
        return []
    out: list[dict[str, str]] = []
    for f in sorted(PLAN_DIR.glob("m*.md")):
        head = f.read_text(encoding="utf-8").split("\n", 1)[0]
        m = re.match(r"^#\s*(M[0-9A-Za-z]+)\s*[—–-]\s*(.*)$", head.strip())
        if m:
            out.append({"id": m.group(1), "title": _strip_markdown(m.group(2))})
        else:
            out.append({"id": f.stem.upper(), "title": _strip_markdown(head.lstrip("# ").strip())})
    # M10 sorts before M2 alphabetically; sort on the numeric part instead.
    out.sort(key=lambda d: (int(re.sub(r"\D", "", d["id"]) or 0), d["id"]))
    return out


def main(quiet: bool = False) -> dict:
    if not PLAN.is_file():
        warn(f"{PLAN} not found — status data will be empty")
        data = {"available": False}
    else:
        text = PLAN.read_text(encoding="utf-8")
        fields = _fields(_status_block(text))
        status = fields.get("Status", "")
        date = ""
        dm = re.match(r"^(\d{4}-\d{2}-\d{2})\.\s*", status)
        if dm:
            date, status = dm.group(1), status[dm.end() :]
        milestone = ""
        mm = re.search(r"\bCurrent milestone\s+(M[0-9A-Za-z]+)", status)
        if mm:
            milestone = mm.group(1)

        data = {
            "available": True,
            "generatedFrom": "docs/implementation-plan.md",
            "date": date,
            "milestone": milestone,
            "stage": "pre-alpha",
            "fields": fields,
            "milestones": _milestones(),
        }

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    if not quiet:
        if data.get("available"):
            ok(
                f"status: milestone {data.get('milestone') or '?'} "
                f"({len(data.get('fields', {}))} fields, {len(data.get('milestones', []))} milestones)"
            )
        else:
            warn("status: no implementation plan found")
    return data


if __name__ == "__main__":
    main()
