import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { RequestDetail, RequestPage } from "../src/api.ts";
import { DIST } from "../build.ts";
import detailFixture from "./fixtures/request.json";
import listFixture from "./fixtures/requests.json";

// The viewer under the base path an editor frames it at, `/embed/<token>/`, in a real Chromium over
// the committed bundle. The server here answers only under that prefix, as `nvs debug` does for a
// framed page, and keeps every path it was asked. A path the viewer asks without the prefix is a `404`
// and shows in that list.
//
// The page is opened at the top level, so `window.parent` is the page itself. A message from the panel
// is a `MessageEvent` dispatched with the panel's origin, and what the page sends is caught by
// replacing `window.postMessage`.

const TOKEN = "0123456789abcdef0123456789abcdef";
const BASE = `/embed/${TOKEN}`;
const PANEL = "vscode-webview://0a1b2c3d";
const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="${BASE}/viewer.css"></head>
<body><div id="app"></div><script type="module" src="${BASE}/viewer.js"></script></body></html>`;

const DETAIL = detailFixture as unknown as RequestDetail;
const LIST = listFixture as RequestPage;
const { trace: TRACE, span: SPAN } = DETAIL.request;

/** Every path the page asked for, in order. */
const asked: string[] = [];

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    fetch(req) {
      const url = new URL(req.url);
      asked.push(url.pathname);
      if (!url.pathname.startsWith(`${BASE}/`) && url.pathname !== BASE) return new Response("not under the prefix", { status: 404 });
      const path = url.pathname.slice(BASE.length) || "/";
      if (path === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (path === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      if (path === "/api/events") return new Response(`data: ${LIST.watermark}\n\n`, { headers: { "content-type": "text/event-stream" } });
      if (path === "/api/requests") {
        const trace = url.searchParams.get("trace");
        const span = url.searchParams.get("span");
        const requests = LIST.requests.filter((row) => (trace === null || row.trace === trace) && (span === null || row.span === span));
        return Response.json({ ...LIST, requests, older: null } satisfies RequestPage);
      }
      if (path === "/api/request") return Response.json(DETAIL);
      if (path.startsWith("/api/")) return Response.json({ error: "not in this test" }, { status: 404 });
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-embed");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** A message from `origin` to the page, as the panel's `postMessage` delivers it. */
const message = (data: unknown, origin: string) =>
  `dispatchEvent(new MessageEvent("message", { data: ${JSON.stringify(data)}, origin: ${JSON.stringify(origin)}, source: window }))`;

describe.skipIf(!EXE)("the viewer an editor frames, in Chromium", () => {
  test("every fetch, the event stream and every move stay under the base path", async () => {
    const { targetId, sessionId } = await browser.tab();
    asked.length = 0;
    await browser.send("Page.navigate", { url: `${server!.url.origin}${BASE}/` }, sessionId);
    await browser.until(`document.readyState == "complete" && document.querySelectorAll("[role=option]").length > 0`, sessionId);
    await browser.until(`performance.getEntriesByType("resource").some((e) => e.name.includes("/api/events"))`, sessionId);
    const api = asked.filter((path) => path.includes("/api/"));
    expect(api.length).toBeGreaterThan(0);
    expect(asked.filter((path) => !path.startsWith(BASE))).toEqual([]);
    expect(api).toContain(`${BASE}/api/requests`);
    expect(api).toContain(`${BASE}/api/events`);

    // A click on a rail link moves inside the viewer and keeps the prefix in the address bar.
    await browser.eval(`document.querySelector('.rail a[href="/routes"]').click()`, sessionId);
    await browser.until(`location.pathname == ${JSON.stringify(`${BASE}/routes`)}`, sessionId);
    await browser.until(`document.querySelector('.rail [aria-current=page]')?.textContent.includes("Routes")`, sessionId);
    // So does a click with Ctrl held, because a new window would open the path without the prefix.
    await browser.eval(
      `document.querySelector('.rail a[href="/log"]').dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, ctrlKey: true }))`,
      sessionId,
    );
    await browser.until(`location.pathname == ${JSON.stringify(`${BASE}/log`)}`, sessionId);
    await browser.eval(`history.back()`, sessionId);
    await browser.until(`document.querySelector('.rail [aria-current=page]')?.textContent.includes("Routes")`, sessionId);
    expect(asked.filter((path) => !path.startsWith(BASE))).toEqual([]);
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);

  test("the panel's messages move the page and set the theme, and a message from any other origin is ignored", async () => {
    const { targetId, sessionId } = await browser.tab();
    await browser.send("Page.navigate", { url: `${server!.url.origin}${BASE}/` }, sessionId);
    await browser.until(`document.readyState == "complete" && !!document.querySelector(".rail")`, sessionId);

    for (const origin of ["https://attacker.example", "http://127.0.0.1", "null", "vscode-file://vscode-app"]) {
      await browser.eval(message({ type: "go", path: "/log" }, origin), sessionId);
      await browser.eval(message({ type: "theme", value: "dark" }, origin), sessionId);
    }
    for (const path of ["//attacker.example/", "javascript:alert(1)", "https://attacker.example/", "/api/settings", "/no-such-view", "\\\\host\\share", 7]) {
      await browser.eval(message({ type: "go", path }, PANEL), sessionId);
    }
    await browser.eval(message({ type: "theme", value: "<b>" }, PANEL), sessionId);
    expect(await browser.eval(`location.pathname`, sessionId)).toBe(`${BASE}/`);
    expect(await browser.eval(`document.documentElement.dataset.theme ?? "system"`, sessionId)).toBe("system");

    await browser.eval(message({ type: "go", path: "/log" }, PANEL), sessionId);
    await browser.until(`location.pathname == ${JSON.stringify(`${BASE}/log`)}`, sessionId);
    await browser.eval(message({ type: "go", path: `/requests/${TRACE}/${SPAN}?tab=queries` }, PANEL), sessionId);
    await browser.until(`location.pathname + location.search == ${JSON.stringify(`${BASE}/requests/${TRACE}/${SPAN}?tab=queries`)}`, sessionId);
    await browser.eval(message({ type: "theme", value: "dark" }, PANEL), sessionId);
    await browser.until(`document.documentElement.dataset.theme == "dark"`, sessionId);
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);

  test("a click on a source site sends open to the panel's origin, and to nobody before the panel has spoken", async () => {
    const { targetId, sessionId } = await browser.tab();
    await browser.send("Page.navigate", { url: `${server!.url.origin}${BASE}/requests/${TRACE}/${SPAN}?tab=dumps` }, sessionId);
    await browser.until(`document.readyState == "complete" && !!document.querySelector("a.site-link")`, sessionId);
    await browser.eval(`window.__sent = []; window.postMessage = (data, origin) => window.__sent.push([data, origin])`, sessionId);
    const site = `document.querySelector("a.site-link")`;
    const text: string = await browser.eval(`${site}.textContent`, sessionId);

    await browser.eval(`${site}.click()`, sessionId);
    expect(await browser.eval(`window.__sent`, sessionId)).toEqual([]);

    await browser.eval(message({ type: "theme", value: "light" }, PANEL), sessionId);
    await browser.eval(`${site}.click()`, sessionId);
    const sent: [{ type: string; file: string; line: number; column?: number }, string][] = await browser.eval(`window.__sent`, sessionId);
    expect(sent.length).toBe(1);
    const [data, origin] = sent[0]!;
    expect(origin).toBe(PANEL);
    expect(data.type).toBe("open");
    expect(text.startsWith(`${data.file}:${data.line}`)).toBe(true);
    expect(await browser.eval(`location.pathname`, sessionId)).toBe(`${BASE}/requests/${TRACE}/${SPAN}`);
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);
});
