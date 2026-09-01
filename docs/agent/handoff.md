# Handoff

## State

**`Core\Storage` is on disk and stage 9's check for it is green.** `crates/nvs-stdlib/src/storage.rs`
is three rows — `put(string $disk, string $key, bytes $contents, {overwrite?})`, `get(...): ?bytes`
and `delete(...): void` — over `nvs_runtime::capability`'s existing `create`, `exists`, `open_read`
and `remove_file`. That module's own doc is the home of every decision behind it and none is
restated here.

**It declares no capability, and that is the point of the row.** ADR 0082 § 2 says "over ADR 0051's
existing `fs.*`", so `registry::CAPABILITIES` carries `FsWrite`/`FsRead`/`FsWrite` for the three
members and there is no `storage.*` grant anywhere;
`storage_over_local_disk_is_gated_on_the_same_fs_capability` asserts that over the registry, against
`Core\IO`'s own rows rather than against a constant.

**One file outside the item's named set was unavoidable:** `crates/nvs-config/src/tree.rs`, because
`deny_unknown_fields` means a `[storage.<name>]` block does not parse until the tree has a type for
it. `StorageDisk` is one `root` field, mirroring `MailEndpoint`.

**Two gaps are recorded rather than worked around**, both in `storage.rs`'s module doc: there is no
`list`, because `nvs_runtime::capability` has no read-directory door and adding one is a
capability-surface decision of its own; and there is no `exists`, because `get` answering `null` is
already that question and a second spelling would be ADR 0063 R20's operation reachable two ways.

**The pack still did not print ADR 0024 § 5**, and `[context] file-set` did not name
`crates/nvs-config/src/tree.rs` for a stage-9 item that adds an operator-named block — every
remaining one of those will need it too.

## Next group

**`Core\Cldr::pluralCategory` — stage 9's last acceptance check — over
`crates/nvs-stdlib/src/cldr.rs`, `crates/nvs-stdlib/src/registry.rs` and
`crates/nvs-stdlib/src/lib.rs`, the same three this session used.**

- [ ] **`Core\Cldr`'s class and its one row** — ADR 0082 § 2's last row, "one member exposing the
      CLDR data `nvs_stdlib::cldr` already holds". **Check that claim first**: today
      `crates/nvs-stdlib/src/cldr.rs:306` is a *date pattern* grammar and its only `pub` item is
      `validate`, so the plural data may not be carried at all and the row's comment may be the
      stale-about-the-tree kind the playbook warns about. The rows go beside this session's at
      `crates/nvs-stdlib/src/registry.rs:1238`, the module beside `mod storage;` at
      `crates/nvs-stdlib/src/lib.rs:261`, and the address arm at
      `crates/nvs-stdlib/src/lib.rs:379`. It needs **no** `CAPABILITIES` row: nothing leaves the
      process.
- [ ] **`plural_category_answers_from_the_carried_cldr_data`** — the `#[test]` the driver's stage-9
      check names, in a `#[cfg(test)]` module beside the member at
      `crates/nvs-stdlib/src/cldr.rs:306`. Assert the categories
      against a locale whose rules differ from English's in a way one table cannot fake.
- [ ] **Three `.nvst` cases**, since `BELOW_THE_FLOOR` in
      `crates/nvs-stdlib/tests/conformance_coverage.rs:268` is empty and the floor is three per
      member. One case may satisfy several members at once, which is how this session's three
      covered nine.

## Backlog

- `Core\Mail` owes TLS and `AUTH PLAIN` — `crates/nvs-stdlib/src/mail.rs`'s module doc.
- `Core\Storage` owes `list`, which owes a read-directory door in `nvs_runtime::capability`.
- `Core\IO` owes `truncate` and `lock` — docs/implementation-plan.md, *Open now*.
- `Core\Log` owes reading `[log] target` — ADR 0020 § 3.
- `Core\Cli` owes `displayWidth` — ADR 0086 § 1.
- Goals 4 and 5 still need a reachable Docker daemon for the shared store and the driver matrix.
