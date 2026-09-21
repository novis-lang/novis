# Handoff

## State

Goal `core-db-rows-and-1-more` — the generated dossier goal over `Core\Db\Rows` and `Core\Db\Schema`
— has 2 of its 11 items landed: `Core\Db\Rows::all` and `Core\Db\Rows::column` each carry their
feature proofs, and `python tools/dossier.py --id '<feature>'` prints `complete.` for both. Nothing
is blocked.

Two things the next session should not re-derive. `result_over` at the tail of
`crates/nvs-stdlib/src/db/row.rs`'s `mod tests` builds a `Core\Db\Rows` over two rows with no class
to hydrate into and no described columns, which is what makes the remaining `Rows` members testable
from Rust at all — the playbook's bullet about no `nvs_db::Connection` being buildable there still
holds, and this goes under it. And a perf record is keyed on the implementing file's text with its
trailing `mod tests` cut off (`benches/members/README.md` § *When a figure is re-measured*), so
appending a Rust case to `row.rs` does not stale a figure already taken.

## Next group

**One file set:** `crates/nvs-stdlib/src/db/registry.rs` (the reference card), `crates/nvs-stdlib/src/db/row.rs`
(the member and the `mod tests` tail), `nvs.toml` (one `[[app]]` per runnable proof), and each
feature's own proof trees. One slice is one feature with all of `rule:testing/feature-proofs`.

- [ ] **`Core\Db\Rows::columns`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:1268` and the member `crates/nvs-stdlib/src/db/row.rs:949`;
      it is a reader over one slot, so the Rust claim is the second reference rather than a copy.
- [ ] **`Core\Db\Rows::count`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1259`, member at `crates/nvs-stdlib/src/db/row.rs:929`.
- [ ] **`Core\Db\Rows::first`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1226`, member at `crates/nvs-stdlib/src/db/row.rs:843`.
- [ ] **`Core\Db\Rows::value`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1236`, member at `crates/nvs-stdlib/src/db/row.rs:867`.

## Backlog

- `Core\Db\Schema`'s five members are items 7 to 11 of this goal and a different file set —
  `docs/agent/loop-goal.md` § *The item list*.
- `Core\Db\Rows::column`'s refusal copies the whole column name into its message
  (`crates/nvs-stdlib/src/db/row.rs:614`), so a million-character name makes a million-character
  message. Bounded by what the caller already held, and the attack survives it.
- `docs/perf/members.md` is rendered from the ledger by `python tools/dossier.py --perf-report`, and
  no session in this goal has run it yet.
