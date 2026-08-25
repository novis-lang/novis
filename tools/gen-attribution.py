#!/usr/bin/env python3
"""Generate THIRD-PARTY-LICENSES.txt from the resolved dependency graph.

MWL ships as MIT, and every permissive license in its dependency tree asks
for the same thing in return: reproduce the notice with the binary. This
script produces the one file that satisfies that, for both the repository
and — via `include_str!` in `mwl-cli` — the shipped `mwl` binary itself.
ADR 0065 owns the policy; this file only implements it.

    python tools/gen-attribution.py            # regenerate
    python tools/gen-attribution.py --check    # exit 1 if stale (CI)

Three properties are worth knowing before changing anything here:

* **It fails closed.** An SPDX identifier this script has never seen, a
  crate whose source is not fetched, or a chosen license with no text
  anywhere in the tree is an error, never a silently omitted notice.
  `deny.toml` decides what is *allowed*; this decides what is *shipped*,
  and the two lists are checked against each other on every run.

* **It is host-independent.** The component list is not filtered by target,
  so a Windows and a Linux checkout produce the same bytes and CI's
  ``--check`` means something. That makes it slightly over-inclusive —
  `windows-sys` is listed on Linux — which is the correct direction to err
  for attribution, and it matches what the repository ships to everyone.

* **License texts are deduplicated by content, not by identifier.** MIT
  requires each component's own copyright line; crates whose text is
  byte-identical share one entry, crates whose copyright differs do not.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUTPUT = REPO / "THIRD-PARTY-LICENSES.txt"
DENY = REPO / "deny.toml"

# The package whose dependency closure is actually distributed. Everything
# else in the workspace is a library it links, or a bench/fuzz target that
# ships to nobody.
ROOT_PACKAGE = "mwl-cli"
BINARY = "mwl"

# Which half of a dual license MWL takes, most-preferred first. MIT leads
# because MWL is MIT: one license covering the most components keeps the
# shipped notice short, which is the whole reason a preference order exists
# rather than "reproduce every offered license".
#
# An identifier absent from this list is an error. That is deliberate — a
# new license entering the tree should be a decision someone makes, not a
# line that appears in a generated file.
#
# This must hold exactly the same identifiers as `deny.toml`'s `licenses.allow`,
# only ordered: that file decides what MWL may *link*, this decides what it
# ships and under which half of a choice. `check_policies_agree` below fails
# if the two ever drift, in either direction — a license allowed but unranked
# would have no notice policy, and one ranked but not allowed would be a
# preference for something CI already refuses.
PREFERENCE = [
    "MIT",
    "Apache-2.0 WITH LLVM-exception",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "CC0-1.0",
    "MPL-2.0",
    # Last on purpose. `Unlicense OR MIT` is how the `regex` family and its
    # dependencies are offered, and a public-domain dedication imposes nothing
    # on MWL — but ranking it below MIT means MIT is always the half taken, so
    # the shipped notice stays one license shorter.
    "Unlicense",
]

# Content fingerprints, checked in order. Classifying a license file by what
# it says rather than by what it is called is what lets this script cope
# with LICENSE, LICENSE-MIT, license-mit and LICENSE.txt all meaning the
# same thing — and with a file called LICENSE that turns out to be Apache.
#
# Order is most-specific-first, and it matters: the Unicode license opens
# with MIT's "Permission is hereby granted, free of charge" and would
# otherwise be filed as MIT, silently dropping a notice we owe.
FINGERPRINTS = [
    ("Apache-2.0 WITH LLVM-exception", ("apache license", "llvm exception")),
    ("Apache-2.0", ("apache license", "version 2.0")),
    ("Unicode-3.0", ("unicode license v3",)),
    ("Unlicense", ("this is free and unencumbered software released into the public domain",)),
    ("ISC", ("permission to use, copy, modify, and/or distribute this software",)),
    ("Zlib", ("altered source versions must be plainly marked as such",)),
    ("MPL-2.0", ("mozilla public license version 2.0",)),
    ("CC0-1.0", ("creative commons legal code", "cc0 1.0 universal")),
    # BSD-3 before BSD-2: the 3-clause text contains the 2-clause text.
    ("BSD-3-Clause", ("redistribution and use in source and binary forms", "endorse or promote")),
    ("BSD-2-Clause", ("redistribution and use in source and binary forms",)),
    ("MIT", ("permission is hereby granted, free of charge",)),
]

LICENSE_FILE_PREFIXES = ("license", "licence", "copying", "unlicense")

WIDTH = 78


# ---------------------------------------------------------------------------
# SPDX expressions
# ---------------------------------------------------------------------------


def parse_spdx(expr: str):
    """Parse an SPDX expression into a nested ``(op, [terms])`` tree.

    Only the three operators Cargo manifests actually use are handled — OR,
    AND and WITH — plus parentheses. `WITH` binds tightest and is folded
    into the identifier it qualifies, because that is how the rest of this
    script (and `deny.toml`) names those licenses.
    """
    # Some crates still use the pre-SPDX `MIT/Apache-2.0` spelling.
    tokens = expr.replace("/", " OR ").replace("(", " ( ").replace(")", " ) ").split()
    pos = 0

    def peek():
        return tokens[pos] if pos < len(tokens) else None

    def primary():
        nonlocal pos
        if peek() == "(":
            pos += 1
            node = expression()
            if peek() != ")":
                raise ValueError(f"unbalanced parentheses in {expr!r}")
            pos += 1
        else:
            token = peek()
            if token is None or token in ("AND", "OR", ")"):
                raise ValueError(f"expected a license identifier in {expr!r}")
            pos += 1
            node = ("id", token)
        if peek() == "WITH":
            pos += 1
            exception = peek()
            if exception is None:
                raise ValueError(f"dangling WITH in {expr!r}")
            pos += 1
            if node[0] != "id":
                raise ValueError(f"WITH applied to a compound expression in {expr!r}")
            node = ("id", f"{node[1]} WITH {exception}")
        return node

    def conjunction():
        nonlocal pos
        terms = [primary()]
        while peek() == "AND":
            pos += 1
            terms.append(primary())
        return terms[0] if len(terms) == 1 else ("and", terms)

    def expression():
        nonlocal pos
        terms = [conjunction()]
        while peek() == "OR":
            pos += 1
            terms.append(conjunction())
        return terms[0] if len(terms) == 1 else ("or", terms)

    tree = expression()
    if pos != len(tokens):
        raise ValueError(f"trailing tokens in {expr!r}")
    return tree


class Undeclared(Exception):
    """An SPDX identifier `PREFERENCE` ranks nowhere.

    Raised rather than exited on so an OR can decline one branch and still
    take another — see `take`. Only `choose` turns it into a message.
    """

    def __init__(self, ident: str):
        super().__init__(ident)
        self.ident = ident


def choose(node, where: str) -> list[str]:
    """Reduce an SPDX tree to the licenses MWL actually takes.

    An OR collapses to the single most-preferred option; an AND keeps every
    conjunct, because that is what "and" means — `(MIT OR Apache-2.0) AND
    BSD-3-Clause` obliges us to reproduce two notices, not one.
    """
    try:
        return take(node)
    except Undeclared as undeclared:
        raise SystemExit(
            f"error: {where} offers {undeclared.ident!r}, which tools/gen-attribution.py has no\n"
            f"       policy for. Add it to PREFERENCE (and to deny.toml's allow list)\n"
            f"       once someone has decided MWL may ship under it."
        ) from None


def take(node) -> list[str]:
    """`choose`'s recursion, raising `Undeclared` instead of exiting."""
    kind = node[0]
    if kind == "id":
        ident = node[1]
        if ident not in PREFERENCE:
            raise Undeclared(ident)
        return [ident]
    if kind == "and":
        out: list[str] = []
        for term in node[1]:
            for ident in take(term):
                if ident not in out:
                    out.append(ident)
        return out
    # An OR: MWL takes exactly one branch, so a branch it has no policy for is
    # one it simply declines. `r-efi`, which `getrandom` brings in for the UEFI
    # target, is offered as `MIT OR Apache-2.0 OR LGPL-2.1-or-later` — MWL
    # takes MIT, and the LGPL half never enters the tree for anyone to have to
    # decide about. This is cargo-deny's own reading of an OR, which is what
    # keeps this file and `deny.toml` agreeing rather than this one being
    # quietly the stricter of the two.
    #
    # An OR with **no** understood branch still fails, and that is where the
    # "someone decides" gate belongs: there the license genuinely is one MWL
    # would have to ship under.
    options: list[list[str]] = []
    declined: list[Undeclared] = []
    for term in node[1]:
        try:
            options.append(take(term))
        except Undeclared as undeclared:
            declined.append(undeclared)
    if not options:
        raise declined[0]
    return min(options, key=lambda opt: min(PREFERENCE.index(i) for i in opt))


# ---------------------------------------------------------------------------
# The dependency graph
# ---------------------------------------------------------------------------


def cargo_metadata() -> dict:
    proc = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=REPO,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if proc.returncode != 0:
        raise SystemExit(f"error: cargo metadata failed:\n{proc.stderr.strip()}")
    return json.loads(proc.stdout)


def shipped_packages(meta: dict) -> list[dict]:
    """Every third-party package reachable from the `mwl` binary.

    Normal and build dependencies only: a dev-dependency (criterion, insta,
    proptest) is never linked into anything a user receives, so attributing
    it would overstate what MWL distributes.
    """
    by_id = {pkg["id"]: pkg for pkg in meta["packages"]}
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}
    roots = [p["id"] for p in meta["packages"] if p["name"] == ROOT_PACKAGE]
    if not roots:
        raise SystemExit(f"error: no package named {ROOT_PACKAGE} in this workspace")

    seen: set[str] = set()
    stack = list(roots)
    while stack:
        pkg_id = stack.pop()
        if pkg_id in seen:
            continue
        seen.add(pkg_id)
        for dep in nodes[pkg_id]["deps"]:
            kinds = {k["kind"] for k in dep["dep_kinds"]}
            if kinds & {None, "build"}:
                stack.append(dep["pkg"])

    # `source: null` marks a path dependency — the workspace's own crates,
    # covered by LICENSE, not by this file.
    external = [by_id[i] for i in seen if by_id[i].get("source") is not None]
    return sorted(external, key=lambda p: (p["name"].lower(), p["version"]))


# ---------------------------------------------------------------------------
# License texts
# ---------------------------------------------------------------------------


def read_text(path: Path) -> str:
    """Read a license file without ever corrupting its copyright line.

    Almost every crate ships UTF-8, but a handful still ship Latin-1 — and
    the byte that differs is invariably the © in the copyright notice, which
    is precisely the part attribution exists to reproduce. Decoding with
    ``errors="replace"`` would turn it into U+FFFD and call that a success,
    so the fallback is a codec that cannot fail instead.
    """
    raw = path.read_bytes()
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return raw.decode("cp1252", errors="replace")


def classify(text: str) -> str | None:
    lowered = text.lower()
    for ident, needles in FINGERPRINTS:
        if all(needle in lowered for needle in needles):
            return ident
    return None


def read_license_files(directory: Path) -> tuple[dict[str, str], list[tuple[str, str]]]:
    """Return ``({spdx: text}, [(filename, text)])`` for one crate directory.

    The second element is every NOTICE file found. Apache-2.0 § 4(d) makes
    reproducing those mandatory, and unlike the license text they are never
    deduplicated away.
    """
    texts: dict[str, str] = {}
    notices: list[tuple[str, str]] = []
    try:
        entries = sorted(os.listdir(directory))
    except OSError as err:
        raise SystemExit(
            f"error: cannot read {directory}: {err}\n"
            f"       Run `cargo fetch` so every dependency's source is on disk."
        )
    for entry in entries:
        path = directory / entry
        if not path.is_file():
            continue
        lowered = entry.lower()
        if lowered.startswith("notice"):
            notices.append((entry, read_text(path)))
            continue
        if not lowered.startswith(LICENSE_FILE_PREFIXES):
            continue
        text = read_text(path)
        ident = classify(text)
        # First file wins per identifier: entries are sorted, so the choice
        # is deterministic when a crate ships the same license twice.
        if ident is not None and ident not in texts:
            texts[ident] = text
    return texts, notices


def normalize(text: str) -> str:
    """Line endings and trailing whitespace only.

    Enough that a CRLF checkout and an LF one produce the same file; never
    enough to alter a word of the license itself.
    """
    return "\n".join(line.rstrip() for line in text.replace("\r\n", "\n").split("\n")).strip()


def deny_allowlist() -> set[str]:
    """The `allow` list from deny.toml, read without a TOML parser.

    A five-line reader beats adding a dependency to the one script whose
    entire job is keeping the dependency set honest.
    """
    if not DENY.exists():
        return set()
    allowed: set[str] = set()
    inside = False
    for line in DENY.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("allow") and "=" in stripped and "[" in stripped:
            inside = True
            continue
        if inside:
            if stripped.startswith("]"):
                break
            value = stripped.split("#", 1)[0].strip().rstrip(",").strip()
            if len(value) >= 2 and value[0] == value[-1] == '"':
                allowed.add(value[1:-1])
    return allowed


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------


def rule(char: str = "-") -> str:
    return char * WIDTH


def render(components: list[dict], groups: list[dict], notices: list[tuple[str, str, str]]) -> str:
    out: list[str] = []
    add = out.append

    add("MWL — third-party attribution")
    add(rule("="))
    add("")
    add("This file is the complete notice for third-party components compiled into")
    add(f"the `{BINARY}` binary. MWL's own terms are MIT and live in LICENSE; they are")
    add("not restated here.")
    add("")
    add("GENERATED FILE — do not edit by hand. Regenerate with:")
    add("")
    add("    python tools/gen-attribution.py")
    add("")
    add("")
    # `mwl info` slices this file at the two section headings below and
    # prints the parts verbatim, so the wording that explains a section
    # belongs inside it rather than in the preamble above — the preamble is
    # about the file, and a shipped binary has no file to talk about.
    # crates/mwl-cli/src/info.rs owns the other half of that agreement.
    add(f"COMPONENTS ({len(components)})")
    add(rule())
    add("")
    add("Where a component offers a choice of licenses, MWL takes the one shown and")
    add("the full offer follows in parentheses. Components are listed for every")
    add("platform MWL builds for, so one may appear that a given build omits.")
    add("")

    name_w = max(len(c["name"]) for c in components)
    version_w = max(len(c["version"]) for c in components)
    taken_w = max(len(c["taken"]) for c in components)
    for comp in components:
        offered = "" if comp["offered"] == comp["taken"] else f"  ({comp['offered']})"
        add(
            f"  {comp['name']:<{name_w}}  {comp['version']:<{version_w}}  "
            f"{comp['taken']:<{taken_w}}{offered}".rstrip()
        )

    add("")
    add("")
    add(f"LICENSE TEXTS ({len(groups)})")
    add(rule())
    add("")
    add("Each text below is reproduced verbatim as its component ships it. Components")
    add("whose text is byte-identical share one entry; components whose copyright")
    add("lines differ do not, which is why one license identifier can appear more")
    add("than once.")
    add("")

    for index, group in enumerate(groups, start=1):
        add("")
        add(rule("="))
        add(f"[{index}/{len(groups)}]  {group['license']}")
        add(rule("="))
        add("")
        add("Applies to:")
        for name in group["components"]:
            add(f"  {name}")
        add("")
        add(rule())
        add("")
        add(group["text"])
        add("")

    if notices:
        add("")
        add(f"NOTICE FILES ({len(notices)})")
        add(rule())
        add("")
        add("Reproduced as required by Apache License 2.0 § 4(d).")
        for component, filename, text in notices:
            add("")
            add(rule("="))
            add(f"{component} — {filename}")
            add(rule("="))
            add("")
            add(text)
            add("")

    return "\n".join(out).rstrip() + "\n"


# ---------------------------------------------------------------------------


def check_policies_agree(allowed: set[str]) -> None:
    """Fails if PREFERENCE and deny.toml's allow list have drifted apart.

    Two lists naming the same thing is exactly the duplication AGENTS.md
    warns about, and they cannot be merged — `deny.toml` is cargo-deny's
    format and carries no ordering. Checking them against each other on
    every run is the next best thing, and it catches the realistic mistake:
    a license added to one file and not the other.
    """
    if not allowed:
        return
    unranked = sorted(allowed - set(PREFERENCE))
    unallowed = sorted(set(PREFERENCE) - allowed)
    problems = []
    if unranked:
        problems.append(
            f"allowed by deny.toml but absent from PREFERENCE: {', '.join(unranked)}"
        )
    if unallowed:
        problems.append(
            f"ranked in PREFERENCE but not allowed by deny.toml: {', '.join(unallowed)}"
        )
    if problems:
        detail = "\n".join(f"  - {problem}" for problem in problems)
        raise SystemExit(
            f"error: the two license policies disagree:\n{detail}\n"
            f"       Both files must name the same identifiers; only the order differs."
        )


def build() -> str:
    meta = cargo_metadata()
    packages = shipped_packages(meta)
    allowed = deny_allowlist()
    check_policies_agree(allowed)

    components: list[dict] = []
    # Keyed by the license text itself, so identical texts collapse and a
    # differing copyright line keeps its own entry.
    by_text: dict[tuple[str, str], list[str]] = {}
    # Crates that ship no license file of their own, resolved in a second
    # pass against the text a sibling crate under the same license ships.
    pending: list[tuple[str, str]] = []
    known_text: dict[str, str] = {}
    notices: list[tuple[str, str, str]] = []
    problems: list[str] = []

    for pkg in packages:
        label = f"{pkg['name']} {pkg['version']}"
        expr = pkg.get("license")
        if not expr:
            # `license-file` instead of `license`: rare, and it means the
            # crate's terms are not an SPDX expression at all.
            problems.append(f"{label}: no `license` field in its manifest")
            continue
        taken = choose(parse_spdx(expr), label)
        components.append(
            {
                "name": pkg["name"],
                "version": pkg["version"],
                "taken": " AND ".join(taken),
                "offered": expr,
            }
        )

        for ident in taken:
            if allowed and ident not in allowed:
                problems.append(f"{label}: {ident} is not in deny.toml's allow list")

        directory = Path(pkg["manifest_path"]).parent
        texts, crate_notices = read_license_files(directory)
        for filename, text in crate_notices:
            notices.append((label, filename, normalize(text)))

        for ident in taken:
            text = texts.get(ident)
            if text is None:
                pending.append((label, ident))
                continue
            body = normalize(text)
            known_text.setdefault(ident, body)
            by_text.setdefault((ident, body), []).append(label)

    # Second pass. A crate that ships no license file is common in a
    # multi-crate repository where only the root carries one; borrowing the
    # sibling's text is exact for a license with no per-crate copyright line
    # (Apache-2.0), and is why this is an error for anything else.
    for label, ident in pending:
        text = known_text.get(ident)
        if text is None:
            problems.append(
                f"{label}: takes {ident} but ships no license file, and no other "
                f"component in the tree ships one for {ident} either"
            )
            continue
        by_text[(ident, text)].append(label)

    if problems:
        message = "\n".join(f"  - {problem}" for problem in problems)
        raise SystemExit(f"error: attribution is incomplete:\n{message}")

    groups = [
        {"license": ident, "text": text, "components": sorted(names, key=str.lower)}
        for (ident, text), names in by_text.items()
    ]
    groups.sort(key=lambda g: (PREFERENCE.index(g["license"]), g["components"][0].lower()))
    notices.sort()

    return render(components, groups, notices)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the committed file is out of date, writing nothing",
    )
    args = parser.parse_args()

    generated = build()

    if args.check:
        if not OUTPUT.exists():
            print(f"error: {OUTPUT.name} does not exist", file=sys.stderr)
            return 1
        current = OUTPUT.read_text(encoding="utf-8").replace("\r\n", "\n")
        if current != generated:
            print(
                f"error: {OUTPUT.name} is out of date with Cargo.lock.\n"
                f"       Run `python tools/gen-attribution.py` and commit the result.",
                file=sys.stderr,
            )
            return 1
        print(f"{OUTPUT.name} is up to date")
        return 0

    OUTPUT.write_text(generated, encoding="utf-8", newline="\n")
    lines = generated.count("\n")
    print(f"wrote {OUTPUT.name} ({len(generated):,} bytes, {lines:,} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
