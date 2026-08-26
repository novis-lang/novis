#!/usr/bin/env python3
"""Pick the playbook bullets a goal actually needs, and report the ones that have gone stale.

`docs/agent/playbook.md` is 47 KB in 86 bullets across six sections, and it is append-mostly by
decision -- every trap a session writes down is charged to every session after it. It is the
single largest thing `orient.py` ships: measured on the current goal, **42 KB of a 78 KB pack,
17k of its 31k tokens**, because `[context] playbook` names four whole sections and a session
reads perhaps three of their bullets.

The fix is not to split the file and not to trim it. `orient.py` already slices it, and a
`[context] playbook` entry may already name **one bullet** -- `"Tooling > A whole ADR"`. What
was missing is any cheap way to decide *which* bullets, out of 86, a given file set implies. That
is this script.

    python tools/playbook.py                       # every section and bullet, one line, with sizes
    python tools/playbook.py --show <selector>     # one bullet or section, as orient.py prints it
    python tools/playbook.py --match <term>...     # bullets ranked against paths / crates / words
    python tools/playbook.py --manifest <term>...  # the same, as a paste-ready `playbook = [...]`
    python tools/playbook.py --goal                # --manifest driven by loop-goal.toml's modules
    python tools/playbook.py --check               # stale paths, colliding selectors, sizes

A term is a path (`crates/mwl-ir/src/lower/expr.rs`), a crate (`mwl-ir`), a tool (`peek.py`) or a
plain word. A path is expanded to the things a bullet would actually spell -- the posix path, the
file name, the stem, the crate in both `mwl-ir` and `mwl_ir` spellings -- so naming the handoff's
own file set is enough.

**This script never writes to the playbook.** Appending a bullet has one home already, and it is
`session.py`'s `## playbook: <heading>` section, which keeps the whole session tail at one call.
A second way to add one would be a second thing to keep in agreement.

`--check` is the only pruning signal an append-mostly file can have: a bullet naming a path that
is no longer in the tree is describing a trap someone already closed. It reports; it does not
delete, and it never exits non-zero over a size (docs/agent/doc-style.md § *Length targets*).
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

#: A source path written into handoff prose, e.g. `crates/mwl-ir/src/lower/expr.rs:1876`.
HANDOFF_PATH = re.compile(r"\b((?:crates|tools|tests|benches|examples|fuzz)/[\w./-]+\.\w+)")

#: Directories a bullet names when it names a *tracked* path. Anything matching one of these is
#: checked against the tree by `--check`; anything else in backticks is prose, a symbol or a
#: command, and this script does not guess about those. `.loop/` and `.agent-tmp/` are left out
#: deliberately: both are runtime scratch, so a bullet naming `.loop/running` is describing an
#: artifact that exists only while a loop runs, not a file that has gone missing.
TREE_DIRS = ("crates/", "tools/", "docs/", "tests/", "benches/", "examples/", "fuzz/", ".github/")

#: A path in prose collects punctuation and a line anchor. Strip both before asking the disk.
PATH_TRIM = re.compile(r"(:\d+(-\d+)?|[.,;:)\]'\"]+)$")


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
#: path in the tree has them. Without this, `crates/mwl-diagnostics/src/lib.rs` expands to `src`
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
            out.add(parts[1])                                # mwl-ir
            out.add(parts[1].replace("-", "_"))              # mwl_ir
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
    two on the exact `mwl-ir`/`mwl-codegen` path the handoff named -- because `modules` still
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


def run_check(text: str, every: list[dict]) -> int:
    print(f"docs/agent/playbook.md: {nbytes(text)} bytes, {len(every)} bullets\n")

    print("== PATHS A BULLET NAMES THAT ARE NOT IN THE TREE")
    stale = 0
    for b in every:
        gone = []
        for raw in re.findall(r"`([^`]+)`", b["body"]):
            cand = PATH_TRIM.sub("", raw.strip().split()[0] if raw.strip() else "")
            if not cand.startswith(TREE_DIRS) or "*" in cand or "<" in cand:
                continue
            if not (ROOT / cand).exists():
                gone.append(cand)
        if gone:
            stale += 1
            print(f"  {b['selector']}")
            for g in sorted(set(gone)):
                print(f"      {g}")
    if not stale:
        print("  none -- every path any bullet names still exists")
    else:
        print(f"\n  {stale} bullet(s). A trap describing a file that is gone is usually a trap")
        print("  someone closed. Read it before deleting it; this reports, it never prunes.")

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
