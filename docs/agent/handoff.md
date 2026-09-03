# Handoff

## State

**M8 goal 5, stage 10.** The driver's failing acceptance check was `check-migration at 94%`, and it
was a coverage floor rather than a missing test: 120 names were unclassified and every one of them
was `pg_*`. `docs/spec/02-php-migration.md` now carries the `pgsql` section — 120 audited rows in
nine subsections — so the table is at **100% of this build's 1,151 inventory functions** and every
`--min` floor in the program is green, goal 6's included.

The header still says `Complete: no` on purpose. That percentage is over the inventory *a build
produced*, and `check-migration.py`'s `UNAUDITED` list is not empty: `mbstring`, `curl`, `intl`,
`gd` and the rest are a known hole that becomes rows the day the inventory is regenerated against a
build loading them. Flipping the header is goal 6's stop condition, not this goal's.

Rows were written against the oracle build's own reflection output, not from memory — the playbook
bullet added this session has the shape. Outcomes follow the `mysqli` section's precedents exactly
where a name has a twin there (`fetch_array` and `fetch_row` dropped under R17, `fetch_assoc`,
`fetch_all`, `fetch_object` and the column readers members), a deprecated alias takes its modern
spelling's outcome, and § 12's three deferrals — `COPY`, `LISTEN`/`NOTIFY`, large objects — are
`dropped` rows naming what replaces them today rather than `open` ones.

**The `Core\Taint::assertTrusted` group below was not started.** It arrived as this session's item
and the failed check outranked it; nothing about it has changed since the last handoff, including
that `grep` finds no `Core\Taint` anywhere in `nvs-stdlib`, so a diagnostic already names a member
the tree cannot resolve.

**Orientation gap:** `[context]` has no field that can name a `docs/spec/` file — `modules` takes
`crates/**` paths and builds the map from module docs — so a session whose failing check is about
the migration table starts blind and spends five calls re-deriving the table's row format, its
`Core\X::y` validation rule and the `mysqli` precedents. A `docs = [...]` selector, or `modules`
learning to take a `docs/spec/*.md` path, would pay for itself the next time an extension is audited.

## Next group

**One file set: `crates/nvs-stdlib/src/registry.rs` with a new `taint.rs` beside it, widening to
`crates/nvs-types/src/core_lib.rs` on the third.**

- [ ] **Decide whether `Core\Taint::assertTrusted` earns a registry row, and record the decision.**
      ADR 0067 § 3 names it as the only way through `Settings.host`, which has no launderer, and
      `nvs_types::expr::args::check_shape_field`'s diagnostic now names it too. ADR 0024 § 3 owns the
      axis. The mark is the real question: `crates/nvs-stdlib/src/registry.rs:152` is `Qual`, whose
      `Launder` doc says a launderer names *the sink it launders for*, and this member launders for
      every sink — the `tainted` twin of `Qual::Reveal`. `crates/nvs-stdlib/src/secret.rs:41` is
      `Core\Secret`, the same escape hatch one axis over and the model to copy, `$reason` included.
- [ ] **If it earns one, write the five edits** — the row, the card, the body, the `address()` arm
      and three `.nvst` cases — modelled on `crates/nvs-stdlib/src/secret.rs:41`, which is the same
      shape one axis over. `crates/nvs-stdlib/src/registry.rs:152` is the mark the row must pick.
- [ ] **Then hold the roster the way the `secret` axis is held**, at
      `crates/nvs-types/src/core_lib.rs:979`, so the two escape hatches are listed by one rule
      rather than by two that can drift apart.

## Backlog

- Flip `**Complete:** yes` in `docs/spec/02-php-migration.md` — goal 6's stop condition, once the
  unaudited extension list is empty or explicitly accounted for.
- `[context]` cannot select a `docs/spec/` file — `docs/agent/loop-goal.toml`.
- The remaining stage 10 conformance cases in that check's `cases` list — `docs/agent/loop-goal.toml`.
- `docs/spec/02-php-migration.md`'s `mysqli`/`pgsql` rows are unchecked against `nvs-stdlib`'s
  registry until that generated layer exists — the file's own header says so.
