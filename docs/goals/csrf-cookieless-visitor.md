---
looper: 1
title: a CSRF token for a visitor with no cookie writes no session row
summary: "Reached when a page that renders Core\\Csrf::token() for 1,000 requests with no cookie leaves no session row in the store, the door still accepts each token on the visitor's next request, an identifier the server never issued is still rejected, and the gap record csrf-token-makes-a-session-per-visitor is deleted."
context: [crates/nvs-stdlib/src/csrf.rs, docs/rules/security/csrf-is-on-by-default.md, docs/rules/security/a-session-id-the-store-did-not-issue-is-rejected.md]
stages:
  - id: 1
    title: "a token for a new visitor sends its cookie and stores no row"
    status: open
    context: [crates/nvs-server/src/route.rs, docs/rules/core-classes/a-session-opens-on-its-first-use.md]
  - id: 2
    title: "the feature's proofs, pages and change entry"
    status: open
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: cookieless-token
    stage: 1
    name: "1,000 cookieless requests to a page that renders a token leave no session row, and each token passes the door on the next request"
    run: [cargo, test, --test, csrf_cookieless]
    timeout: 900
  - id: proofs
    stage: 2
    name: every feature the goal's work reached has its proofs
    run: [bun, nv, proofs, --gate, --goal, csrf-cookieless-visitor]
    timeout: 1800
  - id: close
    stage: 3
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, csrf-cookieless-visitor]
    timeout: 900
---

# a CSRF token for a visitor with no cookie writes no session row

## Why

`Core\Csrf::token` calls `crate::session::sent_id` (`crates/nvs-stdlib/src/session.rs:2133`). For a
session with no identifier, `sent_id` draws one, sends the cookie and marks the record changed, so the
write-back stores an empty row under it. A page that renders a token for every request therefore
writes one row per request with no cookie, and each row lives for `[session] short_lived`. A crawler
or a flood of cookieless requests grows the store with total traffic in that window. That is a cost
the visitor chooses and the server pays, on the security priority.

The row buys nothing. The door's check, `verified` (`crates/nvs-server/src/route.rs:356`), is a keyed
seal over the cookie's identifier and reads no store. The first `Core\Session` use of a later request
discards an identifier the store never issued and draws a fresh one at its first write
(`rule:security/a-session-id-the-store-did-not-issue-is-rejected`). So the token can be bound to an
identifier that only the cookie carries.

`review-hardening` stage 9 found this and filed the gap record
`data/gaps/nvs-stdlib/csrf-token-makes-a-session-per-visitor.json`, which this goal owns and deletes.

## Standing decisions

**A drawn identifier for a token is sent, never stored.** `sent_id` draws the identifier and sends the
cookie as today, and it no longer sets `dirty`. A session that has an identifier is left as it was.
Reason: the door's check needs the cookie and the key, and no row. It saves one store write per new
visitor shown a token, and costs nothing per request.

**The fixation rule is unchanged.** On the visitor's next request the door accepts the token against
the cookie. A `Core\Session` call in that request finds no row, opens a fresh session and draws a new
identifier at its first write, as for any identifier the store did not issue. A token rendered after
that write is bound to the new identifier. A form rendered before it and sent after it is refused by
the door, as a form from an expired session is today; the test pins which case that is.

**Within one request the identifier stays.** A request that renders a token and then writes a session
value keeps the drawn identifier for the write, so the token and the stored row name the same session.
`write` already reads `session.id`; the test covers both orders.

**Safe default if the door turns out to read the store.** Stage 1 first checks `verified` and every
caller of it for a store read. If one is found, the token binds to the drawn identifier sealed with
`[http] csrf_key` in its own cookie, and the door reads that cookie for a request with no session row.
The choice is recorded under `### State`.

**Lanes.** One lane: stage 1 changes `session.rs`, stage 2 writes only docs and data.

## Stages

### 1. a token for a new visitor sends its cookie and stores no row

**Does:** stops `sent_id` from storing a row for a drawn identifier, and proves it on the wire.

- [ ] Measure first: serve a page that prints `Core\Csrf::token()`, send 1,000 requests with no cookie,
  and count `nvs_session_heads` in the data folder's `session.sqlite`. Write the count under
  `### State`. If the probe ends in a `500`, as it did in `review-hardening`, find why first.
- [ ] Check `verified` (`crates/nvs-server/src/route.rs:356`) and its callers for a store read, and
  record the result under `### State`.
- [ ] `sent_id` (`crates/nvs-stdlib/src/session.rs:2133`) no longer sets `dirty` for a drawn identifier,
  and its doc comment says what it spends now.
- [ ] `crates/nvs-cli/tests/csrf_cookieless.rs:1`: 1,000 cookieless requests leave no row; a token from
  one passes the door on the next request with its cookie; a token and a later session write in one
  request name the same row; a forged identifier still opens a fresh session.
- [ ] `rule:security/csrf-is-on-by-default` and `rule:core-classes/a-session-opens-on-its-first-use`
  say that a token's drawn identifier is sent and not stored.
- [ ] `data/gaps/nvs-stdlib/csrf-token-makes-a-session-per-visitor.json` is deleted.

### 2. the feature's proofs, pages and change entry

**Does:** pays what the change owes the website.

- [ ] `docs/examples/core/Csrf/token/about.md` says a new visitor's token stores no session.
- [ ] A `website/src/data/changes.json` entry, `changed` `Core\Csrf::token`.
- [ ] `bun nv proofs --owed --goal csrf-cookieless-visitor` names nothing, and `bun nv site --check
  stale changes review` is green.

### 3. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of [prose.md](../agent/prose.md) § *A newcomer reads it* over the texts
  stage 2 wrote.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

The goal is written; no stage has started.

### Next

Stage 1, its first item: the measurement, with the probe under `.agent-tmp/`.

### Backlog

Two items `review-hardening` left unreproduced, carried here so they stay visible. Neither is this
goal's work.

- A release of the last reference to a suspended generator runs its `finally` bodies
  (`crates/nvs-runtime/src/object.rs:5126`, `unwind_abandoned`). An `InstKind::Release` counts as no
  program code (`rule:types/a-replaced-value-is-changed-in-place` § *What a quiet call is*), so a
  lend may cross a release that runs a `finally` which writes the lent slot. Not reproduced.
- A class constant's initializer, `public const int N = Framework\Base::LIMIT;`, is folded in the
  signatures pass and records no `ExprInfo`, so a restricted constant there is probably not checked.
  Not reproduced.
