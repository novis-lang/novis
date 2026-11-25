import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import { DIST } from "../build.ts";

// The committed bundle in a real Chromium: a page with one `#app` element, `viewer.js` and
// `viewer.css`, the way `nvs debug` serves it. What is tested is that the committed files load and
// render the frame. `chromium.ts` says how the browser is found.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    fetch(req) {
      const path = new URL(req.url).pathname;
      if (path === "/viewer.js") return new Response(Bun.file(join(DIST, "viewer.js")), { headers: { "content-type": "text/javascript" } });
      if (path === "/viewer.css") return new Response(Bun.file(join(DIST, "viewer.css")), { headers: { "content-type": "text/css" } });
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-frame");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

describe.skipIf(!EXE)("the viewer's frame in Chromium", () => {
  test("the committed bundle renders the header into #app, and its stylesheet applies", async () => {
    const { targetId, sessionId } = await browser.tab();
    await browser.send("Page.navigate", { url: `${server!.url}requests/a/b` }, sessionId);
    await browser.until(`document.readyState == "complete" && !!document.querySelector("#app h1")`, sessionId);
    expect(await browser.eval(`document.querySelector("#app h1").textContent`, sessionId)).toBe("Novis debug");
    expect(await browser.eval(`getComputedStyle(document.body).margin`, sessionId)).toBe("0px");
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);

  test("the rail's current item has aria-current=page, and only that one", async () => {
    const { targetId, sessionId } = await browser.tab();
    await browser.send("Page.navigate", { url: `${server!.url}log` }, sessionId);
    await browser.until(`document.readyState == "complete" && !!document.querySelector(".rail")`, sessionId);
    const current = `[...document.querySelectorAll(".rail [aria-current=page]")].map((a) => a.textContent)`;
    expect(await browser.eval(current, sessionId)).toEqual(["Logs"]);
    // A request's own page belongs to the request list.
    await browser.eval(`history.pushState(null, "", "/requests/${"a".repeat(32)}/${"b".repeat(16)}"); dispatchEvent(new PopStateEvent("popstate"))`, sessionId);
    await browser.until(`${current}.join() == "Requests"`, sessionId);
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);

  test("Ctrl+K opens the command palette, typing filters it, and Enter goes to the view", async () => {
    const { targetId, sessionId } = await browser.tab();
    await browser.send("Page.navigate", { url: server!.url }, sessionId);
    await browser.until(`document.readyState == "complete" && !!document.querySelector(".rail")`, sessionId);
    const key = { key: "k", code: "KeyK", windowsVirtualKeyCode: 75, modifiers: process.platform === "darwin" ? 4 : 2 };
    await browser.send("Input.dispatchKeyEvent", { type: "keyDown", ...key }, sessionId);
    await browser.send("Input.dispatchKeyEvent", { type: "keyUp", ...key }, sessionId);
    await browser.until(`document.querySelector("dialog.palette").open && document.activeElement.getAttribute("role") == "combobox"`, sessionId);
    await browser.send("Input.insertText", { text: "stream" }, sessionId);
    // The view comes first, and the last entry searches for the text.
    const options = `[...document.querySelectorAll(".palette [role=option]")].map((o) => o.textContent)`;
    await browser.until(`${options}.join("|") == "Go to StreamViews|Search for “stream”Actions"`, sessionId);
    const enter = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 };
    await browser.send("Input.dispatchKeyEvent", { type: "keyDown", ...enter }, sessionId);
    await browser.send("Input.dispatchKeyEvent", { type: "keyUp", ...enter }, sessionId);
    await browser.until(`location.pathname == "/stream" && !document.querySelector("dialog.palette").open`, sessionId);
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);

  test("the ? dialog lists every key once, and t makes a test rather than changing the theme", async () => {
    const { targetId, sessionId } = await browser.tab();
    await browser.send("Page.navigate", { url: server!.url }, sessionId);
    await browser.until(`document.readyState == "complete" && !!document.querySelector(".rail")`, sessionId);
    const keys = `[...document.querySelectorAll("dialog.keys kbd")].map((k) => k.textContent)`;
    const shown: string[] = await browser.eval(keys, sessionId);
    expect(new Set(shown).size).toBe(shown.length);
    expect(shown).toContain("?");
    expect(shown).toContain("t");
    const rows: string[] = await browser.eval(`[...document.querySelectorAll("dialog.keys tr")].map((r) => r.textContent)`, sessionId);
    expect(rows.find((r) => r.startsWith("t"))).toBe("tMake a test from a request");
    await browser.send("Target.closeTarget", { targetId });
  }, 30_000);
});
