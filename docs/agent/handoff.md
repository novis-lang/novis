# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–3 are on disk and nothing of stages 4–14 is.** Goal `webcrypto`'s whole list is this goal's
Stage 1 floor.

[ADR 0180](../decisions/0180.md) is the record, and it is the home of every decision this goal executes;
its § 1 is stage 3's, which landed: `Core\Test::answerHttp` arms a per-test table on `nvs_runtime::Ctx`
(`crates/nvs-runtime/src/ctx/answers.rs`), `Core\Http\Client`'s members answer from it above `approved`
so a faked call resolves nothing and asks no capability, and `Core\Test::sentHttp` hands back a
`Core\Test\SentRequest` per call. The eleven rules the record creates are on disk as `designed` — stage
14 flips them to `shipped`. Nothing is blocked and no design question is open.

**A record's `body()` is empty for every call**, because no request carries one yet: `faked` writes
`Vec::new()` at `crates/nvs-stdlib/src/http.rs:1000`, and the next group is what fills it.

## Next group

**Stage 4: a request carries a body, and a verb chosen at run time** — one file set:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **The four body keys join the one bag** — `json`, `form`, `body` and `multipart` as flat keys of
      `OPTIONS` at `crates/nvs-stdlib/src/http.rs:343` with their ABI slots beside the existing seven at
      `crates/nvs-stdlib/src/http.rs:390`, two of them at one call refused, and a body on `get` or `head`
      refused. `rule:http-server/an-outbound-request-carries-one-body`, ADR 0180 § 2.
- [ ] **The transport frames the body it is handed** — `Call` gains it at
      `crates/nvs-stdlib/src/http/transport.rs:81`, composed once with its `Content-Type` and
      `Content-Length`, and a file part streamed at one chunk. `rule:http-server/an-outbound-request-carries-one-body`,
      ADR 0180 § 3.
- [ ] **`request(Method, …)` joins the five rows** at `crates/nvs-stdlib/src/http.rs:423` — the verb
      chosen at run time, which the five named rows cannot express. ADR 0180 § 2.
- [ ] **The faked path records what the call would have carried** — `faked` at
      `crates/nvs-stdlib/src/http.rs:1000` fills `HttpSent::body` from the same composed body, so
      `Core\Test\SentRequest::body()` reads the document the subject sent.
      `rule:testing/an-outbound-call-is-answered-from-a-table`.

## Backlog

- `x509-parser` moves from `rcgen`'s `[dev-dependencies]` to `nvs-host`'s own at stage 11 — ADR 0180 § 13.
- `crates/nvs-config/src/default.toml:260-268`'s `[http.client]` comment block gains four keys and a
  `[http.client.tls]` block beside it — the goal's stage 0 list owns the sentence set.
- An answer's `headers` are held and nothing reads them back: `Core\Http\Response` grows its header
  members at stage 5, and the `content-type` a `json` answer declares is asserted there.
- Stage 14 flips the eleven `designed` rules to `shipped` with `guardedBy` filled — the goal's stage 14.
- `Link`, `Retry-After` for a program and RFC 9457 problem details stay the `nvs/rest` package's — the
  goal's § *Standing decisions*, *Not this goal*.
- Two load-sensitive fixtures fail beside the other test binaries and pass alone —
  `crates/nvs-server/src/serve.rs:7958`'s fleet-wide in-flight ceiling and
  `crates/nvs-host/src/watchdog.rs:1051`'s `a_run_that_is_no_core_is_sampled_and_reported_never`. Each
  fails on its own run, never both, and `tools/verify.py` § *Why `test` runs its binaries side by side*
  is the fix each asks for. The rest of the gate is green: fmt, clippy, conformance 1813 and
  differential 276.
