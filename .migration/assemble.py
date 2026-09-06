"""Merge one unit's author parts into an apply-file, and say what it had to decide.

    python .migration/assemble.py --unit B17 --dir <scratchpad>

Reads `<dir>/B17.config.json`, `<dir>/B17.order.md` and the parts the config names, writes
`<dir>/B17.apply.md`, and maintains two scratch files beside them. Migration-only; `.migration/` goes
with the self-destruct at C9.

The config is small on purpose:

    {"topic": "packaging", "title": "Packaging", "order": 170,
     "parts": ["B17.partA.md", "B17.partB.md", "B17.partC.md"],
     "bridge": {},                 # explicit extra bridges, anchor -> landed id
     "claim_ok": [],               # anchors the map gives elsewhere that an author may claim anyway
     "rename": {},                 # old id -> new id, for a slug two co-authors both picked
     "stitch": []}                 # [[rule id, sibling id], ...] hand seeAlso pairs

Each author wrote a bare `## json-rules:` array; the chapter is one file, so the arrays are
concatenated in part order under one topic header. Four things are decided here rather than in an
author's file:

  * a section-*list* key (`0077 §§ 1-3`) is dropped. The driver splits an anchor on `§` and takes
    field 1, so a list yields the empty section -- a key that can never match, silently. The
    rewriter expands a list itself and resolves each item against the per-section keys.

  * a bridge key is *added* for every section of this unit's records that another, already-landed
    topic owns. A citation is rewritten only when every section it names resolves, so `ADR 0077
    §§ 1-3 and 5` is left naming a record forever unless `§3` resolves too. Its rule is the other
    chapter's, and pointing at it is not claiming it: check 3 looks a lost anchor up in the topic
    map by its full key and exempts against that topic. The bridge table is derived, not typed:
    the topic map says who owns the section and `state.json` says what rule it became. A section
    whose owner is not landed yet gets its bridge the next time this unit is assembled, which is
    why an assembly is re-run after a sibling lands.

  * a `## wants: <own id> -> <anchor>` line is a seeAlso an author could not spell because the
    target is a co-author's or a sibling chapter's. It is resolved against this unit's own remaps,
    then the banked tables, then the bridges. One that does not resolve yet is carried forward in
    `<dir>/wants-carried.json` and retried at every later assembly: for this unit it lands in the
    JSON when it resolves; for a unit already applied it is written to `<dir>/wants-landed.json`
    for the driving session to fold in by hand, since that chapter's JSON is in the tree.

  * a `## sweep: file:line -- what` line is a hand-fix for C8. They are appended to
    `.migration/sweep-items.md` under this unit's heading, replacing the heading's earlier content
    when the unit is assembled again.

Beyond that, every `seeAlso`, `guardedBy` and inline `rule:` token is validated against the landed
ids plus this chapter's, every guardedBy path against the disk, and every owned anchor of the work
order must have a remap.
"""

import argparse
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MIGRATION = ROOT / ".migration"
SWEEP = MIGRATION / "sweep-items.md"

ap = argparse.ArgumentParser()
ap.add_argument("--unit", required=True)
ap.add_argument("--dir", required=True, help="the scratchpad holding the parts, order and config")
args = ap.parse_args()
UNIT = args.unit
HERE = pathlib.Path(args.dir)

cfg = json.loads((HERE / f"{UNIT}.config.json").read_text(encoding="utf-8"))
TOPIC, TITLE, ORDER = cfg["topic"], cfg["title"], cfg["order"]
PARTS = cfg["parts"]
EXTRA_BRIDGE: dict[str, str] = cfg.get("bridge", {})
CLAIM_OK: set[str] = set(cfg.get("claim_ok", []))
RENAME: dict[str, str] = cfg.get("rename", {})
STITCH: list[list[str]] = cfg.get("stitch", [])
OUT = HERE / f"{UNIT}.apply.md"
CARRIED = HERE / "wants-carried.json"
LANDED_WANTS = HERE / "wants-landed.json"


def norm(anchor: str) -> str:
    """`0085 § 2` and `0085 §2` are one key to the driver; spell it the work order's way."""
    record, _, section = anchor.partition("§")
    return f"{record.strip()} §{section.strip()}" if section else record.strip()


# ---- what the tree already knows -------------------------------------------------------------

state = json.loads((MIGRATION / "state.json").read_text(encoding="utf-8"))
tmap = json.loads((MIGRATION / "topic-map.json").read_text(encoding="utf-8"))["sections"]
banked: dict[str, str] = {}
landed_topics: set[str] = set()
for uid, u in state["units"].items():
    if u.get("status") == "done":
        banked.update(u.get("remap", {}))
landed: set[str] = set()
for f in (ROOT / "docs/rules").glob("*.json"):
    if f.name != "_index.json":
        chapter = json.loads(f.read_text(encoding="utf-8"))
        landed |= {r["id"] for r in chapter["rules"]}
        landed_topics.add(chapter["topic"])
order_text = (HERE / f"{UNIT}.order.md").read_text(encoding="utf-8")
owned = [norm(m.group(1)) for m in re.finditer(r"^  (\d{4}(?: §\S+)?)\s+\d+ citation", order_text, re.M)]
records = sorted({a.split(" ")[0] for a in owned})


def owner(anchor: str) -> str | None:
    return tmap.get(anchor) or tmap.get(anchor.split(" ")[0])


# ---- read the parts ---------------------------------------------------------------------------

rules: list[dict] = []
fragments: list[tuple[str, str]] = []
remaps: dict[str, str] = {}
wants: list[tuple[str, str]] = []
sweeps: list[str] = []
notes: list[tuple[str, str]] = []
dropped: list[str] = []
problems: list[str] = []


def rename_id(rid: str) -> str:
    return RENAME.get(rid, rid)


def rename_text(text: str) -> str:
    for old, new in RENAME.items():
        text = text.replace(f"rule:{old}", f"rule:{new}")
    return text


for name in PARTS:
    text = (HERE / name).read_text(encoding="utf-8")
    blocks = re.split(r"^## (json-rules|fragment|remap|wants|sweep|note):[ \t]*(.*)$", text, flags=re.M)
    for i in range(1, len(blocks), 3):
        kind, rest, body = blocks[i], blocks[i + 1].strip(), blocks[i + 2]
        if kind == "json-rules":
            try:
                part_rules = json.loads(body)
            except json.JSONDecodeError as exc:
                problems.append(f"{name}: json-rules does not parse: {exc}")
                continue
            for r in part_rules:
                r["id"] = rename_id(r["id"])
                r["seeAlso"] = [rename_id(s) for s in r.get("seeAlso", [])] or r.get("seeAlso", [])
                if not r["seeAlso"]:
                    r.pop("seeAlso", None)
                r["_part"] = name
            rules.extend(part_rules)
        elif kind == "fragment":
            path = rest
            for old, new in RENAME.items():
                path = path.replace(f"/{old.split('/', 1)[1]}.md", f"/{new.split('/', 1)[1]}.md")
            fragments.append((path, rename_text(body.strip("\n"))))
        elif kind == "remap":
            match = re.match(r"^(.+?)\s*->\s*(\S+)$", rest)
            if not match:
                problems.append(f"{name}: unreadable remap line `{rest}`")
                continue
            anchor, rid = match.group(1).strip(), rename_id(match.group(2).strip())
            if "§§" in anchor:
                dropped.append(f"{name}: {anchor} -> {rid}")
                continue
            anchor = norm(anchor)
            if anchor in remaps and remaps[anchor] != rid:
                problems.append(f"{anchor} remapped twice: {remaps[anchor]} and {rid}")
            remaps[anchor] = rid
        elif kind == "wants":
            match = re.match(r"^(\S+)\s*->\s*(.+?)\s*$", rest)
            if not match:
                problems.append(f"{name}: unreadable wants line `{rest}`")
                continue
            wants.append((rename_id(match.group(1)), norm(match.group(2))))
        elif kind == "sweep":
            line = " ".join((rest + " " + body.strip()).split())
            if line:
                sweeps.append(line)
        elif kind == "note":
            notes.append((name, body.strip("\n")))

# ---- ids, fragments, duplicates -------------------------------------------------------------------

ids = [r["id"] for r in rules]
for dup in sorted({i for i in ids if ids.count(i) > 1}):
    parts = [r["_part"] for r in rules if r["id"] == dup]
    problems.append(f"duplicate rule id {dup} in {', '.join(parts)} -- add a `rename` to the config")
for i in ids:
    if not i.startswith(TOPIC + "/"):
        problems.append(f"{i} is not under {TOPIC}/")
frag_ids = [TOPIC + "/" + pathlib.PurePosixPath(p).stem for p, _ in fragments]
for p, _ in fragments:
    if not p.startswith(f"docs/rules/{TOPIC}/"):
        problems.append(f"fragment path {p} is not under docs/rules/{TOPIC}/")
for missing in sorted(set(ids) - set(frag_ids)):
    problems.append(f"{missing} has no fragment")
for extra in sorted(set(frag_ids) - set(ids)):
    problems.append(f"fragment {extra} has no rule")
for dup in sorted({i for i in frag_ids if frag_ids.count(i) > 1}):
    problems.append(f"fragment {dup} appears twice")

# ---- bridges ----------------------------------------------------------------------------------------

bridge: dict[str, str] = dict(EXTRA_BRIDGE)
unbridged: list[str] = []
for anchor, top in tmap.items():
    if anchor.split(" ")[0] not in records or top == TOPIC:
        continue
    if anchor in remaps:
        if anchor in CLAIM_OK or anchor not in banked:
            continue  # claimed on purpose, or declined by its owner and free to take
        problems.append(f"{anchor} was claimed by an author; it is {top}'s, landed as {banked[anchor]}")
        continue
    if anchor in banked:
        bridge[anchor] = banked[anchor]
    else:
        unbridged.append(f"{anchor} ({top}{' - not landed' if top not in landed_topics else ' - never remapped'})")
for anchor, rid in bridge.items():
    if anchor in remaps and remaps[anchor] != rid:
        problems.append(f"{anchor} claimed by an author and bridged to {rid}")
    remaps.setdefault(anchor, rid)

# ---- wants -------------------------------------------------------------------------------------------

by_id = {r["id"]: r for r in rules}


def resolve(anchor: str) -> str | None:
    return remaps.get(anchor) or banked.get(anchor) or bridge.get(anchor)


def add_see_also(rid: str, sib: str) -> None:
    if rid == sib:
        return
    r = by_id[rid]
    r.setdefault("seeAlso", [])
    if sib not in r["seeAlso"]:
        r["seeAlso"].append(sib)


carried: list[dict] = json.loads(CARRIED.read_text(encoding="utf-8")) if CARRIED.exists() else []
landed_wants: list[dict] = json.loads(LANDED_WANTS.read_text(encoding="utf-8")) if LANDED_WANTS.exists() else []
resolved_now: list[str] = []
carried_now: list[str] = []
landed_now: list[str] = []

for rid, anchor in wants:
    if rid not in by_id:
        problems.append(f"wants: {rid} is not a rule of this unit")
        continue
    entry = {"unit": UNIT, "id": rid, "anchor": anchor}
    if entry not in carried:
        carried.append(entry)

still: list[dict] = []
for entry in carried:
    target = resolve(entry["anchor"])
    if entry["unit"] == UNIT:
        if target:
            add_see_also(entry["id"], target)
            resolved_now.append(f"{entry['id']} -> {entry['anchor']} = {target}")
        else:
            top = owner(entry["anchor"])
            if top is None:
                problems.append(f"wants: {entry['id']} -> {entry['anchor']}: no topic owns that anchor")
            elif top in landed_topics and entry["anchor"] not in banked and top != TOPIC:
                problems.append(f"wants: {entry['id']} -> {entry['anchor']}: {top} is landed and never remapped it")
            elif top == TOPIC:
                problems.append(f"wants: {entry['id']} -> {entry['anchor']}: this unit's own anchor, and no author remapped it")
            else:
                carried_now.append(f"{entry['id']} -> {entry['anchor']} ({top}, not landed)")
                still.append(entry)
    else:
        if target:
            item = dict(entry, resolved=target)
            if item not in landed_wants:
                landed_wants.append(item)
            landed_now.append(f"{entry['unit']}: {entry['id']} -> {entry['anchor']} = {target}")
        else:
            still.append(entry)

for rid, sib in STITCH:
    if rid not in by_id:
        problems.append(f"stitch: {rid} is not a rule of this unit")
        continue
    add_see_also(rid, sib)

# ---- validation --------------------------------------------------------------------------------------

known = landed | set(ids)
for r in rules:
    if r.get("status") not in ("shipped", "designed"):
        problems.append(f"{r['id']}: status {r.get('status')!r}")
    if not all(re.fullmatch(r"\d{4}", b) for b in r.get("because", [])):
        problems.append(f"{r['id']}: because {r.get('because')}")
    for s in r.get("seeAlso", []):
        if s not in known:
            problems.append(f"{r['id']}: seeAlso {s} resolves to nothing")
    for g in r.get("guardedBy", []):
        if not (ROOT / g).exists():
            problems.append(f"{r['id']}: guardedBy {g} does not exist")
    if "divergesFromPhp" in r and r["divergesFromPhp"].rstrip().endswith("."):
        problems.append(f"{r['id']}: divergesFromPhp ends with a period")
    for key in list(r):
        if key not in ("id", "title", "status", "because", "divergesFromPhp", "seeAlso", "guardedBy", "_part"):
            problems.append(f"{r['id']}: unknown field {key}")
for p, body in fragments:
    for tok in re.findall(r"rule:([a-z0-9-]+/[a-z0-9-]+)", body):
        if tok not in known:
            problems.append(f"{p}: inline rule:{tok} resolves to nothing")
    if re.search(r"\bADR\s+\d{4}", body):
        problems.append(f"{p}: names an ADR in prose")

for anchor, rid in remaps.items():
    if rid.startswith(TOPIC + "/"):
        if rid not in set(ids):
            problems.append(f"remap {anchor} -> {rid}, which no rule defines")
    elif rid not in landed:
        problems.append(f"bridge {anchor} -> {rid} is not landed")

missing = [a for a in owned if a not in remaps]
if missing:
    problems.append("owned anchors without a remap: " + ", ".join(missing))

if problems:
    print("PROBLEMS:")
    for p in problems:
        print("  " + p)
    sys.exit(1)

# ---- write -------------------------------------------------------------------------------------------

for r in rules:
    r.pop("_part", None)
chapter = {"topic": TOPIC, "title": TITLE, "order": ORDER, "rules": rules}
out = [f"## unit: {UNIT}", f"## topic: {TOPIC}", "", f"## json: docs/rules/{TOPIC}.json",
       json.dumps(chapter, indent=2, ensure_ascii=False), ""]
for path, body in fragments:
    out += [f"## fragment: {path}", body, ""]


def sort_key(anchor: str) -> tuple:
    record, _, section = anchor.partition("§")
    return (record.strip(), int(re.sub(r"\D", "", section) or 0), section)


for anchor in sorted(remaps, key=sort_key):
    out.append(f"## remap: {anchor:9} -> {remaps[anchor]}")
out.append("")
out.append("## note:")
for name, body in notes:
    out.append(f"### from {name}\n{body}\n")
OUT.write_text("\n".join(out), encoding="utf-8")

CARRIED.write_text(json.dumps(still, indent=2), encoding="utf-8")
LANDED_WANTS.write_text(json.dumps(landed_wants, indent=2), encoding="utf-8")

# the sweep file: replace this unit's section, or append it
heading = f"## {UNIT} — {TOPIC}"
sweep_text = SWEEP.read_text(encoding="utf-8") if SWEEP.exists() else "# Sweep items\n"
section = heading + "\n\n" + "".join(f"- {s}\n" for s in sweeps) if sweeps else ""
pattern = re.compile(rf"^{re.escape(heading)}\n.*?(?=^## |\Z)", re.M | re.S)
if pattern.search(sweep_text):
    sweep_text = pattern.sub(lambda _: section, sweep_text)
else:
    sweep_text = sweep_text.rstrip("\n") + "\n\n" + section
SWEEP.write_text(sweep_text.rstrip("\n") + "\n", encoding="utf-8")

# ---- report ------------------------------------------------------------------------------------------

designed = sum(1 for r in rules if r["status"] == "designed")
print(f"{UNIT} {TOPIC}: {len(rules)} rules ({designed} designed), {len(fragments)} fragments, "
      f"{len(remaps)} remaps over {len(owned)} owned anchors")
if dropped:
    print(f"dropped {len(dropped)} section-list key(s):")
    for d in dropped:
        print(f"  {d}")
print(f"bridged {len(bridge)}:")
for a in sorted(bridge, key=sort_key):
    print(f"  {a:9} -> {bridge[a]}")
if unbridged:
    print(f"not bridged, no landed rule yet ({len(unbridged)}):")
    for u in unbridged:
        print(f"  {u}")
print(f"wants resolved now ({len(resolved_now)}):")
for w in resolved_now:
    print(f"  {w}")
if carried_now:
    print(f"wants carried forward ({len(carried_now)}):")
    for w in carried_now:
        print(f"  {w}")
if landed_now:
    print(f"carried wants of landed units that resolve now ({len(landed_now)}) -> {LANDED_WANTS.name}:")
    for w in landed_now:
        print(f"  {w}")
print(f"sweep items appended: {len(sweeps)}")
print(f"-> {OUT}")
