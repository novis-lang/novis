# Handoff

## State

**`examples/http.nvs` is green — all five frozen lines, over a real socket.** The origin it names is
the harness's: `tools/origin.py`, one route (`/ok` → `200 ok`), started per leg by `local_origin` in
`tools/loop.py:1110` and held open across the fixtures *and* the valgrind sweep. The Windows leg gets
one; the WSL leg gets a second one inside the distro, because a WSL loopback is its own. Why the
harness serves it rather than the example spawning it is recorded beside the stage-5 check in
`docs/agent/loop-goal.toml` (and in the staged copy `docs/agent/goals/4-core-part-ii.toml`); the 25ms
hold on every answer — what makes the `deadline hit` line arithmetic rather than a race against a
sub-millisecond round trip — is argued in `origin.py`'s own module doc.

**The running driver will not pick this up.** It imported `loop.py` at start, so its acceptance sweep
still has no origin and `examples/http.nvs` keeps failing until the run is restarted. Verified by
hand on both legs instead: native prints the five lines, and the WSL-side origin binds and shuts down
on stdin EOF (the WSL leg's own binary is stale at `/var/tmp/nvs-target-wsl`, which the driver
rebuilds).

**Stage 5's remaining hole is four named tests**, not the example: of the six in the `nvs-stdlib (the
client and the socket)` check at `docs/agent/loop-goal.toml:2338`, only
`a_redirect_is_re_checked_against_the_same_policy` and
`a_retry_is_jittered_and_shares_the_covering_deadline` exist.

## Next group

**The four missing stage-5 `nvs-stdlib` tests.** They share one file set —
`crates/nvs-stdlib/src/http/transport.rs`'s `mod tests` (the in-process origin helper and `call` are
at `crates/nvs-stdlib/src/http/transport.rs:600`), `crates/nvs-stdlib/src/http.rs:285`'s launderer,
and the policy behind `crates/nvs-runtime/src/capability.rs:118`.

- [ ] **`the_address_policy_is_read_from_the_capability_and_not_from_the_client`** — ADR 0058 § 5.
      The client passes an address it was handed; the refusal is the capability's.
      `crates/nvs-runtime/src/capability.rs:118`, `crates/nvs-stdlib/src/http.rs:285`.
- [ ] **`a_denied_address_range_fails_before_a_connection_is_made`** — ADR 0058 § 3: the assertion is
      that *no socket was opened*, not that an error was thrown.
      `crates/nvs-stdlib/src/http.rs:285`, `crates/nvs-stdlib/src/http/transport.rs:176`.
- [ ] **`an_outbound_request_carries_traceparent`** — ADR 0076 § 2, over the composed request head at
      `crates/nvs-stdlib/src/http/transport.rs:287`.
- [ ] **`a_socket_read_runs_on_the_reactor_and_parks_its_coroutine`** — ADR 0051: `nvs_host`'s parking
      stream rather than a second event loop. `crates/nvs-stdlib/src/http/transport.rs:230`.

## Backlog

- **A `.nvst` case for the transport's success path has no home in `tests/conformance/`** — that
  suite runs on three hosted CI runners and under `verify.py`, none of which has an origin on 8099.
  The success path is `examples/http.nvs` plus `transport.rs`'s in-process tests; a conformance case
  would be the one network-dependent case in the tree. Recorded here rather than left as an item.
- Stage 6's two stores, once stage 5's four tests land — `docs/agent/loop-goal.toml:2364`.
- `docs/plan/m8.md` § *Verify* still owes the "no class outside Tier 0 registers a `Core\` name" CI
  check.
