# Handoff

## State

**ADR 0103 § 7 is landed: a secret arrives as a file whose content is the value.**
`crates/nvs-config/src/secret.rs` owns every rule about it — the whole content minus one trailing
`\n` (and the `\r` before it), nothing else trimmed, and `E0608 E_BAD_SECRET_FILE` for a pair set
twice, an empty file, a whitespace-only one, one over `MAX_SECRET_BYTES` (64 KiB) or one that is not
UTF-8. It runs once over the *flattened* tree from `resolve.rs:210`, because which of two
`password_file` assignments is in force is a question only the merge has answered; it takes the
merge's `origins` map, so § 5 resolves a relative secret path against the file that wrote it and a
refusal names that file.

Three things this cost, each already spent: `Files` gained `read_bytes` (a non-UTF-8 secret is a
refusal about *content*, and a reader that decoded first could only report "cannot read") and
`exposure` (§ 7's advisory half, `trust::exposure` on `Disk`); `Resolved` gained `warnings`, which
is where `W1005 W_SECRET_FILE_READABLE` lands and where every future advisory should. Integrity is
enforced and confidentiality only advised because a Compose secret is mounted `0444` — `trust.rs`'s
module doc owns that split. The Windows half of `exposure` is the same descriptor read as `check`,
asking `FILE_READ_DATA` of § 6's five principals.

`[db.<name>] password` is the only secret pair, spelled out in `secret::materialize` because
`directive.rs`'s registry has no secrecy field; when it grows one that loop becomes a sweep.

34 tests in the crate (9 new in `tests/secret.rs`, which holds bytes rather than text so the UTF-8
question can be asked at all). The acceptance check still fails on `examples/config.nvs` —
`Core\Config` has no `get` — which is Stage 3's snapshot and Stage 4's members, unwritten.

**One finding the next slice needs:** `[[app]]` layering cannot live in `resolve()`. § 2 layers
blocks matching *the entry file*, and `nvs run <file>`'s entry is not known at resolve time, so the
layering belongs to whatever builds the per-app snapshot. To reuse `merge_table`'s later-wins and
its `Override` records — which § 2 explicitly asks for ("every override is reported the same way") —
`Resolved` will have to keep the merged `toml::Table` and the `origins` map that `resolve()` now
takes and drops at `resolve.rs:208`.

## Next group

**The per-app block, on the path comparison the boundary already wrote.** One file set:
`crates/nvs-config/src/resolve.rs`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/trust.rs`, `docs/adr/0104`.

- [ ] **`[[app]]` matching.** ADR 0104 §§ 1-2 — a block matches on a canonicalized `root` prefix at
      component boundaries or an exact `entry`, so a path reaching a root through `..` or a symlink
      does not match. The one implementation of that comparison is `trust::check`'s canonical return
      (`crates/nvs-config/src/trust.rs:83`), which is why the goal's standing decisions forbid a
      second; the block is `crates/nvs-config/src/tree.rs:138` (`App`, `root` at `:140`, `entry` at
      `:142`), and two blocks with the same `root` are a duplicate and refused. Next free code in the
      band is `E0609`.
- [ ] **`[[app]]` layering.** ADR 0104 § 2 — every matching block applies, least-specific first, an
      `entry` match last, each override reported as § 3's are. Read the State note above first: this
      needs the merged table and `origins` kept on `Resolved` (`resolve.rs:145`, dropped at `:208`)
      so `merge_table` (`resolve.rs:493`) does the layering rather than a second field-by-field
      merge over the typed tree.
- [ ] **A case that a symlinked include cycle is refused by name.** `resolve.rs:224`'s `same_file`
      compares canonical paths, so the ring closes on a name — untested against a real filesystem.
      `tests/trust.rs` is the shape (`cfg(unix)`, a scratch dir); a Windows symlink needs privileges
      the runner may not have, so gate it.

## Backlog

- `Core\Config::get`/`set` and the snapshot — the acceptance check's actual blocker (`docs/plan/m6.md`).
- `[db.<name>] path` (SQLite) is documented as § 5-resolved and nothing resolves it (`tree.rs:423`).
- `[context] modules` names `crates/nvs-host/src/budget.rs`, which matched nothing — `orient.py`
  said so on this run (`docs/agent/loop-goal.toml`).
- ADR 0103 § 7 says "a `W1xxx` warning"; the code is now `W1005` and the ADR does not name it.
- `directive.rs` has no secrecy field, so `secret.rs` holds the pair list (its module doc).
- Item 18's `Core\Secret::reveal()` is still not in the registry (`docs/implementation-plan.md`).
