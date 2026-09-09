---
milestone: M8
---
# Loop goal 19 — the request a test builds, and the peer facts it carries

`rule:testing/in-process-request` promises `Core\Test::request(...)`: a
test builds a request, it runs through `rule:routing/routes-are-compiled-not-registered`'s compiled route
table and the real middleware chain in-process, and what comes back is asserted. The **dispatch** half of
that is M8's and is out of this goal's scope. The **request** half is not, and it is the half with a
problem: today `request` exists as one English word in spec § 13's `| Class | Surface | ADR |` table and
one example in the ADR — **no parameter list, no options bag, no return type, and no gate that will ever
notice**, because the coverage walk reads §§ 14–19's bullets and § 19's table and excludes § 13 on purpose.

So this goal builds the request and freezes its shape. Everything a test may say about a request goes into
**one builder with one home**, goal `request-json`'s `.nvst` sections are re-pointed at it so there are not two
spellings, and M8 is left with dispatch and nothing else to invent.

Then the fact that falls out of it. The builder's bag carries `clientIp`, `scheme` and `host` — and those
are exactly the three `Core\Request` members `crates/nvs-stdlib/src/request.rs:20` lists as known gaps.
**Two of them are no longer waiting on a carrier**: `nvs_server::forwarded` is `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s walk, it
answers a client address and an effective scheme per request, and `Inbound::set_peer` now carries both
down — so `clientIp` and `scheme` are each this goal's own five edits, and `host` is the one still owed a
decision. A builder that can say what a trusted-proxy header resolves to is still the cheapest coverage
that walk will ever get.

Its floor is goal `request-json`'s whole list.

## Why here

`Core\Test::request`'s shape, frozen, and the peer fields that fall out of its options bag — which
are what `Core\Request::clientIp`/`scheme`/`host` have been waiting on. After goal `request-json` because it
re-points that goal's `.nvst` sections at one shared builder, so the sections have to exist first.

## Stage 0 — the catch-up

Nothing. Goal `request-json`'s rule is the one this goal builds on, and it lands there.

## Stage 1 — the floor

Goal `request-json`'s whole acceptance list, never traded.

## Stage 2 — the keystone: one builder, one home

1. **`InboundSpec` beside `Inbound`** in `crates/nvs-runtime/src/ctx/inbound.rs:113` — the description of a
   request as data, and the one place it becomes an `Inbound`. Its fields are `Core\Test::request`'s bag
   exactly: `query`, `headers`, `cookies`, `body`, `form`, `json`, `files`, `clientIp`, `scheme`, `host`.
   `nvs-runtime` is the home because all four crates that need it — `nvs-test`, `nvs-cli`, `nvs-stdlib`,
   `nvs-server` — already depend on it, and a builder in any of them is a builder the others copy.
2. **The four body spellings are one field.** `body` is raw; `form` encodes urlencoded and sets its
   `Content-Type`; `json` encodes a value and sets its own; `files` builds a `multipart/form-data` body
   with a boundary. Two of them together is a refusal naming both, never a merge
   (`rule:errors/ambiguous-input-refused`).
3. **The encoder is the builder's, and `crates/nvs-stdlib/src/multipart.rs` stays a parser.** One
   direction each: they are not twins under `rule:core-api/shape-rules` R17,
   because neither can be reached through the other.
4. **Goal `request-json`'s sections re-point at it.** `--GET--`/`--POST--`/`--POST_RAW--`/`--COOKIE--`/`--HEADERS--`
   and `nvs run --request` stop building an `Inbound` by hand and build an `InboundSpec` instead. This is
   the slice that makes "one home" true rather than aspirational.

## Stage 3 — the peer facts, and the three members waiting on them

1. **`Inbound` gains the peer** — the client address and the effective scheme the request was decided to
   have, plus the host. Set by whoever accepted the request: `crates/nvs-server/src/serve.rs:1344` and
   `:1443` for a served one, `crates/nvs-cli/src/serve.rs:323` for the dev server, `InboundSpec` for a
   built one. `nvs_server::forwarded` already computes both — `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`, landed — so this is a field
   and a hand-over, not a new decision.
2. **`Core\Request::clientIp()`, `scheme()` and `host()`** — the five edits each, off
   `crates/nvs-stdlib/src/request.rs`'s known-gap list and into spec § 15's registered roster. All three
   are `tainted`: what a proxy asserted is not this process's fact.
3. **The proof only a built request can give**: a request whose `Forwarded` header says one thing from a
   peer that is not in `[server] trusted_proxies` resolves to the socket peer, and the same request from
   one that is resolves to what the header said. That is `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s whole rule, asserted in-language
   for the first time.

## Stage 4 — the signature, frozen

1. **`rule:testing/in-process-request` is amended** to carry the signature rather than an example: the bag above, the
   mutual-exclusion rule, and `Core\Test\Response` — `status`, `header`, `headers`, `body`, `cookies`,
   `json()` and `jsonAs<T>()`. The last two mirror goal `request-json`'s request-side pair so one spelling reads in
   both directions, and a captured response raises none of the claim or caching questions the request side
   does. The section keeps saying what it already says about dispatch, tainted bodies and
   `#[Test(server:)]`; it gains the shape and nothing else.
2. **Spec § 13's `Core\Test` row becomes a § 15-shaped bullet** — the class, its members one by one, the
   ADR — so `request`'s signature is written where every other member's is, and `Core\Test\Response` gets
   a row of its own. **This does not bring it under the coverage gate**, and it is not meant to:
   `crates/nvs-stdlib/tests/spec_registry_coverage.rs:583` excludes §§ 13/16/17 because their members live
   inside English cells, and teaching that walk to read prose is how a gate starts lying. The hole is
   named there already; this goal makes the roster honest without touching the gate.

## Standing decisions

- **Dispatch is M8's and is out of scope.** This goal builds the request; it does not run one. A session
  that finds itself wiring a route table has left the goal.
- **The builder's home is `nvs-runtime`, beside `Inbound`.** Ambiguity about where a piece of it belongs
  resolves toward that module, recorded in its doc comment, never `BLOCKED`.
- **`Core\Test\Response` gets `json()` and `jsonAs<T>()`.** Settled; not re-opened on `rule:core-api/tier-placement` test 6
  grounds. The argument for the request-side pair does not apply here — nothing about a captured body is
  single-use — but the argument for *one spelling in both directions* does, and it is the stronger one for
  a member a test author reads.
- **The three peer members are `tainted`**
  (`rule:security/tainted-qualifier`). A client address that a proxy
  asserted is peer input; that it passed a trusted-proxy check makes it *trusted enough to believe*, never
  laundered.
- **This goal opens no new ADR number.** `rule:testing/in-process-request` is amended in place — an ADR's body always states
  the current rule — and spec § 13 and § 15 take the rosters. If a session finds a decision that genuinely
  needs a number, that is the one thing worth stopping for.
- **What this spends**, per `rule:programs/memory-priority`'s ledger: three short
  strings per in-flight request on `Inbound` — the peer address, the scheme and the host — and nothing
  per request served. The builder itself exists only in a test process.
