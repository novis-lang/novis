# Handoff

## State

Goal `core-csrf-and-3-more`, items 1 and 2 of 12 are done. `Core\Csrf::issue` and
`Core\Csrf::verify` each carry every feature proof `rule:testing/feature-proofs` names:
`about.md`, three examples with blessed `.out` files, one attack, one bench, and a Rust test
carrying the member's own `covers:` marker. Both figures are appended to
`docs/perf/members.ndjson`, and the `calls 0` each bench declares held.

No proof found a bug. Every forged token in both attacks was refused, a session of sixteen
megabytes is bound and checked with the runtime still standing, and a `$key` that is not 32
octets is the only throw either member has.

`Core\Csv` and `Core\Db` (items 3 to 12) are untouched. Nothing is blocked.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/csv.rs`, plus `docs/examples/core/Csv/`, `tests/hostile/core/Csv/` and
`benches/members/core/Csv/`. `python tools/dossier.py --id '<feature>'` prints the path of
each proof, and `--comments <paths>` counts the three bounds before the wrap does.

- [ ] **`Core\Csv::format`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/csv.rs:211`
- [ ] **`Core\Csv::parse`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/csv.rs:202`
- [ ] **`Core\Csv::rows`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/csv.rs:223`

The two tests at the foot of `crates/nvs-stdlib/src/csrf.rs` are the shape for the Rust half:
a member is driven through `nvs_runtime::call` the way a compiled call site drives it, and the
frame releases every `Value` it built. Write that test before `--bless` and `--record-perf`,
because each of those rebuilds `target/release/nvs` when any Rust file is newer.

## Backlog

- Items 6 to 12 (`Core\Db`, `Core\Db\Column`) share `crates/nvs-stdlib/src/db/registry.rs`;
  the playbook's `-p nvs-stdlib` bullets say only a driver-free member is reachable from a
  Rust test there. `docs/agent/goals/dossier/104-core-csrf-and-3-more.md` is the list.
- `Core\Csrf::issue` measures 1101 ns/op with 5 allocations per token and `Core\Csrf::verify`
  1150 ns/op with 3. Nothing gates either; `docs/perf/members.ndjson` is what a later change
  is re-measured against.
