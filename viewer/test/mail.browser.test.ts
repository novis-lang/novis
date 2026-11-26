import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { MailDetail, MailPage, MailRow } from "../src/api.ts";
import { DIST } from "../build.ts";
import detailFixture from "./fixtures/mail.json";
import listFixture from "./fixtures/mails.json";

// The kept mail list and one message's page in a real Chromium, over the committed bundle. The
// server below stands in for `nvs debug`: it serves the page for every path, answers `/api/mails`
// from the list fixture and `/api/mail` from the message fixture. The Rust test `debug_api` writes
// both from a real index, so the views read the shapes the service sends.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

const LIST = listFixture as MailPage;
const DETAIL = detailFixture as MailDetail;
/** The fixture's two messages, newest first: an alert and a receipt. */
const [ALERT, RECEIPT] = LIST.mails as [MailRow, MailRow];

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/mails` over the fixture, newest first. */
function page(url: URL): Response {
  for (const key of url.searchParams.keys()) {
    if (!["before", "since", "limit"].includes(key)) return json({ error: `unknown key ${key}` }, 400);
  }
  const since = url.searchParams.has("since") ? Number(url.searchParams.get("since")) : undefined;
  const mails = LIST.mails.filter((row) => since === undefined || row.batch > since);
  return json({ watermark: LIST.watermark, mails, older: null } satisfies MailPage);
}

/** `GET /api/mail`: the one message the detail fixture has, or a `404`. */
function one(url: URL): Response {
  if (url.searchParams.get("id") !== String(DETAIL.id)) return json({ error: "no such message" }, 404);
  return json(DETAIL);
}

/** The event stream: a comment at once, and nothing more. */
function events(): Response {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(": open\n\n"));
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
      if (url.pathname === "/api/mails") return page(url);
      if (url.pathname === "/api/mail") return one(url);
      if (url.pathname === "/api/events") return events();
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-mail");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** A new tab at `path`, once `ready` is true in it. */
async function open(path: string, ready: string) {
  const tab = await browser.tab();
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, tab.sessionId);
  await browser.until(`document.readyState == "complete" && (${ready})`, tab.sessionId);
  return {
    ...tab,
    until: (expression: string) => browser.until(expression, tab.sessionId),
    eval: (expression: string) => browser.eval(expression, tab.sessionId),
    enter: async () => {
      const common = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, modifiers: 0 };
      await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyDown", text: "\r" }, tab.sessionId);
      await browser.send("Input.dispatchKeyEvent", { ...common, type: "keyUp" }, tab.sessionId);
    },
    close: () => browser.send("Target.closeTarget", { targetId: tab.targetId }),
  };
}

/** The ids of the list's rows in the DOM, in list order. */
const OPTION_IDS = `[...document.querySelectorAll("[role=option]")].sort((a, b) => a.ariaPosInSet - b.ariaPosInSet).map((o) => o.id)`;
/** The text of each cell of one row, less its age, which depends on the clock. */
const CELLS = (id: number) => `[...document.querySelectorAll("#mail-${id} > span:not(.age)")].map((s) => s.textContent)`;
/** The text of the header table's row named `name`, or `null` when it has none. */
const HEADER = (name: string) =>
  `[...document.querySelectorAll(".mail-headers tr")].find((tr) => tr.querySelector("th").textContent == ${JSON.stringify(name)})?.querySelector("td").textContent ?? null`;

describe.skipIf(!EXE)("the kept mail list and a message's page in Chromium", () => {
  test("the list shows every message newest first, with its block, subject and recipients", async () => {
    const tab = await open("/mail", `document.querySelectorAll("[role=option]").length == 2`);
    expect(await tab.eval(OPTION_IDS)).toEqual([ALERT, RECEIPT].map((row) => `mail-${row.id}`));
    expect(await tab.eval(`document.querySelector("[role=listbox]").getAttribute("aria-label")`)).toBe("Kept mail, newest first");
    expect(await tab.eval(`[...document.querySelectorAll(".cols > span")].map((s) => s.textContent)`)).toEqual(["Sent", "Block", "Subject", "To"]);
    expect(await tab.eval(CELLS(ALERT.id))).toEqual(["alerts", "Disk almost full", "ops@example.com"]);
    expect(await tab.eval(CELLS(RECEIPT.id))).toEqual(["outbox", "Your receipt", "ada@example.com, bo@example.com"]);
    // The rail names the Mail view as the current page.
    expect(await tab.eval(`document.querySelector("a[href='/mail'][aria-current=page]") !== null`)).toBe(true);
    await tab.close();
  }, 30_000);

  test("a click on a row opens the message, with its headers, its HTML in a sandbox, its text and its attachments", async () => {
    const tab = await open("/mail", `document.querySelectorAll("[role=option]").length == 2`);
    await tab.eval(`document.querySelector("[role=listbox]").focus()`);
    await tab.until(`document.querySelector("[role=option][aria-selected=true]") !== null`);
    await tab.eval(`document.querySelector("#mail-${RECEIPT.id}").click()`);
    await tab.until(`location.pathname == "/mail/${RECEIPT.id}" && document.querySelector(".mail-headers") !== null`);
    expect(await tab.eval(`document.querySelector(".req-title h2").textContent`)).toBe("Your receipt");

    // The From shows the name, and the other headers show what the message has.
    expect(await tab.eval(HEADER("From"))).toBe(`"Shop" <noreply@example.com>`);
    expect(await tab.eval(HEADER("To"))).toBe("ada@example.com, bo@example.com");
    expect(await tab.eval(HEADER("Cc"))).toBe("desk@example.com");
    expect(await tab.eval(HEADER("Reply-To"))).toBe("help@example.com");
    expect(await tab.eval(HEADER("In-Reply-To"))).toBe("<0a1b@example.com>");
    expect(await tab.eval(HEADER("Auto-Submitted"))).toBe("auto-generated");
    expect(await tab.eval(HEADER("Bounce tag"))).toBe("order-7");
    // Bcc is apart from the headers every recipient sees.
    expect(await tab.eval(HEADER("Bcc"))).toBeNull();
    expect(await tab.eval(`document.querySelector(".mail-bcc .rt-code").textContent`)).toBe("archive@example.com");

    // The HTML as sent is in a frame with an empty sandbox, its inline image a data URL, and the CSP first.
    expect(await tab.eval(`document.querySelector("iframe.mail-frame").getAttribute("sandbox")`)).toBe("");
    const srcdoc = (await tab.eval(`document.querySelector("iframe.mail-frame").getAttribute("srcdoc")`)) as string;
    expect(srcdoc).not.toContain("cid:4f1d@inline");
    expect(srcdoc).toContain(`src="data:image/png;base64,iVBORw0KGgo`);
    expect(srcdoc.startsWith(`<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">`)).toBe(
      true,
    );

    // The HTML as written shows as source text, and no second frame draws it.
    await tab.eval(`[...document.querySelectorAll(".mail-html .chip")].find((c) => c.textContent == "As written").click()`);
    await tab.until(`document.querySelector(".mail-source") !== null`);
    expect(await tab.eval(`document.querySelector(".mail-source").textContent`)).toBe(DETAIL.message.html_written!);
    expect(await tab.eval(`document.querySelectorAll("iframe").length`)).toBe(0);

    // The text part.
    expect(await tab.eval(`document.querySelector(".mail-text").textContent`)).toBe(DETAIL.message.text);

    // Both attachments are download links, and the image is marked inline.
    expect(await tab.eval(`[...document.querySelectorAll(".mail-files a[download]")].map((a) => [a.getAttribute("download"), a.getAttribute("href")])`)).toEqual([
      ["logo.png", `data:image/png;base64,${DETAIL.message.attachments![0]!.base64}`],
      ["receipt.csv", `data:text/csv;base64,${DETAIL.message.attachments![1]!.base64}`],
    ]);
    expect(await tab.eval(`document.querySelectorAll(".mail-files .mail-inline").length`)).toBe(1);

    // The CSS report lists the kept rules and the unused ones.
    expect(await tab.eval(`[...document.querySelectorAll(".mail-kept li")].map((li) => li.textContent)`)).toEqual(DETAIL.message.kept_css!);
    expect(await tab.eval(`[...document.querySelectorAll(".mail-unused li")].map((li) => li.textContent)`)).toEqual(DETAIL.message.unused_css!);
    await tab.close();
  }, 30_000);

  test("an id with no message says so", async () => {
    // The page shows "Loading." until the fetch ends, so the wait is on a note with other text.
    const tab = await open("/mail/999", `(document.querySelector(".mail-one .note")?.textContent ?? "Loading.") !== "Loading."`);
    expect(await tab.eval(`document.querySelector(".mail-one .note").textContent`)).toBe("No kept message has this id.");
    await tab.close();
  }, 30_000);
});
