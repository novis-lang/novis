# Handoff

## State

**ADR 0067 § 7's transaction is landed, closure and all.** `transaction` is one
`TRANSACTION_ROW` (`crates/nvs-stdlib/src/db.rs:392`) carried by both `CONNECTION` and the new
`TRANSACTION` class, and `handle_of` is what makes one statement path serve either receiver —
which is how ADR 0043's `implements Queryable by $connection` is spelled here: the same rows
under the same symbols, no forwarding bodies. Returning commits, throwing rolls back, and the
`rollBack` flag is read off the transaction's own slot so an intervening `catch (Throwable)`
cannot leave the work committed. `TRANSACTION`'s own doc comment owns the slot layout and why
the first two are `CONNECTION`'s.

**`examples/transaction.nvs` now compiles and runs to the capability check** — the goal's
stated external precondition (a reachable Docker daemon behind `[db.main]`) is all that stands
between it and the five frozen lines. Nothing in the tree blocks it.

**§ 7's options bag is owed and its absence is a subset, not a divergence**: with no
`{isolation?, readOnly?, retries?}` every call takes § 7's own defaults — driver isolation,
read-write, zero retries. `TRANSACTION_ROW`'s doc comment is that fact's home, and the next
group closes it.

**A `use` alias does not reach a `catch`** — the playbook bullet added this session has the
anchor and the three classes it bites. The example works around it by writing
`Core\Db\RolledBack` out; nothing else in the corpus does yet.

## Next group

**§ 7's options bag — the file set is `crates/nvs-stdlib/src/db.rs` and
`crates/nvs-db/src/pg.rs`, which already carries the `Isolation` enum and renders both options
into a `BEGIN`.**

- [ ] **`Core\Db\Isolation`, § 7's five cases as a registry enum** — ADR 0067 § 7, spec
      § 18's own `Isolation { ReadUncommitted, ReadCommitted, RepeatableRead, Snapshot,
      Serializable }` line. `nvs-db`'s half is landed at `crates/nvs-db/src/conn.rs:203` and
      the five spellings PostgreSQL wants at `crates/nvs-db/src/pg.rs:3049`; the registry side
      is a `CoreEnum` beside the classes at `crates/nvs-stdlib/src/db.rs:432`, reached from a
      `CoreTy::Enum` — `registry.rs`'s `CoreTy::Enum` doc says how a name resolves.
- [ ] **`transaction`'s `{isolation?, readOnly?}` bag, reaching `PgConn::begin`** — ADR 0067
      § 7. The row is `crates/nvs-stdlib/src/db.rs:392` and gains a trailing
      `CoreTy::Options`; the body is `crates/nvs-stdlib/src/db.rs:2385`, whose `begin(None,
      false)` is where the two land. Note ADR 0063 R2: an options bag has no `names` entry, so
      only `MethodDoc::params` grows — one `ParamDoc` per option, under the option's own name.
- [ ] **`{retries: n}`, outermost transactions only** — ADR 0067 § 7's last paragraph, whose
      default of 0 is deliberate and stays. The loop goes around the `begin`/`call_closure`
      pair at `crates/nvs-stdlib/src/db.rs:2385`, and re-runs only on `Deadlock` and
      `SerializationFailure`, which means reading § 8's kind off the refusal —
      `crates/nvs-db/src/pg.rs:1018` is the code table that answers it. Backing off has to
      suspend the coroutine rather than block the core.

## Backlog

- `Rows::columns()` and its `Core\Db\Column`/`ColumnType` pair — `crates/nvs-stdlib/src/db.rs`'s
  `ROWS` doc lists the three things it needs.
- `queryAs<T>`, `stream` and `streamAs` on both `Queryable` classes — ADR 0067 §§ 4 and 6.
- `Core\Db::open` waits on a `CoreTy` for a shape *parameter* — that module's known gaps.
- A `use` alias reaching a `catch` — `crates/nvs-ir/src/lower/exception.rs:597`, and the design
  question is which crate resolves the label.
- Stage 5's remaining `nvs-db` cases (`savepoints_nest`, `retries_recover_an_induced_deadlock`)
  — `docs/agent/loop-goal.toml`'s stage 5 list.
- Stages 6 and 7 — the four other drivers and § 13's pool — both need Docker.
