# Handoff

## State

**Goal 6, Stage 5: ADR 0105 §§ 1-2's multipart parse is on disk, and nothing calls it yet.**
`crates/nvs-stdlib/src/multipart.rs` reads the same `nvs_runtime::RequestBody` `bodyStream`
walks: `Multipart::next_part` answers the next **file** part and drains an unconsumed one
(§ 1), `next_chunk` hands a part's body out as spans of a buffer holding one wire chunk plus
one delimiter's tail, and a part with no `filename` is buffered into `Multipart::fields` for
`post()` (§ 2). The module's own doc comment is the whole argument; ten `#[test]`s hold it,
including the sweep over chunk sizes and the body that quotes its own boundary.

**It takes the body per call rather than borrowing it once**, so it can be stored beside the
thing it reads — `crate::request`'s `body_stream_step` is that shape one layer up.

**Three bounds are here and one deliberately is not.** `MAX_PARTS` (ADR 0095 § 4),
`PART_HEADERS`, and `crate::request::REQUEST_BODY` — now `pub(crate)` — for § 2's buffered
fields. `upload_total` stays `nvs_server::body::UPLOAD_TOTAL`, enforced on the wire, because
the parts this module drains are still the server's to count.

**The `#![allow(dead_code)]` at the top of the module is a one-slice loan.** The parse has no
non-test caller until `files()` lands; delete the attribute with that member's first call.

**`examples/upload.nvs` — the failing acceptance check — stays failing**, and the next group's
third item is now a question rather than a rewrite: see it below before touching the example.

## Next group

**`files()`, the part it yields, and the example that proves there is no temp file.** One file
set: `crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/multipart.rs`,
`crates/nvs-stdlib/src/registry.rs`, `examples/upload.nvs`.

- [ ] **`Core\Request::files(): Iterable<Part>` and `Core\Request\Part`** — ADR 0105 §§ 1-3.
      Two classes on `crates/nvs-stdlib/src/request.rs:489`'s `BODY_STREAM` shape, whose
      iterate/advance/current are its own; the rows go in
      `crates/nvs-stdlib/src/request.rs:182`, the arms in
      `crates/nvs-stdlib/src/request.rs:499`, the element type in
      `crates/nvs-stdlib/src/registry.rs:2229`, and `claim_body(ctx, "files")` at
      `crates/nvs-stdlib/src/request.rs:533` is § 15's exclusivity in one line. **Decide first
      where the `Multipart` lives across calls**: `crates/nvs-stdlib/src/io.rs:2187`'s
      `handle_of` keys a per-request table off a slot, and
      `crates/nvs-stdlib/src/request.rs:455` is the other shape — a slot, with
      `ctx.inbound_mut()` re-fetched every call.
- [ ] **`Part::readAll`, `Part::content` and `Part::saveTo`** — ADR 0105 §§ 3-4. `saveTo`
      delegates to `Core\IO::writeStream`, which already landed: its row is
      `crates/nvs-stdlib/src/io.rs:124` and its body `crates/nvs-stdlib/src/io.rs:2724`.
      `filename` and `contentType` are `tainted` — the spelling is beside `bodyStream`'s at
      `crates/nvs-stdlib/src/request.rs:425`.
- [ ] **`examples/upload.nvs` runs and the acceptance check passes** — the check at
      `docs/agent/loop-goal.toml:3382` wants `parts=2 / field=title / file=report.pdf / saved
      4096 bytes / no temp file` from a **program** leg, but a program answers no request, so
      `files()` throws inside one. Settle that before rewriting `examples/upload.nvs:1`: the
      block the root `nvs.toml` carries for it is an `[[app]]` grant and not a mount, so
      either the example has to reach a real request or the check has to become a server
      check. `examples/session.nvs` is the same shape with the same unanswered question.

## Backlog

- The stage-5 check at `docs/agent/loop-goal.toml:3365` files all seven ADR 0105 tests under
  `-p nvs-server`; only `an_upload_total_over_the_cap_is_refused_before_dispatch` is that
  crate's, and `a_part_is_a_file_part_iff_content_disposition_carries_a_filename` already runs
  in `-p nvs-stdlib`. Split it, and fix `docs/agent/goals/` alongside the live copy.
- `[context] adrs` printed ADR 0105 §§ 1-2 only; §§ 3, 4 and 5 and ADR 0095 § 4 were all needed
  and cost four slices of the ADRs to fetch.
- Spec § 15 still has no `spec` selector: `docs/spec/01-core-library.md:1048-1072`.
- `Core\Request::post()` has no registry row; `Multipart::fields()` is what it will read.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, m7.md.
