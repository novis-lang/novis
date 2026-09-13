# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–9 are on disk, including stage 9's end-to-end case; stage 10 has not
started.** [ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal
executes.

Stage 9 is closed. `[http.client.tls]` is read off the boot snapshot and handed to
`nvs_host::tls::configure` by every run site that starts a program — `run_run`
(`crates/nvs-cli/src/main.rs:1815`), `nvs test` (`crates/nvs-cli/src/main.rs:2276`) and
`nvs serve` (`crates/nvs-cli/src/serve.rs:150`) — through `config::install_tls_client`, whose doc
owns why it is not in `boot_in`: `nvs check` reaches that, opens no socket, and would create the key
log file. A refusal is `E0641`, and it names the file because `nvs_host::tls::at_path` puts the path
into the `io::Error` the boot reports.

The client's first end-to-end `https` case is in `crates/nvs-stdlib/src/http/transport.rs`: a
loopback origin terminating TLS under a certificate written to a `roots` file, fetched through this
module's own `send`. Its twin asserts that an origin no configured anchor vouches for throws and is
not retried. `rustls` joins that crate's `[dev-dependencies]` for the origin's half of the
handshake; the client's half was already `nvs-host`'s.

Nothing is blocked.

## Next group

**Stage 10: relaxing trust from code, for a host a grant names** — one file set:
`crates/nvs-config/src/tree.rs`, `crates/nvs-stdlib/src/http.rs`, `crates/nvs-host/src/tls.rs`.

- [ ] **`[capabilities.tls]`'s four host lists, and `net`'s two** — `tls.anchors`, `tls.pin`,
      `tls.any_name`, `tls.insecure` as a new block beside `Capabilities`
      (`crates/nvs-config/src/tree.rs:257`), and `connect_to`/`downgrade` on `CapNet`
      (`crates/nvs-config/src/tree.rs:300`). **None has a `true` spelling**, for `net.internal`'s
      reason, which that struct states where it sits. `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`
      is what specifies them, and `nvs.toml`'s commented block is part of the slice.
- [ ] **The four options join the bag** — `tlsCa`, `tlsPin`, `tlsVerifyHost`, `tlsVerify` beside
      `IDENTITY_OPTION` (`crates/nvs-stdlib/src/http.rs:721`), each refused at the call when the
      URL's host is not in its grant, naming the grant. `tlsMinVersion` needs none and may only
      tighten. `rule:security/capability-question-is-grant-and-scope` is the shape of the ask.
- [ ] **The verifiers, inside the one client** — a call naming any of those options builds its own
      `ClientConfig` through `rustls`'s custom-verifier seam beside `anchors_from`
      (`crates/nvs-host/src/tls.rs:695`), and a caller still hands in a policy value rather than a
      session (`rule:security/one-tls-client`). The policy is part of stage 7's pool key, so a
      relaxed connection never serves a call that verifies.

## Backlog

- `[http.client.tls] keylog` is not resolved against the file that wrote it, where `roots` is
  (`crates/nvs-config/src/http.rs:342`) — whether `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`
  reaches a file the runtime creates rather than reads is undecided.
- The boot printing every relaxed grant, one line per grant and host — stage 10's last bullet in
  [loop-goal.md](loop-goal.md), and goal `outbound-proxy` does the same for its own weakening.
- `Response::tls(): ?Core\Http\TlsInfo` and the `https`→`http` downgrade refusal — stage 11.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest`
  package's, per the goal's § *Standing decisions*.
