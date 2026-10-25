// The page the *View Profile* webview shows: speedscope's own `index.html`, with a content security
// policy and the profile put into it.
//
// The viewer is speedscope's built release, bundled into `out/speedscope/` by `scripts/viewer.mjs`
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). Nothing here draws a profile.
//
// **The profile's bytes are in the page, and no URL reaches it.** speedscope loads a `profileURL`
// only from an `http:` or `https:` page, and a webview is neither. Its other door is the one its
// own CLI uses: with `#localProfilePath` in the hash it defines `window.speedscope.loadFileFromBase64`
// and then loads a `file:` script, which the policy below blocks. The script this page adds sets the
// hash first and calls that function once the viewer defines it. So the webview's only resource root
// is the viewer's folder, and the profile's folder is never served. Spends: the file and its base64
// copy, once per open panel, freed when the panel closes.
//
// Nothing here imports `vscode`, so the headless suite runs it in plain Node. `src/profile.ts` is
// the command and the panel.

/**
 * The folder `scripts/viewer.mjs` copies the viewer into, under the extension's own folder. It is
 * the webview's one resource root.
 */
export const VIEWER = ["out", "speedscope"];

/**
 * The content security policy. `source` is the webview's `cspSource`, which serves only the
 * resource roots above, and `nonce` admits the one inline script this page adds.
 *
 * The viewer's styles are inline `<style>` blocks it writes at run time, so styles allow
 * `'unsafe-inline'`. `'wasm-unsafe-eval'` compiles the WebAssembly embedded in the viewer's own
 * script, which demangles C++ and Rust names; it allows no JavaScript `eval`. Nothing may connect,
 * so a profile never leaves the machine.
 */
export function policy(source: string, nonce: string): string {
  return [
    "default-src 'none'",
    `script-src ${source} 'nonce-${nonce}' 'wasm-unsafe-eval'`,
    `style-src ${source} 'unsafe-inline'`,
    `font-src ${source}`,
    `img-src ${source}`,
    "connect-src 'none'",
    "form-action 'none'",
    "frame-src 'none'",
    "object-src 'none'",
  ].join("; ");
}

/** What the page needs to show one profile. */
export interface Page {
  /** speedscope's `index.html`, as the viewer's folder has it. */
  index: string;
  /** The viewer's folder as a webview URI, which every relative link in `index` resolves against. */
  base: string;
  /** The webview's `cspSource`. */
  source: string;
  nonce: string;
  /** The file's name, which speedscope shows as the profile's title. */
  name: string;
  bytes: Uint8Array;
}

/** `index` with the policy, a `<base>` and the loading script put first in its `<head>`. */
export function page(input: Page): string {
  const head = /<head>/i.exec(input.index);
  if (head === null) {
    throw new Error("the bundled speedscope index.html has no <head>");
  }
  const added = [
    `<meta http-equiv="Content-Security-Policy" content="${attribute(policy(input.source, input.nonce))}">`,
    `<base href="${attribute(input.base.endsWith("/") ? input.base : `${input.base}/`)}">`,
    `<script nonce="${attribute(input.nonce)}">${loader(input.name, input.bytes)}</script>`,
  ].join("\n");
  const at = head.index + head[0].length;
  return `${input.index.slice(0, at)}\n${added}${input.index.slice(at)}`;
}

/**
 * The inline script. It runs before the viewer's own script, sets the hash the viewer reads at
 * start, and waits for the viewer to define `window.speedscope` before it hands the file over.
 */
function loader(name: string, bytes: Uint8Array): string {
  const file = scriptString(name);
  const data = scriptString(Buffer.from(bytes).toString("base64"));
  return [
    `history.replaceState(null, "", "#localProfilePath=profile");`,
    `Object.defineProperty(window, "speedscope", { configurable: true, set(api) {`,
    `  Object.defineProperty(window, "speedscope", { value: api, writable: true, configurable: true });`,
    `  setTimeout(() => api.loadFileFromBase64(${file}, ${data}));`,
    `} });`,
  ].join("\n");
}

/** `value` as a JavaScript string literal that cannot close the `<script>` element it sits in. */
function scriptString(value: string): string {
  return JSON.stringify(value).replace(/</g, "\\u003c");
}

function attribute(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}
