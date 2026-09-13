# Handoff

## State

**Goal `http-client` — stage 12's door now answers the approved set.** `Core\Http\Target`'s second
slot is `addresses`, an array of address texts in the resolver's order and at most eight
(`rule:http-server/an-outbound-call-tries-every-approved-address`); `pin` asks `pin_host_addresses`,
`named_address` and an IP literal answer the set of one, `repinned` re-pins a hop to a set, and
`approved` reads the slot back through `addresses_of` (`crates/nvs-stdlib/src/http.rs:2683`). The
class still has no members. Stages 1–11 are on disk behind it.

**Where the set stops:** `transport::Call` carries `addresses: Vec<IpAddr>` and `sent` walks it to
`one`, which connects to the first of the set (`crates/nvs-stdlib/src/http/transport.rs:1324`).
Falling back across the rest is the next group.

**What the next group has to decide first:** `nvs_host::NvsTcp` has only `connect` and
`connect_timeout` (`crates/nvs-host/src/net.rs:297`, `:318`), both of which park on one address
until it is up, so RFC 8305's *next attempt started after a fixed delay* has no seam to stand on —
`connected` (`crates/nvs-host/src/net.rs:775`) is the single-socket park it would generalise.
That is why the group below opens in `nvs-host` and not in the transport.

Nothing is blocked. The floor's `examples/queue.nvs` failure the driver reported after session 0008
was the Docker daemon having restarted without the test servers, not a regression; they are up and
the example prints its five lines again.

## Next group

**Stage 12: the connect falls back across the set** — one file set: `crates/nvs-host/src/net.rs`,
`crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **A connect that races several addresses and answers the first one up** —
      `rule:http-server/an-outbound-call-tries-every-approved-address`, [ADR 0180](../decisions/0180.md)
      § 14 fourth paragraph. `connect_timeout` (`crates/nvs-host/src/net.rs:318`) is the
      one-address spelling and `connected` (`crates/nvs-host/src/net.rs:775`) the park it wraps; the
      new one starts an attempt per address a fixed delay apart, keeps the first writable socket and
      closes the rest, all inside the one budget it is given.
- [ ] **`one` walks the approved set under the one `connectTimeout`** — same rule. The clamp is at
      `crates/nvs-stdlib/src/http/transport.rs:1358` and the connect it feeds at `:1361`; the
      first-of-the-set narrowing at `crates/nvs-stdlib/src/http/transport.rs:1324` goes when the walk
      lands. Every address failing is one `IOError` naming each. Tests
      `a_dead_first_address_falls_back_to_the_next_within_connect_timeout` and
      `every_address_failing_is_one_io_error_naming_each`.
- [ ] **A retry reuses the set and a pooled connection to any approved address serves the call** —
      same rule, and `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`
      for the key. `attempts` (`crates/nvs-stdlib/src/http/transport.rs:1271`) already re-enters
      `one` with the set rather than re-resolving; `pool_key`
      (`crates/nvs-stdlib/src/http/transport.rs:1446`) is keyed on one socket, so serving a call from
      any approved address is a lookup per address. Tests
      `a_retry_reuses_the_approved_set_and_never_re_resolves` and
      `a_pooled_connection_to_any_approved_address_serves_the_call`.

## Backlog

- `a_slow_lookup_leaves_the_core_free_for_another_task` — stage 12's fifth stdlib test, unwritten;
  it needs a resolver a test can make slow, which the seam `install_resolver` already is
  (`docs/agent/loop-goal.toml:9656`).
- `[context] rules` in `docs/agent/loop-goal.toml` does not name
  `http-server/an-outbound-call-tries-every-approved-address`, which is the rule stage 12 is
  written against — the pack printed six others and not it, and this session read it by hand.
- Stage 13, the `http` trace event (`docs/agent/loop-goal.toml:9668`).
- The `nvs/rest` package and OAuth stay unscheduled ([carried-gaps.md](carried-gaps.md)).
