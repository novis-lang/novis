import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { Health, QuarantinedFile, Settings } from "../src/api.ts";
import { DIST } from "../build.ts";
import healthFixture from "./fixtures/health.json";
import settingsFixture from "./fixtures/settings.json";

// The settings page in a real Chromium, over the committed bundle: the theme, the saved filters, the
// lost records and the button that discards the index. The editor part has its test in
// `exceptions.browser.test.ts`, where its links are drawn. The server below stands in for
// `nvs debug`: it serves the page for every path, `/api/settings` from the settings fixture,
// `/api/health` from the health fixture, `/api/quarantine` from a list a test sets, and counts each
// `POST /api/index/discard`.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

/** The service's state. */
let settings: Settings = settingsFixture as Settings;
let health: Health = healthFixture as Health;
let files: QuarantinedFile[] = [];
let discards = 0;
/** The body of every `PUT /api/settings`, in order. */
let puts: Settings[] = [];

function reset() {
  settings = structuredClone(settingsFixture as Settings);
  health = { ...(healthFixture as Health) };
  files = [];
  discards = 0;
  puts = [];
}

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    idleTimeout: 0,
    async fetch(req) {
      const url = new URL(req.url);
      if (url.pathname === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (url.pathname === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (url.pathname === "/api/health") return json(health);
      if (url.pathname === "/api/quarantine") return json({ files });
      if (url.pathname === "/api/settings" && req.method === "GET") return json(settings);
      if (url.pathname === "/api/settings" && req.method === "PUT") {
        if (req.headers.get("content-type") !== "application/json") return json({ error: "not JSON" }, 415);
        settings = (await req.json()) as Settings;
        puts.push(settings);
        return json(settings);
      }
      if (url.pathname === "/api/index/discard" && req.method === "POST") {
        if (req.headers.get("content-type") !== "application/json") return json({ error: "not JSON" }, 415);
        discards++;
        return json({ watermark: 1 });
      }
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-settings");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

async function waitFor(ok: () => boolean) {
  while (!ok()) await Bun.sleep(25);
}

/** A new tab at `path`, once the settings page has rendered. */
async function open(path: string) {
  const tab = await browser.tab();
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && !!document.querySelector("section.settings")`, tab.sessionId);
  return {
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    enter: async () => {
      const common = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, modifiers: 0 };
      await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyDown", text: "\r" }, tab.sessionId);
      await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, tab.sessionId);
    },
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

describe.skipIf(!EXE)("the settings page in Chromium", () => {
  test("the rail's foot opens the page, and a theme radio sets the theme", async () => {
    reset();
    const tab = await open("/settings");
    expect(await tab.eval(`document.querySelector(".rail [aria-current=page]").textContent`)).toBe("Settings");
    expect(await tab.eval(`document.querySelector("input[name=theme]:checked").value`)).toBe("system");
    await tab.eval(`document.querySelector("input[name=theme][value=light]").click()`);
    await tab.until(`document.documentElement.dataset.theme == "light"`);
    expect(await tab.eval(`document.querySelector("button.theme").getAttribute("aria-label")`)).toBe("Theme: Light");
    // The top bar's button changes the radio too.
    await tab.eval(`document.querySelector("button.theme").click()`);
    await tab.until(`document.querySelector("input[name=theme]:checked").value == "system"`);
    await tab.close();
  }, 30_000);

  test("a saved filter is listed with its view, and deleting it asks first", async () => {
    reset();
    const first = settings.filters[0]!;
    const tab = await open("/settings");
    await tab.until(`document.querySelectorAll(".saved-filters li").length == ${settings.filters.length}`);
    expect(await tab.eval(`document.querySelector(".saved-filters a").getAttribute("href")`)).toBe(`/saved/${encodeURIComponent(first.name)}`);

    await tab.eval(`document.querySelector(".saved-filters button").click()`);
    await tab.until(`!!document.querySelector("dialog.ask[open]")`);
    expect(await tab.eval(`document.activeElement.textContent`)).toBe("Cancel");
    await tab.eval(`document.querySelector("dialog.ask[open] .danger").click()`);
    await waitFor(() => puts.length === 1);
    const kept = (settingsFixture as Settings).filters.filter((one) => one.name !== first.name);
    expect(puts[0]!.filters.map((one) => one.name)).toEqual(kept.map((one) => one.name));
    await tab.until(`document.querySelectorAll(".rail .saved a").length == ${puts[0]!.filters.length}`);
    await tab.close();
  }, 30_000);

  test("the lost records show their counts and the files that could not be read", async () => {
    reset();
    health = { ...health, dropped: 2, dropped_records: 12, unparseable: 3, quarantine: 1 };
    files = [{ path: "spool/9.ndjson", at_us: 1_700_000_000_000_000, retry_us: 1_700_000_060_000_000, failures: 3, reason: "access is denied" }];
    const tab = await open("/settings#lost");
    await tab.until(`document.activeElement?.id == "lost"`);
    await tab.until(`document.querySelectorAll(".lost-counts li").length == 3`);
    expect(await tab.eval(`document.querySelector(".lost-counts li").textContent`)).toContain("12 records were dropped by a program, in 2 reports.");
    expect(await tab.eval(`[...document.querySelectorAll(".lost-counts a")].map((a) => a.getAttribute("href"))`)).toEqual([
      "/stream?kind=dropped",
      "/stream?kind=unparseable",
    ]);
    await tab.until(`document.querySelectorAll(".quarantined tbody tr").length == 1`);
    expect(await tab.eval(`document.querySelector(".quarantined tbody tr").textContent`)).toContain("access is denied");
    await tab.close();
  }, 30_000);

  test("discarding the index asks first, and Enter on the focused Cancel discards nothing", async () => {
    reset();
    const tab = await open("/settings");
    await tab.eval(`document.querySelector(".discard").click()`);
    await tab.until(`!!document.querySelector("dialog.ask[open]")`);
    expect(await tab.eval(`document.activeElement.textContent`)).toBe("Cancel");
    await tab.enter();
    await tab.until(`!document.querySelector("dialog.ask[open]")`);
    expect(discards).toBe(0);

    await tab.eval(`document.querySelector(".discard").click()`);
    await tab.until(`!!document.querySelector("dialog.ask[open]")`);
    await tab.eval(`document.querySelector("dialog.ask[open] .danger").click()`);
    await waitFor(() => discards === 1);
    await tab.until(`!document.querySelector("dialog.ask[open]")`);
    await tab.close();
  }, 30_000);
});
