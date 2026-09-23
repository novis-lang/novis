# Handoff

## State

Goal `core-http-client-and-2-more` is complete. Every one of its twelve features has
`about.md`, three examples, one attack, one bench and a Rust test, and
`python tools/dossier.py --verify --group` answers `nothing owed` for `Core\Http\Client`,
`Core\Http\Identity` and `Core\Http\Part`. The previous session reported `DONE` with only
`delete`, `get` and `head` written. This session wrote the other nine with the fan-out.

Three bugs the proofs found are fixed and pinned:
- `Core\Http\Stream::lines()`/`events()` and every `Core\IO` reader (`read`, `lines`, `stdin`,
  `File::read`, `File::readLine`) built a `string` from octets never checked to be UTF-8. They now
  throw a `RuntimeError`. Pinned by
  `tests/conformance/core/http-stream-lines-and-events-throw-on-a-piece-that-is-not-utf8.nvst` and
  `core_io_read_and_lines_refuse_a_file_that_is_not_utf8`.
- A multipart form opened every file before it checked the names; it now checks every name first
  (`a_multipart_form_checks_every_name_before_it_opens_a_file`).

The root `nvs.toml` grants `fs.read` and `net.connect` to the `Http-Part` proof trees.

## Next group

**Stage 1: the goal is complete** — one file set: `crates/nvs-stdlib/src/http.rs` and the proof trees.

- [x] **Every item of goal `core-http-client-and-2-more` has its feature proofs** — `crates/nvs-stdlib/src/http.rs:1539`

## Backlog
- `Core\IO\File::read(length)` now throws when its chunk ends inside a multi-byte character, so a chunked read of valid text can fail. Returning `bytes` there, or seeking back over the partial character the way `readLine` seeks, is a user decision (`crates/nvs-stdlib/src/io.rs`).
- Other `Value::str(NvsStr::new(..))` sites over outside octets are not checked: `Core\Request::headers` (`crates/nvs-stdlib/src/request.rs:2542`), `crates/nvs-stdlib/src/cache.rs:1731`, `crates/nvs-stdlib/src/uri.rs:1469`.
- `nvs-cli`'s `serve_arms_a_fleet_entry_when_the_shared_tier_can_take_a_lease` and `a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile` each failed once beside the other test binaries and passed alone (`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/script.rs`).
