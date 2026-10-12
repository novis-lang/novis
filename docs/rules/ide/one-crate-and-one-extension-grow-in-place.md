`crates/nvs-lsp` and `editors/vscode` are one crate and one package, and every editor feature extends
them in place. Nothing stands up a second "real" implementation next to a throwaway first one, and
nothing is built to be discarded.

Two implementations of the same client-server pair drift and duplicate work — the identical reasoning
`rule:ide/one-server-two-thin-clients` applies to formatting and language smarts, applied to the editor
packages themselves. The cost is the ordinary one of any early-shipped surface: the server and the
extension are kept building and passing their suites through work that does not depend on them.

**Declined:**
- **A throwaway first server and extension, discarded when the deep half lands.** It is two
  implementations of one client-server pair for a stretch, and they drift
  (`rule:ide/one-server-two-thin-clients`).
