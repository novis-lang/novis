# Handoff

## State

**`Core\Db::open` is live, and `Db\Settings` is the first shape parameter in the registry.**
`nvs_stdlib::db::SETTINGS` is ADR 0135 § 1's two arms — the server arm's ten fields, the SQLite
arm's `path` — and § 3's merged twelve slots arrive at `nvs_core_db_open` as one ordinary
`args: [12]`. ADR 0067 § 3's asymmetry is now stated on both sides: `connect` pre-approves an
operator-written endpoint through `address_of`, and `open` asks `db.open` about the host and then
puts the resolved address through ADR 0058 § 3's table. `nvs_runtime::capability::pinned_address`
is that second half split out of `pin_host`, so the range rule has one home and `open` does not
have to demand `net.connect` as well.

**Two enums § 18 declares are registered for the first time**: `Core\Db\Driver`, whose cases are
what make the two arms disjoint, and `Core\Db\Tls`, whose weaker three are **refused at the call**
rather than honoured — this runtime opens every TCP connection at `VerifyFull` and `settings_tls`
words that refusal.

**Two halves of `open` are still owed and both are in `nvs_stdlib::db`'s known gap 1.** It files
its connection with no lease, so § 13's pool never sees it: that section's key is a hash of every
settings field and `nvs_runtime::pool::Ticket::for_block` takes a block *name*. Within a request
§ 2's memo does hold, keyed on `settings_key`'s hash under a NUL-prefixed name no config block can
have. And ADR 0135 § 2's *exactly one arm accepts it* is not the checker's rule: a literal is
checked against the **merged** list, so `E0402` fires only for a key **every** arm requires
(`driver`), and a missing `host` reaches the helper as a `Tag::Null` — `settings_text` throws a
catchable `RuntimeError` there rather than a `FATAL`, which is the honest reading until § 2 lands.

## Next group

**ADR 0135 § 2's arm selection, then `open`'s pool ticket. File set:
`crates/nvs-types/src/ty.rs` with `crates/nvs-types/src/core_lib.rs` and
`crates/nvs-types/src/expr/args.rs`, then `crates/nvs-runtime/src/pool.rs` with
`crates/nvs-stdlib/src/db.rs`.** The first two items are one file set and one build; the third is
its own.

- [ ] **Arm selection: check a literal against one arm, not the merged list** (0135 § 2).
      `crates/nvs-types/src/ty.rs:292` is `Ty::CoreShape` and its known gap;
      `crates/nvs-types/src/core_lib.rs:628` is `shape_fills`, which is where the arms are
      flattened and so where the per-arm list has to survive to. "Exactly one arm accepts it" —
      zero accepting is the call site's error, two is a registry bug the static test already
      refuses.
- [ ] **The two refusals that rule makes writable, as `.nvst` cases** (0135 § 2, 0067 § 3).
      A `host` beside `Driver::Sqlite` is a compile error, and a server literal missing `host` is
      `E0402` rather than the runtime throw `crates/nvs-stdlib/src/db.rs:3100`'s `settings_text`
      words today. Both are one edit away from
      `tests/conformance/core/db-open-refuses-a-settings-literal-that-names-no-driver.nvst`.
- [ ] **§ 13's pool for `open`, keyed on the settings hash** (0067 § 13).
      `crates/nvs-runtime/src/pool.rs:152` is `Ticket::for_block`, which needs a sibling taking a
      key rather than a block name; `crates/nvs-stdlib/src/db.rs:3129` is `settings_key`, already
      the hash § 13 asks for, and `crates/nvs-stdlib/src/db.rs:3281` is the `None` lease to
      replace.

## Backlog

- `Core\Db\Tls`'s weaker three are refused, not honoured — if a `Tls` mode is ever to mean
  something, it is `nvs_db::PgTarget`/`MySqlTarget` that gain the knob (`nvs_stdlib::db::TLS`).
- MariaDB and SQL Server settings reach `open`'s refusal, not a handshake — `nvs_stdlib::db` known
  gap 2.
- `stream`/`streamAs` are the last two § 18 members with no row
  (`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`).
- `Core\Db::open`'s `timeZone` is resolved to a fixed offset at open time
  (`crate::time::zone_offset_now`); a session that outlives a DST change keeps the offset it
  opened with, as a `[db.<name>]` block's own `time_zone` already does.
