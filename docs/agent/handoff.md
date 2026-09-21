# Handoff

## State

Goal `core-cache-and-5-more` (14 items). Eleven are on disk and complete: the three tier members,
the three plain store operations, the sealed half, and now both CLDR members —
`Core\Cldr::pluralCategory` and `Core\Cldr::ordinalCategory`. Each of the eleven carries `about.md`,
three examples with blessed `.out`, one attack, one bench with a ledger row, and a test from each
side. `Core\Cache::shared` stays a recorded `perf` skip in `tools/data/dossier-policy.toml`.

The two CLDR members' Rust proofs are `// covers:` markers on the two sweeps that already pinned
exactly their claims: `plural_category_answers_from_the_carried_cldr_data` counts that the carried
rules reach all six categories and that Russian is answered from its own rules, and
`an_ordinal_category_is_answered_for_every_language_with_a_published_table` sweeps the second table.
Their `.nvst` sides were already credited by plain calls, since `rule:testing/proof-attribution`
only makes an *instance* member need the marker.

Nothing is blocked. Three items are left, and the driver's acceptance check names the first of them.

## Next group

**Stage 2: the dossier** — two file sets, in this order. `Core\Cap::has` is the failing acceptance
check and is one member over one file: `crates/nvs-stdlib/src/cap.rs`,
`docs/examples/core/Cap/has/`, `tests/hostile/core/Cap/has/`, `benches/members/core/Cap/has.nvs`.
The two `Core\Task\Channel` members then share their own file set,
`crates/nvs-stdlib/src/channel.rs` and the same four trees under `core/Task-Channel/<member>`, and
they are near-twins over one bounded queue, so the understanding one of them buys is the other's as
well. Each is one slice: description, three examples, one attack, one bench, one Rust test carrying
its `covers:` marker. `rule:testing/feature-proofs` is what each owes.

- [ ] **`Core\Cap::has`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cap.rs:81`
- [ ] **`Core\Task\Channel::send`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/channel.rs:121`
- [ ] **`Core\Task\Channel::close`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/channel.rs:130`

## Backlog

- An attack over `Core\Task\Channel` needs a second task, so read `crates/nvs-stdlib/src/channel.rs`
  before budgeting it as one slice — `docs/agent/playbook.md` holds the `-p nvs-cli` scheduler traps.
- `Core\Cache::shared` keeps its `perf` skip; the reason is in `tools/data/dossier-policy.toml`.
