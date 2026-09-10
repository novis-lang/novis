# Handoff

## State

**Goal `signed-urls`, stage 4 is half closed: `Core\Router::urlSigned` is registered, folded, tested
and covered; `Core\Router::signedRoute` is not written.** The `4 router` acceptance check therefore
still fails, on the three test names that need the verifying half — that is an item still open, not
a regression.

**`Core\Router::url` is not an ordinary `Core` member and neither is `urlSigned`.** The route table
exists only while compiling, so `nvs_types::links` folds a literal name into
`nvs_stdlib::router::link`'s symbols and the registry row's own body is what a *computed* name
reaches. `urlSigned` folds to `nvs_core_router_link_signed(<prepared path>, <route name>, $params,
$keys, $until)` — five arguments, the route name as a constant of its own because it is the half a
remount does not change, and the `{keys, until}` shape flattened by hand in
`Lowering::lower_signing_settings`. The playbook bullet this session added is the trap.

**The payload is `crates/nvs-stdlib/src/router.rs:894` (`signed_payload`)**: `{route: <name>,
params: <every entry of `$params` as its text>}`, nested arrays kept as arrays. Text on both sides
on purpose — the verifying half holds a match whose captures were decoded and converted on the way
in, so `42` is what the two can agree on where `int` versus `string` is not. Nothing of the
rendered path is in it, which is the whole property (`rule:core-classes/router-signed-url`).

**`$name` is classified `CoreTy::Text(Qual::Sink)`**, because `registry.rs`'s `UNCLASSIFIED` list
may only shrink and a route name is which handler runs rather than data the answer carries. `url`
and `urlAbsolute` are still on that list and still refuse a `tainted` name by default; classifying
them is a one-line backlog item, not a behaviour change.

## Next group

**Stage 4, second half: `Core\Router::signedRoute`, the verifying door, over `Domain::Route`** — one
file set: `crates/nvs-stdlib/src/router.rs`, `crates/nvs-stdlib/src/request.rs`.

- [ ] **The `signedRoute` row, card, body and `address()` arm** — the methods list is
      `crates/nvs-stdlib/src/router.rs:224`, `urlSigned` at `crates/nvs-stdlib/src/router.rs:252` is
      the row to sit beside, and the answer is the `Core\Router\Match` at
      `crates/nvs-stdlib/src/router.rs:540`, built by `crates/nvs-stdlib/src/router.rs:1134`
      (`match_value`). It takes `array<secret bytes> $keys` alone and no prepared path, so it is an
      ordinary registry row and needs none of the fold above. Read the request through
      `crates/nvs-stdlib/src/request.rs:1690` (`inbound_of`) — `route()` at
      `crates/nvs-stdlib/src/request.rs:1918` is the match, and the `_sig` token and the query
      parameters come off the same request, `crates/nvs-stdlib/src/request.rs:2047` being where the
      query string is parsed today. `rule:core-classes/router-signed-url`,
      `rule:routing/matched-once-before-the-handler`.
- [ ] **The payload the verifier derives is `signed_payload`'s, rebuilt from the match and the
      query** — `crates/nvs-stdlib/src/router.rs:894` is the mint side and
      `crates/nvs-stdlib/src/router.rs:915` (`written_form`) is the text rule both sides share; the
      comparison is `crate::signature::confirm` at `crates/nvs-stdlib/src/signature.rs:516`, with
      `crates/nvs-stdlib/src/signature.rs:457` (`judge`) last for the one distinguishable expiry
      refusal. `rule:core-api/signing-is-over-a-payload`,
      `rule:core-api/one-refusal-except-expiry`.
- [ ] **The three remaining named tests of the `4 router` check**, in
      `crates/nvs-stdlib/src/router.rs`'s test module beside
      `crates/nvs-stdlib/src/router.rs:1817` (`url_signed_launders_for_the_url_path_sink_exactly_as_url_does`),
      which is the one of the four that already passes: `linked` and `docs_template` beside it are
      how a folded helper is driven from a `-p nvs-stdlib` test. Plus three `.nvst` cases for the
      new row — `tests/conformance/core/router-url-signed-*.nvst` are the three this session wrote
      and the shape to follow.

## Backlog

- `Core\Router::url`/`urlAbsolute` still sit in `registry.rs`'s `UNCLASSIFIED`; `urlSigned` shows
  the classification they want — `crates/nvs-stdlib/src/registry.rs:3723`.
- `nvs_hir::requires`'s `ROUTER_SCAN_MEMBERS` comment says `::match` is absent from the registry,
  which stopped being true — `crates/nvs-hir/src/requires.rs:632`. Adding it to the list is a
  behaviour change (a program calling `::match` would opt into the scan), so it needs its own look.
- `examples/signed-url.nvs` is stage 5 and still missing; it is the driver's failing acceptance
  check — `docs/agent/goals/32-signed-urls.md` § *Stage 5*.
- Stage 5's refusal ordering and the `until is written` `.nvst` suite are unwritten —
  `docs/agent/goals/32-signed-urls.md` § *Stage 5*.
