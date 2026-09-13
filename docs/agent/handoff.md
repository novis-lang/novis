# Handoff

## State

**Goal `http-client` — a program talks to a real API. Stages 1–9 are on disk and stage 10 is complete
but for the boot's own line: the grants, the options, the verifiers and now the transport are landed, so
a relaxing call builds its own session and never shares a connection with a strict one.**
[ADR 0180](../decisions/0180.md) § 11 is the record this stage executes.

`policy_of` (`crates/nvs-stdlib/src/http.rs:2037`) turns the five keys `judge_trust` has already proved
a grant for into one `nvs_host::tls::CallPolicy`, carried on `Call::policy`
(`crates/nvs-stdlib/src/http/transport.rs:208`) beside `identity`. The handshake is one door for every
call — `NvsTls::over_policy(stream, &parts.host, &call.policy, identity)`
(`crates/nvs-stdlib/src/http/transport.rs:1313`) — so a call that asked for nothing hands over a default
and still gets the process's own configuration. `pool_key`
(`crates/nvs-stdlib/src/http/transport.rs:1378`) writes the policy as a last field through `policy_key`
(`crates/nvs-stdlib/src/http/transport.rs:1403`), which digests the variable-length halves rather than
putting a PEM bundle in a key that is compared on every draw.

Stage 10's three acceptance checks pass. What § 11 still owes is its last bullet, which no check names:
the boot printing every relaxed grant, one line per grant and host.

Nothing is blocked.

## Next group

**Stage 10: the boot's own line** — one file set: `crates/nvs-cli/src/config.rs`,
`crates/nvs-config/src/capability.rs`.

- [ ] **The boot prints every relaxed grant, one line per grant and host** — ADR 0180 § 11's last
      bullet, beside the install that already reads `[http.client.tls]`: `install_tls_client`
      (`crates/nvs-cli/src/config.rs:348`) walks the snapshot's four `Cap::Tls*` grants, each named in
      its configuration spelling (`crates/nvs-config/src/capability.rs:336`) with the host list behind
      it (`crates/nvs-config/src/capability.rs:438`), and writes one line per grant and host.
      `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s last sentence is what it executes,
      and the run sites are the only callers, which is where the print belongs for the reason that
      function's doc already gives about `nvs check`.
- [ ] **A case over what the boot printed** — in that module's own tests
      (`crates/nvs-cli/src/config.rs:776` is where the `[http.client.tls]` reading is asserted today),
      one line per grant and host and nothing at all for a deployment that granted none. No `[[check]]`
      names it, so it is this stage's own guard.

## Backlog

- Stage 11 — `connectTo`, the plaintext downgrade grant and `Core\Http\Response::tls()` — is back in
  `crates/nvs-stdlib/src/http.rs` and its transport (`docs/agent/loop-goal.toml:9597`).
- An unparseable `tlsCa` or an ill-formed `tlsPin` reaches `over_policy` and comes back as
  `InvalidData`/`InvalidInput`, which the transport reports as "the TLS handshake with `<host>` was
  refused" though no handshake ran. ADR 0180 § *Diagnostics* has no line for it.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest`
  package's, not this goal's (goal `http-client` § *Standing decisions*).
