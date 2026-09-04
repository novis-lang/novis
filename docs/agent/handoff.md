# Handoff

## State

**Goal 6. The driver's Stage 2 blocker is closed**: all four names under the `nvs-server (the
connection seam)` check are now `#[test]`s in `crates/nvs-server/src/serve.rs` — ADR 0138 § 1's three
properties at `serve.rs:1634`, `:1740` and `:1901`, and Stage 2's item 2 at `:1936`. None of the four
existed; the driver's report named only the first, which is now a playbook bullet.

**ADR 0105 § 1 is whole in `nvs-stdlib`.** `there_is_no_temp_path_and_no_move_uploaded_file` landed at
`crates/nvs-stdlib/src/request.rs:3138`: the host-path sweep is over *spellings* because no `CoreTy`
says "path", and the load-bearing half is the exact-equality one — a path crosses the two rosters once
and inward, as `saveTo`'s parameter.

**`Core\Request::post()` has not landed**, and that is what the group's last name actually needs — see
the second item below. `crates/nvs-stdlib/src/multipart.rs:227` says so in the `#[allow(dead_code)]`
reason on `fields()`.

`orient.py` did not print ADR 0138 § 1 or ADR 0105 § 3, both of which this session needed: `[context]
adrs` in `docs/agent/loop-goal.toml` wants `0138` (§ 1 and *Verification*) and `0105` § 3 added.

## Next group

**ADR 0105's last two `nvs-stdlib` names.** One file set: `crates/nvs-stdlib/src/request.rs`,
`crates/nvs-stdlib/src/multipart.rs`, `docs/agent/loop-goal.toml`, `docs/agent/goals/6-server.toml`.

- [ ] **`a_part_is_consumed_by_read_all_by_iteration_or_by_save_to`** — § 3's three consumers, asked
      the *agreement* question: the same part's bytes come back the same way through each, and a part
      is consumed by these three and by nothing else. `crates/nvs-stdlib/src/request.rs:686` is the
      `content` row, `crates/nvs-stdlib/src/request.rs:695` is `readAll`'s and
      `crates/nvs-stdlib/src/request.rs:704` is `saveTo`'s, and
      `crates/nvs-stdlib/src/request.rs:3028` is the walk fixture to copy — `uploading`, `Chunks`, and
      the `lend`/`spend` pair a virtual call's receiver owes. **Decide the `saveTo` leg first**: it
      delegates to `Core\IO::writeStream` (`crates/nvs-stdlib/src/request.rs:715`), so it wants a real
      destination and whatever that member asks of a `-p nvs-stdlib` context. That unknown is what
      stopped this session taking the item, not the assertion.
- [ ] **`a_non_file_part_is_buffered_into_post`** — a *feature* slice and not a test one. § 2's parse
      half is already pinned by `a_part_is_a_file_part_iff_content_disposition_carries_a_filename`
      (`crates/nvs-stdlib/src/multipart.rs:847`); what is open is the member that reads the buffer.
      `post()` needs the five edits beside `Core\Request`'s row block at
      `crates/nvs-stdlib/src/request.rs:270`, reading `Multipart::fields()`
      (`crates/nvs-stdlib/src/multipart.rs:231`), and its first call is what deletes that method's
      `#[allow(dead_code)]`.

## Backlog

- `Core\Session` has no row in `nvs_stdlib::registry` — Stage 5's item 14, `docs/agent/loop-goal.toml`.
- ADR 0105 § 5's refusal at the door, the one check of the three that is genuinely `nvs-server`'s —
  `docs/agent/loop-goal.toml`.
- Raw body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by `docs/plan/m7.md`.
- M7's load-bearing upload case: a body far larger than any in-memory bound received at bounded
  resident memory — `docs/plan/m7.md` *Verify*.
