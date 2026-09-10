# Handoff

## State

**Goal `signed-urls`, stage 4 is closed.** `Core\Router::urlSigned` and `Core\Router::signedRoute`
are both registered, bodied, covered and tested; the `4 router` check's four named tests pass, as do
three new `.nvst` cases. **Stage 5 is entirely unwritten** — none of its three named tests exists and
neither does `examples/signed-url.nvs`, which is the goal's earliest failing acceptance check.

**`signedRoute` takes the ring and nothing else.** It reads the match the door took and the request's
own query off the carrier (`crates/nvs-stdlib/src/router.rs:1230`), derives `signed_payload`'s
document from those two rather than from the URL, and builds the `Core\Router\Match` only after
`confirm` and `judge` have both passed — because building it is what runs a class-typed capture's
`parse`, and no forged link should reach a program's own code
(`rule:core-classes/router-signed-url`, `rule:core-api/one-refusal-except-expiry`).

**The derivation is `derived_payload` at `crates/nvs-stdlib/src/router.rs:1119`**: the query parsed
as `Values::Text`, then each capture over it, and a query pair naming a capture is the one refusal —
`crate::uri::build` writes the query out of what the path did *not* consume, so such a request is one
no signed link could have been minted as, and letting the capture overwrite it would be the one added
parameter the signature did not cover. `capture_text` beside it is `capture_value`'s arms read as
text, reaching **no** program code.

**Three small seams were opened for it**, each with one home: `crate::request::served` (the carrier,
for a reader outside `Core\Request`), `crate::uri::without_signature` is now `pub(crate)` and its doc
covers a raw request query, and `crate::uuid::canonical` is the hyphenated-lower-case spelling
`toString` and this door now share.

## Next group

**Stage 5: one refusal, except expiry, and the ordering that makes it safe** — one file set:
`crates/nvs-stdlib/src/signature.rs`, `tests/conformance/core/`, `examples/`.

- [ ] **The three named tests of the `5 refusals` check**, in `crates/nvs-stdlib/src/signature.rs`'s
      test module: `a_token_past_its_until_throws_the_expired_error`,
      `a_token_both_forged_and_past_its_until_throws_the_invalid_error_not_the_expired_one` and
      `every_other_way_of_not_being_authentic_raises_one_error_with_one_sentence`. The ordering they
      pin is `crates/nvs-stdlib/src/signature.rs:857` (`open`, which checks the tag and the domain)
      before `crates/nvs-stdlib/src/signature.rs:457` (`judge`), with
      `crates/nvs-stdlib/src/signature.rs:431` (`refused`) as the one sentence the first three
      failures share. `rule:core-api/one-refusal-except-expiry`.
- [ ] **`examples/signed-url.nvs`** — the goal's `files` manifest names it and nothing in
      `docs/agent/loop-goal.toml` freezes its output, so this is the earliest failing acceptance
      check and the cheapest to close. `crates/nvs-stdlib/src/router.rs:245` (`urlSigned`) and
      `crates/nvs-stdlib/src/router.rs:278` (`signedRoute`) are the pair it should show against a
      `#[Route]`; `examples/serve.nvs` is the shape a served example takes.
      `rule:core-classes/router-signed-url`.
- [ ] **The `until is written and never omitted` suite check**, `docs/agent/loop-goal.toml:7055`,
      names `tests/conformance/core/signature/`, a **directory that does not exist** — the corpus is
      flat with a `signature-` prefix, and
      `tests/conformance/core/signature-an-expired-signature-is-the-one-distinguishable-refusal.nvst`
      already pins most of the claim. Decide between making the directory and amending the check's
      `args` before writing a case for it; the playbook's *A loop-goal.toml `nvs-suite` check can
      name `.nvst` paths in a directory layout the corpus never adopted* is the trap.
      `rule:core-api/a-lifetime-is-written`.

## Backlog

- `Core\Router::url` and `::urlAbsolute` are still on `registry.rs`'s `UNCLASSIFIED` list and still
  refuse a `tainted` name; classifying them `CoreTy::Text(Qual::Sink)` is one line each.
- A `Core\Uuid` capture is signed as its canonical rendering, so a link minted with an upper-case
  UUID *text* does not verify — `crates/nvs-stdlib/src/router.rs:1063` (`capture_text`) owns why.
- The pack's `[context] modules` names no `nvs-runtime` path, so `routes::Param` and `Inbound` were
  read cold; add `crates/nvs-runtime/src/routes.rs` and `crates/nvs-runtime/src/ctx/inbound.rs` if a
  later goal touches the match again.
