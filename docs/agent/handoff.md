# Handoff

## State

**Goal `http-client` — stage 11 is closed: a call names its address under a grant, an `https` hop needs
both halves to become plaintext, and a reply now reports the session it arrived over through
`Core\Http\Response::tls(): ?Core\Http\TlsInfo`.** Stages 1–10 are on disk behind it.
[ADR 0180](../decisions/0180.md) § 13 is the record and `rule:http-server/a-reply-reports-its-tls-session`
the rule.

`Core\Http\TlsInfo` holds four slots — version, cipher, `verified` and the chain as DER — and answers
seven members over them; the class's own doc comment in `crates/nvs-stdlib/src/http.rs` is why it is four
and not seven. `subject`, `issuer` and `expiry` parse the leaf through `nvs_host::tls::leaf`, beside the
parser `tlsPin` uses, so a reply nobody audits pays no certificate parse and `peerChain` writes PEM at
the member. `exchanged` hands the transport's own `Option<Tls>` back and the streamed half drops it,
because the rule puts the report on `Core\Http\Response` alone.

Nothing is blocked. Stage 12 is untouched.

## Next group

**Stage 12: a name resolved off the core** — one file set: `crates/nvs-runtime/src/capability.rs`,
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **The lookup leaves the core, and the grant and the address check stay on it** —
      `rule:http-server/a-core-is-never-blocked-on-a-syscall`, `docs/agent/loop-goal.md` § *Stage 12*
      first bullet. `resolve_host` (`crates/nvs-runtime/src/capability.rs:217`) is the blocking call,
      asked from `pin_host` (`crates/nvs-runtime/src/capability.rs:194`) after the grant and before the
      address check. `nvs-runtime` cannot reach `nvs_host`, so either the caller hands a resolver in or
      `nvs-host` installs one at boot; `crates/nvs-stdlib/src/process.rs:394` is the
      `nvs_host::blocking::run` precedent.
- [ ] **Every resolved address is judged, and one denied refuses the host** —
      `rule:security/net-address-policy`. `pinned_address`
      (`crates/nvs-runtime/src/capability.rs:264`) answers one address today and has to answer the set;
      `pin` (`crates/nvs-stdlib/src/http.rs:380`) is the caller, and an IP literal or a `connectTo`
      value is a set of one.
- [ ] **`Core\Http\Target` carries the approved set, at most eight, in resolver order** —
      `rule:http-server/allow-url-pins-the-address`. `TARGET` (`crates/nvs-stdlib/src/http.rs:200`)
      keeps no members, `Call::address` (`crates/nvs-stdlib/src/http/transport.rs:149`) becomes the set,
      and `one` (`crates/nvs-stdlib/src/http/transport.rs:1313`) is where a connection is made to one of
      them. A retry reuses the set and never re-resolves.

## Backlog

- Stage 12's fourth bullet — RFC 8305 fallback across the set — is a slice of its own after the three
  above; `docs/agent/loop-goal.md` § *Stage 12*.
- Stage 13 onward of `docs/agent/loop-goal.md`, untouched.
