// `bun nv db-matrix`: point `nvs-db`'s own assertions at five real servers and say which one failed.
//
//     bun nv db-matrix --all              every driver, servers brought up first
//     bun nv db-matrix --driver postgres  one of them (repeatable)
//     bun nv db-matrix --list             the drivers and their endpoints, no daemon needed
//     bun nv db-matrix --all --no-up      the servers are already up
//     bun nv db-matrix --down             stop and destroy them
//
// It is a harness, not a test. Every assertion belongs to the crate that owns what it asserts, written
// once against the shape `rule:core-classes/db-one-api`'s *Verification* section names. This command
// points those suites at five endpoints, one driver at a time, and prints one `<driver>: ok` line each.
// `SUITES`, `NO_SERVER_SUITES` and `SOCKET_SUITES` say which suites each leg runs, and why.
//
// One driver per `cargo test` process. A single run with all five endpoints in the environment would
// report "3 failed" and leave which server broke to a reader of the output; a process per driver makes
// the answer the exit status of that process.
//
// The endpoint comes from `tests/db/compose.yaml`, never from here. Ports and credentials are read back
// out of it with `docker compose config --format json`, so this file holds no copy of either. What it
// holds is the mapping from a driver to the environment-variable names that file uses, `POSTGRES_USER`
// against `MARIADB_USER` against `MSSQL_SA_PASSWORD`, which is knowledge about the images and not a
// second home for a value. `--list` therefore works with no Docker daemon running: reading the file
// needs the `docker compose` CLI and nothing else.
//
// The crate is handed discrete fields in the environment, never a connection string:
//
//     NVS_DB_MATRIX_DRIVER     postgres | mysql | mariadb | mssql | sqlite
//     NVS_DB_MATRIX_HOST       127.0.0.1
//     NVS_DB_MATRIX_PORT       the published port, and the server's own on a socket leg
//     NVS_DB_MATRIX_USER
//     NVS_DB_MATRIX_PASSWORD
//     NVS_DB_MATRIX_DATABASE
//     NVS_DB_MATRIX_CA         the PEM bundle vouching for that server, exported per run
//     NVS_DB_MATRIX_PATH       sqlite only, a scratch file this command creates and removes
//     NVS_DB_MATRIX_SOCKET     the socket leg, and unset on every other leg
//
// `rule:core-classes/db-connection-is-named` makes `Db\Settings` five types rather than one loose shape,
// and Novis has no DSN anywhere in its surface. A harness that invented one would be the first place a
// DSN parser had to exist, and the crate would then be tested through a spelling no program can use.
//
// Three drivers speak over `AF_UNIX` as well as over a port (`rule:core-classes/db-unix-socket-path`),
// so after the published-port legs those three run their case list a second time with
// `NVS_DB_MATRIX_SOCKET` set instead of a host, and print `<driver> over a socket: ok`. The compose file
// publishes each server's socket directory onto the host, and the leg is handed the path that driver's
// `host` is written as: the file for MySQL and MariaDB, the directory for PostgreSQL. The port is the
// server's own rather than the published one, because `.s.PGSQL.<port>` is derived from it, and no
// anchor is handed over, because nothing vouches for a socket. SQL Server has no such transport and
// SQLite has no wire, so neither has a socket leg at all. On Windows that leg runs inside WSL: the
// Unix-domain half of `nvs_host::net` is `#[cfg(unix)]`, and the socket a Linux container publishes is
// reachable from the distro. `cargo test` there is one `wsl.exe` round trip over the distro's own target
// directory, `WSL_TARGET`. On a Linux host the leg is this process's own `cargo test`.
//
// Every server is TLS-only and no public root vouches for any of them, so a driver reaches its server
// against a private anchor or not at all. That anchor is a file inside a container: `docker compose cp`
// copies it into the leg's scratch directory, and `NVS_DB_MATRIX_CA` names the copy. It is copied per
// run because the file belongs to a Docker volume and is reissued whenever that volume is. A driver with
// no anchor is reported `n/a` and not run, because running it would report a green leg for a handshake
// that never happened.
//
// A case that finds `NVS_DB_MATRIX_DRIVER` unset returns without asserting anything, so `bun nv verify`
// stays green on a machine with no containers. That is the crate's rule, and `crates/nvs-db`'s module doc
// owns it.
//
// Exit status: 0 only when every selected leg passed. 1 when one failed its assertions, 2 when one could
// not run at all: no `docker`, no daemon, a compose file that will not parse, no `crates/nvs-db`, or a
// server with no exportable trust anchor. A failure outranks an `n/a` when both happened, because
// assertions that ran and disagreed are the more actionable answer. Progress goes to stderr; stdout
// carries only the per-leg verdicts an acceptance check matches.

import { existsSync, mkdirSync, mkdtempSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { Tree } from "../driver/proctree.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough } from "../lib/proc.ts";
import { ArgError, parseArgs, pyRepr } from "../lib/py.ts";

export const summary = "nvs-db's assertions against five real servers: nv db-matrix --all | --driver NAME | --list | --down";

const USAGE = "usage: nv db-matrix [-h] [--all] [--driver NAME] [--list] [--no-up] [--down]";
const COMPOSE_REL = "tests/db/compose.yaml";
const COMPOSE = join(ROOT, COMPOSE_REL);
const CRATE = join(ROOT, "crates", "nvs-db");
const IS_WINDOWS = process.platform === "win32";

/** Where a leg's scratch directory goes: inside the tree, where every scratch file of this repository is. */
const SCRATCH = join(ROOT, ".agent-tmp", "db-matrix");

/**
 * The directory the compose file publishes the three sockets into, as the process that dials one sees
 * it. On Docker Desktop the daemon runs in a WSL distro of its own and resolves a bind mount inside that
 * one, so the source the compose file names is `/mnt/host/wsl/novis-db`, the shared tmpfs every distro
 * mounts, and a leg inside the default distro finds the same directory at `/mnt/wsl/novis-db`. The
 * compose file's `redis` service owns that explanation. On a Linux host the two are one path, which is
 * what `NOVIS_DB_SOCKET_DIR` is for: it names both ends at once.
 */
const SOCKET_DIR = process.env.NOVIS_DB_SOCKET_DIR || (IS_WINDOWS ? "/mnt/wsl/novis-db" : "/mnt/host/wsl/novis-db");

/**
 * Where a socket leg delegated to WSL builds. Not the tree's own `target/`, which is a Windows directory
 * reached over a 9p mount and holds another triple's artefacts; this is the directory the loop's WSL leg
 * keeps warm, so a leg costs a fingerprint scan rather than a cold build.
 */
const WSL_TARGET = process.env.NVS_DB_MATRIX_WSL_TARGET ?? "/var/tmp/nvs-target-wsl";

/** `docker compose up -d --wait` on SQL Server can take minutes: its healthcheck has a long start period. */
const UP_TIMEOUT_S = 600;
/** One suite of one leg, including the `cargo` build the first of them pays for. */
const TEST_TIMEOUT_S = 900;

/**
 * The suites every driver leg runs, in order, as `cargo test` argument lists. The first failure stops
 * the leg, because the verdict is already decided.
 *
 * More than `nvs-db`'s own, because what a server has to answer does not all live in `nvs-db`. The queue
 * statements are `nvs_stdlib::queue`'s, and `rule:core-classes/db-crate-boundary` forbids the `use
 * nvs_stdlib::…` a `crates/nvs-db` test over them would need, so `crates/nvs-stdlib/tests/queue.rs` runs
 * them. `rule:core-classes/db-streaming`'s promises are `Core\Db\Connection::stream`'s rather than any one
 * driver's, and `crates/nvs-stdlib/tests/db_stream.rs` asserts them on whichever driver the leg names.
 * `nvs queue work` opens its own connection, so the block a worker reads as a dialect is `crates/nvs-cli`'s
 * to answer for, and the last entry is the unit test that opens one.
 *
 * Every entry is narrowed to the targets that ask a server something. The last names no `-p`: a `--bin`
 * already names one target in one package, and a `-p` would resolve features over that package alone
 * and rebuild the workspace beside itself.
 */
const SUITES: string[][] = [
  ["-p", "nvs-db"],
  ["-p", "nvs-stdlib", "--test", "queue"],
  ["-p", "nvs-stdlib", "--test", "db_stream"],
  ["--bin", "nvs", "worker::"],
];

/**
 * What the driver with no server runs after `SUITES`, and no other leg does.
 *
 * `crates/nvs-stdlib/tests/queue_sqlite.rs` holds `rule:concurrency/claiming-is-one-statement`'s claim on
 * the one backend that arbitrates it with a transaction rather than a locking clause, and it reaches
 * that backend through a scratch file rather than through `NVS_DB_MATRIX_*`. It asserts the same thing on
 * every leg, and only on this one is that about the leg's own subject.
 */
const NO_SERVER_SUITES: string[][] = [["-p", "nvs-stdlib", "--test", "queue_sqlite"]];

/**
 * What the SQLite leg runs, and the environment it runs with, `{scratch}` standing for a fresh directory.
 * The acceptance sweep runs these suites again on the covws build to learn which code the matrix runs
 * (`select/checks.ts` `heavyTwin`), because this leg needs no server.
 */
export function sqliteLeg(): { suites: string[][]; env: Record<string, string> } {
  return { suites: [...SUITES, ...NO_SERVER_SUITES], env: { NVS_DB_MATRIX_DRIVER: "sqlite", NVS_DB_MATRIX_PATH: "{scratch}/matrix.sqlite" } };
}

/**
 * What a socket leg runs: `nvs-db`'s own case list, and nothing else. `queue.rs` and `db_stream.rs`
 * return without asserting on a `Location::Socket`, and the worker's case is SQL Server's, because each
 * asserts a dialect or a read state rather than a transport. `crates/nvs-db/tests/handshake.rs` dials
 * whichever `Location` it is handed. A suite that grows a case needing a socket joins this list in the
 * slice that writes the case.
 */
const SOCKET_SUITES: string[][] = [["-p", "nvs-db"]];

/** One column of `rule:core-classes/db-one-api`'s matrix: a driver, and where its server is. */
interface Driver {
  name: string;
  /** The compose service to bring up, or null for the driver that has no server. */
  service: string | null;
  /** The private port the service publishes, matched against the compose file's mapping. */
  port: number | null;
  /** The compose environment keys holding the user, the password and the database. Only the names live here. */
  userKey?: string;
  passwordKey?: string;
  databaseKey?: string;
  /** The in-container path of the certificate that vouches for this server. Absent is a driver reported `n/a`. */
  anchor?: string;
  /** This driver's socket under `SOCKET_DIR`. Absent is a driver with no socket leg. */
  socket?: string;
  /** A fixed user, for an image that names one in its command rather than its environment. */
  user?: string;
  note?: string;
}

const DRIVERS: Driver[] = [
  // MySQL and MariaDB are served the `certs` volume's leaf rather than the certificate each generates
  // for itself, which carries no `subjectAltName` and cannot be verified by name. The compose file's
  // blocks for both say so.
  {
    name: "mysql", service: "mysql", port: 3306, userKey: "MYSQL_USER", passwordKey: "MYSQL_PASSWORD",
    databaseKey: "MYSQL_DATABASE", anchor: "/certs/ca.crt", socket: "mysql/mysqld.sock",
  },
  {
    name: "mariadb", service: "mariadb", port: 3306, userKey: "MARIADB_USER", passwordKey: "MARIADB_PASSWORD",
    databaseKey: "MARIADB_DATABASE", anchor: "/certs/ca.crt", socket: "mariadb/mysqld.sock",
  },
  {
    name: "postgres", service: "postgres", port: 5432, userKey: "POSTGRES_USER", passwordKey: "POSTGRES_PASSWORD",
    databaseKey: "POSTGRES_DB", anchor: "/certs/ca.crt", socket: "postgres",
  },
  // SQL Server's only account is `sa`, and its healthcheck creates the database. It presents the
  // `certs` leaf through an `mssql.conf`, which the compose file's comment explains.
  { name: "mssql", service: "mssql", port: 1433, passwordKey: "MSSQL_SA_PASSWORD", user: "sa", anchor: "/certs/ca.crt" },
  // The one driver with no wire at all (`rule:security/one-tls-client`): a file this command makes and removes.
  { name: "sqlite", service: null, port: null, note: "a scratch file, no container" },
];

/** A reason a leg or the run cannot happen, rendered as one line and an exit status of 2. */
class Fail extends Error {}

interface Result {
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

/** Progress, on stderr: stdout carries only the per-leg verdicts a check matches. */
function say(line: string): void {
  console.error(line);
}

/**
 * Runs `argv` under exactly `env`, not merged into this process's own: a leg reads the
 * `NVS_DB_MATRIX_*` fields as a group, so one left over from the caller's shell would point half a leg
 * somewhere nobody chose.
 */
async function spawn(argv: string[], env: Record<string, string>, timeoutS: number): Promise<Result> {
  const child = Bun.spawn(argv, { cwd: ROOT, env, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const tree = new Tree(child);
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    tree.kill();
  }, timeoutS * 1000);
  try {
    const [stdout, stderr, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    return { code, stdout, stderr, timedOut };
  } finally {
    clearTimeout(timer);
    tree.close();
  }
}

function baseEnv(): Record<string, string> {
  const env: Record<string, string> = {};
  for (const [k, v] of Object.entries(process.env)) if (v !== undefined) env[k] = v;
  return env;
}

/** This process's environment with every `NVS_DB_MATRIX_*` field replaced by `fields`. */
function matrixEnv(fields: Record<string, string>): Record<string, string> {
  const env: Record<string, string> = {};
  for (const [k, v] of Object.entries(baseEnv())) if (!k.startsWith("NVS_DB_MATRIX_")) env[k] = v;
  return { ...env, ...fields };
}

/** The first non-blank line of `text`, or `no output`. */
function firstLine(text: string): string {
  return text.trim().split(/\r?\n/)[0] || "no output";
}

/** A word as a POSIX shell reads it back unchanged. */
function shQuote(s: string): string {
  if (s !== "" && /^[\w@%+=:,./-]+$/.test(s)) return s;
  return `'${s.replace(/'/g, `'"'"'`)}'`;
}

type Config = { services?: Record<string, { ports?: unknown[]; environment?: Record<string, unknown> | string[] }> };

/**
 * `compose.yaml` as the CLI resolves it, so ports and credentials have one home. `config` needs the
 * `docker compose` CLI and not the daemon.
 */
async function composeConfig(): Promise<Config> {
  if (!Bun.which("docker")) throw new Fail("the `docker` command is not on PATH -- Docker Desktop or the engine is needed");
  if (!existsSync(COMPOSE) || !statSync(COMPOSE).isFile()) throw new Fail(`${COMPOSE_REL} is missing`);
  const r = await spawn(["docker", "compose", "-f", COMPOSE, "config", "--format", "json"], baseEnv(), 120);
  if (r.code !== 0 || r.timedOut) {
    throw new Fail(
      `\`docker compose config --format json\` failed -- ${firstLine(r.stderr)}. ` +
        "Compose v2 is required; `docker compose version` says which one is installed.",
    );
  }
  try {
    return JSON.parse(r.stdout) as Config;
  } catch (e) {
    throw new Fail(`\`docker compose config\` did not return JSON -- ${(e as Error).message}`);
  }
}

/** The host port the compose file maps to `target`, in whichever of the two ways it was written. */
function publishedPort(ports: unknown[] | undefined, target: number): number {
  for (const entry of ports ?? []) {
    if (entry && typeof entry === "object") {
      const e = entry as { target?: unknown; published?: unknown };
      if (Number(e.target ?? 0) === target && e.published) return Number(String(e.published).split(":").pop());
    } else if (typeof entry === "string" && entry.endsWith(`:${target}`)) {
      const parts = entry.split(":");
      return Number(parts[parts.length - 2]);
    }
  }
  throw new Fail(`no published port for ${target} in the compose file`);
}

interface Endpoint {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
}

const describe = (e: Endpoint) => `${e.user}@${e.host}:${e.port}/${e.database}`;

/** One driver's endpoint, read out of the compose configuration. */
function endpointOf(driver: Driver, config: Config): Endpoint {
  const services = config.services ?? {};
  const service = driver.service === null ? undefined : services[driver.service];
  if (!service) throw new Fail(`${COMPOSE_REL} has no \`${driver.service}\` service`);
  let env: Record<string, unknown> = {};
  if (Array.isArray(service.environment)) {
    // The `KEY=value` form, which `config` normalises away but may not.
    for (const item of service.environment) {
      const eq = item.indexOf("=");
      if (eq >= 0) env[item.slice(0, eq)] = item.slice(eq + 1);
    }
  } else env = service.environment ?? {};
  const need = (key: string): string => {
    if (!(key in env)) throw new Fail(`\`${driver.service}\` sets no \`${key}\` in ${COMPOSE_REL}`);
    return String(env[key]);
  };
  return {
    host: "127.0.0.1",
    port: publishedPort(service.ports, driver.port!),
    user: driver.user ?? need(driver.userKey ?? ""),
    password: need(driver.passwordKey ?? ""),
    database: driver.databaseKey ? need(driver.databaseKey) : "novis_test",
  };
}

/** `up -d --wait`, so "up" means healthy. The compose file's own header owns that decision. */
async function bringUp(services: string[]): Promise<void> {
  if (services.length === 0) return;
  say(`db-matrix: bringing up ${services.join(", ")} (this is a first-boot wait on a cold tree)`);
  const code = await passthrough(["docker", "compose", "-f", COMPOSE, "up", "-d", "--wait", ...services], {
    timeoutMs: UP_TIMEOUT_S * 1000,
  });
  if (code !== 0) {
    throw new Fail(
      "`docker compose up -d --wait` failed -- the daemon may be unreachable, or a service " +
        "never became healthy. `docker compose -f tests/db/compose.yaml ps` says which.",
    );
  }
}

/** Stop and destroy. There are no data volumes, so this is the whole cleanup. */
async function bringDown(): Promise<number> {
  say("db-matrix: docker compose down");
  return passthrough(["docker", "compose", "-f", COMPOSE, "down", "-v"], { timeoutMs: UP_TIMEOUT_S * 1000 });
}

/** Copies the certificate that vouches for this driver's server out of its container, into `dir`. */
async function exportAnchor(driver: Driver, dir: string): Promise<string> {
  const dest = join(dir, "ca.crt");
  const source = `${driver.service}:${driver.anchor}`;
  const r = await spawn(["docker", "compose", "-f", COMPOSE, "cp", source, dest], baseEnv(), 120);
  if (r.code !== 0 || r.timedOut || !existsSync(dest)) {
    throw new Fail(`\`docker compose cp ${source}\` did not produce a trust anchor -- ${firstLine(r.stderr)}`);
  }
  return dest;
}

/**
 * `bash -lc <line>` inside the default WSL distro. `wsl.exe --exec` starts `bash` directly. `wsl.exe --`
 * would first hand the line to the distro's default shell, which expands every `$` and strips every quote
 * in it before `bash` sees it.
 */
function inDistro(line: string): string[] {
  return ["wsl.exe", "--exec", "bash", "-lc", line];
}

/**
 * One suite of one leg, here or inside the default WSL distro. The distro runs `bash -lc` because that is
 * where the login PATH puts `cargo`, and only the `NVS_DB_MATRIX_*` group crosses into it.
 */
async function cargoTest(suite: string[], env: Record<string, string>, inWsl = false): Promise<Result> {
  if (!inWsl) return spawn(["cargo", "test", "-q", ...suite], env, TEST_TIMEOUT_S);
  const repo = `/mnt/${ROOT[0]!.toLowerCase()}${ROOT.slice(2).replaceAll("\\", "/")}`;
  const fields = Object.keys(env)
    .filter((k) => k.startsWith("NVS_DB_MATRIX_"))
    .sort()
    .map((k) => `${k}=${shQuote(env[k]!)}`)
    .join(" ");
  const inner =
    `cd ${shQuote(repo)} && CARGO_TARGET_DIR=${shQuote(WSL_TARGET)} ${fields} ` +
    `cargo test -q ${suite.map(shQuote).join(" ")}`;
  return spawn(inDistro(inner), baseEnv(), TEST_TIMEOUT_S);
}

type Verdict = ["ok" | "FAILED" | "n/a", string];

/** A finished suite as the verdict and one-line detail a leg reports. */
function verdictOf(r: Result): Verdict {
  if (r.code === 0) return ["ok", ""];
  const lines = `${r.stdout}${r.stderr}`.split(/\r?\n/).filter((l) => l.trim());
  const failed = lines.map((l) => l.trim()).filter((l) => l.startsWith("---- ") || l.includes(" FAILED"));
  let detail = failed[0] ?? (lines.length ? lines[lines.length - 1]!.trim() : "no output");
  // The test's name says which assertion fired and never what the server answered, and a leg runs
  // captured, so the line under the panic is carried out with it. One line, because this is a ledger
  // entry: whoever needs the whole failure reruns the leg.
  const panicked = lines.findIndex((l) => l.includes("panicked at"));
  if (panicked >= 0 && panicked + 1 < lines.length) detail = `${detail} -- ${lines[panicked + 1]!.trim()}`;
  return ["FAILED", detail];
}

/** Runs `suites` in order and stops at the first that fails. */
async function runSuites(suites: string[][], env: Record<string, string>, inWsl = false): Promise<Verdict> {
  let r: Result | undefined;
  for (const suite of suites) {
    r = await cargoTest(suite, env, inWsl);
    if (r.timedOut) return ["FAILED", `no verdict within ${TEST_TIMEOUT_S}s for \`${suite.join(" ")}\``];
    if (r.code !== 0) break;
  }
  return verdictOf(r!);
}

/**
 * One driver over its published port: `SUITES`, and `NO_SERVER_SUITES` after them for the driver with no
 * server. A server with no exportable trust anchor is `n/a` and never run, because a run against it
 * would assert nothing.
 */
async function runDriver(driver: Driver, config: Config | null): Promise<Verdict> {
  if (driver.service !== null && !driver.anchor) {
    return ["n/a", "no trust anchor: that server serves no certificate a client can verify"];
  }
  mkdirSync(SCRATCH, { recursive: true });
  const scratch = mkdtempSync(join(SCRATCH, "leg-"));
  try {
    const env = matrixEnv({ NVS_DB_MATRIX_DRIVER: driver.name });
    let where: string;
    if (driver.service === null) {
      where = join(scratch, "matrix.sqlite");
      env.NVS_DB_MATRIX_PATH = where;
    } else {
      const endpoint = endpointOf(driver, config!);
      env.NVS_DB_MATRIX_HOST = endpoint.host;
      env.NVS_DB_MATRIX_PORT = String(endpoint.port);
      env.NVS_DB_MATRIX_USER = endpoint.user;
      env.NVS_DB_MATRIX_PASSWORD = endpoint.password;
      env.NVS_DB_MATRIX_DATABASE = endpoint.database;
      env.NVS_DB_MATRIX_CA = await exportAnchor(driver, scratch);
      where = describe(endpoint);
    }
    say(`db-matrix: ${driver.name} against ${where}`);
    return await runSuites(driver.service !== null ? SUITES : [...SUITES, ...NO_SERVER_SUITES], env);
  } catch (e) {
    // One driver's endpoint being unreadable stops that driver rather than the run: the others are
    // still worth a verdict, and this one gets a line saying what was missing.
    if (e instanceof Fail) return ["n/a", e.message];
    throw e;
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/**
 * Is the socket published there: the file itself, or for PostgreSQL the directory the engine names its
 * socket inside? Asked from where the leg dials, which on Windows is the distro.
 */
async function socketPublished(path: string): Promise<boolean> {
  if (!IS_WINDOWS) return existsSync(path);
  const probe = await spawn(inDistro(`test -e ${shQuote(path)}`), baseEnv(), 120);
  return probe.code === 0;
}

/**
 * `SOCKET_SUITES`, with the driver pointed at `AF_UNIX`: the same credentials as the driver's own leg, a
 * socket path instead of a host, the server's own port, and no trust anchor.
 */
async function runSocketLeg(driver: Driver, config: Config | null): Promise<Verdict> {
  const path = `${SOCKET_DIR}/${driver.socket}`;
  try {
    if (IS_WINDOWS && !Bun.which("wsl.exe")) {
      throw new Fail(
        "no `wsl.exe` on this host, and a Windows build has no AF_UNIX transport to dial " +
          "one with -- that half of `nvs_host::net` is `#[cfg(unix)]`",
      );
    }
    if (!(await socketPublished(path))) {
      throw new Fail(
        `nothing is published at ${path} -- the servers are not up, or not recreated` +
          ` since \`${COMPOSE_REL}\` grew this mount`,
      );
    }
    const endpoint = endpointOf(driver, config!);
    const env = matrixEnv({
      NVS_DB_MATRIX_DRIVER: driver.name,
      NVS_DB_MATRIX_SOCKET: path,
      // The server's own port: PostgreSQL derives `.s.PGSQL.<port>` from it, and the other two ignore it.
      NVS_DB_MATRIX_PORT: String(driver.port),
      NVS_DB_MATRIX_USER: endpoint.user,
      NVS_DB_MATRIX_PASSWORD: endpoint.password,
      NVS_DB_MATRIX_DATABASE: endpoint.database,
    });
    say(`db-matrix: ${driver.name} against ${path}`);
    return await runSuites(SOCKET_SUITES, env, IS_WINDOWS);
  } catch (e) {
    if (e instanceof Fail) return ["n/a", e.message];
    throw e;
  }
}

interface Leg {
  driver: Driver;
  overSocket: boolean;
}

const labelOf = (leg: Leg) => (leg.overSocket ? `${leg.driver.name} over a socket` : leg.driver.name);

/**
 * Every selected driver's published-port leg, then the socket leg of each that has one. Not interleaved,
 * so a reader of the output, or of a check's `want` list, sees a transport failure as its own line and
 * never as the driver's.
 */
function legsOf(selected: Driver[]): Leg[] {
  return [
    ...selected.map((driver) => ({ driver, overSocket: false })),
    ...selected.filter((d) => d.socket).map((driver) => ({ driver, overSocket: true })),
  ];
}

function help(): string {
  return [
    USAGE,
    "",
    "run nvs-db's assertions against `rule:core-classes/db-one-api`'s five drivers, one process each",
    "",
    "options:",
    "  -h, --help     show this help message and exit",
    "  --all          every driver in the matrix",
    `  --driver NAME  one driver (repeatable): ${DRIVERS.map((d) => d.name).join(", ")}`,
    "  --list         the drivers and their endpoints, then stop",
    "  --no-up        the servers are already running",
    "  --down         stop and destroy the servers, then stop",
  ].join("\n");
}

async function main(flags: Set<string>, names: string[]): Promise<number> {
  if (flags.has("--down")) return bringDown();

  const selected = flags.has("--all") || names.length === 0 ? [...DRIVERS] : [];
  for (const name of names) {
    const driver = DRIVERS.find((d) => d.name === name);
    if (!driver) throw new Fail(`unknown driver ${pyRepr(name)} -- one of ${DRIVERS.map((d) => d.name).join(", ")}`);
    if (!selected.includes(driver)) selected.push(driver);
  }
  const legs = legsOf(selected);
  const list = flags.has("--list");

  // The missing crate is checked before Docker is: it is the cheaper answer and the more useful one,
  // and `--list` is the one mode that reads the compose file without it.
  if (!list && !(existsSync(CRATE) && statSync(CRATE).isDirectory())) {
    for (const leg of legs) console.log(`${labelOf(leg)}: n/a (crates/nvs-db does not exist yet)`);
    say("db-matrix: nothing ran -- the driver crate does not exist");
    return 2;
  }

  // Every selection but a lone `sqlite` needs the compose file read.
  const config = selected.some((d) => d.service) ? await composeConfig() : null;

  if (list) {
    for (const leg of legs) {
      let where: string;
      if (leg.overSocket) where = `${SOCKET_DIR}/${leg.driver.socket}`;
      else if (config === null || leg.driver.service === null) where = leg.driver.note ?? "";
      else where = describe(endpointOf(leg.driver, config));
      console.log(`${labelOf(leg)}: ${where}`);
    }
    return 0;
  }

  // Only the servers a run can reach: waiting minutes for one reported `n/a` for want of a trust anchor
  // buys nothing.
  if (!flags.has("--no-up")) await bringUp(selected.filter((d) => d.service && d.anchor).map((d) => d.service!));

  let failures = 0;
  let unrunnable = 0;
  for (const leg of legs) {
    const [verdict, detail] = await (leg.overSocket ? runSocketLeg : runDriver)(leg.driver, config);
    if (verdict === "ok") console.log(`${labelOf(leg)}: ok`);
    else if (verdict === "n/a") {
      unrunnable++;
      console.log(`${labelOf(leg)}: n/a (${detail})`);
    } else {
      failures++;
      console.log(`${labelOf(leg)}: FAILED -- ${detail}`);
    }
  }
  say(`db-matrix: ${legs.length - failures - unrunnable}/${legs.length} legs ok`);
  if (failures) return 1;
  return unrunnable ? 2 : 0;
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let names: string[];
  try {
    const parsed = parseArgs(args, {
      flags: ["--all", "--list", "--no-up", "--down"],
      valued: [],
      repeated: ["--driver"],
      order: ["--all", "--driver", "--list", "--no-up", "--down"],
    });
    flags = parsed.flags;
    names = parsed.lists.get("--driver") ?? [];
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv db-matrix: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  try {
    return await main(flags, names);
  } catch (e) {
    if (!(e instanceof Fail)) throw e;
    say(`db-matrix: ${e.message}`);
    return 2;
  }
}
