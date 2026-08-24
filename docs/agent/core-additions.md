# Queue: `Core` additions decided with the user, awaiting ADRs

**This file is a queue, not a home.** Each entry below was decided with the user on 2026-08-24 in a session
that reviewed what modern web applications need from userland every day, and asked which of it belongs in
`Core`. None of it is written up yet. **An entry is deleted from this file the moment its ADR lands** — at
which point the ADR body is the only home for the rule, per `AGENTS.md`. If this file is empty, delete it.

Entries are ordered by the order they should be written, which is roughly leverage per unit of surface.
**Numbers are stable across deletions**, so a gap means that entry's ADR has landed — entry 1, derived
codecs, is [ADR 0071](../adr/0071-derived-codecs.md).
Each names the ADRs it amends, the Rust crate that does the work, and the design points the ADR still has
to settle — those are the parts the session did **not** decide, and guessing them is how an overlay gets
written instead of a fold.

## How the ten candidates were filtered

Two tests did the work, and the ADR for each entry should restate the one that admitted it rather than
arguing usefulness: [ADR 0051](../adr/0051-standard-library-tiers.md) § 2's six ordered tests, and
[ADR 0060](../adr/0060-application-security-protocols.md) § 2's three-part test for anything protocol-shaped.
Everything below is admitted by a test. Everything in *Rejected* failed one.

---

## 2. `Core\Task` — the concurrency shape people actually use

**Decision.** `Core\Task::all` and `::map`, each taking `{limit, deadline}`. The first throw cancels its
siblings and propagates; the deadline cancels everything and throws `TimeoutError`. Control does not leave
the call with work still running. **No open-ended scope object** — deliberately deferred, and adding one
later is purely additive.

Plus `Core\Task::afterResponse(callable, {deadline})`: work that runs after the response is on the wire.
Memory stays charged to the request tree, so [ADR 0004](../adr/0004-memory-for-simplicity.md)'s
"attributable, O(in-flight)" holds unchanged — the *connection* closes, the request tree does not. An
uncaught throw inside it goes through [ADR 0020](../adr/0020-error-escalation-ladder.md)'s ladder to
`Core\Log` carrying the scheduling request's trace id; there is no response left for it to affect.

**It is not a queue, and the ADR must say so.** Nothing is durable, nothing retries, a lost process loses
the work. Receipts, webhooks, cache warming, reindexing, audit shipping: yes. Anything that must happen:
inside the transaction, or in a durable queue the application owns.

**Home:** M5's concurrency surface, which [spec 00](../spec/00-overview.md) § 2 explicitly defers. Relates to
[0006](../adr/0006-isolated-script-execution.md), [0004](../adr/0004-memory-for-simplicity.md),
[0020](../adr/0020-error-escalation-ladder.md).

**Crate:** none, and none is possible. `tokio::JoinSet`/`Semaphore` are the prior art and are **unusable
here** — they are built on tokio's async model, and this runtime is `corosensei` stackful coroutines on its
own thread-per-core scheduler. This is a few hundred lines over M5's own primitives.

**Still to settle:** whether `Task::all` over a shape literal binds each field's type from its own closure
(the nice form) or returns a uniform type; what cancellation does to a coroutine blocked in a `Core\Db`
call; `[deferred]`'s directive names and their changeability classes; what `max_concurrent` does when
exceeded (the session assumed: throws rather than queueing without bound).

## 3. The `[schedule]` block

**Decision.** Cron-declared work in `mwl.toml`, firing a `spawn script` — no API surface at all. Keys:
`cron`, `script`, `timezone`, `limits`, `overlap` (`skip`/`queue`/`kill`), and **`scope`, which is
mandatory with no default**: `"fleet"` runs once across the deployment, `"host"` once per host. The user
chose to require it explicitly, because both are commonly correct and either default surprises somebody in
production.

**Home:** [0064](../adr/0064-configuration-file-format.md) gains the block; [0005](../adr/0005-config-changeability.md)
gains its changeability classes. Relates to [0006](../adr/0006-isolated-script-execution.md) — a scheduled
script is an ordinary isolate, and `mwl run` runs the identical file by hand.

**Crate:** `croner` or `cron` parses the expression; `jiff` or `chrono-tz` for the zone. The fleet lock is
ours, over the shared store.

**Still to settle:** whether `scope = "fleet"` refuses to boot when no shared store is configured (the
session's assumption: yes, rather than silently degrading to one run per host).

## 4. HTTP defaults are safe and finite, in both directions

**Decision, inbound.** A `[http.cors]`, `[http.headers]` and `[http.cookies]` block in `mwl.toml`, enforced
by the M7 server, with **secure defaults on** — `nosniff`, HSTS, a frame-ancestors deny, and
`Secure; HttpOnly; SameSite=Lax` cookies apply with no config written at all. CORS stays closed until
origins are named. `origins = ["*"]` together with `credentials = true` is **refused at boot and at
runtime alike**, returning `false` and leaving the value unchanged per ADR 0005.

Every directive is **`Runtime`** class: a request may change or disable any of it *for itself*, and the
change is discarded when the request ends. `Core\Response::setHeader` also overrides a policy header on one
response with no config involved. The user asked for this explicitly, and it costs nothing in security that
userland does not already have.

**Decision, outbound.** `Core\Http\Client` has **no spelling for "wait forever"** — a call inherits a finite
default from `[http.client]` or names its own. Retry is opt-in in the options bag, jittered, and its
`deadline` covers **all** attempts rather than each one. A `POST` is not retried without an
`idempotencyKey`; because R2 makes the options literal a compile-time constant, that is a diagnostic rather
than a runtime throw.

**Home:** [0005](../adr/0005-config-changeability.md), [0064](../adr/0064-configuration-file-format.md), the
M7 milestone; outbound amends [0058](../adr/0058-outbound-request-policy.md) and
[spec § 16](../spec/01-core-library.md).

**Crate:** `tower-http`'s `CorsLayer` and `SetResponseHeaderLayer`, `cookie` — usable because M7 is
hyper-based; if the server does not adopt tower's `Service` stack this becomes roughly 300 lines of ours.
Outbound: `reqwest` plus `reqwest-retry` or `backon`. ADR 0058's pinned-address dial needs a custom
connector, which `reqwest` supports.

**Still to settle:** whether these are one ADR or two (they are one decision — *defaults are safe and finite
by construction* — but two subsystems); the full directive list; whether a policy-owned header set through
`setHeader` is logged.

## 5. `Core\RateLimit`

**Decision.** A `Core` class over the **shared** store: `limit`, `per`, `burst`, and a `retryAfter()` the
`429` is supposed to carry. A second, differently-named member for approximate per-core load shedding, so
the weak guarantee is visible in review — the same two-names reasoning
[ADR 0059](../adr/0059-cross-request-state-is-explicit.md) § 1 gives for `local()`/`shared()`.

**Edge and flood limiting is dropped**, deliberately: per-IP, per-path limiting is what a proxy in front of
us already does earlier and better. What no proxy can do is limit on something only the application knows —
five failed logins *per account*, a quota *per tenant plan* — and that is the whole justification.

**Home:** fills the hole [ADR 0059 § 4](../adr/0059-cross-request-state-is-explicit.md) already named
("locks, rate limits, idempotency keys … use the shared tier or the database") without filling. Relates to
[0060](../adr/0060-application-security-protocols.md) § 2, whose three-part test it passes.

**Crate:** `governor` implements GCRA and covers the approximate tier. **The shared tier has no crate** —
there is no first-class distributed rate limiter in Rust; the standard implementation is a Lua script over
`redis`, and that script is ours.

**Still to settle:** whether the approximate tier ships at all in the first cut; the algorithm the shared
tier commits to (GCRA vs sliding window), since it is observable through `retryAfter()`.

## 6. Observability export

**Decision.** The runtime exports what [ADR 0018](../adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)
and [ADR 0041](../adr/0041-timeline-export-and-gc-spawn-trace-events.md) already measure — request
rate/duration/status, DB query duration per connection, GC pause, isolate spawn — with no code written. An
inbound W3C `traceparent` continues the trace and `Core\Http\Client` propagates it outward. Plus a
three-member `Core\Metrics` (`increment`, `observe`, `gauge`) for an application's own numbers.

The exporter is a **Native**, feature-gated subsystem so a CLI binary does not carry it. Series cardinality
is **capped** in config and evicts with a warning — an unbounded tag set is the failure mode.

**This does not collide with ADR 0059.** Metrics are approximate aggregates that nothing reads to make a
decision, so per-core accumulation with merge-at-scrape is correct; that is exactly the property
0059 § 4 tests for, and the ADR should say so rather than leaving a reader to wonder.

**Home:** relates to [0018](../adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md),
[0041](../adr/0041-timeline-export-and-gc-spawn-trace-events.md),
[0051](../adr/0051-standard-library-tiers.md) § 3 (the roster gains `Core\Metrics`),
[0004](../adr/0004-memory-for-simplicity.md) (the cardinality cap is a stated footprint).

**Crate:** `opentelemetry`, `opentelemetry-otlp`, `metrics-exporter-prometheus`. The W3C TraceContext
propagator is included. Fully covered.

**Still to settle:** whether `Core\Metrics` is Tier 0 while its exporter is Tier 2 (the session assumed yes,
by the same split ADR 0051 § 3 already uses for the Redis driver); the `[metrics]`/`[trace]` directive names.

## 7. Compile-time routing

**Decision.** `#[Route(path:, method:, name:)]` attributes discovered at compile time via
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)'s program enumeration, a route table
built while compiling, and `Core\Router::match` plus `Core\Router::url` for reverse generation. Duplicate
routes, a `:param` with no matching typed parameter, and a bad `url()` name are **compile errors**.

**It stops at matching.** No dispatch, no controller convention, no return-value-to-response rule — those
are framework opinions, and the user confirmed the point of stopping short is that a framework can ignore
all of this, use half of it, or use none. No `#[Route]` in a program means no table and no cost.

**The interaction that made this dangerous is now settled**, and this entry only has to apply it.
[ADR 0046](../adr/0046-attributes-shape-literal-metadata.md) § 4 makes attribute retrieval **structural, not
nominal**, so a naive route scan would also match a *framework's* own `#[Route(path:, method:)]` literals and
double-register routes it does not own. [ADR 0071 § 1](../adr/0071-derived-codecs.md) answers it generally: a
**compiler-recognized** attribute is matched **nominally**, against a closed `Core`-owned list. `Core\Route`
joins that list, and nothing else about 0046 changes.

**Home:** relates to [0061](../adr/0061-compile-time-autoload-and-program-discovery.md),
[0046](../adr/0046-attributes-shape-literal-metadata.md),
[0024](../adr/0024-taint-tracking-for-injection-sinks.md) (path params are `tainted`),
[0019](../adr/0019-reflection-and-ast-parsing-are-core-features.md) and 0046, both of which currently name
"an attribute-driven router" as the userland thing reflection serves — that sentence needs revisiting.

**Crate:** `matchit`, the radix router axum uses, does the matching. Reverse URL generation is not in it,
and the compile-time table build is by definition ours.

**Still to settle:** the `:id` vs `{id}` path syntax; whether `match()` returns something that can `invoke()`
or only a name plus typed params; how a route's `tainted` params reach a typed parameter.

---

## Rejected, with the reason, so it is not re-argued

- **Typed templates / `Core\View`.** A `params {…}` declaration at the top of a template file, checked at
  the call site, was offered and **declined**. Templates stay `require` plus
  [`Core\Out::capture`](../spec/01-core-library.md) § 12. The known cost stands: a template's inputs are
  invisible, and a renamed variable in the caller breaks it at runtime.
- **`mwl migrate`.** Declined. Schema migrations stay with external tools (Flyway, dbmate, phinx), each
  configuring database credentials a second time outside `mwl.toml`.
- **Markdown in `Core`.** Declined in favour of a **first-party extension**. The `Core\Zip` analogy
  (its real risk is policy, not memory safety) does not carry, because the "only `Core` may launder"
  objection dissolves: an extension returns a `tainted` string and `Core\Html::sanitize` — which already
  exists and is already a launderer — does the laundering. [ADR 0051](../adr/0051-standard-library-tiers.md)
  § 3's **Ext** roster gains it; `pulldown-cmark` and `ammonia` are the crates.
- **Edge/flood rate limiting in `mwl.toml`.** Declined: the proxy owns it. See entry 5.
- **A DI container, at any tier.** Declined. The session found a real weakness — MWL has no user-defined
  generics, so a userland container cannot be typed and every resolution goes through `mixed` plus a cast —
  and resolved it by naming the right fix: **user-defined generics**, already an open question in
  [ADR 0007](../adr/0007-explicit-type-system.md)'s *Alternatives*. Closing that fixes containers,
  collections and everything else at once instead of special-casing one subsystem into `Core`. A
  compile-time wiring pass (`Core\Wire::get<T>()` resolved while compiling) was offered and declined.
- **Unchanged rejections**, re-confirmed and not re-argued: ORM and query builder
  ([0067](../adr/0067-core-db.md) settled it, and entry 1 is not this), event dispatcher, middleware
  pipeline, durable job queue, object-storage abstraction, `Str::slug` (looks like one member, is actually
  locale — `ö`→`oe` vs `o` — so it belongs with intl), pagination, PDF, spreadsheets.

## Already answered, checked during this session

Named here only so the next reader does not re-run the search: logging, HTTP client, cache, UUID, dates,
env/config, password and the [ADR 0060](../adr/0060-application-security-protocols.md) protocol roster, CSV,
session, format validators, **email** (`Core\Mail`, Native tier, `lettre` — SMTP, MIME multipart,
attachments, DKIM, rustls), image and intl (Ext), and the inline-HTML template mode, which with
[ADR 0024](../adr/0024-taint-tracking-for-injection-sinks.md)'s auto-escaping sink is already a safer Twig.

*The standing rule this session produced — "domain logic is an existing first-class Rust crate; compiler
passes and scheduler primitives are ours" — has landed, in *Decisions taken at project start* in
[docs/adr/README.md](../adr/README.md) with its `AGENTS.md` bullet. It is not repeated here.*
