import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { ExceptionPage, Health, ReplayTarget, Routes, Settings, Statements } from "../src/api.ts";
import { DIST } from "../build.ts";
import exceptionsFixture from "./fixtures/exceptions.json";
import healthFixture from "./fixtures/health.json";
import listFixture from "./fixtures/requests.json";
import routesFixture from "./fixtures/routes.json";
import settingsFixture from "./fixtures/settings.json";
import statementsFixture from "./fixtures/statements.json";
import summaryFixture from "./fixtures/summary.json";

// The Routes view, the Queries view, the live pill and the Errors count in a real Chromium, over the
// committed bundle. The server below stands in for `nvs debug`: it serves the page for every path,
// `/api/routes`, `/api/statements`, `/api/health` and `/api/exceptions` from the fixtures the Rust
// test writes from a real index, `/api/replay` from a target a test sets, and pushes a watermark on
// `/api/events` when a test changes what the service has.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

const ROUTES = routesFixture as Routes;
const STATEMENTS = statementsFixture as Statements;
const EXCEPTIONS = exceptionsFixture as ExceptionPage;

/** The service's state. */
let health: Health = { ...(healthFixture as Health) };
let target: ReplayTarget = { target: null };
let groups = EXCEPTIONS.groups;
let watermark = 1;
const streams = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();

function reset() {
  health = { ...(healthFixture as Health) };
  target = { target: null };
  groups = EXCEPTIONS.groups;
}

/** Pushes a new watermark, as the ingester does after a batch. */
function push() {
  watermark++;
  for (const stream of streams) {
    try {
      stream.enqueue(encoder.encode(`data: ${watermark}\n\n`));
    } catch {
      streams.delete(stream);
    }
  }
}

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

function events(req: Request): Response {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      streams.add(controller);
      controller.enqueue(encoder.encode(": open\n\n"));
      req.signal.addEventListener("abort", () => streams.delete(controller));
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
      if (url.pathname === "/api/events") return events(req);
      if (url.pathname === "/api/routes") return json(ROUTES);
      if (url.pathname === "/api/statements") return json(STATEMENTS);
      if (url.pathname === "/api/health") return json(health);
      if (url.pathname === "/api/replay") return json(target);
      if (url.pathname === "/api/exceptions") return json({ ...EXCEPTIONS, watermark, groups } satisfies ExceptionPage);
      if (url.pathname === "/api/settings") return json(settingsFixture as Settings);
      if (url.pathname === "/api/requests") return json(listFixture);
      if (url.pathname === "/api/summary") return json(summaryFixture);
      if (url.pathname === "/api/quarantine") return json({ files: [] });
      if (url.pathname === "/api/search") return json({ groups: [], older: null });
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-routes");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** A new tab at `path`, once the frame has rendered. */
async function open(path: string) {
  const tab = await browser.tab();
  await browser.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false }, tab.sessionId);
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && !!document.querySelector("#app h1")`, tab.sessionId);
  return {
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

const ROWS = `[...document.querySelectorAll("[role=option]")].sort((a, b) => a.ariaPosInSet - b.ariaPosInSet)`;
const HERE = `location.pathname + location.search`;

describe.skipIf(!EXE)("the Routes and Queries views in Chromium", () => {
  test("Routes lists each method and route, a column title sorts, and a row opens the filtered list", async () => {
    reset();
    const tab = await open("/routes");
    await tab.until(`${ROWS}.length == ${ROUTES.routes.length}`);
    expect(await tab.eval(`document.querySelector(".rail [aria-current=page]").textContent`)).toBe("Routes");
    // A request no route matched says so.
    expect(await tab.eval(`${ROWS}.some((r) => r.querySelector(".routes-none"))`)).toBe(true);
    // The error share of the one route that failed is drawn as an error.
    const orders = `${ROWS}.find((r) => r.textContent.includes("/orders"))`;
    expect(await tab.eval(`${orders}.querySelector(".bad").textContent`)).toBe("100%");

    // p50 sorts the slowest first, and a second click the fastest first. The sort is in the link.
    const sortBy = (name: string) => `[...document.querySelectorAll("button.routes-sort")].find((b) => b.textContent.trim() == "${name}").click()`;
    await tab.eval(sortBy("p50"));
    await tab.until(`${HERE} == "/routes?sort=p50&dir=desc" && ${ROWS}[0]?.textContent.includes("/orders")`);
    await tab.eval(sortBy("p50"));
    await tab.until(`${HERE} == "/routes?sort=p50&dir=asc" && !!${ROWS}[0]?.querySelector(".routes-none")`);

    await tab.eval(`${orders}.click()`);
    await tab.until(`${HERE} == "/?method=POST&route=%2Forders"`);
    await tab.close();
  }, 30_000);

  test("Queries lists each call site, marks a repeated one, and a row opens the search for it", async () => {
    reset();
    const tab = await open("/queries");
    await tab.until(`${ROWS}.length == ${STATEMENTS.statements.length}`);
    const repeated = STATEMENTS.statements.findIndex((one) => one.repeated > 0);
    expect(await tab.eval(`${ROWS}.map((r) => r.classList.contains("repeated"))`)).toEqual(STATEMENTS.statements.map((one) => one.repeated > 0));
    expect(await tab.eval(`${ROWS}[${repeated}].querySelector(".repeat").textContent`)).toBe(`${STATEMENTS.statements[repeated]!.repeated}×`);

    const first = STATEMENTS.statements[0]!;
    await tab.eval(`${ROWS}[0].click()`);
    await tab.until(`${HERE} == "/search?${new URLSearchParams({ q: first.sql })}"`);
    await tab.close();
  }, 30_000);

  test("the live pill names the server's port, says when recording stopped, and links to lost records", async () => {
    reset();
    target = { target: "127.0.0.1:8080" };
    let tab = await open("/routes");
    await tab.until(`document.querySelector(".live")?.textContent == "Live:8080"`);
    await tab.close();

    target = { target: null };
    tab = await open("/routes");
    await tab.until(`document.querySelector(".live.stopped")?.textContent == "Recording stopped"`);

    // Records lost after the tab opened turn the pill to a warning with their count.
    health = { ...health, dropped: 1, dropped_records: 12 };
    push();
    await tab.until(`document.querySelector("a.live.warn")?.textContent == "12 lost"`);
    await tab.eval(`document.querySelector("a.live.warn").click()`);
    await tab.until(`location.pathname == "/settings" && !!document.querySelector("#lost .lost-counts")`);
    await tab.close();
  }, 30_000);

  test("the rail counts the error groups thrown since the tab opened", async () => {
    reset();
    const tab = await open("/routes");
    await tab.until(`${ROWS}.length > 0`);
    // The fixture's groups were thrown long ago.
    expect(await tab.eval(`document.querySelector(".rail-count")`)).toBeNull();

    groups = [{ ...EXCEPTIONS.groups[0]!, last_us: Date.now() * 1000 + 1_000_000 }, ...EXCEPTIONS.groups.slice(1)];
    push();
    await tab.until(`document.querySelector('.rail a[href="/exceptions"] .rail-count')?.textContent == "1"`);
    await tab.close();
  }, 30_000);
});
