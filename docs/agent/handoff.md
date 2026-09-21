# Handoff

## State

Goal `core-csrf-and-3-more`, 11 of 12 items done. `Core\Db\Column::name` and `Core\Db\Column::type`
now carry every feature proof `rule:testing/feature-proofs` names, and `python tools/dossier.py --id
'Core\Db\Column::name'` and `--id 'Core\Db\Column::type'` both print `complete.` **`::nullable` is
the last item of the goal**, so the session after it runs the `DONE` gates — `verify.py --doc`, then
`owners.py --closes core-csrf-and-3-more` and `playbook.py --closes core-csrf-and-3-more`.

**`[db.notes]` (`nvs.toml:821`) is still the recipe the example, attack and bench programs run on.**
Each entry needs one `[[app]]` block with `[app.capabilities.db] connect = ["notes"]` and **no `fs`
grant** — `rule:core-classes/db-capabilities` is that split, and `nvs.toml:619` is the comment that
owns it. A program with no `[[app]]` block of its own is refused, and one `[[app]]` entry naming a
file that does not exist refuses *every* program in the tree (`E0605`), so write the file before the
block or run nothing.

**A `.nvst` case needs none of that.** It carries its own settings in a `--FILE nvs.toml--` section
and reaches a real engine through a `[db.main]` block naming `sqlite` and `:memory:` —
`tests/conformance/core/db-column-name-is-the-label-the-statement-wrote.nvst:66` is the shape, and
that is what makes a running case cheaper than a `[[app]]` entry.

**The Rust half sits in `crates/nvs-stdlib/src/db/row.rs`'s `mod tests`**, which now opens a real
in-memory SQLite: `nvs_config::tree::Database` → `nvs_db::SqliteTarget::resolve` →
`nvs_db::sqlite::open`, then `conn.query(sql, Vec::new())` for the schema and
`answered.columns().to_vec()` for the description. A member is driven with
`nvs_runtime::call(nvs_core_db_column_nullable, &mut ctx, &[built])` over a
`crate::instance::build(&COLUMN, [...])`, and the local `released` helper at `row.rs:1281` drops
what the frame was handed. Note that `type` and `nullable` return the slot **unowned** — only `name`
wraps it in `owned(...)` — so a case over those two releases the receiver and nothing else.

`Core\Db\Column::name` measures 56.0 ns and `::type` 83.3 ns, both 0 calls and 0 allocations, and
the declared `calls 0` held for each. Neither attack found a bug: ten thousand columns, a label and
a declared type of a million letters each, two thousand columns with two thousand declared types,
and a hundred thousand reads of one description all leave the runtime standing.

The pack did not print `crates/nvs-db/src/sqlite.rs`, which is where `SqliteColumn::column_type`
maps a declared type name onto a case and where the two answers a proof must be honest about are
argued. `[context] modules` should name it; the driver cannot sweep it in, because no commit of this
goal touches that crate.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/`, `docs/examples/core/Db-Column/nullable/`,
`tests/hostile/core/Db-Column/nullable/`, `benches/members/core/Db-Column/nullable.nvs`,
`tests/conformance/core/` and `nvs.toml`. Make the `crates/` edit **before** the first `--bless` or
`--record-perf`: each one buys another release relink, which the playbook's *Running things* bullet
prices.

- [ ] **`Core\Db\Column::nullable`** — owes about, three examples, an attack, a bench and two tests.
      `crates/nvs-stdlib/src/db/registry.rs:1567` is the row, and its comment is the decision to
      say: the answer is `bool` and not `?bool`, because an absence would be a third answer every
      caller branches on to learn nothing. `crates/nvs-stdlib/src/db/execute.rs:999` is why this
      driver answers `true` for every column — SQLite describes no nullability at all on a stepped
      statement — so an example claiming a `not null` column answers `false` is wrong, and
      `COLUMN_NULLABLE_DOC` at `crates/nvs-stdlib/src/db/registry.rs:2664` is the sentence the
      `about.md` is written from. The two sibling Rust cases are at
      `crates/nvs-stdlib/src/db/row.rs:1308` and `crates/nvs-stdlib/src/db/row.rs:1381`.

## Backlog

- `Core\Db\Column::nullable`'s proofs are the goal's last item; the session after it runs the `DONE`
  gates named in `## State`.
- `[context] modules` in `docs/agent/loop-goal.toml` does not name `crates/nvs-db/src/sqlite.rs`.
