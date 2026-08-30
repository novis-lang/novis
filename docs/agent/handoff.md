# Handoff

## State

**ADR 0078 §§ 1-2 is landed: the snapshot a request clones at start.**
`crates/nvs-config/src/snapshot.rs` owns it. `Snapshot::build(resolved, entry, files)` canonicalizes
the entry file, drops the `[[app]]` roster from the tree, and folds each matching block's
`[app.limits]`/`[app.capabilities]` over the global table with `resolve::merge_table` — a block at a
time, in `matching`'s order — then deserializes the result. `mode` and `origin` sit on the block
rather than in a sub-table, so they are read off directly: folding a string over the global `[mode]`
table would replace `default` and `ceiling` together. `Current` is the published `Arc`, `load()` is
what a request clones, and `publish` returns a `Reload`. 71 tests in the crate, 11 new in
`tests/snapshot.rs`.

**The snapshot does not call `app::layer`, and the handoff that planned it that way was wrong.**
Folding the *effective* block over the global tree gives the right values and the wrong origins: an
effective block folded from three files has one origin for all its keys, and a key a block overrode
has to name the file that block was written in. Folding block-by-block gives both, by the same
merge. `layer` now answers "what is the effective `[[app]]` block" for § 9's per-app `nvs config
dump` and has no other production caller yet — its own module doc says so.

**A changed `Boot` key is reported *and carried back*.** `Current::publish` is the only place that
holds both trees, so `carry_boot` writes each changed `Boot` row's running value into the incoming
snapshot before publishing and names the directive in `Reload::boot`. Reporting alone would have
left the new value sitting in the snapshot every later reader sees. A `Boot` row naming a block
(`server`) moves the whole subtree, and its origins move with it.

Three small things this cost, each already spent. `app::block_table`, `app::key_of` and a new
`app::block_origin` (factored out of `layer`) are `pub(crate)`; `directive::governs` is too, because
the dot-boundary test the origins map needs is the same one the registry's longest-prefix lookup
uses and a second one is a second chance to get the boundary wrong. `Snapshot` keeps its
`toml::Table` for `Resolved::table`'s reason — `publish` compares two trees key by key, which the
typed tree cannot do.

**The acceptance check still fails on `examples/config.nvs` — `Core\Config` has no `get`.** Still an
open item and not a regression: the member has never existed. It now needs exactly two things, in
this order — the value comparator below, then the members themselves in `nvs-stdlib`.

**One finding the next group needs:** there is **no size or duration parser anywhere in the tree**
(`Setting::Text("512M")` is compared as a string today). ADR 0064 § 5 says the registry parses a
`Core\Config::set` string "with the same parser the boot path uses", so § 3's boot-time ceiling
check and `set`'s runtime ceiling check are one implementation, and it does not exist yet. That is
why the group below starts with it rather than with § 3.

## Next group

**The value comparator, and the ceiling checks that are its only reason to exist.** One file set:
a new `crates/nvs-config/src/value.rs`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/app.rs`, `crates/nvs-config/src/resolve.rs`, `docs/adr/0064`.

- [ ] **A `Setting` compares as the quantity it spells.** ADR 0064 § 5 — one parser for sizes
      (`512M`), durations (`600s`), counts and the `false` that removes a ceiling, shared by the
      boot path and `Core\Config::set`. `Setting` is `crates/nvs-config/src/tree.rs:53`; the
      `[limits]` fields it types are `crates/nvs-config/src/tree.rs:156`.
- [ ] **`[app.limits.hard]` may only lower, and a block raising its own ceiling is refused at
      boot.** ADR 0104 § 3 — refused, not clamped, exactly as 0005 refuses a `Core\Config::set`.
      It needs the global `[limits.hard]`, which exists at resolve time, so it runs beside
      `crates/nvs-config/src/app.rs:66`'s `canonicalize` from
      `crates/nvs-config/src/resolve.rs:283`. Next free diagnostic in the band is `E0610`.
- [ ] **A block widening `[app.limits]` above the global `[limits.hard]` is the same refusal.**
      Same section, same call site — the widening half of § 3 is already allowed by the fold
      (`crates/nvs-config/src/snapshot.rs:102`), and this is the only thing bounding it.

## Backlog

- `Core\Config::get`/`set`/`restore`/`all` — ADR 0064 § 5, and the acceptance check's whole
  remainder. Needs a `Current` reachable from a request; nothing reads one yet.
- Nothing constructs a `Current`: `nvs-cli` and `nvs-host` still run on compiled-in defaults and
  say so at each site (`crates/nvs-config/src/lib.rs` module doc).
- `app::layer` has no production caller until § 9's `nvs config dump --origin` — its `Layered`
  is exercised only by `crates/nvs-config/tests/app.rs`.
- Item 18's `Core\Secret::reveal()` is not in the registry (`docs/plan/m6.md`).
- `Live::admit`'s same-class check is asked of the answer, not the argument
  (`crates/nvs-runtime/src/graph.rs` § *Known gaps*).
- `orient.py` reported it itself: `[context] modules` names `crates/nvs-host/src/budget.rs`, which
  matches no module — it moved or the glob is wrong.
