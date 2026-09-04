# Handoff

## State

**Goal 6, Stage 5: both readings of a request body are on disk, and the request records which
one took it.** `Core\Request::bodyStream(): Iterable<tainted bytes>` is spec § 15's streaming
reader — nine of § 15's fifteen members now. It answers `Core\Request\BodyStream`, the one
`Iterable` in `Core` that is **its own iterator** rather than a `crate::cursor` snapshot:
`iterate()` hands the receiver back and `advance()` pulls one chunk off
`nvs_runtime::RequestBody`, so nothing accumulates and no `REQUEST_BODY` bound applies. The
class's own doc comment at `crates/nvs-stdlib/src/request.rs:445` is the whole argument.

**The element is `tainted bytes`, which needed a new `CoreTy` spelling.** `registry::CoreTy`
had no return-position `bytes` twin of `TaintedStr`, and a plain `bytes` element would have
made `bodyStream` a launderer for the same octets `body()` marks. `CoreTy::TaintedBytes` is
four sites — the variant, `nvs_types::core_lib`'s lowering, `nvs_stdlib::ast`'s leaf list,
`nvs_cli::meta`'s rendering — and the mark reaches a `foreach` binding through
`registry::ITERABLES`. **Spec § 15's own signature was amended** to `Iterable<tainted bytes>`;
it had written plain `bytes`.

**Spec § 15's exclusivity is enforced, not recorded as a gap.** `nvs_runtime::Inbound`'s new
`claimed_by` field and `Inbound::claim_body` are the rule's only home; `nvs_stdlib::request`'s
`claim_body` is only the wording. The claim is taken where a reading is **named** — so
`bodyStream()` claims once and its `advance()` never does — and it is taken on a request that
carried no body too, because what § 15 makes exclusive is the reading. `files` joins the same
call when it lands.

**Neither slice could take a `.nvst` case for its throws**: a case answers no request, so both
new `Fault::` sites carry `conformance_coverage.rs`'s `no case can reach this` declaration
naming the `#[test]` that asserts them instead.

**`examples/upload.nvs` — the failing acceptance check — stays failing** until `files()` lands;
it wants parts, and nothing yields one yet. The example itself is still the IO-only placeholder
and needs rewriting around `files()` in the same slice.

**`[context]` gaps:** `adrs` still selects no § 3 and no § 5 of ADR 0105; spec § 15 has no
`spec` selector and it is `docs/spec/01-core-library.md:1048-1072`.

## Next group

**`files()`, the parts it yields, and the example that proves there is no temp file.** One file
set: `crates/nvs-stdlib/src/request.rs`, `crates/nvs-server/src/body.rs`, `examples/upload.nvs`,
`tests/conformance/core/`.

- [ ] **The multipart parse, as a `RequestBody` reader** — ADR 0105 §§ 1-2. It pulls the same
      `nvs_runtime::RequestBody` `bodyStream` walks, so the shape to copy is
      `crates/nvs-stdlib/src/request.rs:1032`'s `body_stream_step`, and the boundary comes off
      `Content-Type`, read with `crates/nvs-stdlib/src/request.rs:@joined_field`. The total
      bound is `crates/nvs-server/src/body.rs`'s `UPLOAD_TOTAL`, not
      `crates/nvs-stdlib/src/request.rs:@REQUEST_BODY`.
- [ ] **`Core\Request::files(): Iterable<Part>` and `Core\Request\Part`** — ADR 0105 §§ 3-4 and
      spec § 15 (`docs/spec/01-core-library.md:1063`). Five edits beside `bodyStream`'s: row at
      `crates/nvs-stdlib/src/request.rs:250`, card at `crates/nvs-stdlib/src/request.rs:410`,
      `address` arm at `crates/nvs-stdlib/src/request.rs:490`, class beside
      `crates/nvs-stdlib/src/request.rs:445`. It owes `crates/nvs-stdlib/src/request.rs:@claim_body`
      a call with `"files"`, which is the third member that rule was written for. A part's
      `filename` and `contentType` are `CoreTy::TaintedStr`; `readAll` is `CoreTy::TaintedBytes`.
- [ ] **`examples/upload.nvs` runs, and the acceptance check passes** — the check wants
      `parts=2 / field=title / file=report.pdf / saved 4096 bytes / no temp file`. The file is
      today an IO-only placeholder that never mentions `files()` — `examples/upload.nvs:25` is
      where it starts inventing a directory instead — so rewrite it around the member, keeping
      `examples/upload.nvs:63`'s `Core\IO::within` as the launderer between a claimed filename
      and a path.

## Backlog

- Raw/unparsed body access for an arbitrary content-type — `docs/plan/m7.md`, narrowed to what
  `body()`/`bodyStream()` do not answer; the goal's standing decisions pre-authorize the call.
- `clientIp`, `scheme`, `host` — `crates/nvs-stdlib/src/request.rs`'s module doc; they wait on
  `[server] trusted_proxies` and the forwarded-header walk.
- `route`/`mount` — the same module doc; they wait on ADR 0102's match, made once by
  `nvs_server` before the handler.
- `Core\Request::post()` for a multipart form's non-file parts — spec § 15, line 1068; it falls
  out of the parse the next group writes.
- The `Core\Request\BodyStream` anchor in `docs/novis.md` collides with `Core\Request::bodyStream`'s,
  as `Core\IO\Lines`' already does with `Core\IO::lines`' — `tools/reference.py`, and
  `check-links.py` is green either way.
