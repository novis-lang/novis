// A `.shot`'s steps, taken in a real run, and the picture and `.shot.out` they record
// (`rule:testing/a-screenshot-is-taken-from-a-real-run`). `tools/nv/cmd/site.ts` § *A screenshot* is the
// home of both formats; this module reads the first, takes the run, and writes what the browser showed.
//
// A viewer shot serves a copy of an app under `apps/` in development with `[debug] store` on, so the
// server starts `nvs debug` itself, sends the requests one after another, waits until the viewer lists
// every one, and takes the steps in Chromium. A profile shot runs its program with
// `--profile=speedscope` and opens the export in the speedscope build the VS Code extension bundles.
// Each run lives in a copy under `.cache/nv-shot/`, deleted after it, so a request that writes never
// touches the app's own files.

import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Browser, EXE } from "../test/chromium.ts";
import { CACHE, ROOT } from "./paths.ts";
import { run as runProc } from "./proc.ts";
import { mask } from "./term.ts";

/** The extensions of a screenshot's steps, of its text and of its picture. */
export const SHOT = "shot";
export const SHOT_OUT = "shot.out";
export const SHOT_PNG = "png";

/** The viewer's built assets, whose hash a viewer shot's `.shot.out` carries. */
export const VIEWER_ASSETS = "viewer/dist";
/** The speedscope release the VS Code extension copies into its package. */
export const SPEEDSCOPE = "editors/vscode/node_modules/speedscope/dist/release";

/** The window every shot is taken in, in CSS pixels. */
const WIDTH = 1280;
const HEIGHT = 800;
/** How long a server, the index or one step may take before the run fails. */
const WAIT_MS = 60_000;

/** One step taken in the browser. */
export type Step = { kind: "open"; path: string } | { kind: "click"; selector: string } | { kind: "type"; selector: string; text: string } | { kind: "wait"; selector: string };

/** One request sent to the app before the viewer opens. `body` is sent as a form. */
export interface ShotRequest {
  method: string;
  path: string;
  body?: string;
}

export type Shot = { kind: "viewer"; app: string; requests: ShotRequest[]; steps: Step[] } | { kind: "profile"; program: string };

/** The steps of a `.shot`, or why the text is not one. A line is a word and its arguments; a blank line
 * and a line starting `#` are skipped. */
export function parseShot(text: string): Shot | string {
  let app = "";
  let program = "";
  const requests: ShotRequest[] = [];
  const steps: Step[] = [];
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  for (const [i, raw] of lines.entries()) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) continue;
    const at = `line ${i + 1}`;
    const [word = "", ...rest] = line.split(/\s+/);
    const arg = line.slice(word.length).trim();
    if (word === "app" && /^[a-z0-9-]+$/.test(arg)) app = arg;
    else if (word === "profile" && /^[\w./-]+\.nvs$/.test(arg) && !arg.includes("..")) program = arg;
    else if (word === "request" && /^[A-Z]+$/.test(rest[0] ?? "") && (rest[1] ?? "").startsWith("/")) {
      const body = rest.slice(2).join(" ");
      requests.push({ method: rest[0]!, path: rest[1]!, ...(body ? { body } : {}) });
    } else if (word === "open" && arg.startsWith("/")) steps.push({ kind: "open", path: arg });
    else if ((word === "click" || word === "wait") && arg) steps.push({ kind: word, selector: arg });
    else if (word === "type" && /^(\S+)\s+(.+)$/.test(arg)) {
      const [, selector, typed] = /^(\S+)\s+(.+)$/.exec(arg)!;
      steps.push({ kind: "type", selector: selector!, text: typed! });
    } else return `${at}: \`${line}\` is not a step`;
  }
  if (app && program) return "it names both an `app` and a `profile`";
  if (program) return requests.length || steps.length ? "a `profile` shot has no requests and no steps" : { kind: "profile", program };
  if (!app) return "it names no `app` and no `profile`";
  if (steps[0]?.kind !== "open") return "its first step is not `open`";
  return { kind: "viewer", app, requests, steps };
}

/** The first 16 hex digits of the SHA-256 over the files directly in `dir`, by name, or null when there
 * is no such directory. */
export function assetsHash(dir: string): string | null {
  if (!existsSync(dir)) return null;
  const hash = createHash("sha256");
  for (const name of readdirSync(dir, { withFileTypes: true }).filter((e) => e.isFile()).map((e) => e.name).sort()) {
    hash.update(`${name}\n`);
    hash.update(readFileSync(join(dir, name)));
  }
  return hash.digest("hex").slice(0, 16);
}

/** The values a page shows that differ from run to run, beyond the ones `term.ts` masks. */
const MORE_MASKS: [RegExp, string][] = [
  [/\b\d+(?:\.\d+)?\s(?:ns|µs|μs|us|ms|s|min|m|h)\b/g, "<duration>"],
  [/\b\d+\s(?:seconds?|minutes?|hours?|days?)\sago\b/g, "<ago>"],
];

/** A page's visible text as a `.shot.out` writes it: every value that differs from run to run masked,
 * and no blank line twice. */
export function shotText(text: string): string {
  let out = mask(text);
  for (const [re, to] of MORE_MASKS) out = out.replace(re, to);
  return out.replace(/\n{3,}/g, "\n\n");
}

/** A `.shot.out`: the hash of the assets drawn, a blank line, and the text. */
export const shotOut = (hash: string, text: string) => `assets ${hash}\n\n${text.trimEnd()}\n`;

/** The frames of a speedscope export and how often each was called, per timeline, in the file's order.
 * Times differ on every run, so none is written. */
export function profileText(exported: string): string {
  const file = JSON.parse(exported) as { shared: { frames: { name: string }[] }; profiles: { name: string; events: { type: string; frame: number }[] }[] };
  const blocks = file.profiles.map((p) => {
    const calls = new Map<number, number>();
    for (const e of p.events) if (e.type === "O") calls.set(e.frame, (calls.get(e.frame) ?? 0) + 1);
    return [`timeline ${p.name}`, ...[...calls].map(([f, n]) => `  ${file.shared.frames[f]?.name ?? `#${f}`}\t${n} call${n === 1 ? "" : "s"}`)].join("\n");
  });
  return blocks.join("\n\n");
}

/** Whether a browser is there to take a shot. */
export const canShoot = () => EXE !== null;

/** `body` with a fresh directory under `.cache/nv-shot/`, deleted after it. Windows keeps a file open a
 * moment after the process that had it ends, so the delete is retried. */
async function inCopy<T>(body: (dir: string) => Promise<T>): Promise<T> {
  const dir = join(CACHE, "nv-shot", crypto.randomUUID());
  mkdirSync(dir, { recursive: true });
  try {
    return await body(dir);
  } finally {
    for (let tries = 1; ; tries++) {
      try {
        rmSync(dir, { recursive: true, force: true });
        break;
      } catch (err) {
        if (tries >= 40) throw err;
        await Bun.sleep(250);
      }
    }
  }
}

/** Polls `probe` until it returns a value, or fails with `what` after `WAIT_MS`. */
async function until<T>(what: string, probe: () => Promise<T | null | undefined>): Promise<T> {
  const end = Date.now() + WAIT_MS;
  for (;;) {
    const got = await probe().catch(() => null);
    if (got !== null && got !== undefined) return got;
    if (Date.now() > end) throw new Error(`${what} within ${WAIT_MS / 1000} s`);
    await Bun.sleep(100);
  }
}

/** Opens `url` in a new tab of `browser` at the shot's size, takes `steps` there, and returns the
 * page's visible text and a PNG of it. */
async function takeSteps(browser: Browser, base: string, steps: Step[], ready: string): Promise<{ text: string; png: Uint8Array }> {
  const { sessionId } = await browser.tab();
  await browser.send("Emulation.setDeviceMetricsOverride", { width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false }, sessionId);
  const js = JSON.stringify;
  const present = (selector: string) => until(`\`${selector}\` was not on the page`, async () => ((await browser.eval(`!!document.querySelector(${js(selector)})`, sessionId)) ? true : null));
  for (const step of steps) {
    if (step.kind === "open") {
      await browser.send("Page.navigate", { url: `${base}${step.path}` }, sessionId);
      await until("the page did not load", async () => ((await browser.eval("document.readyState === 'complete'", sessionId)) ? true : null));
    } else if (step.kind === "wait") await present(step.selector);
    else if (step.kind === "click") {
      await present(step.selector);
      await browser.eval(`document.querySelector(${js(step.selector)}).click()`, sessionId);
    } else {
      await present(step.selector);
      await browser.eval(`(() => { const e = document.querySelector(${js(step.selector)}); e.focus(); e.value = ${js(step.text)}; e.dispatchEvent(new Event('input', { bubbles: true })); })()`, sessionId);
    }
  }
  await until("the page did not finish drawing", async () => ((await browser.eval(ready, sessionId)) ? true : null));
  // Fonts and two frames, so the picture is the page as drawn.
  await browser.eval("document.fonts.ready.then(() => new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok))))", sessionId);
  const text = String(await browser.eval("document.body.innerText", sessionId));
  const { data } = await browser.send("Page.captureScreenshot", { format: "png" }, sessionId);
  return { text, png: new Uint8Array(Buffer.from(data, "base64")) };
}

/** `body` with a launched Chromium, closed after it. */
async function withBrowser<T>(body: (b: Browser) => Promise<T>): Promise<T> {
  const browser = await Browser.launch("nv-shot");
  try {
    return await body(browser);
  } finally {
    await browser.close();
  }
}

/** The configuration a viewer shot adds after the app's own: development, recording, and a viewer on a
 * free port. */
const SHOT_TOML = '[mode]\ndefault = "development"\n\n[debug]\nstore = true\nlisten = "127.0.0.1:0"\n';

/** A viewer shot: the app served from a copy, its requests sent, and the steps taken in the viewer. */
async function viewerShot(shot: Extract<Shot, { kind: "viewer" }>, nvs: string, root: string): Promise<{ recording: string; png: Uint8Array }> {
  const hash = assetsHash(join(root, VIEWER_ASSETS));
  if (!hash) throw new Error(`${VIEWER_ASSETS} is not built: run \`bun run --cwd viewer build\``);
  const source = join(root, "apps", shot.app);
  if (!existsSync(join(source, "public", "index.nvs"))) throw new Error(`apps/${shot.app}/public/index.nvs does not exist`);
  return inCopy(async (dir) => {
    const app = join(dir, "app");
    cpSync(source, app, { recursive: true, filter: (p) => !/\.sqlite(-\w+)?$/.test(p) });
    writeFileSync(join(dir, "shot.toml"), SHOT_TOML);
    const configs = existsSync(join(app, "nvs.toml")) ? ["--config", "nvs.toml"] : [];
    const stop = new AbortController();
    let printed = "";
    const served = runProc([nvs, "--data", join(dir, "data"), "serve", "public/index.nvs", "--listen", "127.0.0.1:0", ...configs, "--config", join(dir, "shot.toml")], {
      cwd: app,
      signal: stop.signal,
      env: { NOVIS_NO_INIT: "1", NO_COLOR: "1" },
      hang: false,
      onText: (t) => (printed += t),
    });
    let exited = false;
    void served.then(() => (exited = true));
    try {
      const found = (re: RegExp) => async () => {
        if (exited) throw new Error(`\`nvs serve\` exited: ${(await served).stderr.trim().split("\n").slice(-5).join(" / ")}`);
        return re.exec(printed)?.[1] ?? null;
      };
      const server = await until("`nvs serve` printed no address", found(/listening on (http:\/\/\S+)/));
      const viewer = (await until("`nvs serve` started no debug viewer", found(/the debug viewer is at (http:\/\/\S+)/))).replace(/\/$/, "");
      for (const r of shot.requests) {
        const init: RequestInit = { method: r.method, redirect: "manual" };
        if (r.body !== undefined) Object.assign(init, { body: r.body, headers: { "content-type": "application/x-www-form-urlencoded" } });
        await (await fetch(`${server}${r.path}`, init)).arrayBuffer();
      }
      // The server seals its records on a deadline, and the service reads them once sealed.
      await until(`the viewer did not list all ${shot.requests.length} request(s)`, async () => {
        const page = (await (await fetch(`${viewer}/api/requests`)).json()) as { requests?: unknown[] };
        return (page.requests?.length ?? 0) >= shot.requests.length ? true : null;
      });
      const { text, png } = await withBrowser((b) => takeSteps(b, viewer, shot.steps, "!document.querySelector('[aria-busy=\"true\"]')"));
      return { recording: shotOut(hash, shotText(text.replaceAll(server, "http://127.0.0.1:<port>"))), png };
    } finally {
      stop.abort();
      await served;
    }
  });
}

/** A profile shot: the program run with `--profile=speedscope` in a copy of the fixture, and the export
 * opened in speedscope. */
async function profileShot(shot: Extract<Shot, { kind: "profile" }>, fixture: string, nvs: string, root: string): Promise<{ recording: string; png: Uint8Array }> {
  const release = join(root, SPEEDSCOPE);
  const hash = assetsHash(release);
  if (!hash) throw new Error(`${SPEEDSCOPE} is not installed: run \`npm ci\` in editors/vscode`);
  if (!existsSync(join(fixture, shot.program))) throw new Error(`${shot.program} is not in the shot's fixture directory`);
  return inCopy(async (dir) => {
    cpSync(fixture, dir, { recursive: true });
    const name = "profile.speedscope.json";
    const ran = await runProc([nvs, "run", "--profile=speedscope", "--profile-out", name, shot.program], { cwd: dir, env: { NO_COLOR: "1" }, timeoutMs: WAIT_MS });
    if (ran.code !== 0) throw new Error(`\`nvs run --profile=speedscope ${shot.program}\` exited ${ran.code}: ${ran.stderr.trim().split("\n").slice(-3).join(" / ")}`);
    const exported = readFileSync(join(dir, name), "utf8");
    const files = Bun.serve({
      port: 0,
      hostname: "127.0.0.1",
      fetch(req) {
        const path = decodeURIComponent(new URL(req.url).pathname).replace(/^\/+/, "");
        if (path === name) return new Response(exported, { headers: { "content-type": "application/json" } });
        const file = join(release, path || "index.html");
        return /^[\w.-]+$/.test(path || "index.html") && existsSync(file) ? new Response(Bun.file(file)) : new Response("not found", { status: 404 });
      },
    });
    try {
      const base = `http://127.0.0.1:${files.port}`;
      const open: Step = { kind: "open", path: `/index.html#profileURL=${encodeURIComponent(`${base}/${name}`)}` };
      // Speedscope names the tab after the file once it has drawn the profile.
      const { png } = await withBrowser((b) => takeSteps(b, base, [open, { kind: "wait", selector: "canvas" }], "document.title.includes('main.nvs')"));
      return { recording: shotOut(hash, profileText(exported)), png };
    } finally {
      files.stop(true);
    }
  });
}

/** What the `.shot` at `path` (under `root`) records now, with `nvs`. Its fixture is the directory beside
 * it with its name and no extension. */
export async function recordShot(path: string, nvs: string, root: string = ROOT): Promise<{ recording: string; png: Uint8Array } | { failure: string }> {
  const shot = parseShot(readFileSync(join(root, path), "utf8"));
  if (typeof shot === "string") return { failure: `${path}: ${shot}` };
  if (!canShoot()) return { failure: `${path}: no Chromium headless shell is installed; tools/nv/test/chromium.ts says how to install it` };
  try {
    return shot.kind === "viewer" ? await viewerShot(shot, nvs, root) : await profileShot(shot, join(root, shotFixture(path)), nvs, root);
  } catch (e) {
    return { failure: `${path}: ${(e as Error).message}` };
  }
}

/** The fixture directory of a shot: its `.shot`'s path without the extension. */
export const shotFixture = (p: string) => p.slice(0, -`.${SHOT}`.length);
/** The picture beside a shot. */
export const shotPng = (p: string) => `${shotFixture(p)}.${SHOT_PNG}`;
