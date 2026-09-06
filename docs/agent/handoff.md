# Handoff

## State

**Goal 9 stage 6 is half landed.** `db.schema` is on the capability roster
(`crates/nvs-config/src/capability.rs:79`) — it names `[db.<name>]` blocks like `db.connect`,
gates whether DDL may be issued at all, and is not implied by the `db.connect` that reached the
same database. `Core\Db\Schema` exists with `rule:core-classes/schema-is-a-value`'s two members, `fromArray` and `toArray`
(`crates/nvs-stdlib/src/db/schema.rs`), over the `Node` tree `nvs-db` already owned: that crate's
`schema.rs` says `nvs-stdlib` converts it "once, and nothing else converts anything", and this
module is that once.

**The decision the class rests on is in its own module doc**: an instance holds its canonical
array and nothing else, normalized on the way *in*, because `crate::instance`'s first decision
leaves no slot for a native `Schema` and § 1 makes the array the value anyway. So `toArray`
answers one array for every spelling of one schema, and § 9's members rebuild an
`nvs_db::schema::Schema` from it.

**The driver's acceptance check is still red, and correctly so**: `examples/schema.nvs` calls
`planAgainst` at its fourth statement and that member is the next slice. Nothing else in stage 6
blocks it.

## Next group

**Stage 6's three members and the fixture they need** — one file set: `crates/nvs-stdlib/src/db/`
(`schema.rs`, `registry.rs`, `mod.rs`), with `nvs.toml` and `docs/spec/01-core-library.md` beside
it. Every input they take is on disk and proved: the diff, the grades, the array form and the
capability.

- [ ] **`planAgainst`, `applySafe` and `applyIncludingRisky`** — `rule:core-classes/schema-apply-capability`: planning is an
      ordinary read under the `db.connect` the program already holds; applying takes `db.schema`
      and an ungranted name throws naming it. `applySafe` refuses a plan holding a step that is
      not `Safe`, naming the first one. The three named tests are
      `docs/agent/loop-goal.toml:4580-4590`. Anchors:
      `crates/nvs-stdlib/src/db/schema.rs:189` (where the two helpers sit, and the `Node` ↔ `Value`
      pair they reuse), `crates/nvs-stdlib/src/db/registry.rs:1536` (`SCHEMA`'s rows and cards),
      `crates/nvs-stdlib/src/db/bind.rs:81` (`handle_of` — how a member reaches its connection),
      `crates/nvs-stdlib/src/registry.rs:1978` (where a row declares the capability it needs),
      `crates/nvs-db/src/plan.rs:418` (`diff`, which is the whole of `planAgainst`'s body).
- [ ] **`nvs.toml` owes the example its app and its database** — an `[[app]]` for
      `examples/schema.nvs`, a `[db.schema]` SQLite block (the example owns its own file rather
      than sharing `main`, because § 7 makes its step count depend on every other table), and both
      `db.connect` and `db.schema` grants for that name. Anchors: `./nvs.toml:174` (the `[[app]]`
      shape), `./nvs.toml:247` (`[db.main]`, where the block goes), and
      `examples/schema.nvs:38` (the `Db::connect("schema")` it must answer).
- [ ] **Spec § 18 owes `Core\Db\Schema` a table** — the registry carries the cards (ADR 0117) but
      the spec is authoritative for the surface, and § 18 names no schema member yet. Anchor:
      `docs/spec/01-core-library.md:1170`.

## Backlog

- `examples/schema.json` does not exist, and `nvs schema plan|apply|dump` is unwritten — the
  `command` check at `docs/agent/loop-goal.toml:4592` needs both (`rule:core-classes/schema-apply-capability`).
- `[context] adrs` for this goal prints §§ 4, 5 and 9 of `rule:core-classes/schema-is-a-value` but not §§ 1-2, which are the
  array form and the vocabulary — this session read both by hand. `docs/agent/loop-goal.toml`.
- `docs/reference/tools/20-config.md`'s capability table gained `db.schema` and the `mail.send`
  row it had been missing; `docs/novis.md` is generated from it by `tools/reference.py`.
