#!/usr/bin/env python3
"""Derive every `nvs.toml` leaf key from `Config`'s field graph, and say which of them reach a reader.

The typed block tree in `crates/nvs-config/src/tree.rs` is the one home for what
an `nvs.toml` may say: `deny_unknown_fields` on every block makes the struct
roster the accepted key set exactly. What the struct roster does *not* say is
whether anything downstream ever looks at a key it accepts, and a key that
parses and reaches nothing is worse than one that is refused -- the operator
writes it, the file is accepted, and the setting silently does nothing.

    python tools/directives.py          # the roster: every leaf key and how it is read
    python tools/directives.py --check  # exit 1 on a key with no reader and no trailer,
                                        # or a trailer over a key that now has one
    python tools/directives.py --json   # the roster as JSON, for a generator to consume

This is the roster every later stage of goal `config-is-written` consumes: the
generated `nvs.toml` is rendered from it, so a key missing here is a key missing
from the file an operator reads.

**The walk fails closed.** A field whose type this script does not recognise is
an error, not a key quietly left out: a new block added to `tree.rs` cannot join
the accepted key set by saying nothing here, which is the property
`tools/lints.py` has for a crate that carries no `[lints]` table.

**A reader is counted three ways, and every one of them is load-bearing.** A key
is read when the field is touched outside `tree.rs`, *or* when its dotted key
appears as a string literal in code, *or* -- for a name that is one key in the
whole roster -- when that bare name does. No one of them is the answer:
`Core\\Storage` reaches its disk root as `config.get("storage.{disk}.root")` and
never through the field; most blocks are read through the typed field with the
dotted key written nowhere; and a limit is read by the short name
`Core\\Config::set` takes, which is how `config.get("max_script_depth")` reaches
`[limits] max_script_depth`. Deleting one as redundant turns this gate into a
generator of false gaps, which is the failure it exists to prevent -- each of
the three is the *only* reader of at least one key that ships today.

**A field is the unit, not a path.** `[limits]` and `[app.limits]` are one
struct, and the per-app merge folds the second onto the first for the same code
to read, so a reader found under either path answers for both and one field is
one entry in the gate however many paths reach it.

**A registry row names a key; it does not read it.** `crates/nvs-config/src/directive.rs`
holds one row per directive and `crates/nvs-config/src/capability.rs` holds each
capability's name, so every key in the tree appears as a literal in one of them.
Counting those would make every key look read and the gate a rubber stamp, so
the *literal* half skips those two files -- their field accesses still count,
which is how `Cap::grant`'s `caps.debug.as_ref()?.trace` reads the grant it maps.

**What is searched is non-test crate source.** `crates/*/src` and `benches/*/src`,
with `tree.rs` itself, comments and `#[cfg(test)]` modules removed. A test that
round-trips a key is not a reader -- `[cache] dir` has mentions only under
`crates/nvs-config/tests/`, and it is exactly the key nothing acts on.

**The question is whether the key is read, not whether the value is acted on.**
A grant that `Cap::grant` maps but no door ever asks reads as read here, because
the field is touched. That deeper question belongs to the capability registry
and is answered by hand, not by this walk.

A key with no reader declares itself, in the field's own doc comment in
`tree.rs`, with a trailer on its last line:

    [unread: <why nothing reads it yet> owner: <who closes it>]

The field is the key, so the field's comment is the one home for that and there
is no manifest file to keep in step with it. `--check` fails in both directions:
a key with no reader and no trailer is one that landed silently, and a trailer
over a key that now *has* a reader is the worse failure, because that is the one
that makes a generated file lie about its own surface.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TREE_REL = "crates/nvs-config/src/tree.rs"
TREE = ROOT / TREE_REL

#: The struct the walk starts from. Everything an `nvs.toml` may say is reachable
#: from it, because it is what one configuration file deserializes into.
ROOT_STRUCT = "Config"

#: The two tables that name every key without reading any of them. A literal
#: match here is a classification, so it does not count; a field access does.
REGISTRIES = (
    "crates/nvs-config/src/directive.rs",
    "crates/nvs-config/src/capability.rs",
)

#: Types that end the walk. `Vec<String>` is a leaf too -- a list value, not a
#: table -- and is handled where the generic parameter is unwrapped.
SCALARS = {"String", "bool", "u8", "u16", "u32", "u64", "i32", "i64", "f32", "f64", "usize"}

#: The segment a map block's key carries in place of the name an operator picks:
#: `[db.<name>]`, `[mail.<name>]`, `[storage.<name>]`.
NAME = "<name>"

TRAILER = re.compile(r"\[unread:\s*(?P<why>[^\]]+?)\s+owner:\s*(?P<owner>[^\]]+?)\s*\]")
TRAILER_LIKE = re.compile(r"\[unread:")

#: What this workspace binds a block to when it is neither named after its key nor
#: after its type. `written` is the idiom for *what the operator wrote*, and it is
#: how `nvs_config::db` reads every `[db.<name>.pool]` bound.
BINDINGS = {"written"}

#: A run of method calls between two field accesses -- `.as_ref()?`, `.as_deref()`,
#: `.clone()`. What it may not swallow is another field, so a receiver's name still
#: has to reach the field for the access to count.
CHAIN = r"(?:\s*\.\s*\w+\s*\([^()]*\)\s*\??)*"


# ------------------------------------------------------------------------------ tree.rs


class Field:
    """One `pub name: Type` in a block struct, with what `serde` makes of it."""

    def __init__(self, name: str, ty: str, key: str, doc: list[str], line: int, flatten: bool):
        self.name = name
        self.ty = ty
        self.key = key
        self.doc = doc
        self.line = line
        self.flatten = flatten


def parse_tree(text: str) -> tuple[dict[str, list[Field]], dict[str, list[str]]]:
    """Every block struct's fields and every enum's payload types, from the source.

    A regex reader rather than a Rust parse: the file is one flat run of derived
    structs with no generics, no `impl` blocks between a field and its type, and
    `deny_unknown_fields` on all of them, so the shape this reads is the shape
    the module is required to keep. Anything it cannot read raises rather than
    being skipped."""
    structs: dict[str, list[Field]] = {}
    enums: dict[str, list[str]] = {}
    doc: list[str] = []
    attrs: list[str] = []
    holder: str | None = None
    kind = ""

    for number, raw in enumerate(text.split("\n"), start=1):
        line = raw.strip()
        if holder is None:
            if line.startswith("///"):
                doc.append(line[3:].strip())
                continue
            if line.startswith("#["):
                attrs.append(line)
                continue
            opened = re.match(r"pub (struct|enum) (\w+) \{$", line)
            if opened:
                kind, holder = opened.group(1), opened.group(2)
                if kind == "struct":
                    structs[holder] = []
                else:
                    enums[holder] = []
                for attr in attrs:
                    if "rename_all" in attr:
                        raise SystemExit(
                            f"directives.py: {TREE_REL}:{number}: `{holder}` carries "
                            f"`rename_all`, which this walk does not spell. Teach it the "
                            f"renaming, or drop the attribute."
                        )
            doc, attrs = [], []
            continue

        if line == "}":
            holder = None
            doc, attrs = [], []
            continue
        if line.startswith("///"):
            doc.append(line[3:].strip())
            continue
        if line.startswith("#["):
            attrs.append(line)
            continue
        if kind == "struct":
            field = re.match(r"pub (\w+): (.+),$", line)
            if field:
                name, ty = field.group(1), field.group(2)
                renamed = next(
                    (m.group(1) for a in attrs if (m := re.search(r'rename = "([^"]+)"', a))),
                    None,
                )
                structs[holder].append(
                    Field(name, ty, renamed or name, doc, number,
                          any("flatten" in a for a in attrs))
                )
                doc, attrs = [], []
                continue
        else:
            variant = re.match(r"\w+\((.+)\),$", line)
            if variant:
                enums[holder].append(variant.group(1))
                doc, attrs = [], []
                continue
        if line:
            doc, attrs = [], []

    if ROOT_STRUCT not in structs:
        raise SystemExit(f"directives.py: {TREE_REL} has no `pub struct {ROOT_STRUCT}`")
    return structs, enums


def unwrap(ty: str) -> str:
    """`Option<T>` is how every optional block and leaf is written; the walk sees `T`."""
    inner = re.match(r"Option<(.+)>$", ty)
    return inner.group(1) if inner else ty


class Key:
    """One leaf of the walk: the dotted key, and the field in `tree.rs` that is its home."""

    def __init__(self, dotted: str, owner: str, field: Field, receivers: set[str]):
        self.dotted = dotted
        self.owner = owner
        self.field = field
        self.receivers = receivers
        self.readers: list[str] = []
        self.trailer: tuple[str, str] | None = None

    @property
    def anchor(self) -> str:
        return f"{TREE_REL}:{self.field.line}"


def snake(name: str) -> list[str]:
    """`MailEndpoint` as the identifiers a reader is likely to bind it to: the whole
    name in snake case, and its last word -- `mail_endpoint`, then `endpoint`."""
    parts = re.findall(r"[A-Z][a-z0-9]*", name)
    lowered = [p.lower() for p in parts]
    return ["_".join(lowered), lowered[-1]] if lowered else []


def walk(structs, enums, name: str, prefix: list[str], chain: list[str],
         element: bool = False) -> list[Key]:
    """Every leaf key under `name`, as the dotted path an operator writes.

    `prefix` carries the segments already spelled, `chain` the struct names on the
    way here -- a repeat is a cycle, which would be an unbounded key space rather
    than a deep one, so it raises."""
    if name in chain:
        raise SystemExit(
            f"directives.py: `{name}` reaches itself through "
            f"{' -> '.join(chain)}, so the key space has no bottom."
        )
    # The identifiers a reader plausibly binds *this block* to: the segment it is
    # written under, and its type's name. A map block's segment is the name the
    # operator chose, so for one of those the type is all there is.
    written = [s for s in prefix if s != NAME]
    receivers = ({written[-1]} if written else set()) | set(snake(name))
    receivers |= BINDINGS
    if element:
        # One of many, picked out of an array or a map, so the code that holds it
        # cannot name it after the key -- `[[server.mount]]` is read as `block.scan`
        # and `[[schedule]]` as `entry.cron`. These two are what the corpus uses.
        receivers |= {"block", "entry"}
    found: list[Key] = []
    for field in structs[name]:
        segments = prefix if field.flatten else [*prefix, field.key]
        found += expand(structs, enums, field, unwrap(field.ty), segments, name, receivers, chain)
    return found


def expand(structs, enums, field, ty, segments, owner, receivers, chain) -> list[Key]:
    """One field's contribution: a leaf key, a block to walk into, or both."""
    dotted = ".".join(segments)
    listed = re.match(r"Vec<(.+)>$", ty)
    mapped = re.match(r"BTreeMap<String, (.+)>$", ty)

    if ty in SCALARS or (listed and listed.group(1) in SCALARS):
        return [Key(dotted, owner, field, receivers)]
    if listed and listed.group(1) in structs:
        # `[[include]]`, `[[app]]`, `[[server.mount]]` -- an array of tables, whose
        # keys are spelled under the same segment as a single block's.
        return walk(structs, enums, listed.group(1), segments, [*chain, owner], element=True)
    if mapped and mapped.group(1) in structs:
        return walk(structs, enums, mapped.group(1), [*segments, NAME], [*chain, owner],
                    element=True)
    if ty in structs:
        return walk(structs, enums, ty, segments, [*chain, owner])
    if ty in enums:
        # `Setting` is scalar in every variant and is the leaf; `Pool` is a scalar
        # *and* a table under the same key, so it is both a leaf and a walk.
        found: list[Key] = []
        for payload in enums[ty]:
            inner = re.match(r"Vec<(.+)>$", payload)
            if payload in SCALARS or (inner and inner.group(1) in SCALARS):
                if not any(k.dotted == dotted for k in found):
                    found.append(Key(dotted, owner, field, receivers))
            elif payload in structs:
                found += walk(structs, enums, payload, segments, [*chain, owner])
            else:
                raise SystemExit(
                    f"directives.py: {TREE_REL}:{field.line}: `{ty}::{payload}` is a "
                    f"variant this walk cannot classify."
                )
        return found
    raise SystemExit(
        f"directives.py: {TREE_REL}:{field.line}: `{owner}::{field.name}` is a "
        f"`{ty}`, which is neither a scalar, a block in this module nor a map of "
        f"one. Add the type to the walk rather than leaving its keys uncounted."
    )


# ------------------------------------------------------------------------------ readers


def strip(text: str) -> tuple[str, list[str]]:
    """Rust source with its comments removed, and every string literal in it.

    Comments are not readers: the corpus names `fs.read` and `script.spawn` in
    prose far more often than it reads them, and a doc comment that mentions a
    key would otherwise answer for the code that does not. The scanner tracks
    strings so that a `//` inside one -- a URL, a path -- is not read as a comment
    opener, and raw strings so an escaped quote inside one is not read as its end."""
    out: list[str] = []
    literals: list[str] = []
    i, n = 0, len(text)
    while i < n:
        ch = text[i]
        if ch == "/" and i + 1 < n and text[i + 1] == "/":
            while i < n and text[i] != "\n":
                i += 1
            continue
        if ch == "/" and i + 1 < n and text[i + 1] == "*":
            depth, i = 1, i + 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif text.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            continue
        raw = re.match(r'r(#*)"', text[i:])
        if raw:
            fence = '"' + raw.group(1)
            end = text.find(fence, i + len(raw.group(0)))
            end = n if end < 0 else end + len(fence)
            literals.append(text[i + len(raw.group(0)):end - len(fence)])
            out.append(" " * (end - i))
            i = end
            continue
        if ch == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    break
                j += 1
            literals.append(text[i + 1:j])
            out.append(" " * (j + 1 - i))
            i = j + 1
            continue
        out.append(ch)
        i += 1
    return "".join(out), literals


def corpus() -> list[tuple[str, str, list[str]]]:
    """Every non-test crate source file, as `(path, code, literals)`.

    A `#[cfg(test)]` module at column 0 and everything after it is dropped: the
    module is last by convention in this workspace, and a test that writes a key
    into a fixture and reads it back is not what makes the key live."""
    files: list[tuple[str, str, list[str]]] = []
    for pattern in ("crates/*/src/**/*.rs", "benches/*/src/**/*.rs"):
        for path in sorted(ROOT.glob(pattern)):
            rel = path.relative_to(ROOT).as_posix()
            if rel == TREE_REL:
                continue
            text = path.read_text(encoding="utf-8")
            cut = re.search(r"^#\[cfg\(test\)\]", text, re.M)
            code, literals = strip(text[: cut.start()] if cut else text)
            files.append((rel, code, literals))
    return files


def literal_re(dotted: str) -> re.Pattern:
    """The key as a reader spells it in a string. `<name>` is whatever the operator
    called the block, so it matches any run without a dot, a quote or a space --
    which is what makes `"storage.{disk}.root"` the reader of `storage.<name>.root`."""
    body = r"\.".join(r"[^\".\s]+" if s == NAME else re.escape(s) for s in dotted.split("."))
    return re.compile(rf'(?<![\w.]){body}(?![\w.])')


def access_re(field: str, receivers: set[str]) -> re.Pattern:
    """The field read off a block the reader already holds.

    The receiver has to name the block -- the segment it is written under, or its
    type -- because the field names alone are `path`, `host`, `dir` and `listen`,
    which every crate in the workspace uses for something else. A trailing `(` is
    excluded for the same reason: `cache.dir()` in the artifact cache is a method
    on another type, not a read of `[cache] dir`."""
    recv = "|".join(sorted((re.escape(r) for r in receivers), key=len, reverse=True))
    return re.compile(rf"\b(?:{recv})\b{CHAIN}\s*\.\s*{re.escape(field)}\b(?!\s*\()")


def bare_names(keys: list[Key]) -> set[str]:
    """The field names that identify exactly one key in the roster.

    `Core\\Config::get` takes a limit by its bare name -- `config.get("max_script_depth")`
    is how the isolate depth ceiling is read, and `nvs_config::value`'s unit table is
    keyed the same way -- and the short spelling only resolves at all for a name that
    is one key. A name two blocks share is left to the other two halves."""
    fields = {id(k.field): k.field.name for k in keys}
    counted: dict[str, int] = {}
    for name in fields.values():
        counted[name] = counted.get(name, 0) + 1
    return {name for name, count in counted.items() if count == 1}


def find_readers(keys: list[Key]) -> None:
    """Fill in each key's readers, by all three spellings, over the whole corpus."""
    files = corpus()
    unique = bare_names(keys)
    for key in keys:
        literal = literal_re(key.dotted)
        access = access_re(key.field.name, key.receivers)
        bare = key.field.name if key.field.name in unique else None
        for rel, code, literals in files:
            if rel not in REGISTRIES and any(literal.search(text) for text in literals):
                key.readers.append(f"{rel} (key)")
            elif access.search(code):
                key.readers.append(f"{rel} (field)")
            elif bare and rel not in REGISTRIES and any(text == bare for text in literals):
                key.readers.append(f"{rel} (name)")
    # A block reached at two paths -- `[limits]` and `[app.limits]`, `[capabilities]`
    # and a `[[schedule]]`'s grants -- is one field and one reader: the per-app merge
    # folds the block onto the global one and the same code reads both. So a reader
    # found under either path answers for the field, not for the path it was found at.
    for key in keys:
        if not key.readers:
            key.readers = next((k.readers for k in keys
                                if k.field is key.field and k.readers), [])


# ------------------------------------------------------------------------------ the gate


def read_trailers(keys: list[Key], problems: list[str]) -> None:
    """The `[unread: … owner: …]` each key's own doc comment declares, if any."""
    for key in keys:
        body = [line for line in key.field.doc if line]
        found = TRAILER.search(body[-1]) if body else None
        if found:
            key.trailer = (found.group("why").strip(), found.group("owner").strip())
            continue
        for line in body:
            if TRAILER_LIKE.search(line):
                problems.append(
                    f"{key.anchor}: `{key.dotted}` carries something shaped like an "
                    f"`[unread: … owner: …]` trailer that this does not read. It is "
                    f"`[unread: <why> owner: <who>]`, on the doc comment's last line."
                )
                break


def check(keys: list[Key]) -> list[str]:
    """The gate, reported once per field rather than once per path.

    One field can be several keys -- `[limits]` and `[app.limits]` are one struct --
    and the trailer is on the field, so a field is one decision and one problem."""
    problems: list[str] = []
    read_trailers(keys, problems)
    first = {id(k.field): k for k in reversed(keys)}
    for key in sorted(first.values(), key=lambda k: k.field.line):
        spelled = " / ".join(k.dotted for k in keys if k.field is key.field)
        if key.readers and key.trailer:
            problems.append(
                f"{key.anchor}: `{spelled}` is declared unread, but "
                f"{key.readers[0]} reads it. Delete the trailer -- a generated file "
                f"that marks a live key unimplemented is worse than one that omits it."
            )
        elif not key.readers and not key.trailer:
            problems.append(
                f"{key.anchor}: `{spelled}` reaches no reader and declares nothing. "
                f"Wire it to the code that should act on it, or give the field's doc "
                f"comment an `[unread: <why> owner: <who>]` trailer."
            )
    return problems


def roster() -> list[Key]:
    structs, enums = parse_tree(TREE.read_text(encoding="utf-8"))
    keys = walk(structs, enums, ROOT_STRUCT, [], [])
    find_readers(keys)
    return keys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check", action="store_true",
        help="exit 1 on a key with no reader and no trailer, or a trailer over a read key",
    )
    parser.add_argument("--json", action="store_true", help="the roster as JSON, on stdout")
    args = parser.parse_args()

    keys = roster()
    if args.json:
        read_trailers(keys, [])
        print(json.dumps([
            {"key": k.dotted, "block": k.owner, "field": k.field.name, "anchor": k.anchor,
             "readers": k.readers, "unread": k.trailer[0] if k.trailer else None,
             "owner": k.trailer[1] if k.trailer else None}
            for k in keys
        ], indent=2))
        return 0

    problems = check(keys)
    if args.check:
        if problems:
            print("\n\n".join(problems), file=sys.stderr)
            print(f"\n{len(problems)} problem(s) over {len(keys)} leaf key(s). "
                  f"`python tools/directives.py` lists the roster.", file=sys.stderr)
            return 1
        print(f"directives: {len(keys)} leaf keys, every one read or declared")
        return 0

    read_trailers(keys, [])
    silent = [k for k in keys if not k.readers and not k.trailer]
    declared = [k for k in keys if k.trailer]
    for key in keys:
        mark = "--" if not key.readers else key.readers[0]
        print(f"{key.dotted:<44} {mark}")
    print(f"\n{len(keys)} leaf keys: {len(keys) - len(silent) - len(declared)} read, "
          f"{len(declared)} declared unread, {len(silent)} neither.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
