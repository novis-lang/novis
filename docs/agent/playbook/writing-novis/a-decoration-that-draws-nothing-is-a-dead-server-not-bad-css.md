- **A decoration that draws *nothing* is a dead server, not bad CSS — check which one before
  touching the stylesheet.** In DevTools the difference is one glance: a range that was decorated
  and styled badly carries a decoration class on its token span, and a range nobody answered for is
  a bare `span.mtk<N>` with only its theme colour. `nvs/redactions` returning nothing looks exactly
  like a filter that failed to apply, and `Novis: Restart Language Server` fixes it. The
  status item and the `nvs.lsp.trace.server` channel answer it without guessing.
  [until: gone editors/vscode/src/redactions.ts:filter: blur]
