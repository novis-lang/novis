# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0, 2 and 3 have landed;
4 and 5 are open.** Stage 1 is goal 15's whole list, untouched. The design is settled in the goal prose's
standing decisions, which the pack prints.

**The rule is now true to a program.** `Inbound::claim_body` takes a `BodyNeed` rather than a member name
and answers a matrix over the three facts the carrier holds — the octets, the parse, the first claimant —
so `body()` is idempotent, a `post()` after `body()` parses the hold (urlencoded and multipart alike), and
a streaming reader still refuses every reader after it. Which octets a parse reads is
`Inbound::parts_mut`'s answer, not a member's: it hands out a `BodySource`, the hold where one was filled
and the wire otherwise. `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
is the rule and `crates/nvs-stdlib/src/request.rs`'s module doc is how this crate reads it.

The fragment stays `status: designed` on purpose: its table names `json()` and `jsonAs<T>()`, which stage 4
has not written. Its `guardedBy` now names the two cases below.

Acceptance is still red on `examples/json-body.nvs`, which is stage 5's fixture and unwritten — an open
item, not a regression.

## Next group

**Stage 4: the members — `json()` and `jsonAs<T>()`** — one file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/json.rs`, and cases under
`tests/conformance/core/`. `loop-goal.toml`'s stage-4 check names the five tests these owe.

- [ ] **`json()` registers and reads the body the way `body()` does** —
      `crates/nvs-stdlib/src/request.rs:273` is the member roster and
      `crates/nvs-stdlib/src/request.rs:1963` is `body`'s helper, the shape to write beside: claim with
      `BodyNeed::Octets`, then `held_octets` at `crates/nvs-stdlib/src/request.rs:1995`, which is the
      whole of the sharing. Decode through `crates/nvs-stdlib/src/json.rs:890`.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **The decoded `Value` is held on the request, and `jsonAs<T>()` holds nothing** — a second hold
      beside `crates/nvs-runtime/src/ctx/inbound.rs:298`'s `held`, per the goal's standing decision; the
      recorded fallback is re-decoding per call, which is `post()`'s own shape. `decodeAs` builds objects
      and two callers must never be handed the same one — `crates/nvs-stdlib/src/json.rs:919`.
- [ ] **An absent or empty body is a `ParseError` for these two members** — `whole_body` at
      `crates/nvs-stdlib/src/request.rs:2030` answers an empty body with an empty buffer, which `body()`
      is owed and these two are not. No `Content-Type` gate, on `post()`'s stated reasoning.
- [ ] **The cases** — written the way
      `tests/conformance/core/a-body-read-twice-answers-the-same-octets.nvst:1` is: a `--POST_RAW--`
      document, and the playbook bullet above for what its trailing newline does. The registry rows they
      exercise are `crates/nvs-stdlib/src/request.rs:273`.

## Backlog

- Stage 5's `examples/json-body.nvs` and its `nvs run --request` fixture — the failing acceptance check,
  `docs/agent/loop-goal.md` stage 5.
- The fragment's `status` flips to `shipped` once stage 4 lands — `docs/rules/http-server.json`.
- Spec § 15's roster at `docs/spec/01-core-library.md:1078` gains `json` and `jsonAs` with stage 4; its
  buffering/streaming sentence at :1101 is already the current rule.
- `rule:testing/nvst-is-separate`'s "unchanged as a format" sentence still owes the `.phpt` superset
  wording the goal's standing decisions name.
