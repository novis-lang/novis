# Handoff

## State

**Goal `http-client` is met.** Stage 14 was the rulebook, and the eleven rules this goal shipped now
read `status: shipped` — the four request-shape rules and the six TLS-and-address ones under
`docs/rules/http-server.json`, `security/tls-trust-is-relaxed-only-under-a-host-grant` and
`testing/an-outbound-call-is-answered-from-a-table`. Each carries a `guardedBy` list naming what holds
it: the transport, the pool, the config resolver, and the `.nvst` cases under `tests/conformance/`.
`docs/rules/http-server.md`, `security.md`, `testing.md` and `docs/ground-rules.md` are re-rendered.

`python tools/verify.py` is 11 of 11 green, and `python tools/verify.py --doc` resolves every link —
it named one broken intra-doc link into a `#[cfg(test)]` item, which is now written as the plain code
span its two neighbours in the same file already use.

Stages 1–13 are on disk behind this. Nothing is blocked.

## Next group

**The goal is closed — the next session opens goal `process-cache` on its own seed handoff.**

- [x] The four request-shape rules go `shipped`, each naming the transport and its `.nvst` cases.
- [x] The four TLS and address rules go `shipped`, plus the trust-roots and TLS-session ones, guarded
      by `crates/nvs-config/tests/capability.rs`, `crates/nvs-config/tests/resolve.rs` and
      `crates/nvs-host/src/tls.rs`.
- [x] `rule:testing/an-outbound-call-is-answered-from-a-table` goes `shipped`, guarded by
      `crates/nvs-stdlib/src/test.rs` and the three `test-*-http-*.nvst` cases.

## Backlog

- The stage 10 call-site options — `connectTo`, `redirectToHttp` and the four `tls.*` ones — are held
  by Rust tests alone; no `.nvst` case names one. `python tools/gaps.py` is where that surfaces.
- The REST package and OAuth are the unscheduled `nvs/rest` work — `docs/agent/carried-gaps.md`.
