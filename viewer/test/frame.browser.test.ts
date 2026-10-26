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
});
