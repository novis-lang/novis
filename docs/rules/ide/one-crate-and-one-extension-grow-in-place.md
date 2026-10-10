`crates/nvs-lsp` and `editors/vscode` are one crate and one package across every milestone that touches
them. The first, minimal server and extension are the same files the deep half later extends in place;
nothing stands up a second "real" implementation next to a throwaway first one, and nothing is built
to be discarded.

Two implementations of the same client-server pair drift and duplicate work — the identical reasoning
`rule:ide/one-server-two-thin-clients` applies to formatting and language smarts, applied to the editor
packages themselves. The cost is the ordinary one of any early-shipped surface: the minimal server and
extension have to be kept building and passing through the milestones between, even while nothing in
those milestones depends on them.

**Declined:**
- **A throwaway first server and extension, discarded when the deep half lands.** It is two
  implementations of one client-server pair for a stretch, and they drift
  (`rule:ide/one-server-two-thin-clients`).
