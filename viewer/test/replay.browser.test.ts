import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { Replayed, ReplayTarget, RequestDetail, RequestPage, RequestRow, SearchResults } from "../src/api.ts";
import { DIST } from "../build.ts";
import detailFixture from "./fixtures/request.json";
import listFixture from "./fixtures/requests.json";

// The replay dialog and the compare view in Chromium, over the committed bundle. The server below
// replaces `nvs debug`, and its replay answer names a changed copy of the request fixture.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

// JSON gives a header pair the type `string[]`, so the cast goes through `unknown`.
const FIXTURE = detailFixture as unknown as RequestDetail;
const { trace: TRACE, span: SPAN } = FIXTURE.request as RequestRow & { span: string };

/** The request the replay made: the fixture's input with one JSON value changed, a real value where the
 *  original's `authorization` was redacted, and another status. */
const OTHER: RequestDetail = structuredClone(FIXTURE);
OTHER.request = { ...OTHER.request, id: 7, trace: "1".repeat(32), span: "2".repeat(16), status: 201 };
OTHER.input!.body.text = `{"quantity":3,"item":7}`;
OTHER.input!.headers = [
  ["content-type", "application/json"],
  ["authorization", "Bearer someone-else"],
];

/** An older request to the same route as the fixture's, so the request's page can offer it. */
const PREVIOUS: RequestRow = { ...FIXTURE.request, id: 0, trace: "3".repeat(32), span: "4".repeat(16), start_us: FIXTURE.request.start_us - 1_000_000 };
const ROWS: RequestRow[] = [...(listFixture as RequestPage).requests, PREVIOUS];

/** What `GET /api/search` answers for any search: one match in the first list row and one in the fixture. */
const RESULTS: SearchResults = {
  watermark: 1,
  groups: [ROWS[0]!, FIXTURE.request].map((request, i) => ({ request, hits: [{ kind: "log", id: i + 1, at_us: request.start_us, snippet: "\u0002stock\u0003 is empty" }], more: 0 })),
  older: null,
  opens: null,
};

/** What `GET /api/replay` answers, what `POST /api/replay` answers, and every body it was sent. Each
 *  test sets the first two. */
let target: string | null = "127.0.0.1:8080";
let answer: { status: number; body: Replayed | { error: string } } = { status: 200, body: { status: 201, trace: OTHER.request.trace, span: OTHER.request.span } };
let posted: unknown[] = [];
/** Every `PUT` the tab sent, which must stay empty: a typed header value is never saved. */
let puts: string[] = [];

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/requests`: the rows older than `before`, newest first, or the one with `trace` and `span`. */
function page(url: URL): Response {
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  const before = url.searchParams.has("before") ? Number(url.searchParams.get("before")) : Infinity;
  const requests = ROWS.filter((row) => row.id < before && (trace === null || row.trace === trace) && (span === null || row.span === span));
  return json({ watermark: 1, requests, older: null } satisfies RequestPage);
}

/** `GET /api/request`: the fixture, the replay, or a list row with the fixture's records and no input. */
function one(url: URL): Response {
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  for (const detail of [FIXTURE, OTHER]) if (detail.request.trace === trace && detail.request.span === span) return json(detail);
  const row = ROWS.find((r) => r.trace === trace && r.span === span);
  if (!row) return json({ error: "no such request" }, 404);
  return json({ ...structuredClone(FIXTURE), request: row, input: null } satisfies RequestDetail);
}

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    async fetch(req) {
      const url = new URL(req.url);
      if (req.method === "PUT") puts.push(url.pathname);
      if (url.pathname === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (url.pathname === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (url.pathname === "/api/requests") return page(url);
      if (url.pathname === "/api/request") return one(url);
      if (url.pathname === "/api/search") return json(RESULTS);
      if (url.pathname === "/api/replay" && req.method === "GET") return json({ target } satisfies ReplayTarget);
      if (url.pathname === "/api/replay" && req.method === "POST") {
        posted.push(await req.json());
        return json(answer.body, answer.status);
      }
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-replay");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** Polls a condition on this side, the way `Browser.until` polls one in the tab. */
async function waitFor(ok: () => boolean) {
  while (!ok()) await Bun.sleep(25);
}

/** The keys the tests press, as DevTools describes them. */
const KEYS = {
  c: { code: "KeyC", keyCode: 67, text: "c" },
  m: { code: "KeyM", keyCode: 77, text: "m" },
  r: { code: "KeyR", keyCode: 82, text: "r" },
  "?": { code: "Slash", keyCode: 191, text: "?", modifiers: 8 },
  Escape: { code: "Escape", keyCode: 27 },
} as const;

async function press(key: keyof typeof KEYS, sessionId: string) {
  const k: { code: string; keyCode: number; text?: string; modifiers?: number } = KEYS[key];
  const common = { key, code: k.code, windowsVirtualKeyCode: k.keyCode, modifiers: k.modifiers ?? 0 };
  await browser.send("Input.dispatchKeyEvent", { ...common, type: k.text ? "keyDown" : "rawKeyDown", ...(k.text ? { text: k.text } : {}) }, sessionId);
  await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, sessionId);
}

/** A new tab at `path`, once `ready` holds in it. */
async function open(path: string, ready: string) {
  const tab = await browser.tab();
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && ${ready}`, tab.sessionId);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    press: (key: keyof typeof KEYS) => press(key, tab.sessionId),
    type: (text: string) => browser.send("Input.insertText", { text }, tab.sessionId),
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

/** The fixture's page, once its Replay button has read the target. */
const openRequest = () => open(`/requests/${TRACE}/${SPAN}`, `!!document.querySelector(".replay-action") && !document.querySelector(".replay-action").disabled`);

const DIALOG = `document.querySelector("dialog.replay")`;
const COMPARE = `/compare/${TRACE}/${SPAN}/${OTHER.request.trace}/${OTHER.request.span}`;
/** The text of the row cells of one part of the compare view, by row label. */
const PART = (id: string) =>
  `Object.fromEntries([...document.querySelectorAll("#cmp-${id} tbody tr")].map((tr) => [tr.querySelector("th").firstChild.textContent, { differs: tr.classList.contains("cmp-differs"), cells: [...tr.querySelectorAll("td")].map((td) => td.textContent) }]))`;

describe.skipIf(!EXE)("replay and compare in Chromium", () => {
  test("the dialog opens with Cancel focused and names the method, the path and the target", async () => {
    target = "127.0.0.1:8080";
    posted = [];
    const tab = await openRequest();
    await tab.press("r");
    await tab.until(`${DIALOG}?.open == true`);
    expect(await tab.eval(`document.activeElement.textContent`)).toBe("Cancel");
    expect(await tab.eval(`[${DIALOG}.getAttribute("role"), ${DIALOG}.getAttribute("aria-modal")]`)).toEqual(["dialog", "true"]);
    expect(await tab.eval(`${DIALOG}.querySelector(".replay-request").textContent`)).toBe("POST /orders?coupon=SPRING");
    expect(await tab.eval(`${DIALOG}.querySelector(".replay-target").textContent`)).toBe("127.0.0.1:8080");
    // The redacted header has an empty field, and the header that was not redacted has none.
    expect(await tab.eval(`[...${DIALOG}.querySelectorAll("input")].map((i) => [i.name, i.value, i.type])`)).toEqual([["authorization", "", "text"]]);

    // Escape cancels, nothing is sent, and the dialog opens empty and asks again.
    await tab.eval(`${DIALOG}.querySelector("input").focus()`);
    await tab.type("Bearer typed-once");
    await tab.press("Escape");
    await tab.until(`!${DIALOG}`);
    expect(await tab.eval(`location.pathname`)).toBe(`/requests/${TRACE}/${SPAN}`);
    await tab.eval(`document.querySelector(".replay-action").click()`);
    await tab.until(`${DIALOG}?.open == true`);
    expect(await tab.eval(`${DIALOG}.querySelector("input").value`)).toBe("");
    expect(await tab.eval(`document.activeElement.textContent`)).toBe("Cancel");
    expect(posted).toEqual([]);
    await tab.close();
  }, 30_000);

  test("a typed header value goes only into the replay, and the replay opens beside the original", async () => {
    target = "127.0.0.1:8080";
    answer = { status: 200, body: { status: 201, trace: OTHER.request.trace, span: OTHER.request.span } };
    posted = [];
    puts = [];
    const tab = await openRequest();
    await tab.eval(`document.querySelector(".replay-action").click()`);
    await tab.until(`${DIALOG}?.open == true`);
    await tab.eval(`${DIALOG}.querySelector("input[name=authorization]").focus()`);
    await tab.type("Bearer typed-once");
    await tab.eval(`${DIALOG}.querySelector("button[type=submit]").click()`);
    await waitFor(() => posted.length === 1);
    expect(posted[0]).toEqual({ trace: TRACE, span: SPAN, headers: { authorization: "Bearer typed-once" } });
    await tab.until(`location.pathname == "${COMPARE}" && !!document.querySelector(".cmp-count")`);
    expect(await tab.eval(`JSON.stringify({ ...localStorage }) + JSON.stringify({ ...sessionStorage })`)).not.toContain("typed-once");
    expect(await tab.eval(`document.body.innerHTML.includes("typed-once")`)).toBe(false);
    expect(puts).toEqual([]);

    // The original is on the left and the replay on the right.
    expect(await tab.eval(`document.querySelector(".cmp-left a").getAttribute("href")`)).toBe(`/requests/${TRACE}/${SPAN}`);
    expect(await tab.eval(`document.querySelector(".cmp-right a").getAttribute("href")`)).toBe(`/requests/${OTHER.request.trace}/${OTHER.request.span}`);

    // A header redacted on one side shows as redacted on both and is not a difference.
    const headers = await tab.eval(PART("request-headers"));
    expect(headers.authorization).toEqual({ differs: false, cells: ["Redacted", "Redacted"] });
    expect(headers["content-type"]).toEqual({ differs: false, cells: ["application/json", "application/json"] });
    expect(await tab.eval(`document.body.textContent.includes("someone-else")`)).toBe(false);
    // The JSON bodies are compared key by key, so the reordered keys are not a difference.
    expect(await tab.eval(PART("request-body"))).toEqual({
      item: { differs: false, cells: ["7", "7"] },
      quantity: { differs: true, cells: ["2", "3"] },
    });
    expect((await tab.eval(PART("response")))["Status"]).toEqual({ differs: true, cells: ["500", "201"] });
    // A binary body says so and is not compared as text.
    expect((await tab.eval(PART("response-body")))["Body"]).toEqual({
      differs: false,
      cells: ["Binary data, so it is not compared as text.", "Binary data, so it is not compared as text."],
    });
    // The queries are aligned by the call that sent them.
    expect((await tab.eval(PART("queries")))["src/Shop/Items.nvs:12"]).toEqual({ differs: false, cells: ["SELECT * FROM items WHERE id = ?", "SELECT * FROM items WHERE id = ?"] });
    expect((await tab.eval(PART("logs")))["1"].differs).toBe(false);
    // Quantity and status differ, and nothing else does.
    expect(await tab.eval(`document.querySelector(".cmp-count").textContent`)).toBe("2 rows differ.");
    await tab.close();
  }, 30_000);

  test("an answer without a request shows its status, and an error shows its message in the dialog", async () => {
    target = "127.0.0.1:8080";
    answer = { status: 200, body: { status: 204, trace: null, span: null } };
    const tab = await openRequest();
    await tab.eval(`document.querySelector(".replay-action").click()`);
    await tab.until(`${DIALOG}?.open == true`);
    await tab.eval(`${DIALOG}.querySelector("button[type=submit]").click()`);
    await tab.until(`!!${DIALOG}.querySelector("[role=status]")`);
    expect(await tab.eval(`${DIALOG}.querySelector("[role=status]").textContent`)).toContain("status 204");
    answer = { status: 409, body: { error: "This request's body was not recorded in full, so it cannot be sent again." } };
    await tab.eval(`${DIALOG}.querySelector("button[type=submit]").click()`);
    await tab.until(`!!${DIALOG}.querySelector("[role=alert]")`);
    expect(await tab.eval(`${DIALOG}.querySelector("[role=alert]").textContent`)).toBe(
      "This request's body was not recorded in full, so it cannot be sent again.",
    );
    expect(await tab.eval(`location.pathname`)).toBe(`/requests/${TRACE}/${SPAN}`);
    await tab.close();
  }, 30_000);

  test("a service started by hand disables Replay and says why", async () => {
    target = null;
    const tab = await open(`/requests/${TRACE}/${SPAN}`, `!!document.querySelector("#replay-why")`);
    expect(await tab.eval(`document.querySelector(".replay-action").disabled`)).toBe(true);
    expect(await tab.eval(`document.querySelector("#replay-why").textContent`)).toBe(
      "This service was started by hand, so it has no server to send a replay to.",
    );
    await tab.press("r");
    expect(await tab.eval(`!${DIALOG}`)).toBe(true);
    target = "127.0.0.1:8080";
    await tab.close();
  }, 30_000);

  test("c compares with the previous request to the same route, which says its input was not recorded", async () => {
    const tab = await open(`/requests/${TRACE}/${SPAN}`, `!!document.querySelector(".compare-previous")`);
    await tab.press("c");
    await tab.until(`location.pathname == "/compare/${PREVIOUS.trace}/${PREVIOUS.span}/${TRACE}/${SPAN}" && !!document.querySelector(".cmp-count")`);
    expect(await tab.eval(`document.querySelector(".cmp-unrecorded").textContent`)).toBe(
      "The input of the original request was not recorded. A server records it in development only.",
    );
    expect((await tab.eval(PART("request-headers"))).authorization).toEqual({ differs: false, cells: ["Redacted", "Redacted"] });
    expect((await tab.eval(PART("request-body")))["Body"].cells[0]).toBe("Not recorded. A server records this in development only.");
    await tab.close();
  }, 30_000);

  test("m marks a request in the list, and opening a second one opens the two side by side", async () => {
    const tab = await open("/", `document.querySelectorAll("[role=option]").length > 0`);
    await tab.press("?");
    await tab.until(`document.querySelector("dialog.keys").open`);
    expect(await tab.eval(`document.querySelector("dialog.keys").textContent`)).toContain("Mark the selected request");
    await tab.press("Escape");
    await tab.until(`!document.querySelector("dialog.keys").open`);

    const first = ROWS[0]!;
    await tab.press("m");
    await tab.until(`document.querySelector("#request-${first.id}").classList.contains("marked")`);
    expect(await tab.eval(`document.querySelector(".btn.mark").getAttribute("aria-pressed")`)).toBe("true");
    expect(await tab.eval(`document.querySelector(".mark-state").textContent`)).toContain(`${first.method} ${first.path}`);
    await tab.eval(`document.querySelector("#request-${FIXTURE.request.id}").click()`);
    await tab.until(`location.pathname == "/compare/${first.trace}/${first.span}/${TRACE}/${SPAN}" && !!document.querySelector(".cmp-count")`);
    expect((await tab.eval(PART("request")))["Method"]).toEqual({ differs: true, cells: [first.method, "POST"] });
    await tab.close();
  }, 30_000);

  test("a search result is marked too, and the next result's link opens the two side by side", async () => {
    const tab = await open("/search?q=stock", `document.querySelectorAll("li.group").length == 2`);
    const first = ROWS[0]!;
    await tab.eval(`document.querySelector("li.group .group-link").focus()`);
    await tab.press("m");
    await tab.until(`document.querySelector("li.group").classList.contains("marked")`);
    expect(await tab.eval(`document.querySelector("li.group .btn.mark").getAttribute("aria-pressed")`)).toBe("true");
    expect(await tab.eval(`document.querySelector(".mark-state").textContent`)).toContain(`${first.method} ${first.path}`);
    const link = `/compare/${first.trace}/${first.span}/${TRACE}/${SPAN}`;
    expect(await tab.eval(`document.querySelectorAll("li.group")[1].querySelector(".group-link").getAttribute("href")`)).toBe(link);
    await tab.eval(`document.querySelectorAll("li.group")[1].querySelector(".group-link").click()`);
    await tab.until(`location.pathname == "${link}" && !!document.querySelector(".cmp-count")`);
    expect(await tab.eval(`document.querySelector(".mark-state") === null`)).toBe(true);
    await tab.close();
  }, 30_000);
});
