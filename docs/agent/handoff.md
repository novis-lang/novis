# Handoff

## State

**`Core\Storage` is cased to the depth its module doc argues for.** Three conformance cases
landed over `crates/nvs-stdlib/src/storage.rs`, and no Rust changed: every rule they pin was
already on disk, and what was missing was the case that would notice it breaking.

- **The key grammar, bounded on both sides.** Every refusal is named beside the nearest accepted
  spelling — `a.hidden` beside `.hidden`, `note.txt` beside `note/txt`, `cafe` beside `café`, and
  255 bytes beside 256. The length bound is asked through `list`'s `prefix`, which is `key_of`'s
  own grammar for anything non-empty and builds no path, so the case does not depend on the host's
  own limit on a filename's length.
- **The four rows agree about a missing object**, each in its own vocabulary: `get` answers `null`,
  `delete` fails, `list` omits, and `put` under `overwrite: false` succeeds. The two mutating rows
  put back what they took, so the eight questions are eight readings of two disk states.
- **The two configuration refusals.** No block, a block with no `root` and `root = ""` are one
  sentence, because the difference between them is invisible to a program. A `root` that is there
  and is not a directory is an `IOError` instead, and the case names the class rather than the
  wording — the sentence inside it is the host's own.

The rules themselves live in that module's doc §§ *A key is not a path*, *What `list` answers over*
and *Known gaps*, which stay their only home.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are `Core\Request` and its neighbours
and are goal 6's, so this check cannot pass inside this goal and is not a regression.

**`cargo deny check`'s `advisories` leg is red on a yanked `chacha20`** that predates this goal and
arrives through `rand`; `licenses`, `bans` and `sources` are green. See the playbook bullet.

## Next group

**`[log] target` is the last owed piece of stage 7 — the floor and `Core\Log::write` are already
one serialiser and are still two destinations. The file set is `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-runtime/src/floor.rs`, `crates/nvs-stdlib/src/log.rs` and
`crates/nvs-config/src/tree.rs`; ADR 0092 § 2 and ADR 0020 § 6 specify it.**

- [ ] **One place reads `[log] target`, and both writers reach it.** `Core\Log::write` writes
      through `ctx.write_output` and the floor writes its own way; the destination the config names
      — `stderr`, `file:<path>` or `syslog` — is read nowhere. `crates/nvs-runtime/src/ctx.rs:250`
      is the doc comment that already says what `file:…` selects "once something reads that";
      `crates/nvs-stdlib/src/log.rs:202` and `crates/nvs-runtime/src/floor.rs:172` are the two
      call sites.
- [ ] **An unspelled target is refused where it is written, not where it is used.**
      `crates/nvs-config/src/tree.rs:353` is the `Option<String>` today, so a typo reaches the sink
      as a filename. ADR 0095's direction, and the same shape `[mail.<name>]` uses.
- [ ] **A case that both writers land in the target the deployment named**, which is § 6's "two
      writers are one serialiser" asserted about *where* rather than about the record's shape.
      `crates/nvs-stdlib/src/log.rs:365` is where the floor's line is built beside the member's.

## Backlog

- Encryption without authentication for `Core\Mail` needs an `nvs.toml` key, beside
  `nvs_host::tls`'s unlanded anchor bundle — `crates/nvs-host/src/tls.rs` module doc.
- `AUTH LOGIN` and `XOAUTH2` have no spelling; `PLAIN` over TLS is the whole roster —
  `crates/nvs-stdlib/src/mail.rs` module doc.
- `[context] modules` does not select `nvs-runtime/src/terminal.rs` — `docs/agent/loop-goal.toml`.
- `[context] playbook` filters to the item's anchor paths, so case-authoring bullets never print —
  `docs/agent/loop-goal.toml`.
- `cargo deny check advisories` is red on a yanked `chacha20` via `rand` — `deny.toml`.
- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's —
  `docs/agent/loop-goal.toml`.
