# Handoff

## State

**Stage 13 is closed.** [ADR 0129](../adr/0129-password-verify-reads-a-stored-bcrypt-hash.md)'s read
roster is on disk in `crates/nvs-stdlib/src/password.rs`: `Core\Password::verify` reads a PHP-stored
bcrypt hash under `$2y$`/`$2a$`/`$2b$` through the `bcrypt` crate, refuses a stored cost above 17
before a round runs, and `::needsRehash` answers `true` for every bcrypt row, so the login-time loop
is the migration path. `$2x$` and everything else outside the roster still throw `LogicError` with
the message the landed refusal case pins. Four unit tests and
`tests/conformance/core/password-verify-reads-a-php-stored-bcrypt-hash-and-upgrades-it.nvst` cover it;
`cargo deny check` is green on all four legs after a yanked `chacha20 0.10.1` was bumped to 0.10.2.

**Goal 4's `check-migration --min 74` gate stays green** — 926 of 1,152 names classified, 80%, 226
open. What is left is one domain: `pg_*` (120) and `mysqli_*` (106), which
[ADR 0067](../adr/0067-core-db.md) owes an audited row each, and `## Not yet classified` names only
that. Nothing about § 15's stdlib half moved: the compile-time half of ADR 0112 is still absent
(`crates/nvs-stdlib/src/cap.rs`'s module doc owns what that costs), and
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is still 41 keys.

## Next group

**All three slices are `docs/spec/02-php-migration.md` alone**, appending a `##` section before
`## Not yet classified` and trimming its closing paragraph when the last one lands.
[01 § 18](../spec/01-core-library.md) is the reference the cells name — a full signature roster, so
these cells can name `Core\Db::…` members directly and `check-migration.py` validates every one of
those spellings. Run it after each family, not at the end.

- [ ] **Migration rows: `mysqli_*`, the connection and statement half** (~55). `mysqli_connect`/
      `_init`/`_real_connect`/`_options`/`_ssl_set` against ADR 0067's connection naming and
      memoization and its root-owned `[db.<name>]` block, the `_prepare`/`_stmt_*` roster against
      "there is no `prepare`", and `_query`/`_real_query`/`_multi_query`/`_next_result` against the
      one query member. `docs/spec/02-php-migration.md:1375`, `docs/spec/01-core-library.md:1133`.
- [ ] **Migration rows: `mysqli_*`, the result and transaction half** (~51). `_fetch_*`, `_num_rows`,
      `_affected_rows` and `_insert_id` against § 18's result rows, and
      `_begin_transaction`/`_commit`/`_rollback`/`_autocommit` against ADR 0067's transaction shape —
      one closure, no autocommit lever. `docs/spec/02-php-migration.md:1375`,
      `docs/spec/01-core-library.md:1133`.
- [ ] **Migration rows: `pg_*`** (120), the largest single family left and worth splitting in two if
      the first half spends the budget: the connection and query rows against § 18's entry points,
      then `pg_escape_*` against ADR 0024 § 3's launderers and `COPY`/`pg_lo_*` against what
      `Core\Db` does not carry. `docs/spec/02-php-migration.md:1375`,
      `docs/spec/01-core-library.md:1133`.

## Backlog

- The compile-time half of ADR 0112 (`Core\Cap`), owned by `crates/nvs-stdlib/src/cap.rs`'s module doc.
- `spec-members-part-two-outstanding.txt`'s 41 keys, none of them goal 4's — `docs/plan/m8.md`.
- `Core\Db`'s own driver matrix needs a reachable Docker daemon — the plan's `Blocking` field.
- ADR 0129 § 5's truncation edge has no `.nvst` of its own; the module doc states it, nothing pins it.
