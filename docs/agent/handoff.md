# Handoff

## State

**Goal `tooling-overhaul`: Stages 3, 4 and 5 are landed. Stage 6 has its key function and its units on disk.**
`tools/nv/keys/` holds `scan.ts` (the tiers `raw` > `docs` > `code` > `shipped` > `card`), `tree.ts`
(a lazy reading of the tree, `edited()` for an in-memory what-if), `graph.ts` (`cargo metadata`),
`partition.ts`, `key.ts` (`builtFrom`, the one function) and now `checks.ts`. `checks.ts` lists every
unit a key answers for: 165 test binaries, the two legs and each `[[check]]` of the goal file, 1228
in all. Each unit has a `source` and a `parts(tree)`. `.agent-tmp/checks-smoke.ts` keys them all in
about 2 s, and a what-if in about 1.5 s. It prints what each probe edit moves, grouped by kind. It is
the template for `--probe`. `tools/nv/test/checks.test.ts` pins how each shape of check is sorted.
Nothing calls `checks.ts` yet. `verify`, `impact`, `loop` and `proofs` are still Python and switch
at the cutover. The driver's red check (`nv impact --probe`) is the next item, not a regression.
`data/` is a snapshot, not yet the authority. `main` is frozen. Tag `pre-overhaul` is the rollback.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green. `tools/nv/lib/py.ts` holds what a port needs to print exactly what
Python printed. `bun x tsc --noEmit -p .` typechecks the tools.

What the smoke showed, and what the probes must settle:
- **Holds already.** A `#[test]` in `jwt.rs` moves no program, suite, leg, fuzz, TSan or `{nvs}`
  check. A card `short` in `json.rs` moves no program, suite or leg. Fuzz moves only on the syntax
  edit, and TSan only on the host edit.
- **Moves where a probe says it must not.** The five binaries in `tools/data/impact-wide.txt`
  move on any edit, and so do the binaries whose recorded reads name a whole directory. On the
  `test-module` edit that is nvs-config ×5, nvs-lsp ×2 and nvs-runtime `manifest_policy`, which
  records `.`. The 23 `bun nv` checks key on everything. The dossier gates are observed and read
  `.rs` files, so 155 of them move on the `test-module` edit. The probe table counts those as
  "proofs", which is Stage 7's.

## Next group

**Stage 6: one key for what a check reads, the keystone.** One file set: `tools/nv/keys/checks.ts`,
`tools/nv/cmd/impact.ts`, `tools/nv/cmd/why.ts`, `data/impact-probes.json` and `tools/nv/main.ts`.
loop-goal.md § *Stage 6* is the spec, and its probe table is the contract.

- [ ] **`data/impact-probes.json` and `bun nv impact --probe`**: the ten probes of loop-goal.md
      § *Stage 6*. Each is an edit applied with `Tree.edited`, plus a `rerun` and a `keep` list of
      unit-name patterns. An empty `rerun` is refused. It prints one line per probe name and then
      `impact probes: every probe holds`. `.agent-tmp/checks-smoke.ts` is the body. Units come from
      `tools/nv/keys/checks.ts:240`, and the command goes in `COMMANDS` at `tools/nv/main.ts:31`.
      The smoke findings under `## State` say which `keep` cells fail today. Each one is narrowed
      in `tools/nv/keys/checks.ts:254` (binary reads) or `tools/nv/keys/checks.ts:301`
      (classification). A narrowing is never done by dropping a unit from the `keep` list.
- [ ] **`bun nv why "<check name>"`**: prints `source: …`, then the unit's parts grouped by
      partition. A whole-partition part prints as `<name>`. Crate files print their package once,
      so `nvs-syntax` and `nvs-diagnostics` show for fuzz. It is the four `why` checks of
      stage `6 one key`, and it reads `tools/nv/keys/checks.ts:240`.

## Backlog

- `BUILD_READS` in `tools/nv/keys/key.ts` is a hand table of what each `build.rs` reads outside its
  package. A `--check` that greps `rerun-if-changed` against it is owed (loop-goal.md § *Stage 6*).
- A test binary whose sources leave the package without `nvs_repo` (`tools/impact.py`'s `way_out`)
  is judged only by the list in `tools/data/impact-wide.txt`, which `checks.ts` reads. Port the
  judgement itself with the verify-record source, never as a second copy.
- `checks.ts` keys the conformance and differential suites at `card`, and `tools/verify_keys.py:417`
  keys them at `shipped`. The cutover moves `verify` onto `checks.ts`, and the difference goes in
  the parity list.
