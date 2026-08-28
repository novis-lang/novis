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
    python site.py stop         stop any dev or preview server left running
    python site.py clean        remove build output and generated content

`dev` and `build` run `sync` first, so a developer never has to know which
generators exist. After one `npm ci` — the only step that needs the network —
every command works offline: search is indexed locally by Pagefind, fonts are
the system stack, and no page references a CDN.

**Every command stops any dev or preview server that is already running**, so
a forgotten one from a closed terminal can never sit on the port serving bytes
from an older build. `--keep-servers` opts out of that for one invocation.

This file is intentionally the only entry point. Everything under `tools/` is a
module it calls; nothing there is meant to be run directly, though each will
run standalone for debugging.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import time
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
ASTRO_BIN = NODE_MODULES / "astro" / "bin" / "astro.mjs"

# Astro's own state directory. It records a running dev or preview server in
# `dev.json` / `preview.json` — pid, port and url — and removes the file when
# the server stops. Gitignored, and the reason this tool needs no bookkeeping
# of its own.
ASTRO_STATE = WEB / ".astro"


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


def node() -> str:
    exe = shutil.which("node")
    if not exe:
        raise SystemExit("node is not on PATH. Install Node 20.19+ and run this again.")
    return exe


def run(cmd: list[str], env: dict[str, str] | None = None) -> int:
    return subprocess.run(cmd, cwd=WEB, env={**os.environ, **(env or {})}).returncode


# --------------------------------------------------------------------------
# servers
#
# Astro's dev and preview servers hold a port until something kills them, and
# a session that ends any way other than Ctrl-C leaves one behind. Two of them
# then disagree about which is serving the port you are looking at, which is a
# genuinely confusing way to lose half an hour.
#
# So every `site.py` invocation clears the field first. There is nothing to
# remember and nothing to clean up by hand:
#
#   * every command clears both lock files, which also catches a server
#     someone started with `npm run dev` directly — Astro writes the lock
#     whichever way it was launched;
#   * `site.py stop` additionally scans the process table, for the case where
#     a lock file was deleted by hand and the server outlived it.
#
# The mechanism is Astro's own: it records a running server in
# `.astro/dev.json` and `.astro/preview.json`, and it has `astro dev stop` /
# `astro preview stop` to end one. So the fast path is two `exists()` calls,
# and there is nothing to do at all when nothing is running.
#
# Astro is launched directly rather than through `npm run`, because going
# through npm puts a wrapper process in between and the wrapper is not what
# holds the port.
# --------------------------------------------------------------------------


def _stdout(cmd: list[str], timeout: int = 30) -> str:
    """Captured stdout, always a string. Never raises.

    `errors="replace"` is not optional on Windows: console tools such as
    `tasklist` emit the OEM code page, not the ANSI one Python decodes with by
    default, and a single thousands separator in a memory column is enough to
    raise `UnicodeDecodeError` out of what should be a read-only check.
    """
    try:
        r = subprocess.run(
            cmd, capture_output=True, text=True, errors="replace", timeout=timeout
        )
        return r.stdout or ""
    except (OSError, ValueError, subprocess.SubprocessError):
        return ""


def _lock(kind: str) -> Path:
    return ASTRO_STATE / f"{kind}.json"


def _describe(pid: int) -> str | None:
    """The command line of a live process, or None if it is gone.

    A lock file can outlive the server it describes — a machine that lost power
    leaves one behind — and a process id gets reused. So a recorded id is only
    ever acted on once this has confirmed there is still a node process wearing
    it. Killing the wrong process would be far worse than leaving a dev server
    running.
    """
    if pid <= 0 or pid == os.getpid():
        return None
    if os.name == "nt":
        # tasklist reports "no tasks" on stdout rather than failing, so the
        # image name is what tells a live node process from a stale id.
        out = _stdout(["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"], timeout=10)
        return out if out.lower().startswith('"node') else None
    return _stdout(["ps", "-p", str(pid), "-o", "args="], timeout=10).strip() or None


def _kill(pid: int) -> bool:
    """Ask a process to stop, then insist. True if it is gone afterwards."""
    for sig, wait in ((signal.SIGTERM, 1.5), (getattr(signal, "SIGKILL", signal.SIGTERM), 1.0)):
        try:
            os.kill(pid, sig)
        except (ProcessLookupError, PermissionError, OSError):
            if os.name == "nt":
                subprocess.run(
                    ["taskkill", "/PID", str(pid), "/T", "/F"], capture_output=True, timeout=10
                )
            else:
                return _describe(pid) is None
        deadline = time.time() + wait
        while time.time() < deadline:
            if _describe(pid) is None:
                return True
            time.sleep(0.1)
    return _describe(pid) is None


def _scan_pids() -> list[int]:
    """Astro processes running out of *this* website directory.

    A last resort for `site.py stop`, covering a server whose lock file was
    deleted by hand. Enumerating processes costs a beat, so nothing else pays
    for it — the lock files above are authoritative in every ordinary case.
    """
    needle = str(ASTRO_BIN).lower()
    pids: list[int] = []
    if os.name == "nt":
        ps = _stdout(
            [
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"Name='node.exe'\" | "
                "ForEach-Object { \"$($_.ProcessId)`t$($_.CommandLine)\" }",
            ]
        )
        rows = [line.split("	", 1) for line in ps.splitlines() if "	" in line]
    else:
        ps = _stdout(["ps", "-eo", "pid=,args="])
        rows = [ln.strip().split(None, 1) for ln in ps.splitlines() if ln.strip()]
        rows = [r for r in rows if len(r) == 2]

    for raw_pid, cmd in rows:
        low = cmd.lower()
        if needle not in low or (" dev" not in low and " preview" not in low):
            continue
        try:
            pid = int(raw_pid.strip())
        except ValueError:
            continue
        if pid != os.getpid():
            pids.append(pid)
    return pids


def stop_servers(scan: bool = False, quiet: bool = True) -> int:
    """Stop every dev and preview server belonging to this website.

    Costs nothing when none is running: the two lock files simply do not exist,
    and the function returns after two `exists()` calls. That is what lets every
    command do this unconditionally.
    """
    stopped = 0
    seen: set[int] = set()

    for kind in ("dev", "preview"):
        lock = _lock(kind)
        if not lock.is_file():
            continue

        pid = None
        try:
            pid = json.loads(lock.read_text(encoding="utf-8")).get("pid")
        except (OSError, ValueError, AttributeError):
            pass

        if isinstance(pid, int) and _describe(pid):
            seen.add(pid)
            # Astro's own command first, so it tidies up its lock and logs the
            # way it expects. Force is the fallback, not the plan.
            if ASTRO_BIN.is_file():
                subprocess.run(
                    [node(), str(ASTRO_BIN), kind, "stop"],
                    cwd=WEB,
                    capture_output=True,
                    timeout=30,
                )
            gone = _describe(pid) is None or _kill(pid)
            if gone:
                stopped += 1
                if not quiet:
                    ok(f"stopped the {kind} server (pid {pid})")
            elif not quiet:
                warn(f"could not stop the {kind} server (pid {pid}) — stop it by hand")
        elif not quiet:
            info(f"cleared a stale {kind} lock file")

        # Whether it was running, stale, or unreadable, the lock must not
        # outlive this call — Astro refuses to start when one is present.
        try:
            lock.unlink(missing_ok=True)
        except OSError:
            pass

    if scan:
        for pid in _scan_pids():
            if pid in seen:
                continue
            if _kill(pid):
                stopped += 1
                if not quiet:
                    ok(f"stopped an untracked Astro process (pid {pid})")

    if not quiet and stopped == 0:
        info("no dev or preview server was running")
    return stopped


def serve(kind: str, args_list: list[str], env: dict[str, str] | None = None) -> int:
    """Run `astro dev` or `astro preview` in the foreground."""
    if not ASTRO_BIN.is_file():
        raise SystemExit(f"{ASTRO_BIN} is missing — run `npm ci` in website/")
    return run([node(), str(ASTRO_BIN), kind, *args_list], env)


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
    info(f"http://{args.host}:{args.port}/  —  Ctrl-C, or `python site.py stop`")
    return serve("dev", ["--host", args.host, "--port", str(args.port)], env)


def cmd_build(args: argparse.Namespace) -> int:
    ensure_deps(allow_install=not args.no_install)
    sync()
    step("build")
    env = {"SITE_VERSION": os.environ.get("SITE_VERSION", git_describe())}
    rc = run([node(), str(ASTRO_BIN), "build"], env)
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
    step("preview")
    info(f"http://{args.host}:{args.port}/  —  Ctrl-C, or `python site.py stop`")
    return serve("preview", ["--host", args.host, "--port", str(args.port)])


def cmd_stop(args: argparse.Namespace) -> int:
    step("stop")
    stop_servers(scan=True, quiet=False)
    return 0


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
    # Every subcommand takes `--keep-servers`, defined once here. It is not on
    # the top-level parser as well: argparse would then let the subparser's
    # default overwrite a value given before the subcommand, and the flag would
    # silently do nothing in half the spellings people try.
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument(
        "--keep-servers",
        action="store_true",
        help="leave any running dev/preview server alone",
    )

    sub = p.add_subparsers(dest="cmd", required=True)

    def add(name: str, help_: str) -> argparse.ArgumentParser:
        return sub.add_parser(name, help=help_, parents=[common])

    def serve_args(sp: argparse.ArgumentParser) -> None:
        sp.add_argument("--port", type=int, default=4321)
        sp.add_argument("--host", default="localhost")

    d = add("dev", "start the dev server")
    serve_args(d)
    d.add_argument("--no-install", action="store_true")
    d.set_defaults(fn=cmd_dev)

    b = add("build", "produce dist/")
    b.add_argument("--no-install", action="store_true")
    b.set_defaults(fn=cmd_build)

    pv = add("preview", "serve dist/")
    serve_args(pv)
    pv.set_defaults(fn=cmd_preview)

    add("sync", "regenerate derived content").set_defaults(fn=cmd_sync)

    ck = add("check", "provenance, shape, locks, freshness")
    # Accepted and ignored, so CI can pass the same flag to every command.
    ck.add_argument("--no-install", action="store_true", help=argparse.SUPPRESS)
    ck.set_defaults(fn=cmd_check)

    dr = add("draft", "scaffold an authored page")
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

    bl = add("bless", "re-stamp a page's sources")
    bl.add_argument("path")
    bl.set_defaults(fn=cmd_bless)

    asm = add("assemble", "build every published version (CI)")
    asm.add_argument("--keep", action="store_true", help="do not wipe .site-out first")
    asm.set_defaults(fn=cmd_assemble)

    cl = add("clean", "remove build output and generated content")
    cl.add_argument("--all", action="store_true", help="remove node_modules too")
    cl.set_defaults(fn=cmd_clean)

    st = add("stop", "stop any running dev or preview server")
    st.set_defaults(fn=cmd_stop)

    args = p.parse_args()

    # Clear the field before doing anything. A server left over from a closed
    # terminal serves stale bytes on the port you are about to use, and the
    # only symptom is that your change "did not apply".
    if not args.keep_servers and args.cmd != "stop":
        stopped = stop_servers(quiet=True)
        if stopped:
            info(f"stopped {stopped} server{'s' if stopped > 1 else ''} that was already running")

    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
