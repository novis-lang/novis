#!/usr/bin/env python3
"""Point `nvs-db`'s own assertions at five real servers and say which one failed.

    python tools/db-matrix.py --all              # every driver, servers brought up first
    python tools/db-matrix.py --driver postgres  # one of them (repeatable)
    python tools/db-matrix.py --list             # the drivers and their endpoints, no daemon needed
    python tools/db-matrix.py --all --no-up      # the servers are already up
    python tools/db-matrix.py --down             # stop and destroy them

## What this is, and what it is not

It is a **harness, not a test**. Every assertion belongs to the crate that owns what it asserts,
written once against the shape `rule:core-classes/db-one-api`'s *Verification* section names; this file's whole job is to
point those suites at five endpoints, one driver at a time, and print one `<driver>: ok` line each.
Which suites, why they are no longer only `nvs-db`'s, and why one leg runs a suite the others do
not, is `SUITES` and `NO_SERVER_SUITES` below. `docs/agent/loop-goal.md`
§ *The harness this goal owes* is why it exists before the first driver rather than after: a driver
with no server to run against is a driver whose tests are all mocks.

One driver per `cargo test` process, deliberately. A single run with all five endpoints in the
environment would report "3 failed" and leave which server broke to a reader of the output; a
process per driver makes the answer the exit status of that process.

## The endpoint comes from `tests/db/compose.yaml`, never from here

Ports and credentials are read back out of the compose file with `docker compose config --format
json`, so this tool holds **no copy** of either. What it does hold is the mapping from a driver to
the environment-variable *names* that file uses -- `POSTGRES_USER` against `MARIADB_USER` against
`MSSQL_SA_PASSWORD` -- which is knowledge about the images and not a second home for a value.

`--list` therefore works with no Docker daemon running: reading the file needs the `docker compose`
CLI and nothing else.

## What the crate is handed, and why it is not a DSN

Discrete fields in the environment, never a connection string:

    NVS_DB_MATRIX_DRIVER     postgres | mysql | mariadb | mssql | sqlite
    NVS_DB_MATRIX_HOST       127.0.0.1
    NVS_DB_MATRIX_PORT       the published port, and the server's own on a socket leg
    NVS_DB_MATRIX_USER
    NVS_DB_MATRIX_PASSWORD
    NVS_DB_MATRIX_DATABASE
    NVS_DB_MATRIX_CA         the PEM bundle vouching for that server, exported below
    NVS_DB_MATRIX_PATH       sqlite only, a scratch file this tool creates and removes
    NVS_DB_MATRIX_SOCKET     the socket leg below, and unset for every other leg

`rule:core-classes/db-connection-is-named` makes `Db\\Settings` five types rather than one loose shape, and Novis has no DSN
anywhere in its surface. A harness that invented one would be the first place a DSN *parser* had to
exist, and the crate would then be tested through a spelling no program can use.

## The socket leg

Three drivers speak over `AF_UNIX` as well as over a port -- `rule:core-classes/db-unix-socket-path`
-- so after the five published-port legs those three run the driver's own case list a second time
with `NVS_DB_MATRIX_SOCKET` set instead of a host, and print `<driver> over a socket: ok`. The claim
a second transport has to make is that the driver answers the same over either, and only the same
cases over both say so; `SOCKET_SUITES` is which of `SUITES` that is and why the rest are not.
SQL Server refuses the transport and SQLite has no wire, so neither has a socket leg at all, rather
than a green line for something that did not run.

`tests/db/compose.yaml` publishes each of those servers' own socket directory onto the host, and the
leg is handed the path that driver's `host` is spelled as: the file for MySQL and MariaDB, the
directory for PostgreSQL. `NVS_DB_MATRIX_PORT` carries the server's **own** port rather than the
published one, because that is what `.s.PGSQL.<port>` is derived from and the other two ignore it.
There is no `NVS_DB_MATRIX_CA` in that group at all: nothing vouches for a socket and no handshake
over one asks.

**On Windows the leg runs inside WSL.** The Unix-domain half of `nvs_host::net` is `#[cfg(unix)]`,
so a Windows build has no transport for a socket whatever the kernel offers, and the socket a Linux
container publishes is reachable from the distro rather than from Windows. `cargo test` there is one
`wsl.exe` round trip over the distro's own target directory -- `/var/tmp/nvs-target-wsl`, which
`tools/loop.py`'s WSL leg keeps warm, and `NVS_DB_MATRIX_WSL_TARGET` overrides. On a Linux host
nothing is delegated: the leg is this process's own `cargo test`.

A case that finds `NVS_DB_MATRIX_DRIVER` unset is expected to return without asserting anything, so
`python tools/verify.py` stays green on a machine with no containers. That is the crate's rule, not
this file's, and `crates/nvs-db`'s module doc owns it.

## The trust anchor is exported from the container, per run

Every server in `tests/db/compose.yaml` is TLS-only and no public root vouches for any of them, so
a driver reaches its server against a private anchor or not at all -- `nvs_host::tls` has no
spelling for connecting without verifying. That anchor is a file inside a container rather than one
in this tree: `docker compose cp <service>:<path>` copies it into the same scratch directory the
run already builds, and `NVS_DB_MATRIX_CA` names the copy. Per run and not once, because the file
belongs to a Docker volume and is reissued whenever that volume is; `tests/db/ca.crt`, which
`nvs.toml` names for the *fixtures*, is a copy of the same certificate and is deliberately not in
git.

Where that path is per image is knowledge about the images, like the credential keys above:
PostgreSQL is served the `certs` service's CA, and MySQL issues its own at first boot. **Two
drivers have no anchor a client can be handed** -- MariaDB serves no certificate at all as this
compose file configures it, and SQL Server keeps its self-signed one in the instance rather than in
a file. Those two print `n/a` and are not run. Running them without an anchor would report a green
leg for a handshake that never happened, which is the one outcome a verification matrix must not
produce, and giving them one is Stage 6's work rather than this file's.

## Exit status

`0` only when every selected driver passed. `1` when one failed its assertions, `2` when a selected
driver could not run at all -- no `docker`, no daemon, a compose file that will not parse, a
`crates/nvs-db` that does not exist yet, or a server with no exportable trust anchor. A failure
outranks an `n/a` when both happened, because assertions that ran and disagreed are the more
actionable answer. Anything that did not run prints `<driver>: n/a` rather than `ok`, because a
harness that reports green when nothing ran is worse than one that fails.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPOSE = ROOT / "tests" / "db" / "compose.yaml"
CRATE = ROOT / "crates" / "nvs-db"
IS_WINDOWS = os.name == "nt"

#: The directory `tests/db/compose.yaml` publishes the three sockets into, as the process that dials
#: one sees it -- two spellings of one directory on Docker Desktop. The daemon runs in a WSL distro
#: of its own and resolves a bind mount inside *that* one, so the source the compose file names is
#: `/mnt/host/wsl/novis-db`, the shared tmpfs every distro mounts, and a leg inside the default
#: distro finds the same directory at `/mnt/wsl/novis-db`. That file's `redis` service owns the
#: explanation. On a Linux host the daemon and the client share one filesystem and the two spellings
#: are one path, which is the case `NOVIS_DB_SOCKET_DIR` is for: it names both ends at once.
SOCKET_DIR = os.environ.get("NOVIS_DB_SOCKET_DIR") or (
    "/mnt/wsl/novis-db" if IS_WINDOWS else "/mnt/host/wsl/novis-db"
)
#: Where a socket leg delegated to WSL builds. Not the tree's own `target/`, which is a Windows
#: directory reached over a 9p mount and holds another triple's artefacts; this is the directory
#: `tools/loop.py`'s WSL leg already keeps warm, so a leg costs a fingerprint scan rather than a
#: cold build.
WSL_TARGET = os.environ.get("NVS_DB_MATRIX_WSL_TARGET", "/var/tmp/nvs-target-wsl")

#: `docker compose up -d --wait` on SQL Server can legitimately take minutes: its healthcheck has a
#: 60s start period and forty retries, and the first boot creates the database.
UP_TIMEOUT = 600
#: One driver's assertions, including the `cargo` build the first of them pays for.
TEST_TIMEOUT = 900

#: The suites every driver leg runs, in order, as `cargo test` argument lists. The first failure
#: stops the leg, because the verdict is already decided.
#:
#: More than `nvs-db`'s own, because what a server has to answer no longer all lives in `nvs-db`:
#: ADR 0084's queue statements are `nvs_stdlib::queue`'s -- § 2's schema has one home and that is it
#: -- and `rule:core-classes/db-crate-boundary` forbids the `use nvs_stdlib::…` a `crates/nvs-db` test over them would need,
#: so they are run from `crates/nvs-stdlib/tests/queue.rs` and this is what reaches them.
#: `rule:core-classes/db-streaming`'s promises sit in that crate for a nearer reason: they are
#: `Core\Db\Connection::stream`'s rather than any one driver's read state, and one case asserting
#: them on whichever driver the leg named is what makes *answers on all five* a measurement.
#: `crates/nvs-stdlib/tests/db_stream.rs` is that case. And `nvs queue work` opens its own
#: connection rather than taking one from a pool, so the block that a worker reads as a dialect is
#: `crates/nvs-cli`'s to answer for; the last entry is the unit test that opens one.
#:
#: Every entry is narrowed to the targets that ask a server something, and the last one further to
#: the module: the rest of each crate's suite asks nothing, and every driver leg would pay for it.
#: The last entry names no `-p` because it does not need one -- a `--bin` names one target in one
#: package already, and a `-p` would resolve features over that package alone and rebuild the
#: workspace beside itself.
SUITES = (
    ["-p", "nvs-db"],
    ["-p", "nvs-stdlib", "--test", "queue"],
    ["-p", "nvs-stdlib", "--test", "db_stream"],
    ["--bin", "nvs", "worker::"],
)

#: What the driver with no server runs on top of `SUITES`, and nothing else does.
#:
#: `crates/nvs-stdlib/tests/queue_sqlite.rs` holds `rule:concurrency/claiming-is-one-statement`'s claim on the one backend
#: that arbitrates it with a transaction rather than a locking clause, and it reaches that backend
#: through `nvs_db::sqlite::open` and a scratch file rather than through `NVS_DB_MATRIX_*`. So it
#: asserts exactly the same thing on every leg, and only on this one is what it asserts about the
#: leg's own subject: on the four server legs it would run unchanged, pass unchanged, and say
#: nothing about the server that leg exists to question. Which is also why running it here is not
#: redundant with `python tools/verify.py` running it: this is the leg whose verdict is SQLite's.
NO_SERVER_SUITES = (
    ["-p", "nvs-stdlib", "--test", "queue_sqlite"],
)

#: What a socket leg runs, out of `SUITES`: this crate's own case list, and nothing else.
#:
#: The other three entries gate themselves out of a socket leg in their own source -- `queue.rs` and
#: `db_stream.rs` return without asserting on a `Location::Socket`, and the worker's case is SQL
#: Server's, which has no socket leg to be run on -- because each of them asserts a dialect or a
#: read state rather than a transport, and the same statements cross either one. Running them anyway
#: would be three more fingerprint scans per driver for three suites that assert nothing. So the leg
#: runs the case list that does speak about the transport: `crates/nvs-db/tests/handshake.rs` dials
#: whichever `Location` it is handed and asserts the server answers the same over it. A suite that
#: grows a case needing a socket joins this tuple in the slice that writes the case.
SOCKET_SUITES = (
    ["-p", "nvs-db"],
)


@dataclass(frozen=True)
class Driver:
    """One column of `rule:core-classes/db-one-api`'s matrix: a driver, and where its server is."""

    name: str
    #: The compose service to bring up, or `None` for the driver that has no server.
    service: str | None
    #: The private port the service publishes, matched against the compose file's mapping.
    port: int | None
    #: The compose environment keys holding the user, the password and the database — the names
    #: differ per image, and only the names live here.
    user_key: str | None = None
    password_key: str | None = None
    database_key: str | None = None
    #: The in-container path of the certificate that vouches for this server, copied out per run
    #: and handed over as `NVS_DB_MATRIX_CA`. `None` is a server whose certificate is not reachable
    #: as a file: that driver cannot be connected to and is reported `n/a` rather than run.
    anchor: str | None = None
    #: This driver's socket under [`SOCKET_DIR`], as `rule:core-classes/db-unix-socket-path` has
    #: that driver's `host` spelled -- the file for the two MySQL protocols, the directory for
    #: PostgreSQL. `None` is a driver with no socket leg: TDS has no `AF_UNIX` transport to reach
    #: and SQLite's own path *is* the database.
    socket: str | None = None
    #: A fixed user, for an image that names one in its command rather than its environment.
    user: str | None = None
    note: str = ""


DRIVERS: tuple[Driver, ...] = (
    # MySQL is served the `certs` volume's leaf rather than the one it generates into its data
    # directory at first boot: that one carries no `subjectAltName`, so it is unverifiable by name
    # however good its CA is, and `compose.yaml`'s MySQL block says so at length. The anchor is
    # therefore PostgreSQL's.
    Driver("mysql", "mysql", 3306, "MYSQL_USER", "MYSQL_PASSWORD", "MYSQL_DATABASE",
           anchor="/certs/ca.crt", socket="mysql/mysqld.sock"),
    # MariaDB is served the same `certs` leaf, and for the same reason MySQL is: 11.4 turns TLS on
    # by itself, but the certificate it generates to do that carries no `subjectAltName` and is
    # unverifiable by name however good its CA is. `compose.yaml`'s MariaDB block says so at length.
    Driver("mariadb", "mariadb", 3306, "MARIADB_USER", "MARIADB_PASSWORD", "MARIADB_DATABASE",
           anchor="/certs/ca.crt", socket="mariadb/mysqld.sock"),
    Driver("postgres", "postgres", 5432, "POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB",
           anchor="/certs/ca.crt", socket="postgres"),
    # SQL Server has no `MSSQL_USER`: the image's only account is `sa`, and the database is created
    # by the healthcheck rather than by the entrypoint — `compose.yaml`'s own comment says why. The
    # certificate it presents is the `certs` leaf too, but reaching that took an `mssql.conf` rather
    # than a flag, because the one it generates for itself lives inside the instance and no file on
    # any filesystem is a copy of it.
    Driver("mssql", "mssql", 1433, None, "MSSQL_SA_PASSWORD", None, user="sa",
           anchor="/certs/ca.crt"),
    # The one driver with no wire at all (`rule:security/one-tls-client`): a file this tool makes and removes.
    Driver("sqlite", None, None, note="a scratch file, no container"),
)

BY_NAME = {d.name: d for d in DRIVERS}


class Fail(Exception):
    """A reason the run cannot happen, rendered as one line and an exit status of 2."""


def say(line: str) -> None:
    """Progress, on stderr — stdout carries only the per-driver verdicts a check matches."""
    print(line, file=sys.stderr, flush=True)


def compose_config() -> dict:
    """`compose.yaml` as the CLI resolves it, so ports and credentials have one home.

    `config` needs the `docker compose` CLI and not the daemon, which is what lets `--list`
    answer on a machine with Docker installed and stopped.
    """
    if not shutil.which("docker"):
        raise Fail("the `docker` command is not on PATH -- Docker Desktop or the engine is needed")
    if not COMPOSE.is_file():
        raise Fail(f"{COMPOSE.relative_to(ROOT)} is missing")
    r = subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE), "config", "--format", "json"],
        capture_output=True, text=True, timeout=120,
    )
    if r.returncode != 0:
        first = (r.stderr.strip().splitlines() or ["no output"])[0]
        raise Fail(
            f"`docker compose config --format json` failed -- {first}. "
            "Compose v2 is required; `docker compose version` says which one is installed."
        )
    try:
        return json.loads(r.stdout)
    except json.JSONDecodeError as exc:
        raise Fail(f"`docker compose config` did not return JSON -- {exc}") from exc


def published_port(service: dict, private: int) -> int:
    """The host port `compose.yaml` maps to `private`, in whichever of the two spellings it used."""
    for entry in service.get("ports") or []:
        if isinstance(entry, dict):
            if int(entry.get("target", 0)) == private and entry.get("published"):
                return int(str(entry["published"]).rsplit(":", 1)[-1])
        elif isinstance(entry, str) and entry.endswith(f":{private}"):
            return int(entry.rsplit(":", 2)[-2])
    raise Fail(f"no published port for {private} in the compose file")


@dataclass
class Endpoint:
    """Where one driver's server is, and the credentials the compose file gave it."""

    host: str
    port: int
    user: str
    password: str
    database: str

    def describe(self) -> str:
        return f"{self.user}@{self.host}:{self.port}/{self.database}"


def endpoint_of(driver: Driver, config: dict) -> Endpoint:
    """Resolve one driver's endpoint out of the compose configuration."""
    services = config.get("services") or {}
    if driver.service not in services:
        raise Fail(f"{COMPOSE.relative_to(ROOT)} has no `{driver.service}` service")
    service = services[driver.service]
    env = service.get("environment") or {}
    if isinstance(env, list):  # the `KEY=value` spelling, which `config` normalises away but may not
        env = dict(item.split("=", 1) for item in env if "=" in item)

    def need(key: str) -> str:
        if key not in env:
            raise Fail(f"`{driver.service}` sets no `{key}` in {COMPOSE.relative_to(ROOT)}")
        return str(env[key])

    assert driver.port is not None
    return Endpoint(
        host="127.0.0.1",
        port=published_port(service, driver.port),
        user=driver.user or need(driver.user_key or ""),
        password=need(driver.password_key or ""),
        database=need(driver.database_key) if driver.database_key else "novis_test",
    )


def bring_up(services: list[str]) -> None:
    """`up -d --wait`, so "up" means *healthy* — `compose.yaml`'s own header owns that decision."""
    if not services:
        return
    say(f"db-matrix: bringing up {', '.join(services)} (this is a first-boot wait on a cold tree)")
    r = subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE), "up", "-d", "--wait", *services],
        timeout=UP_TIMEOUT,
    )
    if r.returncode != 0:
        raise Fail(
            "`docker compose up -d --wait` failed -- the daemon may be unreachable, or a service "
            "never became healthy. `docker compose -f tests/db/compose.yaml ps` says which."
        )


def bring_down() -> int:
    """Stop and destroy. There are no data volumes, so this is the whole cleanup."""
    say("db-matrix: docker compose down")
    return subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE), "down", "-v"], timeout=UP_TIMEOUT
    ).returncode


def export_anchor(driver: Driver, into: Path) -> Path:
    """Copy the certificate that vouches for this driver's server out of its container.

    Every run and not once: the file belongs to a Docker volume, is reissued whenever that volume
    is, and a copy kept in the tree is right only until the next `down -v`.
    """
    assert driver.anchor is not None and driver.service is not None
    dest = into / "ca.crt"
    source = f"{driver.service}:{driver.anchor}"
    r = subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE), "cp", source, str(dest)],
        capture_output=True, text=True, timeout=120,
    )
    if r.returncode != 0 or not dest.is_file():
        first = (r.stderr.strip().splitlines() or ["no output"])[0]
        raise Fail(f"`docker compose cp {source}` did not produce a trust anchor -- {first}")
    return dest


def matrix_env(fields: dict[str, str]) -> dict[str, str]:
    """This process's environment with every `NVS_DB_MATRIX_*` field replaced by `fields`.

    Replaced rather than merged: a leg reads those fields as a *group*, so one left over from the
    caller's own shell -- an anchor beside a socket, a host beside a path -- would point half a leg
    somewhere nobody chose, and `crates/nvs-db/src/matrix.rs` would report it as this harness being
    wrong. Everything else in the environment is carried: `cargo` needs its own.
    """
    env = {k: v for k, v in os.environ.items() if not k.startswith("NVS_DB_MATRIX_")}
    env.update(fields)
    return env


def cargo_test(suite: list[str], env: dict[str, str], in_wsl: bool = False) -> subprocess.CompletedProcess:
    """One suite of one leg, here or inside the default WSL distro.

    The distro is where a socket leg runs on Windows, for the reason this module's header gives, and
    it is a `bash -lc` because that is where the login PATH puts `cargo`. Only the `NVS_DB_MATRIX_*`
    group crosses -- the rest of `env` is this process's own environment, which the distro does not
    share and does not want.
    """
    if not in_wsl:
        return subprocess.run(
            ["cargo", "test", "-q", *suite],
            cwd=ROOT, env=env, capture_output=True, text=True, timeout=TEST_TIMEOUT,
        )
    drive = str(ROOT)[0].lower()
    repo = "/mnt/" + drive + str(ROOT)[2:].replace("\\", "/")
    fields = " ".join(
        f"{k}={shlex.quote(v)}" for k, v in sorted(env.items()) if k.startswith("NVS_DB_MATRIX_")
    )
    inner = (
        f"cd {shlex.quote(repo)} && CARGO_TARGET_DIR={shlex.quote(WSL_TARGET)} {fields} "
        f"cargo test -q {' '.join(shlex.quote(a) for a in suite)}"
    )
    return subprocess.run(
        ["wsl.exe", "--", "bash", "-lc", inner],
        capture_output=True, text=True, timeout=TEST_TIMEOUT,
    )


def verdict_of(r: subprocess.CompletedProcess) -> tuple[str, str]:
    """A finished suite as the (verdict, one-line detail) a leg reports."""
    if r.returncode == 0:
        return "ok", ""
    lines = [ln for ln in (r.stdout + r.stderr).splitlines() if ln.strip()]
    failed = [ln.strip() for ln in lines if ln.strip().startswith("---- ") or " FAILED" in ln]
    detail = failed[0] if failed else (lines[-1].strip() if lines else "no output")
    # The test's name says which assertion fired and never what the server answered, and a leg runs
    # captured, so the line under the panic is carried out with it or it is lost with the process.
    # One line, because this is a ledger entry: whoever needs the whole failure reruns the leg.
    panicked = next((i for i, ln in enumerate(lines) if "panicked at" in ln), None)
    if panicked is not None and panicked + 1 < len(lines):
        detail = f"{detail} -- {lines[panicked + 1].strip()}"
    return "FAILED", detail


def run_driver(driver: Driver, config: dict | None) -> tuple[str, str]:
    """Run one driver's suites -- `SUITES`, and `NO_SERVER_SUITES` after them for the driver that
    has no server. Returns (verdict, one-line detail).

    The verdict is `ok`, `FAILED` -- the assertions ran and disagreed -- or `n/a`, a driver whose
    server could not be reached at all. A server with no exportable trust anchor is the second of
    those and never the first: it is not connectable, so a run against it would assert nothing.
    """
    if driver.service is not None and driver.anchor is None:
        return "n/a", "no trust anchor: that server serves no certificate a client can verify"

    env = matrix_env({"NVS_DB_MATRIX_DRIVER": driver.name})
    scratch = Path(tempfile.mkdtemp(prefix="nvs-db-matrix-"))
    try:
        if driver.service is None:
            path = scratch / "matrix.sqlite"
            env["NVS_DB_MATRIX_PATH"] = str(path)
            where = str(path)
        else:
            assert config is not None
            endpoint = endpoint_of(driver, config)
            env["NVS_DB_MATRIX_HOST"] = endpoint.host
            env["NVS_DB_MATRIX_PORT"] = str(endpoint.port)
            env["NVS_DB_MATRIX_USER"] = endpoint.user
            env["NVS_DB_MATRIX_PASSWORD"] = endpoint.password
            env["NVS_DB_MATRIX_DATABASE"] = endpoint.database
            env["NVS_DB_MATRIX_CA"] = str(export_anchor(driver, scratch))
            where = endpoint.describe()

        say(f"db-matrix: {driver.name} against {where}")
        suites = SUITES if driver.service is not None else (*SUITES, *NO_SERVER_SUITES)
        for suite in suites:
            try:
                r = cargo_test(suite, env)
            except subprocess.TimeoutExpired:
                return "FAILED", f"no verdict within {TEST_TIMEOUT}s for `{' '.join(suite)}`"
            if r.returncode != 0:
                break
    except Fail as exc:
        # One driver's endpoint being unreadable stops that driver rather than the run: the other
        # four are still worth a verdict, and this one gets a line saying what was missing.
        return "n/a", str(exc)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

    return verdict_of(r)


def socket_published(path: str) -> bool:
    """Is the socket the compose file publishes there -- the file itself, or for PostgreSQL the
    directory the engine names its own socket inside?

    Asked from where the leg will dial, which on Windows is the distro and not this process: a
    missing socket is then one line saying the servers are not up, rather than a whole suite failing
    at a connect the reader has to recognise.
    """
    if IS_WINDOWS:
        probe = subprocess.run(
            ["wsl.exe", "--", "bash", "-lc", f"test -e {shlex.quote(path)}"],
            capture_output=True, text=True, timeout=120,
        )
        return probe.returncode == 0
    return Path(path).exists()


def run_socket_leg(driver: Driver, config: dict | None) -> tuple[str, str]:
    """`SUITES` again, with `driver` pointed at `AF_UNIX` rather than at a published port.

    The same case list and the same credentials as that driver's own leg above, because what this
    proves is that the driver answers the same over either transport. What changes is the group in
    the environment: a socket path instead of a host, the server's own port instead of the published
    one, and no trust anchor at all.
    """
    assert driver.socket is not None and driver.port is not None
    path = f"{SOCKET_DIR}/{driver.socket}"
    try:
        if IS_WINDOWS and not shutil.which("wsl.exe"):
            raise Fail(
                "no `wsl.exe` on this host, and a Windows build has no AF_UNIX transport to dial "
                "one with -- that half of `nvs_host::net` is `#[cfg(unix)]`"
            )
        if not socket_published(path):
            raise Fail(f"nothing is published at {path} -- the servers are not up, or not recreated"
                       " since `tests/db/compose.yaml` grew this mount")
        assert config is not None
        endpoint = endpoint_of(driver, config)
        env = matrix_env({
            "NVS_DB_MATRIX_DRIVER": driver.name,
            "NVS_DB_MATRIX_SOCKET": path,
            # The server's own port, never the published one: PostgreSQL derives `.s.PGSQL.<port>`
            # from it inside the container's namespace, and the other two ignore it.
            "NVS_DB_MATRIX_PORT": str(driver.port),
            "NVS_DB_MATRIX_USER": endpoint.user,
            "NVS_DB_MATRIX_PASSWORD": endpoint.password,
            "NVS_DB_MATRIX_DATABASE": endpoint.database,
        })
        say(f"db-matrix: {driver.name} against {path}")
        for suite in SOCKET_SUITES:
            try:
                r = cargo_test(suite, env, in_wsl=IS_WINDOWS)
            except subprocess.TimeoutExpired:
                return "FAILED", f"no verdict within {TEST_TIMEOUT}s for `{' '.join(suite)}`"
            if r.returncode != 0:
                break
    except Fail as exc:
        return "n/a", str(exc)
    return verdict_of(r)


@dataclass(frozen=True)
class Leg:
    """One line of the run: a driver, and which transport this pass reaches it over."""

    driver: Driver
    over_socket: bool

    @property
    def label(self) -> str:
        return f"{self.driver.name} over a socket" if self.over_socket else self.driver.name


def legs_of(selected: list[Driver]) -> list[Leg]:
    """Every selected driver's published-port leg, then the socket leg of each that has one.

    In that order rather than interleaved: the five drivers answer for themselves first, so a reader
    of the output -- or of an acceptance check's `want` list -- sees a transport failure as its own
    line and never as the driver's.
    """
    return [Leg(d, False) for d in selected] + [Leg(d, True) for d in selected if d.socket]


def main() -> int:
    ap = argparse.ArgumentParser(
        description="run nvs-db's assertions against `rule:core-classes/db-one-api`'s five drivers, one process each",
    )
    ap.add_argument("--all", action="store_true", help="every driver in the matrix")
    ap.add_argument("--driver", action="append", default=[], metavar="NAME",
                    help="one driver (repeatable): " + ", ".join(d.name for d in DRIVERS))
    ap.add_argument("--list", action="store_true", help="the drivers and their endpoints, then stop")
    ap.add_argument("--no-up", action="store_true", help="the servers are already running")
    ap.add_argument("--down", action="store_true", help="stop and destroy the servers, then stop")
    args = ap.parse_args()

    try:
        if args.down:
            return bring_down()

        selected = list(DRIVERS) if args.all or not args.driver else []
        for name in args.driver:
            if name not in BY_NAME:
                raise Fail(f"unknown driver {name!r} -- one of {', '.join(BY_NAME)}")
            if BY_NAME[name] not in selected:
                selected.append(BY_NAME[name])

        legs = legs_of(selected)

        # The missing crate is checked before Docker is: it is the cheaper answer and the more
        # useful one, and `--list` is the one mode that reads the compose file without it.
        if not args.list and not CRATE.is_dir():
            for leg in legs:
                print(f"{leg.label}: n/a (crates/nvs-db does not exist yet)")
            say("db-matrix: nothing ran -- the driver crate is Stage 2 of docs/agent/loop-goal.md")
            return 2

        # Every selection but a lone `sqlite` needs the compose file read.
        config = compose_config() if any(d.service for d in selected) else None

        if args.list:
            for leg in legs:
                if leg.over_socket:
                    where = f"{SOCKET_DIR}/{leg.driver.socket}"
                elif config is None or leg.driver.service is None:
                    where = leg.driver.note
                else:
                    where = endpoint_of(leg.driver, config).describe()
                print(f"{leg.label}: {where}")
            return 0

        if not args.no_up:
            # Only the servers a run can actually reach: waiting minutes for one that will be
            # reported `n/a` for want of a trust anchor buys nothing.
            bring_up([d.service for d in selected if d.service and d.anchor])

        failures = 0
        unrunnable = 0
        for leg in legs:
            run = run_socket_leg if leg.over_socket else run_driver
            verdict, detail = run(leg.driver, config)
            if verdict == "ok":
                print(f"{leg.label}: ok")
            elif verdict == "n/a":
                unrunnable += 1
                print(f"{leg.label}: n/a ({detail})")
            else:
                failures += 1
                print(f"{leg.label}: FAILED -- {detail}")
        ran = len(legs) - failures - unrunnable
        say(f"db-matrix: {ran}/{len(legs)} legs ok")
        if failures:
            return 1
        return 2 if unrunnable else 0
    except Fail as exc:
        say(f"db-matrix: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
