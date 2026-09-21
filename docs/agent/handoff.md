# Handoff

## State

Goal `core-db-rows-and-1-more` is reached. All eleven of its items carry their feature proofs:
`python tools/dossier.py --group 'Core\Db\Rows' --owed` and the same for `Core\Db\Schema` both print
zero features owed, `--id` prints `complete.` for every member, and the two goal-end gates
(`owners.py --closes`, `playbook.py --closes`) name nothing. Nothing is blocked.

One bug the proofs found and this session fixed: `Core\Db\Schema::applySafe`'s refusal put a full
stop after the grade's reason, which already ends with one, so every refusal read `… elsewhere..`.
The format string in `crates/nvs-stdlib/src/db/schema.rs:673` no longer adds one, and
`apply_safe_refuses_a_plan_holding_a_step_that_is_not_safe` asserts the message holds no `..`.

Two facts the next `Core\Db` dossier goal should not re-derive. **A void member's bench chains
through its receiver**, the way `benches/members/core/Db-Connection/close.nvs` does: two entries of
the same value in an array, indexed by the running total. **An apply program needs
`schema = ["notes"]` beside `connect = ["notes"]` in `nvs.toml`**, and `[db.notes]` is `:memory:`, so
every program creates the tables it wants to find.

## Next group

**Goal `core-db-transaction-and-1-more`, whose own handoff the switch installs over this one** — one
file set: `crates/nvs-stdlib/src/db/transaction.rs`, each feature's proof trees, and `nvs.toml` where
a program opens a transaction. One slice is one feature with all of `rule:testing/feature-proofs`.

- [ ] **`Core\Db\Transaction::execute`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:659`.
- [ ] **`Core\Db\Transaction::executeMany`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:672`.
- [ ] **`Core\Db\Transaction::query`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:617`.

## Backlog

- A grading reason reaches an end user through `applySafe`'s refusal and carries `§ 6` from the
  record that graded it — `crates/nvs-db/src/ddl.rs` writes those sentences, and AGENTS.md
  § *Text an end user reads* says no reader ever needs a section number.
