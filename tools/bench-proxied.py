#!/usr/bin/env python3
"""The proxied benchmark: nginx in front of `nvs serve` and in front of PHP-FPM, one arm at a time.

`benches/proxied/README.md` is this leg's design and the only home for **why** it is shaped this way
-- why nginx fronts both peers, why the two stacks are separate compose files, why there are two
arms, and why its artifact is not `benches/serve.json`. Nothing here restates any of it. This file
owns only how the run is carried out.

The other leg is `tools/bench.py --serve-vs-fpm`: no containers, no proxy, Windows-native, and driven
by a generator written into that file. It stays, it is goal 6's acceptance check, and it is not
comparable with this one. Where the two files would otherwise hold the same code -- the FastCGI
client and the closed-loop generator behind arm 4 -- this one imports it rather than copying it.

## What a run does

Bring up one stack, measure its arms, tear it down, then the other. Never both at once: they would
compete for the same cores and neither number would be worth writing down. The teardown is in a
`finally`, so an interrupted run does not leave a stack holding a CPU budget.
"""

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path, PurePosixPath

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
BENCH_DIR = ROOT / "benches" / "proxied"

# `tools/bench.py` holds the FastCGI client and the closed-loop generator arm 4 is driven by. Imported
# rather than reimplemented: two copies of a protocol client is two things to keep correct, and the
# whole reason arm 4 exists is to be the same measurement the other leg takes.
sys.path.insert(0, str(HERE))
import bench  # noqa: E402  -- same directory; the FastCGI client has one home and it is there

CASE = "hello"

# The published ports, all on the loopback and none of them standard. `tests/db/compose.yaml`'s
# header owns that rule for every compose file here.
PHP_NGINX_PORT = 18080
PHP_FCGI_PORT = 19000
NVS_NGINX_PORT = 18081
NVS_DIRECT_PORT = 18082

# The path `/app` is mounted at inside both backends, as POSIX regardless of the host: `str()` of a
# `pathlib.Path` on Windows would hand PHP-FPM `\app\hello.php` and the responder would answer 404
# to every request in the run.
CONTAINER_PHP = PurePosixPath("/app/hello.php")

PHP_STACK = BENCH_DIR / "compose.php.yaml"
NVS_STACK = BENCH_DIR / "compose.nvs.yaml"

# Every file whose contents could move a number. Digested into each record so a figure can always be
# tied to the configuration that produced it -- and so a run taken after an untracked edit is
# visibly a different run rather than a mysterious one.
TUNABLES = [
    BENCH_DIR / "nginx" / "php.conf",
    BENCH_DIR / "nginx" / "nvs.conf",
    BENCH_DIR / "php" / "opcache.ini",
    BENCH_DIR / "php" / "pool.conf.in",
]


# --------------------------------------------------------------------------------------
# Rendering, staging and the two preconditions
# --------------------------------------------------------------------------------------


def render_pool(children: int) -> Path:
    """Write `generated/pool.conf` from the committed template.

    php-fpm interpolates no environment variable in a pool file -- `pm.max_children = $X` makes the
    whole configuration fail to load, with an error naming no key -- so the one number that varies
    between arms is substituted here. The template's own header carries that fact; this docstring
    does not repeat the reasoning, only the consequence.
    """
    template = BENCH_DIR / "php" / "pool.conf.in"
    out_dir = BENCH_DIR / "generated"
    out_dir.mkdir(exist_ok=True)
    rendered = template.read_text(encoding="utf-8").replace("@PHP_CHILDREN@", str(children))
    target = out_dir / "pool.conf"
    target.write_text(rendered, encoding="utf-8")
    return target


def stage_binary(path: Path, allow_debug: bool) -> None:
    """Copy a prebuilt Linux `nvs` to `.bin/nvs`, refusing anything that is not one.

    Two checks and no more. **ELF magic**, because the obvious mistake on this machine is handing it
    `target/release/nvs.exe` -- a PE binary the image would accept, place on the PATH and fail to
    exec, with a message about a missing file that is plainly there. And a **`debug` path
    component**, because a debug build measured as a release one does not fail at all: it produces a
    number several times too low and nothing in the output says why. `tools/bench.py` refuses the
    same thing for the same reason.
    """
    if not path.is_file():
        sys.exit(f"--nvs-bin {path} is not a file")
    magic = path.read_bytes()[:4]
    if magic != b"\x7fELF":
        sys.exit(
            f"--nvs-bin {path} is not a Linux ELF binary (magic {magic!r}); this image runs Linux, "
            "so a Windows `nvs.exe` cannot be the thing measured"
        )
    if not allow_debug and "debug" in path.parts:
        sys.exit(
            f"--nvs-bin {path} looks like a debug build; a debug binary benchmarks several times "
            "slower and the result would be wrong rather than failed. Pass --allow-debug to insist"
        )
    staged = BENCH_DIR / ".bin"
    staged.mkdir(exist_ok=True)
    (staged / "nvs").write_bytes(path.read_bytes())


def require_docker() -> None:
    """Fail before anything is built if there is no daemon, naming what is missing."""
    try:
        proc = subprocess.run(
            ["docker", "version", "--format", "{{.Server.Version}}"],
            capture_output=True, text=True, timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        sys.exit(f"docker is not runnable ({exc}); this leg is containers and has no fallback")
    if proc.returncode != 0:
        sys.exit(
            "the docker daemon is not answering; this leg is containers and has no fallback. "
            f"`docker version` said: {proc.stderr.strip() or proc.stdout.strip()}"
        )


# --------------------------------------------------------------------------------------
# Driving compose
# --------------------------------------------------------------------------------------


def compose(stack: Path, *args: str, env: dict | None = None, capture: bool = False):
    """One `docker compose -f <stack> ...`, with the environment the compose file reads."""
    argv = ["docker", "compose", "-f", str(stack), *args]
    merged = {**os.environ, **(env or {})}
    if capture:
        return subprocess.run(argv, capture_output=True, text=True, env=merged, timeout=1800)
    return subprocess.run(argv, env=merged, timeout=1800)


def up(stack: Path, env: dict, *services: str) -> None:
    """Bring named services up and wait for **healthy**, not merely started.

    `--wait` is why every service in both compose files carries a healthcheck: without it the first
    requests of a run land on a server that is still initialising, and they land only in the first
    rep -- which reads as a warm-up effect rather than as the harness's own bug.

    **Services are named rather than defaulted, and that is load-bearing.** The origin arms are
    measured with the proxy not yet started; § *Why the origin is measured before its proxy exists*
    in `main` is the reason, and it is a correctness one rather than a tidiness one.
    """
    proc = compose(stack, "up", "-d", "--wait", *services, env=env)
    if proc.returncode != 0:
        raise RuntimeError(
            f"`docker compose -f {stack.name} up {' '.join(services)}` failed "
            f"with {proc.returncode}"
        )


def down(stack: Path) -> None:
    """Tear a stack down, including its network. Never raises: this runs in a `finally`."""
    compose(stack, "down", "-v", "--remove-orphans", capture=True)


def logs_of(stack: Path, service: str) -> str:
    """What a service printed, for the failure message. A generator only ever sees a reset socket."""
    proc = compose(stack, "logs", "--no-color", "--tail", "40", service, capture=True)
    return (proc.stdout or "") + (proc.stderr or "")


# --------------------------------------------------------------------------------------
# The arms
# --------------------------------------------------------------------------------------


def oha_once(stack: Path, url: str, requests: int, concurrency: int, env: dict) -> dict:
    """One `oha` pass from inside the stack's own network, as parsed JSON.

    In-network rather than against the published port: on Windows a published port goes through
    Docker Desktop's userland proxy, which is a cost neither peer pays in production and which is
    large enough at these rates to be the thing being measured.
    """
    proc = compose(
        stack, "run", "--rm", "--no-deps", "oha",
        "-n", str(requests), "-c", str(concurrency),
        "--no-tui", "--output-format", "json",
        url,
        env=env, capture=True,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"oha failed against {url} ({proc.returncode}): {proc.stderr.strip()}")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"oha's output against {url} is not JSON ({exc}): {proc.stdout[:400]}")


def oha_arm(stack: Path, url: str, args, env: dict, kind: str, label: str, detail: str) -> dict:
    """Warm the peer, then take `--reps` passes and keep the best, as the other leg does.

    The warm pass is discarded for the reason `tools/bench.py`'s `measure_server` discards its own:
    the page cache, opcache's first compile and both JITs' first traces are start-up costs, and a
    proxied origin that has been up for a while is what this leg is about.
    """
    oha_once(stack, url, max(50, min(args.requests, 500)), args.concurrency, env)
    passes = []
    for _ in range(args.reps):
        passes.append(oha_once(stack, url, args.requests, args.concurrency, env))
    rates = [p["summary"]["requestsPerSec"] for p in passes]
    best = max(passes, key=lambda p: p["summary"]["requestsPerSec"])
    statuses: dict[str, int] = {}
    for p in passes:
        for code, count in p.get("statusCodeDistribution", {}).items():
            statuses[code] = statuses.get(code, 0) + count
    return {
        "kind": kind,
        "label": label,
        "detail": detail,
        "generator": "oha",
        "requests_per_sec": round(max(rates), 1),
        "median_requests_per_sec": round(statistics.median(rates), 1),
        "ms_per_request": round(1000.0 * args.concurrency / max(rates), 4),
        "latency_ms": best["metrics"]["latency_ms"],
        "success_rate": best["summary"]["successRate"],
        "status_codes": statuses,
        "errors": best.get("errorDistribution", {}),
    }


def fcgi_arm(args) -> dict:
    """Arm 4: PHP-FPM over FastCGI with no proxy, driven by `tools/bench.py`'s own client.

    The one arm `oha` cannot take, and not because of a gap in the harness: **FPM speaks FastCGI and
    nothing else**, so "PHP with no proxy in front" is not an HTTP endpoint that exists. The
    generator is therefore the other leg's, which makes arm 4 against arm 3 a comparison across two
    generators -- said in the record's caveats rather than left for a reader to notice.
    """
    def open_conn():
        return bench.FcgiConn(PHP_FCGI_PORT, CONTAINER_PHP)

    bench.hammer(open_conn, max(50, min(args.requests, 500)), args.concurrency)
    rates, reopened = [], 0
    for _ in range(args.reps):
        elapsed, _body, reconnects = bench.hammer(open_conn, args.requests, args.concurrency)
        rates.append(args.requests / elapsed)
        reopened += reconnects
    best = max(rates)
    return {
        "kind": "php-fpm-fcgi-direct",
        "label": "php-fpm (fcgi, no proxy)",
        "detail": "the FastCGI SAPI, driven as nginx drives it but with nginx removed",
        "generator": "bench.py",
        "requests_per_sec": round(best, 1),
        "median_requests_per_sec": round(statistics.median(rates), 1),
        "ms_per_request": round(1000.0 * args.concurrency / best, 4),
        "latency_ms": None,
        "success_rate": 1.0,
        "status_codes": {},
        "errors": {},
        "reconnects": reopened,
    }


def http_body(port: int, path: str = "/") -> bytes:
    """One GET from the host, for the agreement check. Never timed."""
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
    try:
        conn.request("GET", path)
        return conn.getresponse().read()
    finally:
        conn.close()


def fcgi_body() -> bytes:
    """Arm 4's answer, through the same client that times it."""
    conn = bench.FcgiConn(PHP_FCGI_PORT, CONTAINER_PHP)
    return conn.request()


# --------------------------------------------------------------------------------------
# The record
# --------------------------------------------------------------------------------------


def digests() -> dict:
    """A sha256 per tunable file, so a number can be tied to the configuration that produced it.

    Digests and the effective values below them, rather than the files' full text: the artifact
    keeps a hundred runs, the four files are two hundred lines of comments, and a hash plus the
    commit recovers the exact bytes from git whenever a row is actually questioned.
    """
    out = {}
    for path in TUNABLES:
        if path.is_file():
            out[str(path.relative_to(ROOT)).replace("\\", "/")] = hashlib.sha256(
                path.read_bytes()
            ).hexdigest()[:16]
    return out


def effective_php() -> dict:
    """The opcache keys that would move a number, parsed out of the committed ini."""
    wanted = {
        "opcache.enable", "opcache.jit", "opcache.jit_buffer_size",
        "opcache.validate_timestamps", "opcache.memory_consumption",
    }
    out = {}
    for line in (BENCH_DIR / "php" / "opcache.ini").read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith(";") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        if key.strip() in wanted:
            out[key.strip()] = value.strip()
    return out


def write_record(path: Path, record: dict) -> None:
    """Append to a JSON array, capped, exactly as the other leg's artifact is written."""
    bench.write_serve_record(path, record)


# --------------------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(
        description="nginx in front of both peers: nvs serve against PHP-FPM, in containers.",
        epilog="benches/proxied/README.md is the design; this tool is only how it is run.",
    )
    parser.add_argument(
        "--arm", choices=["fair", "deployed"], default="fair",
        help="fair: one core and one PHP worker each (the only language comparison). "
             "deployed: PHP gets a pool across --backend-cpus while nvs serve keeps its one core",
    )
    parser.add_argument(
        "--backend-cpus", type=int, default=None, metavar="N",
        help="the CPU budget given to EACH backend container (default 1 for fair, 4 for deployed). The PHP worker count is sized from --concurrency instead; php/pool.conf.in says why",
    )
    parser.add_argument("--requests", type=int, default=20000, help="requests per timed pass")
    parser.add_argument("--concurrency", type=int, default=8, help="concurrent connections")
    parser.add_argument("--reps", type=int, default=3, help="timed passes per arm, best wins")
    parser.add_argument("--record", metavar="PATH", help="append this run to a JSON array")
    parser.add_argument("--nvs-bin", metavar="PATH", help="a prebuilt Linux release nvs to measure")
    parser.add_argument("--allow-debug", action="store_true", help="permit a debug --nvs-bin")
    parser.add_argument(
        "--keep-up", action="store_true",
        help="leave the Novis stack running after the run, to poke at by hand. The PHP stack is "
             "always torn down: it comes first, and the Novis arms must not share a box with it",
    )
    parser.add_argument("--down", action="store_true", help="tear both stacks down and stop")
    args = parser.parse_args()

    require_docker()

    if args.down:
        for stack in (PHP_STACK, NVS_STACK):
            down(stack)
        print("both stacks are down")
        return 0

    if args.backend_cpus is None:
        args.backend_cpus = 1 if args.arm == "fair" else 4
    if args.arm == "fair" and args.backend_cpus != 1:
        sys.exit(
            "--arm fair is one core against one core; --backend-cpus "
            f"{args.backend_cpus} is the deployed arm and must say so"
        )
    if args.requests < 1 or args.concurrency < 1 or args.reps < 1:
        sys.exit("--requests, --concurrency and --reps must each be at least 1")

    commit = bench.version("git", ["rev-parse", "--short", "HEAD"])

    # The pool is sized by **concurrency**, never by the CPU budget. `php/pool.conf.in` owns why in
    # full; the short of it is that a FastCGI worker bound to a keep-alive connection accepts
    # nothing else, so one worker per core does not run slowly at concurrency 8 -- it hangs. The
    # floor of 96 clears the 64 idle upstream connections nginx can hold.
    php_children = max(2 * args.concurrency, 96)
    render_pool(php_children)

    # The same budget PHP's container gets, in both arms. `nvs serve` can only use one core of it
    # (one socket, one accept loop), and that is exactly the point: the allowance is equal and the
    # limit is Novis's own, so the deployed arm prices the unlanded fan-out rather than an
    # inequality this harness introduced.
    nvs_env = {"NVS_CPUS": str(args.backend_cpus), "NVS_BUILD_COMMIT": commit}
    php_env = {"BACKEND_CPUS": str(args.backend_cpus)}
    if args.nvs_bin:
        stage_binary(Path(args.nvs_bin), args.allow_debug)
        nvs_env["NVS_STAGE"] = "prebuilt"

    print(
        f"case {CASE}, arm {args.arm}: {args.requests} requests over {args.concurrency} "
        f"connection(s), {args.reps} rep(s), best of reps"
    )
    print(
        f"  backend budget: php {args.backend_cpus} cpu / {php_children} worker(s), "
        f"nvs serve {args.backend_cpus} cpu / 1 core; nginx 4 cpu on both sides"
    )
    print()

    arms: dict[str, dict] = {}
    bodies: dict[str, bytes] = {}

    # ---- the PHP stack, alone on the box -------------------------------------------------
    #
    # § Why the origin is measured before its proxy exists.
    #
    # The origin arm runs with only the backend up, and the proxy is started afterwards for the
    # proxied arm. On the PHP side this is not a preference -- it is the difference between a
    # number and a hang. In the fair arm FPM has **one** worker, and nginx holds its upstream
    # FastCGI connection open (`fastcgi_keep_conn on`, without which its `keepalive` pool is
    # inert). One worker bound to one held-open connection is a worker that never accepts a
    # second one, so a direct FastCGI client is accepted by the kernel's backlog and then waits
    # for a responder that will never come. It presents as a 30-second socket timeout while the
    # container's own log shows healthy 200s going past, which is about as misleading as a
    # failure gets.
    #
    # The Novis stack is phased the same way and does not need to be: `nvs serve` answers every
    # connection on its one core. It is done anyway because arm 3 against arm 4 is only a
    # comparison if the two were taken under the same conditions, and "the proxy was running for
    # one of them" is a difference.
    try:
        print("  bringing up php-fpm alone, for the origin arm ...")
        up(PHP_STACK, php_env, "php")
        bodies["php-fpm (fcgi, no proxy)"] = fcgi_body()
        arms["php-fpm-fcgi-direct"] = fcgi_arm(args)

        print("  bringing up nginx in front of it ...")
        up(PHP_STACK, php_env, "nginx")
        bodies["nginx -> php-fpm"] = http_body(PHP_NGINX_PORT)
        arms["nginx-php-fpm"] = oha_arm(
            PHP_STACK, "http://nginx/", args, php_env,
            "nginx-php-fpm", "nginx -> php-fpm", "PHP 8.5.9 FPM with opcache and tracing JIT",
        )
    except Exception as exc:
        raise RuntimeError(f"{exc}\nphp said:\n{logs_of(PHP_STACK, 'php')}") from exc
    finally:
        # Unconditional, and `--keep-up` deliberately does not reach it: the next stack is about to
        # take the same cores, and leaving this one running would make every number after it a
        # measurement of two stacks sharing a box. `--keep-up` keeps the *last* stack, which is the
        # one there is any reason to poke at afterwards.
        down(PHP_STACK)

    # ---- the Novis stack, alone on the box ------------------------------------------------
    try:
        print("  bringing up nvs serve alone (the first run builds the image) ...")
        up(NVS_STACK, nvs_env, "nvs")
        bodies["nvs serve (no proxy)"] = http_body(NVS_DIRECT_PORT)
        arms["nvs-serve-direct"] = oha_arm(
            NVS_STACK, "http://nvs:8080/", args, nvs_env,
            "nvs-serve-direct", "nvs serve (no proxy)", "the same origin with nginx removed",
        )

        print("  bringing up nginx in front of it ...")
        up(NVS_STACK, nvs_env, "nginx")
        bodies["nginx -> nvs serve"] = http_body(NVS_NGINX_PORT)
        arms["nginx-nvs-serve"] = oha_arm(
            NVS_STACK, "http://nginx/", args, nvs_env,
            "nginx-nvs-serve", "nginx -> nvs serve", "ADR 0097's proxied origin, one core",
        )
    except Exception as exc:
        raise RuntimeError(f"{exc}\nnvs said:\n{logs_of(NVS_STACK, 'nvs')}") from exc
    finally:
        if not args.keep_up:
            down(NVS_STACK)

    # ---- agreement, before any number is reported ------------------------------------------
    # The other leg's rule, and it is not a formality: a peer answering 404 fast is the easiest way
    # for a benchmark to report a large win. Every arm has to have sent the same bytes.
    if len(set(bodies.values())) > 1:
        print()
        print("DIFF: the arms did not answer with the same bytes, so no number here stands")
        for label, body in bodies.items():
            print(f"  {label} sent {body!r}")
        return 1

    # ---- report -----------------------------------------------------------------------------
    order = ["nginx-php-fpm", "nginx-nvs-serve", "nvs-serve-direct", "php-fpm-fcgi-direct"]
    width = max(len(arms[k]["label"]) for k in order)
    print()
    for key in order:
        a = arms[key]
        tail = ""
        if a["latency_ms"]:
            tail = f"  p99 {a['latency_ms']['p99']:>7.3f} ms"
        print(
            f"  {a['label']:<{width}}  {a['requests_per_sec']:>10,.0f} requests/sec"
            f"  {a['ms_per_request']:>8.3f} ms/request{tail}"
        )

    shipped = arms["nginx-nvs-serve"]["requests_per_sec"] / arms["nginx-php-fpm"]["requests_per_sec"]
    origin = (
        arms["nvs-serve-direct"]["requests_per_sec"]
        / arms["php-fpm-fcgi-direct"]["requests_per_sec"]
    )
    print()
    print(f"  as shipped, behind nginx: nvs serve is {shipped:.2f}x php-fpm")
    print(f"  origin against origin:    nvs serve is {origin:.2f}x php-fpm  (two generators)")

    caveats = [
        "nginx fronts both peers, which is the only deployment either has -- FPM speaks FastCGI "
        "and nvs serve is ADR 0097's proxied origin; the proxy's cost is in both numbers",
        "arm 4 is driven by tools/bench.py's FastCGI client and not by oha, because FPM has no "
        "HTTP origin to point a load generator at; origin-against-origin therefore crosses two "
        "generators and is the weaker of the two ratios",
        "this row shares no inputs with benches/serve.json -- different OS, generator, topology "
        "and CPU budget -- and no arithmetic between the two files is meaningful",
    ]
    if args.arm == "deployed":
        caveats.append(
            f"deployed arm: both containers are given {args.backend_cpus} cpus, but nvs serve can "
            "use only one of them -- binding a listener across cores is an unlanded slice "
            "(crates/nvs-cli/src/serve.rs), while php-fpm's process manager uses the whole budget. "
            "This prices that slice; it is not a language comparison"
        )
    else:
        caveats.append(
            f"fair arm: one cpu to each backend container, so the number is runtime against "
            f"runtime. The PHP pool is {php_children} workers, sized from the concurrency rather "
            f"than from the budget, because a FastCGI worker is bound to the keep-alive connection "
            f"it holds and a pool smaller than the open connections stops rather than slows"
        )
    print()
    for caveat in caveats:
        print(f"  note: {caveat}")

    record = {
        "date": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "commit": commit,
        "case": CASE,
        "arm": args.arm,
        "requests": args.requests,
        "concurrency": args.concurrency,
        "reps": args.reps,
        "backend_cpus": args.backend_cpus,
        "php_children": php_children,
        "nginx_cpus": 4,
        "ratio_shipped": round(shipped, 4),
        "ratio_origin": round(origin, 4),
        "caveats": caveats,
        "arms": arms,
        "images": {
            "php": "php:8.5.9-fpm",
            "nginx": "nginx:1.29.3-alpine",
            "oha": "ghcr.io/hatoo/oha@sha256:3ec3dbf5",
        },
        "php_ini": effective_php(),
        "config_digests": digests(),
        "nvs_source": "prebuilt" if args.nvs_bin else "built from this checkout",
        "host": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "cpus": os.cpu_count(),
        },
    }
    print()
    if args.record:
        write_record(Path(args.record), record)
    else:
        print("  not recorded: pass --record PATH to append this run to a JSON array")
    return 0


if __name__ == "__main__":
    sys.exit(main())
