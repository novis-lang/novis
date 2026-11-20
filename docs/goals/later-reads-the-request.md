---
looper: 1
title: a Core\Html::later slot reads the request it belongs to
summary: "Reached when every Core\\Request member that reads the request line, a header, a cookie, the query, a route parameter, the scheme or the client address returns inside a Core\\Html::later slot what it returns in the main script, on a normal and on a slotted route, a body read there throws LogicError naming the main script, and the rule and a conformance case say so."
context: [crates/nvs-runtime/src/ctx/inbound.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-runtime/src/later.rs, crates/nvs-stdlib/src/request.rs, docs/rules/core-classes/html-later.md]
stages:
  - id: 1
    title: a slot reads the request's head, and a body read there throws
    status: done
    context: [crates/nvs-runtime/src/ctx/isolate.rs, crates/nvs-stdlib/src/request.rs]
  - id: 2
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: slot-request
    stage: 1
    name: "inside a Core\\Html::later slot, Core\\Request returns the request's path, header, cookie, query and route parameter, and a body read throws LogicError"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/html-later-a-slot-reads-the-request-it-belongs-to.nvst]
    timeout: 1800
---

# a Core\Html::later slot reads the request it belongs to

## Why

`rule:core-classes/html-later` says that reading the request inside a slot is allowed. Under the
host it is not: a slot runs on a child context (`Ctx::child`), which has no inbound, so every
`Core\Request` member in a slot throws `LogicError: …(): there is no request here`. A `--GET--` case
does not show it, because a run with no host fills the slots on the request's own context. A
`Core\Test::request` or a served request shows it at once. A component that reads a header, a cookie
or a route parameter in a slot fails.

`session-lifetimes` found it and fixed the session half: while the slots run, the request lends its
session and its head to them (`Ctx::lend_session`, `LentSession`, `Inbound::head_alone`).

## Standing decisions

**The slot gets the request's head, read-only, and no body (the authoring session's call).** The
head is an `Rc` already, so a copy costs a reference count. What `Core\Request` reads besides the
head travels with it: the scheme, the client address, the mount prefix and captures, and the matched
route. The body is the main script's: it may already have read it, and two readers of one socket
would each get part of it. So a body read in a slot throws `LogicError`, and its message says to read
the body in the main script and pass the value in. Safer than a body that reads as empty, which
`rule:security/request-state-throws-in-an-isolate` refuses ("no request arrived" and "the request
sent nothing" are different facts).

**One lent carrier, not two (the authoring session's call).** `Ctx::session_inbound` and the new
request read use the same lent head. Once `Ctx::inbound` falls back to the lent head in a slot,
`session_inbound` is `inbound` again, and is removed.

**Only slots.** A `Core\Task` child and after-response work keep no request. Task children are
outside this goal, and `rule:concurrency/deferred-work-cannot-write-the-response` covers deferred work.

**What it costs.** Per request with slots: one small carrier with a shared head, for the length of
the slots. Nothing for a request without slots.

**Every reader of `Ctx::inbound` sees a slot as a request (stage 1's call).** The fallback is in
`Ctx::inbound` itself, so `run_kind`, `working_dir`'s refusal, a log record's request id and the panic
report all read a slot as part of its request, which is what it is. `Ctx::inbound_mut` stays the
context's own and is `None` in a slot, so every body path goes through `request.rs` `body_carrier`,
which throws the slot's `LogicError` first. `query_fields_of` does not cache its parse on the shared
copy, so a slot parses the query once per call. `lend_session` reads the context's own inbound, so a
nested `later` keeps the cell its parent slot was lent.

## Stages

### 1. a slot reads the request's head, and a body read there throws

**Split:** none, one session.

- [x] `crates/nvs-runtime/src/ctx/inbound.rs` `head_alone`: also copy the client address, the mount
  prefix and captures, and the route, and mark the carrier as having no body for a slot.
- [x] `crates/nvs-runtime/src/ctx/wiring.rs` `session_inbound` and `crates/nvs-runtime/src/ctx/inbound.rs`
  `inbound`: `inbound` returns the lent head in a slot. Remove `session_inbound` and its two callers
  in `crates/nvs-stdlib/src/session.rs` `opened_under`.
- [x] `crates/nvs-stdlib/src/request.rs`: every body reader throws `LogicError` on a slot's carrier.
- [x] `tests/conformance/core/html-later-a-slot-reads-the-request-it-belongs-to.nvst`: a routed case
  through `Core\Test::request`, modelled on
  `tests/conformance/core/session-a-first-use-inside-later-reads-the-request-session.nvst`. One slot
  echoes the path, a header, a cookie, a query value and a route parameter. A second slot reads the
  body and catches `LogicError`.
- [x] `docs/rules/core-classes/html-later.md`: the sentence on reading the request names the body.

### 2. the goal is verified once, and every failure is fixed

- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Both stages are done, and the goal is reached. Stage 2 closed it without the full gate: the build
and clippy are green, and so are
`tests/conformance/core/html-later-a-slot-reads-the-request-it-belongs-to.nvst` and
`tests/conformance/core/session-a-first-use-inside-later-reads-the-request-session.nvst`.
`bun nv owners --closes later-reads-the-request` names no gap. `bun nv site --check stale`,
`changes` and `review` are green after these edits: a sentence on reading the request in a slot in
`docs/examples/core/Html/later/about.md`, the same in the `fn` parameter of `LATER_DOC` in
`crates/nvs-stdlib/src/html.rs`, a `changed` entry in `website/src/data/changes.json`, and a
`// covers: Core\Html::later` marker in the new case. Without that marker the changes check did not
count the feature as moved.

The nightly run 44722386375 ran on `5d053fc6e6`, 69 commits before this goal. All nine of its
failed jobs are the two failures `e5a100b18f` (`xml::tests::source_stops_at_the_memory_limit_before_it_escapes`)
and `438e9c44af` (`iter/an-object-map-passes-where-a-keyed-iterable-is-declared.nvst`) already fixed.

### Next

None: the goal is reached. The driver retires it. The next full gate runs at `compare-seven-peers`.
No test is expected to go red.

### Backlog
