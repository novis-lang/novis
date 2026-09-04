#!/usr/bin/env python3
"""Pick the playbook bullets a goal actually needs, and report the ones that have gone stale.

`docs/agent/playbook.md` is append-mostly by decision -- every trap a session writes down is
charged to every session after it -- and it has grown accordingly: **501 KB in 670 bullets** across
six sections, from 47 KB in 86 when this script was written. It is the single largest thing
`orient.py` ships. Run `--check` for the live figures; a number quoted in prose is stale the week
after it is written, which is why the two above are dated by that contrast rather than trusted.

The fix is not to split the file and not to trim it. `orient.py` slices it twice -- by the goal's
`[context] playbook`, then again by the paths the session's own item names -- and an entry there
may name **one bullet** rather than a section: `"Tooling > A whole ADR"`. What was missing is any
cheap way to decide *which* bullets a given file set implies. That is this script.

    python tools/playbook.py                       # every section and bullet, one line, with sizes
    python tools/playbook.py --show <selector>     # one bullet or section, as orient.py prints it
    python tools/playbook.py --match <term>...     # bullets ranked against paths / crates / words
    python tools/playbook.py --manifest <term>...  # the same, as a paste-ready `playbook = [...]`
    python tools/playbook.py --goal                # --manifest driven by loop-goal.toml's modules
    python tools/playbook.py --check               # stale paths, colliding selectors, sizes (CI)
    python tools/playbook.py --dupes               # bullets that already say what another says

A term is a path (`crates/nvs-ir/src/lower/expr.rs`), a crate (`nvs-ir`), a tool (`peek.py`) or a
plain word. A path is expanded to the things a bullet would actually spell -- the posix path, the
file name, the stem, the crate in both `nvs-ir` and `nvs_ir` spellings -- so naming the handoff's
own file set is enough.

**This script never writes to the playbook.** Appending a bullet has one home already, and it is
`session.py`'s `## playbook: <heading>` section, which keeps the whole session tail at one call.
A second way to add one would be a second thing to keep in agreement.

`--check` and `--dupes` are the two pruning signals an append-mostly file can have. A bullet naming
a path that is no longer in the tree is describing a trap someone already closed; a bullet sharing
most of its three-word runs with another is a trap that was written down twice. **Five separate
sessions wrote the `wsl.exe` path-mangling bullet, one each, in five wordings**, and every copy was
charged to every session afterwards -- `--check` could see two of them, because their lead-ins
happened to collide as selectors, and was blind to the other three.

Both report; neither deletes, and neither exits non-zero over a size (docs/agent/doc-style.md
§ *Length targets*) or over a stale-looking path. Two bullets about one file are often two
different traps, and a path a bullet quotes may be gone precisely because the trap was closed --
only a reader can tell either way.

Once a reader has told, `DELIBERATE_STALE` below records it, keyed by the exact `(selector, path)`
pair. Those bullets still print, under their own heading, but out of the list `loop-supervisor.py`
reads -- because a bullet whose whole subject is a path that is gone keeps that signal raised
forever, and a signal that cannot clear schedules an optimization pass whether or not anything
drifted.

**One finding does gate, and `--check` exits 1 on it: a selector that does not resolve to exactly
one bullet.** That is not a judgement call. `orient.py` fetches a trap by selector and a goal's
`[context] playbook` names bullets that way, so a lead-in two bullets share, or one no selector
reaches at all, is a trap the loop silently cannot deliver -- the session never learns it existed.
CI's `docs` job runs this for that finding alone.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import orient as orientmod  # noqa: E402  -- bullet parsing lives there and is not reimplemented

ROOT = Path(__file__).resolve().parent.parent
PLAYBOOK = ROOT / "docs" / "agent" / "playbook.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"
HANDOFF = ROOT / "docs" / "agent" / "handoff.md"

#: A source path written into handoff prose, e.g. `crates/nvs-ir/src/lower/expr.rs:1876`.
HANDOFF_PATH = re.compile(r"\b((?:crates|tools|tests|benches|examples|fuzz)/[\w./-]+\.\w+)")

#: Directories a bullet names when it names a *tracked* path. Anything matching one of these is
#: checked against the tree by `--check`; anything else in backticks is prose, a symbol or a
#: command, and this script does not guess about those. `.loop/` and `.agent-tmp/` are left out
#: deliberately: both are runtime scratch, so a bullet naming `.loop/running` is describing an
#: artifact that exists only while a loop runs, not a file that has gone missing.
TREE_DIRS = ("crates/", "tools/", "docs/", "tests/", "benches/", "examples/", "fuzz/", ".github/")

#: A path in prose collects punctuation and a locator. Strip both before asking the disk. The
#: locator is any of `peek.py`'s target forms -- `:120-160`, `:120+30`, `:@sym`, `:re:pattern` --
#: because a bullet quoting one of those is naming a file that IS in the tree: `--check` reported
#: `docs/agent/loop-goal.toml:re:a_named_connection_is_memoized` as a missing path for as long as
#: it only knew about `:\d+`, and an optimization pass paid to re-derive that it was not.
PATH_TRIM = re.compile(r"(:re:.*|:@[\w:.-]+|:\d+([-+]\d+)?|[.,;:)\]'\"]+)$")

#: Bullets whose missing path is the whole point of the trap -- they quote a path that is gone, or
#: that was never right, *because that is what the bullet is about*. Keyed by the exact
#: `(selector, path)` pair, so any other path in the same bullet, and this path in any other
#: bullet, still reports normally.
#:
#: This exists because `loop-supervisor.py` fires an optimization pass unless the list below says
#: `none`, and these two can never leave it: the trap they describe is the stale path. Three passes
#: in a row read them and wrote down that they were deliberate, and the fourth was scheduled on
#: their account alone -- a signal that cannot clear is a constant, and it spends a pass whether or
#: not anything drifted. They are still printed, under their own heading, so the next reader sees
#: them without the loop paying to schedule that reader.
#:
#: Add an entry only after reading the bullet and recording the finding in `.loop/optimization/`.
#: An entry naming a bullet that no longer exists is reported rather than ignored.
DELIBERATE_STALE = {
    ("Tooling > instaforceupdate=1 rewrites", "crates/nvs-ir/src/lower.rs"):
        "the `source:` header the snapshots still carry from before the split -- the stale header "
        "IS the trap",
    ("Writing a test case > a live-server", "crates/nvs-db/tests/queue.rs"):
        "the wrong home a stage 8 item named; the case belongs at crates/nvs-stdlib/tests/queue.rs",
}


def nbytes(text: str) -> int:
    return len(text.encode("utf-8"))


def read() -> str:
    return PLAYBOOK.read_text(encoding="utf-8")


def sections(text: str) -> list[str]:
    return [m.group(1) for m in re.finditer(r"^## (.+)$", text, flags=re.M)]


def all_bullets(text: str) -> list[dict]:
    """Every bullet, with the section it is in and a selector that resolves to it alone."""
    found = []
    for head in sections(text):
        for name, body in orientmod.bullets(text, head):
            found.append({"section": head, "name": name, "body": body, "bytes": nbytes(body)})
    for b in found:
        b["selector"] = f"{b['section']} > {shortest_key(b, found)}"
    return found


def shortest_key(bullet: dict, every: list[dict]) -> str:
    """The fewest words of a bullet's lead-in that name it and nothing else in its section.

    `slice_bullets` matches a selector as a *substring* of the normalized lead-in, so uniqueness
    has to be tested the same way -- a three-word key that is also inside a neighbour's lead-in
    selects both, and orient.py would print two bullets where the goal asked for one."""
    peers = [b for b in every if b["section"] == bullet["section"] and b is not bullet]
    words = orientmod.normalize(bullet["name"]).split()
    for n in range(2, len(words) + 1):
        key = " ".join(words[:n])
        if not any(key in orientmod.normalize(p["name"]) for p in peers):
            return key
    return orientmod.normalize(bullet["name"])


# ------------------------------------------------------------------------------ matching


#: Path segments that carry no information about *which* work a bullet is about, because every
#: path in the tree has them. Without this, `crates/nvs-diagnostics/src/lib.rs` expands to `src`
#: and `lib`, and every bullet that mentions any Rust file at all scores a hit -- which is how a
#: first run of `--goal` proposed 50 of 86 bullets and called it narrowing.
GENERIC = {"src", "lib", "mod", "main", "crates", "tests", "docs", "tools", "benches", "rs", "md"}


def expand(term: str) -> set[str]:
    """One query term -> every spelling a bullet might use for it."""
    t = term.strip().replace("\\", "/").lower()
    out = {t}
    if "/" in t:
        parts = [p for p in t.split("/") if p]
        out.add(parts[-1])                                   # expr.rs
        out.add(parts[-1].rsplit(".", 1)[0])                 # expr
        if len(parts) >= 2 and parts[0] == "crates":
            out.add(parts[1])                                # nvs-ir
            out.add(parts[1].replace("-", "_"))              # nvs_ir
        if len(parts) >= 2:
            out.add(parts[-2])                               # lower
    else:
        out.add(t.replace("-", "_"))
        out.add(t.replace("_", "-"))
    # A one- or two-letter fragment matches everything; so does a segment every path has.
    return {x for x in out if len(x) > 2 and x not in GENERIC}


def toml_str(s: str) -> str:
    """One selector as a TOML string that parses.

    A lead-in is prose, so a key sliced from one can hold a `"` (a bullet opening with a quoted
    literal) or a `\\` (one naming `Core\\Path`). A TOML *literal* string takes both verbatim,
    which is why it is the default here; only a key that also holds a `'` needs the escaping
    form. Emitting a bare basic string looked right and produced a manifest that would not
    parse."""
    if "'" not in s:
        return f"'{s}'"
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def score(bullet: dict, terms: list[str]) -> tuple[int, list[str]]:
    """How many distinct query terms a bullet mentions, and which ones."""
    hay = (bullet["body"] + " " + bullet["section"]).lower()
    hit = [t for t in terms if any(v in hay for v in expand(t))]
    return len(hit), hit


def goal_terms() -> list[str]:
    """The current goal's own file set, out of `[context] modules`."""
    if not GOAL_TOML.exists():
        return []
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover -- 3.11+ everywhere this runs
        return []
    ctx = (tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")).get("context") or {})
    return [str(m).rstrip("*/") for m in ctx.get("modules", [])]


def next_group_files() -> list[str]:
    """The file set `handoff.md`'s `## Next group` names, as paths.

    The manifest is goal-scoped and the handoff is session-scoped, so the two drift apart every
    time the work moves to a different crate. That drift used to be invisible because naming
    whole sections covered every file set by accident; a bullet-level manifest makes it bite."""
    if not HANDOFF.exists():
        return []
    text = HANDOFF.read_text(encoding="utf-8")
    m = re.search(r"^## Next group.*?(?=^## |\Z)", text, flags=re.M | re.S)
    if not m:
        return []
    seen: dict[str, None] = {}
    for path in HANDOFF_PATH.findall(m.group(0)):
        seen.setdefault(path, None)
    return list(seen)


def current_manifest() -> list[str]:
    if not GOAL_TOML.exists():
        return []
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover
        return []
    ctx = (tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")).get("context") or {})
    return [str(x) for x in ctx.get("playbook", [])]


# ------------------------------------------------------------------------------ commands


def report_manifest(text: str, indent: str = "") -> None:
    """What `[context] playbook` currently costs, and whether every selector still resolves."""
    named = current_manifest()
    if not named:
        print(f"{indent}loop-goal.toml names no `[context] playbook`, so no trap is printed.")
        return
    cost, sections_named, dead = 0, 0, []
    for sel in named:
        hits, complaint = orientmod.slice_bullets(text, sel)
        if complaint:
            dead.append(sel)
            continue
        if ">" not in sel:
            sections_named += 1
        cost += sum(nbytes(h) for h in hits)
    whole = nbytes(text)
    print(f"{indent}[context] playbook names {len(named)} selector(s) "
          f"({sections_named} whole section(s)), costing {cost} B a session "
          f"-- {cost / whole * 100:.0f}% of the file.")
    if sections_named:
        print(f"{indent}A whole section grows every time a trap is written down. "
              "`--goal` proposes a bullet-level list.")
    for sel in dead:
        print(f"{indent}!! {sel!r} matches nothing -- orient.py will warn on it every session")


def run_gap(text: str, every: list[dict], floor: int) -> int:
    """Bullets the handoff's own next group implies that the manifest does not print.

    This is the check a narrowed manifest cannot do without: ranking against `[context] modules`
    answers "what does this GOAL touch", and the work in flight may have moved on. Measured the
    first time this ran, the manifest missed twelve bullets the next group implied -- including
    two on the exact `nvs-ir`/`nvs-codegen` path the handoff named -- because `modules` still
    listed a closed stage's stdlib file set."""
    files = next_group_files()
    if not files:
        print("playbook.py: handoff.md has no `## Next group` naming a path, so there is nothing "
              "to compare the manifest against.")
        return 0
    have = set(current_manifest())
    floor = max(1, min(floor, len(files)))
    print(f"handoff.md `## Next group` names {len(files)} file(s):")
    for f in files:
        print(f"  {f}")

    missing = []
    for b in every:
        n, hit = score(b, files)
        if n >= floor and b["selector"] not in have:
            missing.append((n, b))
    missing.sort(key=lambda x: (-x[0], x[1]["bytes"]))

    print(f"\nBullets those files imply ({floor}+ terms) that `[context] playbook` does NOT print:")
    if not missing:
        print("  none -- the manifest covers the work in flight")
        return 0
    for n, b in missing:
        print(f"  {n} term(s)  {b['bytes']:>5} B  {toml_str(b['selector'])},")
    print(f"\n  {len(missing)} bullet(s), {sum(b['bytes'] for _n, b in missing)} B. Add them to")
    print("  `[context] playbook`, or -- better -- fix `[context] modules` if it no longer")
    print("  describes the work, and re-run `--goal`. A manifest narrower than the work is")
    print("  the one way this tool can cost a session quality rather than save it tokens.")
    return 0


def run_index(text: str, every: list[dict]) -> int:
    total = nbytes(text)
    print(f"docs/agent/playbook.md: {total} bytes, {len(every)} bullets in "
          f"{len(sections(text))} sections")
    for head in sections(text):
        mine = [b for b in every if b["section"] == head]
        size = sum(b["bytes"] for b in mine)
        print(f"\n  ## {head}   {size} bytes, {len(mine)} bullets")
        for b in mine:
            print(f"    {b['bytes']:>5} B  {b['selector']}")

    print()
    report_manifest(text)
    print("\nA selector above is what `[context] playbook` takes verbatim. Naming the section")
    print("instead takes every bullet in it, including the ones written after this goal began.")
    return 0


def run_show(text: str, selector: str) -> int:
    hits, complaint = orientmod.slice_bullets(text, selector)
    if complaint:
        print(f"playbook.py: {complaint}")
        return 1
    for body in hits:
        print(body)
        print()
    print(f"-- {len(hits)} hit(s), {sum(nbytes(h) for h in hits)} bytes")
    if len(hits) > 1:
        print("   More than one: `--manifest` emits selectors that resolve to exactly one.")
    return 0


def run_match(text: str, every: list[dict], terms: list[str], as_manifest: bool,
              floor: int) -> int:
    if not terms:
        print("playbook.py: --match/--manifest needs at least one term, or use --goal")
        return 2
    # A floor above the number of terms can never be met, so `--match one-file` under the default
    # of 2 reported "no bullet mentions it" about a file three bullets name.
    floor = max(1, min(floor, len(terms)))
    scored, weak = [], []
    for b in every:
        n, hit = score(b, terms)
        if n >= floor:
            scored.append((n, b, hit))
        elif n:
            weak.append((n, b, hit))
    scored.sort(key=lambda x: (-x[0], x[1]["bytes"]))
    weak.sort(key=lambda x: (-x[0], x[1]["bytes"]))

    if not scored:
        print(f"playbook.py: no bullet mentions {floor}+ of {terms}. That is a real answer -- "
              "either this goal's file set has no trap written down yet, or --min is too high "
              f"({len(weak)} bullet(s) matched fewer).")
        return 0

    # Never a silent cut: what --min dropped is named, because a threshold that hides its own
    # tail reads as "there was nothing else" and the author cannot tell the two apart.
    def tail() -> None:
        if weak:
            print(f"\n# {len(weak)} further bullet(s) matched fewer than {floor} terms and are "
                  f"NOT listed ({sum(b['bytes'] for _n, b, _h in weak)} B). `--min 1` shows them.")

    if not as_manifest:
        print(f"{len(scored)} of {len(every)} bullets match {floor}+ of {terms}:\n")
        for n, b, hit in scored:
            print(f"  {n} term(s)  {b['bytes']:>5} B  {b['selector']}")
            print(f"             {' '.join(hit)}")
        print(f"\n  {sum(b['bytes'] for _n, b, _h in scored)} bytes in total. "
              "`--manifest` prints the same list as TOML.")
        tail()
        return 0

    total = sum(b["bytes"] for _n, b, _h in scored)
    whole = nbytes(text)
    print("# Bullets whose text names this goal's own file set, most specific first.")
    print(f"# {len(scored)} of {len(every)} bullets, {total} bytes -- against {whole} for the "
          f"whole file. Threshold: {floor}+ matching terms.")
    print("playbook = [")
    for n, b, _hit in scored:
        print(f"  {toml_str(b['selector'])},".ljust(66) + f"# {n} term(s), {b['bytes']} B")
    print("]")
    tail()
    print("\n# Paste over `[context] playbook`, then READ it and add to it. This ranks by what a")
    print("# bullet MENTIONS, which is a proxy for what a session needs and not the same thing.")
    print("# In particular the PROCESS traps -- how a session ends, how it reads, what a commit")
    print("# message may not carry, what to do when this pack is short -- name no crate and no")
    print("# file, so they score zero here and will never be proposed. They apply to every")
    print("# session regardless of file set; keep them in the list by hand.")
    print("# orient.py warns loudly if a selector stops matching, so a bullet that is later")
    print("# reworded fails loudly rather than silently.")
    return 0


def run_dupes(every: list[dict], floor: float) -> int:
    """Bullets that already say what another bullet says.

    An append-mostly file cannot notice that it already knows something. Five separate sessions
    wrote the `wsl.exe` path-mangling trap, one each, in five different wordings -- and every copy
    was charged to every session afterwards, because the playbook is the single largest thing
    `orient.py` ships. `--check` caught that pair only because two of the lead-ins happened to
    collide as selectors; three of the five it could not see at all.

    Similarity is over the *shingles* of each bullet -- its distinct three-word runs -- rather
    than over its characters, because the whole failure mode here is the same trap in different
    prose. A shared code span or path name is what the overlap actually rests on, so `--dupes`
    reports and never prunes: two bullets about the same file are often two different traps, and
    only a reader can tell.

    AGENTS.md is explicit that this file is append-mostly and must not be reworded to say the
    same thing differently. This is how you find the places where it already was."""
    def shingles(body):
        words = re.findall(r"[a-z0-9_./-]+", body.lower())
        return {" ".join(words[i:i + 3]) for i in range(max(0, len(words) - 2))}

    grams = [(b, shingles(b["body"])) for b in every]
    pairs = []
    for i, (a, ga) in enumerate(grams):
        for b, gb in grams[i + 1:]:
            if not ga or not gb:
                continue
            overlap = len(ga & gb) / min(len(ga), len(gb))
            if overlap >= floor:
                pairs.append((overlap, a, b))
    pairs.sort(key=lambda p: -p[0])

    print(f"== BULLETS THAT MAY ALREADY BE SAID ELSEWHERE  (>= {floor:.0%} of the shorter one's "
          "three-word runs)")
    if not pairs:
        print(f"  none at this threshold across {len(every)} bullets. "
              "`--dupes 0.15` lowers it -- `--min` is the `--match` term floor and does "
              "nothing here.")
        return 0
    for overlap, a, b in pairs:
        print(f"\n  {overlap:.0%}  and {a['bytes'] + b['bytes']:,} B between them")
        print(f"      {a['selector']}")
        print(f"      {b['selector']}")
    print(f"\n  {len(pairs)} pair(s). This reports and never prunes -- two bullets about one file")
    print("  are often two different traps, and only a reader can tell. When they are the same")
    print("  trap, merge them into the better-written one and say so in the commit.")
    return 0


def run_check(text: str, every: list[dict]) -> int:
    print(f"docs/agent/playbook.md: {nbytes(text)} bytes, {len(every)} bullets\n")

    print("== PATHS A BULLET NAMES THAT ARE NOT IN THE TREE")
    stale = 0
    splits = 0
    deliberate: list[tuple[str, str, str]] = []
    for b in every:
        gone = []
        for raw in re.findall(r"`([^`]+)`", b["body"]):
            cand = PATH_TRIM.sub("", raw.strip().split()[0] if raw.strip() else "")
            # `*` and `<` are the spellings of a path a bullet never claimed exists; an elision
            # -- `tests/conformance/io/…` -- is a third, and the bullet that spells one is often
            # the trap that the layout it names is the one the tree did NOT take.
            if not cand.startswith(TREE_DIRS) or any(m in cand for m in ("*", "<", "…", "...")):
                continue
            if not (ROOT / cand).exists():
                why = DELIBERATE_STALE.get((b["selector"], cand))
                if why is not None:
                    deliberate.append((b["selector"], cand, why))
                else:
                    gone.append(cand)
        if gone:
            stale += 1
            print(f"  {b['selector']}")
            for g in sorted(set(gone)):
                # `foo.rs` gone while `foo/` stands is a file that was SPLIT, not deleted -- the
                # module is still there and the trap is usually still live. Say so rather than
                # making every pass re-derive it; this annotates, it does not filter.
                asdir = Path(g).with_suffix("")
                if str(asdir) != g and (ROOT / asdir).is_dir():
                    splits += 1
                    print(f"      {g}  -- split into {asdir.as_posix()}/, so the module still stands")
                else:
                    print(f"      {g}")
    if not stale:
        # `loop-supervisor.py` reads this sentence to decide whether the stale-path signal fired,
        # so the first clause of it is a contract. What follows it is not.
        tail = f", or is quoted on purpose ({len(deliberate)} below)" if deliberate else ""
        print(f"  none -- every path any bullet names still exists{tail}")
    else:
        print(f"\n  {stale} bullet(s). A trap describing a file that is gone is usually a trap")
        print("  someone closed. Read it before deleting it; this reports, it never prunes.")
        if splits:
            print(f"  {splits} of the paths above are marked `split into` -- those are the weakest")
            print("  signal of the lot, because the code moved rather than went away.")

    if deliberate:
        print("\n== PATHS A BULLET QUOTES ON PURPOSE  (already read; not a signal)")
        for selector, path, why in deliberate:
            print(f"  {selector}")
            print(f"      {path}  -- {why}")
        print(f"\n  {len(deliberate)} bullet(s), held in `DELIBERATE_STALE` in this script. They are")
        print("  kept out of the list above so the supervisor's signal can reach `none`; the trap")
        print("  each one describes IS its missing path, so no pass can ever prune them.")

    unseen = set(DELIBERATE_STALE) - {(s, p) for s, p, _ in deliberate}
    if unseen:
        print("\n== DELIBERATE_STALE ENTRIES THAT NO LONGER APPLY")
        for selector, path in sorted(unseen):
            print(f"  {selector}  ->  {path}")
        print(f"\n  {len(unseen)} entry(s) matched no bullet: either the bullet was reworded or")
        print("  deleted, or the path is back in the tree. Drop the entry from this script.")

    print("\n== SELECTORS THAT DO NOT RESOLVE TO EXACTLY ONE BULLET")
    bad = 0
    for b in every:
        hits, complaint = orientmod.slice_bullets(text, b["selector"])
        if complaint or len(hits) != 1:
            bad += 1
            print(f"  {b['selector']}  -> {complaint or f'{len(hits)} hits'}")
    if not bad:
        print(f"  none -- all {len(every)} bullets are individually selectable")

    print("\n== WHAT EACH SECTION COSTS A SESSION THAT NAMES IT WHOLE")
    for head in sections(text):
        mine = [b for b in every if b["section"] == head]
        size = sum(b["bytes"] for b in mine)
        print(f"  {size:>6} B  {len(mine):>3} bullets   ## {head}")
    print()
    report_manifest(text, indent="  ")
    print("\n  Nothing here refuses over a size. This is a number to weigh when a goal is")
    print("  written, which is the only moment it can be acted on cheaply.")

    # A stale path and a size are judgement calls and stay reports. An unreachable selector is
    # not: `orient.py` fetches a bullet by exactly this string, so a goal naming one that
    # resolves to none or to two gets a trap it cannot be handed, and finds out never.
    if bad:
        print(f"\n  !! {bad} selector(s) above do not resolve to exactly one bullet. That is what")
        print("  this exits non-zero on: `orient.py` fetches a trap by its selector, so a goal's")
        print("  `[context] playbook` naming one of these is a trap the loop cannot deliver.")
        print("  Reword the colliding lead-in -- the bullet's text, not this tool, is the fix.")
        return 1
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--show", metavar="SELECTOR")
    ap.add_argument("--match", nargs="*", metavar="TERM")
    ap.add_argument("--manifest", nargs="*", metavar="TERM")
    ap.add_argument("--goal", action="store_true",
                    help="--manifest over loop-goal.toml's [context] modules")
    ap.add_argument("--min", type=int, default=2, metavar="N", dest="floor",
                    help="how many query terms a bullet must mention to be listed (default 2)")
    ap.add_argument("--gap", action="store_true",
                    help="bullets the handoff's next group implies that the manifest omits")
    ap.add_argument("--check", action="store_true")
    # 0.22, not the 0.30 this shipped with: the `Core\\Math::gcd` twin trap was written down twice
    # at 23% overlap and the default was blind to it, while the whole 22-30% band held that one
    # pair and no false positive. This reports and never prunes, so the cost of looking lower is a
    # reader's minute.
    ap.add_argument("--dupes", nargs="?", type=float, const=0.22, metavar="RATIO",
                    help="bullets that may already say what another bullet says (default 0.22)")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if not PLAYBOOK.exists():
        print(f"playbook.py: no {PLAYBOOK.relative_to(ROOT).as_posix()}")
        return 2

    text = read()
    every = all_bullets(text)

    if opts.show:
        return run_show(text, opts.show)
    if opts.gap:
        return run_gap(text, every, opts.floor)
    if opts.check:
        return run_check(text, every)
    if opts.dupes is not None:
        return run_dupes(every, opts.dupes)
    if opts.goal:
        terms = goal_terms()
        if not terms:
            print("playbook.py: loop-goal.toml names no `[context] modules`, so there is no "
                  "file set to match against. Pass terms to --manifest instead.")
            return 2
        return run_match(text, every, terms, as_manifest=True, floor=opts.floor)
    if opts.manifest is not None:
        return run_match(text, every, opts.manifest, as_manifest=True, floor=opts.floor)
    if opts.match is not None:
        return run_match(text, every, opts.match, as_manifest=False, floor=opts.floor)
    return run_index(text, every)


if __name__ == "__main__":
    sys.exit(main())
