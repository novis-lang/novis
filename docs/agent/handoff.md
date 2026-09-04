# Handoff

## State

**Goal 6, M7. The driver's stage-5 acceptance failure is closed.**
`an_upload_total_over_the_cap_is_refused_before_dispatch` is green in
`crates/nvs-server/src/serve.rs`. The mechanism was already there — `crate::body::of` reads the
declared length off the head and answers `TooLarge` before a program is asked for — so what landed
is the case, asserting both halves of ADR 0105 § 5's "before dispatch" because neither implies the
other: the peer's `413`, and a counter only a program that ran could have moved. Its handler is
`nvs-cli`'s door in the two lines this is about (`crates/nvs-cli/src/serve.rs:422`), the mapping
from `TooLarge` to a status being the door's while both halves of it are `nvs-server`'s.

**Nothing of ADR 0139 § 1's class is on disk** — the group below is untouched. This session scoped
its first item and spent its budget doing so; what that scoping found is in the items' anchors and
in the new playbook bullet, so the next session should not re-derive any of it.

**`[context]` gained this group's selectors**, in `docs/agent/loop-goal.toml` and
`docs/agent/goals/6-server.toml` alike: `crates/nvs-stdlib/src/session.rs` and `cache.rs` in
`modules`, ADR 0139 §§ 1, 2 and 4 in `adrs`, with the closed routing group's `0102 §§ 4, 8` and
`0096 § 4` taken out. The pack printed none of those, which is why § 1's roster was sliced by hand.

## Next group

**ADR 0139 § 1's seven members, over one file set:** `crates/nvs-stdlib/src/session.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-runtime/src/ctx.rs` and
`tests/conformance/core/session-*.nvst`. **There are three items, not four**: the conformance floor
makes each member's cases part of landing that member, per the new playbook bullet, so no item can
collect them afterwards.

- [ ] **`Core\Session::start` opens the record the store issued** (ADR 0139 §§ 1-2) — conventions'
      five edits, none of them started: there is no `CLASS` const in
      `crates/nvs-stdlib/src/session.rs:67` yet and `crates/nvs-stdlib/src/cache.rs:142` is the shape
      to copy; the class line is `crates/nvs-stdlib/src/registry.rs:1224` and the `address` arm joins
      the chain at `crates/nvs-stdlib/src/lib.rs:365`. The body is `backend`
      (`crates/nvs-stdlib/src/session.rs:119`) → `crate::cache::on_shared`
      (`crates/nvs-stdlib/src/cache.rs:717`) → `load` (`crates/nvs-stdlib/src/session.rs:173`), and
      on absent `mint` (`crates/nvs-stdlib/src/session.rs:149`), `save`
      (`crates/nvs-stdlib/src/session.rs:189`) and the cookie
      (`crates/nvs-stdlib/src/session.rs:144`). **The record has nowhere to live yet**: `Ctx` carries
      no session field — one goes beside `open_connections` at `crates/nvs-runtime/src/ctx.rs:1106`,
      initialised at `crates/nvs-runtime/src/ctx.rs:1595`, and holding the id, ADR 0023's byte
      record and a dirty flag keeps it free of any `Value` to release at teardown.
- [ ] **`get`, `set` and `remove` over the started record** (ADR 0139 § 1) — each decodes the byte
      carrier the field above holds, and a call before `start` throws naming it
      (`crates/nvs-runtime/src/ctx.rs:1106`, `crates/nvs-stdlib/src/session.rs:189` for the
      write-back).
- [ ] **`clear`, `regenerate` and `destroy`** (ADR 0139 §§ 1, 4) — `regenerate` is `mint`
      (`crates/nvs-stdlib/src/session.rs:149`), `save` under the new id
      (`crates/nvs-stdlib/src/session.rs:189`) and a destroy of the old, in that order; § 4's
      one-entry-per-session is what keeps all three single-key.

## Backlog

- The four landed `ADR 0105 §§ 1-3` names, and now the `nvs-server` one, still have not moved up
  into the sibling *what is landed* check — `docs/agent/loop-goal.toml:3515`, and the goal copy with
  it. Nothing fails while it is undone.
- ADR 0105 § 5's `upload_total` and `request_body` are still constants rather than `[limits]` rows —
  `crates/nvs-server/src/body.rs:65` and `crates/nvs-stdlib/src/request.rs:1933` say so; owner is
  `nvs_config`.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
- `Core\Session`'s db backend has no store implementation at all; only the shared tier's wire is
  written (`crates/nvs-stdlib/src/session.rs` module doc).
