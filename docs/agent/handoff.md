# Handoff

## State

Goal `core-http-client-and-2-more` is complete. Every one of its features has `about.md`,
three examples, one attack, one bench and a Rust test. `python tools/dossier.py --verify
--group` answers `nothing owed` for `Core\Http`, `Core\Http\Client`, `Core\Http\Identity` and
`Core\Http\Part`.

The driver's last acceptance check was red on `Core\Http::allowUrl`'s perf record, stale
because `crates/nvs-stdlib/src/http.rs` changed after it was measured. It is re-measured
(`docs/perf/members.ndjson`). `verify.py --doc`, `owners.py --closes` and
`playbook.py --closes` are clean for the goal.

## Next group

**Stage 1: the goal is complete** — one file set: `crates/nvs-stdlib/src/http.rs` and the proof trees.

- [x] **Re-measure `Core\Http::allowUrl`** — `crates/nvs-stdlib/src/http.rs:1`, `rule:testing/member-perf-ledger`.

## Backlog
- `Core\IO\File::read(length)` now throws when its chunk ends inside a multi-byte character, so a chunked read of valid text can fail. Returning `bytes` there, or seeking back over the partial character the way `readLine` seeks, is a user decision (`crates/nvs-stdlib/src/io.rs`).
- Other `Value::str(NvsStr::new(..))` sites over outside octets are not checked: `Core\Request::headers` (`crates/nvs-stdlib/src/request.rs:2542`), `crates/nvs-stdlib/src/cache.rs:1731`, `crates/nvs-stdlib/src/uri.rs:1469`.
- `nvs-cli`'s `serve_arms_a_fleet_entry_when_the_shared_tier_can_take_a_lease` and `a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile` each failed once beside the other test binaries and passed alone (`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/script.rs`).
- A session that edits a member's implementing file after `--record-perf` leaves the figure stale; run `python tools/dossier.py --verify --group <class>` for every class the goal names before reporting `DONE` (`tools/dossier.py`).
