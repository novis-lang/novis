# Handoff

## State

**ADR 0067 § 4 now runs `executeMany` as N executions on every driver, and `COM_STMT_BULK_EXECUTE`
is refused with its reason written down.** The decision is § 4's own text (`docs/adr/0067-core-db.md`,
the paragraph after the transaction one) and it is a *rule* rather than a MariaDB special case: a batch
command that cannot reproduce the loop's observable behaviour is not used. Two of the three
divergences are unrepairable inside a driver and that is what settled it — PostgreSQL flushes every
`Bind`/`Execute`/`Sync` in one `wire.send` (`crates/nvs-db/src/pg.rs:2788`), so the loop cannot be
made to stop at a refusal, while a bulk command cannot be made to continue past one; and a set that
answers with rows cannot be routed to the loop in advance, a prepare reporting `0` columns for any
statement whose result set depends on the data. The third — an aggregate count rather than a sum — is
provenance only and would have been acceptable alone. The price is N round trips, recorded in § 4 the
way § 1 records the prepare's.

**The trade is priority 2 over priority 3 and it is large in one direction**: a 1000-set batch keeps
paying 1000 round trips where one command would have done. That is the ordering applied rather than a
judgement, and § 4 names the single fact that reopens it — whether a server under
`MARIADB_CLIENT_BULK_UNIT_RESULTS` *continues* past a refused set, which needs a real server to
establish and is not knowable from the crates.

`MARIADB_CLIENT_STMT_BULK_OPERATIONS` **stays claimed** and needed no undoing: `maria.rs`'s
`EXTENDED_CAPABILITIES` doc already said the bit is "spent later or not at all", and the word is what
the server answers in too, so the intersection is still how the driver learns which server it reached.

**Stage 6's `nvs-db (per-driver specifics)` check should now be green.** Its other two names already
existed — `crates/nvs-db/tests/handshake.rs:930` and `crates/nvs-db/src/maria.rs:592` — and the third
was this session's; `execute_many_uses_the_bulk_protocol_on_mariadb` is renamed in
`docs/agent/loop-goal.toml` to `execute_many_on_mariadb_is_n_executions_and_not_the_bulk_command`,
with a comment saying the ADR won over the file so nobody restores it.

**The goal's one open check is now `a_db_open_target_in_a_denied_range_fails`**, and it is the next
group below rather than a slice: it waits on `Core\Db::open`, which waits on a registry type for a
shape *parameter*.

## Next group

**One decision that is bigger than this goal, then the type, then the member. File set:
`crates/nvs-stdlib/src/db.rs` and `crates/nvs-stdlib/src/registry.rs`.**

**Read this first:** the decision is a *language-surface* one — how every future fixed-key shape
parameter is passed across the ABI — and goal 5's standing list spends its one ADR slot on ADR 0132.
Standing decision 1 ("decide and record; never `BLOCKED` for a design call") still covers it, so take
it; but say in the ADR that it was opened from goal 5 and re-check the next free number immediately
before creating the file, because another agent derives the same answer from the same directory.

- [ ] **Decide how a fixed-key shape parameter reaches a `Core` member, and write the ADR.**
      `crates/nvs-stdlib/src/db.rs:66`'s known gap 1 states the hole: `CoreTy::Options`
      (`crates/nvs-stdlib/src/registry.rs:594`) is a *trailing* bag, flattened to one ABI argument per
      option and optional by construction, and no row has ever declared a fixed-key shape *argument*.
      § 18's `open(Db\Settings $settings, {shared?: bool})` additionally needs a discriminated union
      of two shapes over ADR 0047's enum-case types — the SQLite arm has a `path` and no `host` — so
      the decision is two questions, and whether the union is one `CoreTy` or a pair is the second.
- [ ] **Add the `CoreTy` variant and its ABI.** `crates/nvs-stdlib/src/registry.rs:180` is the enum
      and `:594` its `Options` neighbour to shape it against; the arity rule that an options bag
      flattens to one argument per option is what the new variant has to answer for itself.
- [ ] **`Core\Db::open`'s five edits, and `a_db_open_target_in_a_denied_range_fails` with them.**
      `crates/nvs-stdlib/src/db.rs:251` (`SCOPE_SLOT`) is where the capability half already sits; the
      test is stage 2's last open check and ADR 0067 § 3 is what it asserts — `db.open`'s targets are
      program-supplied and stay subject to ADR 0058's denied ranges in full, unlike a `connect`-named
      endpoint.

## Backlog

- MariaDB's `RETURNING`, the one place the two drivers still part — `crates/nvs-db/src/maria.rs:415`.
- Whether `BULK_UNIT_RESULTS` continues past a refused set; the one fact that reopens ADR 0067 § 4.
- ADR 0067 has no `Validated by:` field; adding one means naming every test holding a § claim, not one.
- Stage 7's pool names and stage 6's matrix legs — `docs/agent/loop-goal.toml`, stages 6 and 7.
