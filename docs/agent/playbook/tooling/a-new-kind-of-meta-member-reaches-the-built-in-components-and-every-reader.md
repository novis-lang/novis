- **A new kind of row in `nvs meta --json`'s `members` reaches the built-in components and every
  reader of the document.** The components' Novis classes are read by the same walk as a program's, so
  listing promoted properties added 45 `Novis\...::$name` rows. `nvs agent index`, the website
  renderer, the proof roster, `every_member_carries_a_doc_key` and the site's questions each broke
  on them in turn. Before a change adds a member kind, grep `tools/nv` and `crates/nvs-cli` for
  `["members"]`, `.members` and `"signature"`, and give each reader its case. See `promoted_json` in
  `crates/nvs-cli/src/meta.rs`. `until: gone crates/nvs-cli/src/meta.rs:promoted_json`
