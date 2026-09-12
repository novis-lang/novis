# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–4 are on disk, stage 5 is half on disk, and nothing of stages 6–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

`Core\Http\Response` now carries five instance rows over three slots
(`crates/nvs-stdlib/src/http.rs:904`): `status`, `text`, `bytes`, `header` and `headers`. **The body
slot holds octets and `text` is what asks whether they are UTF-8** — the transport no longer decodes,
so a reply that is not text reaches `bytes()` instead of failing the call, and `text()` retags the
slot's own allocation rather than copying it. The header slot is one entry per lower-cased name, each
an array of that name's lines in arrival order; `header` joins them with `, ` and throws a
`LogicError` naming `headers` for `set-cookie`, present or not.

`Core\Test::answerHttp` grew two arms to make those testable: `body` is `string|bytes`, and a
`headers` entry may be an `array<string>` for a field a reply carried twice
(`crates/nvs-stdlib/src/test.rs:214`). Both are test-surface only and neither changes a request.

Nothing is blocked and no design question is open.

## Next group

**Stage 5: reading a reply, the second half** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-types/src/expr/args.rs`.

- [ ] **`jsonAs<T>()` joins the decode-site roster** — a sixth instance row beside `bytes` at
      `crates/nvs-stdlib/src/http.rs:904`, decoding the body slot's octets. The roster is a match on
      the member name alone at `crates/nvs-types/src/expr/args.rs:1649`, so a `jsonAs` on this class
      is recorded by it already — what the slice owes is the case that pins it and the comment above
      it at `crates/nvs-types/src/expr/args.rs:1641`, which names two members and will name three.
      `T` must be a `tainted` shape or a class whose text fields declare `tainted`
      (`rule:security/derived-codec-qualifiers`); the reject case the check names is
      `tests/conformance/reject/http-response-json-as-into-an-unqualified-shape-is-refused.nvst`.
- [ ] **`Retry-After` is read in both forms** — `retry_after` at
      `crates/nvs-stdlib/src/http/transport.rs:663` reads delay-seconds only, and is called for all
      four closed statuses at `crates/nvs-stdlib/src/http/transport.rs:307`. An HTTP-date replaces the
      backoff, one past the remaining deadline throws now, one already past keeps the jittered
      backoff, and the header is read after a `429` or a `503` alone —
      `rule:http-server/retry-is-opt-in-jittered-and-closed`. The four Rust tests the check names are
      the stage's `cargo-named` half.

## Backlog

- Stage 6 onwards of goal `http-client` is untouched; the goal's prose is the order.
- `REPLY_CEILING` still has no streaming member beside it — `crates/nvs-stdlib/src/http/transport.rs:74`
  names the gap, and the goal's later stages own it.
- `Core\Http\Response` has no timing member and will not get one — a standing decision of the goal.
