# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–3 are on disk, stage 4 is most of the way, and nothing of stages 5–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

**A body now reaches the wire.** `Core\Http\Part` is registered with `file` and `bytes`
(`crates/nvs-stdlib/src/http.rs:546`), `body_of` frames all four keys —
`json` through `crate::json::written`, `form` through `crate::uri::encode`, `body` as octets or a
part, `multipart` as segments under a boundary drawn per call
(`crates/nvs-stdlib/src/http.rs:1307`) — and `transport::Call` carries the result, which `compose`
types and lengths and `write_body` writes, a file piece at one `BODY_CHUNK` and rewound per attempt.
The faked path frames with the same function, so `Core\Test\SentRequest::body()` answers the octets a
real call would have sent.

**What stage 4 still owes is the verb half**: there is no `patch` row and no
`request(Core\Http\Method, …)`, so neither dynamic-verb check exists yet. Nothing is blocked and no
design question is open.

## Next group

**Stage 4: the verb half** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/expr/args.rs`.

- [ ] **`patch` joins the five rows** — a sixth `CoreMethod` beside `post` at
      `crates/nvs-stdlib/src/http.rs:795` with its card and its `address()` arm at
      `crates/nvs-stdlib/src/http.rs:204`, and the two rules that already name it: the verb is
      already in `idempotent_retry_rule` at `crates/nvs-stdlib/src/registry.rs:3097` and must join
      `request_body_rule`'s match at `crates/nvs-stdlib/src/registry.rs:3135`.
      `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`, ADR 0180 § 1.
- [ ] **`request(Core\Http\Method $method, …)` is the seventh row** — the verb is a
      `Core\Http\Method` (`crates/nvs-stdlib/src/router.rs:101`) read at run time, so `request` at
      `crates/nvs-stdlib/src/http.rs:1615` takes the verb from slot 0 and every option slot shifts
      by one. There is no `Core\Http\Request` and no `send` — ADR 0180 § 1 settled that.
- [ ] **The two compile-time checks throw before the first attempt where the verb is dynamic** —
      `reject_keyless_retry` at `crates/nvs-types/src/expr/args.rs:955` and
      `reject_ill_formed_body` at `crates/nvs-types/src/expr/args.rs:1034` are the diagnostics; the
      run-time halves belong beside the bounds in `request`, before the clock starts.
      Cases: `tests/conformance/core/http-client-patch-and-request-send-the-verb-they-name.nvst` and
      `tests/conformance/core/http-client-a-dynamic-body-on-get-or-keyless-post-retry-throws-before-sending.nvst`.
- [ ] **The last stage-4 case with no code behind it** — a `secret` is admitted at a header, at a
      `form` value and inside a `json` document, and refused at every ordinary sink, written to
      `tests/conformance/core/http-client-a-secret-reaches-a-header-a-form-and-a-json-body.nvst`.
      The positions that admit it are the option types at `crates/nvs-stdlib/src/http.rs:364` and
      the walk at `crates/nvs-stdlib/src/http.rs:1307`.
      `rule:security/secret-sinks-refuse`'s outbound exemption.

## Backlog

- Stage 5 onwards is untouched: reply headers, `jsonAs`, streaming, compression, the pool, TLS.
  [docs/agent/loop-goal.md](loop-goal.md) is the order.
- `RESPONSE` still answers `status()` and `text()` only — `crates/nvs-stdlib/src/http.rs`'s module
  doc § *What is not here yet* is the home of that list.
- A `Part` in a `multipart` field is framed whole into the record on the faked path, so a test that
  fakes a very large upload holds it; a real call does not. Only worth changing if a case needs it.
