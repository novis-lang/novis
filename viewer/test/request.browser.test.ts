import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { CoverageMap, RequestDetail, RequestPage, SourceFile } from "../src/api.ts";
import { DIST } from "../build.ts";
import coverageFixture from "./fixtures/coverage.json";
import detailFixture from "./fixtures/request.json";
import listFixture from "./fixtures/requests.json";
import sourceFixture from "./fixtures/source.json";

// One request's page in a real Chromium, over the committed bundle. The server below stands in for
// `nvs debug`: it serves the page for every path, `/api/requests` from the list fixture filtered by
// `trace` and `span`, `/api/request` from the request fixture, and `/api/coverage` and `/api/source`
// from theirs, all of which the Rust test writes from a real index. The view therefore reads the
// shapes the service sends.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

const LIST = listFixture as RequestPage;
// JSON gives a header pair the type `string[]`, so the cast goes through `unknown`.
const FIXTURE = detailFixture as unknown as RequestDetail;
const { trace: TRACE, span: SPAN } = FIXTURE.request;

/** What `/api/request` answers for the fixture's request. A test may swap it for a changed copy. */
let detail: RequestDetail = FIXTURE;
/** What `/api/source` answers. A test may swap it for a changed copy. */
let text: SourceFile = sourceFixture;

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/requests` with `trace` and `span`, the filters this view sends. */
function page(url: URL): Response {
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  const requests = LIST.requests.filter((row) => (trace === null || row.trace === trace) && (span === null || row.span === span));
  return json({ watermark: LIST.watermark, requests, older: null } satisfies RequestPage);
}

/** `GET /api/request`: the one request the fixture has, or a `404`. */
function one(url: URL): Response {
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  if (trace !== detail.request.trace || (span !== null && span !== detail.request.span)) return json({ error: "no such request" }, 404);
  return json(detail);
}

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    fetch(req) {
      const url = new URL(req.url);
      if (url.pathname === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (url.pathname === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (url.pathname === "/api/requests") return page(url);
      if (url.pathname === "/api/request") return one(url);
      if (url.pathname === "/api/coverage") return json(coverageFixture satisfies CoverageMap);
      if (url.pathname === "/api/source") {
        const known = url.searchParams.get("coverage") === String(coverageFixture.id) && url.searchParams.get("file") === text.file;
        return known ? json(text) : json({ error: "This request's coverage does not name this file." }, 404);
      }
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-request");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** The keys the tests press, as DevTools describes them. */
const KEYS = {
  ArrowDown: { code: "ArrowDown", keyCode: 40 },
  ArrowUp: { code: "ArrowUp", keyCode: 38 },
  ArrowLeft: { code: "ArrowLeft", keyCode: 37 },
  ArrowRight: { code: "ArrowRight", keyCode: 39 },
  Home: { code: "Home", keyCode: 36 },
  End: { code: "End", keyCode: 35 },
  Enter: { code: "Enter", keyCode: 13, text: "\r" },
} as const;

async function press(key: keyof typeof KEYS, sessionId: string) {
  const k: { code: string; keyCode: number; text?: string } = KEYS[key];
  const common = { key, code: k.code, windowsVirtualKeyCode: k.keyCode, modifiers: 0 };
  await browser.send("Input.dispatchKeyEvent", { ...common, type: k.text ? "keyDown" : "rawKeyDown", ...(k.text ? { text: k.text } : {}) }, sessionId);
  await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, sessionId);
}

/** A new tab on the fixture's request, once its timeline has rendered. */
async function open() {
  const tab = await browser.tab();
  await browser.send("Page.navigate", { url: `${server!.url}requests/${TRACE}/${SPAN}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && document.querySelectorAll(".tl-list [role=option]").length > 0`, tab.sessionId);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    press: (key: keyof typeof KEYS) => press(key, tab.sessionId),
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

/** The text of one tree node's own row, without its children. */
const ROW = (id: string) => `document.querySelector("#${id} > .tv-row").textContent`;
const FOCUSED = `document.activeElement?.id`;
const ACTIVE = `document.querySelector(".tl-list").getAttribute("aria-activedescendant")`;

describe.skipIf(!EXE)("one request's page in Chromium", () => {
  test("the timeline shows every event kind, the dumps and the log line in time order", async () => {
    detail = FIXTURE;
    const tab = await open();
    expect(await tab.eval(`[...document.querySelectorAll(".tl-list [role=option]")].map((o) => o.id)`)).toEqual([
      "event-1",
      "event-2",
      "point-dump-1",
      "point-log-1",
      "event-3",
      "event-4",
      "point-dump-2",
    ]);
    expect(await tab.eval(`[...document.querySelectorAll(".tl-list [role=option] .tl-kind")].map((s) => s.textContent)`)).toEqual([
      "Call",
      "Query",
      "Dump",
      "Log line",
      "Garbage collection",
      "Spawn",
      "Dump",
    ]);
    // A query shows its SQL, its driver and rows, when it started and how long it took.
    const cells = await tab.eval(`[...document.querySelectorAll("#event-2 > span")].map((s) => s.textContent)`);
    expect(cells).toEqual(["Query", "100 µs", "SELECT * FROM items WHERE id = ?", "sqlite, 1 row", "800 µs"]);
    expect(await tab.eval(`document.querySelector("#point-log-1 .tl-label").textContent`)).toBe("stock is empty");
    expect(await tab.eval(`document.querySelector("#point-dump-2 .tl-label").textContent`)).toBe("src/Shop/Orders.nvs:41");

    // The call holds the query and the other events, so they take a second lane, and no two bars in
    // one lane overlap.
    const lanes: { left: number; right: number }[][] = await tab.eval(
      `[...document.querySelectorAll(".tl-lane[data-lane]")].map((lane) => [...lane.querySelectorAll(".tl-bar")].map((b) => { const r = b.getBoundingClientRect(); return { left: r.left, right: r.right }; }))`,
    );
    expect(lanes.map((lane) => lane.length)).toEqual([1, 3]);
    for (const lane of lanes) {
      const bars = [...lane].sort((a, b) => a.left - b.left);
      for (let i = 1; i < bars.length; i++) expect(bars[i]!.left).toBeGreaterThanOrEqual(bars[i - 1]!.right - 0.5);
    }
    expect(await tab.eval(`document.querySelectorAll(".tl-points .tl-point").length`)).toBe(3);
    expect(await tab.eval(`document.querySelectorAll(".cut-note").length`)).toBe(0);

    // The exception and the log line are on the page as text.
    expect(await tab.eval(`document.querySelector(".exception .exc-head").textContent`)).toBe("Shop\\OutOfStock: item 7 is out of stock");
    expect(await tab.eval(`document.querySelector(".exception .exc-site").textContent`)).toBe("Thrown at src/Shop/Orders.nvs:42");
    expect(await tab.eval(`document.querySelector("#log-1 .log-message").textContent`)).toBe("stock is empty");
    await tab.close();
  }, 30_000);

  test("the keys move through the timeline, and Enter on a dump moves to its tree", async () => {
    detail = FIXTURE;
    const tab = await open();
    await tab.eval(`document.querySelector(".tl-list").focus()`);
    expect(await tab.eval(ACTIVE)).toBe("event-1");
    await tab.press("ArrowDown");
    await tab.until(`${ACTIVE} == "event-2"`);
    expect(await tab.eval(`document.querySelector("#event-2").getAttribute("aria-selected")`)).toBe("true");
    expect(await tab.eval(`document.querySelector(".tl-bar.tl-on")?.textContent`)).toBe("SELECT * FROM items WHERE id = ?");
    await tab.press("End");
    await tab.until(`${ACTIVE} == "point-dump-2"`);
    await tab.press("ArrowUp");
    await tab.until(`${ACTIVE} == "event-4"`);
    await tab.press("ArrowDown");
    await tab.until(`${ACTIVE} == "point-dump-2"`);
    await tab.press("Enter");
    await tab.until(`${FOCUSED} == "dump-2-n0"`);
    await tab.press("Home");
    await tab.until(`${FOCUSED} == "dump-2-n0"`);
    await tab.close();
  }, 30_000);

  test("a value tree shows its repeat as a link, its secret and what was cut", async () => {
    detail = FIXTURE;
    const tab = await open();
    expect(await tab.eval(`document.querySelectorAll("[role=tree]").length`)).toBe(3);
    expect(await tab.eval(`document.querySelector("#dump-2 [role=tree]").getAttribute("aria-label")`)).toBe("Value dumped at src/Shop/Orders.nvs:41");
    expect(await tab.eval(ROW("dump-2-n0"))).toBe("▾Shop\\Order #2");
    expect(await tab.eval(ROW("dump-2-n1"))).toBe("card: Secret. The value is hidden.");
    expect(await tab.eval(ROW("dump-2-n2"))).toBe("deep: Not recorded. This value is nested too deeply.");
    expect(await tab.eval(ROW("dump-2-n4"))).toBe("▾lines: list, 40 items");
    expect(await tab.eval(ROW("dump-2-n8"))).toBe("1: 39 of 40 entries are not recorded.");
    expect(await tab.eval(ROW("dump-2-n9"))).toBe(`note: "Leave at the" 120 more bytes are not recorded.`);
    expect(await tab.eval(ROW("dump-1-n2"))).toBe(`name: "Lamp"`);

    // The repeat is a link to the instance with that number, and following it focuses and
    // highlights that node.
    expect(await tab.eval(ROW("dump-2-n6"))).toBe("order: Same object as Shop\\Order #2");
    expect(await tab.eval(`document.querySelector("#dump-2-n6 a").getAttribute("href")`)).toBe("#dump-2-n0");
    expect(await tab.eval(`document.querySelector("#dump-2-n0").dataset.instance`)).toBe("2");
    await tab.eval(`document.querySelector("#dump-2-n6 a").click()`);
    await tab.until(`${FOCUSED} == "dump-2-n0"`);
    expect(await tab.eval(`document.querySelector("#dump-2-n0").classList.contains("tv-jumped")`)).toBe(true);
    expect(await tab.eval(`location.pathname`)).toBe(`/requests/${TRACE}/${SPAN}`);

    // Enter on the repeat does the same from the keyboard.
    await tab.eval(`document.querySelector("#dump-2-n7 > .tv-row").click()`);
    await tab.until(`${FOCUSED} == "dump-2-n7"`);
    await tab.press("ArrowUp");
    await tab.until(`${FOCUSED} == "dump-2-n6"`);
    await tab.press("Enter");
    await tab.until(`${FOCUSED} == "dump-2-n0"`);
    await tab.close();
  }, 30_000);

  test("the arrow keys move through a tree and open and close its nodes", async () => {
    detail = FIXTURE;
    const tab = await open();
    const items = `document.querySelectorAll("#dump-2 [role=treeitem]").length`;
    expect(await tab.eval(items)).toBe(10);
    expect(await tab.eval(`[...document.querySelectorAll("#dump-2 [role=treeitem]")].filter((t) => t.tabIndex == 0).map((t) => t.id)`)).toEqual([
      "dump-2-n0",
    ]);
    await tab.eval(`document.querySelector("#dump-2-n0").focus()`);
    await tab.press("ArrowDown");
    await tab.until(`${FOCUSED} == "dump-2-n1"`);
    expect(await tab.eval(`document.querySelector("#dump-2-n1").tabIndex`)).toBe(0);
    expect(await tab.eval(`document.querySelector("#dump-2-n0").tabIndex`)).toBe(-1);
    await tab.press("End");
    await tab.until(`${FOCUSED} == "dump-2-n9"`);
    await tab.press("ArrowLeft");
    await tab.until(`${FOCUSED} == "dump-2-n0"`);

    // Left closes an open node and Right opens it again.
    await tab.press("ArrowLeft");
    await tab.until(`document.querySelector("#dump-2-n0").getAttribute("aria-expanded") == "false"`);
    expect(await tab.eval(items)).toBe(1);
    await tab.press("ArrowRight");
    await tab.until(`document.querySelector("#dump-2-n0").getAttribute("aria-expanded") == "true"`);
    expect(await tab.eval(items)).toBe(10);
    await tab.press("ArrowRight");
    await tab.until(`${FOCUSED} == "dump-2-n1"`);

    // A closed list hides its items, so Down goes past them.
    for (const id of ["dump-2-n2", "dump-2-n3", "dump-2-n4"]) {
      await tab.press("ArrowDown");
      await tab.until(`${FOCUSED} == "${id}"`);
    }
    await tab.press("Enter");
    await tab.until(`document.querySelector("#dump-2-n4").getAttribute("aria-expanded") == "false"`);
    await tab.press("ArrowDown");
    await tab.until(`${FOCUSED} == "dump-2-n9"`);
    await tab.press("Home");
    await tab.until(`${FOCUSED} == "dump-2-n0"`);
    await tab.close();
  }, 30_000);

  test("a value with markup in it shows as text, and a cut list and a long dump say so", async () => {
    const changed: RequestDetail = structuredClone(FIXTURE);
    changed.dumps[0]!.tree = { $class: "Shop\\Item", $id: 1, $properties: { name: `<img src=x onerror="window.ran=1"><b>Lamp</b>` } };
    changed.dumps[1]!.elided = true;
    changed.dumps[1]!.tree = null;
    changed.cut = { events: true, dumps: false, logs: true, profiles: false };
    detail = changed;
    const tab = await open();
    expect(await tab.eval(ROW("dump-1-n1"))).toBe(`name: "<img src=x onerror="window.ran=1"><b>Lamp</b>"`);
    expect(await tab.eval(`document.querySelectorAll(".request img, .request b").length`)).toBe(0);
    expect(await tab.eval(`window.ran === undefined`)).toBe(true);
    expect(await tab.eval(`document.querySelector("#dump-2 .cut-note").textContent`)).toBe("This value was too long to record.");
    expect(await tab.eval(`document.querySelectorAll("#dump-2 [role=tree]").length`)).toBe(0);
    expect(await tab.eval(`[...document.querySelectorAll(".timeline .cut-note")].map((p) => p.textContent)`)).toEqual([
      "The list of events stopped at its limit, so some events are not shown.",
      "The list of log lines stopped at its limit, so some log lines are not shown.",
    ]);
    detail = FIXTURE;
    await tab.close();
  }, 30_000);

  test("coverage draws each line that ran with its count, and a file's markup shows as text", async () => {
    const covered: RequestDetail = structuredClone(FIXTURE);
    covered.coverage = { id: coverageFixture.id, files: 1, elided: false };
    detail = covered;
    text = { ...sourceFixture, text: sourceFixture.text.replace("return 3;", `return 3; // <img src=x onerror="window.ran=1">`) };
    const tab = await open();
    await tab.until(`document.querySelectorAll(".source-line").length > 0`);
    const lines = sourceFixture.text.replace(/\n$/, "").split("\n");
    expect(await tab.eval(`document.querySelectorAll(".source-line").length`)).toBe(lines.length);
    const ran = Object.keys(coverageFixture.files[sourceFixture.file as keyof typeof coverageFixture.files]).map(Number);
    const heated = await tab.eval(`[...document.querySelectorAll(".source-line")].filter((l) => /heat-/.test(l.className)).map((l) => Number(l.querySelector(".number").textContent))`);
    expect(heated).toEqual(ran);
    expect(await tab.eval(`document.querySelectorAll(".source-line")[${ran[0]! - 1}].querySelector(".count").textContent`)).toBe("1×");
    expect(await tab.eval(`document.querySelector(".coverage-file").value`)).toBe(sourceFixture.file);
    expect(await tab.eval(`document.querySelector(".coverage-changed") !== null`)).toBe(sourceFixture.changed);
    expect(await tab.eval(`document.querySelectorAll(".source img").length`)).toBe(0);
    expect(await tab.eval(`window.ran === undefined`)).toBe(true);
    detail = FIXTURE;
    text = sourceFixture;
    await tab.close();
  }, 30_000);
});
