# Handoff

## State

**Goal 6, Stage 5: ADR 0105 § 4's delegation landed, and Stage 5's acceptance check passes again.**
`Core\Request\Part::saveTo(string $path, {max?, overwrite?})` is the member
(`crates/nvs-stdlib/src/request.rs:704` is the row, `crates/nvs-stdlib/src/request.rs:837` the card,
`crates/nvs-stdlib/src/request.rs:2213` the body). It is a
delegation and not an implementation: it builds the same `PART_CONTENT` walk `content()` answers
with and hands it to `crate::io::stream_to_disk` under **`WRITE_STREAM_OPTIONS` itself** rather than
a second pair of options, so § 4's two rules — `overwrite` defaulting to `false`, a failure removing
the partial file — are this member's without being restated anywhere.

**`stream_to_disk` and `write_chunk` now take the member name** (`crates/nvs-stdlib/src/io.rs:2811`)
instead of reading `WRITE_STREAM`, so a program that called `saveTo` is never told about a member it
did not name. § 3's identity check runs **before** the destination is created, which is the half
worth the case: `stream_to_disk` opens the file before it pulls a chunk.

**`examples/upload.nvs` runs the walk it is named for.** It had simulated an upload with two
`Core\IO::write` calls and never named `Core\Request::files()`; it now writes the receiving end as a
handler does and runs it to the refusal a program answering no request gets. The check's `want` was
`parts=2 / field=title / file=report.pdf`, which no `nvs run` leg can ever produce — fixed in both
`docs/agent/loop-goal.toml` and `docs/agent/goals/6-server.toml`, with the reason in a comment.

**A program still cannot hand `$part->content()` to `Core\IO::writeStream`** — see the playbook
bullet; that is a `nvs-types` conformance question, not this module's.

## Next group

**ADR 0105's server-side half — the six tests the `nvs-server (ADR 0105, whole)` check names and
the tree does not have.** One file set: `crates/nvs-server/src/body.rs`, `docs/agent/loop-goal.toml`,
`docs/agent/goals/6-server.toml`.

- [ ] **Split the check before writing anything for it.** Of its seven names, exactly one is landed
      — `a_part_is_a_file_part_iff_content_disposition_carries_a_filename`, at
      `crates/nvs-stdlib/src/multipart.rs:847` — so it is failing under `-p nvs-server` for the
      playbook's *wrong crate* reason and not for being unwritten. Move that name to the
      `nvs-stdlib` check at `docs/agent/loop-goal.toml:3355`, and the same rows in
      `docs/agent/goals/6-server.toml:3352`.
- [ ] **`files_is_a_lazy_iterator_and_the_only_way_to_receive_an_upload` and
      `there_is_no_temp_path_and_no_move_uploaded_file`** — ADR 0105 §§ 1-2 over a real multipart
      body at the two-task seam: `crates/nvs-server/src/body.rs:104` (`of`),
      `crates/nvs-server/src/body.rs:160` (`Supply`), `crates/nvs-server/src/body.rs:249` (`Pull`).
- [ ] **`a_part_is_consumed_by_read_all_by_iteration_or_by_save_to`** — § 3's two consumers plus
      § 4's, over one body, now that all three exist. Same seam:
      `crates/nvs-server/src/body.rs:249`.

## Backlog

- `Core\Request\PartContent` and `BodyStream` are not accepted where `Iterable<bytes>` is declared,
  so § 4's "one implementation" is unreachable from a program — `crates/nvs-types` conformance.
- `body_stream_is_exclusive_with_body_and_with_files` and
  `an_upload_total_over_the_cap_is_refused_before_dispatch` — `docs/agent/loop-goal.toml:3365`.
- `a_non_file_part_is_buffered_into_post` names `$_POST`, which ADR 0012 removed; check what it
  should assert before writing it — `docs/agent/loop-goal.toml:3365`.
- `clientIp`, `scheme`, `host`, `mount` and `route` stay known gaps of
  `crates/nvs-stdlib/src/request.rs`'s module doc.
- ADR 0105 § 3's code block writes `max: "200M"` and `max: "50M"`, which no longer compile; two
  landed cases cite that spelling as prose, so changing it is a three-file edit.
