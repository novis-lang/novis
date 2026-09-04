# Handoff

## State

**Goal 6, M7. ADR 0074 § 2's closed default is in the tree, and stage 4's two owed check names are
now pinned.** `nvs_server::cors::Cors` is the policy, resolved from `[http.cors]` at boot beside
`Secure` and carried on `Serving`; closed means no CORS header is emitted at all and a preflight is
answered `403` above the handler. That module's doc owns the whole argument and its § *What is not
here yet* (`crates/nvs-server/src/cors.rs:26`) names the open half.

**Both check names the driver was still failing on are green.**
`every_http_response_directive_is_runtime_class` is in `crates/nvs-config/tests/directives.rs`: it
reads each `[http.*]` response block's key set back out of the refusal `deny_unknown_fields` writes
for a key it does not know, so the assertion covers every key `nvs_config::tree` accepts rather than
the ones somebody listed — the helper's own doc comment owns that. The cookie one is a unit test in
`crates/nvs-stdlib/src/response.rs`, comparing the whole `Set-Cookie` line a bare `addCookie` writes
on a context with no configuration attached.

Nothing is blocked. The only half of ADR 0074 still unwritten is § 2's matching side, below.

## Next group

**ADR 0074 § 2's open half — an origin that *is* named.** All three slices are the same two files:
`crates/nvs-server/src/cors.rs` (the type, its unit tests) and `crates/nvs-server/src/serve.rs` (the
one place a request is asked). `nvs_config::http` already refuses `["*"]` with `credentials = true`
at boot, so that pair is never this crate's question.

- [ ] **A named origin is matched, and the answer says so** (ADR 0074 § 2). `Cors` grows the
      origin list it already resolves from `[http.cors]` into a match, and a request carrying an
      allowed `Origin` gets `Access-Control-Allow-Origin` plus `Vary: Origin` — the `Vary` on every
      answer the policy *could* have varied, including the refused one, since a cache keyed without
      it serves one origin's answer to another. `crates/nvs-server/src/cors.rs:51` is the type,
      `crates/nvs-server/src/cors.rs:71` resolves it, and `crates/nvs-server/src/serve.rs:613` is
      where the answer is reached.
- [ ] **A preflight is answered with what § 2 configures** (ADR 0074 § 2). `preflight` at
      `crates/nvs-server/src/cors.rs:89` answers `Some(StatusCode)` today; it grows into the
      response itself — `Access-Control-Allow-Methods`, `-Headers` and `-Max-Age` from the block —
      keeping the `403` for an origin the list does not name and the refusal's position above the
      handler at `crates/nvs-server/src/serve.rs:613`.
- [ ] **`expose` and `credentials` reach a simple response** (ADR 0074 § 2).
      `Access-Control-Expose-Headers` from the block's `expose`, and
      `Access-Control-Allow-Credentials` when it is set — which is also where the `*` echo rule
      lands, `crates/nvs-server/src/cors.rs:51` carrying it. Then delete § *What is not here yet* at
      `crates/nvs-server/src/cors.rs:26`.

## Backlog

- ADR 0074 § 4's cookie prefixes on the *read* side are `Core\Request`'s, not written yet.
- `[http.headers]`'s three free-text values are refused at boot (`E0625`); no case pins the
  `nvs-server` fallback beside it — `crates/nvs-config/src/http.rs`'s module doc names both.
- Stage 10's corpus is open; `python tools/gaps.py` ranks what is thinnest.
