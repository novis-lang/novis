# Handoff

## State

**Stage 5's two capability cases are landed**, both in `crates/nvs-stdlib/src/http.rs`'s `mod tests`:
`the_address_policy_is_read_from_the_capability_and_not_from_the_client` asserts ADR 0058 § 5 as
*agreement* — the launderer and `nvs_runtime::capability::pin_host` answer the same two deployments
identically, refusal text included — and `a_denied_address_range_fails_before_a_connection_is_made`
binds a listener on the address the URL names and asserts it was never accepted, driven through
`request` so it measures the member's order and not the launderer's. The snapshot fixture both
capability suites grant through is now one writer, `nvs_stdlib::granting`, moved out of `process`.

**`examples/http.nvs` is green, verified by hand this session** — all five frozen lines over a real
socket, with `tools/origin.py` held open. **The driver's acceptance sweep still fails it**, and that
is the driver and not the tree: the running process imported `tools/loop.py` before `local_origin`
existed, so its sweep serves nothing on 8099. Restarting the run is the whole fix.

**Stage 5's remaining two cases are not tests over landed work.** `traceparent` is not implemented:
`crates/nvs-stdlib/src/http/transport.rs`'s `compose` sends no such header, `[trace] propagate`
exists in the config tree and is read nowhere, and nothing in the runtime holds a request trace id at
all — ADR 0018's call-site trace on `Ctx` is a different thing, which ADR 0076 § 2 says outright.

## Next group

**`traceparent`, and the reactor case.** They share one file set —
`crates/nvs-stdlib/src/http/transport.rs`'s `compose` and its `mod tests` (the in-process origin
helper is at `crates/nvs-stdlib/src/http/transport.rs:557`), plus the `[trace]` block in
`crates/nvs-config/src/tree.rs`.

- [ ] **A request trace id, and `[trace] propagate` read** — ADR 0076 § 2: an id exists for every
      request whatever the sampling decision, so it is the runtime's and not the client's. Decide
      where it lives on the context; `Core\Server::traceId()` reads the same id and is goal 6's, so
      leave the member out. `crates/nvs-runtime/src/ctx.rs:1120`,
      `crates/nvs-config/src/tree.rs:528`.
- [ ] **`an_outbound_request_carries_traceparent`** — ADR 0076 § 2, emitted from the composed head
      and asserted over what the origin thread was asked.
      `crates/nvs-stdlib/src/http/transport.rs:287`, `crates/nvs-stdlib/src/http/transport.rs:557`.
- [ ] **`a_socket_read_runs_on_the_reactor_and_parks_its_coroutine`** — ADR 0051: the read is
      `nvs_host::net`'s parking stream and not a second event loop.
      `crates/nvs-stdlib/src/http/transport.rs:237`, `crates/nvs-stdlib/src/http/transport.rs:259`.

## Backlog

- Restart the loop driver, or its sweep fails `examples/http.nvs` forever — `docs/agent/loop-goal.toml`.
- `orient.py` did not print ADR 0076 § 2, which both remaining cases are written from: add it to
  `[context] adrs` in `docs/agent/loop-goal.toml`.
- `https` is still refused for want of a TLS client and a trust anchor set — `transport::one`.
- `Core\Net` is governed by the same door and has no rows yet — ADR 0058 § 5.
