import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { QuarantinedFile, RequestPage, SearchResults, Settings, StreamPage, StreamRecord } from "../src/api.ts";
import { DIST } from "../build.ts";
import listFixture from "./fixtures/requests.json";
import searchFixture from "./fixtures/search.json";
import settingsFixture from "./fixtures/settings.json";
import streamFixture from "./fixtures/stream.json";

// The stream, the log, the search and the saved filters in a real Chromium, over the committed bundle.
// The server below stands in for `nvs debug`: it serves the page for every path, `/api/stream` from an
// in-memory list that starts as the fixture (filtered by `kind`, `level`, `trace`, `since` and
// `before` as `crates/nvs-cli/src/debug/stream.rs` does), `/api/search` from the search fixture,
// `/api/settings` from the settings fixture, `/api/quarantine` and `/api/index/discard`, and pushes a
// watermark on `/api/events` when a test adds records. Every fixture is written by the Rust test from a
// real index, so the views read the shapes the service sends.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

const FIXTURE: StreamRecord[] = (streamFixture as StreamPage).records;
const RESULTS = searchFixture as SearchResults;
const LIST = listFixture as RequestPage;
/** The request the search fixture's one group belongs to. */
const FOUND = RESULTS.groups[0]!.request!;

/** The service's state. */
let records: StreamRecord[] = [];
let watermark = 1;
let files: QuarantinedFile[] = [];
let settings: Settings = settingsFixture as Settings;
let discards = 0;
/** The query of every `/api/stream` request, in order. */
let asked: string[] = [];
/** The body of every `PUT /api/settings`, in order. */
let puts: Settings[] = [];
const streams = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();
let opened = 0;

function reset(list: StreamRecord[] = FIXTURE) {
  records = list.map((record) => ({ ...record }));
  watermark = Math.max(1, ...records.map((record) => record.batch));
  files = [];
  settings = structuredClone(settingsFixture as Settings);
  discards = 0;
  asked = [];
  puts = [];
}

/** A record of `kind` at `at_us` with key `key`, copied from the fixture's first record of that kind. */
function record(kind: StreamRecord["kind"], key: number, at_us: number, batch: number, more: Partial<StreamRecord> = {}): StreamRecord {
  const base = FIXTURE.find((one) => one.kind === kind) ?? FIXTURE[0]!;
  return { ...base, kind, key, at_us, batch, ...more };
}

/** Commits `added`, newest first, as one batch and pushes its watermark. */
function push(...added: StreamRecord[]) {
  watermark++;
  records = [...added.map((one) => ({ ...one, batch: watermark })), ...records];
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

/** `GET /api/stream` over `records`, the way the service answers it. */
function streamPage(url: URL): Response {
  asked.push(url.search.slice(1));
  const params = url.searchParams;
  const kinds = params.get("kind")?.split(",") ?? [];
  const level = params.get("level");
  const trace = params.get("trace");
  const since = params.has("since") ? Number(params.get("since")) : undefined;
  const before = params.get("before")?.split(".").map(Number);
  const limit = params.has("limit") ? Number(params.get("limit")) : 100;
  const found = records
    .filter(
      (one) =>
        (kinds.length === 0 || kinds.includes(one.kind)) &&
        (level === null || one.level === level) &&
        (trace === null || one.trace === trace) &&
        (since === undefined || one.batch > since) &&
        (before === undefined || one.at_us < before[0]! || (one.at_us === before[0]! && one.key < before[1]!)),
    )
    .sort((a, b) => b.at_us - a.at_us || b.key - a.key);
  const page = found.slice(0, limit);
  const last = page.at(-1);
  const older = found.length > limit && last ? `${last.at_us}.${last.key}` : null;
  return json({ watermark, records: page, older } satisfies StreamPage);
}

/** `GET /api/search`: the fixture for `stock`, a request for its trace id, a `400` for a bad field. */
function searchPage(url: URL): Response {
  const q = url.searchParams.get("q") ?? "";
  if (q === "stock") return json(RESULTS);
  if (q === FOUND.trace) return json({ ...RESULTS, groups: [], opens: { trace: FOUND.trace, span: FOUND.span } } satisfies SearchResults);
  if (q === "status:abc") return json({ error: "`status:` must be a status such as `500` or `5xx`." }, 400);
  if (q === "markup") {
    const hit = { kind: "log" as const, id: 9, at_us: FOUND.start_us, snippet: '<img src=x onerror="document.title=1"> \u0002markup\u0003' };
    return json({ ...RESULTS, groups: [{ request: FOUND, hits: [hit], more: 2 }] } satisfies SearchResults);
  }
  return json({ ...RESULTS, groups: [] } satisfies SearchResults);
}

/** The event stream: a comment at once, then one `data:` line per push. */
function events(req: Request): Response {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      streams.add(controller);
      opened++;
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
    async fetch(req) {
      const url = new URL(req.url);
      if (url.pathname === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (url.pathname === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (url.pathname === "/api/stream") return streamPage(url);
      if (url.pathname === "/api/events") return events(req);
      if (url.pathname === "/api/search") return searchPage(url);
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
        return json({ watermark });
      }
      if (url.pathname === "/api/requests") {
        const trace = url.searchParams.get("trace");
        return json({ ...LIST, requests: LIST.requests.filter((row) => trace === null || row.trace === trace) });
      }
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-stream");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

async function waitFor(ok: () => boolean) {
  while (!ok()) await Bun.sleep(25);
}

/** A new tab at `path`, once the frame has rendered and, for a feed, its event stream is open. */
async function open(path: string, feed = true) {
  const tab = await browser.tab();
  const before = opened;
  // A fixed screen size, so the rows on screen do not depend on the machine.
  await browser.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 1200, deviceScaleFactor: 1, mobile: false }, tab.sessionId);
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && !!document.querySelector("#app h1")`, tab.sessionId);
  if (feed) await waitFor(() => opened > before);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    press: (key: keyof typeof KEYS) => press(key, tab.sessionId),
    type: (text: string) => browser.send("Input.insertText", { text }, tab.sessionId),
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

const KEYS = {
  j: { code: "KeyJ", keyCode: 74, text: "j" },
  p: { code: "KeyP", keyCode: 80, text: "p" },
  s: { code: "KeyS", keyCode: 83, text: "s" },
  "/": { code: "Slash", keyCode: 191, text: "/" },
  Enter: { code: "Enter", keyCode: 13, text: "\r" },
} as const;

async function press(key: keyof typeof KEYS, sessionId: string) {
  const k: { code: string; keyCode: number; text?: string } = KEYS[key];
  const common = { key, code: k.code, windowsVirtualKeyCode: k.keyCode, modifiers: 0 };
  await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyDown", ...(k.text ? { text: k.text } : {}) }, sessionId);
  await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, sessionId);
}

const ROWS = `[...document.querySelectorAll("[role=option]")].sort((a, b) => a.ariaPosInSet - b.ariaPosInSet)`;
const KINDS = `${ROWS}.map((o) => o.dataset.kind)`;
const LIST_EL = `document.querySelector("[role=listbox]")`;
/** The rows the feed has loaded, on screen or not. */
const LOADED = `Number(${LIST_EL}?.dataset.rows)`;
const HERE = `location.pathname + location.search`;

describe.skipIf(!EXE)("the stream, the log, the search and saved filters in Chromium", () => {
  test("the stream shows every record newest first, and its filter is in the link", async () => {
    reset();
    const tab = await open("/stream");
    await tab.until(`${LOADED} == ${FIXTURE.length}`);
    // Only the rows on screen are in the DOM, so they are the first rows of the page.
    const shown: string[] = await tab.eval(KINDS);
    expect(shown.length).toBeGreaterThan(2);
    expect(shown).toEqual(FIXTURE.map((one) => one.kind).slice(0, shown.length));
    // The dump's row: its kind, its text and its request, linked.
    const dump = FIXTURE.find((one) => one.kind === "dump")!;
    const cells = await tab.eval(`[...document.querySelectorAll("#record-${dump.key} > span")].map((s) => s.textContent)`);
    expect(cells.slice(1, 3)).toEqual(["dump", dump.text]);
    expect(await tab.eval(`document.querySelector("#record-${dump.key} .owner a").getAttribute("href")`)).toBe(
      `/requests/${dump.request!.trace}/${dump.request!.span}`,
    );

    // Ticking two kinds puts them in the link, and the service is asked with the commas as they are.
    await tab.eval(`document.querySelector("input[name=kind][value=dump]").click()`);
    await tab.until(`${HERE} == "/stream?kind=dump"`);
    await tab.eval(`document.querySelector("input[name=kind][value=log]").click()`);
    await tab.until(`${HERE} == "/stream?kind=dump,log"`);
    await tab.until(`${KINDS}.length > 0 && ${KINDS}.every((k) => k == "dump" || k == "log")`);
    expect(asked).toContain("kind=dump,log");

    // A reload of the link shows the same filter.
    await browser.send("Page.reload", {}, tab.sessionId);
    await tab.until(`document.readyState == "complete" && ${LOADED} == ${FIXTURE.filter((one) => one.kind === "dump" || one.kind === "log").length}`);
    expect(await tab.eval(`[...document.querySelectorAll("input[name=kind]:checked")].map((i) => i.value)`)).toEqual(["dump", "log"]);
    await tab.close();
  }, 30_000);

  test("an unreadable line and dropped records are shown, marked, with their fields", async () => {
    const top = FIXTURE[0]!.at_us;
    reset([
      record("unparseable", 1006, top + 2, 1, { text: "{\"kind\":", file: "spool/7.ndjson", trace: null, request: null, fields: { reason: "not JSON", offset: 4096, length: 9, lossy: false } }),
      record("dropped", 1007, top + 1, 1, { text: "records dropped", trace: null, request: null, fields: { records: 12, files: 1 } }),
      ...FIXTURE,
    ]);
    const tab = await open("/stream");
    await tab.until(`document.querySelectorAll(".row.flawed").length == 2`);
    expect(await tab.eval(`document.querySelector("#record-1006 .badge").textContent`)).toBe("Unreadable line");
    expect(await tab.eval(`document.querySelector("#record-1007 .badge").textContent`)).toBe("Records dropped");
    expect(await tab.eval(`document.querySelector("#record-1007 .text").textContent`)).toContain('{"records":12,"files":1}');
    expect(await tab.eval(`document.querySelector("#record-1006 .owner").textContent`)).toBe("No request");
    // The first row is selected, and its fields show below the list.
    await tab.until(`!!document.querySelector(".record-detail [role=tree]")`);
    expect(await tab.eval(`document.querySelector(".record-detail").textContent`)).toContain("not JSON");
    await tab.close();
  }, 30_000);

  test("a pushed record goes on top while the stream follows, and p pauses and counts", async () => {
    reset();
    const tab = await open("/stream");
    await tab.until(`${LOADED} == ${FIXTURE.length}`);
    const top = FIXTURE[0]!.at_us;
    push(record("log", 2001, top + 10, 0, { text: "a new line" }));
    await tab.until(`${ROWS}[0]?.id == "record-2001"`);
    await tab.press("p");
    await tab.until(`document.querySelector(".follow-state").textContent == "Paused."`);
    push(record("log", 2002, top + 20, 0), record("log", 2003, top + 15, 0));
    await tab.until(`document.querySelector(".arrived")?.textContent == "2 new records"`);
    expect(await tab.eval(`${ROWS}[0].id`)).toBe("record-2001");
    await tab.eval(`document.querySelector(".arrived").click()`);
    await tab.until(`${ROWS}[0]?.id == "record-2002"`);
    // `since` asked for what arrived after the watermark the stream had.
    expect(asked.some((query) => query.startsWith("since="))).toBe(true);
    await tab.close();
  }, 30_000);

  test("older records load with `before` as the end of the list comes near", async () => {
    const top = FIXTURE[0]!.at_us;
    reset(Array.from({ length: 250 }, (_, i) => record("log", 5000 - i, top - i * 10, 1)));
    const tab = await open("/log");
    await tab.until(`Number(${LIST_EL}?.dataset.rows) >= 100`);
    let loaded = 0;
    while (loaded < 250) {
      loaded = await tab.eval(`Number(${LIST_EL}.dataset.rows)`);
      if (loaded >= 250) break;
      await tab.eval(`${LIST_EL}.scrollTop = ${LIST_EL}.scrollHeight`);
      await tab.until(`Number(${LIST_EL}.dataset.rows) > ${loaded}`);
    }
    expect(asked.some((query) => /(^|&)before=\d+\.\d+/.test(query))).toBe(true);
    expect(await tab.eval(`document.querySelectorAll("[role=option]").length`)).toBeLessThan(60);
    await tab.close();
  }, 30_000);

  test("the log shows level, message, fields and the request of each line", async () => {
    reset();
    const tab = await open("/log?level=warning");
    const line = FIXTURE.find((one) => one.kind === "log" && one.level === "warning")!;
    await tab.until(`${LOADED} == 1`);
    expect(asked[0]).toBe("kind=log&level=warning");
    const cells = await tab.eval(`[...document.querySelectorAll("#record-${line.key} > span")].map((s) => s.textContent)`);
    expect(cells.slice(1)).toEqual(["warning", line.text, '{"item":7}', `POST /orders ${line.request!.status}`]);
    // The level field keeps the filter, and a new level is a new link.
    expect(await tab.eval(`document.querySelector("input[name=level]").value`)).toBe("warning");
    await tab.eval(`(() => { const i = document.querySelector("input[name=level]"); i.value = "info"; i.form.requestSubmit(); })()`);
    await tab.until(`${HERE} == "/log?level=info"`);
    await tab.close();
  }, 30_000);

  test("the quarantine lists each file, and discarding the index asks first", async () => {
    reset();
    files = [{ path: "spool/9.ndjson", at_us: FIXTURE[0]!.at_us, retry_us: FIXTURE[0]!.at_us + 60_000_000, failures: 3, reason: "access is denied" }];
    const tab = await open("/stream");
    await tab.until(`document.querySelectorAll(".quarantined tbody tr").length == 1`);
    expect(await tab.eval(`document.querySelector(".quarantined tbody tr").textContent`)).toContain("access is denied");

    await tab.eval(`document.querySelector(".discard").click()`);
    await tab.until(`!!document.querySelector("dialog.ask[open]")`);
    expect(await tab.eval(`document.activeElement.textContent`)).toBe("Cancel");
    // Enter on the focused Cancel closes the dialog and discards nothing.
    await tab.press("Enter");
    await tab.until(`!document.querySelector("dialog.ask[open]")`);
    expect(discards).toBe(0);

    const before = asked.length;
    await tab.eval(`document.querySelector(".discard").click()`);
    await tab.until(`!!document.querySelector("dialog.ask[open]")`);
    await tab.eval(`document.querySelector("dialog.ask[open] .danger").click()`);
    await waitFor(() => discards === 1);
    // The view loads again.
    await waitFor(() => asked.length > before);
    await tab.until(`!document.querySelector("dialog.ask[open]")`);
    await tab.close();
  }, 30_000);

  test("a search groups its matches by request and marks each matched word", async () => {
    reset();
    const tab = await open("/search?q=stock", false);
    await tab.until(`document.querySelectorAll(".group").length == 1`);
    expect(await tab.eval(`[...document.querySelectorAll(".hit mark")].map((m) => m.textContent)`)).toEqual(["stock", "stock", "stock"]);
    expect(await tab.eval(`document.querySelector(".search input").value`)).toBe("stock");
    const link = `/requests/${FOUND.trace}/${FOUND.span}`;
    expect(await tab.eval(`[...document.querySelectorAll(".hit a")].every((a) => a.getAttribute("href") == "${link}")`)).toBe(true);
    await tab.eval(`document.querySelector(".hit a").click()`);
    await tab.until(`location.pathname == "${link}"`);
    await tab.close();
  }, 30_000);

  test("a snippet is text: markup in a record is never run", async () => {
    reset();
    const tab = await open("/search?q=markup", false);
    await tab.until(`!!document.querySelector(".hit mark")`);
    expect(await tab.eval(`document.querySelector(".hit img")`)).toBeNull();
    expect(await tab.eval(`document.querySelector(".hit .snippet").textContent`)).toBe('<img src=x onerror="document.title=1"> markup');
    expect(await tab.eval(`document.querySelector(".more").textContent`)).toBe("2 more matches in this request.");
    await tab.close();
  }, 30_000);

  test("typing a search opens its page, a trace id opens its request, and a bad search shows why", async () => {
    reset();
    const tab = await open("/stream");
    await tab.press("/");
    await tab.until(`document.activeElement === document.querySelector(".search input")`);
    await tab.type("status:abc");
    await tab.press("Enter");
    await tab.until(`${HERE} == "/search?q=status%3Aabc"`);
    await tab.until(`document.querySelector(".search-error")?.textContent == "\`status:\` must be a status such as \`500\` or \`5xx\`."`);

    // A search the service answers with `opens` moves to that request.
    await tab.eval(`history.pushState(null, "", "/search?q=${FOUND.trace}"); dispatchEvent(new PopStateEvent("popstate"))`);
    await tab.until(`location.pathname == "/requests/${FOUND.trace}/${FOUND.span}"`);
    await tab.close();
  }, 30_000);

  test("a saved filter is listed, its link opens its view, and a filter can be saved and deleted", async () => {
    reset();
    const tab = await open("/log?level=error");
    const first = settings.filters[0]!;
    await tab.until(`document.querySelector(".saved a")?.textContent == "${first.name}"`);
    expect(await tab.eval(`document.querySelector(".saved a").getAttribute("href")`)).toBe(`/saved/${encodeURIComponent(first.name)}`);

    // `s` asks for a name, and saving sends the whole settings back.
    await tab.eval(`document.querySelector("[role=listbox]").focus()`);
    await tab.press("s");
    await tab.until(`!!document.querySelector("dialog.ask[open] input")`);
    await tab.type("Errors");
    await tab.press("Enter");
    await waitFor(() => puts.length === 1);
    expect(puts[0]).toEqual({ ...settingsFixture, filters: [...settingsFixture.filters, { name: "Errors", view: "log", query: "level=error" }] } as Settings);
    await tab.until(`document.querySelectorAll(".saved a").length == 2`);

    // The saved search's link opens the search with its query.
    await tab.eval(`document.querySelector(".saved a").click()`);
    await tab.until(`${HERE} == "/search?${first.query}"`);

    // Deleting asks first, with Cancel focused.
    await tab.eval(`document.querySelector(".saved button").click()`);
    await tab.until(`!!document.querySelector("dialog.ask[open]")`);
    expect(await tab.eval(`document.activeElement.textContent`)).toBe("Cancel");
    await tab.eval(`document.querySelector("dialog.ask[open] .danger").click()`);
    await waitFor(() => puts.length === 2);
    expect(puts[1]!.filters.map((one) => one.name)).toEqual(["Errors"]);
    await tab.until(`document.querySelectorAll(".saved a").length == 1`);
    await tab.close();
  }, 30_000);

  test("a saved filter's own link opens its view after a reload", async () => {
    reset();
    const first = settings.filters[0]!;
    const tab = await open(`/saved/${encodeURIComponent(first.name)}`, false);
    await tab.until(`${HERE} == "/search?${first.query}"`);
    // The search fixture's service finds nothing for this search, and the view says so.
    await tab.until(`document.querySelector(".search-state")?.textContent == "Nothing matches this search."`);
    await tab.close();
  }, 30_000);
});
