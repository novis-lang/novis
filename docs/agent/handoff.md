# Handoff

## State

**Goal 6, Stage 5. The driver's reported blocker was `examples/session.nvs`, and it was a mis-filed
check rather than work.** Its `want` described a visit counter rising *across requests* — `new
session`, `visits=1`, `visits=2` — which one `nvs run` leg with no cookie round trip cannot print, and
`Core\Session` has no rows in `nvs_stdlib::registry` at all: it is this stage's **item 14** and still
open. The want is now the four lines the fixture does print (ADR 0060 § 1's ring, asserted on both
sides), the cross-request half stays specified by the two names in the `nvs-stdlib (session, and the
tier it may not use)` check, and `loop-goal.toml`'s frozen-output header now names this exception so
it is weighed rather than re-derived.

**ADR 0105's checks are re-filed by crate.** `crates/nvs-server/Cargo.toml` does not name
`nvs-stdlib`, so no test calling `files()` can ever run under `-p nvs-server` — the surface is
`nvs_stdlib::request`. There are three checks now: **what is landed** (green), **§§ 1-3's four open
names** in `nvs-stdlib`, and **§ 5's refusal at the door**, which is genuinely the server's. A name
moves up into the landed check in the commit that writes it, so "did not run" means work.

`files_is_a_lazy_iterator_and_the_only_way_to_receive_an_upload` is landed at
`crates/nvs-stdlib/src/request.rs:3028` — laziness against a body whose second pull never arrives,
and "one door" as a count over `registry::CLASSES` rather than a row read off a line.

## Next group

**ADR 0105's three remaining `nvs-stdlib` names.** One file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/multipart.rs`, `docs/agent/loop-goal.toml`,
`docs/agent/goals/6-server.toml`.

- [ ] **`there_is_no_temp_path_and_no_move_uploaded_file`** — § 1's other half, asked of the rosters
      the way the landed test asks its own: no member on either class answers a host-stored path, and
      no `Core` name spells `move_uploaded_file`. `crates/nvs-stdlib/src/request.rs:654` is `PART`'s
      row block, `crates/nvs-stdlib/src/request.rs:185` is `Core\Request`'s, and
      `crates/nvs-stdlib/src/request.rs:3114` is the `answers` helper to sweep with.
- [ ] **`a_part_is_consumed_by_read_all_by_iteration_or_by_save_to`** — § 3's three consumers, each
      spending the same one pull. `crates/nvs-stdlib/src/request.rs:3292` and
      `crates/nvs-stdlib/src/request.rs:3417` are the two that read a part today;
      `crates/nvs-stdlib/src/request.rs:2213` is `saveTo`'s body. Check what a second consumer on one
      part actually does before pinning it — that may be a gap rather than a test.
- [ ] **`a_non_file_part_is_buffered_into_post`** — § 2's fields go where `post()` reads them, and
      **`post()` does not exist**: there is no `nvs_core_request_post` in the crate, so this slice is
      the member first (`crates/nvs-stdlib/src/request.rs:185` for the row,
      `crates/nvs-stdlib/src/multipart.rs:231` for the `fields()` the walk already buffers).

## Backlog

- **`Core\Session`, stage 5 item 14** — nothing on disk; `docs/agent/loop-goal.md` § *Stage 5* owns it,
  ADR 0012 § 4 and ADR 0059 § 4 are now in `[context] adrs`.
- `body_stream_is_exclusive_with_body_and_with_files` — grep near
  `crates/nvs-stdlib/src/request.rs:2892` first; the exclusivity is already asserted under other names.
- `an_upload_total_over_the_cap_is_refused_before_dispatch` — nvs-server's own § 5 half; nothing in
  `crates/nvs-server/src/*.rs` mentions `request_body` yet.
- A program still cannot hand `$part->content()` to `Core\IO::writeStream` — the playbook bullet owns
  it; a `nvs-types` conformance question, not this module's.
- `[context] modules` gained `request.rs` and `multipart.rs` this session — the map had printed only
  `registry.rs` for a stage whose whole surface is those two files.
