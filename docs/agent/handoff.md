# Handoff

## State

**Goal 17, stage 3 is landed except its `nvs-server` half.** `Core\Request::clientIp()`, `scheme()`
and `host()` are registered, carded and answered (`crates/nvs-stdlib/src/request.rs:@CLASS` and the
three `nvs_helper!` blocks around `crates/nvs-stdlib/src/request.rs:1700`), all three `tainted`, and
four conformance cases ask each of them a different question. `examples/request-fields.nvs` and its
`.nvsr` twin print the nine `want` lines of the acceptance fixture that had been red since the goal
opened — the check needed an `args = ["--request", …]` line as well as the file, written into
`docs/agent/loop-goal.toml` and `docs/agent/goals/17-test-request.toml` alike.

**The peer crosses the `.nvst` → `.nvsr` → carrier path as two sections**, `--CLIENT_IP--` and
`--SCHEME--`, read as an address and as one of two schemes at both ends
(`crates/nvs-test/src/case.rs:54`, `crates/nvs-test/src/request.rs:78`). They are sections rather
than header lines because nothing a peer sends states either one — with `trusted_proxies` empty the
forwarded headers are never read. `host()` needed no carrier field: it folds the `Host` line by the
three equivalences the server compares a host mount by (`host_named`), and `header("host")` still
answers the line verbatim, which is what keeps the two members apart.

**Not landed:** the two `nvs-server` tests stage 3's acceptance list names. The wiring they would
assert is already there — `crates/nvs-server/src/serve.rs:3183` and `crates/nvs-cli/src/serve.rs:402`
both hand `Origin`'s answer to `Inbound::set_peer` — so this is a test-writing slice, not a feature.

## Next group

**Stage 3's server half, then the check that cannot run where it is filed** — one file set:
`crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/forwarded.rs`, `docs/agent/loop-goal.toml`.

- [ ] **`the_forwarded_walks_answer_is_set_on_the_inbound`** — the walk's verdict reaches the carrier,
      asserted over `crates/nvs-server/src/serve.rs:3183`, where `origin.client()` and
      `origin.scheme()` become `Inbound::set_peer`. `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`
      is the rule; `crates/nvs-server/src/forwarded.rs` is where `Origin` is decided and already
      tested on its own terms.
- [ ] **`an_untrusted_peers_forwarded_header_does_not_reach_the_carrier`** — the same seam with
      `trusted_proxies` empty: the header is present, `Inbound::client()` is the socket peer and
      `scheme()` is the connection's. Same anchor, `crates/nvs-server/src/serve.rs:3183`.
- [ ] **Stage 2's `the_request_sections_build_an_inbound_spec` is filed in a crate that cannot host
      it** — `nvs-test` has no dependencies on purpose, so it can never name `InboundSpec`. The seam
      that proves it is `crates/nvs-cli/src/main.rs:1422` (`inbound_from`), which names both sides;
      move the name onto a `-p nvs-cli` check in `docs/agent/loop-goal.toml:5540` and its twin under
      `docs/agent/goals/`, and write the test there.

## Backlog

- `__Host-`/`__Secure-` cookies still ignore the scheme on read, though the carrier now holds it —
  `crates/nvs-stdlib/src/request.rs:@cookie_of`'s doc names it as this module's known gap.
- Stage 4 is the signature frozen; every test its check names already passes under `-p nvs-stdlib`.
- `docs/reference/core/Request.md` is hand-written prose and says nothing of the three new members;
  `tools/reference.py --check` does not ask it to.
