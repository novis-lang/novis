# Handoff

## State

**`Core\Mail` is on disk and stage 9's check for it is green.** `crates/nvs-stdlib/src/mail.rs` is
one row — `Core\Mail::send(string $endpoint, array<string> $to, string $subject, string $text,
{cc?, bcc?, replyTo?, html?}): void` — plus a plaintext SMTP session over `nvs_host`'s parking
stream. The roster was this slice's decision under ADR 0063; that module's own doc is the home of
all four arguments for it, and none is restated here.

**The capability is new: `mail.send`.** ADR 0082 § 2 specifies it, not the `net.connect` the last
handoff predicted — see the playbook bullet this session added. It is `nvs_config::Cap::MailSend`
with `Scope::Name`, granted per `[mail.<name>]` block exactly as ADR 0067 § 3 grants `db.connect`,
and the address inside a granted block is deliberately **not** put through ADR 0058 § 3's denied
ranges.

**Two gaps are recorded rather than worked around**, both in `mail.rs`'s module doc: there is no TLS
under `nvs-stdlib`'s sockets (`crate::http`'s transport is plaintext too), so a `[mail.<name>]`
block that names `user` or `password` is **refused at the send** rather than authenticated in the
clear; and attachments are composition, which ADR 0082 § 2 puts in `nvs/web`.

**The pack still did not print ADR 0024 § 5**, and it did not print ADR 0082 § 2 either until it was
sliced by hand — that section is the specification for every remaining stage-9 item.
`[context] adrs` in `docs/agent/loop-goal.toml` wants `0024 § 5` and `0082 § 2`.

## Next group

**`Core\Storage` and `Core\Cldr` — stage 9's two remaining acceptance checks — over the same file
set this session used: a new module under `crates/nvs-stdlib/src/`,
`crates/nvs-stdlib/src/registry.rs` and `crates/nvs-stdlib/src/lib.rs`.**

- [ ] **`Core\Storage`'s row and its `fs.*` gating** — ADR 0082 § 2's row says "local-filesystem
      object storage over ADR 0051's existing `fs.*` capabilities", so it declares **no new `Cap`**:
      the rows go beside this session's at `crates/nvs-stdlib/src/registry.rs:1231` and
      `crates/nvs-stdlib/src/registry.rs:1515`, the module beside `mod mail;` at
      `crates/nvs-stdlib/src/lib.rs:241`, and the address arm at
      `crates/nvs-stdlib/src/lib.rs:360`. Every door is `nvs_runtime::capability`'s existing
      `open_read`/`create`/`remove_file`, which is what makes the check's name true.
- [ ] **`storage_over_local_disk_is_gated_on_the_same_fs_capability`** — the `#[test]` the driver
      names, in the new module beside `mail_sends_against_an_operator_named_endpoint_and_no_other`
      at `crates/nvs-stdlib/src/mail.rs:869`, which is the shape: assert over the rows and the
      `CAPABILITIES` table, not over a fixture.
- [ ] **`Core\Cldr::pluralCategory`** — ADR 0082 § 2's last row, one member over the data
      `crates/nvs-stdlib/src/cldr.rs:1` already carries. That module has no `CLASS` and no
      `address()` yet, so it owes all five edits plus three `.nvst` cases; the check is
      `plural_category_answers_from_the_carried_cldr_data`.

## Backlog

- `validate_launders_only_what_it_actually_validated` — stage 9's third open check; `Core\Validate`
  is six `Qual::Neutral` predicates today (`crates/nvs-stdlib/src/validate.rs:175`).
- ADR 0024 § 5's third piece, the sink's own escape-and-lift, waits on an HTML response — goal 6's.
- TLS for `nvs-stdlib`'s sockets: `Core\Http\Client`'s HTTPS and `Core\Mail`'s `STARTTLS`/`AUTH` are
  one dependency, owned by ADR 0074.
- `Core\IO::truncate` and `::lock`, per the plan's stage 2 line.
- `Core\Cli::displayWidth`, per the plan's stage 3 line.
