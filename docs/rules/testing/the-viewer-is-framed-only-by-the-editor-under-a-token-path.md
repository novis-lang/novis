The debug service also serves the viewer under `/embed/<token>/`, a path that carries the token in
place of a cookie, and only a response under that path may be framed, and then only by the editor.

**A cookie cannot work in a frame.** A page framed by an editor's webview is cross-site to its frame,
so the browser neither stores nor sends the viewer's `SameSite=Strict` cookie there
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). The token therefore travels
in the path. The service serves the page, the two bundled files and the whole API under
`/embed/<token>/` as it serves them at the root, compares the token in constant time, and sets and
reads no cookie under that prefix. A wrong or missing token is `401`, as at the root.

**Two `frame-ancestors` values.** A response under the prefix carries `frame-ancestors
vscode-webview:`. Every other response keeps `frame-ancestors 'none'`. Every other header the root
sends is sent under the prefix too, and an `Origin` there must be the listener's own or absent, as
at the root.

**The page finds its base path itself.** It reads the prefix from `location.pathname`, and the
router, every fetch and the event stream put it in front of each path, so one bundle serves both.
`Referrer-Policy: no-referrer` keeps the path out of every request the page makes, and the listener
writes no path to a log.

**Embedded, the page and its frame exchange three messages and nothing else.**

| Message | From | Checked by |
|---|---|---|
| `{type: "open", file, line, column}` | the page, in place of following an editor link, and only when its base path is the embed prefix | the extension host opens the file only when it exists under a workspace folder, after the viewer's path mappings, and runs nothing |
| `{type: "theme", value}` | the panel, when the editor's theme changes | the page takes it only from `window.parent` with a `vscode-webview:` origin |
| `{type: "go", path}` | the panel, when a command opens a path in an open panel | the page takes it only from `window.parent` with a `vscode-webview:` origin, and only for a path its router reads |

The panel takes a message only from the frame's origin.

**`nvs debug link` prints the link and starts nothing.** It reads `[debug] listen` and the token
file as the service does, prints the link with `?token=`, and says on standard error when no service
holds `debug/service.lock` or when `[debug] store` is off. `--json` prints `url`, `embed`,
`running` and `recording`. A missing token file is created exactly as the service creates it, so the
link is the one the next service accepts.

- **Spends:** the token is in the frame's URL, where the page's own script can read it. That script
  can already read everything the token opens. One prefix test per viewer request.

**Declined:**
- **A `SameSite=None` cookie.** It sends the token's cookie with a request from every site on the
  machine.
- **VS Code's Simple Browser.** It cannot pass a click back to the editor, and its frame is cross-site
  too.
- **Passing the API through the extension host.** It is a second transport to keep correct, beside
  the one the browser already uses.
