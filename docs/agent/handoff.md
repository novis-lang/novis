# Handoff

## State

**Goal 6, Stage 5: a Novis program can read a request body.** `Core\Request::body(): tainted string`
pulls `nvs_runtime::RequestBody` to its end into one value — eight of spec § 15's fifteen members
now, and the first that reads what arrived *after* the header block. The cap is
`crates/nvs-stdlib/src/request.rs:790`'s `REQUEST_BODY` (ADR 0105 § 5's `[limits] request_body`
default, 8M, as a constant until the row exists — `nvs_server::body::UPLOAD_TOTAL` is its twin) and
it is checked **before** each chunk is copied, so the buffer never holds more than the bound; a
`next_chunk` `Err` is an `IOError` rather than a short body. The helper's own doc comment is the
whole argument, including why nothing is reserved from `Content-Length`.

**The exclusivity spec § 15 states is recorded as a gap, not enforced.** `body`, `bodyStream` and
`files` are exclusive on one request; only the first exists, so a second `body()` answers `""`.
`crates/nvs-stdlib/src/request.rs:43`'s module-doc section owns why the record belongs on `Inbound`
rather than on any member. `nvs_runtime::Inbound::body`'s doc cited an ADR 0105 § 8 that does not
exist — that ADR ends at § 6 — and now cites spec § 15, which is the rule's real home.

**`conformance_coverage.rs`'s error-path gate has a third answer**, because this member's two throws
are the first that a program reaches and no `.nvst` case can: the playbook bullet is the rule, and
`OWED_A_CASE` is still empty.

**`examples/upload.nvs` — the failing acceptance check — stays failing** until `files()` lands; it
wants parts, and nothing yields one yet.

**`[context]` gaps:** `adrs` still selects no § 3 and no § 5 of ADR 0105, which every item in this
group needs and which this session peeked by hand; spec § 15 has no `spec` selector and it is
`docs/spec/01-core-library.md:1048-1070`.

## Next group

**The other two ways to read a body, and the record that keeps them apart.** One file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/io.rs`, `crates/nvs-runtime/src/ctx.rs`,
`tests/conformance/core/`.

- [ ] **`Core\Request::bodyStream(): Iterable<bytes>`** — ADR 0105 § 3's second way and spec § 15
      (`docs/spec/01-core-library.md:1068`). The shape to copy whole is `Core\IO::lines`' named
      `Iterable<string>` class at `crates/nvs-stdlib/src/io.rs:1626` with its `iterate()` symbol at
      `crates/nvs-stdlib/src/io.rs:1618`; the registry's note on the two spellings is
      `crates/nvs-stdlib/src/registry.rs:1238`. The five edits land beside `body`'s — row at
      `crates/nvs-stdlib/src/request.rs:241`, card at `crates/nvs-stdlib/src/request.rs:380`,
      `address` arm at `crates/nvs-stdlib/src/request.rs:417`, helper beside
      `crates/nvs-stdlib/src/request.rs:820`. The pull is `Inbound::body` at
      `crates/nvs-runtime/src/ctx.rs:4498` — **on `Inbound`, not `Ctx`** — and a chunk is borrowed
      only until the next pull, so each yielded `bytes` copies. No `REQUEST_BODY` bound applies:
      nothing accumulates.
- [ ] **The body carries which member claimed it** — spec § 15's exclusivity, enforced. A field
      beside the reader at `crates/nvs-runtime/src/ctx.rs:4486`, set by whichever of the three took
      the borrow and read back as a `LogicError` naming both members; the gap it closes is written
      out at `crates/nvs-stdlib/src/request.rs:43`, and `body`'s own refusal wording to reuse is
      `crates/nvs-stdlib/src/request.rs:400`'s `inbound_of`.
- [ ] **`Core\Request::files()`, and `examples/upload.nvs` runs** — ADR 0105 §§ 1-4's lazily yielded
      parts, over the same `Iterable` shape the first item builds — the multipart split is new code
      beside `crates/nvs-stdlib/src/request.rs:820`, reading the boundary out of `Content-Type` with
      `crates/nvs-stdlib/src/request.rs:400`'s `inbound_of` and the pull at
      `crates/nvs-runtime/src/ctx.rs:4498`. This is the item the driver's standing acceptance
      failure is waiting on.

## Backlog

- `[limits] request_body` and `upload_total` as real rows — ADR 0105 § 5; today both are constants
  (`crates/nvs-stdlib/src/request.rs:790`, `crates/nvs-server/src/body.rs:65`).
- A multipart body far larger than any in-memory bound, received at a bounded high-water mark —
  `docs/plan/m7.md`'s load-bearing acceptance case, and it needs `files()` first.
- Raw/unparsed body access for an arbitrary content-type — `docs/plan/m7.md`, narrowed to what
  `body()` and `bodyStream()` do not answer.
- `clientIp`/`scheme`/`host` on `[server] trusted_proxies` and the forwarded-header walk — ADR 0097.
- `route`/`mount` on the match `nvs_server` makes once before the handler — ADR 0102.
