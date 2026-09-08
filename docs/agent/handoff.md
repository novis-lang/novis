# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0, 2, 3a and 3b have
landed; 3c, 4 and 5 are open.** Stage 1 is goal 15's whole list, untouched. The design is settled in the
goal prose's standing decisions, which the pack prints in full.

**Stage 3a is closed.** The rule is
`rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`, reasoned in
[ADR 0156](../decisions/0156.md) — this goal's one new number, now spent. A buffering reader (`body`,
`post`, `json`, `jsonAs`) fills the request's hold under `[limits] request_body` and answers out of it,
so any of them may follow any other; a streaming reader (`bodyStream`, `files`) keeps nothing and may
only be the first. `post()` after a `files()` walk is that rule rather than an exception to it. The three
amended rules and spec § 15 point at it.

**Stage 3b landed as the pure generalization, with no observable change.** `Inbound::hold_form`/`form()`
are `hold_body`/`held_body()` over a field named `held`, and `urlencoded_form` asks the hold whether to
pull rather than asking `Reading`. `post()` is still the only member that fills it.

**Stage 3c is bigger than the item said, and the unsolved half is multipart.** `body()` then `post()` on
a multipart body would take `multipart_form`'s `Reading::First` branch and install a `Multipart` parse
over a wire `body()` had already drained, answering no fields. `Multipart` reads through
`Inbound::parts_mut`'s `&mut dyn RequestBody`, so parsing the hold needs a `RequestBody` over a slice.
Decide that before writing the claim kinds, or `json()` lands on a `post()` that silently answers empty.

The rustdoc gate is green again: `crates/nvs-test/src/request.rs` linked `crate::run`, which is both a
module and a function.

## Next group

**Stage 3c: the rule becomes true to a program** — one file set:
`crates/nvs-runtime/src/ctx/inbound.rs`, `crates/nvs-stdlib/src/request.rs`, and cases under
`tests/conformance/core/`. Take them in order; the second and third are written against the first.

- [ ] **Decide how a buffering reader follows another over a *multipart* body**, and record it in the
      `held` field's doc at `crates/nvs-runtime/src/ctx/inbound.rs:256`. Either `Multipart` parses the
      hold behind a `RequestBody` over a slice, or a multipart `body()` holds the octets and `post()`
      re-parses them from scratch — the second is the shape the goal's standing decisions pre-authorize
      as the fallback, and costs latency rather than an invariant.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it` says both
      readers must answer; ADR 0156 § 2 is why, and neither says which machinery.
- [ ] **`claim_body` takes a kind** — `crates/nvs-runtime/src/ctx/inbound.rs:634`. `Ok` iff nothing has
      claimed, or the first claim was buffering and this one is too; otherwise the first claimant's name,
      as today. Its `claimed_by` field doc at `crates/nvs-runtime/src/ctx/inbound.rs:277` and its
      `body()` doc at `crates/nvs-runtime/src/ctx/inbound.rs:598` both still state spec § 15's three-way
      exclusivity, and are rewritten whole in the same slice.
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **`body()` becomes idempotent and the two refusals are rewritten** —
      `crates/nvs-stdlib/src/request.rs:1978` claims as buffering, fills the hold and answers out of it;
      the messages at `crates/nvs-stdlib/src/request.rs:1206` and
      `crates/nvs-stdlib/src/request.rs:1255` state the new rule instead of spec § 15's; the `#[test]`s
      at `crates/nvs-stdlib/src/request.rs:3634` and `crates/nvs-stdlib/src/request.rs:4449` assert them.
      Sweep the member comments at `crates/nvs-stdlib/src/request.rs:1983` and
      `crates/nvs-stdlib/src/request.rs:2065` in the same slice, and flip the new rule to `shipped` with
      its `guardedBy` cases.

## Backlog

- Stage 4: `json()` and `jsonAs<T>()` register and land — `docs/agent/loop-goal.md` § stages.
- Stage 5: three conformance cases per member for the pair — `crates/nvs-stdlib/tests/conformance_coverage.rs`.
- Spec § 15's roster gains `json` and `jsonAs` when they land — `docs/spec/01-core-library.md:1078`.
- `rule:testing/nvst-is-separate`'s "`.nvst` is unchanged" sentence still owes goal 16's amendment.
- The new rule is `designed` until 3c lands — `docs/rules/http-server.json`.
