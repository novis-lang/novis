- **A page cannot show code whose output differs per run, such as a CSRF token, as a snippet.** A
  page may not carry an inline Novis fence, and a snippet's `.http.out` is compared byte for byte
  with no wildcard (`tools/nv/cmd/site.ts`), so a served form with a fresh token never matches. Show
  the one line as inline code in the prose, or make the snippet's request branch print only a fixed
  fact about the value, as `docs/examples/core/Csrf/token/01-a-token-on-a-first-visit.nvs` does.
  `until: gone tools/nv/cmd/site.ts:inline Novis fence`
