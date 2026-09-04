# Handoff

## State

**Goal 17 — the request a test builds, and the peer facts it carries — has just started; nothing of it has
landed yet.** Goal 16's whole list is this goal's Stage 1 floor.

The scope line matters more here than in most goals: **ADR 0079 § 18's dispatch half is M8's and is out of
scope.** This goal builds the request and freezes its shape; it does not run one through a route table. A
session wiring a route table has left the goal.

What the goal is really buying is two things at once. `Core\Test::request` has no signature anywhere — one
English word in spec § 13's table and one example in the ADR — so the bag gets written down and built.
And because that bag carries `clientIp`, `scheme` and `host`, the same edit lands the three
`Core\Request` members that `crates/nvs-stdlib/src/request.rs:20` has listed as "waiting on a carrier"
since the module was written: `nvs_server::forwarded` already answers both facts per request (ADR 0097
§ 6, landed), and nothing carries the answer down.

## Next group

**Stage 2: the builder, and goal 16's sections re-pointed at it** — one file set:
`crates/nvs-runtime/src/ctx/inbound.rs`, `crates/nvs-test/src/case.rs`, `crates/nvs-cli/src/main.rs`.

- [ ] **`InboundSpec` beside `Inbound`** — `crates/nvs-runtime/src/ctx/inbound.rs:113`. Fields are
      `Core\Test::request`'s bag exactly: `query`, `headers`, `cookies`, `body`, `form`, `json`, `files`,
      `clientIp`, `scheme`, `host`. `form`/`json`/`files` encode and set their own `Content-Type`; two
      body spellings in one spec is a refusal naming both, never a merge (ADR 0095). The multipart
      encoder is this module's — `crates/nvs-stdlib/src/multipart.rs` stays a parser, one direction each.
- [ ] **The sections build a spec, not an `Inbound`** — goal 16's `--GET--`/`--POST--`/`--POST_RAW--`/
      `--COOKIE--`/`--HEADERS--` parse (`crates/nvs-test/src/case.rs`) and `nvs run --request`
      (`crates/nvs-cli/src/main.rs:923`) both go through it. This slice is what makes "one home" a fact
      rather than a plan, and it is why it shares the group.

## Backlog

- Stage 3 (the peer on `Inbound`, populated at `crates/nvs-server/src/serve.rs:1344`/`:1443` and
  `crates/nvs-cli/src/serve.rs:323`; then `clientIp`/`scheme`/`host` with the five edits each) is the
  second group. It shares `ctx.rs` with stage 2 and adds `nvs-server` and `nvs-stdlib`.
- Stage 4 is prose: ADR 0079 § 18 amended to carry the signature and `Core\Test\Response`'s readers
  (including `json()`/`jsonAs<T>()`), and spec § 13's `Core\Test` row rewritten in § 15's bullet shape.
  It closes no gate — §§ 13/16/17 are excluded from the coverage walk on purpose
  (`crates/nvs-stdlib/tests/spec_registry_coverage.rs:583`) and this goal does not change that.
- **Goal 18 follows this one** — ADR 0140's one converter from `array<mixed>` to a declared shape, and the
  two `Core\Request` members over it. Its stages 4-5 are proven over the builder this goal freezes, so
  this list going green is what makes them writable.
