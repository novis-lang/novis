# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stages 0 and 2 have landed;
stages 3, 4 and 5 are open.** Stage 1 is goal 15's whole list, untouched. The design is settled in the
goal prose's standing decisions, which the pack prints in full.

**Stage 2 is closed and proven end to end.** A `.nvst` case describes a request in five `.phpt`-spelled
sections, `nvs run --request <file>` answers it, and two cases under `tests/conformance/core/` pin both
halves against the real binary — `Core\Request::query`, `::post`, `::header` and `::cookie` all answer
from a case today. The file the two ends agree on is `crates/nvs-test/src/request.rs`'s module doc,
which owns the format, the three facts a case does not write (method, path, query) and why `--BODY--`
is the rest of the file rather than a section.

`::body()` after `::post()` still refuses — that is spec § 15's three-way exclusivity, the sentence
stage 3 replaces. The old rule is still stated in `crates/nvs-stdlib/src/request.rs`: `claim_body`'s
message at `:1219`, the member comments at `:1985` and `:2067`, a test doc at `:4846`. Sweeping it is
stage 3c's, beside the rule that replaces it.

## Next group

**Stage 3: the rule — buffering readers share, streaming readers consume** — one file set:
`crates/nvs-runtime/src/ctx/inbound.rs`, `crates/nvs-stdlib/src/request.rs`, and the new fragment under
`docs/rules/http-server/` with its record. Take them in order; 3b and 3c are written against 3a.

- [ ] **Stage 3a: the body-read rule**, this goal's one new number — a new `http-server/` fragment and
      the decision record for it (re-derive the next free ADR number before claiming it; it was 0155 at
      the start of this session). A *streaming* reader — `bodyStream`, `files` — consumes the body and
      refuses every later reader; a *buffering* reader — `body`, `post`, `json`, `jsonAs` — keeps what
      it read, so any buffering reader may follow another, and `post()` joining `files` stops being an
      exception. Amend `rule:http-server/a-part-is-consumed-in-one-of-three-ways` and
      `rule:http-server/the-body-is-read-on-demand-under-two-caps` where they state the old rule,
      which is the rule `crates/nvs-stdlib/src/request.rs:1219` states to a program today.
- [ ] **Stage 3b: `hold_body`**, generalizing `Inbound::hold_form`
      (`crates/nvs-runtime/src/ctx/inbound.rs:685`), with `claim_body`
      (`crates/nvs-runtime/src/ctx/inbound.rs:634`) rewritten to answer the *class* of the holder
      rather than its name. `nvs_stdlib::request`'s `claim_body`/`claim_form` pair
      (`crates/nvs-stdlib/src/request.rs:1032`, `:1081`) collapses into one call against it.
      `rule:http-server/a-part-is-consumed-in-one-of-three-ways`.
- [ ] **Stage 3c: `body()` becomes idempotent** (`crates/nvs-stdlib/src/request.rs:1696`) — the same
      octets on every call, which is what makes middleware-then-handler work whether or not a JSON
      member is the one reading, plus the old rule's wording swept from `:1219`, `:1985`, `:2067` and
      `:4846`.

## Backlog

- Stage 4: `json()` and `jsonAs<T>()`, the five edits each — `docs/agent/loop-goal.md` § Stage 4.
- Stage 5: `examples/json-body.nvs` is the acceptance check the driver reports as failing, and it is
  the last stage, not an alarm.
- Stage 1 is goal 15's whole list, still untouched.
- No `.nvst` section spells a method or a path, so a case answering anything but a `POST` or `GET` at
  `/` cannot be written; `crates/nvs-test/src/request.rs` owns that derivation and the file format
  already carries both.
- `nvs run --request` is proven only through the runner. A hand-written request file — the other half
  of why it is a flag — has no case of its own.
