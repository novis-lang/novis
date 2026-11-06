// `bun nv walkthrough`: pictures of a website walkthrough at chosen scene times, so a change to one is
// judged on its frames.
//
//     bun nv walkthrough --beats PAGE              each step's number, start, end, title and caption
//     bun nv walkthrough --frames PAGE [options]   a PNG of the stage at the end of every step
//         --width N                                the page's width in CSS pixels (default 1280; below
//                                                  640 the stage uses its `tall` layout)
//         --theme dark|light|both                  the site theme (default dark)
//         --step N[,N...]                          only the end of these steps, counted from 1
//         --at T[,T...]                            these scene times, in seconds, instead of step ends
//         --stage ID                               only the stage with this element id
//     bun nv walkthrough --clean                   delete every frame written so far
//
// PAGE is the page's path on the site, such as `in-depth/walkthrough-after-response`. Git Bash rewrites
// an argument that starts with `/` into a Windows path, so the leading slash is left out, and one that
// was rewritten anyway is read back. Every stage on the page, each `.wt` element, is listed and shot
// unless `--stage` names one; the parts gallery, `in-depth/walkthrough-parts`, has one per part.
//
// The frames come from the Astro dev server. One that answers on `localhost:4321` is used, so the frames
// show the tree as it is edited; when none answers, one is started in `website/` for this run and
// stopped after it. The browser is the tests' headless Chromium (`test/chromium.ts`). A frame is taken
// by seeking the stage, `<root>.wt.seek(t)`, and waiting for two animation frames, so a frame is a
// function of scene time alone, as the engine guarantees. The files go to `.agent-tmp/walkthrough-
// frames/`, named `<stage id>-<width>-<theme>-<step or time>.png`, and each run replaces the frames it names.
// Whoever took them deletes them with `--clean` once they have been looked at.

import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Browser, EXE } from "../test/chromium.ts";
import { ROOT } from "../lib/paths.ts";
import { messageOf } from "../lib/proc.ts";

export const summary = "frames of a website walkthrough: nv walkthrough --frames PAGE [--width N] [--theme T] [--step N | --at T] [--stage ID] | --beats PAGE | --clean";

const USAGE = `usage: bun nv walkthrough --frames PAGE [--width N] [--theme dark|light|both] [--step N,... | --at T,...] [--stage ID]
       bun nv walkthrough --beats PAGE [--stage ID]
       bun nv walkthrough --clean`;

const OUT = join(ROOT, ".agent-tmp", "walkthrough-frames");
const DEV_URL = "http://localhost:4321";

interface Beat {
  title: string;
  caption: string;
  start: number;
  end: number;
  /** Where the stage's own step buttons stop at the end of this step, `stepEnd`. */
  stop: number;
  moment?: boolean;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("-h") || args.includes("--help")) {
    console.log(`${summary}\n${USAGE}`);
    return 0;
  }
  const mode = args[0];
  if (mode === "--clean" && args.length === 1) {
    rmSync(OUT, { recursive: true, force: true });
    console.log("walkthrough: frames deleted");
    return 0;
  }
  if ((mode !== "--frames" && mode !== "--beats") || !args[1] || args[1].startsWith("--")) return usage();
  const page = pagePath(args[1]);
  const opts = new Map<string, string>();
  for (let i = 2; i < args.length; i += 2) {
    const [key, value] = [args[i]!, args[i + 1]];
    if (!["--width", "--theme", "--step", "--at", "--stage"].includes(key) || value === undefined) return usage();
    opts.set(key, value);
  }
  if (mode === "--beats" && [...opts.keys()].some((k) => k !== "--stage")) return usage();
  if (opts.has("--step") && opts.has("--at")) return usage();
  const width = Number(opts.get("--width") ?? 1280);
  const theme = opts.get("--theme") ?? "dark";
  if (!Number.isInteger(width) || width < 200 || !["dark", "light", "both"].includes(theme)) return usage();
  if (!EXE) {
    console.error("walkthrough: no headless Chromium; install it with\n  PLAYWRIGHT_BROWSERS_PATH=0 node node_modules/playwright-core/cli.js install --only-shell chromium");
    return 1;
  }

  const dev = await devServer();
  if (!dev) return 1;
  const browser = await Browser.launch("nv-walkthrough", ["--hide-scrollbars"]);
  try {
    const themes = theme === "both" ? ["dark", "light"] : [theme];
    for (const th of themes) {
      const stages = await open(browser, dev.url, page, width, th, opts.get("--stage"));
      if (!stages) return 1;
      if (mode === "--beats") {
        for (const s of stages) {
          if (stages.length > 1) console.log(`${s.id}:`);
          printBeats(s.beats);
        }
        return 0;
      }
      mkdirSync(OUT, { recursive: true });
      for (const s of stages) {
        const times = pickTimes(s.beats, opts.get("--step"), opts.get("--at"));
        if (!times) {
          console.error(`walkthrough: ${s.id} has ${s.beats.length} steps; name one stage with --stage`);
          return usage();
        }
        for (const { label, t } of times) {
          const png = await s.take(t);
          const file = join(OUT, `${s.id}-${width}-${th}-${label}.png`);
          writeFileSync(file, png);
          console.log(file);
        }
      }
    }
    return 0;
  } catch (e) {
    console.error(`walkthrough: ${messageOf(e)}`);
    return 1;
  } finally {
    await browser.close();
    dev.stop();
  }
}

function usage(): number {
  console.error(USAGE);
  return 2;
}

/** The page's path with no leading or trailing slash, and with Git Bash's rewrite of `/path` undone. */
export function pagePath(arg: string): string {
  const rewritten = /^[A-Za-z]:[\\/].*?[\\/]Git[\\/](.*)$/.exec(arg);
  return (rewritten?.[1] ?? arg).replace(/\\/g, "/").replace(/^\/+|\/+$/g, "");
}

/** Which frames to take: the end of every step, the named steps, or the named times. */
export function pickTimes(beats: Beat[], steps?: string, at?: string): { label: string; t: number }[] | null {
  if (at !== undefined) {
    const ts = at.split(",").map(Number);
    if (ts.some((t) => !Number.isFinite(t) || t < 0)) return null;
    return ts.map((t) => ({ label: `t${t}`, t }));
  }
  const ns = steps === undefined ? beats.map((_, i) => i + 1) : steps.split(",").map(Number);
  if (ns.some((n) => !Number.isInteger(n) || n < 1 || n > beats.length)) return null;
  return ns.map((n) => ({ label: `step${String(n).padStart(2, "0")}`, t: beats[n - 1]!.stop }));
}

function printBeats(beats: Beat[]): void {
  for (const [i, b] of beats.entries()) {
    const mark = b.moment ? " (the moment)" : "";
    console.log(`${String(i + 1).padStart(2)}  ${b.start.toFixed(2).padStart(6)}–${b.end.toFixed(2).padEnd(6)}  ${b.title}${mark}\n    ${b.caption}`);
  }
}

interface Shot {
  id: string;
  beats: Beat[];
  take(t: number): Promise<Buffer>;
}

/**
 * Opens the page in a new tab at `width` with the site theme set, and returns each stage on it, or the
 * one with id `only`: its beats and a camera.
 */
async function open(browser: Browser, base: string, page: string, width: number, theme: string, only?: string): Promise<Shot[] | null> {
  const { sessionId } = await browser.tab();
  await browser.send("Emulation.setDeviceMetricsOverride", { width, height: 1200, deviceScaleFactor: 1, mobile: false }, sessionId);
  await browser.send("Page.enable", {}, sessionId);
  // Starlight reads its theme from storage before the first paint, so it is stored on the origin first.
  await browser.send("Page.navigate", { url: `${base}/` }, sessionId);
  await browser.until("document.readyState === 'complete'", sessionId);
  await browser.eval(`localStorage.setItem('starlight-theme', ${JSON.stringify(theme)})`, sessionId);
  await browser.send("Page.navigate", { url: `${base}/${page}/` }, sessionId);
  await browser.until("document.readyState === 'complete'", sessionId);
  if (!(await browser.eval("!!document.querySelector('.wt')", sessionId))) {
    console.error(`walkthrough: ${base}/${page}/ has no walkthrough stage`);
    return null;
  }
  await browser.until("[...document.querySelectorAll('.wt')].every((e) => e.hasAttribute('data-ready'))", sessionId);
  await browser.eval("document.fonts.ready.then(() => 1)", sessionId);
  // The engine draws every frame from scene time. The page's own CSS transitions, such as the start
  // button fading out, would be caught half way, so they are turned off in this tab. Astro's dev
  // toolbar is fixed to the viewport and would show in the frame of a stage below the fold.
  await browser.eval(
    `(() => { const s = document.createElement('style'); s.textContent = '.wt, .wt * { transition: none !important } astro-dev-toolbar { display: none !important }'; document.head.append(s); for (const e of document.querySelectorAll('.wt')) e.dataset.started = ''; return 1 })()`,
    sessionId,
  );
  const found: { id: string; beats: Beat[] }[] = await browser.eval(
    `[...document.querySelectorAll('.wt')].map((e) => ({ id: e.id, beats: e.wt.tl.beats.map(({ title, caption, start, end, moment }, i) => ({ title, caption, start, end, stop: e.wt.stepEnd(i), moment })) }))`,
    sessionId,
  );
  const stages = found.filter((s) => only === undefined || s.id === only);
  if (stages.length === 0) {
    console.error(`walkthrough: ${base}/${page}/ has no stage ${only}; it has ${found.map((s) => s.id).join(", ")}`);
    return null;
  }
  return stages.map(({ id, beats }) => ({
    id,
    beats,
    async take(t: number): Promise<Buffer> {
      const box = await browser.eval(
        `(() => {
          const root = document.getElementById(${JSON.stringify(id)})
          root.wt.seek(${t})
          const r = root.getBoundingClientRect()
          return new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() =>
            done({ x: r.x + scrollX, y: r.y + scrollY, w: r.width, h: r.height }))))
        })()`,
        sessionId,
      );
      const shot = await browser.send(
        "Page.captureScreenshot",
        { format: "png", clip: { x: box.x, y: box.y, width: box.w, height: box.h, scale: 1 }, captureBeyondViewport: true },
        sessionId,
      );
      return Buffer.from(shot.data, "base64");
    },
  }));
}

/** The dev server to read from: the one on `localhost:4321`, or one started for this run. */
async function devServer(): Promise<{ url: string; stop: () => void } | null> {
  if (await answers(DEV_URL)) return { url: DEV_URL, stop: () => {} };
  const website = join(ROOT, "website");
  if (!existsSync(join(website, "node_modules"))) {
    console.error("walkthrough: no dev server on localhost:4321, and website/node_modules is missing; run `bun install` in website/");
    return null;
  }
  console.error("walkthrough: no dev server on localhost:4321; starting one for this run");
  const proc = Bun.spawn(["bun", "run", "dev", "--", "--port", "4321"], { cwd: website, stdout: "pipe", stderr: "pipe" });
  const reader = (proc.stdout as ReadableStream<Uint8Array>).getReader();
  const dec = new TextDecoder();
  let seen = "";
  let url = "";
  while (!url) {
    const { done, value } = await reader.read();
    if (done) break;
    seen += dec.decode(value);
    url = /(http:\/\/localhost:\d+)/.exec(seen.replace(/\x1b\[[0-9;]*m/g, ""))?.[1] ?? "";
  }
  if (!url) {
    console.error(`walkthrough: the dev server did not start:\n${seen}`);
    proc.kill();
    return null;
  }
  // The rest of its output is read and thrown away, so a full pipe never stalls it.
  void (async () => {
    while (!(await reader.read()).done);
  })().catch(() => {});
  void new Response(proc.stderr as ReadableStream).text().catch(() => {});
  return { url, stop: () => proc.kill() };
}

async function answers(url: string): Promise<boolean> {
  try {
    const r = await fetch(url, { signal: AbortSignal.timeout(3000) });
    return r.status < 500;
  } catch {
    return false;
  }
}
