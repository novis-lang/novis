# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–3 are on disk, stage 4 has its first slice, and nothing of stages 5–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

`Core\Http\Options` now declares `json`, `form`, `body`, `contentType` and `multipart`
(`crates/nvs-stdlib/src/http.rs:364`), and `nvs_types::expr::args`' `reject_ill_formed_body` refuses two
body keys at one call (`E0815`), any of them on `get` or `head` (`E0816`) and a `contentType` with no
octets to type (`E0817`). Which keys those are is `nvs_stdlib::registry::request_body_rule`'s, beside
`idempotent_retry_rule`, so the checker holds no copy of a spelling.

**The surface is ahead of the wire: a body written at a call is checked and then not sent**, because
nothing under `request` (`crates/nvs-stdlib/src/http.rs:1021`) reads the new slots yet. That module's
§ *What is not here yet* says so, and the next group closes it. Nothing is blocked and no design
question is open.

## Next group

**Stage 4: the body reaches the wire** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **`Core\Http\Part` joins the two unions it is missing from** — `Part::file($path, {filename?,
      contentType?})` and `Part::bytes($data, $filename, {contentType?})` as a class beside `CLIENT` at
      `crates/nvs-stdlib/src/http.rs:518`, its `address()` arm at `crates/nvs-stdlib/src/http.rs:200`,
      and `CoreTy::Instance` added to `body` and `multipart` at
      `crates/nvs-stdlib/src/http.rs:364`. Declared first so the composition below is written once.
      `rule:http-server/an-outbound-request-carries-one-body`, ADR 0180 § 2.
- [ ] **The transport frames the body it is handed** — `Call` gains it at
      `crates/nvs-stdlib/src/http/transport.rs:81`, composed once per call from the slots after
      `RETRY_KEY` (`crates/nvs-stdlib/src/http.rs:417`, which is where those slot constants belong)
      with its `Content-Type` and always a `Content-Length`, and a file part read at one chunk per
      attempt. `rule:http-server/an-outbound-request-carries-one-body`, ADR 0180 § 3.
- [ ] **`request(Method, …)` joins the five rows** at `crates/nvs-stdlib/src/http.rs:518` — the verb
      chosen at run time, with the two checks that are diagnostics at a named row throwing before the
      first attempt instead. ADR 0180 § 2,
      `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`.
- [ ] **The faked path records what the call carried** — `faked` at
      `crates/nvs-stdlib/src/http.rs:1111` fills `HttpSent::body` from the same composed body, so
      `Core\Test\SentRequest::body()` reads the document the subject sent.
      `rule:testing/an-outbound-call-is-answered-from-a-table`.

## Backlog

- `headers` is still `array<string>` and so refuses a `secret string`; stage 4's secret case needs it
  declared `array<secret string>` — `docs/agent/loop-goal.md:157`.
- The stage's four remaining `.nvst` cases all read a body back through `Core\Test::sentHttp`, so they
  land with the faked slice above — `docs/agent/loop-goal.toml:9379`.
- The rule's `guardedBy` names the three reject cases; its status stays `designed` until stage 14 —
  `docs/rules/http-server.json`.
- `Core\Http\Response` still answers `status()` and `text()` alone — stage 5,
  `crates/nvs-stdlib/src/http.rs:109`.
