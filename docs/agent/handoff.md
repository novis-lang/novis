# Handoff

## State

Goal `core-http-client-and-2-more` is complete. `Core\Http\Client::delete`, `::get` and `::head`
each have `about.md`, three examples, one attack, one bench and a Rust test on a loopback origin.
The root `nvs.toml` grants `net.connect = true` to `docs/examples/core/Http-Client` and
`tests/hostile/core/Http-Client`, and every proof there is refused before a socket opens. The
benches are answered from `Core\Test::answerHttp`'s table, so they need no grant.

The `head` attack found that `rule:security/net-address-policy`'s table missed IPv4-compatible
(`::/96`) and NAT64 (`64:ff9b::/96`) addresses that carry an internal IPv4 address. That is fixed in
`nvs_config::capability::embedded`. The fix is pinned by
`tests/conformance/core/http-allow-url-refuses-an-ipv6-address-carrying-an-internal-one.nvst`.

## Next group

**Stage 1: the goal is complete** — one file set: `crates/nvs-stdlib/src/http.rs` and the three proof trees.

- [x] **`Core\Http\Client::delete`** — all feature proofs landed. `crates/nvs-stdlib/src/http.rs:1583`
- [x] **`Core\Http\Client::get`** — all feature proofs landed. `crates/nvs-stdlib/src/http.rs:1539`
- [x] **`Core\Http\Client::head`** — all feature proofs landed. `crates/nvs-stdlib/src/http.rs:1592`

## Backlog

- The `head` attack's numeric host forms (`2130706433`, `0x7f000001`) go to the system resolver,
  which takes about 0.6 s each on Windows, so the attack declares `timeout-ms 30000`
  (`tests/hostile/README.md`).
- The next `Core\Http\Client` member goal can reuse the two `[[app]]` grants in `nvs.toml` and the
  `exchanged_once` loopback helper in `crates/nvs-stdlib/src/http.rs`'s tests.
