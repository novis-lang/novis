import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

// `rule:core-classes/html-later`'s polyfill in a real Chromium, launched once with native `<template
// for>` and once without. A page is streamed the way a slotted response is -- the shell, then each fill
// with the trigger, the polyfill ahead of the first -- and the same page without its scripts is the
// control: what native patching alone makes of it.
//
// The browser is Playwright's pinned Chromium headless shell, downloaded into `node_modules`:
//
//     PLAYWRIGHT_BROWSERS_PATH=0 node node_modules/playwright-core/cli.js install --only-shell chromium
//
// Playwright's own driver neither launches nor connects under Bun on Windows, so this speaks the
// DevTools protocol over Bun's WebSocket. With no download the suite is skipped, unless `NV_BROWSER`
// is set, which is how CI's `browser` job makes a missing browser a failure.

const SLOTTED = join(ROOT, "crates", "nvs-server", "src", "slotted");
const POLYFILL = readFileSync(join(SLOTTED, "polyfill.js"), "utf8");
const TRIGGER = readFileSync(join(SLOTTED, "trigger.js"), "utf8");

/** The headless shell's executable under Playwright's in-tree browser directory, or `null`. */
function headlessShell(): string | null {
  const base = join(ROOT, "node_modules", "playwright-core", ".local-browsers");
  if (!existsSync(base)) return null;
  for (const d of readdirSync(base).filter((d) => d.startsWith("chromium_headless_shell-")).sort().reverse()) {
    for (const p of readdirSync(join(base, d)).filter((p) => p.startsWith("chrome-headless-shell-"))) {
      const exe = join(base, d, p, process.platform === "win32" ? "chrome-headless-shell.exe" : "chrome-headless-shell");
      if (existsSync(exe)) return exe;
    }
  }
  return null;
}

const EXE = headlessShell();
if (!EXE && process.env.NV_BROWSER) throw new Error("NV_BROWSER is set and no Chromium headless shell is installed");

const NAME = (n: number) => `nvs-AAAAAAAAAAAAAAAA-${n}`;
const fill = (n: number, content: string, polyfill: boolean, scripts: boolean) =>
  (scripts && polyfill ? `<script>${POLYFILL}</script>` : "") +
  `<template for="${NAME(n)}">${content}</template>` +
  (scripts ? `<script>${TRIGGER}</script>` : "");

// Turns every marker the parser made a comment into the processing instruction a browser that parses
// `<?start>` makes, so one launch tests both kinds of marker.
const AS_PI = `<script>const cs = [], w = document.createTreeWalker(document.body, 128);
while (w.nextNode()) cs.push(w.currentNode);
for (const c of cs) { const m = /^\\?(\\S+) ?(.*)$/.exec(c.data); if (m) c.replaceWith(document.createProcessingInstruction(m[1], m[2])); }</script>`;

/** The page's chunks: two slots, the second finishing first, with or without the polyfill and trigger. */
function chunks(scripts: boolean, pi: boolean): string[] {
  return [
    `<!doctype html><html><head><title>Shop</title></head><body><h1>Shop</h1>` +
      `<?start name="${NAME(0)}"><p>loading</p><?end><div id="keep">static</div>` +
      `<?start name="${NAME(1)}"><p>loading</p><?end>` + (pi ? AS_PI : ""),
    fill(1, `<p class="fill">second</p>`, true, scripts),
    fill(0, `<p class="fill">first</p>`, false, scripts),
    `</body></html>`,
  ];
}

const FILLED = `<h1>Shop</h1><p class="fill">first</p><div id="keep">static</div><p class="fill">second</p>`;

// Runs in the page: the body without its scripts, then whether one more `_nvs()` changes anything.
const PROBE = `(() => {
  const body = document.body.cloneNode(true);
  for (const s of body.querySelectorAll("script")) s.remove();
  const mo = new MutationObserver(() => {});
  mo.observe(document, { subtree: true, childList: true, attributes: true, characterData: true });
  const defined = typeof _nvs == "function";
  if (defined) _nvs();
  const changed = mo.takeRecords().length;
  mo.disconnect();
  return { html: body.innerHTML, defined, changed, templates: document.querySelectorAll("template").length };
})()`;

type Probe = { html: string; defined: boolean; changed: number; templates: number };

let server: ReturnType<typeof Bun.serve> | undefined;

beforeAll(() => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    fetch(req) {
      const path = new URL(req.url).pathname;
      const parts = chunks(path !== "/bare", path === "/slotted-pi");
      const enc = new TextEncoder();
      // A pause between chunks, so the parser meets each fill after the previous one has run.
      const body = new ReadableStream({
        async pull(c) {
          const next = parts.shift();
          if (next === undefined) return c.close();
          c.enqueue(enc.encode(next));
          await Bun.sleep(40);
        },
      });
      return new Response(body, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
});

afterAll(() => server?.stop(true));

/** One Chromium process and a DevTools connection to it. */
class Browser {
  private next = 0;
  private waiting = new Map<number, (m: { result?: unknown; error?: { message: string } }) => void>();

  private constructor(
    private proc: ReturnType<typeof Bun.spawn>,
    private ws: WebSocket,
    private profile: string,
  ) {
    ws.onmessage = (e) => {
      const m = JSON.parse(String(e.data));
      if (m.id !== undefined) this.waiting.get(m.id)?.(m);
    };
  }

  static async launch(native: boolean): Promise<Browser> {
    const profile = join(ROOT, ".agent-tmp", `html-later-polyfill-${process.pid}-${native ? "native" : "polyfill"}`);
    const args = [
      EXE!, "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run",
      native ? "--enable-blink-features=DocumentPatching" : "--disable-blink-features=DocumentPatching,HTMLProcessingInstruction", "about:blank",
    ];
    if (process.platform === "linux") args.splice(1, 0, "--no-sandbox");
    const proc = Bun.spawn(args, { stdout: "ignore", stderr: "pipe" });
    const dec = new TextDecoder();
    const reader = (proc.stderr as ReadableStream<Uint8Array>).getReader();
    let seen = "";
    let url = "";
    while (!url) {
      const { done, value } = await reader.read();
      if (done) break;
      seen += dec.decode(value);
      url = /DevTools listening on (ws:\/\/\S+)/.exec(seen)?.[1] ?? "";
    }
    if (!url) throw new Error(`Chromium did not start:\n${seen}`);
    // The rest of stderr is read and thrown away, so a full pipe never stalls Chromium's logging.
    void (async () => {
      while (!(await reader.read()).done);
    })().catch(() => {});
    const ws = new WebSocket(url);
    await new Promise((ok, fail) => {
      ws.onopen = ok;
      ws.onerror = fail;
    });
    return new Browser(proc, ws, profile);
  }

  send(method: string, params: object = {}, sessionId?: string): Promise<any> {
    const id = ++this.next;
    return new Promise((ok, fail) => {
      this.waiting.set(id, (m) => {
        this.waiting.delete(id);
        if (m.error) fail(new Error(`${method}: ${m.error.message}`));
        else ok(m.result);
      });
      this.ws.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }

  async eval(expression: string, sessionId: string): Promise<any> {
    const r = await this.send("Runtime.evaluate", { expression, returnByValue: true }, sessionId);
    if (r.exceptionDetails) throw new Error(`${expression}: ${JSON.stringify(r.exceptionDetails)}`);
    return r.result.value;
  }

  /** Loads `path` from the test server in a new tab, waits for the stream to end, and probes it. */
  async probe(path: string): Promise<Probe> {
    const { targetId } = await this.send("Target.createTarget", { url: `${server!.url}${path.slice(1)}` });
    const { sessionId } = await this.send("Target.attachToTarget", { targetId, flatten: true });
    // A new tab starts on a complete `about:blank`, so the URL is checked with the state. The wait is
    // bounded by the test's own timeout, so a loaded machine is slow here rather than probing a half page.
    while (!(await this.eval(`location.protocol == "http:" && document.readyState == "complete"`, sessionId))) {
      await Bun.sleep(25);
    }
    const out = await this.eval(PROBE, sessionId);
    await this.send("Target.closeTarget", { targetId });
    return out;
  }

  async close() {
    this.ws.close();
    this.proc.kill();
    await this.proc.exited;
    rmSync(this.profile, { recursive: true, force: true });
  }
}

for (const native of [true, false]) {
  describe.skipIf(!EXE)(native ? "with native <template for>" : "without native <template for>", () => {
    let browser: Browser;
    beforeAll(async () => {
      browser = await Browser.launch(native);
    }, 60_000);
    afterAll(() => browser?.close());

    test("the launch is the one it says: native patching applies the bare page, or leaves it alone", async () => {
      const bare = await browser.probe("/bare");
      if (native) expect(bare.html).toBe(FILLED);
      else expect(bare.html).toContain(`<p>loading</p>`);
    }, 30_000);

    test("the slotted page ends with each slot filled once and no template or marker left", async () => {
      const page = await browser.probe("/slotted");
      expect(page.defined).toBe(true);
      expect(page.html).toBe(FILLED);
      expect(page.templates).toBe(0);
    }, 30_000);

    test("a second _nvs() changes nothing", async () => {
      expect((await browser.probe("/slotted")).changed).toBe(0);
    }, 30_000);

    if (!native) {
      test("the polyfill fills markers that are processing instructions as well as comments", async () => {
        const page = await browser.probe("/slotted-pi");
        expect(page.html).toBe(FILLED);
        expect(page.changed).toBe(0);
      }, 30_000);
    }

    if (native) {
      test("the polyfill changes nothing native patching already applied", async () => {
        const bare = await browser.probe("/bare");
        const slotted = await browser.probe("/slotted");
        expect(slotted.html).toBe(bare.html);
      }, 30_000);
    }
  });
}
