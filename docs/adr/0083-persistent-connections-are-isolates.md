# ADR 0083 — A persistent connection is an isolate, and it is opened the way a script is spawned

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** WebSocket and Server-Sent Events — what a connection *is* in the execution model, how one is
  opened, how code inside it is written, how many connections talk to each other (`Core\Topic`), what
  bounds them, and what crosses the boundary. Not in scope: the wire protocols themselves (RFC 6455 framing
  and the SSE format are implementation, carried by the same `hyper` stack M7 already builds), and
  cross-*machine* fan-out, which § 4 places outside the runtime deliberately. The upgrade is **HTTP/1.1
  only**, where it is native — [0097](0097-development-server-and-proxied-origin.md) § 1 removes h2c, so
  RFC 8441 extended `CONNECT` is neither reachable nor needed.
- **Amends:** [0006](0006-isolated-script-execution.md) — `spawn script`'s `with(...)` clause gains a second
  caller in § 2 below, with identical grant, limit and argument rules; the isolate itself is the same
  `Isolate`, not a second isolation path.
  [0072](0072-core-task-structured-concurrency.md) § 6 — `afterResponse` and a connection isolate are the
  two things in Novis that outlive a response, and § 6 below states the difference so they are not confused,
  the way [docs/spec/00-overview.md](../spec/00-overview.md) § 2 does for `require` and `spawn script`.
  [0074](0074-http-defaults-safe-and-finite.md) — its "no spelling for an unbounded outbound wait" rule
  extends to a connection's idle, lifetime and send timeouts, all finite with nothing configured.
  [0059](0059-cross-request-state-is-explicit.md) — `Core\Topic` is a second explicit cross-request
  mechanism beside `Core\Cache`, and § 4 states why it is not built on it.
  [0051](0051-standard-library-tiers.md) § 3 — the Core roster gains `Core\Socket`, `Core\Sse` and
  `Core\Topic`.
  [docs/implementation-plan.md](../implementation-plan.md) — M7 gains the upgrade path and the topic bus.
- **Amended by:** 0084, 0097

> **In short:** an upgraded connection is **its own root isolate** — the same `Isolate` a request and a
> `spawn script` child already are, with its own memory, CPU and time budget, sharing nothing but compiled
> code. It is opened the way a script is spawned: `Core\Socket::upgrade('sockets/chat.nvs', with(args: …))`
> **names a file**, not a closure, so [ADR 0006](0006-isolated-script-execution.md)'s existing rules for
> grants, limits, arguments and path checking are reused whole and nothing crosses the boundary except
> values copied by [0023](0023-clone-serialize-and-cross-boundary-copy.md)'s graph copy. Inside, code is
> **an ordinary loop** — `while (var $msg = $conn->receive()) { … }` — because suspension has no colour, so
> there is no callback shape and no second execution model to learn. Connections talk to each other through
> **`Core\Topic`**, a runtime-owned publish/subscribe bus that reaches every core of the process, with a
> **bounded per-subscriber queue that closes a slow subscriber rather than blocking the publisher** — a
> fan-out must never become a denial of service. **SSE is the same thing with no `receive`**; a stream that
> ends with its request is just a streaming response and is not a connection at all.

## Context

- **Nothing in the first seventy-nine ADRs covers a connection that outlives a request**, and the request
  model is the sharpest thing in the runtime: a request is an isolate, it is torn down at the end, and only
  compiled code survives it ([0006](0006-isolated-script-execution.md),
  [0017](0017-hot-reload-without-restart.md)). A WebSocket is by definition the thing that does not end
  there, so it had to be designed rather than implemented.
- **It is not optional for the audience.** [ADR 0080](0080-the-audience-nvs-is-built-for.md)'s multi-tenant
  platforms want live dashboards, per-tenant push, progress streams and collaborative surfaces. A 2026 web
  language without a persistent-connection story reads as unfinished regardless of what else it has.
- **The obvious designs each import something Novis does not have.** A callback handler
  (`onOpen`/`onMessage`/`onClose`) is the shape every language with coloured async settles on, and Novis has
  stackful coroutines precisely so it does not need it. A dedicated connection worker pool is a second
  execution model to specify, secure and test. A connection that is "a request that never ends" makes the
  request budget meaningless.
- **The pieces for the right answer already exist.** `spawn script` is a construct for *run this file
  isolated, with these grants, these limits and this argument*, with path canonicalisation and root checking
  already argued. Stackful coroutines mean a blocking-shaped loop suspends for free. The arena gives an
  isolate a bounded heap. What was missing was the decision to point them at a socket.
- **Fan-out is where this design is actually hard.** One connection isolate sharing nothing with another is
  the correct default and also useless on its own — chat, presence and live dashboards are all
  *one-writes-many-read*. Any mechanism for that is a cross-isolate channel, which is exactly the thing
  [ADR 0059](0059-cross-request-state-is-explicit.md) makes explicit and bounded rather than ambient.

## Decision

### 1. A connection is a root isolate

On upgrade, the runtime creates a **root isolate** — not a child of the request tree — and moves the socket
into it. Everything true of a request isolate is true of it:

- Its own arena, its own globals and statics, its own copy of nothing. It shares only immutable compiled
  code ([0017](0017-hot-reload-without-restart.md)).
- Its own `[limits]` budget: memory, CPU time, output, and `spawn script` depth
  ([0005](0005-config-changeability.md)). A connection that exceeds one is closed with a defined code, the
  same way a request that exceeds one is terminated.
- Its own capability grants, narrowed from the request's — never widened
  ([0006](0006-isolated-script-execution.md)).
- Its own entry in the timeline and its own metric series
  ([0041](0041-timeline-export-and-gc-spawn-trace-events.md), [0076](0076-observability-export.md)).

**The request that upgraded it ends normally.** The connection is not a suspended request, does not hold the
request's arena, and cannot see the request's session, cookies or headers unless a value was explicitly
passed. This is the security property that makes the model worth its cost: authentication happens in an
ordinary HTTP request with all of [0074](0074-http-defaults-safe-and-finite.md)'s defaults applied, and what
reaches the long-lived isolate is whatever the application chose to hand it — an account id, a tenant id —
never the credential that proved it.

### 2. Opening one is `spawn script`-shaped

```php
#[Route(path: "/live/chat/{room}", method: Http\Method::Get)]
public static function chat(string $room): Http\Response {
    var $user = Web\Auth::require();                       // an ordinary authenticated request
    return Core\Socket::upgrade('sockets/chat.nvs', with(
        args:   {room: $room, userId: $user->id},
        limits: {memory: 8mb, idle: 5m},
        grants: {},                                        // narrowed: this socket needs nothing
    ));
}
```

- **The target is a file path**, resolved and root-checked exactly as
  [ADR 0006](0006-isolated-script-execution.md) resolves `spawn script`'s. It is not a closure: a closure
  would have to carry captured state across a boundary that exists to prevent exactly that, and
  [0031](0031-callable-is-the-only-closure-type.md) removed the `use` clause that would have made the
  capture visible.
- **`with(...)` is 0006's clause**, unchanged — `args`, `limits`, `grants`, `on`. `args` crosses by
  [0023](0023-clone-serialize-and-cross-boundary-copy.md)'s graph copy, so it is a value and never a shared
  reference.
- **A `secret` may not be passed**, per [0033](0033-secret-qualifier-for-confidential-values.md); a
  `tainted` value stays `tainted` on the other side, per
  [0024](0024-taint-tracking-for-injection-sinks.md).
- **Returning the upgrade is what performs it.** A handler that computes an upgrade and discards it does not
  upgrade, and the type system says so — the same shape as any other `Http\Response`.

### 3. Inside, it is a loop

```php
<?nvs
// sockets/chat.nvs — a connection isolate.
var $conn = Core\Socket::current();
var $room = Core\Script::args()->room as string;

Core\Topic::subscribe("room:" . $room);

while (var $msg = $conn->receive()) {                 // suspends; no colour, no callback
    Core\Topic::publish("room:" . $room, Core\Validate::text($msg->text, {max: 2000}));
}
```

- **`receive()` suspends the coroutine** and returns `?Socket\Message` — `null` when the peer closed. There
  is no handler interface, no event registration and no second control-flow style, because Novis's stackful
  coroutines make the straight-line loop the *simple* implementation rather than a nicer-looking one.
- **`send()` suspends until the frame is buffered**, and throws on the send timeout rather than waiting
  forever ([0074](0074-http-defaults-safe-and-finite.md)).
- **A received frame's payload is `tainted`** — it is untrusted input arriving over a network, exactly like
  a request body — and `Core\Validate` is the only way to launder it
  ([0024](0024-taint-tracking-for-injection-sinks.md) § 3, [0082](0082-the-first-party-framework.md) § 2).
- **Escalation is [ADR 0020](0020-error-escalation-ladder.md)'s ladder**, with the connection as the unit a
  fatal error tears down.

### 4. `Core\Topic` — the only way two connections meet

```php
Core\Topic::subscribe(string $topic): void
Core\Topic::publish(string $topic, mixed $value): uint      // returns subscribers delivered to
Core\Topic::unsubscribe(string $topic): void
```

- **Runtime-owned, in-process, across every core.** A publish from a connection on core 3 reaches
  subscribers on core 0. This is the one place the thread-per-core design is crossed on purpose, and it is
  a bounded message hand-off rather than shared state.
- **A published value is copied** by [0023](0023-clone-serialize-and-cross-boundary-copy.md)'s graph copy,
  so subscribers share nothing with the publisher or with each other.
- **A slow subscriber is closed, never tolerated.** Each subscriber has a bounded queue; when it overflows,
  **that subscriber's connection is closed** with a defined code and a metric increments. The publisher is
  never blocked and no queue grows without bound — a fan-out to ten thousand clients must not become a way
  for one of them to stall the other nine thousand nine hundred and ninety-nine, and this is priority 1
  ([0004](0004-memory-for-simplicity.md)), not a tuning choice.
- **A topic name refuses `tainted`**, the rule [0076](0076-observability-export.md) already applies to a
  metric label, and for the same reason: a name derived from user input is how one tenant subscribes to
  another's stream. A name is built from checked values or it does not compile.
- **A `secret` may never be published**, per [0033](0033-secret-qualifier-for-confidential-values.md).
- **It is not built on `Core\Cache`.** [ADR 0059](0059-cross-request-state-is-explicit.md)'s store is
  deliberately lossy — "it must always be correct to find nothing there" — which is right for a cache and
  wrong for a message that a subscriber is waiting on. Two mechanisms, two contracts, stated here so the
  next reader does not try to unify them.
- **Cross-machine fan-out is not the runtime's.** A fleet that needs a publish on one host to reach a
  subscriber on another bridges topics to a broker in application code. Building an inter-node bus would
  mean owning a distributed system's failure modes, which
  [ADR 0051](0051-standard-library-tiers.md)'s domain-logic rule says to take as a dependency, not write.

### 5. SSE is the same model without `receive`

`Core\Sse::upgrade(...)` takes the identical clause and produces a connection isolate whose `send` writes
events. The line that decides which construct to use is sharp and worth stating, because it is the one
place two spellings could appear for one job:

- **A stream that ends when the response ends is a streaming response**, which M7 already builds. Progress
  for one request, a large export, a chunked file. It is not a connection, it stays in the request isolate,
  and it is bounded by the request's budget.
- **A stream that outlives its request is a connection isolate.** Notifications, a live dashboard, anything
  a client keeps open across page lifetimes.

### 6. The two things that outlive a response, and how they differ

| | `Core\Task::afterResponse` ([0072](0072-core-task-structured-concurrency.md)) | A connection isolate (this ADR) |
|---|---|---|
| Belongs to | the request tree, kept alive past the connection | nothing — it is a root |
| Sees | the request's heap and values | only what `args` copied in |
| Budget | the request's remaining budget | its own, from `with(limits:)` |
| Ends | when its work finishes, bounded by `[deferred] max_concurrent` | when the peer closes, a timeout fires, or a budget is exceeded |
| For | finishing work the client need not wait for | talking to a client over time |

### 7. Bounds, and what happens on reload

- Every bound is finite with nothing configured: connections per process, connections per tenant
  ([0075](0075-core-ratelimit.md) applies to the upgrade request like any other), maximum frame size,
  maximum message size, idle timeout, total lifetime, send timeout, subscriber queue depth.
- **A connection isolate holds the compiled unit it started with.** An edit swaps the pointer for *new*
  connections; existing ones run to completion on the code they began with
  ([0017](0017-hot-reload-without-restart.md), [0042](0042-on-disk-artifact-cache-format.md)). This is the
  same rule a long request already follows, applied to a longer-lived thing, and it means a deploy does not
  break open connections — it drains them.
- **Graceful shutdown and `nvs ctl reload`** ([0078](0078-config-reload-and-control-socket.md)) close
  connections with a defined code after a drain period, so a client's reconnect logic sees a clean close
  rather than a reset.

## Consequences

- **Memory scales with concurrent connections, not with request rate** — an arena and a coroutine stack per
  connection. That is [ADR 0004](0004-memory-for-simplicity.md)'s trade taken knowingly, and it means
  sizing a deployment for a connection-heavy application means sizing for connections. The plan's
  *Consequences to accept* already says deployments are sized by concurrency; this adds a second axis to
  that sentence.
- **Ten thousand idle connections cost ten thousand arenas.** For the target audience's scale that is
  affordable; for a chat product with a million idle clients it is not, and Novis would be the wrong choice.
  Saying so plainly here is better than discovering it in a benchmark.
- **There is no way to broadcast without `Core\Topic`**, and no way for one connection to reach another
  directly. That is restrictive and it is the point — it is what keeps a connection an isolate rather than
  a thread with a socket.
- **A closed slow subscriber is a visible behaviour**, not a silent drop. Applications must handle
  reconnection, which they must anyway.
- **The upgrade is a file, so it participates in every existing mechanism** — the artifact cache, hot
  reload, grants, limits, coverage and tracing probes — with no special case in any of them. This is the
  main payoff of the `spawn script` shape and the reason it was chosen over a closure.

## Alternatives rejected

- **SSE only, WebSocket deferred.** Genuinely cheap — a streaming response is already built — and it covers
  dashboards and notifications, which is most real-time demand. Rejected because the design question is the
  same either way: the moment a stream outlives its request, the isolate question must be answered. Having
  answered it, WebSocket costs the framing and little else, and deferring would leave a checkbox unticked
  for no saving.
- **A callback handler interface** (`onOpen`/`onMessage`/`onClose`). Familiar from every coloured-async
  runtime. Rejected because Novis bought stackful coroutines specifically so that control flow could stay
  straight-line, and adopting a callback shape here would be the language arguing with itself.
- **A dedicated connection worker pool outside the request path.** Best raw throughput at very high
  connection counts. Rejected because it is a second execution model — its own scheduling, its own
  isolation story, its own security review — to serve a scale
  [0080](0080-the-audience-nvs-is-built-for.md)'s audience does not have.
- **A connection as a suspended request.** The smallest change, and superficially elegant. Rejected because
  a request's budget, teardown and observability all assume it ends; stretching that to hours makes every
  one of those meaningless, and the "request" would hold a session and a cookie jar for the lifetime of the
  socket, which § 1 exists to prevent.
- **Building `Core\Topic` on `Core\Cache`.** One mechanism instead of two. Rejected: a lossy store cannot
  carry a message somebody is waiting for, and blurring the two contracts would make both harder to reason
  about ([0059](0059-cross-request-state-is-explicit.md)).
- **A closure as the upgrade target.** Reads better at the call site. Rejected because it would either
  capture the request's heap — destroying the isolation this ADR is about — or need a rule for what a
  closure may capture across an isolate boundary, which is a new language question asked to save a file.

## Revisiting

- **If per-connection memory proves to be the binding constraint** — the ten-thousand-idle-connections case
  in *Consequences* — the thing to reconsider is whether an idle connection can release its arena and
  restore it on the next frame, not whether it is an isolate.
- **If applications routinely bridge `Core\Topic` to an external broker**, that pattern is a candidate for a
  first-party package under [0082](0082-the-first-party-framework.md) § 3 — still not a runtime feature.
- **If a third construct appears that outlives a response**, § 6's table is where it must be placed before
  it is built, so the confusion this repository heads off between `require` and `spawn script` does not
  reappear here.

## Verification

- **Isolation:** a connection isolate cannot read a variable, a static or a session value from the request
  that upgraded it; the state-bleed suite M7 already runs across request and isolate boundaries gains
  connections as a third parameterisation rather than a second suite.
- **Lifetime:** the upgrading request's arena is released while the connection is still open, asserted by a
  memory probe — the claim that a connection is not a held request.
- **Budgets:** a connection exceeding its memory, CPU or lifetime budget is closed with the defined code and
  reported as that, never as an out-of-memory; a connection whose isolate panics is contained exactly as
  `benches/abi-probe`'s existing containment tests require.
- **Fan-out:** a publish reaches subscribers on other cores; a subscriber that never reads is closed once
  its queue is full and the publisher's latency is unaffected, asserted with one deliberately stalled
  subscriber among many.
- **Qualifiers:** a received frame is `tainted` and fails to compile at a sink without laundering; a
  `tainted` topic name fails to compile; a `secret` passed through `args` or `publish` fails to compile.
  Each is a fixture that would not compile if the qualifier were wrong.
- **Reload:** an open connection survives an edit to its own source file and continues on its original
  compiled unit; a connection opened after the swap runs the new one.
- **Shutdown:** `nvs ctl reload` and graceful shutdown close connections with the defined code after the
  drain period, and no connection isolate outlives the drain.
- **Bounds:** every timeout and cap in § 7 has a default that applies with nothing configured, asserted the
  way [0074](0074-http-defaults-safe-and-finite.md)'s defaults already are.
