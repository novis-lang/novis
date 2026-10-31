import { existsSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

// A real Chromium for the browser tests: Playwright's pinned headless shell, downloaded into
// `node_modules`:
//
//     PLAYWRIGHT_BROWSERS_PATH=0 node node_modules/playwright-core/cli.js install --only-shell chromium
//
// Playwright's own driver neither launches nor connects under Bun on Windows, so this speaks the
// DevTools protocol over Bun's WebSocket. With no download a suite that uses it is skipped, unless
// `NV_BROWSER` is set, which is how CI's `browser` job makes a missing browser a failure.

/** The headless shell's executable under Playwright's in-tree browser directory, or `null`. */
function headlessShell(): string | null {
  const base = join(ROOT, "node_modules", "playwright-core", ".local-browsers");
  if (!existsSync(base)) return null;
  for (const d of readdirSync(base).filter((d) => d.startsWith("chromium_headless_shell-")).sort().reverse()) {
    for (const p of readdirSync(join(base, d)).filter((p) => p.startsWith("chrome-headless-shell-"))) {
      const exe = join(base, d, p, process.platform === "win32" ? "chrome-headless-shell.exe" : "chrome-headless-shell");
      if (existsSync(exe)) return exe;
    }
  }
  return null;
}

export const EXE = headlessShell();
if (!EXE && process.env.NV_BROWSER) throw new Error("NV_BROWSER is set and no Chromium headless shell is installed");

/** One Chromium process and a DevTools connection to it. */
export class Browser {
  private next = 0;
  private waiting = new Map<number, (m: { result?: unknown; error?: { message: string } }) => void>();
  private listeners: ((method: string, params: any, sessionId?: string) => void)[] = [];

  private constructor(
    private proc: ReturnType<typeof Bun.spawn>,
    private ws: WebSocket,
    private profile: string,
  ) {
    ws.onmessage = (e) => {
      const m = JSON.parse(String(e.data));
      if (m.id !== undefined) this.waiting.get(m.id)?.(m);
      else if (m.method) for (const l of this.listeners) l(m.method, m.params, m.sessionId);
    };
  }

  /** Calls `listener` with every event Chromium sends, from any session. */
  on(listener: (method: string, params: any, sessionId?: string) => void) {
    this.listeners.push(listener);
  }

  /** Starts Chromium with `flags`, its profile under `.agent-tmp/` named after `name`. */
  static async launch(name: string, flags: string[] = []): Promise<Browser> {
    const profile = join(ROOT, ".agent-tmp", `${name}-${process.pid}`);
    const args = [EXE!, "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run", ...flags, "about:blank"];
    if (process.platform === "linux") args.splice(1, 0, "--no-sandbox");
    const proc = Bun.spawn(args, { stdout: "ignore", stderr: "pipe" });
    const dec = new TextDecoder();
    const reader = (proc.stderr as ReadableStream<Uint8Array>).getReader();
    let seen = "";
    let url = "";
    while (!url) {
      const { done, value } = await reader.read();
      if (done) break;
      seen += dec.decode(value);
      url = /DevTools listening on (ws:\/\/\S+)/.exec(seen)?.[1] ?? "";
    }
    if (!url) throw new Error(`Chromium did not start:\n${seen}`);
    // The rest of stderr is read and thrown away, so a full pipe never stalls Chromium's logging.
    void (async () => {
      while (!(await reader.read()).done);
    })().catch(() => {});
    const ws = new WebSocket(url);
    await new Promise((ok, fail) => {
      ws.onopen = ok;
      ws.onerror = fail;
    });
    return new Browser(proc, ws, profile);
  }

  send(method: string, params: object = {}, sessionId?: string): Promise<any> {
    const id = ++this.next;
    return new Promise((ok, fail) => {
      this.waiting.set(id, (m) => {
        this.waiting.delete(id);
        if (m.error) fail(new Error(`${method}: ${m.error.message}`));
        else ok(m.result);
      });
      this.ws.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }

  async eval(expression: string, sessionId: string): Promise<any> {
    const r = await this.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true }, sessionId);
    if (r.exceptionDetails) throw new Error(`${expression}: ${JSON.stringify(r.exceptionDetails)}`);
    return r.result.value;
  }

  /** A new tab on `about:blank` and a session attached to it. */
  async tab(): Promise<{ targetId: string; sessionId: string }> {
    const { targetId } = await this.send("Target.createTarget", { url: "about:blank" });
    const { sessionId } = await this.send("Target.attachToTarget", { targetId, flatten: true });
    return { targetId, sessionId };
  }

  /** Waits until `expression` is true in the tab, polling. The wait is bounded by the test's own timeout,
   * so a loaded machine is slow here rather than reading a half-loaded page. */
  async until(expression: string, sessionId: string): Promise<void> {
    while (!(await this.eval(expression, sessionId))) await Bun.sleep(25);
  }

  /** Ends Chromium and deletes its profile. `Browser.close` lets Chromium end its child processes,
   *  which on Windows keep files in the profile open for a moment after the main process is killed,
   *  so the delete is retried until they have exited. */
  async close() {
    await Promise.race([this.send("Browser.close").catch(() => {}), Bun.sleep(2_000)]);
    this.ws.close();
    this.proc.kill();
    await this.proc.exited;
    for (let tries = 1; ; tries++) {
      try {
        rmSync(this.profile, { recursive: true, force: true });
        return;
      } catch (err) {
        if (tries >= 40) throw err;
        await Bun.sleep(250);
      }
    }
  }
}
