# Handoff

## State

**Goal `signed-urls`, stage 3 is closed: `$uri->sign` and `$uri->verifySignature` are registered, and
both of stage 3's `cargo-named` acceptance checks pass.** `crates/nvs-stdlib/src/uri.rs` carries the two
rows, their cards, the `address` arms, the two bodies and the ten named tests; four `.nvst` cases sit
under `tests/conformance/core/uri-sign-*`. `crates/nvs-stdlib/tests/spec-members-outstanding.txt` now
holds no keys at all, which is what that file's own header says spec §§ 1-12 being registered whole
looks like.

**The payload is `equivalent`'s output** — the same function `compareTo` calls, not a second
normalization — with two departures written down at `crates/nvs-stdlib/src/uri.rs:2016`
(`payload_of`): the fragment is absent, and the query is signed as its *parameters* rather than its
text, so reordering is not a forgery. The reserved parameter is `_sig`, excluded from its own input,
and a URL carrying two of them is refused rather than checked under either.

**Two functions in `crates/nvs-stdlib/src/signature.rs` are now shared by both doors and are what
stage 4 builds on**: `confirm` at `:516` — for a payload the verifier *derives* rather than is given,
comparing the two by writing both as documents, so the codec is the equality — and `judge` at `:457`,
the one distinguishable expiry refusal, written once so two doors cannot word it differently.

**What a signed URL spends is length**: the token carries the signed document as well as the tag, so a
signed link runs a little over twice the URL it signs (`4/3 × (URL + 40)` characters). That is the
figure this goal's § *Standing decisions* already priced, and it buys one wire format for all three
doors; `nvs_core_uri_sign`'s *Cost* paragraph is its home.

The driver's failing acceptance check — `examples/signed-url.nvs is missing` — is stage 5 work and
still open, not a regression.

## Next group

**Stage 4: `Core\Router::urlSigned` and `Core\Router::signedRoute`, over `Domain::Route`, so a
signature survives a remount** — one file set: `crates/nvs-stdlib/src/router.rs`,
`crates/nvs-stdlib/src/signature.rs`. The coverage floor is three `.nvst` cases per member, so a row
without its cases fails `cargo test -p nvs-stdlib`; that keeps it to one slice, as stage 3 was.

- [ ] **The two rows on `Core\Router`** — the methods list is `crates/nvs-stdlib/src/router.rs:222` and
      `url` at `crates/nvs-stdlib/src/router.rs:226` is the row to sit beside. `urlSigned` takes
      `string $name`, `array<string, mixed> $params` and the shared shape
      `crates/nvs-stdlib/src/signature.rs:179` (`SIGNING`) — whose `$settings` name the spec's own
      signature column has to write, per the playbook bullet this session added — and answers a
      `string` that launders for the URL-path sink exactly as `url` does; `signedRoute` takes
      `array<secret bytes> $keys` and answers the `Core\Router\Match` at
      `crates/nvs-stdlib/src/router.rs:453`, throwing on refusal.
      `rule:core-api/signing-is-over-a-payload`, `rule:core-classes/router-signed-url`.
- [ ] **The payload is the route's name and its typed parameters, never the path it renders to** —
      that is the whole of what the mount prefix costs: `Domain::Route` at
      `crates/nvs-stdlib/src/signature.rs:571`, minted through `mint` and checked through `confirm` at
      `crates/nvs-stdlib/src/signature.rs:516`, with `judge` at `crates/nvs-stdlib/src/signature.rs:457`
      for the expiry. `signedRoute` reads the match `rule:routing/matched-once-before-the-handler` has
      the door take once rather than re-parsing anything.
- [ ] **The four named tests** of the `4 router` check, in `crates/nvs-stdlib/src/router.rs`'s test
      module — `url_signed_verifies_through_signed_route_after_the_mount_prefix_changes` and the three
      beside it in `docs/agent/loop-goal.toml` — plus three `.nvst` cases per member under
      `tests/conformance/core/router-sign*`. `crates/nvs-stdlib/src/uri.rs:4471` onward is the shape:
      an agreement case is what catches a second canonical form.

## Backlog

- `examples/signed-url.nvs` does not exist, and the driver's acceptance check names it — stage 5,
  `docs/agent/goals/32-signed-urls.md`.
- `docs/spec/01-core-library.md` § 12 still says the two `Uri` signing members "land with M8"; they
  have, and the sentence is a schedule claim rather than a rule.
- `rule:core-api/signing-is-over-a-payload` stays `designed` in `docs/rules/core-api.json` until
  `Core\Router`'s pair lands — the `5 registered` check is what flips it.
