# Handoff

## State

Goal `core-cache-and-5-more` (14 items). Six are on disk and complete. The three tier members
(`Core\Cache::local`, `::process`, `::shared`) landed earlier; this session finished the three plain
store operations, `Core\Cache\Store::put`, `::get` and `::forget`. Each of the six carries `about.md`,
three examples with blessed `.out`, one attack, and a Rust test marked `// covers:`; the store
operations carry a bench and a ledger row as well, and `Core\Cache::shared` stays a recorded `perf`
skip in `tools/data/dossier-policy.toml`.

The store operations' Rust proofs are `// covers:` markers on three tests that already pinned exactly
those claims — a rewrite costing what the first write cost, a read that leaves an expired entry where
it was, a forget that credits the cap — which `docs/agent/conventions.md` § *Feature proofs* names as
the whole edit for an instance member. `::get` and `::forget` also gained a `.nvst` case, since
neither had one; `::put` was already credited by
`tests/conformance/core/cache-an-entry-is-a-copy-that-shares-nothing-with-the-request.nvst`.

Nothing is blocked. `python tools/dossier.py --id` reports all three complete.

## Next group

**The sealed half of the cache** — one file set: `crates/nvs-stdlib/src/cache.rs`,
`docs/examples/core/Cache-Store/<member>/`, `docs/examples/core/Cache-SecretEntry/of/`,
`tests/hostile/core/Cache-{Store,SecretEntry}/<member>/`,
`benches/members/core/Cache-{Store,SecretEntry}/<member>.nvs`. Each is one slice: description, three
examples, one attack, one bench, one Rust test carrying its `covers:` marker.
`rule:testing/feature-proofs` is what each owes. A sealed entry takes a `Core\Keyring\Key` array, so
`crates/nvs-stdlib/src/keyring.rs` is now in the goal's `[context] modules`.

- [ ] **`Core\Cache\Store::putSecret`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cache.rs:370`
- [ ] **`Core\Cache\Store::getSecret`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cache.rs:395`
- [ ] **`Core\Cache\SecretEntry::of`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cache.rs:703`

## Backlog

- `Core\Cap::has` — item 10, `crates/nvs-stdlib/src/cap.rs:81`.
- `Core\Task\Channel::send` and `::close` — items 11 and 12, `crates/nvs-stdlib/src/channel.rs:121`.
- `Core\Cldr::pluralCategory` and `::ordinalCategory` — items 13 and 14,
  `crates/nvs-stdlib/src/cldr.rs:1605`.
- `docs/examples/core/Cache-Store/` has no class-level `about.md`; the dossier does not ask for one,
  so this is only worth doing if the website wants a page above the members.
