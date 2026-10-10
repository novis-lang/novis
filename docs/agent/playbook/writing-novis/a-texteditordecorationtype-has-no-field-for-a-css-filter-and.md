- **A `TextEditorDecorationType` has no field for a CSS filter, and `textDecoration` is the one that
  reaches the rendered rule verbatim.** A blur is written `textDecoration: "none; filter: blur(Npx);
  clip-path: inset(0)"` — the declaration is closed with `none` and the rest follows it. The clip is not optional: a blur bleeds about its radius past the decorated
  range, and `overflow` cannot cut it back because a decoration is a non-replaced inline box. A colour belongs in a `ThemeColor` field
  (`rule:ide/novis-ships-names-not-colours`).
  [until: gone editors/vscode/src/redactions.ts:filter: blur]
