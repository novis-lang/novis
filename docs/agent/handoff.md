# Handoff

## State

**Goal 6, M7. ADR 0074 § 2's closed default is in the tree, both halves.**
`nvs_server::cors::Cors` is the policy (`crates/nvs-server/src/cors.rs`), resolved from
`[http.cors]` at boot beside `Secure` and carried on `Serving` as its fourth field. Closed —
`origins = []`, which is what a tree writing no `[http.cors]` resolves to — means two observable
things and they are kept two different ways. **No CORS header is emitted at all**, kept by there
being nothing in this crate that writes one: a policy able to emit `Access-Control-Allow-Origin`
before it has a list to match against is the failure § 2 exists to prevent, so the emitting half is
written when there is something to match and not one release earlier. **A preflight is answered
`403`**, and that is a decision: `Cors::preflight` reads an `OPTIONS` carrying
`Access-Control-Request-Method`, and the refusal is taken in `serve_connection` under ADR 0097 § 5's
valve and *above* the handler, so a preflight nobody configured selects no mount, allocates no
isolate and runs no Novis code. `403` and not `405` — the verb is implemented, the origin is not
allowed. That module's doc owns the whole argument and its § *What is not here yet* names the open
half.

**The driver's failed acceptance check is closed**: `cors_is_closed_with_nothing_configured` exists
and passes, over a socket, asserting the absent `access-control-` prefix on a `200` a peer sent an
`Origin` on. Two stage-4 check names are still unwritten and are the group below; neither is in
`nvs-server`.

## Next group

**The two names stage 4's checks still owe.** They share no file — each is a single case over
surface that already exists, in a different crate — so take them in either order and take both.

- [ ] **`every_http_response_directive_is_runtime_class`** (ADR 0074 § 5, ADR 0005). The registry is
      `DIRECTIVES` and the `[http]` row is `crates/nvs-config/src/directive.rs:114`, one `Runtime`
      row covering the whole block; `Class` and `Apply` are at
      `crates/nvs-config/src/directive.rs:28`. The case is one assertion over every `http.*` key a
      *response* reads — the blocks are `crates/nvs-config/src/tree.rs:408` (`[http.cors]`) and
      `crates/nvs-config/src/tree.rs:429` (`[http.cookies]`), plus `[http.headers]` above them —
      asserting the class each resolves to rather than listing the keys, so a key added to a block
      without a row fails here.
- [ ] **`a_cookie_is_secure_httponly_samesite_lax_by_default`** (ADR 0074 § 3). The member is
      `crates/nvs-stdlib/src/response.rs:314` and its reference card already spells the answer —
      `crates/nvs-stdlib/src/response.rs:529` says `SameSite=Lax; Path=/`. The assertion is over the
      `Set-Cookie` an `addCookie` with no options bag writes, with nothing under `[http.cookies]`.
- [ ] **§ 2's open half** (ADR 0074 § 2), once the two above are green. An `Origin` matched against a
      named list, `Access-Control-Allow-Origin` and `Vary: Origin` on the answer, and a preflight
      answered with the methods, headers, `expose` set and `max_age` § 2 configures.
      `crates/nvs-server/src/cors.rs:64` is the type to grow and
      `crates/nvs-server/src/serve.rs:613` is the one place a request is asked about.
      `nvs_config::http` already refuses `["*"]` with `credentials = true` at boot, so that pair is
      not this slice's.

## Backlog

- A response-side seam for the open half's headers: nothing calls `Cors` after the handler yet.
- `[http.headers]` values the wire cannot carry are dropped rather than refused at boot — a
  diagnostic naming the line is `nvs-config`'s (`crates/nvs-server/src/secure.rs` module doc).
- ADR 0074 § 5's outbound bounds have no `Core\Http` yet — `docs/plan/m7.md`.
- Raw/unparsed body access for an arbitrary content-type, ADR 0024 *Revisiting* and `docs/plan/m7.md`.
