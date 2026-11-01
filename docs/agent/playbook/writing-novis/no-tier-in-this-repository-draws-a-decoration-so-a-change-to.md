- **No tier in this repository draws a decoration, so a change to how one looks is unverified until
  someone opens an editor.** `test:headless` runs the reveal state machine and the position
  conversion in plain Node, and the extension-host tier does not render; `DEVELOPING.md`'s *No pixel
  tier* says why neither will. A CSS change here is a proposal, not a landed fix — say so, and get
  it looked at before writing it down as one. See `editors/vscode/DEVELOPING.md`. `until: gone editors/vscode/DEVELOPING.md:No pixel tier`
