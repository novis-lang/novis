# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–5 are on disk. Stage 6's surface is complete — `Core\Http\Stream`, its four readers, and now
the two bounds only it carries — and stage 6's behaviour is not: the body still arrives whole with the
head. Nothing of stages 7–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

`stream` is the one row of `Core\Http\Client` whose bag is not `OPTIONS`. `request_options!`
(`crates/nvs-stdlib/src/http.rs:379`) is `request_params!`'s shape one axis over — the shared keys, then
a group a row appends — and `STREAM_OPTIONS` (`crates/nvs-stdlib/src/http.rs:510`) is `OPTIONS` plus
`idle` and `maxDuration`. Either key on a buffered member is `E0454` from the checker finding no such
option, which is
`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`'s compile-time half and is
pinned by `tests/conformance/reject/http-client-idle-on-a-buffered-member-is-refused.nvst`.

**What is not yet true: neither bound bounds anything.** `exchanged` (`crates/nvs-stdlib/src/http.rs:2052`)
hands the whole reply over with the head, so the two keys are judged positive at the call
(`crates/nvs-stdlib/src/http/stream.rs:794`) and then nothing reads them — `[http.client] idle` and
`max_duration` ship as `[unread:]` keys with a `# NOT IMPLEMENTED` note, which the next group deletes.
`tls()`, which the same rule names on the head, waits for § 13's TLS session.

Nothing is blocked and no design question is open.

## Next group

**Stage 6: the streamed body arrives as a walk, under its own two bounds** — one file set:
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/stream.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **The transport hands over a reader rather than a body** — `send`
      (`crates/nvs-stdlib/src/http/transport.rs:288`) reads the head and the body into one `Reply`
      (`crates/nvs-stdlib/src/http/transport.rs:229`) inside `exchange`
      (`crates/nvs-stdlib/src/http/transport.rs:404`), which `parse`
      (`crates/nvs-stdlib/src/http/transport.rs:800`) frames whole. A streamed call has to stop at the
      head and hand back the connection with whatever octets came with it, so the walk runs on the
      socket rather than on a `Vec`; `exchanged` (`crates/nvs-stdlib/src/http.rs:2052`) is where the
      two answers fork today and `Core\Http\Stream`'s body slot
      (`crates/nvs-stdlib/src/http/stream.rs:794`) is what stops being `Value::bytes`.
      `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`.
- [ ] **The two bounds are read from `[http.client]`, and the unread notes come off** — `bound_of`
      (`crates/nvs-stdlib/src/http.rs:1543`) over `http.client.idle` and `http.client.max_duration`
      with the record's shipped `30s` and `5m`, beside `DEFAULT_DEADLINE`
      (`crates/nvs-stdlib/src/http.rs:853`); `idle` refreshes per read and `maxDuration` is one
      `Instant` for the walk. Then delete the `[unread:]` trailers
      (`crates/nvs-config/src/tree.rs:517`) and the `# NOT IMPLEMENTED` block
      (`crates/nvs-config/src/default.toml:270`) — `python tools/directives.py --check` fails while
      either outlives its gap.
- [ ] **Stage 6's second check, three cases in `transport.rs`'s own module** —
      `stream_idle_ends_a_silent_reply`, `stream_max_duration_ends_an_endless_reply` and
      `stream_is_retried_before_its_head_and_never_after`, beside
      `a_retry_is_jittered_and_shares_the_covering_deadline`
      (`crates/nvs-stdlib/src/http/transport.rs:1041`), whose origin fixture is the shape to copy.
      `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`.

## Backlog

- `Core\Http\Stream::tls()` — the rule names it on the head; it needs § 13's TLS session
  (`docs/agent/goals/48-http-client.md` § *Stage 13*).
- `crates/nvs-stdlib/src/http.rs` is at 2,929 lines and stages 7 and 10 both add options to it
  (`docs/agent/playbook.md` § *Splitting a file that got too big*).
- `directives.py` trusts a generic `written.<field>` wherever the field name is unique in the roster;
  that half is still the unsound spelling its own module doc describes (`tools/directives.py`).
