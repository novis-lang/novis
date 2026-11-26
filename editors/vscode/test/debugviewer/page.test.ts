// *Open Debug Viewer*'s panel: its page, the policy it runs under, and the checks on the three
// messages it passes (`rule:ide/the-debug-viewer-opens-from-one-button`,
// `rule:testing/the-viewer-is-framed-only-by-the-editor-under-a-token-path`).
//
// The page's script is run in a `vm` context with a stand-in frame and a stand-in editor API, so the
// test sees which messages reach the extension and which reach the frame. What `src/debugviewer.ts`
// does with an `open` is asserted as text, the way the contributions suite asserts a command.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { runInNewContext } from "node:vm";

import { frameAddress, frameOrigin, goable, opened, page, policy, themeOf, within } from "../../src/debugpage";

const ROOT = resolve(__dirname, "..", "..", "..");
const EMBED = "http://127.0.0.1:8137/embed/0123456789abcdef0123456789abcdef/";
const ORIGIN = "http://127.0.0.1:8137";
const NONCE = "bm9uY2Utb2YtdGhlLXRlc3Q=";

// The panel's code with its prose removed, since a comment may name what the code does not do.
const PANEL = readFileSync(join(ROOT, "src", "debugviewer.ts"), "utf8")
  .replace(/\/\*[\s\S]*?\*\//g, "")
  .replace(/^\s*\/\/.*$/gm, "");

function built(path = "/"): string {
  return page({ embed: EMBED, path, theme: "dark", nonce: NONCE });
}

/** The policy's directives, by name. */
function directives(html: string): Map<string, string[]> {
  const meta = /<meta http-equiv="Content-Security-Policy" content="([^"]*)">/.exec(html);
  assert.ok(meta, "the page has no content security policy");
  return new Map((meta[1] ?? "").split(";").map((part) => {
    const [name = "", ...values] = part.trim().split(/\s+/);
    return [name, values];
  }));
}

interface Sent {
  message: unknown;
  to?: string;
}

/** Runs the page's script with a stand-in frame. Gives what it sent each way and a way to post. */
function running(html: string) {
  const code = /<script nonce="[^"]*">([\s\S]*?)<\/script>/.exec(html)?.[1];
  assert.ok(code, "the page has no script");
  // A value made inside the `vm` context has that context's prototypes, so each is copied out.
  const copy = (value: unknown): unknown => JSON.parse(JSON.stringify(value)) as unknown;
  const toExtension: unknown[] = [];
  const toFrame: Sent[] = [];
  const frameWindow = {
    postMessage: (message: unknown, to: string) => toFrame.push({ message: copy(message), to }),
  };
  const listeners: Record<string, ((event: unknown) => void)[]> = {};
  const frameListeners: Record<string, (() => void)[]> = {};
  const frame = {
    contentWindow: frameWindow,
    addEventListener: (name: string, fn: () => void) => (frameListeners[name] ??= []).push(fn),
  };
  const host = {};
  runInNewContext(code, {
    acquireVsCodeApi: () => ({ postMessage: (message: unknown) => toExtension.push(copy(message)) }),
    document: { getElementById: (id: string) => (id === "viewer" ? frame : null) },
    addEventListener: (name: string, fn: (event: unknown) => void) => (listeners[name] ??= []).push(fn),
  });
  const post = (data: unknown, source: unknown, origin: string) =>
    (listeners.message ?? []).forEach((fn) => fn({ data, source, origin }));
  return {
    toExtension,
    toFrame,
    load: () => (frameListeners.load ?? []).forEach((fn) => fn()),
    fromFrame: (data: unknown, origin = ORIGIN) => post(data, frameWindow, origin),
    fromExtension: (data: unknown) => post(data, host, "vscode-webview://panel"),
  };
}

describe("the debug viewer panel's page", () => {
  it("admits the frame's one origin and one script, and nothing may connect", () => {
    const policy = directives(built());
    assert.deepEqual(policy.get("default-src"), ["'none'"]);
    assert.deepEqual(policy.get("frame-src"), [ORIGIN]);
    assert.deepEqual(policy.get("script-src"), [`'nonce-${NONCE}'`]);
    assert.deepEqual(policy.get("connect-src"), ["'none'"]);
    assert.deepEqual(policy.get("form-action"), ["'none'"]);
    assert.equal((built().match(/<iframe /g) ?? []).length, 1, "one frame");
    assert.equal((built().match(/<script /g) ?? []).length, 1, "one script");
  });

  it("frames the embed link at the path it was given", () => {
    assert.match(built(), new RegExp(`<iframe id="viewer" title="Novis Debug" src="${EMBED}"`));
    assert.equal(frameAddress(EMBED, "/requests/abc/def"), `${ORIGIN}/embed/0123456789abcdef0123456789abcdef/requests/abc/def`);
    assert.equal(frameAddress(EMBED, "//elsewhere.example.com/"), EMBED, "a path with a host is not followed");
  });

  it("takes only an http or https link under /embed/", () => {
    assert.equal(frameOrigin(EMBED), ORIGIN);
    assert.equal(frameOrigin("https://localhost:9000/embed/t/"), "https://localhost:9000");
    assert.equal(frameOrigin("http://127.0.0.1:8137/?token=abc"), undefined);
    assert.equal(frameOrigin("javascript:alert(1)//embed/"), undefined);
    assert.equal(frameOrigin("file:///embed/x/"), undefined);
    assert.throws(() => page({ embed: "http://127.0.0.1:8137/", path: "/", theme: "dark", nonce: NONCE }));
    assert.equal(policy(ORIGIN, NONCE).includes("frame-src http://127.0.0.1:8137"), true);
  });

  it("sends the theme to the frame once it loads, at the frame's origin", () => {
    const page = running(built());
    page.load();
    assert.deepEqual(page.toFrame, [{ message: { type: "theme", value: "dark" }, to: ORIGIN }]);
  });

  it("passes open from the frame to the extension, and nothing from another origin", () => {
    const page = running(built());
    page.fromFrame({ type: "open", file: "/w/app.nvs", line: 3, column: 2 });
    page.fromFrame({ type: "open", file: "/w/app.nvs", line: 3 }, "http://evil.example.com");
    page.fromFrame({ type: "go", path: "/routes" });
    assert.deepEqual(page.toExtension, [{ type: "open", file: "/w/app.nvs", line: 3, column: 2 }]);
  });

  it("passes theme and go from the extension to the frame alone", () => {
    const page = running(built());
    page.fromExtension({ type: "go", path: "/requests/a/b" });
    page.fromExtension({ type: "theme", value: "light" });
    assert.deepEqual(page.toFrame, [
      { message: { type: "go", path: "/requests/a/b" }, to: ORIGIN },
      { message: { type: "theme", value: "light" }, to: ORIGIN },
    ]);
    assert.deepEqual(page.toExtension, []);
  });
});

describe("the debug viewer panel's message checks", () => {
  const folder = resolve(ROOT, "test", "host", "fixture");

  it("opens a file only inside a workspace folder", () => {
    assert.equal(within(join(folder, "app.nvs"), [folder]), true);
    assert.equal(within(join(folder, "..", "..", "package.json"), [folder]), false);
    assert.equal(within(resolve(ROOT, "..", "elsewhere.nvs"), [folder]), false);
    assert.equal(within(folder, [folder]), false, "the folder itself is not a file in it");
    assert.equal(within(join(folder, "app.nvs"), []), false, "no folder, nothing opens");
  });

  it("checks the panel against the folders before it opens anything", () => {
    const check = PANEL.indexOf("within(file, roots)");
    const show = PANEL.indexOf("showTextDocument(Uri.file(file)");
    assert.ok(check > 0 && show > check, "the workspace check comes before the file opens");
    assert.match(PANEL, /file = await realpath\(site\.file\)/);
  });

  it("reads an open message only with an absolute file and lines counted from 1", () => {
    const file = join(folder, "app.nvs");
    assert.deepEqual(opened({ type: "open", file, line: 4 }), { file, line: 4 });
    assert.deepEqual(opened({ type: "open", file, line: 4, column: 7 }), { file, line: 4, column: 7 });
    assert.equal(opened({ type: "open", file: "app.nvs", line: 4 }), undefined, "a relative path");
    assert.equal(opened({ type: "open", file, line: 0 }), undefined);
    assert.equal(opened({ type: "open", file, line: 1.5 }), undefined);
    assert.equal(opened({ type: "open", file, line: 2, column: -1 }), undefined);
    assert.equal(opened({ type: "run", file, line: 2 }), undefined);
    assert.equal(opened("open"), undefined);
  });

  it("checks a go path before it is sent", () => {
    assert.equal(goable("/requests/abc/def?tab=queries"), true);
    assert.equal(goable("/"), true);
    assert.equal(goable("//example.com/"), false);
    assert.equal(goable("https://example.com/"), false);
    assert.equal(goable("/a\\b"), false);
    assert.equal(goable("/a b"), false);
    assert.equal(goable(`/${"a".repeat(2048)}`), false);
    assert.equal(goable(3), false);
    assert.match(PANEL, /goable\(path\) \? path : undefined/);
  });

  it("gives the viewer the editor's light or dark theme", () => {
    assert.equal(themeOf(1), "light");
    assert.equal(themeOf(2), "dark");
    assert.equal(themeOf(3), "dark");
    assert.equal(themeOf(4), "light");
  });
});
