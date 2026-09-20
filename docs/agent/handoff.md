# Handoff

## State

Goal `core-cache-and-5-more` (14 items). Items 1 to 3 are on disk and complete: `Core\Cache::local`,
`::process` and `::shared` each carry `about.md`, three examples with blessed `.out`, one attack, and a
Rust test marked `// covers:`. `local` and `process` carry a bench and a ledger row;
`Core\Cache::shared` is a recorded `perf` skip in `tools/data/dossier-policy.toml` — it dials a store no
sweep runs, so a figure would measure another server and the network.

`tools/impact.py`'s `--check` reported `nvs-runtime test manifest_policy` as narrow and demanded its
`impact-wide.txt` line be deleted. It is not narrow: a binary recompiled since its last recorded run has
no reads to judge, and only the first of `findings`' two loops skipped that state. Fixed in place, and
the `impact-wide.txt` line stands.

## Next group

The three plain store operations, in the order a reader meets them. They share one file set:
`crates/nvs-stdlib/src/cache.rs`, `docs/examples/core/Cache-Store/<member>/`,
`tests/hostile/core/Cache-Store/<member>/`, `benches/members/core/Cache-Store/<member>.nvs`. Each is one
slice: description, three examples, one attack, one bench, one Rust test carrying its `covers:` marker.
`rule:testing/feature-proofs` is what each owes.

- [ ] **`Core\Cache\Store::put`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cache.rs:326`
- [ ] **`Core\Cache\Store::get`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cache.rs:346`
- [ ] **`Core\Cache\Store::forget`** — owes about, examples, hostile, perf, one Rust test.
      `crates/nvs-stdlib/src/cache.rs:358`

Two facts that save the next session a round trip. A keyed array is `array<V>`, and a helper must be a
`public static function` inside a class — there are no free functions. And every Rust edit of the group
goes in before `python tools/dossier.py --record-perf --only …`, because a comment in `cache.rs` re-stales
every figure already written.

## Backlog

- Items 4, 7, 9 of the goal — `Core\Cache\SecretEntry::of`, `Store::getSecret`, `Store::putSecret`.
- Items 10 to 14 — `Core\Cap::has`, `Core\Task\Channel::close`/`::send`, `Core\Cldr::ordinalCategory`/
  `::pluralCategory`. Different file sets (`cap.rs`, `channel.rs`, `cldr.rs`), so a group of their own.
- The 16 conformance cases crediting `Core\Cache::local` do so by calling it; none carries a `covers:`
  marker. Nothing owes one, and `dossier.py --id` says so on every run.
