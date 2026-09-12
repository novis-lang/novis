# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stage 2 is complete and nothing of stages 3–14 is on disk.** Goal `webcrypto`'s whole list is this goal's
Stage 1 floor.

[ADR 0180](../decisions/0180.md) is the record, and it is the home of every decision this goal executes:
§ 1 the answer table, § 2–3 bodies and `request(Method, …)`, § 4 the reply's members and `Retry-After` in
both forms, § 5 the stream and its two bounds, § 6–8 the pool, the identity and the content codings, § 9
the credential drop, § 10–13 trust, the address, the downgrade and `Response::tls()`, § 14 the address
set, § 15 the `http` trace event, § 16 the four directives and the numbers they ship, § 17 what it spends.
The eleven rules it creates are on disk as `designed` — stage 14 flips them to `shipped` — and the
`Core\Http\Client` spec row now states the surface. Nothing is blocked and no design question is open:
what the record did not inherit from the goal's § *Standing decisions* it fixes itself, including the
shipped `idle`, `max_duration`, `pool_idle`, `pool_idle_timeout`, `roots` and `min_version`, the SSE line
and event caps, the RFC 8305 attempt delay, and `x509-parser` promoted out of `rcgen`'s dev graph for
§ 13's three leaf facts.

## Next group

**Stage 3: the keystone, an outbound call answered from a table** — one file set:
`crates/nvs-stdlib/src/test.rs`, `crates/nvs-stdlib/src/http/transport.rs`,
`crates/nvs-stdlib/src/http.rs`.

- [ ] **`Core\Test::answerHttp` and `::sentHttp`, and the table they fill** — two static rows beside
      `request` at `crates/nvs-stdlib/src/test.rs:357`, the per-test answer store, exact and `*`-prefix
      matching, and `Core\Test\SentRequest`'s four readonly members with nothing on it `tainted`.
      `rule:testing/an-outbound-call-is-answered-from-a-table`, ADR 0180 § 1.
- [ ] **The transport consults the table before it connects** — one registered answer takes the whole
      test off the network and an unmatched call throws a `LogicError` naming the URL, spliced ahead of
      the exchange at `crates/nvs-stdlib/src/http/transport.rs:290`; the scheme and the grant are still
      judged, while resolution and the address check at `crates/nvs-stdlib/src/http.rs:252` are skipped,
      no connection being made for an address to be pinned to.
      `rule:testing/an-outbound-call-is-answered-from-a-table`, `rule:security/outbound-url-is-a-sink`.
- [ ] **The `.nvst` cases the later stages are written against** — a faked call answered, an unmatched
      one refused, `sentHttp` in order, and a table reached with no `net.connect` and no `net.internal`
      granted, which is the cost `crates/nvs-stdlib/src/test.rs:161` records for a real listener.
      `rule:testing/nvst-is-separate`.

## Backlog

- `x509-parser` moves from `rcgen`'s `[dev-dependencies]` to `nvs-host`'s own at stage 11 — ADR 0180 § 13.
- `crates/nvs-config/src/default.toml:260-268`'s `[http.client]` comment block gains four keys and a
  `[http.client.tls]` block beside it — the goal's stage 0 list owns the sentence set.
- Stage 14 flips the eleven `designed` rules to `shipped` with `guardedBy` filled — the goal's stage 14.
- `Link`, `Retry-After` for a program and RFC 9457 problem details stay the `nvs/rest` package's — the
  goal's § *Standing decisions*, *Not this goal*.
- `nvs-server`'s `the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle`
  (`crates/nvs-server/src/serve.rs:7958`) fails beside the other test binaries and passes alone — a
  load-sensitive fixture, untouched by this session's docs-only change, and `tools/verify.py` § *Why
  `test` runs its binaries side by side* is the fix it asks for.
