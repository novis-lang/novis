# Handoff

## State

**Goal 29 — signing a URL, and the payload behind it — has just started; nothing of it has landed yet.**
Goal 28's whole list is this goal's Stage 1 floor.

The design is finished and is not this goal's to re-open:
[ADR 0146](../../adr/0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md) holds all of
it, ADR 0060's roster already reads five, and ADR 0077 § 4 already lists `urlSigned`/`signedRoute`. The
spec is already written against nothing — `docs/spec/01-core-library.md` § 12 carries `$uri->sign` and
`$uri->verifySignature`, § 16 carries `Core\Signature` — and both outstanding-key files name **29** as
the owner of those three rows.

**The one thing a session must not re-decide:** what gets signed. It is `equivalent()`, the RFC 3986
§ 6.2.2 normalization `uri.rs` already carries for `compareTo`. A second normalization written for the
signature alone is the exact defect this goal exists to not ship, and it will look reasonable at the
call site.

## Next group

**Stage 2: `Core\Signature`, over a payload and never over text** — one file set: the new class, the
construction under it, and the registry rows.

- [ ] **The canonical payload encoding** — `crates/nvs-stdlib/src/signature.rs` (new). Keys sorted,
      each value encoded with its type so `1` and `"1"` differ, `until` inside the signed region. Every
      later stage calls this and none of them writes a second one.
- [ ] **The key ring, shared not copied** — `crates/nvs-stdlib/src/signed_cookie.rs:@open` is the walk
      to lift: `array<secret bytes>`, newest at `[0]`, sign under `$keys[0]` alone. Two ring walks in
      the crate means one of them is wrong.
- [ ] **The two rows** — `crates/nvs-stdlib/src/registry.rs`, with `sign`'s payload contagious, the
      ring neutral, and `verify` answering a **`tainted`** map (ADR 0060 § 5; `SignedCookie`'s
      laundering exemption does not reach here, and ADR 0146 § 1 says why).
- [ ] **`{keys, until}` as a declared shape** — `crates/nvs-types/src/core_lib.rs`, ADR 0135's `CoreTy`
      arm. `until` is a **required key holding a nullable value**: `{until: null}` compiles and
      omitting the key does not.
- [ ] **Strike `§16 Core\Signature  # 29`** from
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`, in the slice that registers it.

## Backlog

- **Stage 3 — `$uri->sign`/`$uri->verifySignature`.** File set: `uri.rs` plus the registry. Cheap to
  take straight after stage 2, because the registry and the shape are already loaded and `uri.rs` is
  the only new read. Rewrite the module doc's "the normalization lives on `compareTo` and nowhere
  else" when `equivalent()` gains its second caller, and strike the two `§12` keys.
- **Stage 4 — `Core\Router::urlSigned`/`signedRoute`.** File set: `router.rs`, `request.rs`. Its
  acceptance property is the remount, not the round trip: a signed route identity survives a mount
  moving from `/ModuleA` to `/` and a signed path does not.
- **Stage 5 — the two refusals, and the ordering.** Signature first, clock second, so a token both
  forged and expired throws the *invalid* error. Plus the `.nvst` cases for `until` and both suites.
- When this goal's last check goes green the driver takes goal 50.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
