# Handoff

## State

**ADR 0084 § 2's schema is data now**: `nvs_stdlib::queue::MIGRATION` is five PostgreSQL statements —
both tables, gap 3's partial unique index, § 4's claim index and the dead-letter one — written beside
the `insert` and `select` that read their columns and held to them by
`the_ddl_creates_every_column_the_statements_name`. Its doc owns the three decisions in it:
epoch-millisecond `bigint`s, `claimed_at` as the claim instant and not a deadline, and the
dead-letter row as the job's columns plus `failed_at` and `errors`.

**`nvs queue migrate` prints that schema and cannot apply it**, which `crates/nvs-cli/src/queue.rs`'s
module doc owns: this binary opens a connection only from inside a request and a migration is not
one. `--dry-run` resolves the tree, proves the `[db.<name>]` and its driver, prints, and exits 0;
without the flag it prints the same statements and exits 1 rather than looking like a migration that
happened. The acceptance check's `want` is met by the statement labels — `jobs`, `dead_letter` — so
`DEAD_TABLE` keeps the name it had.

**`nvs.toml` writes `[queue] connection = "main"`, `workers = 1`.** Stage 8's fixture is past the
refusal it failed on and now fails one move later, against a database with no `nvs_jobs` table in it
— that is the applying half above rather than a fixture bug. Beyond it the fixture still needs a
worker for `claimed 1`, `ran` and `retried`.

**`orient.py`'s pack was short in the same place as last time**: `[context] modules` names no
`nvs-config/src/*` and owes `crates/nvs-stdlib/src/queue.rs`, both sliced by hand again. ADR 0084
§§ 2 and 4 were printed and were the right two.

## Next group

**The applying half and the claim, over `crates/nvs-cli/src/queue.rs`, `crates/nvs-db/src/pg.rs` and
`crates/nvs-stdlib/src/queue.rs`.**

- [ ] **`nvs queue migrate` applies what it prints** — ADR 0084 § 2. The command is
      `crates/nvs-cli/src/queue.rs:62`, with the wall stated in that file's module doc;
      `crates/nvs-db/src/pg.rs:385` turns a `[db.<name>]` block into a target and
      `crates/nvs-db/src/pg.rs:586` opens one, and `crates/nvs-cli/src/main.rs:786` is this binary's
      only existing path that runs work inside an `nvs-host` task, which is what a parking stream
      needs. Decide whether `nvs-cli` gains an `nvs-db` dependency or borrows that task, and record
      which in the module doc. This is what puts the fixture's tables on the compose server.
- [ ] **§ 4's claim statement, beside `INSERT`** — ADR 0084 § 4. `crates/nvs-stdlib/src/queue.rs:240`
      is `INSERT`, and `crates/nvs-stdlib/src/queue.rs:171` is the DDL its columns come from —
      `claimed_at` and the `jobs.due` index exist for this statement to read. `for update skip
      locked` on PostgreSQL, `now - visibility` against `claimed_at` for the expiry. It has no caller
      until the worker lands, so land it *with* the worker: a `const` nothing reads is a warning.
- [ ] **The worker `[queue] workers` starts** — ADR 0084 §§ 4-6. Where an in-process worker is
      started at boot is undecided and is this slice's first move; `crates/nvs-stdlib/src/queue.rs:1`
      says why the connection has to be the request's, which a worker has not got.

## Backlog

- `Core\Queue`'s gaps 1, 2 and 5 — shape parameters, `secret` in `$args`, four drivers with no
  statement path — `crates/nvs-stdlib/src/queue.rs` § *Known gaps*.
- `[context] modules` owes `nvs-config/src/*` and `crates/nvs-stdlib/src/queue.rs` —
  `docs/agent/loop-goal.toml`.
- § 6's dead-letter move, which is the only writer of the `errors` column the DDL now creates —
  `docs/adr/0084-durable-background-jobs.md` § 6.
- Stage 9's `nvs check` literal-query diagnostics — `docs/agent/loop-goal.toml`.
