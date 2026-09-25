---
milestone: M7
---
# Loop goal 6 — the server, and the parity program's last gate

Finish **M7** — [docs/plan/m7.md](../../plan/m7.md) is the scope and this file does not restate it.
`nvs serve` accepts a request, dispatches it into a **root isolate of a request tree** — the same
`Isolate` goal `concurrency` built, not a second isolation path — and answers it.

This is goal `server` of the chain and the **last goal of the parity program**
([goals/README.md](README.md)). Everything the request-facing half of `Core` was waiting on now exists,
and `python tools/check-migration.py` reaching **100% classified** is this goal's final stage and the
program's stop condition.

## Two things every session must hold

**`hyper`, and how it runs with no async runtime.** `hyper` with `default-features = false, features =
["http1", "server"]` depends on `http`, `http-body`, `bytes`, `futures-core` and `pin-project-lite` — and,
measured rather than assumed, on `tokio` as well: 1.11 takes it unconditionally at `features = ["sync"]` for
one `oneshot` in its upgrade path, which is a channel library and not a runtime. **No `rt`, no `net`, no
`time`, no executor, no `spawn`**, so `rule:concurrency/one-scheduler`'s rule holds and the workspace `Cargo.toml`'s comment above
the dependency owns that reading. h1 requires no `Executor` and `serve_connection` spawns nothing, so the connection future is
driven by a **`block_on` on the coroutine that owns the connection** — a waker that marks the coroutine
ready, poll, park on `Pending` — over `hyper::rt::Read`/`Write` adapters wrapping goal `concurrency`'s parking stream.
That is one polled future per connection and not a second scheduler, so `rule:concurrency/one-scheduler`'s rejection of tokio's
task primitives is untouched. Hand-rolling h1 was weighed and refused: `docs/plan/design.md` gives the
reason about FCGI and it applies here — framing is where request smuggling lives, and it is not a parser
to own.

**A filesystem path is never derived from a URL at request time.**
`rule:http-server/a-path-is-never-derived-from-a-url` is the server's governing rule, and
§ 4's five-step resolution is how it is kept: a request selects a **mount** from a table whose globs were
expanded against disk **at boot**. The test that says the rule holds is not a traversal fixture — it is the
assertion that **the set of paths the server can execute after boot equals the expanded mount table**, and
that is stated in Stage 9 rather than left to a suite of attempted escapes.

## Stage 0 — the catch-up

Nothing. Every deferred half this goal picks up — `Core\Router::match`, `Core\Session`, `Core\Metrics`,
`[http.*]`'s runtime behaviour, `nvs ctl` — was deferred *to* this goal by name, in the goal that deferred
it, and is work rather than debt.

## Stage 1 — the floor

M4's and goals `core-depth` through `database`'s whole acceptance lists — five goals deep, **never traded.** This is the goal where
that matters most: a listener is where an old assumption about isolation, capabilities or the graph copy
gets its first adversarial traffic.

## Stage 2 — the keystone: one connection, one request, one isolate

1. **`crates/nvs-server` exists, and `nvs serve` answers one request.** Per-core accept and dispatch, a
   connection on a coroutine, `hyper` h1 over the `block_on` above. **No mount table, no routing, no
   response policy yet** — just the path from a socket to a root isolate and back.
2. **The request is the root isolate of a request tree**, and it is goal `concurrency`'s `Isolate`. m7.md says "not a
   second isolation path" and that is the item: if this stage grows its own isolation, the state-bleed
   suite in Stage 9 is testing two mechanisms and proving neither.
3. **`Core\Request` and `Core\Server`, populated from it** — `rule:statements/no-host-populated-variables`'s
   replacement for `$_GET`/`$_POST`/`$_SERVER`/`$_COOKIE`/`$_FILES`. **Every value originating outside the
   process is `tainted`** (`rule:security/tainted-qualifier`), and that is
   not decoration: goal `core-part-ii` built every launderer, and this is the stage that gives them something to launder.
4. **The shapes that are rules, not fields.** `method` reports `Get` for a `HEAD` request so a `Get`-only
   route table still matches, with `isHead` carrying the truth; `clientIp` and `scheme` resolve from the
   socket peer **unless a peer in `[server] trusted_proxies` asserted otherwise** (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`); `path` is
   the request path with the matched mount's prefix **removed**, and `mount()` is what was removed.

## Stage 3 — the mount table

5. **The mount table, expanded at boot** — `rule:http-server/a-mount-table-expands-at-boot`, and § 4's five-step resolution over it. This is
   what makes several entry points under one document root, and vhost-per-module, cost one line each.
6. **Prefix stripping and the relocatable module it buys**, and **static-file serving as one policy in both
   modes** — the development server and the proxied origin differ in what they serve, not in how they
   decide.
7. **The `[server]` block**, § 5: four finite idle timeouts, `max_in_flight` with its pre-allocation `503`,
   the optional health path, and `Core\Server::isDraining()`. `max_in_flight` is **the result of an
   arithmetic against the memory budget** rather than a number someone picked — `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` amended § 5 to say
   so, and picking a number is the regression.
8. **A mount routes and carries nothing else; policy is the per-app block's** — § 10. A mount that grows a
   limit or a grant has re-implemented goal `governance`'s `[[app]]`.

## Stage 4 — the response

9. **`Core\Response`'s body surface is five typed members** — `html`, `json`, `text`, `bytes`, `sendFile` —
   each setting its own `Content-Type`, with `echo` the **HTML-only sixth path** and **mixing the two a
   compile error** (`rule:security/response-body-is-one-typed-member`).
   This is where a JSON body stops being an `echo` the auto-escape sink would corrupt.
10. **The `echo` binding table is enforced from here** — § 3. The HTML sink is attached **by a request and
    by nothing else**, so a scheduled script's and an isolate's `echo` take the terminal sink's
    neutralization instead. Goal `core-part-ii` built that terminal sink; this is what decides which one is attached.
11. **The response policy applies with nothing configured** —
    `rule:http-server/secure-headers-with-nothing-written`, `rule:http-server/cors-is-closed-until-origins-are-named`, `rule:http-server/cookies-are-secure-httponly-and-lax` and `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`: secure headers, closed CORS,
    `Secure; HttpOnly; SameSite=Lax` cookies, every directive `Runtime` so a request may change it for
    itself and `setHeader` still wins. Goal `governance` landed the *boot-time* refusals; this is the runtime half.

## Stage 5 — routing, sessions, uploads

12. **`Core\Router::match`, over the table goal `core-depth` compiled**, plus `methodsFor` and `urlAbsolute`. `rule:routing/matched-once-before-the-handler`
    : **the match happens once, before the handler, and travels on the request** as
    `Core\Request::route()` — which is what the CSRF check and the `route` metric label read rather than
    matching again. § 2: a missing path and a refused verb are different answers (empty ⇒ 404, else 405 +
    `Allow:`).
13. **§ 7's mount captures are how one table serves many tenants**, and § 8's split: CSRF is the server's,
    the access decision is the dispatcher's.
14. **`Core\Session`**, which **may not be backed by `Core\Cache`'s local tier** — `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` names it as
    a hole that tier must not fill, and a session that vanishes because a core evicted it is an
    authentication bug.
15. **Uploads** — `rule:http-server/an-upload-is-received-only-through-files`
    whole: `files()` is a lazy iterator and **the only way to receive an uploaded file**; a part is a file
    part iff `Content-Disposition` carries `filename`; three ways to consume one; goal `core-part-ii`'s
    `Core\IO::writeStream` is where it reaches disk; **there is still no temp file and no
    `move_uploaded_file`**. Its two caps, `request_body` and `upload_total`, are new rows in goal `governance`'s
    `[limits]`/`[limits.hard]` pair, and `upload_total` is refused **pre-dispatch** when `Content-Length`
    already exceeds it.
16. **`bodyStream()` is the raw-body alternative to `body`**, exclusive with it and with `files` on one
    request (`rule:http-server/a-mount-table-expands-at-boot`, `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`, `rule:http-server/head-runs-as-get` and `rule:http-server/the-body-is-read-on-demand-under-two-caps`).

## Stage 6 — what runs beside a request

17. **The `[[schedule]]` ticker** — `rule:config/scheduled-work-is-a-config-block`: each entry
    fires as a **root** isolate through goal `concurrency`'s `Isolate`, with the fleet lease over goal `core-part-ii`'s shared
    store. Goal `governance` landed its boot-time validation; this is the runtime half.
18. **`Core\Task::afterResponse`'s tree stays alive past the connection**, bounded by `[deferred]
    max_concurrent` — `rule:concurrency/after-response-outlives-the-connection` and `rule:concurrency/deferred-is-bounded-by-two-directives`. Goal `concurrency` built the member under compiled-in defaults; this is where
    the connection actually ends while the tree does not.
19. **The observability export** — `rule:observability/the-runtime-exports-what-it-already-measures`: `Core\Metrics`, the
    default series, W3C `traceparent` **inbound**, with a trace id generated for every request **whether
    sampled or not**, and spans derived from `rule:observability/trace-events-carry-a-kind`'s existing event kinds **with no probe added to `rule:testing/debug-probes`'s measured path**. Goal `core-part-ii` built the outbound half; this closes the loop.

## Stage 6b — persistent connections

`rule:concurrency/a-connection-is-a-root-isolate` whole. [m7.md](../../plan/m7.md) places
it in this milestone and no stage above carries it. A connection is a **root isolate** opened by a request
that then ends normally, so this stage adds a lifetime, not an isolation path — and item 25's state-bleed
suite gains connections as its third parameterisation rather than a second suite.

19a. **The upgrade seam.** `hyper`'s `on_upgrade` hands back the `Upgraded` io, which downcasts to the
    coroutine's own `NvsStream`; RFC 6455 framing is `tungstenite` over that stream — it is a plain
    `Read + Write`, so the sync crate fits with no adapter — with its `max_frame_size` and
    `max_message_size` set from § 7's caps. Framing is not owned, for the reason h1 is not: it is where the
    smuggling-class bugs live. The connection isolate is goal `concurrency`'s `Isolate` with the socket moved in, and
    **the upgrading request's arena is released while the connection is open** — the memory probe in the
    ADR's *Verification*, and the claim that a connection is not a held request.
19b. **`Core\Socket::upgrade` and the entry rule it shares with `spawn script`.** `upgrade` takes `rule:security/isolate-shares-nothing`'s
    operand — a file path, or a static method — and 0006's options as ordinary named arguments (`args:`,
    `limits:`, `grants:`, `on:`), and returning it is what performs it. The operand's method half lands
    **first at `spawn script`**: the parser, the type check that binds `args:` to the entry's parameters by
    name and refuses an `fn` literal or a `callable`-typed variable with a diagnostic naming the method
    form, and a function→`Program` arm beside `program_over` in `crates/nvs-cli/src/script.rs`; `upgrade`
    then reuses all three rather than growing a check of its own.
19c. **`Core\Socket::current`, `Socket\Message`, `send`, `receive`, `close`** — `rule:concurrency/a-connection-is-a-loop`. `receive()`
    is **the one wait**, over the peer *and* the connection's subscribed topics, answering a peer frame
    (payload `tainted`) or a topic delivery (the copied value and the topic's name); there is no
    `Core\Topic::receive()` and no two-task scaffold in a connection script. `send` suspends until the frame
    is buffered and throws on the send timeout.
19d. **`Core\Topic`** — § 4, the one place thread-per-core is crossed on purpose. A publish serialises
    once with Stage 5's byte carrier (goal `concurrency` item 16) and each subscriber unserialises into its own arena;
    the wake across cores is `Reactor::remote_wake`; the per-subscriber queue is bounded and **overflow
    closes that subscriber with a defined code**, never blocking the publisher. A `tainted` topic name and
    a `secret` value are compile errors, the fixtures goal `core-part-ii`'s qualifier suite already has a shape for.
19e. **`Core\Sse::upgrade`** — § 5: the same isolate with no `receive`, and the line it draws — a stream
    that ends with its request is a streaming response (Stage 4) and stays in the request isolate.
19f. **Bounds and lifetimes** — § 7: connections per process, frame and message size, idle, lifetime and
    send timeouts, subscriber queue depth — every one finite with nothing configured, on the timer table
    goal `concurrency` built, asserted the way `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s defaults are. A connection exceeding its memory, CPU or
    lifetime budget closes with the defined code and reports as that, never as an out-of-memory.
19g. **Reload and drain** — § 7, over items 20 and 22: an open connection keeps the compiled unit it began
    with and one opened after the swap runs the new one; `nvs ctl reload` and graceful shutdown close every
    connection with the defined code after the drain period, and none outlives it.

## Stage 7 — the operator's surface

20. **The control socket and `nvs ctl`** — `rule:config/one-local-control-socket` and `rule:config/no-network-control-surface`
    : a local unix socket (named pipe on Windows), created `0600`, **refused if its directory is
    world-writable**, speaking HTTP so a network listener would later be a second `bind` rather than a
    second protocol. `nvs ctl reload` is its **only** operation and there is **no control port in either
    direction of configuration**. Goal `governance` built the snapshot this swaps.
21. **`nvs service`** — `rule:packaging/a-service-is-one-stored-argv`,
    the only copy. SCM registration on Windows with the hosted argv in a quoted absolute `ImagePath`, a
    per-service virtual account, `STOP_PENDING` from the graceful drain and `PARAMCHANGE` into the reload;
    a printed hardened systemd unit on Linux, written to disk only on an explicit `--install`. **The
    installer is a sink and fails closed**: a closed `serve`/`run` allowlist, no relative path, no install
    whose output would go nowhere, no password on a command line, and a refusal to install from an `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
    bundle.
22. **Hot-reload of the compiled-unit cache** — `rule:config/an-edit-reaches-the-next-request-without-a-restart`,
    the only copy: a per-path pointer over goal `governance`'s content-addressed cache, revalidated lazily and
    rate-capped, **swapped without ever blocking a request-serving core**, with `validate`'s startup default
    selected by the run mode. This is what makes "no restart to see an edit" true of a running server.

## Stage 8 — the testing surface the server unlocks

23. **`Core\Test::request`'s in-process dispatch through the compiled route table**, `#[Test(db:)]`'s
    rolled-back transaction, `#[Test(server: true)]`'s ephemeral listener, and inline snapshots with their
    source updater — `rule:testing/inline-snapshots`, `rule:testing/db-transaction` and `rule:testing/in-process-request`. Each waited
    for a capability that now exists, and `#[Test(db:)]` waited for goal `database`.

## Stage 9 — the load-bearing assertions, and the program's last gate

24. **10k concurrent cold requests for the same file compile it exactly once**, asserted via a compile
    counter, with no stalled requests. This is m7.md's *core requirement* and it is the one number the
    whole hot-reload design exists to make true.
25. **A state-bleed suite proves nothing leaks between requests, and the same suite runs across an isolate
    boundary** — which the shared `Isolate` makes a *parameterisation* rather than a second suite. If it is
    two suites, item 2 was not done.
26. **The set of paths the server can execute after boot equals the expanded mount table.** `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s
    governing rule, stated as a test rather than as a suite of attempted escapes.
27. **A multipart body far larger than any in-memory bound is received in full at bounded resident
    memory**, asserted against a high-water mark — `rule:http-server/an-upload-is-received-only-through-files`'s load-bearing case.
28. **Path traversal, header injection and request-smuggling suites pass**, and a request whose isolates
    are still running when the client disconnects leaves none of them behind.
31. **Live bytes are O(in-flight) under a cycle-building load.** A soak of many thousands of requests,
    each building object cycles, holds a flat live-byte measure across the run — the server-side proof of
    `rule:security/isolate-teardown-is-a-drain-then-a-sweep`'s teardown sweep, which goal
    4's stage 11 lands. Added when the drain-only teardown was found to retain cycles for the
    life of the process; numbered out of sequence because item 30 was already written as the program's
    last gate and stays it.
29. **`wrk`/`oha` throughput against PHP 8.5 + FPM + opcache, recorded in `benches/`.** A number, committed.
30. **`python tools/check-migration.py` reports 100% classified.** Every one of the oracle build's 1167
    functions and 255 types is a `member`, `language` or `dropped` row; every `member` row's member is
    registered; every one of them has a conformance case. **This is the parity program's stop condition**
    and the last check in the chain.

## The harness this goal owes

**`python tools/bench.py --serve-vs-fpm --record benches/serve.json`** — item 29. m7.md asks for the
number to be *recorded*, not merely produced, so the flag writes it and the check asserts it was written.
PHP 8.5 is already on this machine and in the WSL distro as the differential oracle; FPM and opcache are
what this adds.

## Acceptance

**This goal is retired: its checks are the floor stage of the live goal**, carried
there by the switch that left it and folded forward at every switch since.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.**
- **One ADR slot: the `block_on` seam** (Stage 2, item 1), and it is that stage's first slice. How a
  `hyper` connection future is driven from a coroutine, what the waker does, what happens when the future
  wakes on a core other than the one that parked it, and why this is not an executor. Every other design in
  this goal is already argued — 0012, 0017, 0072, 0073, 0074, 0076, 0077, 0078, 0079, 0088, 0093, 0097,
  0102, 0105.
- **`hyper` stays, and h1 only.** No TLS listener and no h2c — `rule:http-server/two-deployments-and-nothing-a-proxy-owns` dropped both, and a proxy
  terminates TLS. If a capability appears to need h2, that is Backlog, not a scope decision.
- **One isolation path.** The request is goal `concurrency`'s `Isolate`. A second one makes Stage 9's state-bleed suite
  meaningless, which is why item 2 is stated as an item rather than assumed.
- **`max_in_flight` is an arithmetic, not a number.** `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` amended `rule:http-server/the-server-block-is-boot-class` to say so.
- **`tungstenite` is the framing crate**, sync, over `NvsStream` with no adapter, picked under `rule:packaging/a-c-dependency-answers-two-questions`
  's pre-authorization; owning RFC 6455 is refused for the reason owning h1 is.
- **`receive()` selects over both sources** — `rule:concurrency/a-connection-is-a-loop` — and an isolate's entry is a path or a
  static method with `args:` bound to its parameters — `rule:security/isolate-shares-nothing`. Both are decided in those bodies; a
  session that wants a `Core\Topic::receive()`, an `fn` literal entry or a capturing closure has found the
  decision, not a gap.
- **`Core\Session` may not use the local cache tier.** `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`.
- **A mount routes and carries nothing else.** Policy is the per-app block's, which goal `governance` built.
- **`nvs ctl reload` is the socket's only operation**, and there is no network-reachable control surface in
  either direction of configuration. `rule:config/no-network-control-surface`.
- **No session installs a service, and item 21's checks are deliberately not end-to-end.** Registering
  with the SCM needs administrator rights the loop does not have and should not be given, and a systemd
  unit written to disk on an unattended box is a change nobody asked for. What is checked is what can be
  checked without either: **every refusal** — the closed `serve`/`run` allowlist, a relative path, a
  password on a command line, an install whose output would go nowhere, an `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` bundle — plus the
  *shape* of what would be installed: a quoted absolute `ImagePath` on Windows, a printed unit on Linux
  with `--install` withheld. That is the whole of `rule:packaging/a-service-is-one-stored-argv`'s *Verification* that does not require a
  privileged machine, and a session that finds the coverage thin has found this decision rather than a
  gap. Real installation is a manual gate, fired by the user on a machine they chose.
- **Raw/unparsed body access for an arbitrary content-type is an open gap**, flagged by `rule:security/tainted-qualifier`'s
  *Revisiting* and narrowed by m7.md to what `body()` and `bodyStream()` do not already answer. If a
  session finds it genuinely needed, that is a decided-and-recorded call in `Core\Request`'s module doc —
  not a new ADR and not a `BLOCKED`.
- **Picking every dependency but the two the user named** stays pre-authorized under `rule:packaging/a-c-dependency-answers-two-questions`.

## What this goal does not touch

The extension system (M9), `nvs fmt` and the editor (M4B and M10), the transpiler (M11), packages (M15)
and the `nvs/web` package (M16). `Web\Migration` stays blocked by `rule:programs/no-migration-runner`. A session that reaches one
of these puts it in `## Backlog` and moves on — and when the last check here goes green, the parity
program is finished and the chain has no next goal.
