# Handoff

## State

**ADR 0104 §§ 1-2 is landed: an application is its entry file path.** `crates/nvs-config/src/app.rs`
owns all of it. `canonicalize` runs at resolve time beside § 7's secrets — `[[app]]` blocks
accumulate across the tree, so the roster only exists once the merge is done — resolving each
block's `root`/`entry` against the file that wrote it (0103 § 5), canonicalizing it and writing it
back in place. `matching` returns the covering blocks least-specific first, `layer` folds them into
one effective `App`. `E0609 E_BAD_APP_BLOCK` refuses a block naming both keys, neither, or a
canonical path another block already claimed; a key naming something that cannot be examined is
`E0605`, and the ADR's § 1 now states that refusal rather than leaving it open.

Three things this cost, each already spent. `Files` gained `canonical` — the trust check is wrong
for a web root nobody reads a byte of, but the *canonicalization* is shared, so `trust::canonical`
was factored out of `trust::check` and is the only one in the crate (the goal's standing decision).
`merge_table` now claims an array-of-tables entry **by index**, so `app.1.root` names the file that
appended that block and two files can each write a relative `root`. And `Resolved` keeps `table`
and `origins`: § 2 is 0103 § 3's later-wins in a different order, which is only true if one
function decides both, and `merge_table` folds `toml::Table`s the typed tree cannot be turned back
into.

60 tests in the crate (13 new in `tests/app.rs`, whose fake resolves `..` and symlinks so § 1's
claim can be asserted on Windows, where a real symlink needs privilege). `tests/resolve.rs`'s fake
now follows links too, which let a symlinked include cycle be pinned as an `E0606` naming the chain
— it is caught by name, not by the depth cap, and that test's doc comment had said the opposite.

**The acceptance check still fails on `examples/config.nvs` — `Core\Config` has no `get`.** That is
Stage 3's snapshot and Stage 4's members, unwritten, and it is an open item rather than a
regression: the member has never existed. It needs `nvs-stdlib`, a different file set from this
group's.

**One finding the snapshot slice needs:** `layer` is deliberately *not* called from `resolve()` —
it needs an entry file, and `nvs run <file>`'s entry is unknown at resolve time. Whatever builds
ADR 0078 § 1's snapshot is its only caller, and it folds `Layered::app`'s `limits`/`capabilities`
over the global ones while reading `mode` and `origin` off it directly. `Layered::blocks` is § 2's
`info: app blocks: …` line, already in order.

## Next group

**The per-app snapshot, on the fold this group just wrote.** One file set:
`crates/nvs-config/src/resolve.rs`, a new `crates/nvs-config/src/snapshot.rs`,
`crates/nvs-config/src/app.rs`, `docs/adr/0078`.

- [ ] **The immutable snapshot a request clones at start.** ADR 0078 § 1 — one `Arc`-shared value
      built from a `Resolved` plus an entry file, so a request that started before a swap reads the
      old one to completion. `layer` is the per-app half (`crates/nvs-config/src/app.rs:118`), and
      `Resolved` is `crates/nvs-config/src/resolve.rs:170`.
- [ ] **`[limits.hard]` bounds a block's grant.** ADR 0104 § 3 — a block may widen as well as
      narrow, but never past the global ceiling. The fold is `app::layer`
      (`crates/nvs-config/src/app.rs:118`) and the ceiling is `tree::Limits::hard`.
- [ ] **A case that a `Boot` key changed by a reload is reported and does not take effect.**
      ADR 0078 § 2 — `directive::Apply` already holds the field
      (`crates/nvs-config/src/directive.rs`), and nothing asks it anything yet.

## Backlog

- `Core\Config::get`/`set` are unwritten, which is what the acceptance check reports —
  `docs/plan/m6.md`, Stage 4.
- `nvs config check` and `nvs config dump --origin` — ADR 0103 § 9; `Resolved::origins` is now
  public, which is what `--origin` needed.
- `Core\Secret::reveal()` is not in the registry, though `Qual::Reveal` is decided — item 18.
- `Live::admit`'s same-class check asks the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- `Core\Script`'s members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `orient.py`'s `[context] adrs` has no 0104 §§ 1-2, which this item was about; it printed 0103's
  sections instead and the two sections cost a call to slice by hand.
