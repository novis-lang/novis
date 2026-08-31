# Handoff

## State

**Stage 2's *Handles* bullet has opened, and it outranked the handoff's own next group** — the
driver's acceptance report had advanced to `nvs-stdlib (Core\IO) [2 filesystem]`, where five of
seven named tests had never existed because `Core\IO::open` did not. Two of the five are closed now.
The playbook bullet under *Tooling* is the home of how to read that report; do not treat an early
stage in it as stale.

**`Core\IO::open` answers a `Core\IO\File`, and the descriptor lives in the request.** The object's
first slot is a key into `nvs_runtime::Ctx`'s own `open_files` table and its second is the path, so
every refusal names the file. `Ctx::hold_open_file`'s doc comment is the one home of the accounting
and of why a key is never reused; `Core\Script\Handle` is the same shape and landed first, and
`crates/nvs-stdlib/src/instance.rs`'s first decision is why a `std::fs::File` cannot be in a slot.

**The mode decides the capability, which is new.** `nvs_runtime::capability::Access` is the runtime's
own spelling of the four `Core\IO\FileMode` cases, and `capability::open` asks `fs.read` for `Read`,
`fs.write` for `Write`/`Append` and **both** for `ReadWrite`. `registry::CAPABILITIES` has one cap
per member and cannot express that, so `open`'s row names the stronger one and its comment says why;
`Core\IO\File`'s three members carry `None` rows, because the descriptor was checked at the door.

**What stage 2 still owes** is `writeStream` (ADR 0105 § 4 — `overwrite` defaults to `false`, and a
write that fails mid-stream removes the partial file), the `no_member_dispatches_on_a_uri_scheme`
test over landed work, and the rest of § 14's handle roster (`readLine`, `seek`, `tell`, `truncate`,
`flush`, `lock`) plus `stdin`/`stdout`/`stderr`. Stage 8's reflective property write — the group the
previous handoff named — is untouched and moves to the backlog with its finding intact.

## Next group

**The rest of stage 2's named check, over `crates/nvs-stdlib/src/io.rs`, `crates/nvs-runtime/src/capability.rs`
and `crates/nvs-stdlib/tests/capability.rs` — the same three files this session held. ADR 0105 § 4
and spec § 14 are the specification, and `docs/agent/loop-goal.toml:2218` is the check.**

- [ ] **`Core\IO::writeStream(string $path, Iterable<bytes> $src, {max?, overwrite?})` — the five
      edits** over `crates/nvs-stdlib/src/io.rs:1126`'s sibling `write`, with the door extended at
      `crates/nvs-runtime/src/capability.rs:225`. The `Iterable<bytes>` is consumed through
      `nvs_runtime::sequence::ITERATE` the way `crates/nvs-stdlib/src/io.rs:655`'s neighbour
      `Core\IO\Lines` answers one; ADR 0105 § 4 is the contract. Write to a sibling temporary and
      rename, so "removes the partial file" is the failure path doing nothing rather than a cleanup
      that can itself fail.
- [ ] **`write_stream_defaults_to_no_overwrite` and
      `a_write_stream_that_fails_midway_removes_the_partial_file`**, beside this session's two at
      `crates/nvs-stdlib/tests/capability.rs:427`, with `ctx_reading_and_writing` already there for
      the grants. Both are named by the acceptance check and neither exists.
- [ ] **`no_member_dispatches_on_a_uri_scheme`** — ADR 0052, over landed work only: a sweep asserting
      that no `Core\IO` body branches on a `://` prefix and that no registry row anywhere spells a
      scheme. Same file, `crates/nvs-stdlib/tests/capability.rs:427`.

## Backlog

- **Stage 8's reflective property write** — ADR 0014's hook is emitted at the *call site* by
  `crates/nvs-ir/src/lower/expr.rs:3539`, so a native member cannot reach it and
  `crates/nvs-runtime/src/object.rs:2481` is not where it lives; the decision is still open.
  `docs/agent/loop-goal.toml:2474` names the test.
- Stage 7 still owes the schema-identity test — application code and the engine floor produce
  schema-identical records (`docs/plan/m8.md`, ADR 0020's M7/M8 list).
- § 14's remaining handle members and the standard streams, per `crates/nvs-stdlib/src/io.rs:655`'s
  own doc comment.
