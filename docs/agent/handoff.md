# Handoff

## State

**Goal 38 — an encoder ends a cycle where it closes. Stage 2's two `cargo-named` checks are green.**
The five tests they name are in `crates/nvs-stdlib/src/json.rs`'s test module, over the object
fixture that module had never had: `holder_class` defines a class with a `mixed` codec and leaks its
table, `instance` builds one, `set_property` writes a slot through a handle rebuilt from the value's
own address, and `encoded` is the walk without the member's option bag. The encoder itself needed no
change — it already carried the ancestor chain (`crates/nvs-stdlib/src/json.rs:726`, and the array
arm's own guard at `:824`).

Stage 2's third check is the whole conformance tree, and this goal's case is already on disk:
`tests/conformance/core/json-encode-ends-a-cycle-at-the-first-repeat-and-names-its-path.nvst` states
the cycle, the shared value and the depth bound in one case.

**Stage 3's check is red, and "stage 3 is complete" was true of the audit alone.** The audit landed
as a paragraph in each walker's module doc, but `docs/agent/loop-goal.toml:6365` names a test
`every_encoder_that_reaches_an_object_graph_refuses_a_cycle_rather_than_recursing`, which exists
nowhere under `crates/` — grepped, not inferred. Stage 4 is untouched:
`rule:classes/an-encoder-ends-a-cycle-by-identity` is still `designed`.

**The encode walk's frame budget is not the depth cap.** Encoding at `DEPTH_CEILING` needs more stack
than a test thread is given by default, which is that module's own gap 7 rather than anything this
goal decided; the deep test spawns a thread that sizes its own stack and the playbook holds the trap.

## Next group

**Stage 3 then stage 4: the audit's own test, then the rulebook** — one file set:
`crates/nvs-stdlib/src/json.rs`'s test module and `docs/rules/classes*`.

- [ ] **The audit's agreement test** — `crates/nvs-stdlib/src/json.rs:2576` is the fixture the test
      builds on, and the name is exact, from `docs/agent/loop-goal.toml:6365`:
      `every_encoder_that_reaches_an_object_graph_refuses_a_cycle_rather_than_recursing`. That
      check's stage comment is the specification — *an answer exists per file, not that the answer
      is yes* — so what it asserts is the audit's finding: the one walker that reaches an object
      graph refuses a cycle, and the others are handed a leaf they refuse by type.
      `rule:classes/an-encoder-ends-a-cycle-by-identity`.
- [ ] **The rule stops being `designed`** — `docs/rules/classes.json:525` is the entry, and stage 4's
      three checks are `python tools/rules.py --check`, `--render --check` and `python
      tools/decisions.py --gate`. Its `guardedBy` owes the conformance case named in `## State` and
      the tests landed this session; the fragment's prose already states the shipped rule.

## Backlog

- Stage 2's `nvs-suite` check passes on the tree, but nothing re-reads whether the goal wanted a
  *second* conformance case — `docs/agent/loop-goal.toml:6354`.
- `Core\Json::encode`'s frame budget at the ceiling, `crates/nvs-stdlib/src/json.rs`'s gap 7.
