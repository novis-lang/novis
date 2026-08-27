"""Build every published version and assemble them into one deployable tree.

This is the versioning mechanism, and it is deliberately not a plugin.

The alternative — a Starlight versioning plugin — archives a full copy of the
documentation into the repository each time a version is cut. That is N copies
of every page under source control, drifting away from the code they document.
Here, a version's documentation is the `website/` directory *at that git ref*.
Cutting a release copies nothing; it adds one line to `versions.config.json`.

    .site-out/
      src/<label>/       a git worktree at that ref  (removed afterwards)
      site/              the deployable tree
        index.html       the default version, at the root
        docs/…
        main/            the development line
        v0.3/            a tag
        versions.json    what the switcher fetches at runtime
        CNAME

Old refs are handled by simply not existing: a tag cut before this website was
written has no `website/` directory, and is skipped with a warning rather than
failing the deploy.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

from common import REPO, WEB, fail, info, ok, step, warn

OUT = WEB / ".site-out"
MANIFEST = WEB / "versions.config.json"


def _run(cmd: list[str], cwd: Path, env: dict[str, str] | None = None) -> int:
    printable = " ".join(cmd)
    info(f"{cwd.name}$ {printable}")
    full = {**os.environ, **(env or {})}
    return subprocess.run(cmd, cwd=cwd, env=full).returncode


def _npm() -> str:
    exe = shutil.which("npm")
    if not exe:
        raise SystemExit("npm not found on PATH")
    return exe


def _worktree(label: str, ref: str) -> Path | None:
    dest = OUT / "src" / label
    if dest.exists():
        shutil.rmtree(dest, ignore_errors=True)
    dest.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        ["git", "worktree", "add", "--detach", str(dest), ref],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        warn(f"cannot check out ref {ref!r}: {r.stderr.strip().splitlines()[-1:] or ['unknown']}")
        return None
    return dest


def _drop_worktree(dest: Path) -> None:
    subprocess.run(["git", "worktree", "remove", "--force", str(dest)], cwd=REPO, capture_output=True)


def build_one(label: str, ref: str, base: str, site_root: Path) -> bool:
    step(f"version {label}  (ref {ref}, base {base})")

    tree = _worktree(label, ref)
    if tree is None:
        return False

    try:
        web = tree / "website"
        if not (web / "package.json").is_file():
            warn(f"{ref} has no website/ directory — skipping (it predates this site)")
            return False

        env = {"SITE_BASE": base, "SITE_VERSION": label}

        if _run([_npm(), "ci", "--no-audit", "--no-fund"], web) != 0:
            fail(f"{label}: npm ci failed")
            return False
        if _run([sys.executable, "site.py", "build", "--no-install"], web, env) != 0:
            fail(f"{label}: build failed")
            return False

        dist = web / "dist"
        target = site_root / base.strip("/") if base.strip("/") else site_root
        target.mkdir(parents=True, exist_ok=True)
        shutil.copytree(dist, target, dirs_exist_ok=True)
        ok(f"{label} → {('/' + base.strip('/')).rstrip('/') or '/'}")
        return True
    finally:
        _drop_worktree(tree)


def main(clean: bool = True) -> int:
    if not MANIFEST.is_file():
        fail(f"{MANIFEST} not found")
        return 1

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    versions = manifest.get("versions", [])
    default_label = manifest.get("default")

    if clean and OUT.exists():
        shutil.rmtree(OUT, ignore_errors=True)
    site_root = OUT / "site"
    site_root.mkdir(parents=True, exist_ok=True)

    published: list[dict] = []
    for v in versions:
        label, ref, base = v["label"], v.get("ref", v["label"]), v.get("base", f"/{v['label']}/")
        if build_one(label, ref, base, site_root):
            published.append(
                {
                    "label": label,
                    "base": base,
                    "preview": bool(v.get("preview")),
                    "default": label == default_label,
                }
            )

    if not published:
        fail("no version built successfully")
        return 1

    # What the switcher fetches. Lives at the root so every version shares it,
    # which is how a build cut a year ago learns that newer versions exist.
    (site_root / "versions.json").write_text(
        json.dumps({"versions": published}, indent=2) + "\n", encoding="utf-8"
    )

    cname = WEB / "public" / "CNAME"
    if cname.is_file():
        shutil.copy2(cname, site_root / "CNAME")

    # GitHub Pages runs Jekyll over the artifact unless told not to, and Jekyll
    # drops every directory beginning with an underscore — including Pagefind's
    # `_pagefind/`, which is the entire search index.
    (site_root / ".nojekyll").write_text("", encoding="utf-8")

    shutil.rmtree(OUT / "src", ignore_errors=True)

    step("assembled")
    for p in published:
        flag = " (latest)" if p["default"] else " (preview)" if p["preview"] else ""
        ok(f"{p['label']}{flag}  →  {p['base']}")
    info(f"deployable tree: {site_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
