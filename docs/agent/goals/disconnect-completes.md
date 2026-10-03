---
milestone: post-parity
position: last
---
# Loop goal 186 — a request whose client went away runs to its end, and a method can be set to cancel instead

Today a client that disconnects cancels its request at the next safepoint
(`rule:http-server/an-abandoned-request-is-cancelled-at-the-drop`). This goal reverses the default:
**a request runs to its end whether or not its client is still there**, under every limit it already
has, and its response is thrown away. A deployment that wants the old behaviour turns it on per
request method:

```toml
[limits]
cancel_on_disconnect = ["GET", "HEAD"]   # default: []
disconnect_grace = "30s"                  # default
```

`cancel_on_disconnect` lists the methods whose requests are cancelled when the client goes away. An
empty list, the default, cancels none.

`disconnect_grace` is the fallback bound for a request that has no `wall_time`. **A request with a
`wall_time` is bounded by that alone**, whether or not its client is still there. A request with no
`wall_time`, which is the default, is cancelled once `disconnect_grace` has passed since its client
left. The grace starts at the disconnect, so a request that ends within it is never affected.
`disconnect_grace` is finite with nothing written and has no unbounded spelling. Both keys are
`System` keys, so a request cannot change them, and both may be written in `[app.limits]` for one
application.

## Why here

The user's decision of 2026-10-03. A user who navigates away, refreshes, closes a tab or clicks a
submit button twice abandons a request, so a disconnect is the everyday case and not a rare one.
Cancelling then stops a script between two writes that are not in one transaction, and a PHP
developer does not expect it: PHP-FPM learns of a disconnect only at its next output, and with
output buffering most PHP pages write nothing until the end, so in practice a PHP script finishes its
work. The user expects broken data in production from the current default and wants completion for
every method, with cancellation opt-in per method.

The reason for cancelling is also weaker here than under PHP-FPM. A waiting request costs a pooled
stack and not a worker process (`rule:http-server/an-abandoned-request-is-cancelled-at-the-drop`
§ 2), and the CPU, memory and output limits still bound it. What is new is the one unbounded case:
`wall_time` has no cap by default (`crates/nvs-config/src/default.toml:70`), so a request parked on
a slow dependency after its client left would hold its admission place forever. That is exactly what
`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` forbids, and `disconnect_grace` is the
bound that rule requires. It applies only where `wall_time` does not, so a deployment that sets
`wall_time` has one limit to reason about, and a long job whose client left is not cut short by a
second one.

It sits right after goal `goal-closeout` because it changes request-path behaviour that goal
`performance-pass` then measures. It carries `position: last` because the goals around it do.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/http-server/an-abandoned-request-is-cancelled-at-the-drop.md` — the rule becomes "a
  request whose client went away runs to its end under its `wall_time`, or under `disconnect_grace`
  when it has none, unless its method is listed in `cancel_on_disconnect`". The decision record renames it or replaces it; the PHP-FPM
  paragraph stays true and stays.
- `docs/rules/concurrency/cancellation-runs-no-user-code.md:7-8` — "the parent died, which for a
  served request includes the client disconnecting" is true only for a listed method or a request
  past its grace.
- `crates/nvs-server/src/serve.rs` — the doc comments on `Peer` (`serve.rs:930-938`), `Streamed`
  (`serve.rs:972-983`), the drain's `poll_frame` (`serve.rs:331-335`), and the two disconnect tests
  (`serve.rs:5650-5677` and the one after it).
- `crates/nvs-runtime/src/host.rs:585-600` (`Running::abandon`'s contract) and
  `crates/nvs-host/src/worker.rs:500-506`, if what they say about the caller stops being true.
- `crates/nvs-config/src/default.toml` — the two new keys under `[limits]` and `[app.limits]`.
- `docs/reference/tools/25-server.md` and `docs/reference/tools/30-php-differences.md`, neither of
  which says anything about a disconnect today.

The stage closes with the same search it opens with:
`grep -rn "disconnect\|abandon\|goes away\|client.*gone" docs/rules docs/reference crates/nvs-server crates/nvs-host crates/nvs-runtime`,
read line by line. Every hit is either true as it stands, rewritten, or under `docs/decisions/`.
Generated files (`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters, `website/`)
are regenerated, never edited.

## Stage 1 — the floor

Nothing is carried. Goal `goal-closeout` makes a finished goal deleted. The suites, the `.nvst` trees
and `nv verify` are the floor. The two existing disconnect tests (`serve.rs:5679` and `serve.rs:5828`)
test the cancellation path. They keep testing it with their method listed in `cancel_on_disconnect`,
and Stage 3 adds the tests for the new default beside them.

## Stage 2 — the two keys, the keystone

**Does:** Adds `cancel_on_disconnect` and `disconnect_grace` to `[limits]` and `[app.limits]`.

One file set: `crates/nvs-config/src/default.toml`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/value.rs`, `crates/nvs-config/src/app.rs` — the three that know
`max_regex_steps`, which is a `[limits]` key the same way.

- **The decision record**, written first from § *Standing decisions*, for the whole goal. It
  `modifies` `http-server/an-abandoned-request-is-cancelled-at-the-drop` and
  `concurrency/cancellation-runs-no-user-code`, and the fragments are written with it.
- **`cancel_on_disconnect`** is an array of method names, default `[]`. A method is matched exactly
  and case-sensitively, as HTTP matches it. A name that is not a valid HTTP token is refused at
  load with the key named. A repeated name is refused. Any token is allowed, so a custom method can
  be listed.
- **`disconnect_grace`** is a duration, default `"30s"`. `false` and `0` are refused as `[server]`'s
  waits refuse them. It has no `[limits.hard]` ceiling, because no request may set it.
- **`[app.limits]`** takes both, and `rule:config/every-matching-app-block-applies-least-specific-first`
  decides which value a request gets.
- **Pinned by** the Stage 2 checks.

## Stage 3 — the server

**Does:** A request whose client disconnects runs to its end unless its method is listed, and one
with no `wall_time` is cancelled once `disconnect_grace` has passed.

One file set: `crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/admit.rs`,
`crates/nvs-host/src/worker.rs`, `crates/nvs-runtime/src/host.rs`.

- **The drop detaches instead of abandoning.** `Peer::drop` (`serve.rs:964`) and the end of
  `serve_connection` for a `Streamed` request read the request's method against
  `cancel_on_disconnect`. A listed method abandons as today. Any other method hands the `Running`
  to an owner on the same core that joins it when it ends and discards its `Completion`.
- **The detached request keeps its admission place.** `Streamed::_place` is an
  `crate::admit::InFlight<'a>` borrowed from the connection (`serve.rs:995`). The detached owner
  needs an owned form of it, and the place is given back only when the request ends. A request that
  gave its place back at the disconnect would be tier C of
  `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` broken: a client could open a
  request, disconnect, and repeat, until the core runs unbounded work.
- **The grace, only without a `wall_time`.** At the disconnect, a request whose effective
  `wall_time` is unset gets a deadline of the disconnect plus `disconnect_grace`. A request with a
  `wall_time` gets nothing new. `wall_time` is a `Runtime` key, so a request may set or clear it
  after its client left: the deadline follows it, so that from the disconnect on a request always has
  exactly one of the two bounds and never neither. When the grace runs out, the request is cancelled
  exactly as a listed method is, and the owner waits for it to end. The deadline is the request's
  own deadline machinery, not a second clock, if that machinery can take a deadline from outside.
- **What the script sees.** A read of the request body after the client left fails as it does today
  (`serve.rs:5662-5665`), and the script handles that through its own code. A write to the
  response, buffered or streamed, succeeds and its bytes are thrown away. Nothing throws because
  the client left. `Core\Sse` and a WebSocket are upgraded connections and not requests: they keep
  their behaviour and are not under these keys.
- **After-response work.** The request's frame returns, so `Core\Task::afterResponse` work runs,
  as `rule:concurrency/after-response-outlives-the-connection` already says. Today a disconnect
  before that return cancels it, and now it does not.
- **The drain.** A graceful stop waits for detached requests as it waits for attached ones, up to
  `drain_timeout`, and then cancels what is left. A detached request does not hold the process past
  the drain.
- **The log and the trace.** The request's log line and root span record how it really ended (its
  status, or the cancellation at the grace) and that the client had gone. What the log writes for a
  disconnect today is read first and kept where it is still true.
- **Pinned by** the Stage 3 checks: a request whose client leaves mid-write finishes both writes, a
  listed method is still cancelled, the grace cancels a request with no `wall_time` parked on
  something that never answers, a request with a `wall_time` runs past the grace up to its
  `wall_time`, a request that clears its `wall_time` after the disconnect is still cancelled at the
  grace, the admission place stays counted until the end, the drain waits for a detached request,
  and after-response work runs.

## Stage 4 — the feature proofs and the reference

**Does:** Adds the reference section, the tests, examples, attack and bench for what a disconnect
does.

- **A new reference section** in `docs/reference/tools/25-server.md`, `# When the client goes away`,
  next to `# Stopping and reloading: the drain`. It is a new heading, so it is a new feature on the
  roster, and `bun nv proofs --id` names what it owes.
- **Three examples** under `docs/examples/tools/server/` in the section's folder, each with its
  `nvs.toml`: a two-step write that finishes after the client left; `cancel_on_disconnect` set for
  `GET` and `HEAD`, where a slow search stops; one application in `[app.limits]` that cancels
  everything.
- **The attack** under `tests/hostile/`: a client opens many requests to a slow endpoint and
  disconnects from each, with no `wall_time` configured. Every detached request still counts against
  `max_in_flight`, and each one is cancelled at `disconnect_grace`. A second step clears `wall_time`
  from inside the request after the disconnect, and is still cancelled at the grace.
- **The bench** under `benches/members/`: a request whose client disconnects, served to its end,
  against the same request with its client still there. Detaching must cost nothing a request with
  a client does not pay.
- **The help** in the binary, and the `about.md`.
- `docs/reference/tools/30-php-differences.md` gains one paragraph: Novis finishes a request whose
  client left, like PHP in practice, and a request cannot change that itself, because there is no
  `ignore_user_abort()`.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The user's calls, 2026-10-03.** Completion is the default for every method. Cancellation is
  turned on in `nvs.toml` per request method. Nothing about this is per route or per request.
  `disconnect_grace` bounds only a request with no `wall_time`, and a request with a `wall_time` has
  that one limit, connected or not.
- **My calls, not yet confirmed by the user:** the two key names and their place in `[limits]`;
  the grace's `30s` default; the write to a gone client succeeding silently; the
  graceful drain treating a detached request as an attached one. A session that finds one of these
  impossible writes it under the handoff's `## Backlog` for the user rather than choosing again.
- **No runtime switch.** There is no `Core` method that changes either key for one request, and no
  method that tells a script its client has gone. Both are possible later goals and are not in
  this one.
- **A cancelled request still runs no user code** (`rule:concurrency/cancellation-runs-no-user-code`).
  That applies to a listed method and to a request past its grace alike.
- **One ADR slot**: one new record and no other number, checked right before it is written. It
  states the tradeoffs. Performance: none on a request whose client stays; a detached request uses
  a core until it ends. Memory: a detached request keeps its heap until it ends or reaches its
  `wall_time` or, with none, `disconnect_grace`, per request and under its own `memory` cap. Usability: a script's writes
  finish as a PHP developer expects, and a disconnect no longer leaves half-written data. Simplicity:
  two keys, and the default needs neither.
- **Every name in a test, an example and the record is neutral** — `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every new `about.md` follows `AGENTS.md` § *Text an end user
  reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before the
  wrap.
