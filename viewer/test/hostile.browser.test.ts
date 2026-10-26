import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type {
  ExceptionPage,
  ExceptionRequests,
  RequestDetail,
  RequestPage,
  RequestRow,
  SearchResults,
  Settings,
  StreamPage,
  StreamRecord,
} from "../src/api.ts";
import { DIST } from "../build.ts";
import groupFixture from "./fixtures/exception.json";
import groupsFixture from "./fixtures/exceptions.json";
import detailFixture from "./fixtures/request.json";
import listFixture from "./fixtures/requests.json";
import searchFixture from "./fixtures/search.json";
import streamFixture from "./fixtures/stream.json";

// The viewer handed records whose every text is hostile, in a real Chromium over the committed bundle
// (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`: it never shows a value in
// a way that runs it). Each answer is a Rust fixture with every string replaced by markup that would
// run script, make an element or load a URL if it were read as HTML: a `<script>`, an `<img
// onerror>`, an `<svg onload>`, an `<iframe>`, a closing `</textarea>`, a `javascript:` link, control
// characters, bidi overrides and a long run of all of them. The shapes stay the fixtures', so `tsc`
// checks them against the API types. The trace and span ids, the kinds and levels stay, so the views
// route and filter as they do over a real index.
//
// Each view is opened and then checked for what the payloads would leave behind: the flag their script
// sets, an element with the class only they carry, an element the viewer never makes, an `on…`
// attribute, a request to `/pwned/`, and a link whose scheme runs script. The hostile text must be on
// the page as text. Nothing here waits on time: an element made from a payload is found by its class
// whether or not its handler has run yet.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

/** What a payload shows when the viewer renders it as text. */
const SHOWN = "<b class=pwn>PWN</b>";
/** Markup that runs script, makes elements and fetches `/pwned/` if it is ever parsed. */
const MARKUP =
  `<script class=pwn>window.__pwned="script"</script><img class=pwn src="/pwned/img" onerror="window.__pwned='img'">` +
  `<svg class=pwn onload="window.__pwned='svg'"></svg><iframe class=pwn src="/pwned/frame"></iframe>` +
  `</textarea></title></style></noscript><a class=pwn href="javascript:window.__pwned='a'">x</a>${SHOWN}`;
/** Every replacement starts or ends with `MARKUP`, so each text the page shows has it. */
const VARIANTS = [
  MARKUP,
  `javascript:window.__pwned='link'//${MARKUP}`,
  `\u0000\u0007\u0008\u001b[31m\r\n\t\u007f\u0085${MARKUP}`,
  `‮gnp.exe‬⁦⁩‏؜﻿​${MARKUP}‮`,
  ` \tJaVa\nScRiPt:window.__pwned='link2'//${MARKUP}`,
  `data:text/html,${MARKUP}`,
  MARKUP.repeat(400),
];
/** The keys whose values the views route and filter on, kept as the fixture has them. */
const KEEP = new Set(["trace", "span", "kind", "level", "cut", "editor", "theme", "view", "older", "before"]);
/** Objects whose keys are the program's own, so a payload goes in as a key as well. */
const KEYED = new Set(["$properties", "fields"]);
const BASE64 = Buffer.from(`${MARKUP}\u0000ÿ`).toString("base64");

/** `value` with every string replaced by a hostile one, in turn from `offset`, and its shape kept. */
function poison<T>(value: T, offset = 0): T {
  let at = offset;
  const next = () => VARIANTS[at++ % VARIANTS.length]!;
  const walk = (v: unknown, key: string): unknown => {
    if (typeof v === "string") {
      if (key === "base64") return BASE64;
      // A search snippet marks each matched word with U+0002 before and U+0003 after.
      if (key === "snippet") return `${next()}\u0002${MARKUP}\u0003${next()}`;
      return next();
    }
    if (Array.isArray(v)) return v.map((one) => walk(one, key));
    if (v === null || typeof v !== "object") return v;
    const out: Record<string, unknown> = {};
    for (const [k, one] of Object.entries(v)) out[k] = KEEP.has(k) ? one : walk(one, k);
    if (KEYED.has(key)) {
      out[MARKUP] = next();
      out[VARIANTS[1]!] = MARKUP;
    }
    return out;
  };
  return walk(value, "") as T;
}

// JSON gives a header pair the type `string[]`, so the cast goes through `unknown`.
const DETAIL = poison(detailFixture as unknown as RequestDetail);
const { trace: TRACE, span: SPAN } = DETAIL.request as RequestRow & { span: string };
const LIST = poison(listFixture as RequestPage);
/** The request set against the fixture's in the compare view: another row, with other payloads. */
const OTHER_ROW = LIST.requests.find((row) => row.span !== null && row.trace !== TRACE)! as RequestRow & { span: string };
const OTHER: RequestDetail = { ...poison(detailFixture as unknown as RequestDetail, 3), request: OTHER_ROW };
const RECORDS: StreamRecord[] = poison(streamFixture as StreamPage).records;
const RESULTS = poison(searchFixture as SearchResults);
const GROUPS = poison(groupsFixture as ExceptionPage);
const GROUP = poison(groupFixture as ExceptionRequests);
// A group's class and file go into a link's query, so they are short payloads.
GROUP.group.class = VARIANTS[1]!;
GROUP.group.file = VARIANTS[3]!;

/** A custom editor template that makes a link of the file alone, so a file that is a `javascript:`
 *  URL is a link that runs script unless the viewer refuses it. */
const SETTINGS: Settings = {
  editor: "custom",
  editor_template: "{file}:{line}",
  path_mappings: [{ from: "/", to: "javascript:window.__pwned='map'//" }],
  theme: "system",
  filters: [{ name: MARKUP, query: `q=${encodeURIComponent(MARKUP)}`, view: "search" }],
};

/** Every request the page made to `/pwned/`, which only an element made from a payload makes. */
let fetched: string[] = [];
const encoder = new TextEncoder();

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/requests` with `trace` and `span`. */
function requests(url: URL): Response {
  const trace = url.searchParams.get("trace");
  const span = url.searchParams.get("span");
  const found = LIST.requests.filter((row) => (trace === null || row.trace === trace) && (span === null || row.span === span));
  return json({ watermark: LIST.watermark, requests: found, older: null } satisfies RequestPage);
}

/** `GET /api/request`: the fixture's request, the compare view's other one, or a `404`. */
function request(url: URL): Response {
  const trace = url.searchParams.get("trace");
  if (trace === TRACE) return json(DETAIL);
  if (trace === OTHER_ROW.trace) return json(OTHER);
  return json({ error: "no such request" }, 404);
}

/** `GET /api/stream` with `kind`, `level` and `trace`, newest first. */
function stream(url: URL): Response {
  const kinds = url.searchParams.get("kind")?.split(",") ?? [];
  const level = url.searchParams.get("level");
  const trace = url.searchParams.get("trace");
  const records = RECORDS.filter(
    (one) => (kinds.length === 0 || kinds.includes(one.kind)) && (level === null || one.level === level) && (trace === null || one.trace === trace),
  );
  return json({ watermark: 1, records, older: null } satisfies StreamPage);
}

/** `GET /api/exceptions`: the groups, or the one group the query names. */
function exceptions(url: URL): Response {
  if (url.searchParams.get("class") === null) return json(GROUPS);
  const { group } = GROUP;
  if (url.searchParams.get("class") !== group.class || url.searchParams.get("file") !== group.file || url.searchParams.get("line") !== String(group.line)) {
    return json({ error: "no exception has this class and site" }, 404);
  }
  return json(GROUP);
}

/** The event stream: a comment at once, and nothing more. */
function events(): Response {
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
      if (url.pathname.startsWith("/pwned")) {
        fetched.push(url.pathname);
        return new Response("", { status: 404 });
      }
      if (url.pathname === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (url.pathname === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (url.pathname === "/api/requests") return requests(url);
      if (url.pathname === "/api/request") return request(url);
      if (url.pathname === "/api/stream") return stream(url);
      if (url.pathname === "/api/search") return json(RESULTS);
      if (url.pathname === "/api/exceptions") return exceptions(url);
      if (url.pathname === "/api/quarantine") return json({ files: [] });
      if (url.pathname === "/api/replay" && req.method === "GET") return json({ target: MARKUP });
      if (url.pathname === "/api/settings" && req.method === "GET") return json(SETTINGS);
      if (url.pathname === "/api/events") return events();
      if (url.pathname.startsWith("/api/")) return json({ error: MARKUP }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-hostile");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

const KEYS = {
  r: { code: "KeyR", keyCode: 82, text: "r" },
} as const;

/** A new tab at `path`, once `ready` holds in it. A dialog the page opens sets the flag instead. */
async function open(path: string, ready: string) {
  const tab = await browser.tab();
  await browser.send("Page.enable", {}, tab.sessionId);
  await browser.send(
    "Page.addScriptToEvaluateOnNewDocument",
    { source: `for (const f of ["alert", "confirm", "prompt", "print"]) window[f] = () => { window.__pwned = f; };` },
    tab.sessionId,
  );
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && !!document.querySelector("#app h1") && (${ready})`, tab.sessionId);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    press: async (key: keyof typeof KEYS) => {
      const k = KEYS[key];
      const common = { key, code: k.code, windowsVirtualKeyCode: k.keyCode, modifiers: 0 };
      await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyDown", text: k.text }, tab.sessionId);
      await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, tab.sessionId);
    },
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

/** What the payloads left on the page. Every list is empty, `pwned` is `null` and `shown` is `true`
 *  when the viewer showed them as text. The one `<svg>` the viewer draws is the exception chart, and
 *  the viewer's own forms (the search box, the dialogs) are not counted, since no payload has one. */
const LEFT_BEHIND = `(() => {
  const body = document.body;
  const made = [...body.querySelectorAll("script, img, iframe, frame, object, embed, textarea, style, link, meta, base, svg, math, noscript")]
    .filter((el) => !(el.matches('script[type=module][src="/viewer.js"]') || el.matches("svg.exc-chart")))
    .map((el) => el.tagName.toLowerCase());
  const handlers = [...document.querySelectorAll("*")].flatMap((el) =>
    [...el.attributes].filter((a) => /^on/i.test(a.name)).map((a) => el.tagName.toLowerCase() + " " + a.name));
  const links = [...document.querySelectorAll("[href], [src], [action], [formaction], [xlink\\\\:href]")].flatMap((el) =>
    ["href", "src", "action", "formaction", "xlink:href"].map((name) => el.getAttribute(name)).filter((v) => v !== null).filter((v) => {
      try { return ["javascript:", "data:", "vbscript:", "blob:"].includes(new URL(v, location.href).protocol); } catch { return false; }
    }));
  return {
    pwned: window.__pwned ?? null,
    pwn: document.querySelectorAll(".pwn").length,
    made,
    handlers,
    links,
    shown: body.textContent.includes(${JSON.stringify(SHOWN)}),
  };
})()`;

const CLEAN = { pwned: null, pwn: 0, made: [], handlers: [], links: [], shown: true };

async function expectClean(tab: Awaited<ReturnType<typeof open>>) {
  expect(await tab.eval(LEFT_BEHIND)).toEqual(CLEAN);
  expect(fetched).toEqual([]);
}

describe.skipIf(!EXE)("the viewer over hostile values in Chromium", () => {
  test("the request list shows a hostile route, path, method and exception as text", async () => {
    const tab = await open("/", `document.querySelectorAll("[role=option]").length > 0`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("a request page shows hostile dump trees, log lines, events and an exception as text", async () => {
    const tab = await open(`/requests/${TRACE}/${SPAN}`, `document.querySelectorAll(".tl-list [role=option]").length > 0 && !!document.querySelector(".exception")`);
    await tab.until(`document.querySelectorAll("[role=tree]").length > 0 && !!document.querySelector(".log-message")`);
    // The tree's keys are hostile too: a property and a log field named with markup.
    expect(await tab.eval(`[...document.querySelectorAll("[role=treeitem]")].some((t) => t.textContent.includes(${JSON.stringify(SHOWN)}))`)).toBe(true);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("the replay dialog shows the recorded hostile method, path, headers and target as text", async () => {
    const tab = await open(`/requests/${TRACE}/${SPAN}`, `!!document.querySelector(".replay-action") && !document.querySelector(".replay-action").disabled`);
    await tab.press("r");
    await tab.until(`document.querySelector("dialog.replay")?.open == true`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("the compare view shows two requests' hostile input, headers and bodies as text", async () => {
    const tab = await open(`/compare/${TRACE}/${SPAN}/${OTHER_ROW.trace}/${OTHER_ROW.span}`, `!!document.querySelector(".cmp-count")`);
    await tab.until(`document.querySelectorAll(".cmp-part").length > 0`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("the stream and a selected record show hostile text and fields as text", async () => {
    const tab = await open("/stream", `document.querySelectorAll("[role=option]").length > 0`);
    await tab.eval(`document.querySelector("[role=option]").click()`);
    await tab.until(`!!document.querySelector(".record-detail")`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("the log shows hostile log lines as text", async () => {
    const tab = await open("/log", `document.querySelectorAll("[role=option]").length > 0`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("the search shows hostile snippets and requests as text, with the matched words marked", async () => {
    const tab = await open("/search?q=pwn", `document.querySelectorAll(".group").length > 0`);
    expect(await tab.eval(`[...document.querySelectorAll(".snippet mark")].some((m) => m.textContent == ${JSON.stringify(MARKUP)})`)).toBe(true);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("the exceptions view shows hostile classes, messages and routes as text", async () => {
    const tab = await open("/exceptions", `document.querySelectorAll("[role=option]").length == ${GROUPS.groups.length}`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);

  test("one exception group shows its hostile class, file and requests as text", async () => {
    const { group } = GROUP;
    const query = `class=${encodeURIComponent(group.class)}&file=${encodeURIComponent(group.file)}&line=${group.line}`;
    const tab = await open(`/exceptions?${query}`, `document.querySelectorAll("[role=option]").length == ${GROUP.requests.length}`);
    await expectClean(tab);
    await tab.close();
  }, 30_000);
});
