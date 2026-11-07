import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { cpSync, mkdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { newestNvs } from "../lib/nvs-binary.ts";
import { ROOT } from "../lib/paths.ts";
import { Browser, EXE } from "./chromium.ts";

// The browser half of `apps/chat` and `apps/poll` in a real Chromium, under each app's own
// `Content-Security-Policy`. A copy of each app is served by `nvs serve` on a free port, with a data
// folder of its own, so its SQLite file and its sessions start empty and a restart keeps the sessions.
//
// Each page is a tab in a browser context of its own, so two pages are two visitors with their own
// cookies. Every page records its console errors, its uncaught exceptions and every message Chromium
// logs, and each test ends by checking that none of them is an error: a script the policy blocked, a
// connection it blocked, or a script that threw. A step ends on a page condition, polled, with a bound
// that only names a stuck step.

// A relative `NVS_BIN`, as CI gives, is resolved from the repository root, because `serve` runs the
// binary from inside the app's copy.
const NVS = process.env.NVS_BIN ? resolve(ROOT, process.env.NVS_BIN) : newestNvs();
const DIR = join(ROOT, ".agent-tmp", `realtime-copy-${process.pid}`);

/** The longest one step of a page may take. A bound on a stuck step, never a wait a test relies on. */
const STEP_MS = 60_000;

/** One app's copy, served by `nvs serve`. */
class Served {
  readonly dir: string;
  private proc: ReturnType<typeof Bun.spawn> | undefined;
  /** Everything the server printed, for a stuck step's message. */
  said = "";
  /** `127.0.0.1:<port>`, from the server's `listening on` line. */
  addr = "";

  constructor(readonly app: string) {
    this.dir = join(DIR, app);
    mkdirSync(join(this.dir, "data"), { recursive: true });
    for (const part of ["src", "public", "nvs.toml"]) cpSync(join(ROOT, "apps", app, part), join(this.dir, part), { recursive: true });
  }

  get origin() {
    return `http://${this.addr}`;
  }

  /** Serves the copy on `listen` and resolves once the server prints its `listening on` line. */
  async start(listen = "127.0.0.1:0"): Promise<void> {
    const proc = Bun.spawn(
      [NVS!, "--data", join(this.dir, "data"), "--no-init", "serve", "public/index.nvs", "--config", "nvs.toml", "--listen", listen],
      { cwd: this.dir, stdin: "ignore", stdout: "pipe", stderr: "pipe" },
    );
    this.proc = proc;
    const from = this.said.length;
    const drain = async (s: ReadableStream<Uint8Array>, found?: (addr: string) => void) => {
      const dec = new TextDecoder();
      for await (const chunk of s) {
        this.said += dec.decode(chunk);
        const addr = /listening on http:\/\/(\S+)/.exec(this.said.slice(from))?.[1];
        if (addr && found) found(addr);
      }
    };
    void drain(proc.stderr as ReadableStream<Uint8Array>);
    this.addr = await new Promise<string>((ok, fail) => {
      void drain(proc.stdout as ReadableStream<Uint8Array>, ok).then(() => fail(new Error(`nvs serve ended:\n${this.said}`)));
    });
  }

  /** Ends the server and waits until its process has exited. */
  async stop() {
    this.proc?.kill();
    await this.proc?.exited;
    this.proc = undefined;
  }
}

let browser: Browser;
let chat: Served;
let poll: Served;

beforeAll(async () => {
  if (!EXE) return;
  if (!NVS) throw new Error("no built nvs: run `cargo build`");
  chat = new Served("chat");
  poll = new Served("poll");
  await Promise.all([chat.start(), poll.start()]);
  browser = await Browser.launch("realtime-browser");
}, 300_000);

afterAll(async () => {
  await browser?.close();
  await chat?.stop();
  await poll?.stop();
  for (let tries = 1; ; tries++) {
    try {
      rmSync(DIR, { recursive: true, force: true });
      return;
    } catch (err) {
      if (tries >= 40) throw err;
      await Bun.sleep(250);
    }
  }
}, 60_000);

/** One message a page logged that is an error: a console error, an exception, or a blocked load. */
type Logged = { source: string; text: string; url?: string };

/** One tab in a browser context of its own. */
class Page {
  readonly errors: Logged[] = [];

  private constructor(
    private context: string,
    readonly session: string,
    private served: Served,
  ) {}

  static async open(served: Served): Promise<Page> {
    const { browserContextId } = await browser.send("Target.createBrowserContext");
    const { targetId } = await browser.send("Target.createTarget", { url: "about:blank", browserContextId });
    const { sessionId } = await browser.send("Target.attachToTarget", { targetId, flatten: true });
    const page = new Page(browserContextId, sessionId, served);
    browser.on((method, params, from) => {
      if (from !== sessionId) return;
      if (method === "Log.entryAdded" && params.entry.level === "error") {
        page.errors.push({ source: params.entry.source, text: params.entry.text, url: params.entry.url });
      } else if (method === "Runtime.consoleAPICalled" && params.type === "error") {
        page.errors.push({ source: "console", text: params.args.map((a: any) => a.value ?? a.description).join(" ") });
      } else if (method === "Runtime.exceptionThrown") {
        page.errors.push({ source: "exception", text: params.exceptionDetails.exception?.description ?? params.exceptionDetails.text });
      }
    });
    await page.send("Page.enable");
    await page.send("Log.enable");
    await page.send("Runtime.enable");
    return page;
  }

  send(method: string, params: object = {}) {
    return browser.send(method, params, this.session);
  }

  /** The value of `expression` in the page, or `undefined` while a navigation replaces it. */
  async eval(expression: string): Promise<any> {
    try {
      return await browser.eval(expression, this.session);
    } catch (err) {
      if (/context|navigat|Cannot find/i.test(String(err))) return undefined;
      throw err;
    }
  }

  /** Waits until `expression` is true. A step that does not end names the status line it stopped at. */
  async until(expression: string) {
    const stuck = Date.now() + STEP_MS;
    while (!(await this.eval(expression))) {
      if (Date.now() > stuck) {
        const status = await this.eval(`document.getElementById("status")?.textContent`);
        throw new Error(`the page never made \`${expression}\` true. The status line says: ${status}\nthe server: ${this.served.said}`);
      }
      await Bun.sleep(25);
    }
  }

  /** Opens `path` and waits until it has loaded. */
  async go(path: string) {
    await this.send("Page.navigate", { url: this.served.origin + path });
    await this.until(loaded(path));
  }

  /** Fills the form `selector` with `fields` and sends it, with the button `button` when one is named. */
  async submit(selector: string, fields: Record<string, string>, button?: string) {
    const fill = Object.entries(fields)
      .map(([name, value]) => `f.elements[${JSON.stringify(name)}].value = ${JSON.stringify(value)};`)
      .join("\n");
    const by = button ? `f.querySelector(${JSON.stringify(button)})` : "undefined";
    const sent = await this.eval(`(() => {
  const f = document.querySelector(${JSON.stringify(selector)});
  if (!f) return false;
  ${fill}
  f.requestSubmit(${by});
  return true;
})()`);
    if (!sent) throw new Error(`the page has no ${selector}`);
  }

  /** The errors this page logged, less the ones `expected` names. */
  unexpected(expected: (e: Logged) => boolean = () => false): Logged[] {
    // A browser asks every site for `/favicon.ico`, and neither app has one.
    return this.errors.filter((e) => !e.url?.endsWith("/favicon.ico") && !/favicon\.ico/.test(e.text) && !expected(e));
  }

  close() {
    return browser.send("Target.disposeBrowserContext", { browserContextId: this.context });
  }
}

const loaded = (path: string) => `location.pathname === ${JSON.stringify(path)} && document.readyState === "complete"`;
const status = (words: string) => `(document.getElementById("status")?.textContent ?? "").includes(${JSON.stringify(words)})`;
/** The text of each chat line on the page, after the time and the name: notices are left out. */
const LINES = `[...document.querySelectorAll("#lines li:not(.notice)")].map((li) => li.textContent.replace(/^.*?: /, ""))`;
const NOTICES = `[...document.querySelectorAll("#lines li.notice")].map((li) => li.textContent)`;
const linesWith = (text: string) => `${LINES}.filter((t) => t === ${JSON.stringify(text)}).length`;

/** Opens the chat's home page, joins `room` as `name`, and waits until the WebSocket is open. */
async function enter(name: string, room: string): Promise<Page> {
  const page = await Page.open(chat);
  await page.go("/");
  await page.submit('form[action="/join"]', { name, room });
  await page.until(`${loaded(`/rooms/${room}`)} && ${status("Connected.")}`);
  return page;
}

/** Sends one chat line from the page's own form. */
function say(page: Page, text: string) {
  return page.submit("#say", { text });
}

describe.skipIf(!EXE)("the chat and the poll in Chromium", () => {
  test("a line in one chat page shows once in the other, as text", async () => {
    const ana = await enter("Ana", "lobby");
    const ben = await enter("Ben", "lobby");
    await say(ana, "Hello from Ana");
    await ben.until(`${linesWith("Hello from Ana")} === 1`);
    await say(ana, "<b>bold</b>");
    await ben.until(`${linesWith("<b>bold</b>")} === 1`);
    // The only `<b>` in the line is the name. The text of the line made no element.
    const bold = await ben.eval(`[...document.querySelectorAll("#lines li:not(.notice)")].at(-1).querySelectorAll("b").length`);
    expect(bold).toBe(1);
    await ana.until(`${linesWith("Hello from Ana")} === 1 && ${linesWith("<b>bold</b>")} === 1`);
    expect(await ben.eval(`${linesWith("Hello from Ana")}`)).toBe(1);
    expect(ana.unexpected()).toEqual([]);
    expect(ben.unexpected()).toEqual([]);
    await ana.close();
    await ben.close();
  }, 120_000);

  test("the policy blocks an inline script, and the page logs it as an error", async () => {
    // This proves the other tests' checks: a violation is an error those checks would see.
    const page = await Page.open(chat);
    await page.go("/");
    await page.eval(`(() => { const s = document.createElement("script"); s.textContent = "window.ran = true"; document.body.append(s); })()`);
    const stuck = Date.now() + STEP_MS;
    while (!page.unexpected().some((e) => /Content Security Policy/.test(e.text)) && Date.now() < stuck) await Bun.sleep(25);
    expect(await page.eval("window.ran === true")).toBe(false);
    expect(page.unexpected().some((e) => /Content Security Policy/.test(e.text))).toBe(true);
    await page.close();
  }, 120_000);

  test("a chat page that sends too fast shows the notice", async () => {
    const page = await enter("Cleo", "help");
    // The connection allows 5 lines in 10 seconds, so the sixth one is not sent to the room.
    for (let i = 1; i <= 6; i++) await say(page, `line ${i}`);
    await page.until(`${NOTICES}.some((t) => t.includes("You are sending too fast."))`);
    await page.until(`${linesWith("line 5")} === 1`);
    expect(await page.eval(`${linesWith("line 6")}`)).toBe(0);
    expect(page.unexpected()).toEqual([]);
    await page.close();
  }, 120_000);

  test("a chat page connects again after a restart and shows each line once", async () => {
    const page = await enter("Dana", "random");
    await say(page, "one");
    await page.until(`${linesWith("one")} === 1`);
    await say(page, "two");
    await page.until(`${linesWith("two")} === 1`);
    const addr = chat.addr;
    await chat.stop();
    await page.until(status("The connection is lost."));
    await chat.start(addr);
    await page.until(status("Connected."));
    // The server sends the history first on each connection, so once `three` shows, the history has arrived.
    await say(page, "three");
    await page.until(`${linesWith("three")} === 1`);
    expect(await page.eval(LINES)).toEqual(["one", "two", "three"]);
    // While the server is stopped, each try to connect fails and Chromium logs it.
    expect(page.unexpected((e) => e.source === "network" && /WebSocket connection to .* failed/.test(e.text))).toEqual([]);
    await page.close();
  }, 120_000);

  test("a vote in one poll page moves the bar in the other, and a second vote moves it", async () => {
    const eve = await Page.open(poll);
    await eve.go("/");
    await eve.submit('form[action="/polls"]', { question: "Where do we eat?", answers: "Pizza\nSoup\n" });
    await eve.until(`/^\\/polls\\/\\d+$/.test(location.pathname) && document.readyState === "complete"`);
    const path: string = await eve.eval("location.pathname");
    const finn = await Page.open(poll);
    await finn.go(path);
    await eve.until(status("The results update live."));
    await finn.until(status("The results update live."));
    const [pizza, soup]: string[] = await eve.eval(`[...document.querySelectorAll("li[data-answer]")].map((li) => li.dataset.answer)`);
    const votes = (id: string) => `document.querySelector('li[data-answer="${id}"] .votes').textContent`;
    const bar = (id: string) => `document.querySelector('li[data-answer="${id}"] progress').value`;
    const total = `document.getElementById("total").textContent`;

    await eve.submit("#vote", {}, `button[value="${pizza}"]`);
    await finn.until(`${total} === "1" && ${votes(pizza!)} === "1" && ${bar(pizza!)} === 1`);
    await eve.until(`document.querySelector("button.chosen")?.value === ${JSON.stringify(pizza)}`);

    // The same browser votes again. Its vote moves to the other answer, and the total stays 1.
    await eve.submit("#vote", {}, `button[value="${soup}"]`);
    await finn.until(`${votes(soup!)} === "1" && ${votes(pizza!)} === "0" && ${bar(pizza!)} === 0`);
    expect(await finn.eval(total)).toBe("1");
    await eve.until(`document.querySelector("button.chosen")?.value === ${JSON.stringify(soup)} && ${total} === "1"`);
    expect(eve.unexpected()).toEqual([]);
    expect(finn.unexpected()).toEqual([]);
    await eve.close();
    await finn.close();
  }, 120_000);
});
