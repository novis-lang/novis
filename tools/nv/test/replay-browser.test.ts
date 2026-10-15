import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { Browser, EXE } from "./chromium.ts";

// `rule:testing/a-replay-plays-what-the-language-server-returned`'s playing rules in a real Chromium.
// A page is built from `website/config/replay-frames.mjs`'s markup, the style in
// `website/src/components/Replay.astro`, and `replay-player.mjs` loaded as a module, the way the built
// site loads it. The replay is a fixture: what is tested is the player, and `site-replay.test.ts` tests
// the recording against the real server. `chromium.ts` says how the browser is found.

const CONFIG = join(ROOT, "website", "config");
const { replayMarkup } = (await import(join(CONFIG, "replay-frames.mjs"))) as { replayMarkup(o: object): string };
const CSS = /<style is:global>([\s\S]*?)<\/style>/.exec(readFileSync(join(ROOT, "website/src/components/Replay.astro"), "utf8"))![1];

// Typing `na` opens a list at `n`, and `na` narrows it to one row. Then `name` is accepted and a hover
// shows a diagnostic.
const REPLAY = "--START--\n<?nvs\n<|>\n--STEPS--\ntype na\naccept name\nhover\n";
const RECORDING = [
  "completion 1 2:2",
  "name\tvariable\tstring",
  "number\tfunction\tint\t()\tnumber()",
  "count\tvariable\tint",
  "an\tkeyword",
  "",
  "diagnostic 3 2:1+4 E0001",
  "  `name` is not defined.",
  "",
].join("\n");

const replay = () => replayMarkup({ replay: REPLAY, recording: RECORDING, title: "01-name.nvs", install: "/install/editor/", where: "fixture" });

const page = (body: string) =>
  `<!doctype html><html><head><meta charset="utf-8"><style>${CSS}</style></head><body style="margin:0">${body}` +
  `<script type="module">import { startReplays } from "/config/replay-player.mjs"; startReplays();</script></body></html>`;

const PAGES: Record<string, string> = {
  // One replay below the first screen, with room to scroll past it.
  "/one": page(`<div style="height:1500px"></div>${replay()}<div style="height:1500px"></div>`),
  // Two replays side by side, both in view when the page opens.
  "/two": page(`<div style="display:flex;gap:1rem">${replay()}${replay()}</div>`),
};

let server: ReturnType<typeof Bun.serve> | undefined;
let browser: Browser;

beforeAll(async () => {
  if (!EXE) return;
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    fetch(req) {
      const path = new URL(req.url).pathname;
      const module = /^\/config\/([\w-]+\.mjs)$/.exec(path);
      if (module) return new Response(Bun.file(join(CONFIG, module[1]!)), { headers: { "content-type": "text/javascript" } });
      const html = PAGES[path];
      if (html === undefined) return new Response("not found", { status: 404 });
      return new Response(html, { headers: { "content-type": "text/html; charset=utf-8" } });
    },
  });
  browser = await Browser.launch("replay-browser");
}, 60_000);

afterAll(async () => {
  await browser?.close();
  server?.stop(true);
}, 30_000);

/** Opens `path` in a new tab, after `before` has set the tab up, and waits until every replay started. */
async function open(path: string, before?: (sessionId: string) => Promise<void>) {
  const { targetId, sessionId } = await browser.tab();
  await browser.send("Emulation.setFocusEmulationEnabled", { enabled: true }, sessionId);
  await before?.(sessionId);
  await browser.send("Page.navigate", { url: `${server!.url}${path.slice(1)}` }, sessionId);
  await browser.until(`location.protocol == "http:" && document.readyState == "complete" && [...document.querySelectorAll("[data-replay]")].every((r) => r.dataset.replayState)`, sessionId);
  const run = (expression: string) => browser.eval(expression, sessionId);
  return { sessionId, run, close: () => browser.send("Target.closeTarget", { targetId }) };
}

const STATES = `[...document.querySelectorAll("[data-replay]")].map((r) => r.dataset.replayState)`;
const FRAME = `+document.querySelector("[data-replay]").dataset.replayFrame`;

describe.skipIf(!EXE)("a replay in Chromium", () => {
  test("a replay starts when it scrolls into view and pauses when it leaves", async () => {
    const tab = await open("/one");
    await Bun.sleep(500);
    expect(await tab.run(STATES)).toEqual(["ready"]);
    expect(await tab.run(FRAME)).toBe(0);
    await tab.run(`document.querySelector("[data-replay]").scrollIntoView({ block: "center" })`);
    await browser.until(`${STATES}[0] == "playing" && ${FRAME} > 0`, tab.sessionId);
    await tab.run(`window.scrollTo(0, 0)`);
    await browser.until(`${STATES}[0] == "paused"`, tab.sessionId);
    const stopped = await tab.run(FRAME);
    await Bun.sleep(600);
    expect(await tab.run(FRAME)).toBe(stopped);
    await tab.run(`document.querySelector("[data-replay]").scrollIntoView({ block: "center" })`);
    await browser.until(`${STATES}[0] == "playing"`, tab.sessionId);
    await tab.close();
  }, 30_000);

  test("reduced motion shows the last frame and does not play", async () => {
    const tab = await open("/one", (sessionId) =>
      browser.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] }, sessionId),
    );
    await tab.run(`document.querySelector("[data-replay]").scrollIntoView({ block: "center" })`);
    await Bun.sleep(800);
    expect(await tab.run(STATES)).toEqual(["paused"]);
    const poster = await tab.run(`JSON.parse(document.querySelector("[data-replay-data]").textContent).poster`);
    expect(await tab.run(FRAME)).toBe(poster);
    expect(await tab.run(`document.querySelectorAll(".nv-list .nv-row").length`)).toBe(1);
    await tab.close();
  }, 30_000);

  test("only one replay plays at a time", async () => {
    const tab = await open("/two");
    await browser.until(`${STATES}.includes("playing")`, tab.sessionId);
    // Both are in view, and the first that came into view plays.
    const states: string[] = await tab.run(STATES);
    expect([...states].sort()).toEqual(["playing", "ready"]);
    const other = states.indexOf("ready");
    await tab.run(`document.querySelectorAll('[data-replay-do="play"]')[${other}].click()`);
    expect((await tab.run(STATES))[other]).toBe("playing");
    expect((await tab.run(STATES))[1 - other]).toBe("paused");
    await tab.run(`document.querySelectorAll('[data-replay-do="play"]')[${1 - other}].click()`);
    expect((await tab.run(STATES))[other]).toBe("paused");
    expect((await tab.run(STATES))[1 - other]).toBe("playing");
    await tab.close();
  }, 30_000);

  test("the code in a replay never takes keyboard focus", async () => {
    const tab = await open("/two");
    const focused: string[] = [];
    for (let i = 0; i < 12; i++) {
      for (const type of ["rawKeyDown", "keyUp"]) {
        await browser.send("Input.dispatchKeyEvent", { type, key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 }, tab.sessionId);
      }
      focused.push(await tab.run(`(() => { const a = document.activeElement; return a.closest(".nv-replay-frame") ? "frame" : a.dataset.replayDo ?? a.tagName.toLowerCase() })()`));
    }
    expect(focused).toContain("play");
    expect(focused).toContain("next");
    expect(focused).not.toContain("frame");
    // A click on the code does nothing: it does not focus it and does not change the replay.
    await tab.run(`document.querySelectorAll('[data-replay-do="play"]')[0].click()`);
    const before = await tab.run(STATES);
    const box = await tab.run(`(() => { const r = document.querySelector(".nv-replay-code").getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 } })()`);
    for (const type of ["mousePressed", "mouseReleased"]) {
      await browser.send("Input.dispatchMouseEvent", { type, x: box.x, y: box.y, button: "left", clickCount: 1 }, tab.sessionId);
    }
    expect(await tab.run(STATES)).toEqual(before);
    expect(await tab.run(`!!document.activeElement.closest(".nv-replay-frame")`)).toBe(false);
    await tab.close();
  }, 30_000);

  test("the list narrows as letters are typed", async () => {
    const tab = await open("/one");
    const next = `document.querySelector('[data-replay-do="next"]').click()`;
    const list = `[...document.querySelectorAll(".nv-list .nv-row")].map((r) => [r.querySelector(".nv-label").textContent, [...r.querySelectorAll("mark")].map((m) => m.textContent).join(""), r.classList.contains("nv-selected")])`;
    expect(await tab.run(list)).toEqual([]);
    await tab.run(next);
    expect(await tab.run(list)).toEqual([
      ["name", "n", true],
      ["number", "n", false],
      ["an", "n", false],
      ["count", "n", false],
    ]);
    await tab.run(next);
    expect(await tab.run(list)).toEqual([["name", "na", true]]);
    await tab.run(next);
    expect(await tab.run(list)).toEqual([]);
    expect(await tab.run(`document.querySelector(".nv-replay-code").textContent`)).toContain("name");
    await tab.run(next);
    expect(await tab.run(`document.querySelector(".nv-squiggle").textContent`)).toBe("name");
    expect(await tab.run(`document.querySelector(".nv-hover-message").textContent`)).toBe("`name` is not defined.");
    expect(await tab.run(STATES)).toEqual(["ended"]);
    await tab.close();
  }, 30_000);
});
