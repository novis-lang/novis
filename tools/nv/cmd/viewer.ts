// `bun nv viewer`: the debug viewer's two committed files, built from `viewer/` (`viewer/build.ts`).
//
//     bun nv viewer                  build viewer/dist/viewer.js and viewer/dist/viewer.css
//     bun nv viewer --check          rebuild in memory; exit 1 when a committed file differs
//     bun nv viewer --verify         the viewer's types, the --check, then its unit tests
//     bun nv viewer --dev            serve the source on 127.0.0.1:8138, rebuilt and reloaded on each
//                                    save, with every /api/ request passed to a running `nvs debug`
//     bun nv viewer --dev --port <n> --debug <url>
//
// The two files are committed, the way `THIRD-PARTY-LICENSES.txt` is, and `nvs-cli` embeds them with
// `include_bytes!` (`crates/nvs-cli/src/viewer.rs`), so a Rust build never runs Bun. `--check` is what
// keeps the committed bytes the build of the committed source, and `nv verify`'s `viewer` step runs
// `--verify` when a file under `viewer/` changed. A build is minified with no source map, so it names
// no path of the machine that built it, and two builds of one tree give the same bytes
// (`test/viewer.test.ts`).
//
// `--dev` serves the page the way `nvs debug` does: one `#app` element, `/viewer.js` and
// `/viewer.css`. The bundle there is not minified, and a change under `viewer/src/` rebuilds it and
// reloads every open tab through a server-sent event. A path under `/api/` goes to the `nvs debug`
// named by `--debug`, the service's fixed default when it is not given, with `Origin` rewritten to the
// service's own. Any other path is the page, so a link to a request opens in the dev server too.

import { existsSync, mkdirSync, readdirSync, readFileSync, watch, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { build, DIST, VIEWER, type Bundle } from "../../../viewer/build.ts";
import { passthrough } from "../lib/proc.ts";
import { ROOT } from "../lib/paths.ts";

export const summary = "build the debug viewer into viewer/dist/, --check it is current, or --dev serve it";

/** Where `nvs debug` listens when `[debug] listen` and `--listen` are both unset. */
const DEBUG_DEFAULT = "http://127.0.0.1:8137";
/** The dev server's port: the one after the service's. */
const DEV_PORT = 8138;

const USAGE = `usage: bun nv viewer [--check | --verify | --dev [--port <n>] [--debug <url>]]`;

export async function run(args: string[]): Promise<number> {
  if (args.includes("-h") || args.includes("--help")) {
    console.log(`${summary}\n${USAGE}`);
    return 0;
  }
  const mode = args[0];
  if (mode === undefined) return write();
  if (mode === "--check" && args.length === 1) return check();
  if (mode === "--verify" && args.length === 1) return verify();
  if (mode === "--dev") {
    const port = flag(args, "--port");
    const debug = flag(args, "--debug");
    if (port === null || debug === null) return usage();
    return dev(port === undefined ? DEV_PORT : Number(port), debug ?? DEBUG_DEFAULT);
  }
  return usage();
}

function usage(): number {
  console.error(USAGE);
  return 2;
}

/** The value after `name`: `undefined` when absent, `null` when it has no value. */
function flag(args: string[], name: string): string | undefined | null {
  const i = args.indexOf(name);
  if (i < 0) return undefined;
  return args[i + 1] ?? null;
}

const shown = (p: string) => relative(ROOT, p).replace(/\\/g, "/");

async function write(): Promise<number> {
  const bundle = await build();
  mkdirSync(DIST, { recursive: true });
  for (const [name, bytes] of Object.entries(bundle)) writeFileSync(join(DIST, name), bytes);
  console.log(`nv viewer: wrote ${Object.entries(bundle).map(([n, b]) => `${shown(join(DIST, n))} (${b.length} bytes)`).join(" and ")}`);
  return 0;
}

/** The names under `dist/` whose committed bytes are not `bundle`'s. */
export function stale(bundle: Bundle, dist = DIST): string[] {
  return Object.entries(bundle)
    .filter(([name, bytes]) => {
      const path = join(dist, name);
      return !existsSync(path) || !Buffer.from(bytes).equals(readFileSync(path));
    })
    .map(([name]) => name);
}

async function check(): Promise<number> {
  const bad = stale(await build());
  if (bad.length === 0) {
    console.log("nv viewer: viewer/dist/ is the build of viewer/src/");
    return 0;
  }
  console.log(`nv viewer: ${bad.map((n) => shown(join(DIST, n))).join(" and ")} differ(s) from a rebuild -- run \`bun nv viewer\` and commit the result`);
  return 1;
}

/** The unit tests under `viewer/test/`: every `*.test.ts` that is not a browser test. */
function unitTests(): string[] {
  const dir = join(VIEWER, "test");
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => f.endsWith(".test.ts") && !f.endsWith(".browser.test.ts"))
    .map((f) => join(dir, f));
}

async function verify(): Promise<number> {
  const tsc = await passthrough(
    [process.execPath, join(ROOT, "node_modules", "typescript", "bin", "tsc"), "--noEmit", "-p", join(VIEWER, "tsconfig.json")],
    { timeoutMs: 5 * 60 * 1000 },
  );
  if (tsc !== 0) {
    console.log(`nv viewer: tsc failed with exit ${tsc}`);
    return 1;
  }
  console.log("nv viewer: the types check");
  if ((await check()) !== 0) return 1;
  const tests = unitTests();
  if (tests.length === 0) {
    console.log("nv viewer: no unit tests under viewer/test/");
    return 0;
  }
  const code = await passthrough([process.execPath, "test", ...tests], { timeoutMs: 5 * 60 * 1000 });
  if (code !== 0) {
    console.log(`nv viewer: bun test failed with exit ${code}`);
    return 1;
  }
  console.log(`nv viewer: ${tests.length} unit test file(s) pass`);
  return 0;
}

/** The page `--dev` serves, the page `nvs debug` serves with a reload hook added. */
const DEV_PAGE = `<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Novis debug (dev)</title><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script>
<script>new EventSource("/__reload").onmessage = () => location.reload();</script></body>
</html>
`;

async function dev(port: number, debug: string): Promise<number> {
  if (!Number.isInteger(port) || port <= 0 || port > 65535) return usage();
  let bundle: Promise<Bundle> | null = null;
  const current = () => (bundle ??= build(false));
  const tabs = new Set<ReadableStreamDefaultController<string>>();
  watch(join(VIEWER, "src"), { recursive: true }, () => {
    bundle = null;
    for (const tab of tabs) tab.enqueue("data: reload\n\n");
  });
  const server = Bun.serve({
    hostname: "127.0.0.1",
    port,
    idleTimeout: 0,
    async fetch(req) {
      const url = new URL(req.url);
      if (url.pathname.startsWith("/api/")) {
        const headers = new Headers(req.headers);
        headers.delete("host");
        // The service admits only its own origin, and this server is a stand-in for its page.
        if (headers.has("origin")) headers.set("origin", new URL(debug).origin);
        try {
          return await fetch(new URL(url.pathname + url.search, debug), { method: req.method, headers, body: req.body, redirect: "manual" });
        } catch (e) {
          return new Response(`nv viewer: no \`nvs debug\` answers at ${debug} (${e}) -- start it, or pass --debug <url>\n`, { status: 502 });
        }
      }
      if (url.pathname === "/__reload") {
        let mine: ReadableStreamDefaultController<string>;
        const body = new ReadableStream<string>({
          start: (c) => void tabs.add((mine = c)),
          cancel: () => void tabs.delete(mine),
        });
        return new Response(body, { headers: { "content-type": "text/event-stream", "cache-control": "no-store" } });
      }
      const files: Record<string, [keyof Bundle, string]> = {
        "/viewer.js": ["viewer.js", "text/javascript; charset=utf-8"],
        "/viewer.css": ["viewer.css", "text/css; charset=utf-8"],
      };
      const file = files[url.pathname];
      if (file) {
        try {
          return new Response((await current())[file[0]], { headers: { "content-type": file[1], "cache-control": "no-store" } });
        } catch (e) {
          bundle = null;
          return new Response(String(e), { status: 500 });
        }
      }
      return new Response(DEV_PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  console.log(`nv viewer: serving viewer/src/ at http://127.0.0.1:${server.port}/, with /api/ passed to ${debug}, until stopped`);
  return new Promise<number>(() => {});
}
