import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { Browser, EXE } from "../../tools/nv/test/chromium.ts";
import type { RunDetail, RunPage, RunRow } from "../src/api.ts";
import { DIST } from "../build.ts";
import detailFixture from "./fixtures/run.json";
import listFixture from "./fixtures/runs.json";

// The run list and one run's page in a real Chromium, over the committed bundle. The server below
// stands in for `nvs debug`: it serves the page for every path, answers `/api/runs` from the list
// fixture with the `of`, `outcome`, `min_us` and `min_memory` filters, and `/api/run` from the run
// fixture. The Rust test `debug_api` writes both from a real index, so the views read the shapes
// the service sends.

const PAGE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="/viewer.css"></head>
<body><div id="app"></div><script type="module" src="/viewer.js"></script></body></html>`;

// A JSON import types a run's `of`, `outcome` and an event's `kind` as `string`.
const LIST = listFixture as unknown as RunPage;
const DETAIL = detailFixture as unknown as RunDetail;
/** The fixture's three runs, newest first: a command, a failed queue job and a schedule. */
const [COMMAND, QUEUE, SCHEDULE] = LIST.runs as [RunRow, RunRow, RunRow];

/** The filters the run list sends, and every key `/api/runs` takes. */
const FILTERS = ["of", "outcome", "min_us", "min_memory", "name"];
/** Every `/api/runs` query, in order, so a test can read what the list sent. */
const asked: string[] = [];

function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}

/** `GET /api/runs` over the fixture, newest first, the way the service filters it. */
function page(url: URL): Response {
  for (const key of url.searchParams.keys()) {
    if (![...FILTERS, "before", "since", "limit"].includes(key)) return json({ error: `unknown key ${key}` }, 400);
  }
  asked.push(url.search);
  const get = (key: string) => url.searchParams.get(key);
  const num = (key: string) => (url.searchParams.has(key) ? Number(get(key)) : undefined);
  const since = num("since");
  const runs = LIST.runs.filter(
    (row) =>
      (since === undefined || row.batch > since) &&
      (get("of") === null || row.of === get("of")) &&
      (get("outcome") === null || row.outcome === get("outcome")) &&
      (get("name") === null || row.name === get("name")) &&
      (num("min_us") === undefined || row.duration_us >= num("min_us")!) &&
      (num("min_memory") === undefined || (row.memory_peak ?? -1) >= num("min_memory")!),
  );
  return json({ watermark: LIST.watermark, runs, older: null } satisfies RunPage);
}

/** `GET /api/run`: the one run the detail fixture has, or a `404`. */
function one(url: URL): Response {
  if (url.searchParams.get("trace") !== DETAIL.run.trace) return json({ error: "no such run" }, 404);
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
      if (url.pathname === "/api/runs") return page(url);
      if (url.pathname === "/api/run") return one(url);
      if (url.pathname === "/api/events") return events();
      if (url.pathname.startsWith("/api/")) return json({ error: "not found" }, 404);
      return new Response(PAGE, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("viewer-runs");
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
const CELLS = (id: number) => `[...document.querySelectorAll("#run-${id} > span:not(.age)")].map((s) => s.textContent)`;

describe.skipIf(!EXE)("the run list and a run's page in Chromium", () => {
  test("the list shows every run newest first, with its kind, outcome, duration and memory", async () => {
    const tab = await open("/runs", `document.querySelectorAll("[role=option]").length == 3`);
    expect(await tab.eval(OPTION_IDS)).toEqual([COMMAND, QUEUE, SCHEDULE].map((row) => `run-${row.id}`));
    expect(await tab.eval(`document.querySelector("[role=listbox]").getAttribute("aria-label")`)).toBe("Runs, newest first");
    // Kind, name, outcome, duration, memory, queries and dumps.
    expect(await tab.eval(CELLS(COMMAND.id))).toEqual(["Command", "run scripts/import.nvs", "OK", "30.0 ms", "4.00 MiB", "1", "1"]);
    expect(await tab.eval(CELLS(SCHEDULE.id))).toEqual(["Schedule", "nightly-report", "OK", "2.00 ms", "1.00 MiB", "1", "1"]);
    // A run that threw shows its exception's class and site after its name, and its outcome in red.
    expect(await tab.eval(`document.querySelector("#run-${QUEUE.id} .exc").textContent`)).toBe("Shop\\MailDown");
    expect(await tab.eval(`document.querySelector("#run-${QUEUE.id} .exc-at").textContent`)).toBe("jobs/receipt.nvs:9");
    expect(await tab.eval(`document.querySelector("#run-${QUEUE.id} .outcome").className`)).toContain("s5");
    // The rail names the Runs view as the current page.
    expect(await tab.eval(`document.querySelector("a[href='/runs'][aria-current=page]") !== null`)).toBe(true);
    await tab.close();
  }, 30_000);

  test("a filter is part of the link, and the list asks the service for it", async () => {
    asked.length = 0;
    const tab = await open("/runs?outcome=failed", `document.querySelectorAll("[role=option]").length == 1`);
    expect(await tab.eval(OPTION_IDS)).toEqual([`run-${QUEUE.id}`]);
    expect(asked.some((search) => new URLSearchParams(search).get("outcome") === "failed")).toBe(true);
    expect(await tab.eval(`document.querySelector("[role=listbox]").getAttribute("aria-label")`)).toBe("Runs that match the filter, newest first");
    await tab.close();

    // A filter no run matches shows a button that clears it.
    const none = await open("/runs?of=schedule&outcome=failed", `document.querySelector(".empty") !== null`);
    expect(await none.eval(`document.querySelector(".empty").textContent`)).toContain("No run matches this filter");
    await none.close();
  }, 30_000);

  test("Enter on a row opens its page, with the run's tabs and its exception's stack", async () => {
    const tab = await open("/runs?of=queue", `document.querySelectorAll("[role=option]").length == 1`);
    await tab.eval(`document.querySelector("[role=listbox]").focus()`);
    await tab.until(`document.querySelector("[role=option][aria-selected=true]") !== null`);
    await tab.enter();
    await tab.until(`location.pathname == "/runs/${QUEUE.trace}" && document.querySelector(".req-head") !== null`);
    expect(await tab.eval(`document.querySelector(".req-title h2").textContent`)).toBe("Queue jobs/receipt.nvs");
    expect(await tab.eval(`document.querySelector(".req-title .outcome").textContent`)).toBe("Failed");
    expect(await tab.eval(`document.querySelector(".req-crumb .req-id").textContent`)).toBe(QUEUE.trace);
    // A run has no Request, Response or Coverage tab, and an Exception tab because it threw.
    expect(await tab.eval(`[...document.querySelectorAll(".tabs a")].map((a) => a.textContent.replace(/\\d+$/, ""))`)).toEqual([
      "Timeline",
      "Queries",
      "Cache",
      "Dumps",
      "Logs",
      "Exception",
    ]);
    // The timeline has the run's query, its dump and its log line.
    await tab.until(`document.querySelectorAll(".tl-list [role=option]").length == 3`);
    expect(await tab.eval(`[...document.querySelectorAll(".tl-list [role=option] .tl-kind")].map((s) => s.textContent)`)).toEqual([
      "Query",
      "Dump",
      "Log line",
    ]);
    await tab.close();

    const thrown = await open(`/runs/${QUEUE.trace}?tab=exception`, `document.querySelector(".exception .exc-head") !== null`);
    expect(await thrown.eval(`document.querySelector(".exception .exc-head").textContent`)).toBe("Shop\\MailDown: the mail server did not answer");
    expect(await thrown.eval(`[...document.querySelectorAll(".exception > .stack > .frames .frame")].map((f) => f.textContent)`)).toEqual([
      "{main}jobs/receipt.nvs:9",
    ]);
    await thrown.close();
  }, 30_000);

  test("a trace with no run says so", async () => {
    const tab = await open(`/runs/${"0".repeat(31)}9`, `document.querySelector(".run .note") !== null`);
    expect(await tab.eval(`document.querySelector(".run .note").textContent`)).toBe("No run has this trace id.");
    await tab.close();
  }, 30_000);
});
