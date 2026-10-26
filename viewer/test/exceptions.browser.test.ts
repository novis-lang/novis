import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { ExceptionPage, ExceptionRequests, RequestDetail, RequestPage, Settings } from "../src/api.ts";
import { DIST } from "../build.ts";
import groupFixture from "./fixtures/exception.json";
import groupsFixture from "./fixtures/exceptions.json";
import detailFixture from "./fixtures/request.json";

// The exceptions view and the editor links in Chromium, over the committed bundle and the Rust fixtures.
// The server stands in for `nvs debug`, and refuses a `javascript:` template the way the service does.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

const GROUPS = groupsFixture as ExceptionPage;
const GROUP = groupFixture as ExceptionRequests;
const DETAIL = detailFixture as RequestDetail;
/** A log message that names a site in its text. It must stay text. */
const LOGGED = "stock check failed in src/Shop/Orders.nvs:42";

/** What `/api/settings` gives, and every body `PUT` sent. */
let settings: Settings = { editor: "vscode", path_mappings: [], theme: "system", filters: [] };
let puts: Settings[] = [];
const encoder = new TextEncoder();
let opened = 0;

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/exceptions`: the groups, or the one group the query names, or a `404`. */
function exceptions(url: URL): Response {
  const cls = url.searchParams.get("class");
  if (cls === null) return json(GROUPS);
  const { group } = GROUP;
  if (cls !== group.class || url.searchParams.get("file") !== group.file || url.searchParams.get("line") !== String(group.line)) {
    return json({ error: "no exception has this class and site" }, 404);
  }
  return json(GROUP);
}

/** `GET /api/requests` over the group's requests, with `trace` and `span`. */
function requests(url: URL): Response {
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  const found = GROUP.requests.filter((row) => (trace === null || row.trace === trace) && (span === null || row.span === span));
  return json({ watermark: GROUP.watermark, requests: found, older: null } satisfies RequestPage);
}

/** `GET /api/request`: the request fixture's records, given to the group's request the link names. */
function request(url: URL): Response {
  const row = GROUP.requests.find((one) => one.trace === url.searchParams.get("trace"));
  if (!row) return json({ error: "no such request" }, 404);
  const { group } = GROUP;
  return json({
    ...DETAIL,
    request: row,
    exception: { class: group.class, message: group.message ?? "", file: group.file, line: group.line },
    logs: DETAIL.logs.map((log, i) => (i === 0 ? { ...log, message: LOGGED } : log)),
  } satisfies RequestDetail);
}

/** `PUT /api/settings`: kept, unless its template's scheme runs script. */
async function put(req: Request): Promise<Response> {
  const next = (await req.json()) as Settings;
  puts.push(next);
  if (next.editor_template !== undefined && /^\s*javascript:/i.test(next.editor_template)) {
    return json({ error: "the editor template must not start with javascript:" }, 400);
  }
  settings = next;
  return json(settings);
}

/** The event stream: a comment at once, and nothing more. */
function events(): Response {
  opened++;
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(encoder.encode(": open\n\n"));
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
      if (url.pathname === "/api/exceptions") return exceptions(url);
      if (url.pathname === "/api/requests") return requests(url);
      if (url.pathname === "/api/request") return request(url);
      if (url.pathname === "/api/settings" && req.method === "GET") return json(settings);
      if (url.pathname === "/api/settings" && req.method === "PUT") return put(req);
      if (url.pathname === "/api/events") return events();
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-exceptions");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** Polls a condition on this side, the way `Browser.until` polls one in the tab. */
async function waitFor(ok: () => boolean) {
  while (!ok()) await Bun.sleep(25);
}

/** A new tab at `path`, once the frame has rendered and the view's event stream is open. */
async function open(path: string) {
  const tab = await browser.tab();
  const before = opened;
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && !!document.querySelector("#app h1")`, tab.sessionId);
  await waitFor(() => opened > before);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    press: (key: keyof typeof KEYS) => press(key, tab.sessionId),
    type: (text: string) => browser.send("Input.insertText", { text }, tab.sessionId),
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

/** The keys the tests press, as DevTools describes them. */
const KEYS = {
  j: { code: "KeyJ", keyCode: 74, text: "j" },
  k: { code: "KeyK", keyCode: 75, text: "k" },
  ArrowDown: { code: "ArrowDown", keyCode: 40 },
  Enter: { code: "Enter", keyCode: 13, text: "\r" },
  Escape: { code: "Escape", keyCode: 27 },
} as const;

async function press(key: keyof typeof KEYS, sessionId: string) {
  const k: { code: string; keyCode: number; text?: string } = KEYS[key];
  const common = { key, code: k.code, windowsVirtualKeyCode: k.keyCode, modifiers: 0 };
  await browser.send("Input.dispatchKeyEvent", { ...common, type: k.text ? "keyDown" : "rawKeyDown", ...(k.text ? { text: k.text } : {}) }, sessionId);
  await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, sessionId);
}

const OPTION_IDS = `[...document.querySelectorAll("[role=option]")].sort((a, b) => a.ariaPosInSet - b.ariaPosInSet).map((o) => o.id)`;
const SELECTED = `document.querySelector("[role=option][aria-selected=true]")?.id`;
const HREF = (id: string) => `document.querySelector("#${id} .exc-at a")?.getAttribute("href")`;
const FIRST = GROUPS.groups[0]!;

describe.skipIf(!EXE)("the exceptions view in Chromium", () => {
  test("the groups render with their count, message, routes, chart and editor link", async () => {
    settings = { editor: "vscode", path_mappings: [], theme: "system", filters: [] };
    const tab = await open("/exceptions");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${GROUPS.groups.length}`);
    expect(await tab.eval(OPTION_IDS)).toEqual(GROUPS.groups.map((group) => `exception-${group.id}`));
    const cell = (name: string) => tab.eval(`document.querySelector("#exception-${FIRST.id} .${name}").textContent`);
    expect(await cell("exc-class")).toBe(FIRST.class);
    expect(await cell("exc-count")).toBe(String(FIRST.count));
    expect(await cell("exc-message")).toBe(FIRST.message!);
    expect(await cell("exc-routes")).toBe(FIRST.routes.join(", "));
    expect(await cell("exc-at")).toBe(`${FIRST.file}:${FIRST.line}`);
    expect(await tab.eval(HREF(`exception-${FIRST.id}`))).toBe("vscode://file/src/Shop/Orders.nvs:42:1");
    // The chart has one bar per span, and a bar is drawn for each span with a throw.
    const chart = `document.querySelector("#exception-${FIRST.id} svg")`;
    expect(await tab.eval(`${chart}.getAttribute("role") + "|" + ${chart}.getAttribute("aria-label")`)).toBe("img|Throws over time");
    expect(await tab.eval(`${chart}.querySelectorAll("rect").length`)).toBe(24);
    expect(await tab.eval(`[...${chart}.querySelectorAll("rect")].filter((r) => Number(r.getAttribute("height")) > 0).length`)).toBe(
      FIRST.chart.filter((n) => n > 0).length,
    );
    await tab.close();
  }, 30_000);

  test("the keys select a group, Enter opens its requests, and Enter on a request opens its exception", async () => {
    settings = { editor: "vscode", path_mappings: [], theme: "system", filters: [] };
    const tab = await open("/exceptions");
    await tab.until(`document.querySelectorAll("[role=option]").length == ${GROUPS.groups.length}`);
    expect(await tab.eval(SELECTED)).toBe(`exception-${FIRST.id}`);
    await tab.press("j");
    await tab.until(`${SELECTED} == "exception-${GROUPS.groups[1]!.id}"`);
    await tab.press("k");
    await tab.until(`${SELECTED} == "exception-${FIRST.id}"`);
    await tab.press("Enter");
    await tab.until(`location.pathname == "/exceptions" && new URLSearchParams(location.search).get("line") == "42"`);
    await tab.until(`document.querySelectorAll("[role=option]").length == ${GROUP.requests.length}`);
    expect(await tab.eval(OPTION_IDS)).toEqual(GROUP.requests.map((row) => `request-${row.id}`));
    expect(await tab.eval(`document.querySelector(".exc-summary .title").textContent`)).toBe(GROUP.group.class);
    expect(await tab.eval(`document.querySelector(".exc-summary .exc-count").textContent`)).toBe(String(GROUP.group.count));
    expect(await tab.eval(`document.querySelector(".exc-summary .site a").getAttribute("href")`)).toBe("vscode://file/src/Shop/Orders.nvs:42:1");

    // Enter opens the selected request at its exception.
    const row = GROUP.requests[1]!;
    await tab.press("ArrowDown");
    await tab.until(`${SELECTED} == "request-${row.id}"`);
    await tab.press("Enter");
    await tab.until(`location.pathname == "/requests/${row.trace}/${row.span}" && location.hash == "#exception"`);
    await tab.until(`document.activeElement?.id == "exception"`);
    // The log line's message names a file and a line, and it is text, not a link.
    await tab.until(`!!document.querySelector(".log-message")`);
    expect(await tab.eval(`document.querySelector(".log-message").textContent`)).toBe(LOGGED);
    expect(await tab.eval(`document.querySelectorAll(".logs a").length`)).toBe(0);
    // The exception's own site is a link.
    expect(await tab.eval(`document.querySelector(".exception .exc-site a").getAttribute("href")`)).toBe("vscode://file/src/Shop/Orders.nvs:42:1");
    await tab.close();
  }, 30_000);

  test("a path mapping in the settings changes the link", async () => {
    settings = { editor: "vscode", path_mappings: [{ from: "src/", to: "/home/dev/shop/src/" }], theme: "system", filters: [] };
    const tab = await open("/exceptions");
    await tab.until(`${HREF(`exception-${FIRST.id}`)} == "vscode://file/home/dev/shop/src/Shop/Orders.nvs:42:1"`);
    await tab.close();
  }, 30_000);

  test("the editor settings save a preset, and show the service's message when it refuses a template", async () => {
    settings = { editor: "vscode", path_mappings: [], theme: "system", filters: [] };
    puts = [];
    const tab = await open("/exceptions");
    await tab.until(`!!${HREF(`exception-${FIRST.id}`)}`);
    const choose = (editor: string) =>
      tab.eval(`(() => { const s = document.querySelector(".editor-settings select"); s.value = "${editor}"; s.dispatchEvent(new Event("change")); })()`);

    // PhpStorm, with one path mapping added in the dialog.
    await tab.eval(`document.querySelector(".editor-open").click()`);
    await tab.until(`document.querySelector("dialog.editor-settings").open`);
    await choose("phpstorm");
    await tab.eval(`document.querySelector(".mapping-add").click()`);
    await tab.until(`!!document.querySelector(".mapping-from")`);
    await tab.eval(`document.querySelector(".mapping-from").focus()`);
    await tab.type("src/");
    await tab.eval(`document.querySelector(".mapping-to").focus()`);
    await tab.type("/work/");
    await tab.eval(`document.querySelector(".editor-settings [type=submit]").click()`);
    await tab.until(`!document.querySelector("dialog.editor-settings").open`);
    expect(puts.at(-1)).toEqual({ editor: "phpstorm", path_mappings: [{ from: "src/", to: "/work/" }], theme: "system", filters: [] });
    await tab.until(`${HREF(`exception-${FIRST.id}`)} == "phpstorm://open?file=/work/Shop/Orders.nvs&line=42"`);

    // A template the service refuses keeps the dialog open with the service's message.
    await tab.eval(`document.querySelector(".editor-open").click()`);
    await tab.until(`document.querySelector("dialog.editor-settings").open`);
    await choose("custom");
    await tab.until(`!!document.querySelector(".editor-template")`);
    await tab.eval(`document.querySelector(".editor-template").focus()`);
    await tab.type("javascript:alert(1)//{file}");
    await tab.eval(`document.querySelector(".editor-settings [type=submit]").click()`);
    await tab.until(`document.querySelector(".editor-settings [role=alert]")?.textContent == "the editor template must not start with javascript:"`);
    expect(await tab.eval(`document.querySelector("dialog.editor-settings").open`)).toBe(true);
    expect(await tab.eval(HREF(`exception-${FIRST.id}`))).toBe("phpstorm://open?file=/work/Shop/Orders.nvs&line=42");
    await tab.close();
  }, 30_000);
});
