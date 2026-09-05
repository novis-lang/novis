# Handoff

## State

**Goal 6, M7 — stage 9's `-p nvs-server` check has seven names and five of them now exist and pass.**
The three landed this session are all in `crates/nvs-server/src/serve.rs`'s test module:
`the_header_injection_suite_passes`, `the_request_smuggling_suite_passes` and
`a_client_disconnect_leaves_no_isolate_behind`, beside the two `mount.rs` already had.

**Two names are left, and they are the whole of what stage 9 owes**:
`the_state_bleed_suite_passes_within_a_request_and_across_an_isolate_boundary` and
`a_multipart_body_far_over_the_memory_bound_is_received_at_bounded_resident_memory`.

**One measured divergence is recorded in a test comment and nowhere else.** `hyper` follows RFC 9112
for a request carrying both a `Content-Length` and `Transfer-Encoding: chunked`: it disambiguates —
chunked wins, the length is ignored — rather than refusing, where
[ADR 0095](../adr/0095-ambiguous-input-is-refused-never-repaired.md) says ambiguous input is refused
and never repaired. It is not a hole: the connection does not survive the message, so the trailing
bytes are never read as a request, and that `connection: close` is what those two rows assert. Whether
ADR 0095 or ADR 0097 § 1 should say so out loud is an ADR edit no session has made.

## Next group

**The two claims stage 9 still owes, sharing `crates/nvs-server/src/body.rs` and the body half of
`crates/nvs-server/src/serve.rs`** — both are about what the door holds while a request is reading,
so the same two files are open for either. Take them in this order: the first establishes the
high-water fixture the second's isolate half reads.

- [ ] **A multipart body far over the memory bound is received at bounded resident memory** — ADR 0105's
      load-bearing case, asserted against a high-water mark rather than against the request merely
      succeeding. The crossing is `crates/nvs-server/src/body.rs:68`'s `Arrived`, driven from
      `crates/nvs-server/src/serve.rs:798`'s `Reply::Run` arm, which pumps one chunk per poll.
      **`nvs-stdlib` is not a dependency of `crates/nvs-server`** (the playbook's manifest bullet), so
      the multipart *parse* is out of reach here and the claim that is testable is the crossing's own
      resident bytes: `nvs_runtime::budget::live_bytes()`, read the way
      `crates/nvs-server/src/serve.rs:2447` reads it. A client that writes megabytes against a small
      `[limits]` cap, a program that reads to the end, and a peak sampled inside the pull.
- [ ] **The state-bleed suite passes within a request and across an isolate boundary** — M7's
      acceptance paragraph. **Item 25 is a parameterisation, not a second suite**: the same rows run
      twice, once inside one request and once with a child isolate between them, which is what the
      shared `Isolate` buys. Two requests down one connection is the fixture
      (`crates/nvs-server/src/serve.rs:3252`'s `echo_the_body` is the closest shape, and the playbook's
      bullet says a second *connection* is what a test cannot ask for), and the isolate arm hangs off
      `crates/nvs-server/src/serve.rs:798`.

## Backlog

- An isolate parked on something the connection cannot fail wedges the core — measured at three
  minutes, not diagnosed; `crates/nvs-server/src/serve.rs:446`'s `Peer::drop` and `nvs_host`'s
  cancellation own it between them.
- ADR 0095 versus `hyper`'s RFC 9112 disambiguation of `Content-Length` + `Transfer-Encoding`: stated
  in `the_request_smuggling_suite_passes`'s comment, in no ADR.
- `[context] adrs` was missing `0097 §1` and `[context] rules` was missing `0095`; both were needed for
  the smuggling item and were sliced by hand (`docs/agent/loop-goal.toml`).
- `Core\Response::setHeader`'s member-half of the injection defence is already pinned, by
  `tests/conformance/core/a-response-set-header-names-both-sides-of-a-header-line.nvst` — no second
  case is owed in `nvs-stdlib`.
