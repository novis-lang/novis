# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections — has just started; nothing of it has landed yet.** Goal `webcrypto`'s whole list is this goal's Stage 1 floor.

**The design is settled with the user and is written into the goal's § *Standing decisions*; the record
that states it is not written yet.** Stage 2 writes it — one new record, the goal names no number — and it
transcribes those decisions rather than re-deriving them. The things not to re-decide: bodies are flat
keys of the one bag, at most one per call and checked while compiling; `Client::request(Method, …)`
replaces the spec's `send(Request)` and there is no `Core\Http\Request`; a stream's body is bounded by
`idle` and `maxDuration`; gzip only; the pool is per core and keyed on the pinned address and on the
client identity (mTLS) a call may present; a hop to
another origin drops credentials; a test answers outbound calls from a table.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/http-server*`,
`docs/rules/testing*`, `docs/spec/`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates` the five rules the
      goal's stage 2 table names, `changes.modifies`
      `http-server/a-non-idempotent-retry-needs-an-idempotency-key`. Its body is the goal's standing
      decisions and stages 3-8, argued, with the numbers it ships for `idle`, `max_duration`, `pool_idle`
      and `pool_idle_timeout`.
- [ ] **The five rule fragments and their JSON entries**, all `designed`, then `python tools/rules.py
      --render`. The idempotency fragment says `request` where it said `send`.
- [ ] **The spec row** — `docs/spec/01-core-library.md:1167` (`Core\Http\Client`) takes the surface:
      `patch`, `request`, `stream`, the eleven-key bag, and the reply's readers.

## Backlog

- Stage 3 — the table. `crates/nvs-stdlib/src/test.rs`, `registry.rs`, and the seam above
  `crate::http::transport` the table answers at. The keystone: every later `.nvst` case needs it.
- Stage 4 — bodies and verbs. `crates/nvs-stdlib/src/http.rs`, `json.rs`, `registry.rs`, and
  `crates/nvs-types/src/expr/args.rs` for the two diagnostics. Its own session.
- Stage 5 — the reply's readers. `http.rs`, `transport.rs`'s `Reply`, and the decode-site roster in
  `nvs-types`. Shares `http.rs` with stage 4.
- Stage 6 — streaming. `http.rs`, `transport.rs`, `crates/nvs-config/src/directive.rs`. Its own session.
- Stages 7 and 8 — the pool, gzip, the client identity, and the hop's credential rule. `transport.rs`,
  `compress.rs`, `crates/nvs-host/src/tls.rs`, `directive.rs`. One file set, two stages, two sessions.
- Stage 9 — the flips. `docs/rules/` only.
- When this goal's last check goes green the driver takes goal `process-cache`.
