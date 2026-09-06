#!/usr/bin/env python3
"""The docs migration driver. One-time tooling; it deletes itself when the migration lands.

`docs/agent/docs-migration.md` is the contract -- why the migration exists, what the end state is,
and what each unit does. This file is the machine that walks it across many sessions without losing
its place, and without ever letting the tree end a session in a half-migrated state.

    python tools/migrate-docs.py --status          the unit board: done, in flight, pending
    python tools/migrate-docs.py --snapshot        A3: capture the baseline. ONCE, before anything moves
    python tools/migrate-docs.py --topic-map       A5: propose the section-to-topic map for review
                                                   (refuses once the map has been judged by hand)
    python tools/migrate-docs.py --next            the next unit's work order -- a whole session prompt
    python tools/migrate-docs.py --unit B9         that unit's work order instead of the next one
    python tools/migrate-docs.py --apply FILE      one transaction: write, rewrite, gate, or restore
    python tools/migrate-docs.py --gate            run the gate now and change nothing
    python tools/migrate-docs.py --sweep           C8: the deep completeness pass, fixing what it can
    python tools/migrate-docs.py --self-destruct   C9: remove the migration machinery

THE RULE THIS TOOL EXISTS TO ENFORCE

A subagent never writes into the repository tree. It reads, and it writes an *apply-file* into the
scratchpad. Only `--apply` touches `docs/`, `crates/` or the goal manifests -- in the driving
session, one transaction at a time, each through the whole gate. That single rule dissolves
subagent-versus-subagent conflicts, subagent-versus-loop conflicts, and any possibility of a
half-applied topic.

THE GATE, AND WHY CHECK 3 IS THE POINT

Six checks; all of them pass or every byte the transaction touched is restored. Check 3 diffs each
goal's `orient.py` pack against the snapshot and fails if a goal *lost* information -- that is the
whole basis of the guarantee that already-planned goals keep working, and it is why the snapshot is
unit A3 rather than an afterthought. Nothing moves before it exists.

A lost `ADR NNNN § N` is not a loss when the topic map says that section became a rule and the pack
now carries that rule. Anything else lost is a failure.

rules-py:examples
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rules as rulebook  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
MIGRATION = ROOT / ".migration"
STATE = MIGRATION / "state.json"
SNAPSHOT = MIGRATION / "snapshot"
TOPIC_MAP = MIGRATION / "topic-map.json"
EXEMPLAR = MIGRATION / "exemplar.md"

ADR_DIR = ROOT / "docs" / "adr"
GOALS_DIR = ROOT / "docs" / "agent" / "goals"
PLAN = ROOT / "docs" / "agent" / "docs-migration.md"

#: An `ADR 0007 § 2` citation in any of its spellings, bare or inside a markdown link.
ADR_CITE = re.compile(r"(?:ADR\s+)?(\d{4})(?:\s*§+\s*([0-9]+[a-z]?(?:\s*,\s*[0-9]+[a-z]?)*))?")
ADR_BARE = re.compile(r"ADR\s+(\d{4})(?:\s*§+\s*([0-9]+[a-z]?))?")
ADR_LINK = re.compile(r"\]\((?:[./]*docs/adr/|)(\d{4})-[a-z0-9-]+\.md")

SCAN_GLOBS = ("crates/**/*.rs", "docs/**/*.md", "docs/**/*.toml", "tests/**/*.nvst", "tools/**/*.py")


# --------------------------------------------------------------------------- units


@dataclass
class Unit:
    id: str
    phase: str
    title: str
    detail: str
    parallel: bool = False
    topic: str | None = None


def unit_registry() -> list[Unit]:
    """The thirty-six units, in order. docs/agent/docs-migration.md is the prose for each."""
    units = [
        Unit("A1", "A", "tools/rules.py", "the rulebook library: load, validate, render, resolve", parallel=True),
        Unit("A2", "A", "tools/migrate-docs.py", "this driver: state, work orders, transactions"),
        Unit("A3", "A", "the snapshot", "every goal's orient pack, brief, novis.md, every ADR citation", parallel=True),
        Unit("A4", "A", "the gate", "all six checks -- `--gate`; the verify.py wiring is C6's"),
        Unit("A5", "A", "the topic map", "every ### section assigned to exactly one topic; USER REVIEWS"),
    ]
    pilot = ["errors"]
    cheap = ["programs", "statements", "enums", "iteration", "attributes", "testing", "expressions"]
    heavy = ["types", "classes", "core-api", "core-classes"]
    rest = ["security", "concurrency", "routing", "config", "packaging", "observability",
            "http-server", "tooling", "ide", "php-migration"]
    order = pilot + cheap + heavy + rest
    for n, topic in enumerate(order, start=1):
        detail = "THE PILOT -- runs alone, sets the style exemplar, USER REVIEWS after" if n == 1 else \
                 "author in parallel, apply serially, audit in parallel"
        units.append(Unit(f"B{n}", "B", f"topic: {topic}", detail, parallel=(n > 1), topic=topic))
    units += [
        Unit("C1", "C", "freeze the records", "strip Amends/Amended by, add changes:, move to docs/decisions/"),
        Unit("C2", "C", "retire the old trees", "docs/spec/, ground-rules.md, divergences.md, adr/README tables"),
        Unit("C3", "C", "AGENTS.md", "routing table, the five rules, the session workflow", parallel=True),
        Unit("C4", "C", "the agent docs", "commands, conventions, doc-style, session-prompt, loop-authoring", parallel=True),
        Unit("C5", "C", "adr.py", "fold and amend machinery deleted, audit kept", parallel=True),
        Unit("C6", "C", "re-point the tools", "orient, brief, dossier, chain, plan, session, check-links", parallel=True),
        Unit("C7", "C", "expiry for append-mostly knowledge", "playbook, carried-gaps, carried-refusals, guard-name-debt"),
        Unit("C8", "C", "the sweep", "deep completeness pass, auto-fixing what it can -- `--sweep`"),
        Unit("C9", "C", "self-destruct", "remove this tool, .migration/ and the plan -- `--self-destruct`"),
    ]
    return units


# --------------------------------------------------------------------------- state


def load_state() -> dict:
    if STATE.exists():
        return json.loads(STATE.read_text(encoding="utf-8"))
    return {"started": None, "units": {}, "snapshot": None}


def save_state(state: dict) -> None:
    MIGRATION.mkdir(parents=True, exist_ok=True)
    STATE.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")


def unit_status(state: dict, uid: str) -> str:
    return state["units"].get(uid, {}).get("status", "pending")


# --------------------------------------------------------------------------- shell


def run(cmd: list[str], cwd: Path = ROOT) -> tuple[int, str]:
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def read_verbatim(path: Path) -> str:
    """Read a tree file without translating its line endings.

    `Path.read_text` maps `\\r\\n` to `\\n`, and `write_text` writes back what it was handed -- so a
    transaction that only meant to change one citation rewrote every line ending in the file too.
    On a CRLF checkout that is ~250 files reported as modified with an empty content diff, and it
    makes the rollback's "every byte restored" false. Both directions go through these two.
    """
    with path.open("r", encoding="utf-8", newline="") as handle:
        return handle.read()


def write_verbatim(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(text)


def goal_tomls() -> list[Path]:
    return sorted(p for p in GOALS_DIR.glob("*.toml") if p.name != "chain.toml")


# --------------------------------------------------------------------------- A3: snapshot


def adr_sections() -> dict[str, str]:
    """Every `NNNN §N` anchor in docs/adr/, mapped to its heading text."""
    out: dict[str, str] = {}
    for path in sorted(ADR_DIR.glob("[0-9][0-9][0-9][0-9]-*.md")):
        number = path.name[:4]
        out[number] = path.name
        for line in path.read_text(encoding="utf-8").splitlines():
            match = re.match(r"^###\s+(\d+[a-z]?)\.\s+(.*)$", line.strip())
            if match:
                out[f"{number} §{match.group(1)}"] = match.group(2).strip()
    return out


def scan_adr_citations() -> dict[str, list[str]]:
    """Every distinct ADR citation in the tree, mapped to the sites that make it."""
    sites: dict[str, list[str]] = {}
    for pattern in SCAN_GLOBS:
        for path in sorted(ROOT.glob(pattern)):
            if not path.is_file() or ".migration" in path.parts or "target" in path.parts:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            rel = str(path.relative_to(ROOT)).replace("\\", "/")
            for line_no, line in enumerate(text.splitlines(), 1):
                for match in ADR_BARE.finditer(line):
                    key = f"{match.group(1)} §{match.group(2)}" if match.group(2) else match.group(1)
                    sites.setdefault(key, []).append(f"{rel}:{line_no}")
                for match in ADR_LINK.finditer(line):
                    sites.setdefault(match.group(1), []).append(f"{rel}:{line_no}")
    return sites


def carried_items(pack: str) -> set[str]:
    """The things an orient pack *carries*, as a set, for the gate's loss test.

    A pack is prose and its bytes move for a hundred innocent reasons, so byte-diffing it would
    report noise forever. What matters is whether the pack still names each thing it named: an ADR
    section, a module path, a `file:line` anchor, a rule id, a playbook bullet's lead-in.
    """
    items: set[str] = set()
    for line in pack.splitlines():
        for match in ADR_BARE.finditer(line):
            items.add(f"adr:{match.group(1)} §{match.group(2)}" if match.group(2) else f"adr:{match.group(1)}")
        for match in rulebook.CITATION.finditer(line):
            items.add(f"rule:{match.group(1)}")
        for match in re.finditer(r"\b((?:crates|docs|tools|tests|benches)/[\w./-]+\.\w+)", line):
            items.add(f"path:{match.group(1)}")
        stripped = line.strip()
        if stripped.startswith("### ") or re.match(r"^[A-Z][A-Z ,'`-]{8,}$", stripped):
            items.add(f"head:{stripped[:80]}")
    return items


def cmd_snapshot(state: dict, force: bool) -> int:
    if state.get("snapshot") and not force:
        print(f"a snapshot already exists, taken {state['snapshot']}.")
        print("it is the gate's baseline and must NOT be retaken mid-migration -- pass --force only")
        print("if you are certain nothing has migrated yet.")
        return 1

    SNAPSHOT.mkdir(parents=True, exist_ok=True)
    (SNAPSHOT / "orient").mkdir(exist_ok=True)

    print("capturing the baseline. nothing may move until this exists.\n")

    goals = goal_tomls()
    for toml in goals:
        code, out = run([sys.executable, "tools/orient.py", "--goal", str(toml.relative_to(ROOT))])
        target = SNAPSHOT / "orient" / f"{toml.stem}.txt"
        target.write_text(out, encoding="utf-8")
        mark = "ok " if code == 0 else "WARN"
        print(f"  {mark} orient pack  {toml.stem:<24} {len(out):>8} B  {len(carried_items(out)):>4} items")

    code, out = run([sys.executable, "tools/brief.py"])
    (SNAPSHOT / "brief.txt").write_text(out, encoding="utf-8")
    print(f"  ok  brief.py     {'':<24} {len(out):>8} B")

    novis = ROOT / "docs" / "novis.md"
    if novis.exists():
        shutil.copyfile(novis, SNAPSHOT / "novis.md")
        print(f"  ok  novis.md    {'':<24} {novis.stat().st_size:>8} B")

    sections = adr_sections()
    (SNAPSHOT / "adr-sections.json").write_text(json.dumps(sections, indent=2), encoding="utf-8")

    sites = scan_adr_citations()
    (SNAPSHOT / "citations.json").write_text(json.dumps(sites, indent=2), encoding="utf-8")
    total = sum(len(v) for v in sites.values())

    # Citations that are ALREADY broken today. The gate cannot demand zero from unit B1 -- it would
    # be failing every transaction for debt that predates the migration -- so it gates on *new*
    # breakage instead, and the sweep at C8 is where this set has to reach zero.
    legacy = {a: s for a, s in sites.items() if a not in sections}
    (SNAPSHOT / "known-dangling.json").write_text(json.dumps(legacy, indent=2), encoding="utf-8")
    if legacy:
        print(f"\n  {len(legacy)} citation(s) are ALREADY broken and become migration debt:")
        for anchor, where in sorted(legacy.items()):
            print(f"    {anchor:<12} {where[0]}")

    print(f"\n  {len(sections)} ADR anchors, {len(sites)} distinct citations, {total} sites")
    print(f"  {len(goals)} goal packs captured\n")

    state["snapshot"] = date.today().isoformat()
    state["started"] = state.get("started") or date.today().isoformat()
    state["units"].setdefault("A3", {})["status"] = "done"
    save_state(state)
    print(f"snapshot written to {SNAPSHOT.relative_to(ROOT)}. commit it -- the gate diffs against it.")
    return 0


# --------------------------------------------------------------------------- A5: topic map


def cmd_topic_map(state: dict, force: bool = False) -> int:
    """Propose an owner topic for every ADR section, from the citation graph and the titles.

    This is a *proposal*. The user reviews it, and it is the thing that makes parallel authoring
    safe: without it, two topics independently claim one rule, or neither does.

    It is also **destructive by construction**: it writes the heuristic's answer over whatever is
    already there. After the review the file is worth much more than this function can produce -- A5
    corrected 99 UNASSIGNED anchors and 310 misfilings by hand -- so a map carrying rows the
    heuristic would not reproduce is refused unless `--force`. See the guard below.
    """
    if not SNAPSHOT.exists():
        print("take the snapshot first: python tools/migrate-docs.py --snapshot", file=sys.stderr)
        return 1

    topics = [u.topic for u in unit_registry() if u.topic]
    keywords = {
        "types": ["type", "uint", "conversion", "cast", "array", "union", "mixed", "decimal", "literal", "shape"],
        "expressions": ["operator", "expression", "truthy", "precedence", "folding", "nullable"],
        "statements": ["statement", "storage", "static", "global", "include", "require", "exit"],
        "classes": ["class", "property", "interface", "trait", "clone", "magic", "constructor", "delegation"],
        "enums": ["enum"],
        "iteration": ["iteration", "generator", "foreach", "iterator"],
        "errors": ["error", "exception", "throw", "propagation", "escalation", "diagnostic", "refus"],
        "concurrency": ["task", "concurren", "spawn", "channel", "await", "scheduler"],
        "attributes": ["attribute", "annotation"],
        "testing": ["test", "coverage", "assert"],
        "core-api": ["core api", "convention", "member", "signature", "naming", "casing", "stdlib", "tier"],
        "core-classes": ["core\\", "db", "cache", "ratelimit", "uri", "uuid", "regex", "process", "codec"],
        "security": ["taint", "secret", "isolat", "sandbox", "capabilit", "authorit", "access", "sign", "password"],
        "routing": ["rout", "request", "dispatch"],
        "config": ["config", "reload", "control socket", "scheduled"],
        "packaging": ["package", "depend", "autoload", "executable", "attribution", "version", "artifact"],
        "observability": ["observab", "timeline", "trace", "profil", "metric", "export"],
        "http-server": ["http", "server", "proxied", "worker", "outbound", "origin"],
        "tooling": ["cli", "terminal", "format", "tool", "convert", "reflection", "hot reload"],
        "ide": ["ide", "lsp", "editor", "completion", "vscode", "resilient tree", "parsing"],
        "php-migration": ["php", "migration", "divergence", "compat"],
        "programs": ["program", "entry", "file", "discovery", "application", "audience", "framework"],
    }

    sections = json.loads((SNAPSHOT / "adr-sections.json").read_text(encoding="utf-8"))
    titles = {p.name[:4]: p.name[5:-3].replace("-", " ") for p in ADR_DIR.glob("[0-9][0-9][0-9][0-9]-*.md")}

    mapping: dict[str, str] = {}
    unsure: list[str] = []
    for anchor, heading in sorted(sections.items()):
        record = anchor.split()[0]
        haystack = f"{titles.get(record, '')} {heading}".lower()
        best, score = None, 0
        for topic in topics:
            hits = sum(1 for kw in keywords.get(topic, []) if kw in haystack)
            if hits > score:
                best, score = topic, hits
        if best is None:
            unsure.append(anchor)
            best = "UNASSIGNED"
        mapping[anchor] = best

    # THE GUARD. This command *overwrites* the map from the heuristic, and the heuristic is the
    # thing A5 exists to correct: it left 99 anchors UNASSIGNED and misfiled 310 more. Re-running it
    # over a reviewed map silently discards every one of those calls, and there is no signal that it
    # happened -- the file still looks like a topic map. So the rows that disagree with the
    # heuristic are counted first, and any disagreement refuses the run.
    if TOPIC_MAP.exists():
        current = json.loads(TOPIC_MAP.read_text(encoding="utf-8")).get("sections", {})
        judged = sorted(a for a, topic in current.items() if mapping.get(a) != topic)
        if judged and not force:
            print(f"REFUSED: {TOPIC_MAP.relative_to(ROOT)} carries {len(judged)} row(s) the heuristic "
                  f"would not reproduce.", file=sys.stderr)
            print("Those are hand calls -- A5's whole deliverable -- and this command would drop "
                  "every one:\n", file=sys.stderr)
            for anchor in judged[:8]:
                print(f"  {anchor:<16} {current[anchor]:<16} <- heuristic says "
                      f"{mapping.get(anchor, '(nothing)')}", file=sys.stderr)
            if len(judged) > 8:
                print(f"  ... and {len(judged) - 8} more", file=sys.stderr)
            resolved = sum(1 for a, t in current.items() if t != "UNASSIGNED")
            print(f"\n{resolved}/{len(current)} anchors currently carry a topic; the heuristic alone "
                  f"reaches {len(mapping) - len(unsure)}.", file=sys.stderr)
            print("\nIf you genuinely want the heuristic's proposal back, `--topic-map --force` "
                  "writes it and\nkeeps the current map beside it as topic-map.previous.json. "
                  "`git log -p -- .migration/topic-map.json`\nis the other way back.", file=sys.stderr)
            return 1
        if judged:
            previous = TOPIC_MAP.with_name("topic-map.previous.json")
            previous.write_text(TOPIC_MAP.read_text(encoding="utf-8"), encoding="utf-8")
            print(f"--force: {len(judged)} hand-judged row(s) overwritten. The map they were in is "
                  f"now {previous.relative_to(ROOT)}.\n")

    TOPIC_MAP.write_text(
        json.dumps({"generated": date.today().isoformat(), "sections": mapping}, indent=2) + "\n",
        encoding="utf-8",
    )
    counts: dict[str, int] = {}
    for topic in mapping.values():
        counts[topic] = counts.get(topic, 0) + 1
    print(f"proposed a topic for {len(mapping)} anchors -> {TOPIC_MAP.relative_to(ROOT)}\n")
    for topic, n in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"  {topic:<16} {n:>4}")
    print(f"\n{len(unsure)} anchor(s) UNASSIGNED and needing a human call.")
    print("REVIEW THIS FILE BEFORE PHASE B OPENS. It is what stops two topics claiming one rule.")
    return 0


# --------------------------------------------------------------------------- the gate


@dataclass
class GateResult:
    checks: list[tuple[str, bool, str]] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return all(passed for _, passed, _ in self.checks)

    def report(self) -> None:
        for name, passed, detail in self.checks:
            print(f"  {'ok  ' if passed else 'FAIL'} {name}")
            if not passed and detail:
                for line in detail.strip().splitlines()[:20]:
                    print(f"         {line}")


def gate(quick: bool = False) -> GateResult:
    """The six checks. All pass, or the transaction that called this is rolled back."""
    result = GateResult()

    # 1 -- no citation anywhere resolves to nothing, EXCEPT the debt that predates the migration.
    book = rulebook.Rulebook()
    dangling = book.check_citations()
    known = set(adr_sections())
    legacy_path = SNAPSHOT / "known-dangling.json"
    legacy = set(json.loads(legacy_path.read_text(encoding="utf-8"))) if legacy_path.exists() else set()
    for anchor, sites in scan_adr_citations().items():
        if anchor not in known and anchor not in legacy:
            dangling.append(rulebook.Finding(sites[0], f"cites ADR {anchor}, which does not exist"))
    label = "1 no new dangling citation" + (f" ({len(legacy)} legacy)" if legacy else "")
    result.checks.append((label, not dangling, "\n".join(str(f) for f in dangling)))

    # 2 -- every goal manifest resolves.
    bad: list[str] = []
    for toml in goal_tomls():
        text = toml.read_text(encoding="utf-8")
        for match in re.finditer(r'"(\d{4})(?:\s*§+[^"]*)?"', text):
            if match.group(1) not in known:
                bad.append(f"{toml.name}: names ADR {match.group(1)}, which does not exist")
        for match in rulebook.CITATION.finditer(text):
            if match.group(1) not in book.by_id:
                bad.append(f"{toml.name}: names rule:{match.group(1)}, which does not exist")
    result.checks.append(("2 goal manifests resolve", not bad, "\n".join(bad)))

    # 3 -- no goal loses information, measured against the snapshot.
    lost_report: list[str] = []
    if (SNAPSHOT / "orient").exists():
        mapping = {}
        if TOPIC_MAP.exists():
            mapping = json.loads(TOPIC_MAP.read_text(encoding="utf-8")).get("sections", {})
        for toml in goal_tomls():
            base = SNAPSHOT / "orient" / f"{toml.stem}.txt"
            if not base.exists():
                continue
            before = carried_items(base.read_text(encoding="utf-8"))
            _, now = run([sys.executable, "tools/orient.py", "--goal", str(toml.relative_to(ROOT))])
            after = carried_items(now)
            for item in sorted(before - after):
                if item.startswith("adr:"):
                    anchor = item[4:]
                    topic = mapping.get(anchor)
                    # Accounted for when the section became a rule and the pack carries that topic.
                    if topic and any(a.startswith(f"rule:{topic}/") for a in after):
                        continue
                if item.startswith("head:"):
                    continue  # a heading may legitimately be reworded; content items carry the test
                lost_report.append(f"{toml.stem}: lost {item}")
    result.checks.append(("3 no goal loses information", not lost_report, "\n".join(lost_report)))

    if quick:
        return result

    # 4 -- the reference regenerates and its examples still run.
    code, out = run([sys.executable, "tools/reference.py", "--check"])
    result.checks.append(("4 novis.md regenerates", code == 0, out))

    # 5 -- the standing doc checks.
    detail = []
    ok5 = True
    for cmd in (["tools/check-links.py"], ["tools/plan.py", "--check"], ["tools/rules.py", "--check"]):
        code, out = run([sys.executable, *cmd])
        if code != 0:
            ok5 = False
            detail.append(f"$ python {' '.join(cmd)}\n{out}")
    result.checks.append(("5 links, plan, rulebook", ok5, "\n".join(detail)))

    # 6 -- the build.
    code, out = run([sys.executable, "tools/verify.py"])
    result.checks.append(("6 verify.py", code == 0, out[-3000:]))
    return result


def cmd_gate(quick: bool) -> int:
    print("the gate" + (" (quick: checks 1-3 only)" if quick else "") + "\n")
    result = gate(quick=quick)
    result.report()
    print()
    if result.ok:
        print("gate clean")
        return 0
    print("gate FAILED -- a transaction seeing this restores every byte it touched")
    return 1


# --------------------------------------------------------------------------- status and work orders


def cmd_status(state: dict) -> int:
    units = unit_registry()
    done = sum(1 for u in units if unit_status(state, u.id) == "done")
    print(f"docs migration -- {done}/{len(units)} units done")
    if state.get("snapshot"):
        print(f"snapshot taken {state['snapshot']}")
    else:
        print("NO SNAPSHOT YET -- nothing may move. run --snapshot first.")
    print()
    phase = None
    for u in units:
        if u.phase != phase:
            phase = u.phase
            titles = {"A": "build the machine, move nothing", "B": "one topic, one transaction",
                      "C": "sweep and retire"}
            print(f"  -- phase {phase}: {titles[phase]}")
        st = unit_status(state, u.id)
        mark = {"done": "[x]", "in-flight": "[~]", "pending": "[ ]"}[st]
        par = " ||" if u.parallel else "   "
        print(f"  {mark}{par} {u.id:<4} {u.title:<28} {u.detail}")
    nxt = next((u for u in units if unit_status(state, u.id) != "done"), None)
    print(f"\nnext: {nxt.id} -- python tools/migrate-docs.py --next" if nxt else "\nall units done")
    return 0


def cmd_next(state: dict, uid: str | None) -> int:
    units = {u.id: u for u in unit_registry()}
    if uid:
        unit = units.get(uid.upper())
        if unit is None:
            print(f"no such unit: {uid}", file=sys.stderr)
            return 1
    else:
        unit = next((u for u in unit_registry() if unit_status(state, u.id) != "done"), None)
        if unit is None:
            print("all units done -- run --sweep, then --self-destruct")
            return 0

    if not state.get("snapshot") and unit.id != "A3":
        print("NO SNAPSHOT. run `python tools/migrate-docs.py --snapshot` first -- the gate has no", file=sys.stderr)
        print("baseline without it, and check 3 is the whole guarantee that goals keep working.", file=sys.stderr)
        return 1

    book = rulebook.Rulebook()
    print("=" * 78)
    print(f"UNIT {unit.id} -- {unit.title}")
    print("=" * 78)
    print(f"\n{unit.detail}\n")
    print(f"Read docs/agent/docs-migration.md first. It is the contract; this is the work order.\n")

    if unit.topic:
        mapping = {}
        if TOPIC_MAP.exists():
            mapping = json.loads(TOPIC_MAP.read_text(encoding="utf-8")).get("sections", {})
        owned = sorted(a for a, t in mapping.items() if t == unit.topic)
        sites = {}
        if (SNAPSHOT / "citations.json").exists():
            sites = json.loads((SNAPSHOT / "citations.json").read_text(encoding="utf-8"))

        print(f"ABSORB -- {len(owned)} anchor(s) this topic owns, per the reviewed topic map:\n")
        for anchor in owned:
            n = len(sites.get(anchor, []))
            print(f"  {anchor:<14} {n:>4} citation site(s)")
        records = sorted({a.split()[0] for a in owned})
        print(f"\n  read them in one call:")
        print(f"  python tools/peek.py " + " ".join(f"docs/adr/{r}-*.md" for r in records[:8]))

        print(f"\nCITATIONS AT STAKE -- rewritten by --apply, not by you:")
        total = sum(len(sites.get(a, [])) for a in owned)
        print(f"  {total} site(s) across the tree carry these anchors.\n")

        taken = sorted(book.by_id)
        print(f"NAMESPACE -- {len(taken)} rule id(s) already taken. Yours are all `{unit.topic}/...`,")
        print(f"  so you cannot collide with another topic by construction.\n")

        if EXEMPLAR.exists():
            print(f"EXEMPLAR -- match this style exactly. {EXEMPLAR.relative_to(ROOT)}:\n")
            for line in EXEMPLAR.read_text(encoding="utf-8").splitlines()[:40]:
                print(f"  | {line}")
            print()
        else:
            print("EXEMPLAR -- none yet. This unit sets it: after it lands, copy the chapter to")
            print(f"  {EXEMPLAR.relative_to(ROOT)} so every later topic matches it.\n")

    print("OUTPUT -- one apply-file, written to the scratchpad, never into the tree:\n")
    print(apply_format())
    print("\nGATE -- run before you hand the file over; --apply runs it again and rolls back on failure:")
    print("  python tools/migrate-docs.py --gate --quick     # checks 1-3, seconds")
    print("  python tools/migrate-docs.py --apply <file>     # the transaction, all six checks")

    state["units"].setdefault(unit.id, {})["status"] = "in-flight"
    save_state(state)
    return 0


def apply_format() -> str:
    return """  ## unit: B9
  ## topic: types

  ## json: docs/rules/types.json
  { "topic": "types", "title": "Types", "order": 20, "rules": [ ... ] }

  ## fragment: docs/rules/types/conversion.md
  `expr as T` produces a value of T or throws.

  ## fragment: docs/rules/types/declaration.md
  Every binding declares a type...

  ## remap: 0007 §2 -> types/conversion
  ## remap: 0066    -> types/nullable-conversion

  ## note:
  Anything the next session needs to know. Optional."""


# --------------------------------------------------------------------------- --apply


def parse_apply(path: Path) -> dict:
    doc: dict = {"json": {}, "fragments": {}, "remap": {}, "unit": None, "topic": None, "note": ""}
    key: tuple[str, str] | None = None
    buf: list[str] = []

    def flush() -> None:
        if key is None:
            return
        kind, name = key
        text = "\n".join(buf).strip("\n")
        if kind == "json":
            doc["json"][name] = text
        elif kind == "fragment":
            doc["fragments"][name] = text
        elif kind == "note":
            doc["note"] = text

    for line in path.read_text(encoding="utf-8").splitlines():
        header = re.match(r"^##\s+(unit|topic|json|fragment|remap|note):\s*(.*)$", line.strip())
        if header:
            flush()
            kind, rest = header.group(1), header.group(2).strip()
            buf = []
            if kind in ("unit", "topic"):
                doc[kind] = rest
                key = None
            elif kind == "remap":
                match = re.match(r"^(.+?)\s*->\s*(\S+)$", rest)
                if match:
                    doc["remap"][match.group(1).strip()] = match.group(2).strip()
                key = None
            else:
                key = (kind, rest)
            continue
        if key is not None:
            buf.append(line)
    flush()
    return doc


def cmd_apply(state: dict, path: Path, dry_run: bool) -> int:
    if not path.exists():
        print(f"no such apply-file: {path}", file=sys.stderr)
        return 1
    doc = parse_apply(path)
    if not doc["unit"]:
        print("the apply-file names no `## unit:`", file=sys.stderr)
        return 1

    writes: dict[Path, str] = {}
    for rel, text in doc["json"].items():
        try:
            json.loads(text)
        except json.JSONDecodeError as exc:
            print(f"{rel} is not valid JSON: {exc}", file=sys.stderr)
            return 1
        writes[ROOT / rel] = text.rstrip("\n") + "\n"
    for rel, text in doc["fragments"].items():
        writes[ROOT / rel] = text.rstrip("\n") + "\n"

    # The topic must be registered in _index.json before its chapter renders.
    if doc["topic"]:
        index_path = rulebook.INDEX
        index = json.loads(index_path.read_text(encoding="utf-8")) if index_path.exists() else {"topics": []}
        if not any(t["topic"] == doc["topic"] for t in index["topics"]):
            order = (max((t.get("order", 0) for t in index["topics"]), default=0)) + 10
            index["topics"].append({"topic": doc["topic"], "title": doc["topic"].replace("-", " ").title(),
                                    "order": order})
            writes[index_path] = json.dumps(index, indent=2) + "\n"

    rewrites = plan_citation_rewrites(doc["remap"])

    print(f"unit {doc['unit']}: {len(writes)} file(s) written, {len(rewrites)} file(s) re-cited\n")
    for p in sorted(writes):
        print(f"  write   {p.relative_to(ROOT)}")
    for p in sorted(rewrites):
        print(f"  re-cite {p.relative_to(ROOT)}  ({rewrites[p][1]} site(s))")

    if dry_run:
        print("\n--dry-run: nothing written")
        return 0

    backup: dict[Path, str | None] = {}
    try:
        for p, text in writes.items():
            backup[p] = read_verbatim(p) if p.exists() else None
            write_verbatim(p, text)
        for p, (text, _) in rewrites.items():
            backup.setdefault(p, read_verbatim(p))
            write_verbatim(p, text)

        if any(p.suffix == ".rs" for p in rewrites):
            for p in format_rust(backup):
                print(f"  fmt     {rulebook.Rulebook._rel(p)}")

        run([sys.executable, "tools/rules.py", "--render"])
        for p in rulebook.Rulebook().generated():
            backup.setdefault(p, None)

        print("\nrunning the gate...\n")
        result = gate()
        result.report()
        if not result.ok:
            raise RuntimeError("gate failed")
    except Exception as exc:  # noqa: BLE001 -- any failure restores every byte
        for p, original in backup.items():
            if original is None:
                p.unlink(missing_ok=True)
            else:
                write_verbatim(p, original)
        print(f"\nROLLED BACK: {exc}")
        print("every byte this transaction touched has been restored.")
        return 1

    state["units"].setdefault(doc["unit"], {})["status"] = "done"
    state["units"][doc["unit"]]["applied"] = date.today().isoformat()
    save_state(state)
    print(f"\nunit {doc['unit']} applied and gated clean. commit it, then --next.")
    return 0


#: One entry in a citation's section list: `2`, `2a`, the range `2-3`, or a named section like
#: `*Measured cost*` -- which is how a record with no numbered sections, ADR 0002, gets cited.
SECTION_ITEM = r"\d+[a-z]?(?:\s*[-–]\s*\d+[a-z]?)?|\*[^*\n]+\*"
#: What separates them: `§§ 2-3`, `§§ 3, 5`, `§§ 3 and 6`, `§§ 1, 3 and 4`.
SECTION_SEP = r",\s*and\s+|,\s*|\s+and\s+|\s*&\s*"
SECTION_LIST = rf"(?:{SECTION_ITEM})(?:\s*(?:{SECTION_SEP})(?:{SECTION_ITEM}))*"
#: What may sit between a record and its `§`. In a doc comment or a `//!` module header the two are
#: routinely on different lines, so the gap crosses a line break *and* the next line's comment
#: marker. Matching only `\s*` here read `[ADR 0046](...)\n/// § 2` as a **bare** record citation,
#: rewrote it to the bare record's rule and stranded the `§ 2` as prose: a citation that names the
#: wrong rule and still resolves, which is the one failure gate check 1 cannot see. Because the
#: match now spans the break, substituting it collapses the two lines into one correct one.
SECTION_GAP = r"[ \t]*(?:\r?\n[ \t]*(?://!|///|//|\#|\*|>)?[ \t]*)?"


def format_rust(backup: dict[Path, str | None]) -> list[Path]:
    """Re-format the Rust the rewrite touched, recording each file's pre-fmt bytes for rollback.

    A citation rewrite changes line *widths* -- `[ADR 0092](/docs/adr/0092-...md)` is 48 characters
    and `` `rule:errors/log-level` `` is 23 -- so a doc comment or a `panic!` argument that was
    correctly wrapped before is not afterwards, and gate check 6 fails on `cargo fmt` for a
    transaction that is otherwise perfect. B1 rolled back on exactly that.

    Two passes on purpose: `--check` names the files first so their originals are in `backup`
    before anything is written. Rolling back a formatted file we never recorded would leave the
    tree neither where it was nor where the transaction meant to put it.
    """
    _, out = run(["cargo", "fmt", "--all", "--", "--check"])
    touched: list[Path] = []
    for line in out.splitlines():
        match = re.match(r"^Diff in (.+?):\d+", line.strip())
        if not match:
            continue
        path = Path(match.group(1).replace("\\\\?\\", ""))
        if path in touched or not path.exists():
            continue
        touched.append(path)
        backup.setdefault(path, read_verbatim(path))
    if touched:
        run(["cargo", "fmt", "--all"])
    return touched


def plan_citation_rewrites(remap: dict[str, str]) -> dict[Path, tuple[str, int]]:
    """Turn every spelling of `ADR 0007 § 2` into `rule:types/conversion`, for the anchors this unit owns.

    One pass per *record* rather than per anchor. A citation names a section **list** -- `§§ 2-3`,
    `§§ 3, 5`, `§§ 1, 3 and 4` -- and an anchor-at-a-time pass rewrote the first number and left the
    rest as prose debris (`` `rule:errors/log-level`-3's ``). It also has to know the three ways a
    record is spelled: an inline link, a bare `ADR NNNN`, and a reference-style `[ADR NNNN]` whose
    `[ADR NNNN]: path` definition is dead once the last use of it is gone. The B1 pilot found 33
    such sites in 804; unfixed, that is ~700 across the twenty-two topics.

    A citation is rewritten only when **every** section it names resolves. One unowned section
    leaves the whole citation alone, because half a rewrite is worse than none.
    """
    if not remap:
        return {}
    by_record: dict[str, dict[str | None, str]] = {}
    for anchor, rid in remap.items():
        record = anchor.split()[0]
        section = anchor.split("§")[1].strip() if "§" in anchor else None
        by_record.setdefault(record, {})[section] = rid

    out: dict[Path, tuple[str, int]] = {}
    for pattern in SCAN_GLOBS:
        for path in sorted(ROOT.glob(pattern)):
            if not path.is_file() or ".migration" in path.parts or "target" in path.parts:
                continue
            try:
                text = read_verbatim(path)
            except (UnicodeDecodeError, OSError):
                continue
            if rulebook.EXAMPLES_ONLY in text:
                continue
            new, hits = rewrite_citations(text, by_record, path.suffix == ".nvst")
            if hits:
                debris = find_rewrite_debris(new)
                if debris:
                    raise RuntimeError(
                        f"the rewrite would corrupt {path.relative_to(ROOT)}: "
                        + "; ".join(debris[:3])
                        + "\nthis is a citation spelling `rewrite_citations` cannot read. Teach it "
                        "the spelling, or name the section explicitly in a `## remap:` line."
                    )
                out[path] = (new, hits)
    return out


#: What a correct rewrite never leaves behind. A citation's section list stranded as prose
#: (``  `rule:errors/log-level`-3  ``), or a rule token still wearing the brackets of the
#: reference-style link it replaced. Both were silent before the B1 pilot measured them: they
#: resolve, so gate check 1 passes, and only a reader notices. The transaction refuses instead.
DEBRIS = (
    (re.compile(rf"`rule:[a-z0-9-]+/[a-z0-9-]+`{SECTION_GAP}"
                rf"(?:[-–,]|\s+and\s+)\s*(?:\d|\*[^*\n]{{1,40}}\*)"),
     "stranded section"),
    (re.compile(r"\[`rule:[a-z0-9-]+/[a-z0-9-]+`\]"), "rule token inside link brackets"),
    (re.compile(rf"`rule:[a-z0-9-]+/[a-z0-9-]+`{SECTION_GAP}§"), "leftover section marker"),
)


def find_rewrite_debris(text: str) -> list[str]:
    """Every place a rewrite left a citation half-converted. Empty is the only acceptable answer."""
    out: list[str] = []
    for pattern, what in DEBRIS:
        for match in pattern.finditer(text):
            line = text[: match.start()].count("\n") + 1
            out.append(f"{what} at line {line}: {match.group(0).strip()!r}")
    return out


def rewrite_citations(
    text: str, by_record: dict[str, dict[str | None, str]], keep_line_breaks: bool = False
) -> tuple[str, int]:
    """One file's worth of the rewrite above. Split out so it is testable without a tree.

    `keep_line_breaks` is for `.nvst` only. Collapsing a citation's two lines into one is free
    everywhere else, but a case file's expected output pins line numbers in `case.nvs` -- a
    citation in a `//` comment inside a `--FILE--` block sits *above* the code the diagnostic
    points at, so joining it shifts every `--> case.nvs:NN:CC` below it and the case fails for a
    reason that has nothing to do with what it tests. B6 rolled back on exactly that. Here the
    record becomes the rule token and the break is left standing, so the line count never moves.
    """
    hits = 0
    for record, sections in by_record.items():
        # `[ADR 0020](url)`, `[ADR 0020]`, or a bare `ADR 0020` -- but never the `[ADR 0020]:` of a
        # reference-link definition, which is handled after the last use of it has gone.
        cite = re.compile(
            rf"(?:\[ADR\s+{record}\]|(?<!\[)ADR\s+{record}(?!\d))"
            rf"(?:\([^)]*\))?"
            rf"(?!\s*:)"
            rf"(?:(?P<gap>{SECTION_GAP})§+[ \t]*(?P<sections>{SECTION_LIST}))?"
            rf"(?!{SECTION_GAP}§)"
        )

        def substitute(match: re.Match[str]) -> str:
            nonlocal hits
            ids = resolve_sections(match.group("sections"), sections)
            if ids is None:
                return match.group(0)
            hits += 1
            tokens = [f"`rule:{rid}`" for rid in ids]
            out = tokens[0] if len(tokens) == 1 else ", ".join(tokens[:-1]) + " and " + tokens[-1]
            gap = match.group("gap") or ""
            if keep_line_breaks and "\n" in gap:
                # Put the break and its comment marker back. Its trailing indent goes only when
                # the prose that followed the `§` brings its own space, so `§ 2 says` does not
                # come back as `//  says` and `§ 2's` does not come back as `//'s`.
                tail = match.string[match.end() :]
                out += gap.rstrip(" \t") if tail[:1] in (" ", "\t") else gap
            return out

        text = cite.sub(substitute, text)
        text = drop_dead_link_definition(text, record)
    return text, hits


def resolve_sections(sections: str | None, table: dict[str | None, str]) -> list[str] | None:
    """The rule ids a citation's section list names, or None to leave the citation untouched."""
    if sections is None:
        rid = table.get(None)
        return [rid] if rid else None
    ids: list[str] = []
    for item in expand_section_list(sections):
        rid = table.get(item)
        if rid is None:
            return None
        if rid not in ids:
            ids.append(rid)
    return ids or None


def expand_section_list(sections: str) -> list[str]:
    """`1, 3 and 4` -> [1, 3, 4]; `2-3` -> [2, 3]. A range is only a range between two integers."""
    out: list[str] = []
    for part in re.split(SECTION_SEP, sections.strip()):
        part = part.strip()
        if not part:
            continue
        span = re.fullmatch(r"(\d+)\s*[-–]\s*(\d+)", part)
        if span and int(span.group(1)) <= int(span.group(2)):
            out.extend(str(n) for n in range(int(span.group(1)), int(span.group(2)) + 1))
        else:
            out.append(part)
    return out


def drop_dead_link_definition(text: str, record: str) -> str:
    """Remove `[ADR 0095]: ../path.md` once nothing in the file references it any more.

    Left behind, it is either an orphan definition or -- worse -- gets rewritten itself into
    ``[`rule:...`]: path``, which is not a link definition at all.
    """
    if re.search(rf"\[ADR\s+{record}\](?!\s*:)", text):
        return text
    return re.sub(rf"^[^\S\n]*(?://!|///)?[^\S\n]*\[ADR\s+{record}\]:[^\n]*\n?", "", text, flags=re.M)


# --------------------------------------------------------------------------- C8: the sweep


def cmd_sweep(state: dict, fix: bool) -> int:
    """The deep completeness pass. Fixes what it can; names precisely what it cannot.

    Run at C8, after every topic has landed. This is the check that the migration is *finished*
    rather than merely stopped -- the loose ends a topic-at-a-time walk leaves behind are exactly
    the ones nobody notices, so they are enumerated here and mostly repaired automatically.
    """
    book = rulebook.Rulebook()
    fixed: list[str] = []
    manual: list[str] = []

    print("the completeness sweep\n")

    # 1 -- every ADR section claimed by some rule.
    mapping = json.loads(TOPIC_MAP.read_text(encoding="utf-8")).get("sections", {}) if TOPIC_MAP.exists() else {}
    claimed = {b for r in book.by_id.values() for b in r.because}
    unclaimed = sorted({a for a in mapping if a.split()[0] not in claimed})
    if unclaimed:
        manual += [f"anchor never became a rule: {a} (mapped to {mapping[a]})" for a in unclaimed]
    print(f"  {'ok  ' if not unclaimed else 'FAIL'} every mapped anchor became a rule "
          f"({len(mapping) - len(unclaimed)}/{len(mapping)})")

    # 2 -- no dangling citation of either kind.
    dangling = book.check_citations()
    known = set(adr_sections())
    stale_adr = {a: s for a, s in scan_adr_citations().items() if a not in known}
    # The sweep grants no legacy exemption: the debt the snapshot recorded has to be gone by now.
    print(f"  {'ok  ' if not dangling and not stale_adr else 'FAIL'} no dangling citation, legacy "
          f"debt included ({len(dangling)} rule, {len(stale_adr)} ADR)")
    manual += [str(f) for f in dangling]
    manual += [f"stale ADR citation {a} at {s[0]}" for a, s in stale_adr.items()]

    # 3 -- generated files current. Auto-fixable, always.
    stale = [p for p, text in book.generated().items()
             if not p.exists() or p.read_text(encoding="utf-8") != text]
    if stale and fix:
        run([sys.executable, "tools/rules.py", "--render"])
        fixed += [f"re-rendered {p.relative_to(ROOT)}" for p in stale]
    elif stale:
        manual += [f"stale generated file: {p.relative_to(ROOT)}" for p in stale]
    print(f"  {'ok  ' if not stale or fix else 'FAIL'} generated files current ({len(stale)} stale)")

    # 4 -- nothing still points at a retired tree. Auto-fixable where the target moved predictably.
    retired = {
        r"docs/adr/(\d{4})-[a-z0-9-]+\.md": "docs/decisions/\\1.md",
        r"docs/adr/ground-rules\.md": "docs/ground-rules.md",
        r"docs/adr/divergences\.md": "docs/divergences.md",
    }
    touched = 0
    for pattern in SCAN_GLOBS:
        for path in sorted(ROOT.glob(pattern)):
            if not path.is_file() or ".migration" in path.parts or "target" in path.parts:
                continue
            try:
                text = read_verbatim(path)
            except (UnicodeDecodeError, OSError):
                continue
            if rulebook.EXAMPLES_ONLY in text:
                continue
            new = text
            for old, repl in retired.items():
                new = re.sub(old, repl, new)
            if new != text:
                touched += 1
                if fix:
                    path.write_text(new, encoding="utf-8")
                    fixed.append(f"re-pointed links in {path.relative_to(ROOT)}")
                else:
                    manual.append(f"still points at a retired path: {path.relative_to(ROOT)}")
    print(f"  {'ok  ' if not touched or fix else 'FAIL'} no link into a retired tree ({touched} file(s))")

    # 5 -- every decision record frozen, with a `changes:` block.
    decisions = ROOT / "docs" / "decisions"
    unfrozen: list[str] = []
    if decisions.exists():
        for path in sorted(decisions.glob("*.md")):
            head = path.read_text(encoding="utf-8")[:600]
            if "changes:" not in head:
                unfrozen.append(path.name)
            if "Amended by" in head or "**Amends:**" in head:
                unfrozen.append(f"{path.name} still carries amendment metadata")
    else:
        unfrozen.append("docs/decisions/ does not exist -- unit C1 has not run")
    manual += [f"record not frozen: {u}" for u in unfrozen]
    print(f"  {'ok  ' if not unfrozen else 'FAIL'} every record frozen ({len(unfrozen)} outstanding)")

    # 6 -- every rule reachable from a topic, every fragment declared.
    findings = book.validate()
    manual += [str(f) for f in findings]
    print(f"  {'ok  ' if not findings else 'FAIL'} rulebook internally consistent ({len(findings)})")

    # 7 -- the gate itself.
    print("\n  running the full gate...\n")
    result = gate()
    result.report()
    if not result.ok:
        manual.append("the gate does not pass")

    print()
    if fixed:
        print(f"FIXED AUTOMATICALLY ({len(fixed)}):")
        for line in fixed[:40]:
            print(f"  {line}")
        if len(fixed) > 40:
            print(f"  ... and {len(fixed) - 40} more")
        print()
    if manual:
        print(f"NEEDS A HUMAN OR A SESSION ({len(manual)}):")
        for line in manual[:40]:
            print(f"  {line}")
        if len(manual) > 40:
            print(f"  ... and {len(manual) - 40} more")
        print("\nsweep INCOMPLETE -- fix these, re-run, and only then --self-destruct")
        return 1

    state["units"].setdefault("C8", {})["status"] = "done"
    save_state(state)
    print("sweep clean: the migration is complete. next: --self-destruct")
    return 0


# --------------------------------------------------------------------------- C9: self-destruct


def cmd_self_destruct(state: dict, confirm: bool) -> int:
    """Remove the one-time machinery. The migration's last act is to leave no trace of itself."""
    if unit_status(state, "C8") != "done":
        print("--sweep must be clean first. the sweep is what proves there is nothing left to do.",
              file=sys.stderr)
        return 1

    targets = [MIGRATION, PLAN, Path(__file__).resolve()]
    print("this removes the migration machinery permanently:\n")
    for t in targets:
        size = sum(f.stat().st_size for f in t.rglob("*") if f.is_file()) if t.is_dir() else t.stat().st_size
        print(f"  {t.relative_to(ROOT)}  ({size:,} B)")
    print("\n`git log` keeps all of it. docs/rules/ and docs/decisions/ are untouched.")
    print("\ntools/rules.py SURVIVES -- it is the permanent rulebook library. Strip only its one")
    print("migration-only command by hand: `--unclaimed`, which reads .migration/topic-map.json.")

    if not confirm:
        print("\nre-run with --yes to do it.")
        return 0

    for t in targets:
        if t.is_dir():
            shutil.rmtree(t)
        else:
            t.unlink()
        print(f"removed {t.relative_to(ROOT)}")
    print("\ndone. run `python tools/verify.py` and commit.")
    return 0


# --------------------------------------------------------------------------- main


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--status", action="store_true", help="the unit board")
    ap.add_argument("--snapshot", action="store_true", help="A3: capture the gate's baseline")
    ap.add_argument("--topic-map", action="store_true", help="A5: propose the section-to-topic map")
    ap.add_argument("--next", action="store_true", help="the next unit's work order")
    ap.add_argument("--unit", metavar="ID", help="that unit's work order instead of the next")
    ap.add_argument("--apply", metavar="FILE", help="apply one transaction, gated")
    ap.add_argument("--gate", action="store_true", help="run the gate and change nothing")
    ap.add_argument("--quick", action="store_true", help="with --gate: checks 1-3 only")
    ap.add_argument("--sweep", action="store_true", help="C8: the deep completeness pass")
    ap.add_argument("--self-destruct", action="store_true", help="C9: remove the machinery")
    ap.add_argument("--dry-run", action="store_true", help="with --apply: say what it would do")
    ap.add_argument("--no-fix", action="store_true", help="with --sweep: report, repair nothing")
    ap.add_argument("--force", action="store_true",
                    help="with --snapshot: retake it. with --topic-map: overwrite hand-judged rows")
    ap.add_argument("--yes", action="store_true", help="with --self-destruct: actually do it")
    args = ap.parse_args()

    if hasattr(sys.stdout, "reconfigure"):  # the § and box characters below are not cp1252
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
        sys.stderr.reconfigure(encoding="utf-8", newline="\n")

    state = load_state()

    if args.snapshot:
        return cmd_snapshot(state, args.force)
    if args.topic_map:
        return cmd_topic_map(state, args.force)
    if args.gate:
        return cmd_gate(args.quick)
    if args.apply:
        return cmd_apply(state, Path(args.apply), args.dry_run)
    if args.sweep:
        return cmd_sweep(state, fix=not args.no_fix)
    if args.self_destruct:
        return cmd_self_destruct(state, args.yes)
    if args.next or args.unit:
        return cmd_next(state, args.unit)
    return cmd_status(state)


if __name__ == "__main__":
    sys.exit(main())
