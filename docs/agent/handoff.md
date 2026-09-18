# Handoff

## State

Milestone `dossier`, goal `config-directives-1-3` — 16 features, **3 landed**, 13 open. Each of
`directive:app`, `directive:cache.local` and `directive:cache.process` now has its Rust test, its
one example with a blessed `.out`, its attack and its `about.md`;
`python tools/dossier.py --group 'config:directives'` reports all three complete and is the only
scoreboard worth reading. A directive owes **one** example and no bench —
`tools/data/dossier-policy.toml`'s table is what each kind owes, and the goal's "three examples"
paragraph is the generic blurb, not this kind's row.

The repository's own `nvs.toml` grew three blocks, on purpose and as fixtures: an `[[app]]` block
keyed on the app example's entry file so that example can print a value a narrower block put in
force, and `[cache.local]`/`[cache.process]` written at exactly the figures `nvs_stdlib::cache`
ships, so the two cache pages have a value to show while the tree behaves as it did. Nothing is
blocked.

## Next group

**Goal `config-directives-1-3`, items 4–6 — one file set:** `crates/nvs-config/tests/directives.rs`
(every directive census test lives here; `keys_listed` is the helper that lists a block's accepted
keys), `nvs.toml`, `docs/examples/config/<key-with-dots-as-dashes>/` and
`tests/hostile/config/<same>/`. Read `docs/examples/README.md` and `tests/hostile/README.md` once at
the start — they own what an example and an attack *are*, the pack prints neither, and there is no
`[context]` field that would.

- [ ] **`directive:cache.shared`** — the coherent tier, `System` and the block's one `Boot` row,
      `rule:core-api/two-cache-tiers`. `crates/nvs-config/src/directive.rs:189`. The example has two
      honest shapes: read `cache.shared.url` back and show the refused `set`, which needs nothing
      running; or open the store, which needs the `cache.shared` capability in a new `[[app]]` block
      (copy `examples/cache.nvs`'s) *and* the compose Redis on `127.0.0.1:16379` to bless against.
      Prefer the first unless Redis is already up.
- [ ] **`directive:capabilities`** — the one row that is `RuntimeTighten`: a script may drop a right
      it holds and never add one, `rule:config/three-changeability-classes`,
      `rule:security/isolate-shares-nothing`. `crates/nvs-config/src/directive.rs:118`. A program
      under the repository's `root = "."` block already holds `capabilities.script.spawn`, so
      `Core\Config::set` on it answers `true` narrowing and `false` widening — the example writes
      itself, and the attack is every widening spelling.
- [ ] **`directive:control.socket`** — `System` and `Boot`, and the row carries no comment of its
      own. `crates/nvs-config/src/directive.rs:202`; `nvs_config::control`'s module doc is where the
      reasoning is.

## Backlog

- `directive:cache.shared` and the 12 other items of this goal — `docs/agent/loop-goal.md` § *The
  item list* is the order.
- `about.md` is written for every feature here but counted by nothing; goal `the-description-is-owed`
  is where the check switches on (`tools/data/dossier-policy.toml` has `about = false` for a
  directive today).
- `[context] modules` was widened this session to `crates/nvs-config/src/*.rs` plus
  `nvs-stdlib`'s `config.rs` and `cache.rs`; the tree READMEs above still have no field.
