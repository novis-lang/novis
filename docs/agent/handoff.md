# Handoff

## State

**Goal 17, Stage 2 has landed.** `InboundSpec`, with `SpecBody` and `SpecPart`, is
`crates/nvs-runtime/src/ctx/inbound.rs:1012`, exported at the crate root, and it is the one place a
described request becomes an `Inbound`. `nvs run --request` builds through it
(`crates/nvs-cli/src/main.rs:1422`), so every `.nvst` case's request does too.

The `.nvst` sections reach it **through the `.nvsr` file rather than directly**: `nvs-test` has no
dependencies on purpose, so the cookie join, `content-type` and `content-length` are written in
`crates/nvs-test/src/request.rs` as well as in the spec. That module doc is the home of the fact, and
the rule that keeps the two from fighting is that a field written by hand is never written twice.

**Stage 3 is open and nothing of it has landed.** Scope is unchanged — dispatch is M8's — and
`Inbound` already carries the client address and the effective scheme, so Stage 3's first slice is a
member, not a carrier field. The one thing this session found that Stage 3 has to decide: **the
`.nvsr` file cannot say who the peer was**, so a `.nvst` case reads `clientIp()` as `null` and
`scheme()` as `http` whatever the request meant — the file format is the gap, not the carrier.

## Next group

**Stage 3: the peer facts, and the two members that no longer wait on a carrier** — one file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-test/src/request.rs`, `crates/nvs-cli/src/main.rs`.

- [ ] **The `.nvsr` file learns its peer** — `crates/nvs-test/src/request.rs:60` (`Wire`, plus the
      render and the read around it) and `crates/nvs-cli/src/main.rs:1422`, where the two new fields
      become `InboundSpec::set_peer`. Take this **first**: without it the two members below have no
      value a conformance case can assert, and the three-case floor gate wants cases with values in
      them. A section per fact, named as `--HEADERS--` and `--GET--` are.
- [ ] **`Core\Request::clientIp()` and `scheme()`** — the registry rows beside `method`'s at
      `crates/nvs-stdlib/src/request.rs:221` and the helper it is written as at `:1685`. Both are
      `tainted` (the goal's standing decisions), both read `Inbound::client`/`Inbound::scheme` at
      `crates/nvs-runtime/src/ctx/inbound.rs:224`, and both come off this module's known-gap list at
      `crates/nvs-stdlib/src/request.rs:23` in the same edit.
- [ ] **Three conformance cases per member, not one** — the floor gate at
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155` counts distinct case files, and the
      playbook's bullet on it applies. `clientIp()` answering `null` where no peer was named is one
      of the three and is a real assertion, not a filler.

## Backlog

- `Core\Request::host()` is the member still owed a decision — whether a *forwarded* host may be
  believed, which `crates/nvs-stdlib/src/request.rs:24` states and ADR 0097 § 6 deliberately does not
  answer.
- Stage 4: `rule:testing/in-process-request` amended to carry `Core\Test::request`'s signature rather
  than an example, and spec § 13's `Core\Test` row rewritten as a § 15-shaped bullet.
- `Core\Test::request` itself is unregistered — the spec exists, the member does not, and the bag's
  keys are `InboundSpec`'s fields one for one.
- `crates/nvs-cli/src/serve.rs:402` and `crates/nvs-server/src/serve.rs:3183` fill a carrier directly
  and should stay that way: a door reads its fields off a socket, and the spec is for a request
  nobody sent.
