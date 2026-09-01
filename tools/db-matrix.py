#!/usr/bin/env python3
"""Point `nvs-db`'s own assertions at five real servers and say which one failed.

    python tools/db-matrix.py --all              # every driver, servers brought up first
    python tools/db-matrix.py --driver postgres  # one of them (repeatable)
    python tools/db-matrix.py --list             # the drivers and their endpoints, no daemon needed
    python tools/db-matrix.py --all --no-up      # the servers are already up
    python tools/db-matrix.py --down             # stop and destroy them

## What this is, and what it is not

It is a **harness, not a test**. Every assertion is `crates/nvs-db`'s own, written once against the
shape ADR 0067's *Verification* section names; this file's whole job is to point that suite at five
endpoints, one driver at a time, and print one `<driver>: ok` line each. `docs/agent/loop-goal.md`
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
    NVS_DB_MATRIX_PORT       the published port
    NVS_DB_MATRIX_USER
    NVS_DB_MATRIX_PASSWORD
    NVS_DB_MATRIX_DATABASE
    NVS_DB_MATRIX_PATH       sqlite only, a scratch file this tool creates and removes

ADR 0067 § 2 makes `Db\\Settings` five types rather than one loose shape, and Novis has no DSN
anywhere in its surface. A harness that invented one would be the first place a DSN *parser* had to
exist, and the crate would then be tested through a spelling no program can use.

A case that finds `NVS_DB_MATRIX_DRIVER` unset is expected to return without asserting anything, so
`python tools/verify.py` stays green on a machine with no containers. That is the crate's rule, not
this file's, and `crates/nvs-db`'s module doc owns it.

## Exit status

`0` only when every selected driver passed. `1` when one failed its assertions, `2` when the run
could not happen at all -- no `docker`, no daemon, a compose file that will not parse, or a
`crates/nvs-db` that does not exist yet. The missing crate prints `<driver>: n/a` rather than
`ok`, because a harness that reports green when nothing ran is worse than one that fails.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPOSE = ROOT / "tests" / "db" / "compose.yaml"
CRATE = ROOT / "crates" / "nvs-db"

#: `docker compose up -d --wait` on SQL Server can legitimately take minutes: its healthcheck has a
#: 60s start period and forty retries, and the first boot creates the database.
UP_TIMEOUT = 600
#: One driver's assertions, including the `cargo` build the first of them pays for.
TEST_TIMEOUT = 900


@dataclass(frozen=True)
class Driver:
    """One column of ADR 0067's matrix: a driver, and where its server is."""

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
    #: A fixed user, for an image that names one in its command rather than its environment.
    user: str | None = None
    note: str = ""


DRIVERS: tuple[Driver, ...] = (
    Driver("mysql", "mysql", 3306, "MYSQL_USER", "MYSQL_PASSWORD", "MYSQL_DATABASE"),
    Driver("mariadb", "mariadb", 3306, "MARIADB_USER", "MARIADB_PASSWORD", "MARIADB_DATABASE"),
    Driver("postgres", "postgres", 5432, "POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB"),
    # SQL Server has no `MSSQL_USER`: the image's only account is `sa`, and the database is created
    # by the healthcheck rather than by the entrypoint — `compose.yaml`'s own comment says why.
    Driver("mssql", "mssql", 1433, None, "MSSQL_SA_PASSWORD", None, user="sa"),
    # The one driver with no wire at all (ADR 0132 § 3): a file this tool makes and removes.
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


def run_driver(driver: Driver, config: dict | None) -> tuple[bool, str]:
    """Run `nvs-db`'s suite against one driver. Returns (passed, one-line detail)."""
    env = dict(os.environ)
    env["NVS_DB_MATRIX_DRIVER"] = driver.name
    scratch: Path | None = None
    if driver.service is None:
        scratch = Path(tempfile.mkdtemp(prefix="nvs-db-matrix-")) / "matrix.sqlite"
        env["NVS_DB_MATRIX_PATH"] = str(scratch)
        where = str(scratch)
    else:
        assert config is not None
        endpoint = endpoint_of(driver, config)
        env["NVS_DB_MATRIX_HOST"] = endpoint.host
        env["NVS_DB_MATRIX_PORT"] = str(endpoint.port)
        env["NVS_DB_MATRIX_USER"] = endpoint.user
        env["NVS_DB_MATRIX_PASSWORD"] = endpoint.password
        env["NVS_DB_MATRIX_DATABASE"] = endpoint.database
        where = endpoint.describe()

    say(f"db-matrix: {driver.name} against {where}")
    try:
        r = subprocess.run(
            ["cargo", "test", "-q", "-p", "nvs-db"],
            cwd=ROOT, env=env, capture_output=True, text=True, timeout=TEST_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return False, f"no verdict within {TEST_TIMEOUT}s"
    finally:
        if scratch is not None:
            shutil.rmtree(scratch.parent, ignore_errors=True)

    if r.returncode == 0:
        return True, ""
    lines = [ln for ln in (r.stdout + r.stderr).splitlines() if ln.strip()]
    failed = [ln.strip() for ln in lines if ln.strip().startswith("---- ") or " FAILED" in ln]
    detail = failed[0] if failed else (lines[-1].strip() if lines else "no output")
    return False, detail


def main() -> int:
    ap = argparse.ArgumentParser(
        description="run nvs-db's assertions against ADR 0067's five drivers, one process each",
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

        # The missing crate is checked before Docker is: it is the cheaper answer and the more
        # useful one, and `--list` is the one mode that reads the compose file without it.
        if not args.list and not CRATE.is_dir():
            for driver in selected:
                print(f"{driver.name}: n/a (crates/nvs-db does not exist yet)")
            say("db-matrix: nothing ran -- the driver crate is Stage 2 of docs/agent/loop-goal.md")
            return 2

        # Every selection but a lone `sqlite` needs the compose file read.
        config = compose_config() if any(d.service for d in selected) else None

        if args.list:
            for driver in selected:
                where = driver.note if config is None or driver.service is None \
                    else endpoint_of(driver, config).describe()
                print(f"{driver.name}: {where}")
            return 0

        if not args.no_up:
            bring_up([d.service for d in selected if d.service])

        failures = 0
        for driver in selected:
            passed, detail = run_driver(driver, config)
            if passed:
                print(f"{driver.name}: ok")
            else:
                failures += 1
                print(f"{driver.name}: FAILED -- {detail}")
        say(f"db-matrix: {len(selected) - failures}/{len(selected)} drivers ok")
        return 1 if failures else 0
    except Fail as exc:
        say(f"db-matrix: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
