# Handoff

## State

The chain is inside the generated dossier goals and goal `lang-testing` is live; two of its three
features are finished. `lang:testing/a-test-is-a-method-marked-test` and
`lang:testing/assertions` each carry every feature proof — `about.md`, three examples, one bench
with a figure in `docs/perf/members.ndjson`, an attack, and conformance cases carrying the
`covers:` marker a language feature is attributed by. `python tools/dossier.py --only
'lang:testing/a-test-is-a-method-marked-test' 'lang:testing/assertions' --verify` is green.

Two findings are recorded rather than fixed, because each needs a decision no dossier goal may
take. An `abstract` `#[Test]` compiles and `nvs test` then raises the internal-error fault for a
method it cannot find (`crates/nvs-types/src/testing.rs`, `check_method_shape` § *Known gaps*);
the attack that found it is marked `known-gap` and goes green the day the refusal lands. And the
assertion ledger is appended to under `nvs run` as well, where nothing reads it
(`crates/nvs-runtime/src/ctx/error.rs`, `record_assertion` § *Known gaps*).

## Next group

**One slice is one feature with all its feature proofs** — one file set:
`docs/examples/lang/testing/<slug>/`, `tests/hostile/lang/testing/<slug>/`,
`benches/members/lang/testing/<slug>.nvs` and `tests/conformance/lang/`. This is the goal's last
feature; after it lands, the goal's closing gates are what the session owes.

- [ ] **`lang:testing/core-test-fixture-built-once-injected-by-type`** — owes about, examples,
      hostile, perf, tests. `rule:testing/fixtures` specifies it; an attack wants the two shapes
      `reject_uninjectable_parameters` already refuses at
      `crates/nvs-types/src/testing.rs:1028`, and the chapter is
      `docs/reference/lang/95-testing.md:177`.

## Backlog
- An `abstract` `#[Test]` is not refused — `crates/nvs-types/src/testing.rs` § *Known gaps* 1.
- The assertion ledger is never truncated outside a test — `crates/nvs-runtime/src/ctx/error.rs` § *Known gaps* 1.
- `[context] shapes` does not print `docs/examples/README.md` § *The description*, so every dossier session re-reads it to write `about.md` — `docs/agent/loop-goal.toml`.
- `[context] shapes` does not print the `.nvst` `--RUN--` directive, which every case that drives `nvs test` needs — `docs/agent/loop-goal.toml`.
- `lang:testing` has four features after this goal: data rows, the per-test isolate, `#[Test(...)]` options and `nvs test` itself — `docs/agent/goals/dossier/`.
