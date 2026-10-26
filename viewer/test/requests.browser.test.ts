import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { RequestPage, RequestRow } from "../src/api.ts";
import { DIST } from "../build.ts";
import fixture from "./fixtures/requests.json";

// The request list in a real Chromium, over the committed bundle. The server below stands in for
// `nvs debug`: it serves the page for every path, answers `/api/requests` from an in-memory list the way
// `crates/nvs-cli/src/debug/api.rs` pages the index (newest first, `before`, `since`, `trace`, `span`,
// `limit`), and pushes a watermark on `/api/events` when a test adds requests. The rows start as the
// fixture the Rust test writes from a real index, so the view reads the shapes the service sends.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

const FIXTURE: RequestRow[] = (fixture as RequestPage).requests;
/** The request in the fixture that threw, and the two that share a trace id. */
const THREW = FIXTURE.find((row) => row.exception !== null)!;
const SHARED = "0af7651916cd43dd8448eb211c80319c";

/** The service's state: its rows, newest first, and its watermark. */
let rows: RequestRow[] = [];
let watermark = 1;
/** Every open event stream, and how many have opened in all. */
const streams = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();
let opened = 0;

/** Starts the service over `list`, newest first. */
function reset(list: RequestRow[]) {
  rows = list.map((row) => ({ ...row }));
  watermark = Math.max(1, ...rows.map((row) => row.batch));
}

/** `n` requests older than the fixture's, so the fixture's six are the newest. */
function many(n: number): RequestRow[] {
  const extra = Array.from({ length: n - FIXTURE.length }, (_, i) => clone(FIXTURE[i % FIXTURE.length]!, n - FIXTURE.length - i, 1));
  return [...FIXTURE.map((row) => ({ ...row, id: row.id + n - FIXTURE.length })), ...extra];
}

/** A copy of `row` with id `id` in batch `batch`, and its own trace id and span id. */
function clone(row: RequestRow, id: number, batch: number): RequestRow {
  return { ...row, id, batch, trace: id.toString(16).padStart(32, "f"), span: id.toString(16).padStart(16, "e") };
}

/** Commits `n` new requests as one batch and pushes its watermark. Gives the newest. */
function push(n: number): RequestRow {
  watermark++;
  const top = rows[0]?.id ?? 0;
  const added = Array.from({ length: n }, (_, i) => clone(FIXTURE[i % FIXTURE.length]!, top + n - i, watermark));
  rows = [...added, ...rows];
  for (const stream of streams) {
    try {
      stream.enqueue(encoder.encode(`data: ${watermark}\n\n`));
    } catch {
      streams.delete(stream); // A tab already closed.
    }
  }
  return added[0]!;
}

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/requests` over `rows`, the way the service answers it. */
function page(url: URL): Response {
  for (const key of url.searchParams.keys()) {
    if (!["before", "since", "trace", "span", "limit"].includes(key)) return json({ error: `unknown key ${key}` }, 400);
  }
  const num = (key: string) => (url.searchParams.has(key) ? Number(url.searchParams.get(key)) : undefined);
  const before = num("before");
  const since = num("since");
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  const limit = num("limit") ?? 100;
  const found = rows.filter(
    (row) =>
      (before === undefined || row.id < before) &&
      (since === undefined || row.batch > since) &&
      (trace === null || row.trace === trace) &&
      (span === null || row.span === span),
  );
  const requests = found.slice(0, limit);
  const older = found.length > limit ? requests.at(-1)!.id : null;
  return json({ watermark, requests, older } satisfies RequestPage);
}

/** The event stream: a comment at once, then one `data:` line per push, until the tab closes it. */
function events(req: Request): Response {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      streams.add(controller);
      opened++;
      controller.enqueue(encoder.encode(": open\n\n"));
      req.signal.addEventListener("abort", () => streams.delete(controller));
    },
    cancel() {
      for (const stream of streams) if (stream.desiredSize === null) streams.delete(stream);
    },
  });
  return new Response(body, { headers: { "content-type": "text/event-stream", "cache-control": "no-store" } });
}

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    idleTimeout: 0,
    fetch(req) {
      const url = new URL(req.url);
      if (url.pathname === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (url.pathname === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (url.pathname === "/api/requests") return page(url);
      if (url.pathname === "/api/events") return events(req);
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-requests");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** Polls a condition on this side, the way `Browser.until` polls one in the tab. */
async function waitFor(ok: () => boolean) {
  while (!ok()) await Bun.sleep(25);
}

/** A new tab at `path`, once the frame has rendered and, for the list, its event stream is open. */
async function open(path: string, list = true) {
  const tab = await browser.tab();
  const before = opened;
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && !!document.querySelector("#app h1")`, tab.sessionId);
  if (list) await waitFor(() => opened > before);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    press: (key: keyof typeof KEYS) => press(key, tab.sessionId),
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

/** The keys the tests press, as DevTools describes them. */
const KEYS = {
  j: { code: "KeyJ", keyCode: 74, text: "j" },
  k: { code: "KeyK", keyCode: 75, text: "k" },
  p: { code: "KeyP", keyCode: 80, text: "p" },
  "?": { code: "Slash", keyCode: 191, text: "?", modifiers: 8 },
  Enter: { code: "Enter", keyCode: 13, text: "\r" },
  Escape: { code: "Escape", keyCode: 27 },
  Home: { code: "Home", keyCode: 36 },
  End: { code: "End", keyCode: 35 },
} as const;

async function press(key: keyof typeof KEYS, sessionId: string) {
  const k: { code: string; keyCode: number; text?: string; modifiers?: number } = KEYS[key];
  const common = { key, code: k.code, windowsVirtualKeyCode: k.keyCode, modifiers: k.modifiers ?? 0 };
  await browser.send("Input.dispatchKeyEvent", { ...common, type: k.text ? "keyDown" : "rawKeyDown", ...(k.text ? { text: k.text } : {}) }, sessionId);
  await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, sessionId);
}

/** The ids of the rows in the DOM, in list order. */
const OPTION_IDS = `[...document.querySelectorAll("[role=option]")].sort((a, b) => a.ariaPosInSet - b.ariaPosInSet).map((o) => o.id)`;
const SELECTED = `document.querySelector("[role=option][aria-selected=true]")?.id`;
const LIST = `document.querySelector("[role=listbox]")`;

describe.skipIf(!EXE)("the request list in Chromium", () => {
  test("the rows render newest first, with their columns", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    expect(await tab.eval(OPTION_IDS)).toEqual(FIXTURE.map((row) => `request-${row.id}`));
    const cells = (id: number) =>
      tab.eval(`[...document.querySelectorAll("#request-${id} > span")].map((s) => s.textContent)`);
    // Method, route, status, duration, queries, dumps, exception.
    expect(await cells(6)).toEqual(["DELETE", "/items/{id}", "403", "610 µs", "0", "0", ""]);
    expect(await cells(THREW.id)).toEqual(["POST", "/orders", "500", "15.4 ms", "1", "2", "Shop\\OutOfStock"]);
    // A request with no route shows its path.
    expect((await cells(4))[1]).toBe("/missing");
    expect(await tab.eval(`[${LIST}.getAttribute("aria-label"), document.querySelector("#request-6").getAttribute("aria-setsize")]`)).toEqual([
      "Requests, newest first",
      "6",
    ]);
    await tab.close();
  }, 30_000);

  test("with 5000 requests, the DOM holds only the rows on screen while older pages load", async () => {
    reset(many(5000));
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length > 0`);
    let loaded = 0;
    while (loaded < 5000) {
      loaded = await tab.eval(`Number(${LIST}.dataset.rows)`);
      expect(await tab.eval(`document.querySelectorAll("[role=option]").length`)).toBeLessThan(60);
      if (loaded >= 5000) break;
      await tab.eval(`${LIST}.scrollTop = ${LIST}.scrollHeight`);
      await tab.until(`Number(${LIST}.dataset.rows) > ${loaded}`);
    }
    // At the end every request is loaded, the set size is known, and the oldest row is on screen.
    await tab.eval(`${LIST}.scrollTop = ${LIST}.scrollHeight`);
    await tab.until(`document.querySelector("#request-1")?.getAttribute("aria-setsize") == "5000"`);
    expect(await tab.eval(`document.querySelector("#request-1").getAttribute("aria-posinset")`)).toBe("5000");
    expect(await tab.eval(`document.querySelectorAll("[role=option]").length`)).toBeLessThan(60);
    await tab.close();
  }, 60_000);

  test("a pushed watermark puts the new request on top while the list follows", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    const added = push(1);
    await tab.until(`${OPTION_IDS}[0] == "request-${added.id}"`);
    expect(await tab.eval(`document.querySelectorAll("[role=option]").length`)).toBe(FIXTURE.length + 1);
    expect(await tab.eval(`document.querySelector(".follow-state").textContent`)).toBe("Following new requests.");
    await tab.close();
  }, 30_000);

  test("scrolling away pauses the list: a push moves nothing and is counted, and the top shows it", async () => {
    reset(many(300));
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length > 0`);
    await tab.eval(`${LIST}.scrollTop = 280`);
    await tab.until(`document.querySelector(".follow-state").textContent == "Paused."`);
    const shown = await tab.eval(OPTION_IDS);
    const added = push(1);
    await tab.until(`document.querySelector(".arrived")?.textContent == "1 new request"`);
    expect(await tab.eval(OPTION_IDS)).toEqual(shown);
    expect(await tab.eval(`${LIST}.scrollTop`)).toBe(280);
    await tab.eval(`${LIST}.scrollTop = 0`);
    await tab.until(`${OPTION_IDS}[0] == "request-${added.id}"`);
    expect(await tab.eval(`document.querySelector(".follow-state").textContent`)).toBe("Following new requests.");
    await tab.close();
  }, 30_000);

  test("p pauses the list, and clicking the count shows what arrived", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    await tab.press("p");
    await tab.until(`document.querySelector(".follow-state").textContent == "Paused."`);
    const added = push(3);
    await tab.until(`document.querySelector(".arrived")?.textContent == "3 new requests"`);
    expect(await tab.eval(OPTION_IDS)).toEqual(FIXTURE.map((row) => `request-${row.id}`));
    await tab.eval(`document.querySelector(".arrived").click()`);
    await tab.until(`${OPTION_IDS}[0] == "request-${added.id}"`);
    expect(await tab.eval(`document.querySelectorAll("[role=option]").length`)).toBe(FIXTURE.length + 3);
    await tab.close();
  }, 30_000);

  test("the keyboard moves the selection, Enter opens the request, and Escape goes back", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    expect(await tab.eval(SELECTED)).toBe("request-6");
    await tab.press("j");
    await tab.until(`${SELECTED} == "request-5"`);
    expect(await tab.eval(`${LIST}.getAttribute("aria-activedescendant")`)).toBe("request-5");
    expect(await tab.eval(`document.activeElement === ${LIST}`)).toBe(true);
    await tab.press("End");
    await tab.until(`${SELECTED} == "request-1"`);
    await tab.press("k");
    await tab.until(`${SELECTED} == "request-2"`);
    await tab.press("Home");
    await tab.until(`${SELECTED} == "request-6"`);
    await tab.press("j");
    await tab.until(`${SELECTED} == "request-5"`);
    await tab.press("Enter");
    const five = FIXTURE.find((row) => row.id === 5)!;
    await tab.until(`location.pathname == "/requests/${five.trace}/${five.span}"`);
    await tab.until(`document.querySelector(".request .title")?.textContent == "PUT /items/{id}"`);
    await tab.press("Escape");
    await tab.until(`location.pathname == "/" && document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    await tab.close();
  }, 30_000);

  test("? opens the list of keys, and Escape closes it", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    await tab.press("?");
    await tab.until(`document.querySelector("dialog.keys").open`);
    expect(await tab.eval(`document.querySelector("dialog.keys").textContent`)).toContain("Open the selected request");
    // While the list of keys is open, a list key does nothing.
    await tab.press("j");
    expect(await tab.eval(SELECTED)).toBe("request-6");
    await tab.press("Escape");
    await tab.until(`!document.querySelector("dialog.keys").open`);
    await tab.close();
  }, 30_000);

  test("a traceresponse value or a trace id typed into the search box opens its request", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${FIXTURE.length}`);
    const search = async (text: string) => {
      await tab.eval(`document.querySelector(".search input").focus()`);
      await browser.send("Input.insertText", { text }, tab.sessionId);
      await tab.press("Enter");
    };

    await search("not an id");
    await tab.until(`document.querySelector("#search-note").textContent == "This is not a trace id."`);
    await tab.eval(`document.querySelector(".search input").value = ""`);

    await search(`00-${THREW.trace.toUpperCase()}-${THREW.span}-01`);
    await tab.until(`location.pathname == "/requests/${THREW.trace}/${THREW.span}"`);
    await tab.until(`document.querySelector(".request .title")?.textContent == "POST /orders"`);
    expect(await tab.eval(`document.querySelector(".fields .exc").textContent`)).toBe("Shop\\OutOfStock at src/Shop/Orders.nvs:42");

    // A trace id two requests share shows both.
    await search(SHARED);
    await tab.until(`location.pathname == "/requests/${SHARED}" && document.querySelectorAll(".matches li").length == 2`);

    // A trace id one request has opens that request at its full link.
    const one = FIXTURE.find((row) => row.id === 1)!;
    await search(one.trace);
    await tab.until(`location.pathname == "/requests/${one.trace}/${one.span}"`);
    await tab.until(`document.querySelector(".request .title")?.textContent == "GET /"`);
    await tab.close();
  }, 30_000);

  test("reloading a request's link shows the same request", async () => {
    reset(FIXTURE);
    const tab = await open(`/requests/${THREW.trace}/${THREW.span}`, false);
    const title = `document.querySelector(".request .title")?.textContent == "POST /orders"`;
    await tab.until(title);
    await browser.send("Page.reload", {}, tab.sessionId);
    await tab.until(`document.readyState == "complete" && ${title}`);
    // The link with the trace id alone shows the same matches after a reload.
    await tab.eval(`history.pushState(null, "", "/requests/${SHARED}")`);
    await browser.send("Page.reload", {}, tab.sessionId);
    await tab.until(`document.readyState == "complete" && document.querySelectorAll(".matches li").length == 2`);
    await tab.close();
  }, 30_000);

  test("the theme follows the browser, and a fixed one is kept across a reload", async () => {
    reset(FIXTURE);
    const tab = await open("/");
    const bg = `getComputedStyle(document.body).backgroundColor`;
    await browser.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] }, tab.sessionId);
    await tab.until(`${bg} == "rgb(13, 17, 23)"`);
    await browser.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "light" }] }, tab.sessionId);
    await tab.until(`${bg} == "rgb(255, 255, 255)"`);
    await tab.eval(`(() => { const s = document.querySelector(".theme select"); s.value = "dark"; s.dispatchEvent(new Event("change")); })()`);
    await tab.until(`document.documentElement.dataset.theme == "dark" && ${bg} == "rgb(13, 17, 23)"`);
    await browser.send("Page.reload", {}, tab.sessionId);
    await tab.until(`document.readyState == "complete" && !!document.querySelector("#app h1") && ${bg} == "rgb(13, 17, 23)"`);
    expect(await tab.eval(`document.querySelector(".theme select").value`)).toBe("dark");
    await tab.eval(`localStorage.clear()`);
    await tab.close();
  }, 30_000);
});
