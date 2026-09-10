# Handoff

## State

**Goal `signed-urls`, stage 5 is closed.** The `5 refusals` check's three named tests exist and pass
in `crates/nvs-stdlib/src/signature.rs`'s test module; `examples/signed-url.nvs` is written and runs;
and the `until is written and never omitted` check names a path the corpus actually holds, plus the
three cases that pin the claim.

**Every member this goal owes is registered, bodied, covered and struck from its outstanding-key
file** — `crates/nvs-stdlib/tests/spec-members-outstanding.txt` and its two siblings name none of
`Core\Signature`, `$uri->sign`, `$uri->verifySignature`, `urlSigned` or `signedRoute`, and
`crates/nvs-stdlib/src/uri.rs`'s module doc no longer says the normalization lives on `compareTo`
alone. Stage 0's catch-up and stages 2–5 are all on disk.

**What the goal still owes is the rulebook's own status**, which is the next group: five rules it
implemented are still `designed` in `docs/rules/*.json`. A fragment is always currently true, so
that is a wrong rule rather than bookkeeping left over.

**The ordering is asserted end-to-end, never on the two halves apart.** The three new tests drive
`nvs_core_signature_verify` through `nvs_runtime::call` on a fixed clock
(`crates/nvs-stdlib/src/signature.rs:1810`), because what they pin is that `open` runs before
`judge` — which neither function holds on its own — and the second of them uses two rings at one
instant so that neither half of the case is vacuous.

## Next group

**Stage 5: the rulebook catches up with what shipped** — one file set: `docs/rules/core-api.json`,
`docs/rules/core-classes.json` and the fragments under those two topics.

- [ ] **The three `core-api` rules this goal implemented are `shipped`, not `designed`** —
      `docs/rules/core-api.json:553` (`a-lifetime-is-written`), `docs/rules/core-api.json:561`
      (`signing-is-over-a-payload`) and `docs/rules/core-api.json:571`
      (`one-refusal-except-expiry`). Each fragment's closing marker goes in the same slice:
      `docs/rules/core-api/a-lifetime-is-written.md:14` and
      `docs/rules/core-api/one-refusal-except-expiry.md:14` both still end **Designed, not
      shipped.** `rule:core-api/signing-is-over-a-payload`.
- [ ] **The two `core-classes` rules, the same way** — `docs/rules/core-classes.json:682`
      (`core-classes/signature`) and `docs/rules/core-classes.json:690`
      (`core-classes/router-signed-url`). Both classes are registered, bodied and covered, and the
      guard paths those two rules name are the `.nvst` cases that pass today.
      `rule:core-classes/router-signed-url`.
- [ ] **Regenerate, then gate** — `python tools/rules.py --render` writes `docs/rules/<topic>.md`
      and the generated artifacts from the JSON, and `python tools/rules.py --render --check` is
      what fails on a stale one; `tools/rules.py:99` is the status roster the first two items edit.

## Backlog

- Whether this goal is met is the driver's own acceptance run to say; nothing this session read
  names a stage-5 check still open — `docs/agent/loop-goal.md`.
- `Core\Router::urlSigned` has no compile-error case for an omitted `until`, because that door needs
  a route table and the case is single-file — `tests/conformance/core/signature-a-lifetime-is-written-and-omitting-it-does-not-compile.nvst`.
- The three doors share one `SIGNING` shape const, so a fourth door gets the lifetime rule for free —
  `crates/nvs-stdlib/src/signature.rs:179`.
