# Handoff

## State

**Goal 6, Stage 5: ADR 0105 §§ 1-2's parse has its caller, and `Core\Request::files()` walks
it.** Two classes on `BODY_STREAM`'s shape — `Core\Request\Files`, a handle whose
iterate/advance/current are its own (`crates/nvs-stdlib/src/request.rs:580`), and
`Core\Request\Part` (`crates/nvs-stdlib/src/request.rs:642`), which answers `name()`,
`filename()` and `contentType()` off four slots. `claim_body(ctx, "files")` is § 15's
exclusivity, now asserted in both directions against all three readings.

**Where the parse lives across calls, decided:** on `nvs_runtime::Inbound` as
`Option<Box<dyn Any>>`, with `hold_parts` and a `parts_mut` that borrows the parse and the
body *together* — the parse takes the body per call, so two accessors could not hand that
out. Not `Ctx::hold_open_file`'s keyed table: that exists to hold many of something, and a
request has one body. The field's own doc is the argument.

**Three decisions recorded in ADR 0105's body and spec § 15, not overlaid:** a part's three
declarations are **members, not properties** (a `Core` instance has no reachable property —
see the playbook bullet); **all three are `tainted`**, `name` included, because a peer
chooses the field name as freely as the filename; and `contentType()` answers RFC 7578
§ 4.4's `text/plain` where the part declared none, rather than being nullable.

**A request that is not `multipart/form-data` walks empty**; one that *says* it is and does
not say how is refused (`ParseError`). `crate::multipart::is_multipart` is where the two
split. A failure off the wire is an `IOError`, told apart from a parse refusal by
`Multipart::failed_on_the_wire` rather than by reading the message.

**`Core\IO::writeStream` already exists** (`crates/nvs-stdlib/src/io.rs:124`), so § 4 is
landed and `saveTo` is a delegation rather than an implementation.

**`examples/upload.nvs` — the failing acceptance check — stays failing**, and the third item
below is still a question: the check wants `parts=2` / `field=title` from a *program* leg,
and a program answering no request cannot call `files()` at all.

## Next group

**§ 3's three ways to consume a part, then the example.** One file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/multipart.rs`,
`crates/nvs-stdlib/src/registry.rs`, `examples/upload.nvs`.

- [ ] **`Part::content(): Iterable<bytes>` and `Part::readAll({max?})`** — ADR 0105 § 3. A third
      handle class on the same shape, whose `advance()` is
      `crates/nvs-stdlib/src/multipart.rs:296`'s `next_chunk` (delete its `#[allow(dead_code)]`
      with the first call). Both are guarded by `crates/nvs-stdlib/src/request.rs:605`'s
      `PART_ORDINAL` against `Multipart::opened()`
      (`crates/nvs-stdlib/src/multipart.rs:209`) — § 3's "valid only while this part is the
      iterator's current one", which nothing checks yet. `readAll`'s bare bound is
      `crates/nvs-stdlib/src/request.rs:927`'s `REQUEST_BODY`, an explicit `max` is
      `[limits] memory`. Rows and cards go beside
      `crates/nvs-stdlib/src/request.rs:642`; the slot reader is
      `crates/nvs-stdlib/src/request.rs:1577`.
- [ ] **`Part::saveTo(string $path, {max?, overwrite?})`** — ADR 0105 § 4, a delegation to
      `Core\IO::writeStream` (`crates/nvs-stdlib/src/io.rs:124`), which already removes a
      partial file on a mid-stream failure. The row goes at
      `crates/nvs-stdlib/src/request.rs:642`; the path is a sink, so the destination is the
      application's own and a `tainted` filename never reaches it.
- [ ] **`examples/upload.nvs` and the acceptance check** — `docs/agent/loop-goal.toml:3385`
      wants `parts=2`, `field=title`, `file=report.pdf`, `saved 4096 bytes`, `no temp file`
      from `examples/upload.nvs:1`, which runs as a CLI program. **Decide first whether a
      program leg can receive a request at all**: `Core\Request::files()` throws in one, and
      `examples/session.nvs` has the same problem with the same shape of `want`. If it cannot,
      the check is the playbook's "names something that is not a test" trap and the fix is in
      `docs/agent/goals/<goal>.toml` as well as the live copy.

## Backlog
- `Core\Request::post()` reads `Multipart::fields()` (`crates/nvs-stdlib/src/multipart.rs:231`),
  still `#[allow(dead_code)]` — ADR 0105 § 2, spec § 15.
- `clientIp`/`scheme`/`host` wait on `[server] trusted_proxies` — `crates/nvs-stdlib/src/request.rs`'s
  module doc.
- `route`/`mount` wait on the match `nvs_server` makes once — ADR 0102.
- ADR 0105 § 5's `upload_total` is enforced on the wire only — `crates/nvs-server/src/body.rs`.
- M7's bounded-resident-memory case for a body far larger than any in-memory bound —
  `docs/plan/m7.md`'s verify paragraph.
