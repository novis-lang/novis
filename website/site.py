#!/usr/bin/env python3
"""The one command that drives the Novis website.

    python site.py dev          start the dev server (installs deps on first run)
    python site.py build        produce dist/
    python site.py preview      serve dist/ locally
    python site.py sync         regenerate everything derived from the repository
    python site.py check        provenance, page shape, locks, generated freshness
    python site.py draft PATH   scaffold a new authored page from its sources
    python site.py bless PATH   re-stamp a page's sources after rereading it
    python site.py assemble     build every published version into one tree (CI)
    python site.py clean        remove build output and generated content

`dev` and `build` run `sync` first, so a developer never has to know which
generators exist. After one `npm ci` — the only step that needs the network —
every command works offline: search is indexed locally by Pagefind, fonts are
the system stack, and no page references a CDN.

This file is intentionally the only entry point. Everything under `tools/` is a
module it calls; nothing there is meant to be run directly, though each will
run standalone for debugging.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

WEB = Path(__file__).resolve().parent
sys.path.insert(0, str(WEB / "tools"))

from common import (  # noqa: E402
    REPO,
    blob_hash,
    fail,
    git_describe,
    info,
    ok,
    parse_frontmatter,
    replace_frontmatter_list,
    step,
    warn,
)

NODE_MODULES = WEB / "node_modules"
LOCKFILE = WEB / "package-lock.json"
INSTALLED_MARKER = NODE_MODULES / ".package-lock.json"


# --------------------------------------------------------------------------
# process helpers
# --------------------------------------------------------------------------


def npm() -> str:
    exe = shutil.which("npm")
    if not exe:
        raise SystemExit(
            "npm is not on PATH.\n"
            "The website needs Node 20.19+ (24 is what this repo develops against).\n"
            "Install Node, then run this command again — it does the rest."
        )
    return exe


def run(cmd: list[str], env: dict[str, str] | None = None) -> int:
    return subprocess.run(cmd, cwd=WEB, env={**os.environ, **(env or {})}).returncode


# --------------------------------------------------------------------------
# dependencies
# --------------------------------------------------------------------------


def ensure_deps(allow_install: bool = True) -> None:
    """Install node modules if they are missing or behind the lockfile.

    This is the one step that touches the network, and it happens once. A build
    with `node_modules` already present makes no network requests at all, which
    is what makes an offline checkout buildable.
    """
    fresh = (
        NODE_MODULES.is_dir()
        and INSTALLED_MARKER.is_file()
        and (not LOCKFILE.is_file() or INSTALLED_MARKER.stat().st_mtime >= LOCKFILE.stat().st_mtime)
    )
    if fresh:
        return

    if not allow_install:
        raise SystemExit(
            "node_modules is missing or stale and --no-install was given.\n"
            "Run `npm ci` in website/ first."
        )

    step("dependencies")
    if NODE_MODULES.is_dir():
        info("lockfile is newer than the installed tree — reinstalling")
    else:
        info("first run: this is the only step that needs the network")

    cmd = [npm(), "ci" if LOCKFILE.is_file() else "install", "--no-audit", "--no-fund"]
    if run(cmd) != 0:
        raise SystemExit("dependency install failed")
    ok("dependencies installed")


# --------------------------------------------------------------------------
# generation
# --------------------------------------------------------------------------


def sync(quiet: bool = False) -> None:
    """Regenerate everything this site derives from the rest of the repository.

    Three generators, three sources, no overlap:

      grammar  ← crates/nvs-syntax/src/token.rs     (the keyword table)
      status   ← docs/implementation-plan.md        (the status block)
      design   ← docs/adr/                          (every decision, verbatim)

    Each writes into a gitignored path. If you want to change what one of them
    produces, change its source or change the generator — never the output.
    """
    if not quiet:
        step("sync")

    import gen_grammar
    import gen_status
    import sync_adr

    try:
        gen_grammar.main(quiet=quiet)
    except SystemExit as e:
        warn(str(e))

    gen_status.main(quiet=quiet)

    sync_adr.main(
        repo_url=os.environ.get("REPO_URL"),
        repo_ref=os.environ.get("REPO_REF"),
    )


# --------------------------------------------------------------------------
# commands
# --------------------------------------------------------------------------


def cmd_dev(args: argparse.Namespace) -> int:
    ensure_deps(allow_install=not args.no_install)
    sync()
    step("dev server")
    env = {"SITE_VERSION": os.environ.get("SITE_VERSION", git_describe())}
    return run([npm(), "run", "dev", "--", "--host", args.host, "--port", str(args.port)], env)


def cmd_build(args: argparse.Namespace) -> int:
    ensure_deps(allow_install=not args.no_install)
    sync()
    step("build")
    env = {"SITE_VERSION": os.environ.get("SITE_VERSION", git_describe())}
    rc = run([npm(), "run", "build"], env)
    if rc == 0:
        # Pagefind's index lives in `_pagefind/`; GitHub Pages' Jekyll pass
        # deletes underscore directories unless this file exists.
        (WEB / "dist" / ".nojekyll").write_text("", encoding="utf-8")
        ok("dist/ is ready — `python site.py preview` to look at it")
    return rc


def cmd_preview(args: argparse.Namespace) -> int:
    if not (WEB / "dist").is_dir():
        fail("no dist/ — run `python site.py build` first")
        return 1
    return run([npm(), "run", "preview", "--", "--host", args.host, "--port", str(args.port)])


def cmd_sync(args: argparse.Namespace) -> int:
    sync()
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    step("check")
    sync(quiet=True)
    import check_pages

    return check_pages.main()


def cmd_draft(args: argparse.Namespace) -> int:
    import draft

    title = args.title or args.path.rstrip("/").rsplit("/", 1)[-1].replace("-", " ").capitalize()
    return draft.main(args.path, title, args.source, force=args.force)


def cmd_bless(args: argparse.Namespace) -> int:
    """Re-stamp a page's sources, recording that a human has reread it.

    This is the only command that says "this page is current". It is manual on
    purpose: a stamp means somebody checked, and nothing automated is entitled
    to make that claim.
    """
    path = Path(args.path)
    for candidate in (path, WEB / path, WEB / "src" / "content" / "docs" / path):
        if candidate.is_file():
            path = candidate
            break
    else:
        fail(f"no such page: {args.path}")
        return 1

    text = path.read_text(encoding="utf-8")
    fm = parse_frontmatter(text)
    sources = fm.get("sources") or []
    if isinstance(sources, str):
        sources = [sources]
    if not sources:
        warn(f"{path.name} declares no sources — nothing to stamp")
        return 0

    updated: list[str] = []
    for entry in sources:
        src = str(entry).split("@")[0].strip()
        p = REPO / src
        if not p.exists() and src.startswith("adr/"):
            p = REPO / "docs" / src
        h = blob_hash(p)
        if h is None:
            fail(f"source not found: {src}")
            return 1
        updated.append(f"{src}@{h}")

    path.write_text(replace_frontmatter_list(text, "sources", updated), encoding="utf-8")
    ok(f"stamped {len(updated)} source(s) on {path.name}")
    for u in updated:
        info(u)
    return 0


def cmd_assemble(args: argparse.Namespace) -> int:
    import assemble

    return assemble.main(clean=not args.keep)


def cmd_clean(args: argparse.Namespace) -> int:
    step("clean")
    targets = [
        WEB / "dist",
        WEB / ".astro",
        WEB / ".site-out",
        WEB / "src" / "generated",
        WEB / "src" / "grammars",
        WEB / "src" / "content" / "docs" / "design",
    ]
    if args.all:
        targets.append(NODE_MODULES)
    for t in targets:
        if t.exists():
            shutil.rmtree(t, ignore_errors=True)
            ok(f"removed {t.relative_to(WEB).as_posix()}")
    if not args.all:
        info("node_modules kept — `--all` removes it too (needs the network to restore)")
    return 0


# --------------------------------------------------------------------------


def main() -> int:
    p = argparse.ArgumentParser(
        prog="site.py",
        description="Build and check the Novis website.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    sub = p.add_subparsers(dest="cmd", required=True)

    def serve_args(sp: argparse.ArgumentParser) -> None:
        sp.add_argument("--port", type=int, default=4321)
        sp.add_argument("--host", default="localhost")

    d = sub.add_parser("dev", help="start the dev server")
    serve_args(d)
    d.add_argument("--no-install", action="store_true")
    d.set_defaults(fn=cmd_dev)

    b = sub.add_parser("build", help="produce dist/")
    b.add_argument("--no-install", action="store_true")
    b.set_defaults(fn=cmd_build)

    pv = sub.add_parser("preview", help="serve dist/")
    serve_args(pv)
    pv.set_defaults(fn=cmd_preview)

    sub.add_parser("sync", help="regenerate derived content").set_defaults(fn=cmd_sync)

    ck = sub.add_parser("check", help="provenance, shape, locks, freshness")
    # Accepted and ignored, so CI can pass the same flag to every command.
    ck.add_argument("--no-install", action="store_true", help=argparse.SUPPRESS)
    ck.set_defaults(fn=cmd_check)

    dr = sub.add_parser("draft", help="scaffold an authored page")
    dr.add_argument("path", help="page path under src/content/docs, without extension")
    dr.add_argument("--title")
    dr.add_argument(
        "--from",
        dest="source",
        action="append",
        default=[],
        metavar="FILE",
        help="repository file this page is written from; repeatable",
    )
    dr.add_argument("--force", action="store_true")
    dr.set_defaults(fn=cmd_draft)

    bl = sub.add_parser("bless", help="re-stamp a page's sources")
    bl.add_argument("path")
    bl.set_defaults(fn=cmd_bless)

    asm = sub.add_parser("assemble", help="build every published version (CI)")
    asm.add_argument("--keep", action="store_true", help="do not wipe .site-out first")
    asm.set_defaults(fn=cmd_assemble)

    cl = sub.add_parser("clean", help="remove build output and generated content")
    cl.add_argument("--all", action="store_true", help="remove node_modules too")
    cl.set_defaults(fn=cmd_clean)

    args = p.parse_args()
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
