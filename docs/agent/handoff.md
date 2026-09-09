# Handoff

## State

**Goal 38 — an encoder ends a cycle where it closes. Stage 2's *implementation* landed and its *tests*
did not:** `Core\Json::encode` carries the ancestor chain and refuses at the first repeat
(`crates/nvs-stdlib/src/json.rs:726`), but none of the five test names stage 2's checks list exists
anywhere under `crates/` — grepped, not inferred. That is the goal's earliest red check, and it is
unwritten work rather than a regression. Goal 25's whole list remains this goal's stage 1 floor and is
untouched.

**Stage 3 is complete.** Each of the three walkers was audited and none can reach an object graph, so
each took one paragraph in its own module doc and nothing else, which is what the goal's § *Standing
decisions* asks for that outcome. `Core\Csv::format` walks exactly two non-recursive levels and
requires a `string` at the leaf; `Core\Uri::buildQuery` descends into arrays alone and hands an object
whole to `scalar_text`; no `Core\Encoding` member takes a container at all.

The design is settled by [0164](../decisions/0164.md) and its § *Standing decisions* — the set is the
ancestor chain, the answer is a throw with no marker in the document, the depth cap stays. Stage 3
decided nothing and opened no ADR.

## Next group

**Stage 2: the keystone's five tests, which nothing on disk holds** — one file set:
`crates/nvs-stdlib/src/json.rs`'s test module. The encoder already behaves the way these pin it to;
what is missing is the pinning. That module is decode-only today and has **no object fixture**, so the
first item builds one — the playbook's `-p nvs-stdlib` bullets under *Writing a test case* own how a
class table is installed and what a compiled class needs.

- [ ] **The object fixture, and the three refusal tests** — `crates/nvs-stdlib/src/json.rs:2499` is
      the module; the refusal is raised at `crates/nvs-stdlib/src/json.rs:726` and its message built
      at `crates/nvs-stdlib/src/json.rs:587`. The names are exact, from
      `docs/agent/loop-goal.toml:6327`:
      `an_object_that_holds_itself_is_refused_at_the_property_that_closes_the_cycle`,
      `the_refusal_names_the_property_chain_rather_than_a_count_of_levels`, and
      `a_cycle_that_closes_through_an_array_is_refused_at_the_same_place`, which lands on the array
      arm's guard at `crates/nvs-stdlib/src/json.rs:817`.
      `rule:classes/an-encoder-ends-a-cycle-by-identity`.
- [ ] **The two bounds that must *not* be refused as cycles** — same module,
      `crates/nvs-stdlib/src/json.rs:2499`, checked by `docs/agent/loop-goal.toml:6344`:
      `one_object_held_by_two_properties_is_written_twice_rather_than_refused` pins the
      ancestor-chain-not-everything-seen half of the rule, and
      `a_document_deeper_than_the_ceiling_still_reports_depth_and_not_a_cycle` pins the depth guard at
      `crates/nvs-stdlib/src/json.rs:808` still reporting depth.
- [ ] **This goal's two conformance cases** — `docs/agent/loop-goal.toml:6354` names the check and not
      the file names. `tests/conformance/core/` is flat with a subsystem prefix, so read the
      neighbours before choosing one; a cycle here is a runtime throw, so neither case is an
      `--EXPECTF-ERROR--`.

## Backlog

- An acyclic document deeper than the native stack on the encode side — `crates/nvs-stdlib/src/json.rs`
  known gap 7 owns it, and nothing schedules it.
- `Core\Csv` has no streaming half — `crates/nvs-stdlib/src/csv.rs` known gap 1, waiting on `Core\IO`.
