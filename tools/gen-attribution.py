#!/usr/bin/env python3
"""Generate THIRD-PARTY-LICENSES.txt from the resolved dependency graph.

Novis ships as MIT, and every permissive license in its dependency tree asks
for the same thing in return: reproduce the notice with the binary. This
script produces the one file that satisfies that, for both the repository
and — via `include_str!` in `nvs-cli` — the shipped `nvs` binary itself.
`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand` owns the policy; this file only implements it.

    python tools/gen-attribution.py            # regenerate
    python tools/gen-attribution.py --check    # exit 1 if stale (CI)

It carries a second, related gate over the same graph, because the graph is
already resolved here and reading it twice in two scripts is the duplication
this repository does not keep: `--check-c-deps` enumerates the default
binary's C dependencies and fails on one `rule:packaging/a-c-dependency-answers-two-questions` has no record for.
M8's verification list asks for that check by name; `C_DEPENDENCIES` below is
the ledger, and `rule:packaging/a-c-dependency-answers-two-questions` owns the two questions it answers.

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
ROOT_PACKAGE = "nvs-cli"
BINARY = "nvs"

# Which half of a dual license Novis takes, most-preferred first. MIT leads
# because Novis is MIT: one license covering the most components keeps the
# shipped notice short, which is the whole reason a preference order exists
# rather than "reproduce every offered license".
#
# An identifier absent from this list is an error. That is deliberate — a
# new license entering the tree should be a decision someone makes, not a
# line that appears in a generated file.
#
# This must hold exactly the same identifiers as `deny.toml`'s `licenses.allow`,
# only ordered: that file decides what Novis may *link*, this decides what it
# ships and under which half of a choice. `check_policies_agree` below fails
# if the two ever drift, in either direction — a license allowed but unranked
# would have no notice policy, and one ranked but not allowed would be a
# preference for something CI already refuses.
PREFERENCE = [
    "MIT",
    # MIT with the attribution clause struck out, which is how `borrow-or-share`
    # is offered. Ranked next to MIT rather than beside `Unlicense` because the
    # text is MIT's, so a notice reproducing it costs nothing a reader has not
    # already read — and it is never a choice anyway: no crate here offers it as
    # one half of a dual license.
    "MIT-0",
    "Apache-2.0 WITH LLVM-exception",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "CC0-1.0",
    # The Community Data License Agreement's permissive variant, which is what
    # `webpki-roots` offers Mozilla's CA set under. A *data* license and not a
    # code one: 2.0 drops even the notice requirement 1.0 had, so this is here
    # to be reproduced rather than because it demands to be.
    "CDLA-Permissive-2.0",
    "MPL-2.0",
    # Last on purpose. `Unlicense OR MIT` is how the `regex` family and its
    # dependencies are offered, and a public-domain dedication imposes nothing
    # on Novis — but ranking it below MIT means MIT is always the half taken, so
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
    ("CDLA-Permissive-2.0", ("community data license agreement", "permissive")),
    # BSD-3 before BSD-2: the 3-clause text contains the 2-clause text.
    ("BSD-3-Clause", ("redistribution and use in source and binary forms", "endorse or promote")),
    ("BSD-2-Clause", ("redistribution and use in source and binary forms",)),
    # MIT-0 before MIT: "MIT No Attribution" is MIT's text with the attribution
    # clause struck out, so it matches MIT's fingerprint as well and would
    # otherwise be filed as MIT -- a notice claiming a condition the license
    # does not actually impose.
    ("MIT-0", ("mit no attribution",)),
    ("MIT", ("permission is hereby granted, free of charge",)),
]

LICENSE_FILE_PREFIXES = ("license", "licence", "copying", "unlicense")

WIDTH = 78


# ---------------------------------------------------------------------------
# The C-dependency ledger — `rule:packaging/a-c-dependency-answers-two-questions`
# ---------------------------------------------------------------------------

# Build dependencies that mean "this crate compiles or links C". A crate that
# pulls one of these into the shipped graph is building something that is not
# Rust; a crate that declares `links` is claiming a native library outright.
# Between them these two signals catch every way C enters the default binary
# without asking anyone to maintain a list of `-sys` name suffixes.
#
# `cc` and `cmake` also compile assembly and C++, which is the same question
# for this ledger's purposes: it is memory-unsafe code the Rust toolchain did
# not check.
C_BUILD_TOOLS = frozenset({"cc", "cmake", "pkg-config", "bindgen", "nasm-rs", "meson"})

# The three verdicts an entry may carry, which are `rule:packaging/a-c-dependency-answers-two-questions`'s two questions
# plus the case where the question does not arise:
#
# * `no-native-code` — a signal above fired but this tree builds nothing
#   native from the crate. Either `links` is Cargo's one-version token rather
#   than a library, or the feature selection that turns the C off is named in
#   `requires` below and checked.
# * `unreachable` — § 4's question 1 is *no*: attacker-controlled data does not
#   reach the code, so it is accepted under ordinary audit.
# * `verified` — question 1 is *yes*, and the entry names the demonstrable,
#   exceptional verification record question 2 asks for. § 4 credits SQLite
#   with one and says almost nothing else clears the bar; a crate that cannot
#   show one is confined to wasm, which means it never reaches this ledger.
VERDICTS = ("no-native-code", "unreachable", "verified")

# The ledger itself: every C dependency of the default binary, answered against
# § 4 rather than argued case by case. The value is
# ``(verdict, required features, the record)``.
#
# `requires` is what makes a `no-native-code` verdict hold over time: the named
# features must still be active in the resolved graph, so a crate whose C half
# is off by a feature flag cannot have it switched back on without this gate
# failing. `cargo metadata` reports a build dependency whether or not the
# feature that uses it is on, which is why the flag is checked here rather than
# left to change the enumeration.
#
# Each record is one or two sentences and points at the home of the decision —
# `Cargo.toml`'s own dependency comment, which is where the crate was chosen —
# rather than restating it.
#
# `--check-c-deps` fails on a C dependency with no entry here **and** on an
# entry naming a crate the tree no longer builds, so the ledger cannot drift in
# either direction. Adding an entry is a decision someone makes, deliberately
# in the same shape as PREFERENCE above: the gate is that a human wrote the
# sentence, not that a tool could infer it.
C_DEPENDENCIES: dict[str, tuple[str, tuple[str, ...], str]] = {
    "ring": (
        "verified",
        (),
        "The one genuine C dependency in the default binary: BoringSSL's "
        "pregenerated assembly behind a Rust API. It is rustls's crypto "
        "provider, and `nvs-stdlib` reaches it directly for JWS's four "
        "signature algorithms — RSASSA-PKCS1-v1_5, RSASSA-PSS, ECDSA over P-256 "
        "and Ed25519. Question 1 is yes on both counts — a TLS record layer and "
        "a token verifier are each exactly where attacker bytes land — and "
        "question 2 is answered once for both by OSS-Fuzz and BoringSSL's "
        "formally verified field arithmetic. Cargo.toml's `rustls` comment is "
        "the home of the transport decision, including why the wasm branch is "
        "not available to a client that owns its socket, and its `ring` row is "
        "the home of the signature one.",
    ),
    "libsqlite3-sys": (
        "verified",
        ("bundled",),
        "SQLite itself, compiled from the amalgamation by `rusqlite`'s sys crate. "
        "Question 1 is yes -- a database engine parses SQL an application composed "
        "and stores bytes a request supplied -- and question 2 is the one record "
        "`rule:core-api/tier-placement` section 4 names by hand: TH3, 100% MC/DC branch coverage over the "
        "whole library, plus a continuous fuzzing corpus and the anomaly log SQLite "
        "publishes against every release. `bundled` is required rather than "
        "incidental, and Cargo.toml's `rusqlite` comment is the home of why -- a "
        "host's own libsqlite3 is a different build of a different version, and the "
        "verification record belongs to the one this tree compiles.",
    ),
    "sqlite-wasm-rs": (
        "no-native-code",
        (),
        "`libsqlite3-sys`'s wasm32 half, and a target-gated dependency this tree "
        "never compiles: it is reached only under `cfg(target_arch = \"wasm32\")`, "
        "and Novis's own wasm target is the extension sandbox, which links no "
        "database at all. It is in the graph for the same reason `windows-sys` is "
        "listed on Linux -- this enumeration is deliberately host-independent, so "
        "a Windows and a Linux checkout produce the same bytes. The engine this "
        "binary actually runs is `libsqlite3-sys`'s bundled amalgamation, "
        "immediately above.",
    ),
    "blake3": (
        "no-native-code",
        ("pure",),
        "Ships hand-written assembly built through `cc` by default, and this tree "
        "takes `default-features = false` with `pure` instead, so nothing native "
        "is compiled. Cargo.toml's `blake3` comment is the home of why: the "
        "portable implementation is already faster than SHA-256, so the pure-Rust "
        "default costs nothing worth spending an exception on.",
    ),
    "defmt": (
        "no-native-code",
        (),
        "`links = \"defmt\"` is Cargo's one-version token, not a native library — "
        "the crate is a logging framework for embedded targets and compiles no C. "
        "It is in the graph only because this enumeration is host-independent, "
        "the same way `windows-sys` is listed on Linux.",
    ),
    "wasm-bindgen-shared": (
        "no-native-code",
        (),
        "`links = \"wasm_bindgen\"` is the same one-version token as `defmt`'s, "
        "used to keep the macro and the runtime at one version. It builds no C, "
        "and it is reached only through a `cfg(target_arch = \"wasm32\")` "
        "dependency that no shipped `nvs` binary compiles.",
    ),
}


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
    """Reduce an SPDX tree to the licenses Novis actually takes.

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
            f"       once someone has decided Novis may ship under it."
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
    # An OR: Novis takes exactly one branch, so a branch it has no policy for is
    # one it simply declines. `r-efi`, which `getrandom` brings in for the UEFI
    # target, is offered as `MIT OR Apache-2.0 OR LGPL-2.1-or-later` — Novis
    # takes MIT, and the LGPL half never enters the tree for anyone to have to
    # decide about. This is cargo-deny's own reading of an OR, which is what
    # keeps this file and `deny.toml` agreeing rather than this one being
    # quietly the stricter of the two.
    #
    # An OR with **no** understood branch still fails, and that is where the
    # "someone decides" gate belongs: there the license genuinely is one Novis
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
    """Every third-party package reachable from the `nvs` binary.

    Normal and build dependencies only: a dev-dependency (criterion, insta,
    proptest) is never linked into anything a user receives, so attributing
    it would overstate what Novis distributes.
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

    add("Novis — third-party attribution")
    add(rule("="))
    add("")
    add("This file is the complete notice for third-party components compiled into")
    add(f"the `{BINARY}` binary. Novis's own terms are MIT and live in LICENSE; they are")
    add("not restated here.")
    add("")
    add("GENERATED FILE — do not edit by hand. Regenerate with:")
    add("")
    add("    python tools/gen-attribution.py")
    add("")
    add("")
    # `nvs info` slices this file at the two section headings below and
    # prints the parts verbatim, so the wording that explains a section
    # belongs inside it rather than in the preamble above — the preamble is
    # about the file, and a shipped binary has no file to talk about.
    # crates/nvs-cli/src/info.rs owns the other half of that agreement.
    add(f"COMPONENTS ({len(components)})")
    add(rule())
    add("")
    add("Where a component offers a choice of licenses, Novis takes the one shown and")
    add("the full offer follows in parentheses. Components are listed for every")
    add("platform Novis builds for, so one may appear that a given build omits.")
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


def c_dependencies(meta: dict, packages: list[dict]) -> list[tuple[str, str, str]]:
    """Every shipped package that compiles or links C, and why it counts.

    Returns ``(name, version, signal)``, sorted, over the same package set
    the attribution notice is built from — so this reads the default
    binary's own graph rather than a hand-kept list, and it is
    host-independent for the same reason `shipped_packages` is.

    The third element is the *signal*, not a finding: whether the crate
    actually compiles C here is what its ledger entry answers, and both
    signals fire on crates that turn out not to. A crate matching both is
    reported by `links`, which is the stronger claim.
    """
    by_id = {pkg["id"]: pkg for pkg in meta["packages"]}
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}

    found: list[tuple[str, str, str]] = []
    for pkg in packages:
        links = pkg.get("links")
        if links:
            found.append((pkg["name"], pkg["version"], f"declares `links = \"{links}\"`"))
            continue
        tools = sorted(
            {
                by_id[dep["pkg"]]["name"]
                for dep in nodes[pkg["id"]]["deps"]
                if any(kind["kind"] == "build" for kind in dep["dep_kinds"])
                and by_id[dep["pkg"]]["name"] in C_BUILD_TOOLS
            }
        )
        if tools:
            joined = ", ".join(f"`{tool}`" for tool in tools)
            found.append((pkg["name"], pkg["version"], f"build-depends on {joined}"))
    return sorted(found)


def check_c_deps() -> int:
    """`rule:packaging/a-c-dependency-answers-two-questions`'s standing test, as a gate over the resolved graph.

    M8's verification list asks for a check "enumerating the default
    binary's C dependencies, failing on any addition not recorded against
    `rule:packaging/a-c-dependency-answers-two-questions`'s two questions". That is this: the enumeration comes from
    `cargo metadata`, the record comes from `C_DEPENDENCIES`, and the two
    are compared in both directions.
    """
    meta = cargo_metadata()
    packages = shipped_packages(meta)
    found = c_dependencies(meta, packages)
    active = {
        pkg["name"]: set(node["features"])
        for node in meta["resolve"]["nodes"]
        for pkg in packages
        if pkg["id"] == node["id"]
    }
    present = {name for name, _, _ in found}
    problems: list[str] = []

    print(
        f"C dependencies of the `{BINARY}` binary: "
        f"{len(found)} in the graph, {len(C_DEPENDENCIES)} recorded"
    )
    for name, version, reason in found:
        recorded = C_DEPENDENCIES.get(name)
        if recorded is None:
            print(f"  {name} {version} — {reason} [NOT RECORDED]")
            problems.append(
                f"  - {name} {version} {reason}, and nothing in this file records it.\n"
                f"    Answer `rule:packaging/a-c-dependency-answers-two-questions`'s two questions — does attacker-controlled data\n"
                f"    reach it, and if so what is its verification record — and add the\n"
                f"    entry to C_DEPENDENCIES in tools/gen-attribution.py, or confine the\n"
                f"    code to wasm as § 4's second question requires."
            )
            continue
        verdict, requires, _record = recorded
        print(f"  {name} {version} — {reason} [{verdict}]")
        if verdict not in VERDICTS:
            problems.append(
                f"  - {name} is recorded with the verdict {verdict!r}, which is not one\n"
                f"    of {', '.join(VERDICTS)}."
            )
        missing = [feature for feature in requires if feature not in active.get(name, set())]
        if missing:
            problems.append(
                f"  - {name}'s record holds only while it is built with "
                f"{', '.join(repr(f) for f in requires)},\n"
                f"    and {', '.join(repr(f) for f in missing)} is no longer active. Either restore\n"
                f"    the feature in Cargo.toml or answer `rule:packaging/a-c-dependency-answers-two-questions` for the native code\n"
                f"    it now compiles."
            )

    for name in sorted(C_DEPENDENCIES):
        if name not in present:
            problems.append(
                f"  - {name} is recorded in C_DEPENDENCIES but nothing in the graph\n"
                f"    signals it any more. Delete the entry: a ledger of dependencies\n"
                f"    that left is a ledger nobody trusts."
            )

    if not problems:
        return 0

    sys.stdout.flush()
    print("error: the C-dependency ledger is out of date:", file=sys.stderr)
    print("\n".join(problems), file=sys.stderr)
    return 1


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
    parser.add_argument(
        "--check-c-deps",
        action="store_true",
        help="list the default binary's C dependencies; exit non-zero on one `rule:packaging/a-c-dependency-answers-two-questions` has no record for",
    )
    args = parser.parse_args()

    if args.check_c_deps:
        return check_c_deps()

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
