#!/usr/bin/env python3
"""The release cycle, as one command per step.

`.github/workflows/release.yml` is the schedule; this file is every decision it makes. The
workflow holds no version arithmetic, no changelog rules and no packaging logic, for the reason
`nv verify` holds the verification steps: YAML that computes things can only be tested by
pushing it, and a release is the one pipeline you cannot afford to debug in production.

Everything here runs locally against a clone. `--preview` renders the exact notes and the exact
version a dispatch would produce, and writes nothing:

    python tools/release.py --preview major

## The four steps, in the order the workflow runs them

    python tools/release.py --plan major --github-output    # 1. the version math, no writes
    python tools/release.py --notes 0.1.0 --out notes.md    # 2. render the changelog section
    python tools/release.py --package ... --out dist/       # 3. archive a built binary
    python tools/release.py --apply 0.1.0 --notes notes.md  # 4. rewrite the manifests, prepend

`--check` is the gate: the workspace version, the fourteen `[workspace.dependencies]` pins and
the latest tag all agree, and `CHANGELOG.md` has a section for the version on disk.

## The version scheme is not plain SemVer, and this is not the place that decides it

`rule:packaging/below-1-0-the-breaking-slot-moves-left` owns it: one
number for the whole workspace, and **before 1.0 the breaking slot moves left by one** -- `0.MINOR`
carries breaking changes and `0.MINOR.PATCH` is always compatible. So `--plan major` on 0.0.1
produces **0.1.0**, not 1.0.0, and below 1.0 `minor` and `patch` are deliberately the same
increment because there is no third slot to put a compatible feature in.

That is also why `--plan` refuses to reach 0.1.0 without `--allow-contract`. `rule:packaging/the-version-contract-starts-at-0-1-0` makes
0.1.0 the release that throws the switch from the prototyping regime to the version contract --
"the switch is thrown once, in the commit that tags 0.1.0" -- and a switch thrown by a dropdown
nobody read is exactly the failure that ADR is written against.

## What a version number lives in

Fifteen places in one file, which is why this exists rather than a `sed`. `[workspace.package]`
carries the version every crate inherits, and `[workspace.dependencies]` re-states it on each of
the fourteen `nvs-*` path dependencies because a path dependency needs a version to be
publishable. `Cargo.lock` then records all sixteen crates again -- the fifteen under `crates/`
plus `nvs-abi-probe` under `benches/`, which is exactly the entry a hand-written `sed` over
`crates/` would have missed. `cargo update --workspace --offline` is what
rewrites the lock: it touches workspace members only, so a release cannot smuggle in a
dependency bump -- that pass is a human's, fired by hand, per
[docs/agent/dependency-update.md](../docs/agent/dependency-update.md).

## Why the notes are compacted rather than complete

`git log` here is written by an unattended loop and is roughly half bookkeeping: `docs(agent)`
handoffs, `docs(loop)` chain edits, `test(stdlib)` case runs. A release body listing all of it is
a release body nobody reads. So `feat`, `fix` and `perf` are listed in full and grouped by scope,
anything marked breaking is lifted to the top, and every other conventional type collapses into
one *Internal* line with per-type counts. Nothing is dropped silently: the counts add up to the
commit total, and the compare link at the foot reaches the rest.

A subject that does not parse as a conventional commit is listed in full under *Other* rather
than swept into the counts, because the two such commits in this history ("added logos") are
exactly the ones a rule would hide by accident.

## Why this never publishes

`--apply` writes the tree and stops. Tagging, pushing and the GitHub draft are the workflow's,
because they need credentials and this file must be runnable by anyone with a clone and no
secrets at all. There is no code path here that reads a token.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tarfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"
CHANGELOG = ROOT / "CHANGELOG.md"

# Commit subjects in this history contain `↔`, `→` and `§`, and a Windows console is cp1252 by
# default -- so printing the rendered notes there died with a UnicodeEncodeError while the same
# code was fine on a runner. The notes are UTF-8 by construction; say so on the way out rather
# than letting the terminal's codepage decide whether a release can be previewed.
for _stream in (sys.stdout, sys.stderr):
    if hasattr(_stream, "reconfigure"):
        _stream.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]

# Shipped beside the binary in every archive. THIRD-PARTY-LICENSES.txt is not optional --
# `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand` makes it the notice this project owes at distribution, and an archive is
# distribution. A file missing here fails the packaging step rather than shipping without it.
ARCHIVE_EXTRAS = ("README.md", "LICENSE", "THIRD-PARTY-LICENSES.txt", "CHANGELOG.md")

CHANGELOG_HEADER = """# Changelog

Every released version of Novis, newest first. Generated from the commit log by
`tools/release.py` and prepended by the release workflow -- edit a section only to correct it,
never to add one by hand.

What a version number promises is `rule:packaging/the-versioned-surface-is-enumerated` and `rule:packaging/below-1-0-the-breaking-slot-moves-left`
: it covers the language, the `Core` library, `nvs.toml`, the CLI, diagnostic identity, the
extension ABI and `serialize()` output -- and explicitly not the Rust APIs of the `nvs-*` crates.
Before 1.0 the breaking slot moves left: `0.MINOR` carries breaking changes.

A release is cut by [the release workflow](.github/workflows/release.yml), fired by hand from the
Actions tab; [docs/release.md](docs/release.md) is the procedure and what it needs configured.
"""

# `feat(scope)!: subject` / `fix: subject`. The `!` and the scope are both optional; a subject
# that does not match at all is kept and reported under *Other* rather than discarded.
CONVENTIONAL = re.compile(r"^(?P<type>[a-z]+)(?:\((?P<scope>[^)]*)\))?(?P<bang>!)?: (?P<subject>.+)$")

# Listed in full, in this order. Everything else collapses into the Internal count.
HEADLINE_SECTIONS = (("feat", "Features"), ("fix", "Fixes"), ("perf", "Performance"))

# Most entries any one section lists before it turns into a count. A release body on GitHub is
# capped at 125,000 characters, and rendering this repository's whole history unbounded produced
# 1,379 bullets -- well past it, so the *first* release would have been the one that failed. An
# ordinary release lands nowhere near this and is listed in full; the compare link carries the
# remainder either way. Raise it with --max-per-section if a release genuinely warrants it.
SECTION_CAP = 100


def die(message: str) -> None:
    print(f"release.py: {message}", file=sys.stderr)
    raise SystemExit(1)


def git(*args: str, check: bool = True) -> str:
    """Run git in the repository root and hand back its stdout.

    Never invoked through a shell, here or anywhere below: a commit subject reaches this file as
    an argument and as bytes on a pipe, and a subject in this history may contain backticks,
    `$(...)` or a quote. That is the same reason the workflow passes values through `env:` and
    never interpolates them into a `run:` block.
    """
    proc = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace"
    )
    if check and proc.returncode != 0:
        die(f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc.stdout


def cargo(*args: str) -> None:
    proc = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True, errors="replace")
    if proc.returncode != 0:
        die(f"cargo {' '.join(args)} failed:\n{proc.stdout}\n{proc.stderr}")


# ---------------------------------------------------------------------------
# Versions
# ---------------------------------------------------------------------------


def parse(version: str) -> tuple[int, int, int]:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", version.strip())
    if not match:
        die(f"{version!r} is not MAJOR.MINOR.PATCH")
    return tuple(int(part) for part in match.groups())  # type: ignore[return-value]


def workspace_version() -> str:
    """The `version` under `[workspace.package]` -- the one every crate inherits."""
    text = CARGO_TOML.read_text(encoding="utf-8")
    section = re.search(r"^\[workspace\.package\]$(.*?)^\[", text, re.M | re.S)
    if not section:
        die("Cargo.toml has no [workspace.package] section")
    match = re.search(r'^version\s*=\s*"([^"]+)"', section.group(1), re.M)
    if not match:
        die("[workspace.package] has no version")
    return match.group(1)


def tags() -> list[tuple[tuple[int, int, int], str]]:
    """Every `vX.Y.Z` tag, newest version first. Anything else is not a release tag."""
    found = []
    for line in git("tag", "--list", "v*").splitlines():
        line = line.strip()
        if re.fullmatch(r"v\d+\.\d+\.\d+", line):
            found.append((parse(line[1:]), line))
    return sorted(found, reverse=True)


def next_version(current: str, bump: str) -> str:
    """`rule:packaging/below-1-0-the-breaking-slot-moves-left`'s scheme, which is SemVer only at and above 1.0."""
    major, minor, patch = parse(current)
    if major == 0:
        # Below 1.0 the breaking slot is MINOR, so there is no slot left for a compatible
        # feature to differ from a fix in. `minor` and `patch` agreeing here is the scheme
        # working, not a bug -- see this file's docstring.
        return f"0.{minor + 1}.0" if bump == "major" else f"0.{minor}.{patch + 1}"
    if bump == "major":
        return f"{major + 1}.0.0"
    if bump == "minor":
        return f"{major}.{minor + 1}.0"
    return f"{major}.{minor}.{patch + 1}"


# ---------------------------------------------------------------------------
# Container image tags
# ---------------------------------------------------------------------------

# Suffix per runtime base. The default image carries none, so `novis:0.4.1` is the one most
# people ever type; the variant appends rather than prefixes, giving `latest-debian` and
# `0.4-debian` -- which sort beside their defaults in a registry listing where `debian-latest`
# would not.
DOCKER_VARIANTS = {"distroless": "", "debian": "-debian"}


def docker_tags(version: str, image: str, variant: str, floating: bool) -> list[str]:
    """The registry tags one variant of one release claims.

    Here rather than in `docker/metadata-action` because that action's `type=semver` implements
    SemVer, and `rule:packaging/below-1-0-the-breaking-slot-moves-left` is not SemVer below 1.0 -- it moves the breaking slot left, so
    `0.MINOR` is what carries a breaking change. Under plain SemVer a bare `0` tag is the stable
    line; under this scheme it would follow 0.0 -> 0.1 straight across a breaking change, which
    is the one thing a floating tag must never do. So there is no bare-major tag until there is
    a major, and the rule is one sentence for both regimes: **the compatible line is always
    MAJOR.MINOR, and MAJOR alone is a tag only where MAJOR is the breaking slot.**

    `floating` selects the tags that *move*. The release run pushes only the immutable ones; the
    promote workflow adds these when a human publishes the draft, because until that click the
    notes for this version are not readable by anyone -- and a `latest` resolving to a version
    nobody can read about is worse than a `latest` one release behind.
    """
    if variant not in DOCKER_VARIANTS:
        die(f"{variant!r} is not a known image variant ({', '.join(sorted(DOCKER_VARIANTS))})")
    major, minor, _ = parse(version)
    suffix = DOCKER_VARIANTS[variant]
    # `--short` and not the full hash: this is the human-readable pin, and the digest is already
    # the cryptographic one. Seven is what `git log --oneline` and every GitHub URL show.
    short = git("rev-parse", "--short=7", "HEAD").strip()

    chosen = [f"{version}{suffix}", f"sha-{short}{suffix}"]
    if floating:
        known = tags()
        # Publishing an older draft after a newer release has already gone out would otherwise
        # walk `latest` backwards. The immutable tags above are still correct in that case, so
        # this drops the moving ones rather than failing the run.
        if known and parse(version) < known[0][0]:
            print(
                f"release.py: {version} is behind {known[0][1]}; not moving the floating tags.",
                file=sys.stderr,
            )
        else:
            chosen.append(f"{major}.{minor}{suffix}")
            if major >= 1:
                chosen.append(f"{major}{suffix}")
            chosen.append(f"latest{suffix}")
    return [f"{image}:{tag}" for tag in chosen]


def resolve(bump: str, exact: str | None, allow_contract: bool) -> tuple[str, str, str | None]:
    """(current, next, previous tag) -- every guard a release has to clear before it writes."""
    current = workspace_version()
    known = tags()
    previous_tag = known[0][1] if known else None

    if previous_tag and known[0][0] > parse(current):
        die(
            f"tag {previous_tag} is ahead of Cargo.toml's {current}. The tree is behind a release "
            f"that already happened -- reconcile before releasing again."
        )

    target = exact.strip() if exact and exact.strip() else next_version(current, bump)
    parse(target)

    if parse(target) <= parse(current):
        die(f"{target} does not advance {current}")
    if any(version == parse(target) for version, _ in known):
        die(f"v{target} is already tagged")

    if parse(target) >= (0, 1, 0) and parse(current) < (0, 1, 0) and not allow_contract:
        die(
            f"{current} -> {target} crosses into the version contract.\n"
            "  `rule:packaging/the-version-contract-starts-at-0-1-0`: 0.1.0 is the release that declares the language complete enough to\n"
            "  write against, and the switch is thrown once, in the commit that tags it. That is a\n"
            "  decision a person takes, not a dropdown.\n"
            "  Re-run the workflow with 'I understand this declares the version contract' ticked,\n"
            "  or pass --allow-contract locally."
        )
    return current, target, previous_tag


# ---------------------------------------------------------------------------
# Notes
# ---------------------------------------------------------------------------


def repo_url() -> str:
    """Where a commit link points.

    In CI the runner knows; locally, `[package.metadata]`'s `repository` is the committed answer.
    Neither is guessed from `origin`, which on a contributor's clone is a fork or a mirror.
    """
    server = os.environ.get("GITHUB_SERVER_URL")
    slug = os.environ.get("GITHUB_REPOSITORY")
    if server and slug:
        return f"{server.rstrip('/')}/{slug}"
    match = re.search(r'^repository\s*=\s*"([^"]+)"', CARGO_TOML.read_text(encoding="utf-8"), re.M)
    return match.group(1).rstrip("/") if match else ""


def commits(since: str | None) -> list[dict[str, str]]:
    """Every commit in the release, oldest first, split on control characters.

    `%x1f` and `%x1e` rather than a newline or a pipe because a commit body here contains both,
    routinely, and a body is what carries `BREAKING CHANGE:`.
    """
    span = f"{since}..HEAD" if since else "HEAD"
    raw = git("log", "--reverse", "--no-merges", "--format=%H%x1f%s%x1f%b%x1e", span)
    out = []
    for record in raw.split("\x1e"):
        record = record.strip("\n")
        if not record.strip():
            continue
        sha, subject, body = (record.split("\x1f") + ["", ""])[:3]
        out.append({"sha": sha.strip(), "subject": subject.strip(), "body": body})
    return out


def classify(commit: dict[str, str]) -> dict[str, str | bool | None]:
    match = CONVENTIONAL.match(commit["subject"])
    breaking = "BREAKING CHANGE:" in commit["body"] or "BREAKING-CHANGE:" in commit["body"]
    if not match:
        return {**commit, "type": None, "scope": None, "clean": commit["subject"], "breaking": breaking}
    return {
        **commit,
        "type": match.group("type"),
        "scope": (match.group("scope") or "").strip() or None,
        "clean": match.group("subject"),
        "breaking": breaking or bool(match.group("bang")),
    }


def bullet(entry: dict, url: str) -> str:
    short = entry["sha"][:8]
    link = f" ([`{short}`]({url}/commit/{entry['sha']}))" if url else f" (`{short}`)"
    scope = f"**{entry['scope']}**: " if entry["scope"] else ""
    return f"- {scope}{entry['clean']}{link}"


def render(version: str, previous_tag: str | None, entries: list[dict], url: str, cap: int = SECTION_CAP) -> str:
    today = datetime.now(timezone.utc).date().isoformat()
    lines = [f"## [{version}] - {today}", ""]

    if not entries:
        lines += ["No commits since the previous release.", ""]
        return "\n".join(lines)

    def listing(rows: list[dict]) -> list[str]:
        ordered = sorted(rows, key=lambda r: (r["scope"] or "", r["clean"]))
        out = [bullet(row, url) for row in ordered[:cap]]
        if len(ordered) > cap:
            out.append(f"- …and {len(ordered) - cap} more, in the full changes below.")
        return out

    def section(title: str, rows: list[dict]) -> None:
        if not rows:
            return
        lines.append(f"### {title}")
        lines.append("")
        lines.extend(listing(rows))
        lines.append("")

    breaking = [entry for entry in entries if entry["breaking"]]
    if breaking:
        lines += [
            "### Breaking changes",
            "",
            "What a break may and may not be is `rule:packaging/the-versioned-surface-is-enumerated`, `rule:packaging/below-1-0-the-breaking-slot-moves-left` and `rule:packaging/who-can-see-it-decides-the-release-slot`.",
            "",
        ]
        lines.extend(listing(breaking))
        lines.append("")

    listed = {id(entry) for entry in breaking}
    for kind, title in HEADLINE_SECTIONS:
        rows = [entry for entry in entries if entry["type"] == kind and id(entry) not in listed]
        listed.update(id(row) for row in rows)
        section(title, rows)

    other = [entry for entry in entries if entry["type"] is None and id(entry) not in listed]
    listed.update(id(row) for row in other)
    section("Other", other)

    # Everything left is bookkeeping. Counted, never listed -- the docstring says why.
    rest = [entry for entry in entries if id(entry) not in listed]
    if rest:
        counts: dict[str, int] = {}
        for entry in rest:
            counts[str(entry["type"])] = counts.get(str(entry["type"]), 0) + 1
        tally = ", ".join(f"{kind} ({count})" for kind, count in sorted(counts.items(), key=lambda p: -p[1]))
        lines += ["### Internal", "", f"{len(rest)} further commits: {tally}.", ""]

    if url:
        span = f"{previous_tag}...v{version}" if previous_tag else f"v{version}"
        lines += [f"[Full changes]({url}/compare/{span}) — {len(entries)} commits.", ""]
    return "\n".join(lines)


def notes_for(version: str, previous_tag: str | None, cap: int = SECTION_CAP) -> str:
    return render(version, previous_tag, [classify(c) for c in commits(previous_tag)], repo_url(), cap)


# ---------------------------------------------------------------------------
# Applying
# ---------------------------------------------------------------------------


def rewrite_manifest(version: str) -> int:
    """The workspace version and the fourteen path-dependency pins, in one pass."""
    text = CARGO_TOML.read_text(encoding="utf-8")
    changed = 0

    def bump_package(match: re.Match[str]) -> str:
        nonlocal changed
        body = re.sub(r'^(version\s*=\s*)"[^"]+"', rf'\1"{version}"', match.group(2), count=1, flags=re.M)
        changed += 1
        return match.group(1) + body

    text, hits = re.subn(r"(^\[workspace\.package\]$)(.*?)(?=^\[)", bump_package, text, count=1, flags=re.M | re.S)
    if hits != 1:
        die("could not find [workspace.package] to rewrite")

    def bump_pin(match: re.Match[str]) -> str:
        nonlocal changed
        changed += 1
        return f'{match.group(1)}"{version}"'

    text, pins = re.subn(
        r'(^nvs-[a-z0-9-]+\s*=\s*\{\s*path\s*=\s*"[^"]+"\s*,\s*version\s*=\s*)"[^"]+"',
        bump_pin,
        text,
        flags=re.M,
    )
    if pins == 0:
        die("no nvs-* path dependencies carried a version pin; the manifest shape changed")

    CARGO_TOML.write_text(text, encoding="utf-8", newline="")
    return changed


def prepend_changelog(section: str) -> None:
    """Insert above the newest existing release, creating the file on a first release."""
    if not CHANGELOG.exists():
        CHANGELOG.write_text(f"{CHANGELOG_HEADER}\n{section}", encoding="utf-8", newline="")
        return
    text = CHANGELOG.read_text(encoding="utf-8")
    marker = re.search(r"^## \[", text, re.M)
    if marker:
        CHANGELOG.write_text(text[: marker.start()] + section + "\n" + text[marker.start() :], encoding="utf-8", newline="")
    else:
        CHANGELOG.write_text(text.rstrip("\n") + "\n\n" + section, encoding="utf-8", newline="")


# ---------------------------------------------------------------------------
# Packaging
# ---------------------------------------------------------------------------


def package(version: str, target: str, name: str, kind: str, out_dir: Path) -> None:
    """Archive one built binary with the notices it must ship beside.

    Deliberately not `strip`ped. `[profile.release]` in Cargo.toml keeps
    `debug = "line-tables-only"` with the stated reason "keep backtraces useful in production
    builds" -- stripping the archive would spend exactly what that line buys, to save bytes,
    which is the trade AGENTS.md's priority ordering puts last.
    """
    exe = "nvs.exe" if "windows" in target else "nvs"
    built = ROOT / "target" / target / "release" / exe
    if not built.is_file():
        die(f"{built} does not exist; build before packaging")

    stem = f"nvs-{version}-{name}"
    staged = out_dir / stem
    if staged.exists():
        shutil.rmtree(staged)
    staged.mkdir(parents=True)
    shutil.copy2(built, staged / exe)
    for extra in ARCHIVE_EXTRAS:
        source = ROOT / extra
        if not source.is_file():
            die(f"{extra} is missing; every archive ships it (`rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`)")
        shutil.copy2(source, staged / extra)

    if kind == "zip":
        archive = out_dir / f"{stem}.zip"
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
            for path in sorted(staged.rglob("*")):
                zf.write(path, f"{stem}/{path.relative_to(staged).as_posix()}")
    else:
        archive = out_dir / f"{stem}.tar.gz"

        def normalise(info: tarfile.TarInfo) -> tarfile.TarInfo:
            # Windows has no mode bit to copy, so the binary would land in the tarball
            # non-executable when a cross-built archive is produced there. Set it explicitly
            # and zero the ownership so the archive is reproducible.
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mode = 0o755 if Path(info.name).name == exe or info.isdir() else 0o644
            return info

        with tarfile.open(archive, "w:gz", compresslevel=9) as tf:
            tf.add(staged, arcname=stem, filter=normalise)

    shutil.rmtree(staged)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (out_dir / f"{archive.name}.sha256").write_text(f"{digest}  {archive.name}\n", encoding="utf-8", newline="")
    print(f"{archive.name}  {archive.stat().st_size / 1_048_576:.1f} MiB  {digest}")


# ---------------------------------------------------------------------------
# Checking
# ---------------------------------------------------------------------------


def check() -> int:
    """What the tree owes if a release has landed. Run by CI's `docs` job."""
    problems = []
    version = workspace_version()
    text = CARGO_TOML.read_text(encoding="utf-8")
    for pin in re.finditer(r'^(nvs-[a-z0-9-]+)\s*=\s*\{\s*path[^}]*?version\s*=\s*"([^"]+)"', text, re.M):
        if pin.group(2) != version:
            problems.append(f"{pin.group(1)} is pinned at {pin.group(2)}, workspace is {version}")

    known = tags()
    if known:
        newest, tag = known[0]
        if newest > parse(version):
            problems.append(f"{tag} is ahead of Cargo.toml's {version}")
        if newest == parse(version) and CHANGELOG.exists():
            if f"## [{version}]" not in CHANGELOG.read_text(encoding="utf-8"):
                problems.append(f"{tag} is released but CHANGELOG.md has no [{version}] section")

    for problem in problems:
        print(f"release.py: {problem}", file=sys.stderr)
    if problems:
        return 1
    print(f"release.py: version {version} is consistent across the manifest, the tags and the changelog.")
    return 0


# ---------------------------------------------------------------------------


def emit_output(to_github: bool, **values: str) -> None:
    for key, value in values.items():
        print(f"{key}={value}")
    path = os.environ.get("GITHUB_OUTPUT")
    if to_github and path:
        with open(path, "a", encoding="utf-8") as handle:
            for key, value in values.items():
                handle.write(f"{key}={value}\n")


def emit_lines(to_github: bool, key: str, lines: list[str]) -> None:
    """A multi-line step output, in the heredoc form `$GITHUB_OUTPUT` requires.

    `key=a\\nb` would be read as `key=a` followed by a malformed line, so a value with newlines
    in it has to be delimited. The delimiter is fixed rather than random because the only thing
    written through here is a tag list this file just built out of a version and a short hash --
    none of which can contain it. A value from anywhere less controlled would need a random one,
    and would be the wrong thing to pass through a step output at all.
    """
    for line in lines:
        print(line)
    path = os.environ.get("GITHUB_OUTPUT")
    if to_github and path:
        with open(path, "a", encoding="utf-8") as handle:
            handle.write(f"{key}<<NVS_RELEASE_EOF\n")
            handle.write("".join(f"{line}\n" for line in lines))
            handle.write("NVS_RELEASE_EOF\n")


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--plan", choices=["patch", "minor", "major"], help="compute the next version; write nothing")
    mode.add_argument("--preview", choices=["patch", "minor", "major"], help="--plan and the notes, to stdout")
    mode.add_argument("--notes", metavar="VERSION", help="render that version's changelog section")
    mode.add_argument("--apply", metavar="VERSION", help="rewrite the manifests and prepend the changelog")
    mode.add_argument("--package", action="store_true", help="archive a built binary")
    mode.add_argument("--check", action="store_true", help="the manifests, the tags and the changelog agree")
    mode.add_argument("--docker-tags", metavar="VERSION", help="the registry tags one image variant claims")

    parser.add_argument("--version", default="", help="exact version, overriding --plan/--preview's arithmetic")
    parser.add_argument("--allow-contract", action="store_true", help="permit crossing into 0.1.0 (`rule:packaging/the-version-contract-starts-at-0-1-0`)")
    parser.add_argument("--out", type=Path, help="output file (--notes) or directory (--package)")
    parser.add_argument("--notes-file", type=Path, help="the rendered section --apply prepends")
    parser.add_argument("--since", help="previous tag, when it is not the newest one")
    parser.add_argument("--manifests-only", action="store_true", help="--apply without touching CHANGELOG.md")
    parser.add_argument("--github-output", action="store_true", help="also append to $GITHUB_OUTPUT")
    parser.add_argument("--target", help="--package: the Rust target triple that was built")
    parser.add_argument("--name", help="--package: the platform name used in the archive filename")
    parser.add_argument("--archive", choices=["tar.gz", "zip"], default="tar.gz", help="--package: format")
    parser.add_argument("--image", help="--docker-tags: the registry repository, e.g. ghcr.io/novis-lang/novis")
    parser.add_argument(
        "--variant", choices=sorted(DOCKER_VARIANTS), default="distroless", help="--docker-tags: the runtime base"
    )
    parser.add_argument(
        "--floating", action="store_true", help="--docker-tags: also the tags that move (latest, the MAJOR.MINOR line)"
    )
    parser.add_argument(
        "--max-per-section", type=int, default=SECTION_CAP, help=f"entries a section lists before it counts (default {SECTION_CAP})"
    )
    args = parser.parse_args()

    if args.check:
        return check()

    if args.docker_tags:
        parse(args.docker_tags)
        if not args.image:
            die("--docker-tags needs --image (e.g. ghcr.io/novis-lang/novis)")
        # Lowercased because a registry reference must be, and `GITHUB_REPOSITORY` carries the
        # owner and repository as they were typed -- `Novis-Lang/Novis` would be pushed as-is and
        # rejected by the registry after the whole build had already run.
        emit_lines(
            args.github_output,
            "tags",
            docker_tags(args.docker_tags, args.image.strip().lower(), args.variant, args.floating),
        )
        return 0

    if args.package:
        if not (args.version and args.target and args.name and args.out):
            die("--package needs --version, --target, --name and --out")
        args.out.mkdir(parents=True, exist_ok=True)
        package(args.version, args.target, args.name, args.archive, args.out)
        return 0

    if args.plan or args.preview:
        bump = args.plan or args.preview
        current, target, previous_tag = resolve(bump, args.version, args.allow_contract)
        if args.plan:
            emit_output(
                args.github_output,
                current=current,
                version=target,
                tag=f"v{target}",
                previous_tag=previous_tag or "",
            )
        else:
            print(f"# {current} --{bump}--> {target}   (previous tag: {previous_tag or 'none'})\n")
            print(notes_for(target, previous_tag, args.max_per_section))
        return 0

    if args.notes:
        parse(args.notes)
        previous = args.since or (tags()[0][1] if tags() else None)
        rendered = notes_for(args.notes, previous, args.max_per_section)
        if args.out:
            args.out.write_text(rendered, encoding="utf-8", newline="")
            print(f"release.py: wrote {args.out}")
        else:
            print(rendered)
        return 0

    if args.apply:
        version = args.apply
        parse(version)
        fields = rewrite_manifest(version)
        cargo("update", "--workspace", "--offline")
        if not args.manifests_only:
            if not args.notes_file or not args.notes_file.is_file():
                die("--apply needs --notes-file (render it with --notes first)")
            prepend_changelog(args.notes_file.read_text(encoding="utf-8").rstrip("\n") + "\n")
        print(f"release.py: {fields} version fields at {version}; Cargo.lock updated.")
        return 0

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
