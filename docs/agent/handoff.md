# Handoff

## State

Goal `core-db-rows-and-1-more` is reached. All eleven features of `Core\Db\Rows` and `Core\Db\Schema`
carry their feature proofs, and the one check holding the DONE claim — the `vscode (headless)` floor
check — is green: 250 passing, 0 failing. Both goal-end gates (`owners.py --closes`, `playbook.py
--closes`) name nothing and `verify.py --doc` resolves every link. Nothing is blocked.

That check was a real defect, not stale work. `editors/vscode/src/shadow.ts`'s `sweep` deletes an
earlier build's copy of the same source at once, and still expressed "at once" as
`now - Math.max(mtimeMs, birthtimeMs) >= grace` with `grace` at `0`. A file written a moment ago can
carry a timestamp ahead of `Date.now()`: a probe of 2000 rounds on this machine read one ahead in over
half of them, by up to 2.3 ms. The subtraction then reads as an age below zero and the copy stays. The
same-source branch no longer reads an age at all, and a new case sweeps with a clock a minute behind
the file system, which fails against the old comparison with the assertion the floor check reported.

## Next group

**Goal `core-db-transaction-and-1-more`, whose own handoff the switch installs over this one** — one
file set: `crates/nvs-stdlib/src/db/transaction.rs`, each feature's proof trees, and `nvs.toml` where
a program opens a transaction. One slice is one feature with all of `rule:testing/feature-proofs`.

- [ ] **`Core\Db\Transaction::execute`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:660`, and its `symbol` is the connection's: one body is
      reached through either handle.
- [ ] **`Core\Db\Transaction::executeMany`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:673`.
- [ ] **`Core\Db\Transaction::query`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:618`.

One fact from the `Core\Db\Schema` group that still applies: a program that runs statements needs its
database named in `nvs.toml` beside `connect`, and `[db.notes]` is `:memory:`, so every program creates
the tables it wants to find.

## Backlog

- The extension's own tests write their scratch trees under the OS temp directory
  (`mkdtempSync(join(tmpdir(), …))`, `editors/vscode/test/client/shadow.test.ts:31`). `AGENTS.md`
  rule 10 puts an agent's scratch under `.agent-tmp/` and says nothing about a test's; the user's call.
