# Handoff

## State

**Milestone M8, goal `queue-purge`. `Core\Queue::delete` is landed and gated**, as the six edits a
capability-bearing member takes: the `CLASS` row, the card, the body, the `address()` arm, the
`CAPABILITIES` row, and three conformance cases. `python tools/verify.py` is 9 of 9 green.

**`queue.purge` is on the capability roster** — `nvs_config::Cap::QueuePurge`, written as
`[capabilities.queue] purge` and asked at `Scope::Name` of the queue a `Queue\Id` carries. That is
stage 4's `-p nvs-config` half, landed early because a member that destroys rows may not exist
ungated for even one commit; the grant is asked before the `[queue]` block is read and before
anything opens, so an ungranted caller cannot read the difference between the two refusals.

**Open:** `purge` itself. Its four statements have been on disk and matrix-tested since the last
session; the member is not written, and `examples/queue-purge.nvs` — the fixture the driver's
acceptance check fails on — waits on it.

**A goal bug, not a gap:** stage 4's diagnostic check names `E0635`, which is already
`E_NO_UNIX_TRANSPORT` at `crates/nvs-diagnostics/src/lib.rs:1778`. The band's next free code is
`E0637`, and that check's number has to be re-decided before its test can be written.

## Next group

**Stage 3: `purge`, the member** — one file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-stdlib/src/registry.rs`, `tests/conformance/core/`.

- [ ] **`Core\Queue::purge(string $queue, {state?, tag?, before?, limit?}): uint`**, as `delete`'s
      six edits one member over: the row after `crates/nvs-stdlib/src/queue.rs:1327`, a card after
      `crates/nvs-stdlib/src/queue.rs:1507`, a body in `nvs_core_queue_delete`'s shape at
      `crates/nvs-stdlib/src/queue.rs:2785` — which is the twin, down to the door check, the
      `Spans` and the `counted_row` call — the `address()` arm at
      `crates/nvs-stdlib/src/queue.rs:2976`, and the `CAPABILITIES` row beside `delete`'s at
      `crates/nvs-stdlib/src/registry.rs:2337`. The four statements are written and each owns its
      own shape decision (`crates/nvs-stdlib/src/queue.rs:1027`, `:1049`, `:1069`, `:1078`), so what
      is left to the member is choosing the dead pair only for `state: Dead`, binding the optional
      `tag` and `before`, refusing `State::Claimed` at the call as a `LogicError`, and reading the
      count out of `Counted`. `rule:concurrency/queue-deletion-is-explicit-and-bounded`, ADR 0153
      § 2 and § 4.
- [ ] **Three `.nvst` cases for `purge`** under `tests/conformance/core/`, each asking a different
      question — the floor at `crates/nvs-stdlib/tests/conformance_coverage.rs:194` turns `cargo
      test -p nvs-stdlib` red the moment the row lands, so they are part of that slice's green
      build and not a later one. `delete`'s
      three are the shape and the naming:
      `tests/conformance/core/queue-delete-is-asked-with-the-receipt-and-not-the-queue.nvst`.
      `rule:concurrency/queue-deletion-is-explicit-and-bounded`, `rule:core-api/shape-rules`.
- [ ] **Stage 5's three `-p nvs-stdlib` tests**, beside `queue_statements_agree_with_the_state_enum`
      at `crates/nvs-stdlib/src/queue.rs:3573`:
      `core_queue_declares_delete_and_purge_beside_its_four_and_both_declare_the_capability`,
      `purges_reference_card_names_the_default_set_the_two_opt_ins_and_the_count`, and
      `pushs_reference_card_documents_tag_beside_key_and_says_which_one_groups` — `tag`'s `ParamDoc`
      already exists at `crates/nvs-stdlib/src/queue.rs:1392`, so that third one is an assertion and
      not an edit. `rule:concurrency/queue-four-members`,
      `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`.

## Backlog

- `E0635` is taken; stage 4's diagnostic needs a free code — `docs/agent/loop-goal.toml:7308`.
- `examples/queue-purge.nvs`, the driver's failing acceptance fixture — `docs/agent/loop-goal.md`.
- Stage 4's static half in `nvs-types`: a *written* `purge` queue name outside the grant is refused
  while checking, a computed one is not — `docs/agent/loop-goal.toml:7308`.
- `tools/reference.py --check` owes an entry for each member and a documented `[capabilities.queue]`
  block — stage 6, `docs/agent/loop-goal.toml:7354`.
