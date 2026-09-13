# Handoff

## State

**Goal `http-client` — stage 12's runtime half is closed: the capability door reaches a name through a
seam a worker installs, judges every address that name answered, and answers the approved set.**
Stages 1–11 are on disk behind it. [ADR 0180](../decisions/0180.md) § 14 is the record and
`rule:http-server/an-outbound-call-tries-every-approved-address` the rule.

`nvs_runtime::capability::install_resolver` is a **per-thread** hook, and `nvs-host` installs
`resolve_off_core` on every worker in `Worker::spawn`; a thread that is not a worker installs nothing
and resolves inline, which is what `nvs_host::blocking::run` does off a core anyway, so the fallback
is exact rather than merely safe. `PINNED_ADDRESSES` is `8`, applied in the resolver's order and
before the policy is asked, with the same address never kept twice. `pin_host_addresses` and
`pinned_addresses` answer the set; `pin_host`, `pinned_address` and `resolve_host` are its head. An IP
literal reaches no resolver at all.

Stage 12's second check is untouched: `Core\Http\Target` still carries one address and the transport
still connects to exactly that one. Nothing is blocked.

## Next group

**Stage 12: the approved set reaches the wire** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http/pool.rs`.

- [ ] **`Core\Http\Target` carries the approved set, at most eight, in the resolver's order** —
      `rule:http-server/an-outbound-call-tries-every-approved-address`, [ADR 0180](../decisions/0180.md)
      § 14 third paragraph. `pin` (`crates/nvs-stdlib/src/http.rs:380`) asks `pin_host_addresses`
      rather than `pin_host`; its callers are the launderer (`crates/nvs-stdlib/src/http.rs:435`), the
      redirect re-pin (`crates/nvs-stdlib/src/http.rs:2571`) and the `connectTo` path
      (`crates/nvs-stdlib/src/http.rs:2545`, which is a set of one). `TARGET`'s `ret` prose at
      `crates/nvs-stdlib/src/http.rs:181` still says "one address", and the class still has no
      members.
- [ ] **The connect falls back across the set, RFC 8305's way** — same rule, § 14 fourth paragraph.
      `Call::address` (`crates/nvs-stdlib/src/http/transport.rs:148`) becomes the set;
      `attempts`/`repin` (`crates/nvs-stdlib/src/http/transport.rs:1093`), the connect in `one`
      (`crates/nvs-stdlib/src/http/transport.rs:1313`) and the pool key
      (`crates/nvs-stdlib/src/http/pool.rs`) read it. Families interleaved, a `250ms` attempt delay as
      a constant, all under the one `connectTimeout`; every address failing is one `IOError` naming
      each. Sequential fallback in the same order is the fallback if the parking stream cannot hold
      two connects in flight.
- [ ] **A retry reuses the set and never re-resolves, and a pooled connection to any approved address
      serves the call** — `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`,
      `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`. The retry loop is
      `crates/nvs-stdlib/src/http/transport.rs:1268`; the pool's own case is
      `crates/nvs-stdlib/src/http/transport.rs:3230`. The five tests this owes are named at
      `docs/agent/loop-goal.toml:9651`.

## Backlog

- `Core\Net::connect` and `Core\Db::open` now have every resolved address judged, but still connect to
  the head of the set — falling back there is not in this goal's stages
  (`docs/agent/loop-goal.md` § *Stage 12*).
- The `nvs/rest` package, OAuth and a resolver of Novis's own stay out of this goal
  (`docs/agent/loop-goal.md` § *Standing decisions*, *Not this goal*).
- `[http.client]`'s `pool_idle`/`pool_idle_timeout` and `[http.client.tls]` ship the numbers
  [ADR 0180](../decisions/0180.md) § 16 fixes; nothing re-reads them per call.
