# Handoff

## State

Goal `lang-testing` is **met**: `python tools/dossier.py --verify --group lang:testing` reports
nothing owed and both suites at zero failures, so all seven features in the group carry every
feature proof. `python tools/owners.py --closes lang-testing` and `python tools/playbook.py --closes
lang-testing` both report nothing owned, and `python tools/verify.py --doc` is green.

Two findings stay recorded rather than fixed, each needing a decision no dossier goal may take. An
`abstract` `#[Test]` compiles and `nvs test` then raises the internal-error fault
(`crates/nvs-types/src/testing.rs`, `check_method_shape` § *Known gaps*), and the assertion ledger is
appended to under `nvs run` as well, where nothing reads it
(`crates/nvs-runtime/src/ctx/error.rs`, `record_assertion` § *Known gaps*). A failed assertion under
`nvs run` throws `Core\Test\Failure` out of the top-level statement that made it, which the
`running-tests-nvs-test` attack catches in its step 5.

## Next group

**Goal `core-arr-1-4`, stage 2: one slice is one feature with all its feature proofs** — one file
set: `docs/examples/core/Arr/<member>/`, `tests/hostile/core/Arr/<member>/`,
`benches/members/core/Arr/<member>.nvs` and a `-p nvs-stdlib` Rust test beside the member. A `Core`
member is credited by a case that plainly calls it, so an existing conformance case needs no
`covers:` marker; the Rust `#[test]` needs one.

- [ ] **`Core\Arr::all`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/arr.rs:216`
- [ ] **`Core\Arr::any`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/arr.rs:204`
- [ ] **`Core\Arr::append`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/arr.rs:371`

## Backlog

- The `lang:testing` benches for `test-options` and `running-tests-nvs-test` declare no
  `// bench: allocations`, because what a construction allocates was not known before the first
  measurement — `benches/members/README.md`.
- `Core\Test` has no `assertFalse`; the roster is `docs/reference/lang/95-testing.md:285`.
