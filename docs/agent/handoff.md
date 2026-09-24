# Handoff

## State

**Goal `tooling-overhaul`: Stages 3, 4 and 5 are landed, and Stage 6's key function is on disk.**
`tools/nv/keys/` holds it: `scan.ts` (the Rust scanner, the tiers `raw` > `docs` > `code` >
`shipped` > `card`, and include sites marked in or out of a test module), `tree.ts` (a lazy
reading of every file git does not ignore, `edited()` for an in-memory what-if, analyses memoized in
`.agent-tmp/nv-key-tiers.json`), `graph.ts` (`cargo metadata`, `closure`, `testBinaries` named as
`verify` names them), `partition.ts` (loop.py's partitions) and `key.ts` (`builtFrom`, the one
function, and `keyOf`). `tools/nv/test/keys.test.ts` pins the tiers. On the real tree a `#[test]`
added to `jwt.rs` or `jwe.rs` moves only `nvs-stdlib`'s 11 test binaries; a card's `short` moves 125
test binaries (they key `shipped`) and not a card-tier program key; 180 binaries key in about 0.2 s,
and a what-if in about 0.13 s. `.agent-tmp/key-smoke.ts` is that run, and a template for the probes.
Nothing calls `builtFrom` yet: `verify`, `impact`, `loop` and `proofs` are still Python and switch
at the cutover. The driver's red check (`nv impact --probe`) is the next item, not a regression.
`data/` is a snapshot, not yet the authority; `main` is frozen; tag `pre-overhaul` is the rollback.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools.

## Next group

**Stage 6: one key for what a check reads, the keystone** — one file set: `tools/nv/keys/**`,
`tools/nv/cmd/impact.ts`, `tools/nv/cmd/why.ts`, `data/impact-probes.json`. loop-goal.md § *Stage 6*
is the spec, and its probe table is the contract.

- [ ] **`tools/nv/keys/checks.ts`: every keyed unit and its source** — the test binaries
      (`tools/nv/keys/graph.ts:107`, keyed by `tools/nv/keys/key.ts:104`) plus each `[[check]]` in
      `docs/agent/loop-goal.toml` (read with `smol-toml`), classified as `tools/loop.py:2055`'s
      `reads_of` and `tools/loop.py:3190`'s `inputs_for` do, with the spec's changes: a program
      check, `valgrind `, the suites, fuzz, TSan and the database matrix build at the `card` tier;
      `{nvs}` meta/agent/doc/lsp commands and cost margins at `shipped`; fuzz is the closure of the
      crates `fuzz/fuzz_targets/prefix.rs` `use`s plus `fuzz/**`; TSan is `tools/tsan.sh`'s `-p`
      packages raw with `test: true`, plus the script and `examples/`; a test check and the
      conformance and differential suites answer `source: verify record`. Each unit returns
      `Part[]`, so `why` and the probes share it.
- [ ] **`data/impact-probes.json` and `bun nv impact --probe`** — the ten probes of loop-goal.md
      § *Stage 6*, each a synthetic edit applied with `Tree.edited` (`tools/nv/keys/tree.ts:84`),
      and `rerun`/`keep` selectors over the unit names; an empty `rerun` is refused. Prints each
      probe's name, then `impact probes: every probe holds`.
- [ ] **`bun nv why "<check name>"`** — prints `source: …`, then the unit's parts grouped by
      `partition` with their tier (`tools/nv/keys/key.ts:58`). The four stage-6 `why` checks name
      the check and the wanted lines.

## Backlog

- `BUILD_READS` in `tools/nv/keys/key.ts` is a hand table of what each `build.rs` reads outside its
  package; a `--check` that greps `rerun-if-changed` against it is owed (loop-goal.md § *Stage 6*).
- A test binary whose sources leave the package without `nvs_repo` (`tools/impact.py`'s
  `way_out`, the list in `tools/data/impact-wide.txt`) is not yet judged in TypeScript; port it with
  the verify-record source, never as a second copy.
- Run-time reads recorded in `.agent-tmp/impact-reads.json` are not yet in a TypeScript key.
