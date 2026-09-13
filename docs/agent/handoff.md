# Handoff

## State

**Goal `http-client` — a program talks to a real API. Stages 1–10 are on disk: the grants, the options,
the verifiers, the transport and now the boot's own line, so a relaxing call builds its own session,
never shares a connection with a strict one, and no deployment relaxes trust without being told so at
every start.** [ADR 0180](../decisions/0180.md) § 11 is the record stage 10 executes, and every bullet of
it is landed.

The boot's reading of a relaxed grant is `nvs_config::tree::Capabilities::tls_relaxations`
(`crates/nvs-config/src/capability.rs:691`), which finds the grants by `Cap::family` rather than by a
roster of its own and reads each through `grant_for`, so an operator's `true` — a spelling these grants
do not have — names no host at the boot either. `relaxed_grants`
(`crates/nvs-cli/src/config.rs:389`) turns that into one `note:` line per grant and host, and
`install_tls_client` writes them once the client is built, so a boot that refuses to start reports the
refusal alone.

Stage 11 is untouched: `connectTo`, the `https`→`http` downgrade and the reply's TLS report are what the
driver's acceptance check is waiting on. Nothing is blocked.

## Next group

**Stage 11: the address, the downgrade and the report** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **`connectTo` names the address a call connects to, and it is still judged** — ADR 0180 § 11's
      stage, `rule:security/outbound-url-is-a-sink`: a new option const beside the five TLS ones
      (`crates/nvs-stdlib/src/http.rs:847`) and its row in the options bag
      (`crates/nvs-stdlib/src/http.rs:1621`), the address judged against `Cap::NetConnectTo` before a
      socket is opened, the URL's host still checked, and `connectTo` beside a `Core\Http\Target` a
      `LogicError` — the record's *Diagnostics* section names that one.
- [ ] **An `https`→`http` redirect needs the `net.downgrade` grant and the option** —
      `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`: `redirect_of`
      (`crates/nvs-stdlib/src/http/transport.rs:1785`) is where a hop's scheme is known, and
      `redirects_of` (`crates/nvs-stdlib/src/http.rs:3048`) is where the count is read. Refused without
      the grant, refused under the grant without the option, followed with both; a refusal is the
      record's one-sentence `RuntimeError`.
- [ ] **The reply reports its TLS session, and a table's reply reports none** — the `tls` member on
      `Core\Http\Response`, whose rows and cards sit at `crates/nvs-stdlib/src/http.rs:1350`, over what
      the exchange kept (`crates/nvs-stdlib/src/http/transport.rs:317`): version, cipher and the peer
      chain, `null` for a reply the test table answered. Every string of it is `tainted`
      (`crates/nvs-stdlib/src/registry.rs:2810` is where a derived-taint pair is written), and the
      `.nvst` case the check names is
      `tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst`.

## Backlog

- Parsing `Link`, `Retry-After` for a program and RFC 9457 problem details are the `nvs/rest` package's,
  not this goal's — `docs/agent/loop-goal.md` § *Standing decisions*.
- Goal `outbound-proxy` prints its own weakening at boot; `relaxed_grants`
  (`crates/nvs-cli/src/config.rs:389`) is the shape and `Capabilities::tls_relaxations` the reading it
  should grow a sibling of rather than a second reading.
