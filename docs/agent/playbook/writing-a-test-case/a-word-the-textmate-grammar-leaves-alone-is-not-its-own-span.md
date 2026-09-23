- **A word the TextMate grammar leaves alone is not its own span, so a test cannot fetch it by its
  text.** `vscode-textmate` emits one span per run of identically-scoped bytes, so an uncoloured name
  is swallowed into the plain run around it — ` DEFAULT = Currency` is one span — and
  `spans.filter((s) => s.text === "Currency")` finds the coloured occurrence alone. Assert a negative
  as an absence (`filter(…).length === 0`) or against the whole run the way `names.test.ts`'s
  `plain(" Base {")` does, never by expecting an uncoloured span to exist.
  [until: gone editors/vscode/test/grammar/tokenize.ts]
