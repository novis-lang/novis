# Handoff

## State

**Stage 6's shared tier is on disk.** `Core\Cache::shared()` reads `[cache.shared] url`, pins its
host through `nvs_runtime::capability::pin_host` under `net.connect`, dials the store and answers a
`Core\Cache\Store` whose `tier` slot says `shared`; `put` and `get` dispatch on that slot.
`crates/nvs-stdlib/src/cache/redis.rs` is the wire — RESP `SET`/`GET` over `nvs_host::net::NvsTcp`,
one connection per core with one reconnect-and-replay behind each command — and it takes a
`SocketAddr` rather than a `Ctx`, so its two cases drive a whole exchange against a loopback
listener. § 2's named test is green, and its second half is a source scan: this module reaches
`nvs_runtime::encode`/`decode` from exactly one place each, so a tier cannot grow a carrier of its
own.

**Three decisions, each argued where it lives.** `shared()` is the **door** and the two operations
are behind it (`cache.rs`'s module doc), which is ADR 0058 § 4's shape and is why `Core\Cache\Store`
has no capability row. `Core\Cache::local` is now the one entry in `NEEDS_NO_CAPABILITY`, with its
reason in `docs/agent/loop-goal.md` § *Standing decisions* as ADR 0118 § 7 requires. And the URL
grammar is `redis://host[:port]` and nothing else — a password, a database index and `rediss://` are
each refused with a sentence rather than half-served.

**The driver's acceptance failure on `examples/cache.nvs` is the ordinary state, not a
regression.** That fixture is stage 6's *end*: it wants `shared hit` from a reachable store and two
`Core\RateLimit::consume` lines that have no member yet. Both are in the next group.

## Next group

**Stage 6's close, over `crates/nvs-stdlib/src/cache.rs`, `crates/nvs-stdlib/src/cache/redis.rs`,
`crates/nvs-stdlib/src/registry.rs` and a new `crates/nvs-stdlib/src/ratelimit.rs`.** The stage's
five `Core\RateLimit` test names are at `docs/agent/loop-goal.toml:2398`.

- [ ] **`Core\RateLimit::consume`, GCRA over the shared tier** — ADR 0075 §§ 2 and 5: the limit is
      an argument and no directive exists, the decision carries an exact `retryAfter`, and an
      unreachable store throws rather than deciding *allowed*. The one route to the store is
      `crates/nvs-stdlib/src/cache.rs:485` (`on_shared`, private today — this is the slice that
      makes it `pub(crate)`), and the connection it hands over is
      `crates/nvs-stdlib/src/cache/redis.rs:120`, which speaks `SET` and `GET` and nothing else: an
      atomic GCRA needs one more command beside them (`EVAL` with our own script, per the standing
      decision that the shared tier's atomic script is ours), added at
      `crates/nvs-stdlib/src/cache/redis.rs:148`'s `command`. The row and its card go beside
      `crates/nvs-stdlib/src/registry.rs:1308`.
- [ ] **The harness `examples/cache.nvs` needs, and its grant** — the fixture wants `shared hit`,
      so the acceptance run has to be serving a store, exactly as `tools/origin.py` serves 8099 for
      `examples/http.nvs`. The pattern to copy is `tools/loop.py:1125` (`local_origin`, one per leg,
      readiness is the harness's own stdout line), and the configuration is a fourth `[[app]]` block
      in the repo-root `nvs.toml`, beside the `examples/http.nvs` one about 96 lines in:
      `entry = "examples/cache.nvs"`, its
      `[app.capabilities.net] connect`/`internal` for the loopback, and `[cache.shared] url`. The
      frozen five lines are `docs/agent/loop-goal.toml:2410`.
- [ ] **The local tier's memory is the core's, and capped** — ADR 0059 § 3, the slice this session
      did not take. The entries are `crates/nvs-stdlib/src/cache.rs:302` and the write is
      `crates/nvs-stdlib/src/cache.rs:306`. **What makes it more than a cap:** a request is charged
      the *difference* between the thread's live balance now and at its `Ctx`'s creation
      (`crates/nvs-runtime/src/ctx.rs:1291`, baseline field at `crates/nvs-runtime/src/ctx.rs:314`),
      so an entry this request puts is charged to it against `[limits] memory` — which is exactly
      what § 3 says cache memory is not. Either that baseline gains a way to be shifted, in
      `nvs-runtime`, or the ADR's sentence is not implementable as written; decide that before
      writing the cap.

## Backlog

- `[context] adrs` never printed ADR 0059 §§ 1-3 — the item's *own* ADR — so this session read it by
  hand; add the three selectors to `docs/agent/loop-goal.toml`.
- `[context] modules` covers no `nvs-config` module, and a new directive needs `tree.rs` and
  `directive.rs`; add them beside the `nvs-config/src/capability.rs` line already there.
- A `[cache.shared]` password (`AUTH`) waits on ADR 0103 § 7's secret plumbing — `nvs_stdlib::cache`
  module doc.
- `rediss://` waits on the same trust-anchor decision `crate::http::transport` is waiting on.
- A TTL, an eviction and a `forget` on `Core\Cache\Store` — `nvs_stdlib::cache` module doc.
- Stage 7 (`Core\Fatal`, `Core\Log`) is untouched — `docs/agent/loop-goal.toml:2420`.
