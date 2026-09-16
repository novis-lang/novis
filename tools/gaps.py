#!/usr/bin/env python3
"""What Part I's corpus does not ask yet, as a worklist a session can take an item off.

Stage 4's frontier is behavioural *depth* per member, not coverage: every registered member has a
case already, so a session's expensive question is no longer "which member" but "which claim". That
question was being answered by hand, every session, out of the source -- `loop-stats.py --attribute`
put 36% of a session's context in `source` and another 16% in `discovery`, over half of it spent
arriving at a claim before a line of the case is written.

Both halves of that question have a machine-readable answer already in the tree, and this prints
them:

*   **A differential gap.** `docs/spec/01-core-library.md`'s **Replaces** column names, for every
    member, the PHP built-ins it subsumes -- which is exactly the twin a `--ORACLE--` case needs. A
    member with a named twin and no case in `tests/differential/` is a case whose expected output
    nobody has to derive, because PHP computes it.

*   **A thin class.** Every registered member has a case, so the ranking that picks a next group
    is *cases per member*, and both halves of it are on the tree: the registry knows what a class
    declares and the suite knows what names it. A session was deriving this by hand every time --
    `ls tests/conformance/core/ | grep -i <family>` beside `grep -n 'name: "' <family>.rs`, about
    ten times over one 19-session run, half of them in the tail.

*   **An unasserted error path.** Every `Fault::` site in `nvs-stdlib` is a boundary the
    implementation is written around. A case that pins one catches it and echoes `$e->message`, so
    the message text lands in the case's `--EXPECT--` block -- and a message that appears in no case
    is a boundary nothing asks about. That is the *edges* shape, ready-made.

Neither list is a plan. An entry is a candidate: some `Fault::fatal` sites are internal invariants
no program can reach, and a member whose PHP twin diverges by decision wants `--ORACLE-DIVERGES--`
and a reason rather than a twin. Judging that is the session's job, and it is the part worth its
context. Finding the candidate is not.

    python tools/gaps.py                          all three lists, counts and a sample
    python tools/gaps.py --coverage               cases per member, per class, thinnest first
    python tools/gaps.py --differential           every member with a PHP twin and no oracle case
    python tools/gaps.py --errors                 every Fault site no case asserts
    python tools/gaps.py --member 'Core\\Arr::chunk'   what the corpus already asks of one member
    python tools/gaps.py --limit 0                no truncation
    python tools/gaps.py --json                   one JSON object instead

Counts move as the corpus grows, so nothing here is copied into a document. Run it.
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "docs" / "spec" / "01-core-library.md"
STDLIB = ROOT / "crates" / "nvs-stdlib" / "src"
CONFORMANCE = ROOT / "tests" / "conformance"
DIFFERENTIAL = ROOT / "tests" / "differential"

BS = chr(92)  # a literal backslash, spelled so no layer of quoting can eat it


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def rel(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def cases(root: Path) -> list[Path]:
    return sorted(root.rglob("*.nvst")) if root.is_dir() else []


def corpus(root: Path) -> str:
    return "\n".join(read(p) for p in cases(root))


# ------------------------------------------------------------------------ the registry

#: `CoreMethod { name: "chunk", … symbol: "nvs_core_arr_chunk" }`, paired by position: each
#: literal carries exactly one of each, and `symbol` always follows `name` inside it.
#: A method names itself inline or through a `&str` const (`cap.rs` writes `name: HAS`), which
#: `class_consts` resolves the same way it resolves a class's.
METHOD_RE = re.compile(r'CoreMethod\s*\{\s*name:\s*(?:"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))')
SYMBOL_RE = re.compile(r'symbol:\s*"([^"]+)"')
#: A `CoreClass` names itself either inline (`name: r"Core\Arr"`) or through a file-level const
#: (`name: NAME`), and the second spelling is the majority -- matching only the first found 7 of
#: the 20 classes on the tree and silently shortened every list in this file to those 7.
CLASS_RE = re.compile(r'CoreClass\s*\{\s*name:\s*(?:r"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))')

#: `pub(crate) const NAME: &str = r"Core\Csv";` -- what the second spelling above resolves against,
#: in either of the two ways a file spells it: the raw string, or the plain one with its backslash
#: doubled (`"Core\\Signal"`). A const this does not match (`cli.rs` forwards one out of
#: `nvs_runtime`) leaves its class unnamed, which suppresses that class's members rather than
#: handing them to the class above it.
NAME_CONST_RE = re.compile(r'const\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&str\s*=\s*'
                           r'(?:r"([^"]+)"|"((?:[^"\\]|\\.)*)")')


def name_consts(text: str) -> dict[str, str]:
    """`{const name: the class name it holds}` for one file, both spellings unescaped."""
    return {m.group(1): m.group(2) if m.group(2) is not None else m.group(3).replace("\\\\", "\\")
            for m in NAME_CONST_RE.finditer(text)}


#: `pub const NAME: &str = nvs_runtime::CARRIER_CLI_TEXT;` -- a const that forwards another rather
#: than spelling a string, resolved against the tree's map by the last path segment.
ALIAS_CONST_RE = re.compile(r'const\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&str\s*=\s*'
                            r'(?:[A-Za-z_][A-Za-z0-9_]*::)+([A-Za-z_][A-Za-z0-9_]*)\s*;')

#: Where the tree-wide map is read from: the stdlib, and the runtime crate a stdlib file forwards
#: a carrier's name out of.
CONST_ROOTS = (STDLIB, ROOT / "crates" / "nvs-runtime" / "src")

_tree_consts: dict[str, str] | None = None


def class_consts(path: Path, text: str) -> dict[str, str]:
    """The name consts a `CoreClass` or `CoreMethod` literal in `text` may spell its `name:` with.

    In order of precedence: the file's own consts; the ones it forwards from another crate by
    alias; its parent module's, since `db/registry.rs` opens `Core\\Db` as `name: NAME` under a
    `use super::*` and reading only its own file left every `Core\\Db` class unnamed; and behind
    those the tree's *unambiguous* ones. A name declared in more than one file with different
    values -- `NAME`, which nearly every file declares -- is left out of the tree's map, because a
    literal spelling it for a const imported from elsewhere would otherwise resolve to whichever
    file sorts last and credit its members to that class.
    """
    global _tree_consts
    if _tree_consts is None:
        seen: dict[str, set[str]] = {}
        for root in CONST_ROOTS:
            for p in sorted(root.rglob("*.rs")):
                for name, value in name_consts(read(p)).items():
                    seen.setdefault(name, set()).add(value)
        _tree_consts = {name: next(iter(v)) for name, v in seen.items() if len(v) == 1}
    sibling = path.parent / "mod.rs"
    parent = name_consts(read(sibling)) if path.name != "mod.rs" and sibling.is_file() else {}
    aliases = {m.group(1): _tree_consts[m.group(2)]
               for m in ALIAS_CONST_RE.finditer(text) if m.group(2) in _tree_consts}
    return {**_tree_consts, **parent, **aliases, **name_consts(text)}


def registry() -> dict[tuple[str, str], tuple[Path, int, str]]:
    """(class, member) -> (file, line, symbol), read out of the `CoreClass` literals themselves.

    A member is attributed to the class whose literal most recently opened above it. That is
    positional rather than brace-matched on purpose: the alternative is a Rust parser, and the
    files this reads put one class's methods in one run, so position answers it exactly. A file
    holding several classes (`time.rs` holds seven) is the case this is written for.

    A class whose name does not resolve still opens a run, under the empty name -- so its methods
    are skipped rather than credited to whichever class happens to sit above it.
    """
    found: dict[tuple[str, str], tuple[Path, int, str]] = {}
    for path in sorted(STDLIB.rglob("*.rs")):
        text = read(path)
        consts = class_consts(path, text)
        starts = [(m.start(), m.group(1) or consts.get(m.group(2), ""))
                  for m in CLASS_RE.finditer(text)]
        if not starts:
            continue
        for m in METHOD_RE.finditer(text):
            owner = ""
            for pos, name in starts:
                if pos < m.start():
                    owner = name
                else:
                    break
            member = m.group(1) or consts.get(m.group(2), "")
            if not owner or not member:
                continue
            sym = SYMBOL_RE.search(text, m.end(), m.end() + 600)
            line = text.count("\n", 0, m.start()) + 1
            found[(owner, member)] = (path, line, sym.group(1) if sym else "")
    return found


#: `return_ty: CoreTy::Instance(DATETIME_NAME)` -- the member answers an instance of that class,
#: which is how a case reaches a class it never spells. Resolved through the same file-level name
#: consts `CLASS_RE`'s second spelling uses.
#:
#: `CoreTy::InstanceAt(ROWS_NAME, &[...])` is the same claim about a generic class at written
#: arguments, and only the class it names matters here -- what `Core\Db\Connection::query` answers
#: is a `Core\Db\Rows` whether or not the row spells its `T`. The Rust half of this attribution is
#: `crates/nvs-stdlib/tests/corpus/mod.rs`'s `Attribution::new`, and the two agree by hand.
RETURNS_RE = re.compile(
    r'return_ty:\s*CoreTy::Instance(?:At)?\(\s*(?:r"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))\s*[,)]')


def producers() -> dict[tuple[str, str], str]:
    """(class, member) -> the `Core` class an instance of which that member answers.

    `Core\\Time::fromIso` answers an `Instant` and `Instant::in` answers a `DateTime`, so a case
    written `var $d = Core\\Time::fromIso(...)->in($z);` exercises three classes and spells one.
    That is not a corner of the library: half the registry is instance-shaped and reached from a
    factory on some *other* class, which is exactly the shape `coverage`'s attribution used to
    miss entirely -- it ranked `Core\\Time\\DateTime` at one case over 17 members with two whole
    cases about it on disk, and named three members "no case calls" that one of them calls.
    """
    found: dict[tuple[str, str], str] = {}
    for path in sorted(STDLIB.rglob("*.rs")):
        text = read(path)
        consts = class_consts(path, text)
        starts = [(m.start(), m.group(1) or consts.get(m.group(2), ""))
                  for m in CLASS_RE.finditer(text)]
        if not starts:
            continue
        # A literal's own extent, rather than a fixed window: `Core\Time::at` writes its options
        # bag out inline and its `return_ty` sits some 900 characters below its name.
        methods = list(METHOD_RE.finditer(text))
        for index, m in enumerate(methods):
            owner = ""
            for pos, name in starts:
                if pos < m.start():
                    owner = name
                else:
                    break
            stop = methods[index + 1].start() if index + 1 < len(methods) else len(text)
            made = RETURNS_RE.search(text, m.end(), stop)
            if not owner or not made:
                continue
            name = made.group(1) or consts.get(made.group(2), "")
            member = m.group(1) or consts.get(m.group(2), "")
            if name and member:
                found[(owner, member)] = name
    return found


def symbol_lines() -> dict[str, tuple[Path, int]]:
    """`nvs_core_arr_range` -> where that function is declared, for a file:line anchor on the
    implementation rather than on the signature table."""
    out: dict[str, tuple[Path, int]] = {}
    for path in sorted(STDLIB.rglob("*.rs")):
        text = read(path)
        for m in re.finditer(r"\bfn\s+(nvs_core_[a-z0-9_]+)\s*\(", text):
            out.setdefault(m.group(1), (path, text.count("\n", 0, m.start()) + 1))
    return out


# ----------------------------------------------------------------------------- the spec

#: A backticked token in the **Replaces** column that is a PHP function name rather than an
#: expression: `strpos` yes, `$s == ""` no.
PHP_NAME_RE = re.compile(r"`([a-z_][a-z0-9_]*)`")
#: `## 7. `Core\Encoding` and `Core\Bytes`` -- a heading may name more than one class, which is
#: why the registry decides which of them owns a row.
SPEC_CLASS_RE = re.compile(r"`(Core(?:" + re.escape(BS) + r"[A-Za-z]+)*)`")


def spec_rows() -> list[tuple[list[str], str, str, str]]:
    """(classes named by the enclosing heading, member, signature, replaces), one per member row."""
    rows = []
    heading: list[str] = []
    for line in read(SPEC).splitlines():
        if line.startswith("## "):
            heading = SPEC_CLASS_RE.findall(line)
        if not line.startswith("|") or line.count("|") < 4:
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 3 or not cells[0].startswith("`") or "(" not in cells[1]:
            continue
        rows.append((heading, cells[0].strip("`"), cells[1], cells[2]))
    return rows


def php_twins(cell: str) -> list[str]:
    """The PHP built-ins a **Replaces** cell names, or nothing when it names none."""
    if "nothing" in cell.lower():
        return []
    return PHP_NAME_RE.findall(cell)


# -------------------------------------------------------------------------- the two gaps


def called_members(text: str) -> set[str]:
    """Every `Core\\X::member(` and `->member(` the given corpus text calls."""
    out = set()
    for m in re.finditer(r"(Core(?:" + re.escape(BS) + r"[A-Za-z]+)+)::([A-Za-z][A-Za-z0-9]*)", text):
        out.add(m.group(1) + "::" + m.group(2))
    for m in re.finditer(r"->([a-z][A-Za-z0-9]*)\s*\(", text):
        out.add("->" + m.group(1))
    return out


def differential_gaps() -> list[dict]:
    """Members whose spec entry names a PHP built-in and whose name no differential case calls."""
    reg = registry()
    syms = symbol_lines()
    called = called_members(corpus(DIFFERENTIAL))
    out = []
    for classes, member, _sig, replaces in spec_rows():
        twins = php_twins(replaces)
        if not twins:
            continue
        owner = next((c for c in classes if (c, member) in reg), "")
        if not owner:
            continue  # named in the spec, not registered: the registry ratchet owns that gap
        if f"{owner}::{member}" in called or f"->{member}" in called:
            continue
        path, line, sym = reg[(owner, member)]
        impl = syms.get(sym)
        out.append({
            "member": f"{owner}::{member}",
            "php": twins,
            "anchor": f"{rel(impl[0])}:{impl[1]}" if impl else f"{rel(path)}:{line}",
        })
    return out


FAULT_RE = re.compile(r"Fault::(thrown_as|thrown|fatal)\s*\(")


def error_gaps() -> list[dict]:
    """`Fault::` sites in `nvs-stdlib` whose message stem appears in no case of either suite."""
    seen = corpus(CONFORMANCE) + "\n" + corpus(DIFFERENTIAL)
    out = []
    for path in sorted(STDLIB.rglob("*.rs")):
        text = read(path)
        owners = [(m.start(), m.group(1)) for m in re.finditer(r"\bfn\s+(nvs_core_[a-z0-9_]+)\s*\(", text)]
        for m in FAULT_RE.finditer(text):
            # The literal runs to the first *unescaped* quote: a message that quotes its own
            # operand back writes `\"` inside itself, and a class that stopped at it would read
            # every one of `encoding.rs`'s decoders as a stem of `Core\Encoding::fromBase64(): \`
            # -- a stem no case can ever contain, so the site stays on this list however well it
            # is asserted.
            esc_bs = re.escape(BS)
            quoted = re.search(
                rf'"((?:[^"{esc_bs}]|{esc_bs}[\s\S]){{10,400}})"', text[m.end():m.end() + 700]
            )
            if not quoted:
                continue
            # Two rewrites, in this order, to recover the text a case actually echoes. A trailing
            # backslash continues a Rust literal onto the next line and eats the indent that
            # follows, so a message wrapped for rustfmt would otherwise be truncated at the wrap
            # -- which is most of `bytes.rs`. Then each remaining escape is the one character it
            # spells, in one left-to-right pass so `\\"` is a backslash and then a quote rather
            # than an escaped one -- which is how every `Core\Bytes` in a message is spelled.
            message = re.sub(esc_bs + r"\n\s*", "", quoted.group(1))
            message = re.sub(rf'{esc_bs}(["{esc_bs}])', lambda esc: esc.group(1), message)
            stem = re.split(r"[{}]", message)[0].strip()
            if len(stem) < 14 or stem in seen:
                continue
            fn = ""
            for pos, name in owners:
                if pos < m.start():
                    fn = name
                else:
                    break
            out.append({
                "anchor": f"{rel(path)}:{text.count(chr(10), 0, m.start()) + 1}",
                "kind": m.group(1),
                "fn": fn,
                # Two spellings on purpose: `stem` is the literal run before the first format
                # hole, which is all that can be matched against a case's frozen output, while
                # `message` is what a reader needs -- a message opening on a hole (`pack: `{code}`
                # ...`) has a true stem too short to tell one row from the next.
                "stem": stem,
                "message": " ".join(message.split()),
            })
    # `thrown` first: it is a boundary a program can reach and a case can catch, where most of
    # the `fatal` rows are the argument type-guards the checker already refuses. Truncating a
    # list that led with those would hide every actionable row behind noise.
    order = {"thrown": 0, "thrown_as": 1, "fatal": 2}
    out.sort(key=lambda r: (order.get(r["kind"], 3), r["anchor"]))
    return out


def coverage() -> list[dict]:
    """Per `Core` class: registered members, the cases that call it, and which members none does.

    Stage 4's question is *depth* -- every registered member has a case, so the useful ranking is
    how many cases the class's **thinnest members** carry, and that class is the next group.

    DEPTH IS THE MEDIAN CASES PER MEMBER, not the class's case count divided by its member count.
    The divided form is what this printed first, and it ranks by class *size*: `Core\\Math` led it
    at 1.00 with 38 members and 38 cases, and a session took a whole group off the top of that
    ranking before finding that every one of the three claims it named was already pinned -- the
    thinnest member of that class carries three cases, and the class carries thirty-one dedicated
    files. A case names five or ten members at once, so a big class can never reach a high quotient
    however deeply each member is asked. The median asks the question a session actually has ("how
    much is a typical member of this class asked?"), `floor` is its worst member, and `thin` names
    the three worst with anchors so the group is picked from members rather than from a class.

    Sessions were answering this by
    hand: measured over one 19-session run, `ls tests/conformance/core/ | grep -i <family>` paired
    with `grep -n 'name: "' crates/nvs-stdlib/src/<family>.rs` ran about ten times, half of them in
    the tail where a call is most expensive, to arrive at a ranking the tree already holds.

    A case *belongs* to a class when it names it at all -- `Core\\ObjectMap<Tag, int> $m = new
    Core\\ObjectMap...` names no `::` and is still that class's case, and half the registry is
    instance-shaped like that. The name has to end at a boundary or `Core\\Time` would collect
    every `Core\\Time\\Duration` case as its own.

    **Or when it holds one of that class's values without ever spelling the name**, which is what
    `var $d = Core\\Time::fromIso($text)->in($zone);` does: `producers` says `fromIso` answers an
    `Instant` and `Instant::in` a `DateTime`, so a written `Owner::member(` attributes the case to
    what it builds, and then an instance call `->member(` on a class the case already holds
    attributes it to what *that* builds, to a fixed point. Without that step the time family read
    as the thinnest on the tree while carrying two dedicated cases apiece -- the ranking was
    measuring how often a case writes a type annotation, not what it exercises.

    A member is *called* by `Core\\X::member`, or by `->member(` in a case that names the class --
    which is `differential_gaps`' rule narrowed from the whole corpus to the class's own cases,
    because corpus-wide every `->get(` marks every class's `get` as covered.
    """
    reg = registry()
    syms = symbol_lines()

    def anchor_of(owner: str, member: str) -> str:
        path, line, sym = reg[(owner, member)]
        impl = syms.get(sym)
        return f"{rel(impl[0])}:{impl[1]}" if impl else f"{rel(path)}:{line}"

    per_class: dict[str, list[str]] = {}
    for owner, member in reg:
        per_class.setdefault(owner, []).append(member)

    # (class -> member -> what an instance of it answers), for the walk in `holders`.
    builds: dict[str, dict[str, str]] = {}
    for (owner, member), made in producers().items():
        builds.setdefault(owner, {})[member] = made

    # `(?![\w\\])`: `Core\Time` matches `Core\Time::now` and `Core\Time $t`, never
    # `Core\Time\Duration`, which is a different class with its own row.
    owns = {name: re.compile(re.escape(name) + r"(?![A-Za-z0-9_" + re.escape(BS) + r"])")
            for name in per_class}

    def holders(text: str) -> set[str]:
        """Every class whose values this case handles, named or not."""
        held = {name for name, pattern in owns.items() if pattern.search(text)}
        for m in re.finditer(r"(Core(?:" + re.escape(BS) + r"[A-Za-z][A-Za-z0-9]*)*)"
                             + r"::([A-Za-z][A-Za-z0-9]*)", text):
            made = builds.get(m.group(1), {}).get(m.group(2))
            if made:
                held.add(made)
        arrows = set(re.findall(r"->([a-z][A-Za-z0-9]*)\s*\(", text))
        growing = True
        while growing:
            growing = False
            for name in list(held):
                for member, made in builds.get(name, {}).items():
                    if member in arrows and made not in held:
                        held.add(made)
                        growing = True
        return held

    texts = [read(p) for p in cases(CONFORMANCE)]
    handled = [holders(t) for t in texts]
    out = []
    for owner, members in per_class.items():
        mine = [t for t, held in zip(texts, handled) if owner in held]
        asked = {member: 0 for member in members}
        for text in mine:
            named = {m.group(1) for m in
                     re.finditer(re.escape(owner) + r"::([A-Za-z][A-Za-z0-9]*)", text)}
            named.update(re.findall(r"->([a-z][A-Za-z0-9]*)\s*\(", text))
            for member in named & asked.keys():
                asked[member] += 1
        missing = [{"member": member, "anchor": anchor_of(owner, member)}
                   for member in sorted(m for m in asked if not asked[m])]
        counts = sorted(asked.values())
        order = sorted(asked, key=lambda m: (asked[m], m))
        out.append({
            "class": owner,
            "members": len(members),
            "cases": len(mine),
            "depth": statistics.median(counts) if counts else 0.0,
            "floor": counts[0] if counts else 0,
            "thin": [{"member": m, "cases": asked[m], "anchor": anchor_of(owner, m)}
                     for m in order[:3]],
            "uncalled": missing,
        })
    out.sort(key=lambda r: (r["depth"], r["floor"], -r["members"]))
    return out


def member_report(name: str) -> list[str]:
    """Which cases already call one member, so a session can see what is asked before adding."""
    lines = []
    for label, root in (("conformance", CONFORMANCE), ("differential", DIFFERENTIAL)):
        for path in cases(root):
            text = read(path)
            if name in text:
                title = ""
                for i, ln in enumerate(text.splitlines()):
                    if ln.strip() == "--TEST--":
                        title = text.splitlines()[i + 1].strip()
                        break
                lines.append(f"  {label:12} {rel(path)}")
                if title:
                    lines.append(f"               {title[:110]}")
    return lines


# ------------------------------------------------------------------------------- driver


def show(rows: list, limit: int, render) -> None:
    shown = rows if limit <= 0 else rows[:limit]
    for row in shown:
        print(render(row))
    if len(shown) < len(rows):
        print(f"  ... and {len(rows) - len(shown)} more (--limit 0 for all)")


def main() -> int:
    # Spec prose and a `Core` message both carry `§` and em dashes, and a redirected stdout on
    # Windows defaults to cp1252, where the first of them raises UnicodeEncodeError. loop.py's
    # own header says why this is not optional.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass

    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--differential", action="store_true", help="only the oracle-case gap")
    ap.add_argument("--errors", action="store_true", help="only the unasserted error paths")
    ap.add_argument("--coverage", action="store_true",
                    help="only the per-class depth table: which family is thinnest")
    ap.add_argument("--member", help="what the corpus already asks of one member")
    ap.add_argument("--limit", type=int, default=25, help="rows per list; 0 for all")
    ap.add_argument("--json", action="store_true", help="print one JSON object instead")
    opts = ap.parse_args()

    if not SPEC.exists():
        print(f"missing {rel(SPEC)}", file=sys.stderr)
        return 2

    if opts.member:
        found = member_report(opts.member)
        print(f"== {opts.member}")
        print("\n".join(found) if found else "  no case calls it")
        return 0

    both = not (opts.differential or opts.errors or opts.coverage)
    diff = differential_gaps() if (both or opts.differential) else []
    errs = error_gaps() if (both or opts.errors) else []
    cov = coverage() if (both or opts.coverage) else []

    if opts.json:
        print(json.dumps({"differential": diff, "errors": errs, "coverage": cov}, indent=1))
        return 0

    if both or opts.coverage:
        thin = [r for r in cov if r["uncalled"]]
        print(f"== CONFORMANCE DEPTH BY CLASS  ({len(cov)} classes, thinnest first; "
              f"{len(thin)} with a member no case calls)")
        print("-- DEPTH is the MEDIAN cases per member and FLOOR its worst member, so a big class")
        print("-- is not thin merely for being big -- take the group from the members named at the")
        print("-- right, which are the three each class asks least, with their anchors.")
        show(cov, opts.limit, lambda r: (
            f"  {r['depth']:>5.1f}{r['floor']:>6}{r['cases']:>7}{r['members']:>9}"
            f"   {r['class']:<22}"
            + (" no case calls " + ", ".join(
                f"{u['member']} {u['anchor']}" for u in r["uncalled"][:3]) if r["uncalled"]
               else " " + ", ".join(f"{t['member']} {t['cases']}" for t in r["thin"]))
        ))
        print("     ^depth ^floor ^cases ^members  ^thinnest members, and their case counts")
        print()

    if both or opts.differential:
        have = len(cases(DIFFERENTIAL))
        print(f"== DIFFERENTIAL GAP  ({len(diff)} members with a PHP twin and no oracle case; "
              f"the suite holds {have})")
        print("-- the twin is the spec's Replaces column; PHP computes the expectation, so a case")
        print("-- here needs no frozen output. Never in tests/conformance/ (conventions.md).")
        show(diff, opts.limit,
             lambda r: f"  {r['member']:34} <- {', '.join(r['php'])[:44]:46} {r['anchor']}")
        print()

    if both or opts.errors:
        by_kind = {}
        for e in errs:
            by_kind[e["kind"]] = by_kind.get(e["kind"], 0) + 1
        kinds = ", ".join(f"{v} {k}" for k, v in sorted(by_kind.items()))
        print(f"== UNASSERTED ERROR PATHS  ({len(errs)}: {kinds})")
        print("-- a `Fault::fatal` may be an internal invariant no program can reach; a")
        print("-- `thrown` is a boundary a case can catch and echo. Judge before writing.")
        show(errs, opts.limit,
             lambda r: f"  {r['anchor']:36} {r['kind']:10} {r['message'][:76]}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
