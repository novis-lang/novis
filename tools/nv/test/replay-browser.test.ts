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
const LIST = ["completion 1 2:2", "name\tvariable\tstring", "number\tfunction\tint\t()\tnumber()", "count\tvariable\tint", "an\tkeyword", ""];
const RECORDING = [...LIST, "diagnostic 3 2:1+4 E0001", "  `name` is not defined.", ""].join("\n");
// The same steps with no hover: a recording that shows only completion lists.
const LIST_ONLY = { replay: "--START--\n<?nvs\n<|>\n--STEPS--\ntype na\naccept name\n", recording: LIST.join("\n") };

const markup = (o: { replay: string; recording: string }) => replayMarkup({ ...o, title: "01-name.nvs", install: "/install/editor/", where: "fixture" });
const replay = () => markup({ replay: REPLAY, recording: RECORDING });

// Two of Starlight's own rules, in its layers: every box is border-box, and the page's text adds a margin
// between sibling blocks unless they are under `.not-content`. The replay sits in that text on a page.
const STARLIGHT =
  "@layer starlight.reset { *, *::before, *::after { box-sizing: border-box } * { margin: 0 } }" +
  "@layer starlight.content { .sl-markdown-content :not(a, strong, em, del, span, input, code, br) + :not(a, strong, em, del, span, input, code, br, :where(.not-content *)) { margin-top: 1rem } }";

const page = (body: string) =>
  `<!doctype html><html><head><meta charset="utf-8"><style>${STARLIGHT}${CSS}</style></head><body><div class="sl-markdown-content">${body}</div>` +
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

/** Opens `path` in a new tab, after `before` has set the tab up, and waits until every replay has its player. */
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
const DATA = `JSON.parse(document.querySelector("[data-replay-data]").textContent)`;
const BUTTONS = `[...document.querySelectorAll('[data-replay-do="play"]')].map((b) => [b.textContent, b.querySelectorAll("svg").length])`;
const CLICK = (i: number) => `document.querySelectorAll('[data-replay-do="play"]')[${i}].click()`;
const ROWS = `[...document.querySelectorAll(".nv-list .nv-row")].map((r) => [r.querySelector(".nv-label").textContent, [...r.querySelectorAll("mark")].map((m) => m.textContent).join(""), r.classList.contains("nv-selected")])`;
const PROGRESS = `getComputedStyle(document.querySelector(".nv-replay-progress")).getPropertyValue("--nv-progress")`;
const INTO_VIEW = `document.querySelector("[data-replay]").scrollIntoView({ block: "center" })`;

describe("a replay's button", () => {
  test("is labelled with what the recording shows", () => {
    expect(replay()).toContain('data-replay-label="Preview the error message"');
    expect(markup(LIST_ONLY)).toContain('data-replay-label="Preview the autocompletion"');
    expect(replay()).not.toContain('data-replay-do="next"');
  });
});

describe.skipIf(!EXE)("a replay in Chromium", () => {
  test("a replay does not start by itself, and shows its last list until it plays", async () => {
    const tab = await open("/one");
    await tab.run(INTO_VIEW);
    await Bun.sleep(800);
    expect(await tab.run(STATES)).toEqual(["ready"]);
    expect(await tab.run(FRAME)).toBe(await tab.run(`${DATA}.poster`));
    expect(await tab.run(ROWS)).toEqual([["name", "na", true]]);
    expect(await tab.run(BUTTONS)).toEqual([["Preview the error message", 1]]);
    expect(await tab.run(PROGRESS)).toBe("0");
    expect(await tab.run(`document.querySelector("[data-replay]").classList.contains("not-content")`)).toBe(true);
    await tab.close();
  }, 30_000);

  test("the button plays from the first frame, pauses, and plays again after the end", async () => {
    const tab = await open("/one");
    await tab.run(INTO_VIEW);
    await tab.run(CLICK(0));
    expect(await tab.run(STATES)).toEqual(["playing"]);
    expect(await tab.run(FRAME)).toBe(0);
    expect(await tab.run(BUTTONS)).toEqual([["Pause", 1]]);
    await browser.until(`${FRAME} > 0`, tab.sessionId);
    await tab.run(CLICK(0));
    expect(await tab.run(STATES)).toEqual(["paused"]);
    expect(await tab.run(BUTTONS)).toEqual([["Preview the error message", 1]]);
    const stopped = await tab.run(FRAME);
    await Bun.sleep(1600);
    expect(await tab.run(FRAME)).toBe(stopped);
    // A paused replay plays on from the frame it stopped at.
    await tab.run(CLICK(0));
    expect(await tab.run(FRAME)).toBe(stopped);
    await browser.until(`${STATES}[0] == "ended"`, tab.sessionId);
    expect(await tab.run(FRAME)).toBe(await tab.run(`${DATA}.frames.length - 1`));
    expect(await tab.run(BUTTONS)).toEqual([["Play again", 1]]);
    expect(await tab.run(PROGRESS)).toBe("1");
    await tab.run(CLICK(0));
    expect(await tab.run(STATES)).toEqual(["playing"]);
    expect(await tab.run(FRAME)).toBe(0);
    await tab.close();
  }, 30_000);

  test("a replay pauses when it scrolls out of view, and stays paused when it comes back", async () => {
    const tab = await open("/one");
    await tab.run(INTO_VIEW);
    await tab.run(CLICK(0));
    await browser.until(`${FRAME} > 0`, tab.sessionId);
    await tab.run(`window.scrollTo(0, 0)`);
    await browser.until(`${STATES}[0] == "paused"`, tab.sessionId);
    const stopped = await tab.run(FRAME);
    await tab.run(INTO_VIEW);
    await Bun.sleep(1600);
    expect(await tab.run(STATES)).toEqual(["paused"]);
    expect(await tab.run(FRAME)).toBe(stopped);
    await tab.close();
  }, 30_000);

  test("only one replay plays at a time", async () => {
    const tab = await open("/two");
    expect(await tab.run(STATES)).toEqual(["ready", "ready"]);
    await tab.run(CLICK(0));
    expect(await tab.run(STATES)).toEqual(["playing", "ready"]);
    await tab.run(CLICK(1));
    expect(await tab.run(STATES)).toEqual(["paused", "playing"]);
    await tab.run(CLICK(0));
    expect(await tab.run(STATES)).toEqual(["playing", "paused"]);
    await tab.close();
  }, 30_000);

  test("under reduced motion the cursor does not blink and the progress bar does not slide", async () => {
    const tab = await open("/one", (sessionId) =>
      browser.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] }, sessionId),
    );
    await tab.run(INTO_VIEW);
    expect(await tab.run(STATES)).toEqual(["ready"]);
    expect(await tab.run(FRAME)).toBe(await tab.run(`${DATA}.poster`));
    expect(await tab.run(`getComputedStyle(document.querySelector(".nv-caret")).animationName`)).toBe("none");
    expect(await tab.run(`getComputedStyle(document.querySelector(".nv-replay-progress span")).transitionDuration`)).toBe("0s");
    await tab.close();
    // Without the setting, both move.
    const moving = await open("/one");
    expect(await moving.run(`getComputedStyle(document.querySelector(".nv-caret")).animationName`)).toBe("nv-blink");
    expect(await moving.run(`getComputedStyle(document.querySelector(".nv-replay-progress span")).transitionDuration`)).not.toBe("0s");
    await moving.close();
  }, 30_000);

  test("the button works from the keyboard, and the code in a replay never takes keyboard focus", async () => {
    const tab = await open("/two");
    const focused: string[] = [];
    for (let i = 0; i < 12; i++) {
      for (const type of ["rawKeyDown", "keyUp"]) {
        await browser.send("Input.dispatchKeyEvent", { type, key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 }, tab.sessionId);
      }
      focused.push(await tab.run(`(() => { const a = document.activeElement; return a.closest(".nv-replay-frame") ? "frame" : a.dataset.replayDo ?? a.tagName.toLowerCase() })()`));
      if (focused.at(-1) === "play") break;
    }
    expect(focused).toContain("play");
    expect(focused).not.toContain("frame");
    // Enter on the focused button plays its replay.
    for (const type of ["keyDown", "keyUp"]) {
      await browser.send("Input.dispatchKeyEvent", { type, key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: type === "keyDown" ? "\r" : undefined }, tab.sessionId);
    }
    await browser.until(`${STATES}[0] == "playing"`, tab.sessionId);
    // A click on the code does nothing: it does not focus it and does not change the replay.
    const before = await tab.run(STATES);
    const box = await tab.run(`(() => { const r = document.querySelector(".nv-replay-code").getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 } })()`);
    for (const type of ["mousePressed", "mouseReleased"]) {
      await browser.send("Input.dispatchMouseEvent", { type, x: box.x, y: box.y, button: "left", clickCount: 1 }, tab.sessionId);
    }
    expect(await tab.run(STATES)).toEqual(before);
    expect(await tab.run(`!!document.activeElement.closest(".nv-replay-frame")`)).toBe(false);
    await tab.close();
  }, 30_000);

  test("the list narrows as letters are typed, and opens under the start of the word", async () => {
    const tab = await open("/one");
    const show = (i: number) => tab.run(`document.querySelector("[data-replay]").nvReplay.show(${i})`);
    await show(0);
    expect(await tab.run(ROWS)).toEqual([]);
    await show(1);
    expect(await tab.run(ROWS)).toEqual([
      ["name", "n", true],
      ["number", "n", false],
      ["an", "n", false],
      ["count", "n", false],
    ]);
    // The list's top-left corner is on the line under the cursor, at the column of the `n` it completes.
    const place = await tab.run(`(() => {
      const caret = document.querySelector(".nv-caret").getBoundingClientRect();
      const line = document.querySelector(".nv-caret").closest(".nv-line").getBoundingClientRect();
      const text = document.querySelector(".nv-caret").closest(".nv-text");
      const ch = text.getBoundingClientRect().width / text.textContent.length;
      const list = document.querySelector(".nv-list").getBoundingClientRect();
      return [Math.round(list.left - (caret.left - ch)), Math.round(list.top - line.bottom)];
    })()`);
    expect(place).toEqual([0, 0]);
    await show(2);
    expect(await tab.run(ROWS)).toEqual([["name", "na", true]]);
    await show(3);
    expect(await tab.run(ROWS)).toEqual([]);
    expect(await tab.run(`document.querySelector(".nv-replay-code").textContent`)).toContain("name");
    await show(4);
    expect(await tab.run(`document.querySelector(".nv-squiggle").textContent`)).toBe("name");
    expect(await tab.run(`document.querySelector(".nv-hover-message").textContent`)).toBe("`name` is not defined.");
    await tab.close();
  }, 30_000);
});
