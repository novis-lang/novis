# Handoff

## State

**Goal `http-client` — stage 13 is closed.** An outbound call is a trace event: `TraceKind::Http`
is the fifth kind and `Ctx::record_http` files it (`crates/nvs-runtime/src/ctx/trace.rs:181`), its
fixed field set is `HttpSpan` (`crates/nvs-stdlib/src/http/span.rs:54`), the transport fills one
through `Call::span` as it goes, and `traced` (`crates/nvs-stdlib/src/http.rs:3317`) is the single
filer — above the transport, because that module holds no `Ctx`. A call the answer table served
never opens one. Stage 13's four `-p nvs-stdlib` checks are green, and so is `verify.py` whole.

Stages 1–12 are on disk behind it. Nothing is blocked.

Two decisions this stage made, both recorded where the code is: the resolve time is measured in
`exchanged` around `approved` and handed to the span, and is **zero** for a call given an
already-pinned `Core\Http\Target`, whose lookup was an earlier call's; and a call that never
reaches an answer files no event, since the field set has no spelling for a status that never came.

## Next group

**Stage 14: the rulebook** — one file set: `docs/rules/http-server.json`, `docs/rules/security.json`,
`docs/rules/testing.json` and the fragments under `docs/rules/http-server/`. Eleven rules this goal
shipped still read `status: designed`; each check is `python tools/rules.py --show <id>` wanting
`shipped`. Flip the status in the topic's `.json`, give each one its `guards`, and re-render — the
prose fragments are already true, so this is metadata rather than rewriting.

- [ ] **The four request-shape rules go `shipped`** — `rule:http-server/an-outbound-request-carries-one-body`
      at `docs/rules/http-server.json:957`, and beside it `a-streamed-reply-is-bounded-by-idle-and-a-lifetime`,
      `an-outbound-connection-is-pooled-per-core-and-stays-pinned` and
      `a-cross-origin-redirect-drops-credentials`. Each needs a `guards` list naming what holds it —
      `crates/nvs-stdlib/src/http/transport.rs` and the `.nvst` cases under
      `tests/conformance/core/`. Run `python tools/rules.py --render` after, never edit
      `docs/rules/http-server.md`.
- [ ] **The four TLS and address rules go `shipped`** —
      `rule:http-server/an-outbound-call-tries-every-approved-address` at
      `docs/rules/http-server.json:1077`, with `the-client-trust-roots-are-the-operators`,
      `an-https-redirect-never-becomes-plaintext` and `a-reply-reports-its-tls-session`;
      `rule:security/tls-trust-is-relaxed-only-under-a-host-grant` is in
      `docs/rules/security.json` and `an-outbound-call-names-its-address-only-under-a-grant` back
      in the http-server one.
- [ ] **The table rule goes `shipped`** — `rule:testing/an-outbound-call-is-answered-from-a-table`
      in `docs/rules/testing.json`, guarded by
      `crates/nvs-stdlib/src/http.rs:3477` (`faked`) and
      `a_call_the_table_answered_files_no_http_event` beside it. Then
      `python tools/verify.py --doc`, which is the gate a goal only meets at its end.

## Backlog

- A call that fails outright — a timeout, a refused connection — files no `http` event, so a trace
  shows only the calls that answered; if that is wanted, the field set needs a spelling for "no
  status" ([ADR 0180](../decisions/0180.md) § 15 fixes the fields, so it is a record question).
- `rule:observability/four-kinds-become-a-span` derives the outbound span from this event and is
  still *designed*; nothing exports a trace yet.
- The `nvs/rest` package and OAuth are unscheduled — `docs/agent/carried-gaps.md`.
