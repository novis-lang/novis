# Handoff

## State

Goal `core-cache-and-5-more` (14 items). Nine are on disk and complete: the three tier members, the
three plain store operations, and now the sealed half — `Core\Cache\Store::putSecret`, `::getSecret`
and `Core\Cache\SecretEntry::of`. Each of the nine carries `about.md`, three examples with blessed
`.out`, one attack, one bench with a ledger row, and a test from each side.
`Core\Cache::shared` stays a recorded `perf` skip in `tools/data/dossier-policy.toml`.

The sealed members' Rust proofs are `// covers:` markers on two tests that already pinned exactly
their claims — the binding a write seals under, and the sealed expiry a read obeys whatever the store
says. `Core\Cache\SecretEntry::of` needed a new one, `a_secret_entry_holds_the_secret_itself_and_the
_lifetime_as_a_count`, which asserts the two slots and that the secret is one more reference to the
same text rather than a copy. Its `.nvst` side was already credited by a plain call, since
`rule:testing/proof-attribution` only makes an *instance* member need the marker.

Nothing is blocked. `python tools/dossier.py --id` reports all three complete, and
`--run hostile`/`--run examples` are green over both groups.

## Next group

**The two plural-category members** — one file set: `crates/nvs-stdlib/src/cldr.rs`,
`docs/examples/core/Cldr/<member>/`, `tests/hostile/core/Cldr/<member>/`,
`benches/members/core/Cldr/<member>.nvs`. They are near-twins over one body of CLDR data, so the
understanding one slice buys is the other's as well. Each is one slice: description, three examples,
one attack, one bench, one Rust test carrying its `covers:` marker.
`rule:testing/feature-proofs` is what each owes.

- [ ] **`Core\Cldr::pluralCategory`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cldr.rs:1605`
- [ ] **`Core\Cldr::ordinalCategory`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cldr.rs:1617`

## Backlog

- `Core\Task\Channel::send` and `::close` — one file set, `crates/nvs-stdlib/src/channel.rs:121`
  and `:130`; a bench and an attack over a channel both need a task to be suspended on.
- `Core\Cap::has` — the goal's one singleton, `crates/nvs-stdlib/src/cap.rs:81`.
- A member bench declaring `allocations` is still worth adding where the count is knowable;
  `benches/members/README.md` § *What a bench declares* is the home.
