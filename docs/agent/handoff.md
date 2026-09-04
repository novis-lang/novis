# Handoff

## State

**Goal 6, and ADR 0097 § 6's forwarded walk has landed.** The driver's failed acceptance check —
`nvs-server (the peer, and who may speak for it)` — is closed:
`client_ip_and_scheme_come_from_the_peer_unless_a_trusted_proxy_asserted` is at
`crates/nvs-server/src/forwarded.rs:406`, asserted as a partition over every combination of a
written `trusted_proxies` and a peer rather than as one example.

`crates/nvs-server/src/forwarded.rs` is the whole of § 6 that this crate can hold: `Trusted::of`
resolves `[server] trusted_proxies` into addresses and CIDR blocks, `walk` answers a client address
and a [`Scheme`] per request, the chain is walked **right to left** from the peer, and the one
refusal is a `400` for a token the walk lands on and cannot read. § 6's four defect rows each have
their own assertion. `crates/nvs-server/src/serve.rs:558` is the call site — per request, not per
connection, because the assertion is a header — and `unusable_forward()` beside `failed()` is the
`400`. `nvs-cli` resolves the directive at boot and prints a note per entry that names no network;
the *diagnostic* for one is still `nvs-config`'s, on `secure.rs`'s own precedent for `[http.headers]`.

**`origin.client()` has nowhere to go yet.** `nvs_runtime::Inbound` carries no peer and the handler
is `Fn(Request<Incoming>) -> Reply`, so `Core\Request::clientIp()`/`scheme()` are still gaps —
`crates/nvs-stdlib/src/request.rs:18` now says exactly what they wait on. § 6's one `Warn` for a
trusted `Forwarded` with no `X-Forwarded-For` is computed (`Origin::ignored_forwarded`) and
unreported, for the same reason a reset peer is: this loop has no log.

**`Core\Request::post()` still has not landed** — `crates/nvs-stdlib/src/multipart.rs:231` says so
in the `#[allow(dead_code)]` reason on `fields()`. It was this session's item until the acceptance
check outranked it, and it is untouched.

`[context] adrs` needed `0097` § 6, which the pack did not print — it prints §§ 2 and 4. Add § 6.

## Next group

**ADR 0105 § 2's form fields — `Core\Request::post()`, the member and then its test.** One file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/multipart.rs`,
`tests/conformance/core/`.

- [ ] **`Core\Request::post()` — the five edits** (§ 2). A non-file part is a form field and is
      buffered rather than streamed. The parse half exists and is dead-coded pending exactly this:
      `crates/nvs-stdlib/src/multipart.rs:231`. `crates/nvs-stdlib/src/request.rs:216` is `query`'s
      row and the shape to copy — same `mixed` answer, same § 9 bracket convention and so the same
      reason there is no qualifier on it; `crates/nvs-stdlib/src/request.rs:270` is where the row
      goes, beside `files`; `crates/nvs-stdlib/src/request.rs:336` is `QUERY_DOC`, the card to copy;
      `crates/nvs-stdlib/src/request.rs:933` is the `address()` arm a miss turns into a runtime
      panic. Three `.nvst` cases, each asking a different question. **Decide first** whether a body
      already claimed by `body()`/`files()` refuses `post()` — `nvs_runtime::Inbound::claim_body` is
      the rule and `a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other` is the
      existing shape.
- [ ] **`a_non_file_part_is_buffered_into_post`** (§ 2). The claim under the member: a part with no
      `filename` in its `Content-Disposition` is a form field, reaches `post()` and never `files()`,
      and a repeated name keeps both values in arrival order.
      `crates/nvs-stdlib/src/multipart.rs:220` is the field store and its order rule.
- [ ] **The bound on what `post()` holds** (§ 2, § 5). A buffered field is resident memory and is
      charged against `[limits] request_body` rather than a third directive.
      `crates/nvs-stdlib/src/multipart.rs:231` is what it reads; the playbook's
      `Ctx::set_memory_limit` bullet is the trap that makes the fixture, not the member, fail.

## Backlog

- `Core\Request::clientIp()`/`scheme()`: `nvs_runtime::Inbound` needs a peer field and the handler a
  way to be handed one — `crates/nvs-stdlib/src/request.rs:18` owns the statement of the gap.
- § 6's boot `Warn` — production, every listener loopback or Unix, `trusted_proxies` empty —
  is `nvs_config::server::validate`'s, beside the other `[server]` checks.
- A diagnostic in the `E06xx` band for a `trusted_proxies` entry that names no network;
  `crates/nvs-server/src/forwarded.rs`'s module doc records why it is dropped until then.
- A Unix-domain listener: `nvs_host::NvsListener` is TCP only, so `Arrival::Unix` has no producer.
- `docs/agent/loop-goal.toml`'s comment above the `(the peer, and who may speak for it)` check
  still cites `serve.rs:538` as naming the walk unlanded; stale now, and the driver holds that file.
- ADR 0024's raw/unparsed body gap for an arbitrary content-type — `docs/plan/m7.md` narrows it.
