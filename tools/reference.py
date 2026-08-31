#!/usr/bin/env python3
"""Generate `docs/novis.md` -- the one-file reference to everything Novis has -- and prove its examples.

    python tools/reference.py                 # regenerate docs/novis.md, then run every example in it
    python tools/reference.py --check         # regenerate in memory; exit 1 if docs/novis.md is stale
    python tools/reference.py --no-examples   # regenerate only (sub-second)
    python tools/reference.py --examples-only [--only <substring>]   # run the examples, write nothing
    python tools/reference.py --keep          # leave the example directories under .agent-tmp/ behind

## What it builds, and from what

`docs/novis.md` is written for a reader that has never seen this repository -- a search engine, a
language model, a person -- and is generated, never edited. Its three inputs:

1. **`nvs meta --json`** from the freshly built binary: every `Core` class, member, signature,
   reference card, constant and enum, plus the exception tree, the global interfaces, the
   compiler-recognized attributes and the `nvs.toml` directives. The binary is the one source for
   all of Part B, so the reference cannot describe a member that does not run.
2. **`docs/reference/lang/*.md` and `docs/reference/tools/*.md`** -- one hand-written chapter per
   language topic and per tool, in filename order. `docs/reference/README.md` is the format.
3. **`docs/reference/core/<Class>.md`** -- an optional hand-written introduction and example per
   `Core` class, keyed by the class name after `Core\\` with `\\` written `-` (`Time-DateTime.md`).

Every fenced `nvs` block in a chapter is a program this tool **runs against the binary**, and the
`output` fence after it is what the program must print -- so an example that stops being true fails
`python tools/verify.py` rather than misleading the next reader. The fence grammar is the README's.

## Why a generated file rather than the spec

The spec is authoritative for the *designed* surface, implemented or not; the ADRs hold reasoning
a language user never needs. This file holds only what ships, in the order a user learns it, with
every member once. `python tools/verify.py` regenerates it after every build, so it follows the
registry without anyone remembering to.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "novis.md"
SOURCES = ROOT / "docs" / "reference"
MIGRATION = ROOT / "docs" / "spec" / "02-php-migration.md"
TMP = ROOT / ".agent-tmp" / "reference-examples"
BINARY = ROOT / "target" / "debug" / ("nvs.exe" if os.name == "nt" else "nvs")

#: A chapter's leading `---` block: `key: value` lines, three of which mean something.
FRONT_RE = re.compile(r"\A---\n(.*?)\n---\n", re.S)
#: A fenced block: the info string, then the body up to the closing fence.
FENCE_RE = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.M | re.S)
#: A generated-table placeholder inside a chapter.
PLACEHOLDER_RE = re.compile(r"^<!-- generated: ([a-z-]+) -->$", re.M)
#: A chapter's own-source note, stripped from the generated file.
SRC_RE = re.compile(r"^<!-- src:.*?-->[ \t]*\n?", re.M)
#: A markdown heading, for demotion and for the anchor index.
HEADING_RE = re.compile(r"^(#{1,6}) (.*)$", re.M)
#: One row of the migration table.
ROW_RE = re.compile(r"^\| `([^`]+)` \| (member|language|dropped|open) \| (.*) \|$")
#: A markdown link whose target is a path rather than a URL or an in-page anchor.
REL_LINK_RE = re.compile(r"(?<=\]\()(?!\w+:|[#/])([^)]+)(?=\))")

TIMEOUT = 60  # seconds per example; a hung example is a bug in the example

#: A constant whose value the binary answers per platform (`Core\Path::SEPARATOR`), and the one
#: spelling the generated file uses for it on every platform.
PLATFORM_VALUES = {'"\\\\"': '"/" ("\\" on Windows)', '"/"': '"/" ("\\" on Windows)'}


# ------------------------------------------------------------------ sources


@dataclass
class Chapter:
    path: Path
    id: str
    title: str
    keywords: str
    summary: str
    body: str

    @property
    def anchor(self) -> str:
        part = "tools" if self.path.parent.name == "tools" else "lang"
        return f"{part}-{self.id}"


def parse_front(text: str, path: Path) -> tuple[dict[str, str], str]:
    m = FRONT_RE.match(text.replace("\r\n", "\n"))
    if not m:
        sys.exit(f"reference.py: {path.relative_to(ROOT).as_posix()} has no leading `---` block")
    fields: dict[str, str] = {}
    for line in m.group(1).splitlines():
        key, _, value = line.partition(":")
        value = value.strip()
        # A title holding a `:` has to be quoted to survive the split above; the quotes are
        # the front matter's, not the title's.
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        fields[key.strip()] = value
    return fields, text.replace("\r\n", "\n")[m.end():]


def load_chapters(part: str) -> list[Chapter]:
    out = []
    for path in sorted((SOURCES / part).glob("*.md")):
        fields, body = parse_front(path.read_text(encoding="utf-8"), path)
        for need in ("id", "title"):
            if need not in fields:
                sys.exit(f"reference.py: {path.relative_to(ROOT).as_posix()} lacks `{need}:`")
        out.append(Chapter(path, fields["id"], fields["title"], fields.get("keywords", ""),
                           fields.get("summary", ""), body.strip("\n")))
    return out


def class_intro(name: str) -> tuple[dict[str, str], str] | None:
    """The hand-written introduction for a `Core` class, or None when nobody wrote one."""
    stem = name.removeprefix("Core\\").replace("\\", "-")
    path = SOURCES / "core" / f"{stem}.md"
    if not path.is_file():
        return None
    return parse_front(path.read_text(encoding="utf-8"), path)


def registry() -> dict:
    if not BINARY.is_file():
        sys.exit(f"reference.py: no binary at {BINARY.relative_to(ROOT).as_posix()} -- "
                 "`cargo build -p nvs-cli` first")
    p = subprocess.run([str(BINARY), "meta", "--json"], capture_output=True, cwd=ROOT)
    if p.returncode != 0:
        sys.exit(f"reference.py: `nvs meta --json` failed:\n{p.stderr.decode('utf-8', 'replace')}")
    return json.loads(p.stdout.decode("utf-8"))


def reroot_links(text: str, source: Path) -> str:
    """Rewrite `text`'s relative links from `source`'s directory to `OUT`'s.

    A cell copied out of `docs/spec/` keeps the links it was written with, and those
    resolve from `docs/spec/` -- not from `docs/novis.md`, one directory up, where the
    copy ends up. Re-express each one against `OUT`'s directory so it still resolves.
    """
    def sub(m: re.Match) -> str:
        target, _, anchor = m.group(1).partition("#")
        if not target:                       # a bare `#anchor` is in-page; leave it
            return m.group(1)
        moved = os.path.relpath(source.parent / target, OUT.parent).replace(os.sep, "/")
        return moved + ("#" + anchor if anchor else "")
    return REL_LINK_RE.sub(sub, text)


def migration_rows() -> list[tuple[str, str, str, str]]:
    """(section, php, outcome, novis) for every row of docs/spec/02-php-migration.md."""
    rows = []
    section = ""
    for line in MIGRATION.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            section = line[3:].strip()
        m = ROW_RE.match(line)
        if m:
            rows.append((section, m.group(1), m.group(2), m.group(3)))
    return rows


# ------------------------------------------------------------------ rendering helpers


def anchor_id(*parts: str) -> str:
    text = "-".join(parts).lower()
    text = text.replace("\\", "-").replace("::", "-")
    text = re.sub(r"[^a-z0-9]+", "-", text)
    return text.strip("-")


def anchor(id_: str) -> str:
    return f'<a id="{id_}"></a>'


def demote(body: str, by: int) -> str:
    """Push every heading in a chapter body down `by` levels, so a chapter's own `##` nests."""
    return HEADING_RE.sub(lambda m: "#" * min(6, len(m.group(1)) + by) + " " + m.group(2), body)


def code(text: str) -> str:
    return "`" + text + "`"


def esc(text: str) -> str:
    """Escape a `|` so a cell holding one does not split the table row."""
    return text.replace("|", "\\|")


#: A parenthesised ADR citation inside a registry card -- `(ADR 0013)`, `(ADR 0056 § 4)` -- which
#: names a file this repository's readers have and the reference's readers do not.
ADR_PAREN_RE = re.compile(r"\s*\(ADR \d{4}(?: §+ [\w.\-]+)?\)")


def card(text: str) -> str:
    """A card's prose with its parenthesised ADR citations removed; an inline one stays, since
    cutting it would leave the sentence ungrammatical."""
    return ADR_PAREN_RE.sub("", text)


# ------------------------------------------------------------------ Part B: the Core library


def member_heading(class_name: str, member: dict) -> str:
    if member["kind"] == "instance":
        return f"{class_name}->{member['name']}"
    return f"{class_name}::{member['name']}"


def spelled_signature(class_name: str, member: dict) -> str:
    """How a call is written: `Core\\Str::length(string $s): uint`, `$dt->format(string $pattern)`."""
    sig = member["signature"]
    if member["kind"] == "instance":
        return f"${class_var(class_name)}->{sig}"
    if member["kind"] == "constructor":
        return f"new {class_name}{type_args(member)}({sig[len('constructor('):]}"
    return f"{class_name}::{sig}"


def type_args(member: dict) -> str:
    return ""


def class_var(class_name: str) -> str:
    return class_name.rsplit("\\", 1)[-1][0].lower() + class_name.rsplit("\\", 1)[-1][1:]


def render_member(class_name: str, member: dict, lines: list[str]) -> None:
    heading = member_heading(class_name, member)
    lines.append(anchor(anchor_id("core", heading)))
    lines.append(f"#### `{heading}`")
    lines.append("")
    lines.append("```nvs skip")
    lines.append(spelled_signature(class_name, member))
    lines.append("```")
    lines.append("")
    doc = member.get("doc", {})
    if doc.get("short"):
        lines.append(card(doc["short"]))
        lines.append("")
    docs_by_name = {p["name"]: p for p in doc.get("params", [])}
    params = member.get("params", [])
    options = member.get("options", [])
    if params or options:
        lines.append("| Parameter | Type | Meaning |")
        lines.append("|---|---|---|")
        for p in params:
            name = "..." + "$" + p["name"] if p.get("variadic") else "$" + p["name"]
            ty = p["type"]
            extras = []
            if "default" in p:
                extras.append(f"default {code(p['default'])}")
            if p.get("qualifier") and p["qualifier"] != "contagious":
                extras.append(p["qualifier"])
            desc = card(docs_by_name.get(p["name"], {}).get("desc", ""))
            shape = docs_by_name.get(p["name"], {}).get("shape", [])
            if shape:
                desc += " Keys: " + "; ".join(
                    f"`{k['key']}` ({k['type']}) {card(k['desc'])}" for k in shape)
            meta = f" ({', '.join(extras)})" if extras else ""
            lines.append(f"| `{name}` | `{esc(ty)}`{meta} | {esc(desc)} |")
        for o in options:
            extras = [f"default {code(o['default'])}"]
            if o.get("qualifier") and o["qualifier"] != "contagious":
                extras.append(o["qualifier"])
            desc = card(docs_by_name.get(o["name"], {}).get("desc", ""))
            lines.append(f"| `{{{o['name']}: …}}` | `{esc(o['type'])}` ({', '.join(extras)}) | {esc(desc)} |")
        lines.append("")
    ret = card(doc.get("return", ""))
    if member["kind"] != "constructor":
        lines.append(f"**Returns** `{member['returns']}`" + (f" — {ret}" if ret else ""))
        lines.append("")
    if doc.get("errors"):
        lines.append("**Throws** " + "; ".join(
            f"`{e['error']}` — {card(e['desc'])}" for e in doc["errors"]))
        lines.append("")


def render_class(cls: dict, lines: list[str], index: list[str]) -> None:
    name = cls["name"]
    display = name + ("<" + ", ".join(cls["typeParams"]) + ">" if cls.get("typeParams") else "")
    lines.append(anchor(anchor_id("core", name)))
    lines.append(f"### `{display}`")
    lines.append("")
    intro = class_intro(name)
    summary = ""
    keywords = ", ".join(m["name"] for m in cls["members"])
    if intro:
        fields, body = intro
        summary = fields.get("summary", "")
        if fields.get("keywords"):
            keywords = fields["keywords"] + ", " + keywords
    lines.append(f"Keywords: {keywords}")
    lines.append("")
    if intro:
        lines.append(demote(body.strip("\n"), 3))
        lines.append("")
    index.append(f"| [`{display}`](#{anchor_id('core', name)}) | {esc(summary)} |")
    # The quick index of members, one line each, before the cards.
    lines.append("| Member | Signature |")
    lines.append("|---|---|")
    if cls.get("constructor"):
        c = cls["constructor"]
        lines.append(f"| `new {name}` | `{esc(spelled_signature(name, c))}` |")
    for m in cls["members"]:
        lines.append(f"| [`{member_heading(name, m)}`](#{anchor_id('core', member_heading(name, m))})"
                     f" | `{esc(m['signature'])}` |")
    for k in cls.get("constants", []):
        value = k["value"]
        if value in PLATFORM_VALUES:
            # The binary answers the platform it was built on; the reference is read
            # everywhere, so it names both rather than whichever machine ran the generator.
            value = PLATFORM_VALUES[value]
        lines.append(f"| `{name}::{k['name']}` | `{k['type']}` = `{esc(value)}` — {esc(card(k.get('doc', '')))} |")
    lines.append("")
    if cls.get("constructor"):
        render_member(name, dict(cls["constructor"], name="constructor"), lines)
    for m in cls["members"]:
        render_member(name, m, lines)


def render_enums(enums: list[dict], lines: list[str]) -> None:
    lines.append(anchor("core-enums"))
    lines.append("### `Core` enums")
    lines.append("")
    lines.append("Every enum `Core` declares. A case is written `Core\\Order::Asc` and is an `int` "
                 "underneath (see the enums chapter); a member's signature names the enum it takes.")
    lines.append("")
    for e in enums:
        doc = e.get("doc", {})
        lines.append(anchor(anchor_id("enum", e["name"])))
        lines.append(f"#### `{e['name']}`")
        lines.append("")
        if doc.get("short"):
            lines.append(card(doc["short"]))
            lines.append("")
        lines.append("| Case | Meaning |")
        lines.append("|---|---|")
        for c in doc.get("cases", []):
            lines.append(f"| `{e['name']}::{c['name']}` | {esc(card(c['desc']))} |")
        lines.append("")


def table_exceptions(reg: dict) -> str:
    lines = ["| Class | Extends | Own properties |", "|---|---|---|"]
    for x in reg["exceptions"]:
        lines.append(f"| `{x['name']}` | {code(x['parent']) if x.get('parent') else '— (the root)'} | "
                     f"{', '.join(code('$' + p) for p in x.get('properties', [])) or '—'} |")
    return "\n".join(lines)


def table_interfaces(reg: dict) -> str:
    lines = ["| Interface | Type parameters |", "|---|---|"]
    for i in reg["interfaces"]:
        params = ", ".join(i.get("typeParams", [])) or "—"
        lines.append(f"| `{i['name']}` | {params} |")
    return "\n".join(lines)


def table_attributes(reg: dict) -> str:
    return "\n".join(f"- `#[{a}]`" for a in reg["attributes"])


def table_directives(reg: dict) -> str:
    lines = ["| Key | Class | Applies |", "|---|---|---|"]
    meaning = {"System": "operator only — a request cannot change it",
               "Runtime": "a request may retune it, up to the `[limits.hard]` ceiling",
               "RuntimeTighten": "a request may only narrow it"}
    for d in reg["directives"]:
        lines.append(f"| `{d['key']}` | {meaning.get(d['class'], d['class'])} | "
                     f"{'at reload' if d['apply'] == 'Reload' else 'at boot only'} |")
    return "\n".join(lines)


def table_migration(reg: dict) -> str:
    known = set()
    for c in reg["classes"]:
        for m in c["members"]:
            known.add(f"{c['name']}::{m['name']}")
            known.add(f"{c['name']}->{m['name']}")
    lines = ["| PHP | Outcome | Novis |", "|---|---|---|"]
    kept = 0
    for _section, php, outcome, novis in migration_rows():
        if outcome == "open":
            continue
        if outcome == "member":
            names = re.findall(r"`(Core\\[A-Za-z\\]+(?:::|->)[a-zA-Z]+)`", novis)
            if not names or not all(n in known for n in names):
                continue
        lines.append(f"| `{php}` | {outcome} | {reroot_links(novis, MIGRATION)} |")
        kept += 1
    return "\n".join(lines)


TABLES = {
    "exceptions": table_exceptions,
    "interfaces": table_interfaces,
    "attributes": table_attributes,
    "directives": table_directives,
    "php-migration": table_migration,
}


# ------------------------------------------------------------------ the document


HOW_TO_READ = """\
## How to read this file

This is the complete reference to the Novis language and its `Core` library, in one file, generated
from the compiler's own registry and from one hand-written chapter per topic. **Do not read it top to
bottom.** Find what you need through the index below, then read one section:

- **Every section starts with an HTML anchor** — `<a id="lang-types"></a>` — directly above its
  heading, and every heading names its subject in full (`#### Core\\Str::length`), so a search for the
  anchor id, the heading text, or a keyword lands on the section. A `Keywords:` line under each
  chapter and class heading lists what it covers for searching.
- **Part A is the language**: syntax and semantics, one chapter per topic, each with runnable
  examples. Read A.1 first if you have never seen Novis; it is short. Read a chapter in full when you
  need its topic — they are written to be complete rather than introductory.
- **Part B is the `Core` library**: one section per class, opening with a short introduction and one
  worked example, then a member index, then **one card per member** with its signature, parameters,
  return value and what it throws. Every member exists in the shipped binary; nothing planned is here.
- **Part C is the toolchain**: the `nvs` command, `nvs.toml`, and testing.
- **Part D is the PHP crosswalk**: for someone who knows PHP, what each built-in became.

Conventions the whole file uses:

- A signature reads `Core\\Str::length(string $s): uint` for a static member — every `Core` member is
  static unless written `$x->name(...)`, which marks an instance method reached through a value that
  some other member returned. The `$name` of every parameter is callable by name: `Core\\Str::length(s:
  $x)`. A trailing `{a?: T, b?: U}` is an *options bag* — one optional shape argument, written
  `{a: value}`, addressable as `options:`.
- `?T` means "`T` or `null`", and is how every member spells *absent*. Failure is a thrown
  `Throwable` subclass named on the card; nothing returns `false` to mean failure.
- Every code block marked `nvs` is a complete program that was run against the binary while this
  file was generated, and the `output` block after it is exactly what it printed. Blocks marked
  `nvs skip` are fragments. Run a program with `nvs run file.nvs`.
"""


def build(reg: dict) -> str:
    lang = load_chapters("lang")
    tools = load_chapters("tools")
    lines: list[str] = []
    lines.append("# Novis — the complete reference")
    lines.append("")
    # The `GENERATED FILE` marker is the repository's convention and `tools/check-links.py` reads it:
    # without it this file's chapter-relative links (`[the type chapter](20-types.md)`) are checked
    # again here, where they cannot resolve, having become in-file anchors on the way in.
    lines.append("<!-- GENERATED FILE — do not edit by hand. Written by `python tools/reference.py` from "
                 "`nvs meta --json` and docs/reference/: edit the chapter under docs/reference/ or the "
                 "registry in crates/nvs-stdlib, then regenerate. -->")
    lines.append("")
    lines.append(HOW_TO_READ)
    # ---- the index
    lines.append("## Index")
    lines.append("")
    lines.append("### Part A — The language")
    lines.append("")
    for n, ch in enumerate(lang, 1):
        lines.append(f"- A.{n} [{ch.title}](#{ch.anchor}) — {ch.summary or ''} *({ch.keywords})*")
    lines.append("")
    lines.append("### Part B — The `Core` library")
    lines.append("")
    class_index: list[str] = []
    body_b: list[str] = []
    for cls in reg["classes"]:
        render_class(cls, body_b, class_index)
    render_enums(reg["enums"], body_b)
    lines.append("| Class | What it is for |")
    lines.append("|---|---|")
    lines.extend(class_index)
    lines.append(f"| [`Core` enums](#core-enums) | every enum a member takes, with its cases |")
    lines.append("")
    lines.append("### Part C — The toolchain")
    lines.append("")
    for n, ch in enumerate(tools, 1):
        lines.append(f"- C.{n} [{ch.title}](#{ch.anchor}) — {ch.summary or ''} *({ch.keywords})*")
    lines.append("")
    lines.append("### Part D — Coming from PHP")
    lines.append("")
    lines.append("- D.1 [PHP built-ins and what each became](#php-migration)")
    lines.append("")
    # ---- Part A
    used: set[str] = set()

    def expand(body: str) -> str:
        def sub(m: re.Match) -> str:
            name = m.group(1)
            if name not in TABLES:
                sys.exit(f"reference.py: unknown placeholder `{name}`")
            used.add(name)
            return TABLES[name](reg)
        body = PLACEHOLDER_RE.sub(sub, body)
        # A chapter's `<!-- src: ADR 0007 § 3 -->` lines are for this repository's own
        # readers -- which decision owns the paragraph -- and never for the generated file.
        return SRC_RE.sub("", body)

    lines.append("# Part A — The language")
    lines.append("")
    for n, ch in enumerate(lang, 1):
        lines.append(anchor(ch.anchor))
        lines.append(f"## A.{n} {ch.title}")
        lines.append("")
        lines.append(f"Keywords: {ch.keywords}")
        lines.append("")
        lines.append(expand(demote(ch.body, 2)))
        lines.append("")
    # ---- Part B
    lines.append("# Part B — The `Core` library")
    lines.append("")
    lines.append("Every function and constant in Novis is a member of a class under the reserved `Core` "
                 "namespace; there are no free functions. The sections below are in the registry's own "
                 "order. A class with type parameters (`Core\\ObjectMap<K, V>`) is written with them at "
                 "`new`.")
    lines.append("")
    lines.extend(body_b)
    # ---- Part C
    lines.append("# Part C — The toolchain")
    lines.append("")
    for n, ch in enumerate(tools, 1):
        lines.append(anchor(ch.anchor))
        lines.append(f"## C.{n} {ch.title}")
        lines.append("")
        lines.append(f"Keywords: {ch.keywords}")
        lines.append("")
        lines.append(expand(demote(ch.body, 2)))
        lines.append("")
    # ---- Part D
    lines.append("# Part D — Coming from PHP")
    lines.append("")
    lines.append(anchor("php-migration"))
    lines.append("## D.1 PHP built-ins and what each became")
    lines.append("")
    lines.append("Keywords: PHP, migration, replaces, equivalent, what happened to")
    lines.append("")
    lines.append("One row per PHP built-in. *member*: a `Core` member in Part B does the job. "
                 "*language*: an operator or keyword does it. *dropped*: nothing does, and the cell says "
                 "why and what to write instead — a `Core\\Name` in a *dropped* row's text that has no "
                 "section in Part B is a description of the rewrite, not a member that exists today. "
                 "Built-ins still undecided are not listed.")
    lines.append("")
    lines.append(TABLES["php-migration"](reg))
    used.add("php-migration")
    lines.append("")
    # Any roster no chapter placed gets appended so nothing the binary declares is lost.
    for name, fn in TABLES.items():
        if name not in used:
            lines.append(anchor(anchor_id("roster", name)))
            lines.append(f"## {name}")
            lines.append("")
            lines.append(fn(reg))
            lines.append("")
    text = "\n".join(lines)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.rstrip("\n") + "\n"


# ------------------------------------------------------------------ examples


@dataclass
class Example:
    chapter: Path
    index: int
    line: int
    mode: str            # run | error | test
    entry: str           # the entry program's source
    files: dict[str, str] = field(default_factory=dict)
    expected: str | None = None
    exit_code: int = 0


def parse_info(info: str) -> tuple[str, dict[str, str]]:
    parts = info.strip().split()
    lang = parts[0] if parts else ""
    attrs: dict[str, str] = {}
    for p in parts[1:]:
        k, _, v = p.partition("=")
        attrs[k] = v
    return lang, attrs


def examples_in(path: Path) -> list[Example]:
    text = path.read_text(encoding="utf-8").replace("\r\n", "\n")
    out: list[Example] = []
    pending: dict[str, str] = {}
    last: Example | None = None
    for m in FENCE_RE.finditer(text):
        lang, attrs = parse_info(m.group(1))
        body = m.group(2)
        line = text.count("\n", 0, m.start()) + 1
        if lang == "output":
            if last is None:
                sys.exit(f"reference.py: {path.name}:{line}: an `output` block with no program before it")
            last.expected = body
            last = None
            continue
        if "file" in attrs:
            pending[attrs["file"]] = body
            continue
        if lang != "nvs" or "skip" in attrs:
            continue
        mode = "run"
        if "error" in attrs:
            mode = "error"
        elif "test" in attrs:
            mode = "test"
        ex = Example(path, len(out) + 1, line, mode, body, dict(pending),
                     exit_code=int(attrs.get("exit", "0")))
        pending = {}
        out.append(ex)
        last = ex
    return out


def normalize(s: str) -> str:
    s = s.replace("\r\n", "\n")
    return "\n".join(line.rstrip() for line in s.split("\n")).strip("\n")


def run_example(ex: Example, keep: bool) -> str | None:
    """None when the example holds; otherwise one paragraph saying how it failed."""
    where = f"{ex.chapter.relative_to(ROOT).as_posix()}:{ex.line}"
    work = TMP / ex.chapter.parent.name / f"{ex.chapter.stem}-{ex.index:02d}"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    for name, body in ex.files.items():
        target = work / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body, encoding="utf-8", newline="\n")
    entry = work / "main.nvs"
    entry.write_text(ex.entry, encoding="utf-8", newline="\n")
    verb = "test" if ex.mode == "test" else ("check" if ex.mode == "error" else "run")
    try:
        p = subprocess.run([str(BINARY), verb, "main.nvs"], cwd=work, capture_output=True,
                           timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return f"{where}: timed out after {TIMEOUT}s"
    stdout = p.stdout.decode("utf-8", "replace")
    stderr = p.stderr.decode("utf-8", "replace")
    problem = None
    if ex.mode == "error":
        if p.returncode == 0:
            problem = "was expected to fail `nvs check`, but it compiled"
        elif ex.expected is not None and normalize(ex.expected) not in normalize(stderr):
            problem = f"failed as expected, but the diagnostic does not contain the `output` block:\n{stderr.strip()}"
    elif ex.mode == "test":
        if p.returncode != ex.exit_code:
            problem = f"`nvs test` exited {p.returncode}, expected {ex.exit_code}:\n{stdout.strip()}\n{stderr.strip()}"
        elif ex.expected is not None:
            missing = [l for l in normalize(ex.expected).split("\n") if l and l not in stdout]
            if missing:
                problem = f"`nvs test` output lacks {missing!r}:\n{stdout.strip()}"
    else:
        if p.returncode != ex.exit_code:
            problem = f"exited {p.returncode}, expected {ex.exit_code}:\n{stdout.strip()}\n{stderr.strip()}"
        elif ex.expected is not None and normalize(stdout) != normalize(ex.expected):
            problem = f"printed something else.\n--- expected\n{normalize(ex.expected)}\n--- got\n{normalize(stdout)}"
            if stderr.strip():
                problem += f"\n--- stderr\n{stderr.strip()}"
    if not keep and problem is None:
        shutil.rmtree(work, ignore_errors=True)
    return f"{where}: {problem}" if problem else None


def check_examples(only: str | None, keep: bool) -> int:
    paths = sorted((SOURCES / "lang").glob("*.md")) + sorted((SOURCES / "tools").glob("*.md")) \
        + sorted((SOURCES / "core").glob("*.md"))
    if only:
        paths = [p for p in paths if only in p.as_posix()]
    examples = [ex for p in paths for ex in examples_in(p)]
    if not examples:
        print("reference.py: no examples to run")
        return 0
    TMP.mkdir(parents=True, exist_ok=True)
    width = max(2, min(8, (os.cpu_count() or 2) // 2))
    with ThreadPoolExecutor(max_workers=width) as pool:
        results = list(pool.map(lambda ex: run_example(ex, keep), examples))
    failures = [r for r in results if r]
    for f in failures:
        print(f"FAIL {f}\n")
    print(f"reference.py: {len(examples) - len(failures)} of {len(examples)} examples hold"
          + (f" ({len(failures)} failed)" if failures else ""))
    return 1 if failures else 0


# ------------------------------------------------------------------ main


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true",
                    help="regenerate in memory and exit 1 if docs/novis.md differs")
    ap.add_argument("--no-examples", action="store_true", help="regenerate only")
    ap.add_argument("--examples-only", action="store_true", help="run the examples, write nothing")
    ap.add_argument("--only", help="with the examples: only chapters whose path contains this")
    ap.add_argument("--keep", action="store_true", help="leave example directories behind")
    opts = ap.parse_args()

    if opts.examples_only:
        return check_examples(opts.only, opts.keep)

    reg = registry()
    text = build(reg)
    current = OUT.read_text(encoding="utf-8").replace("\r\n", "\n") if OUT.is_file() else ""
    if opts.check:
        if text != current:
            print("reference.py: docs/novis.md is stale -- run `python tools/reference.py` and commit it")
            return 1
        print("reference.py: docs/novis.md is current")
    else:
        if text != current:
            OUT.write_text(text, encoding="utf-8", newline="\n")
            print(f"reference.py: wrote {OUT.relative_to(ROOT).as_posix()} "
                  f"({len(text.encode('utf-8')) // 1024} KB, {text.count(chr(10))} lines)")
        else:
            print("reference.py: docs/novis.md unchanged")
    if opts.no_examples:
        return 0
    return check_examples(opts.only, opts.keep)


if __name__ == "__main__":
    sys.exit(main())
