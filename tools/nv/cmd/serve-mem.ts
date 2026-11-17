// `bun nv serve-mem`: what `nvs serve` keeps in memory per recompile, and how it answers while its
// source is edited.
//
//     bun nv serve-mem [--app html|registry]      the app to serve, `html` by default
//                      [--fails <n>] [--oks <n>]  write n broken edits, restore, then n good edits, restore
//                      [--wait <s>]             the pause after each edit before a sample, 6 s by default
//                      [--load <conns>]           keep-alive load from <conns> connections while it edits
//                      [--edits <n>]              how many edits the load phase writes, 20 by default
//                      [--every <ms>]             the pause between two edits of the load phase, 1500 ms
//                      [--memory <size>]          `[limits] memory` for the copy, such as `1G`; `html` only
//
// It copies the app under `.agent-tmp/serve-mem/<app>/` (`html` is
// `benches/compare/cases/html-page`, `registry` is `apps/registry` without its database and its
// ignored folders), starts `target/release/nvs serve` on the copy, and edits one source file the entry
// uses. The copy is deleted when the command ends, and the tree is never edited.
//
// The memory phase runs with `--fails` or `--oks`. After each step it prints the server's
// private bytes (`PrivateMemorySize64` on Windows, `RssAnon` on Linux), the change since the step
// before, its thread count, and `MAPPED_JIT_UNITS` when the server's log prints a `jit_units=<n>` line.
// A broken edit adds a method that does not compile, and a good edit adds a method that returns a
// number. No source text is known to make the compiler panic, so there is no panic edit;
// `crates/nvs-fuzz-checks` searches for one.
//
// The load phase runs with `--load`. One raw keep-alive connection sends a request without a pause and
// reads the version each answer prints, and <conns> more connections send requests beside it. Each
// round is one edit, and the command prints the `200`s, the `500`s, the other answers and the errors of
// that round, and what the raw connection saw last: the version of a `200`, or the status of any other
// answer. It exits 1 when any answer was not a `200`.
//
// The binary is the release build, so run `bun nv relink --build` first when the tree changed. The
// server's own output is in `.agent-tmp/serve-mem/<app>/server.log` while it runs, and is kept as
// `.agent-tmp/serve-mem/<app>.log` once it ends; whoever reads it deletes it.

import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync, appendFileSync } from "node:fs";
import { connect, createServer } from "node:net";
import { join } from "node:path";
import { ROOT, rel } from "../lib/paths.ts";
import { releaseCli } from "../lib/relink.ts";

export const summary =
  "measure nvs serve's memory per recompile and its answers under edits: nv serve-mem [--app html|registry] [--fails n] [--oks n] [--load conns] [--edits n] [--memory size]";

const WORK = join(ROOT, ".agent-tmp", "serve-mem");

interface App {
  /** The folder copied. */
  src: string;
  /** Entries of `src` left out of the copy: the database and the folders git ignores. */
  skip: Set<string>;
  entry: string;
  /** The file every edit rewrites, relative to the app. */
  edited: string;
  /** A path that answers `200` once the server is up. */
  ready: string;
  config: string | null;
  /** The class an edit adds a method to. */
  cls: RegExp;
  /** Writes `n` into the page the load phase reads, so an answer shows which edit it ran. */
  version: ((src: string, n: number) => string) | null;
}

const APPS: Record<string, App> = {
  html: {
    src: join(ROOT, "benches", "compare", "cases", "html-page"),
    skip: new Set(["index.php", "main.go", "server.ts"]),
    entry: "main.nvs",
    edited: "main.nvs",
    ready: "/products?q=a",
    config: null,
    cls: /(class Product\s*\{)/,
    version: (src, n) => src.replace("<h1>Results for", `<h1>Results v${n} for`),
  },
  registry: {
    src: join(ROOT, "apps", "registry"),
    skip: new Set(["registry.sqlite", "registry.sqlite-wal", "registry.sqlite-shm", "builds", "work", "files", "backup"]),
    entry: "public/index.nvs",
    edited: "src/App.nvs",
    ready: "/policy",
    config: "nvs.toml",
    cls: /(final class App\s*\{)/,
    version: null,
  },
};

/** Ten ways to break the file: type errors, unknown names and syntax errors. */
const BROKEN = [
  (n: number) => `return 'text${n}';`,
  (n: number) => `return $missing${n};`,
  (n: number) => `return 1 +;`,
  (n: number) => `return Nope${n}::x();`,
  (n: number) => `int $a = ${n} return $a;`,
  (n: number) => `return Core\\Str::length(${n});`,
  (n: number) => `return $this->x${n};`,
  (n: number) => `string $s = ${n}; return $s;`,
  (n: number) => `return Core\\Arr::nope${n}([]);`,
  (n: number) => `return [${n}];`,
];

interface Args {
  app: string;
  fails: number;
  oks: number;
  wait: number;
  load: number;
  edits: number;
  every: number;
  memory: string;
}

function parse(args: string[]): Args | string {
  const out: Args = { app: "html", fails: 0, oks: 0, wait: 6, load: 0, edits: 20, every: 1500, memory: "" };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    const key = arg.replace(/^--/, "");
    if (!arg.startsWith("--") || !(key in out)) return `unknown argument ${arg}`;
    const value = args[++i];
    if (value === undefined) return `${arg} needs a value`;
    if (key === "app") {
      if (!(value in APPS)) return `--app is one of ${Object.keys(APPS).join(", ")}`;
      out.app = value;
    } else if (key === "memory") {
      if (!/^\d+[KMG]?$/.test(value)) return "--memory needs a size such as 1G";
      out.memory = value;
    } else {
      const n = Number(value);
      if (!Number.isFinite(n) || n < 0) return `${arg} needs a number`;
      (out as unknown as Record<string, number>)[key] = n;
    }
  }
  if (out.fails === 0 && out.oks === 0 && out.load === 0) {
    return "nothing to do: give --fails, --oks or --load";
  }
  if (out.memory && APPS[out.app]!.config) return "--memory needs an app with no configuration file of its own";
  return out;
}

/** A port nothing listens on now. */
async function freePort(): Promise<number> {
  return await new Promise((done, fail) => {
    const s = createServer();
    s.once("error", fail);
    s.listen(0, "127.0.0.1", () => {
      const addr = s.address();
      const port = typeof addr === "object" && addr ? addr.port : 0;
      s.close(() => done(port));
    });
  });
}

/** The server's private bytes: the memory it allocated, with no file mapping counted. */
function privateBytes(pid: number): number {
  if (process.platform === "win32") {
    const r = spawnSync("powershell.exe", ["-NoProfile", "-Command", `(Get-Process -Id ${pid}).PrivateMemorySize64`], { encoding: "utf8" });
    return Number(r.stdout.trim());
  }
  if (process.platform === "linux") {
    const status = readFileSync(`/proc/${pid}/status`, "utf8");
    return Number(status.match(/RssAnon:\s+(\d+)/)?.[1] ?? NaN) * 1024;
  }
  return NaN;
}

/** The server's thread count, which falls when a thread ends. */
function threads(pid: number): number {
  if (process.platform === "win32") {
    const r = spawnSync("powershell.exe", ["-NoProfile", "-Command", `(Get-Process -Id ${pid}).Threads.Count`], { encoding: "utf8" });
    return Number(r.stdout.trim());
  }
  if (process.platform === "linux") {
    const status = readFileSync(`/proc/${pid}/status`, "utf8");
    return Number(status.match(/Threads:\s+(\d+)/)?.[1] ?? NaN);
  }
  return NaN;
}

const mb = (b: number) => (Number.isNaN(b) ? "?" : (b / 1048576).toFixed(1));

class Server {
  readonly child: ChildProcess;
  readonly base: string;
  /** The last `jit_units=<n>` the server printed, if it prints one. */
  jitUnits: string = "-";
  constructor(exe: string, dir: string, app: App, port: number, config: string | null) {
    const log = join(dir, "server.log");
    const data = join(dir, ".data");
    mkdirSync(data, { recursive: true });
    const argv = ["serve", app.entry, "--listen", `127.0.0.1:${port}`, "--data", data, "--no-init"];
    if (config) argv.push("--config", config);
    this.child = spawn(exe, argv, { cwd: dir, stdio: ["ignore", "pipe", "pipe"] });
    const read = (d: Buffer) => {
      appendFileSync(log, d);
      const m = [...d.toString().matchAll(/jit_units=(\d+)/g)].pop();
      if (m) this.jitUnits = m[1]!;
    };
    this.child.stdout!.on("data", read);
    this.child.stderr!.on("data", read);
    this.base = `http://127.0.0.1:${port}`;
  }
  /** Waits until the server answers `path`, for at most two minutes. */
  async up(path: string): Promise<boolean> {
    const end = performance.now() + 120_000;
    while (performance.now() < end) {
      if (this.child.exitCode !== null) return false;
      try {
        await (await fetch(this.base + path, { signal: AbortSignal.timeout(30_000) })).arrayBuffer();
        return true;
      } catch {
        await Bun.sleep(200);
      }
    }
    return false;
  }
  async probe(path: string): Promise<string> {
    try {
      const res = await fetch(this.base + path, { headers: { connection: "close" }, keepalive: false, signal: AbortSignal.timeout(30_000) });
      return `${res.status} ${(await res.text()).length}B`;
    } catch (e) {
      return `error ${String(e).slice(0, 40)}`;
    }
  }
}

/** Adds a static method named after `n` to the app's class. */
function withMethod(app: App, src: string, n: number, body: string): string {
  return src.replace(app.cls, `$1\n    public static function edit${n}(): int\n    {\n        ${body}\n    }\n`);
}

async function memoryPhase(server: Server, app: App, file: string, original: string, a: Args): Promise<void> {
  let first = NaN;
  let prev = NaN;
  let n = 0;
  const pid = server.child.pid!;
  console.log(`step        answer        private MB   change   since start  threads  jit units`);
  const step = async (label: string) => {
    await Bun.sleep(a.wait * 1000);
    const priv = privateBytes(pid);
    if (Number.isNaN(first)) first = priv;
    const served = await server.probe(app.ready);
    const change = Number.isNaN(prev) ? 0 : priv - prev;
    console.log(
      `${label.padEnd(12)}${served.padEnd(14)}${mb(priv).padStart(10)}${(change >= 0 ? "+" : "") + mb(change)}`.padEnd(45) +
        `${mb(priv - first).padStart(10)}  ${String(threads(pid)).padStart(7)}  ${server.jitUnits}`,
    );
    prev = priv;
  };
  await step("baseline");
  for (let i = 0; i < a.fails; i++) {
    n++;
    writeFileSync(file, withMethod(app, original, n, BROKEN[i % BROKEN.length]!(n)));
    await step(`fail ${i + 1}`);
  }
  if (a.fails > 0) {
    writeFileSync(file, original);
    await step("fixed");
  }
  for (let i = 0; i < a.oks; i++) {
    n++;
    writeFileSync(file, withMethod(app, original, n, `return ${n};`));
    await step(`ok ${i + 1}`);
  }
  if (a.oks > 0) {
    writeFileSync(file, original);
    await step("restored");
  }
}

/** One connection that is certainly kept alive: a raw socket, one request at a time. */
function rawClient(port: number) {
  const sock = connect(port, "127.0.0.1");
  let buf = "";
  let waiting: ((s: string) => void) | null = null;
  sock.on("data", (d) => {
    buf += d.toString("latin1");
    const head = buf.indexOf("\r\n\r\n");
    if (head < 0) return;
    const len = Number(buf.slice(0, head).match(/content-length: (\d+)/i)?.[1] ?? 0);
    if (buf.length >= head + 4 + len) {
      const resp = buf.slice(0, head + 4 + len);
      buf = buf.slice(head + 4 + len);
      waiting?.(resp);
    }
  });
  sock.on("error", () => waiting?.("ERROR"));
  sock.on("close", () => waiting?.("CLOSED"));
  return {
    ready: new Promise((r) => sock.on("connect", r)),
    get: (path: string) =>
      new Promise<string>((r) => {
        waiting = r;
        sock.write(`GET ${path} HTTP/1.1\r\nhost: x\r\n\r\n`);
      }),
    close: () => sock.destroy(),
  };
}

interface Round {
  ok: number;
  e500: number;
  other: number;
  errors: number;
  version: string;
  firstBad: string;
}

async function loadPhase(server: Server, app: App, file: string, original: string, port: number, a: Args): Promise<boolean> {
  const rounds: Round[] = [];
  const fresh = (): Round => ({ ok: 0, e500: 0, other: 0, errors: 0, version: "-", firstBad: "" });
  let round = fresh();
  let running = true;
  const count = (status: number, body: string) => {
    if (status === 200) round.ok++;
    else {
      if (status === 500) round.e500++;
      else round.other++;
      if (!round.firstBad) round.firstBad = `${status} ${body.slice(0, 300).replace(/\s+/g, " ")}`;
    }
  };
  const worker = async () => {
    while (running) {
      try {
        const res = await fetch(server.base + app.ready, { signal: AbortSignal.timeout(10_000) });
        count(res.status, await res.text());
      } catch {
        round.errors++;
        await Bun.sleep(5);
      }
    }
  };
  const raw = rawClient(port);
  await raw.ready;
  const kept = async () => {
    while (running) {
      const resp = await Promise.race([raw.get(app.ready), Bun.sleep(10_000).then(() => "TIMEOUT")]);
      const status = Number(resp.slice(9, 12));
      if (!Number.isFinite(status) || status === 0) {
        round.errors++;
        if (!round.firstBad) round.firstBad = `keep-alive: ${resp.slice(0, 60)}`;
        return;
      }
      const head = resp.indexOf("\r\n\r\n");
      count(status, resp.slice(head + 4));
      round.version = status === 200 ? `v${resp.match(/Results v(\d+) for/)?.[1] ?? "0"}` : String(status);
    }
  };
  const tasks = [kept(), ...Array.from({ length: a.load }, worker)];
  console.log(`round  200s    500s  other  errors  keep-alive saw`);
  for (let e = 1; e <= a.edits; e++) {
    let src = withMethod(app, original, e, `return ${e};`);
    if (app.version) src = app.version(src, e);
    writeFileSync(file, src);
    await Bun.sleep(a.every);
    rounds.push(round);
    const r = round;
    round = fresh();
    console.log(`${String(e).padStart(5)}${String(r.ok).padStart(6)}${String(r.e500).padStart(8)}${String(r.other).padStart(7)}${String(r.errors).padStart(8)}  ${app.version ? r.version : "-"}`);
    if (r.firstBad) console.log(`       first: ${r.firstBad}`);
  }
  running = false;
  raw.close();
  await Promise.all(tasks);
  const bad = rounds.reduce((s, r) => s + r.e500 + r.other + r.errors, 0);
  const ok = rounds.reduce((s, r) => s + r.ok, 0);
  console.log(`total: ${ok} answered 200, ${bad} did not`);
  return bad === 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((x) => x === "-h" || x === "--help")) {
    console.log(summary);
    return 0;
  }
  const a = parse(args);
  if (typeof a === "string") {
    console.error(`nv serve-mem: ${a}`);
    return 2;
  }
  const exe = releaseCli();
  if (!existsSync(exe)) {
    console.error(`nv serve-mem: ${rel(exe)} does not exist; run \`bun nv relink --build\` first`);
    return 1;
  }
  const app = APPS[a.app]!;
  const dir = join(WORK, a.app);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  for (const name of readdirSync(app.src)) {
    if (!app.skip.has(name)) cpSync(join(app.src, name), join(dir, name), { recursive: true });
  }
  const file = join(dir, app.edited);
  const original = readFileSync(file, "utf8");
  const port = await freePort();
  let config = app.config;
  if (a.memory) {
    config = "nvs.toml";
    writeFileSync(join(dir, config), `[limits]\nmemory = "${a.memory}"\n`);
  }
  const server = new Server(exe, dir, app, port, config);
  let code = 0;
  try {
    if (!(await server.up(app.ready))) {
      console.error(`nv serve-mem: the server did not answer ${app.ready}; its output is in ${rel(join(dir, "server.log"))}`);
      return 1;
    }
    for (let i = 0; i < 20; i++) await server.probe(app.ready);
    if (a.fails > 0 || a.oks > 0) await memoryPhase(server, app, file, original, a);
    if (a.load > 0) {
      writeFileSync(file, original);
      if (!(await loadPhase(server, app, file, original, port, a))) code = 1;
    }
  } finally {
    server.child.kill();
    await new Promise((r) => (server.child.exitCode !== null ? r(null) : server.child.once("exit", r)));
    const log = join(dir, "server.log");
    if (existsSync(log)) cpSync(log, `${dir}.log`);
    rmSync(dir, { recursive: true, force: true });
  }
  return code;
}
