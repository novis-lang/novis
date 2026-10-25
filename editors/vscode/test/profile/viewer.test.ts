// *View Profile*'s page: the bundled speedscope viewer, the policy it runs under, and the profile
// it is handed (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
//
// The page is built from speedscope's own `index.html`, as the pinned devDependency ships it, and
// a fixture speedscope file. The inline script is run in a `vm` context with a stand-in `window`,
// so the test sees the bytes the viewer would be given. What `src/profile.ts` passes the webview is
// asserted as text, the way the contributions suite asserts a registered command.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { runInNewContext } from "node:vm";

import { VIEWER, page, policy } from "../../src/viewer";

const ROOT = resolve(__dirname, "..", "..", "..");
const RELEASE = join(ROOT, "node_modules", "speedscope", "dist", "release");
const INDEX = readFileSync(join(RELEASE, "index.html"), "utf8");
const FIXTURE = readFileSync(join(ROOT, "test", "profile", "fixture.speedscope.json"));
const SOURCE = "https://file+.vscode-resource.vscode-cdn.net";
const BASE = `${SOURCE}/ext/out/speedscope`;
const NONCE = "bm9uY2Utb2YtdGhlLXRlc3Q=";

// The panel's code with its prose removed, since a comment may name what the code does not do.
const PANEL = readFileSync(join(ROOT, "src", "profile.ts"), "utf8")
  .replace(/\/\*[\s\S]*?\*\//g, "")
  .replace(/^\s*\/\/.*$/gm, "");

function built(name = "profile.1234.speedscope.json", bytes: Uint8Array = FIXTURE): string {
  return page({ index: INDEX, base: BASE, source: SOURCE, nonce: NONCE, name, bytes });
}

/** The policy's directives, by name. */
function directives(html: string): Map<string, string[]> {
  const meta = /<meta http-equiv="Content-Security-Policy" content="([^"]*)">/.exec(html);
  assert.ok(meta, "the page has no content security policy");
  const content = (meta[1] ?? "").replace(/&#39;|&quot;/g, "'");
  return new Map(content.split(";").map((part) => {
    const [name = "", ...values] = part.trim().split(/\s+/);
    return [name, values];
  }));
}

/** The inline script, run against a stand-in `window`. Returns what the viewer was given. */
function loaded(html: string): { hash: string; name: string; text: string } {
  const script = /<script nonce="[^"]*">([\s\S]*?)<\/script>/.exec(html);
  assert.ok(script, "the page has no inline script");
  const handed = { hash: "", name: "", text: "" };
  const timers: (() => void)[] = [];
  const window: Record<string, unknown> = {};
  runInNewContext(script[1] ?? "", {
    window,
    history: { replaceState: (_s: unknown, _t: string, url: string) => { handed.hash = url; } },
    setTimeout: (run: () => void) => timers.push(run),
    Object,
  });
  window["speedscope"] = {
    loadFileFromBase64: (name: string, data: string) => {
      handed.name = name;
      handed.text = Buffer.from(data, "base64").toString("utf8");
    },
  };
  for (const run of timers) {
    run();
  }
  return handed;
}

describe("the bundled viewer", () => {
  it("is speedscope's release, with its index page and its licence", () => {
    assert.match(INDEX, /<script src="speedscope-[A-Z0-9]+\.js"><\/script>/);
    assert.ok(readFileSync(join(ROOT, "node_modules", "speedscope", "LICENSE"), "utf8").includes("MIT"));
  });

  it("is copied into the folder the panel serves, before the client is bundled", () => {
    const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as {
      scripts: Record<string, string>;
      devDependencies: Record<string, string>;
    };
    assert.match(manifest.devDependencies["speedscope"] ?? "", /^\^/);
    assert.match(manifest.scripts["bundle"] ?? "", /^node scripts\/viewer\.mjs && esbuild /);
    assert.deepEqual(VIEWER, ["out", "speedscope"]);
    const copy = readFileSync(join(ROOT, "scripts", "viewer.mjs"), "utf8");
    assert.match(copy, /join\(ROOT, "out", "speedscope"\)/);
    assert.match(copy, /"LICENSE"/);
  });
});

describe("the View Profile page", () => {
  it("allows nothing but the viewer's folder and its own nonce", () => {
    const policy = directives(built());
    assert.deepEqual(policy.get("default-src"), ["'none'"]);
    assert.deepEqual(policy.get("script-src"), [SOURCE, `'nonce-${NONCE}'`, "'wasm-unsafe-eval'"]);
    assert.deepEqual(policy.get("style-src"), [SOURCE, "'unsafe-inline'"]);
    assert.deepEqual(policy.get("font-src"), [SOURCE]);
    assert.deepEqual(policy.get("img-src"), [SOURCE]);
    for (const closed of ["connect-src", "form-action", "frame-src", "object-src"]) {
      assert.deepEqual(policy.get(closed), ["'none'"], `${closed} is open`);
    }
    for (const [name, values] of policy) {
      for (const value of values) {
        assert.ok(!/^(\*|https?:|data:|blob:|file:)$/.test(value) && value !== "'unsafe-eval'",
                  `${name} allows ${value}`);
      }
    }
  });

  it("puts the policy, the base and its script before the viewer's script", () => {
    const html = built();
    const at = (text: string): number => html.indexOf(text);
    assert.ok(at("Content-Security-Policy") > at("<head>"));
    assert.ok(at(`<base href="${BASE}/">`) > 0, "the viewer's links do not resolve against its folder");
    assert.ok(at(`<script nonce="${NONCE}">`) < at('<script src="speedscope-'),
              "the viewer reads the hash before the page sets it");
  });

  it("hands the viewer the profile's bytes and name", () => {
    const handed = loaded(built());
    assert.equal(handed.hash, "#localProfilePath=profile");
    assert.equal(handed.name, "profile.1234.speedscope.json");
    assert.equal(handed.text, FIXTURE.toString("utf8"));
    const profile = JSON.parse(handed.text) as { $schema: string };
    assert.equal(profile.$schema, "https://www.speedscope.app/file-format-schema.json");
  });

  it("does not let a file name close the script", () => {
    const html = built("</script><script>alert(1)</script>.speedscope.json");
    assert.equal(html.match(/<script/g)?.length, 2, "a file name opened a script element");
    assert.equal(loaded(html).name, "</script><script>alert(1)</script>.speedscope.json");
  });

  it("gives the policy a nonce the panel draws fresh for each page", () => {
    assert.notEqual(policy(SOURCE, "a"), policy(SOURCE, "b"));
    assert.match(PANEL, /nonce: randomBytes\(16\)/);
  });
});

describe("the View Profile panel", () => {
  it("serves the viewer's folder alone, and runs no command link", () => {
    assert.match(PANEL, /localResourceRoots: \[folder\]/);
    assert.match(PANEL, /const folder = Uri\.joinPath\(context\.extensionUri, \.\.\.VIEWER\)/);
    assert.match(PANEL, /enableCommandUris: false/);
  });

  it("reads the file through the workspace, so it works in a remote window", () => {
    assert.match(PANEL, /workspace\.fs\.readFile\(file\)/);
    assert.doesNotMatch(PANEL, /from "node:fs"/);
  });
});
