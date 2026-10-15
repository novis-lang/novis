import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { Browser, EXE } from "./chromium.ts";

// `rule:core-classes/html-later`'s polyfill in a real Chromium, launched once with native `<template
// for>` and once without. A page is streamed the way a slotted response is -- the shell, then each fill
// with the trigger, the polyfill ahead of the first -- and the same page without its scripts is the
// control: what native patching alone makes of it. `chromium.ts` says how the browser is found.

const SLOTTED = join(ROOT, "crates", "nvs-server", "src", "slotted");
const POLYFILL = readFileSync(join(SLOTTED, "polyfill.js"), "utf8");
const TRIGGER = readFileSync(join(SLOTTED, "trigger.js"), "utf8");

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

/** Loads `path` from the test server in a new tab, waits for the stream to end, and probes it. */
async function probe(browser: Browser, path: string): Promise<Probe> {
  const { targetId } = await browser.send("Target.createTarget", { url: `${server!.url}${path.slice(1)}` });
  const { sessionId } = await browser.send("Target.attachToTarget", { targetId, flatten: true });
  // A new tab starts on a complete `about:blank`, so the URL is checked with the state.
  await browser.until(`location.protocol == "http:" && document.readyState == "complete"`, sessionId);
  const out = await browser.eval(PROBE, sessionId);
  await browser.send("Target.closeTarget", { targetId });
  return out;
}

for (const native of [true, false]) {
  describe.skipIf(!EXE)(native ? "with native <template for>" : "without native <template for>", () => {
    let browser: Browser;
    beforeAll(async () => {
      browser = await Browser.launch(`html-later-polyfill-${native ? "native" : "polyfill"}`, [
        native ? "--enable-blink-features=DocumentPatching" : "--disable-blink-features=DocumentPatching,HTMLProcessingInstruction",
      ]);
    }, 60_000);
    afterAll(() => browser?.close(), 30_000);

    test("the launch is the one it says: native patching applies the bare page, or leaves it alone", async () => {
      const bare = await probe(browser, "/bare");
      if (native) expect(bare.html).toBe(FILLED);
      else expect(bare.html).toContain(`<p>loading</p>`);
    }, 30_000);

    test("the slotted page ends with each slot filled once and no template or marker left", async () => {
      const page = await probe(browser, "/slotted");
      expect(page.defined).toBe(true);
      expect(page.html).toBe(FILLED);
      expect(page.templates).toBe(0);
    }, 30_000);

    test("a second _nvs() changes nothing", async () => {
      expect((await probe(browser, "/slotted")).changed).toBe(0);
    }, 30_000);

    if (!native) {
      test("the polyfill fills markers that are processing instructions as well as comments", async () => {
        const page = await probe(browser, "/slotted-pi");
        expect(page.html).toBe(FILLED);
        expect(page.changed).toBe(0);
      }, 30_000);
    }

    if (native) {
      test("the polyfill changes nothing native patching already applied", async () => {
        const bare = await probe(browser, "/bare");
        const slotted = await probe(browser, "/slotted");
        expect(slotted.html).toBe(bare.html);
      }, 30_000);
    }
  });
}
