# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it — has just started; nothing of it
has landed yet.** Goal 15's whole list is this goal's Stage 1 floor. The design is settled in the goal
prose's standing decisions: ADR 0139 replaces spec § 15's three-way exclusivity with *buffering readers
share, streaming readers consume*; `Core\Request::json({maxDepth?}): tainted mixed` and
`jsonAs<T>({maxDepth?}): T` both land; `json()` holds its decoded value on the request and `jsonAs<T>()`
holds nothing; no `Content-Type` gate; an absent or empty body is a `ParseError`. One new ADR number and
no other.

The thing to understand before touching anything: **a `.nvst` case answers no request today**, which is
why every request-facing member in this repository is proven by a Rust `#[test]` name rather than a case,
and why goal 6's `upload.nvs` and `session.nvs` legs assert `"no request"`. Stage 2 is what changes that,
and it is the keystone for a reason — the three-`.nvst`-case floor
(`every_core_class_has_a_conformance_floor_of_three`) cannot be met honestly for `json`/`jsonAs` until it
lands.

## Next group

**Stage 0 then Stage 2** — they share `crates/nvs-test` and `tests/conformance/core/`, and stage 0 is
three files of re-pointing. Stage 0 first: a "0" stage runs ahead of the native leg, so leaving it is
what the ledger names.

- [ ] **Stage 0: re-point the exclusivity cases.** `a-request-post-refuses-on-the-terms-the-other-body-readers-do.nvst`
      becomes `a-request-post-reads-a-body-another-reader-held.nvst`; the two `nvs-stdlib` unit tests
      `a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other`
      (`crates/nvs-stdlib/src/request.rs`, near `claim_body` at `:1032`) and
      `body_stream_is_exclusive_with_body_and_with_files` keep their questions and change their answers.
      `a-request-refuses-every-read-when-no-request-arrived.nvst` is untouched — it is ADR 0012 § 7's
      rule, not the exclusivity one.
- [ ] **Stage 2a: the five sections** — `--GET--`, `--POST--`, `--POST_RAW--`, `--COOKIE--`, `--HEADERS--`
      in `crates/nvs-test/src/lib.rs:20`'s table and `case.rs`'s parse, spelled as `.phpt` spells them.
      `--POST--` with `--POST_RAW--` is a parse error.
- [ ] **Stage 2b: the carrier** — a case spawns `nvs run` (`crates/nvs-cli/src/main.rs:1163`), so the
      sections cross a process boundary as `nvs run --request <file>`. The `Ctx` is built at
      `crates/nvs-cli/src/main.rs:923`; `Ctx::set_inbound` is `crates/nvs-runtime/src/ctx/inbound.rs:23`; the
      build itself is three calls — `Inbound::new` (`:4508`), `push_header` (`:4544`), `set_body`
      (`:4565`) — and `crates/nvs-cli/src/serve.rs:323` is the worked example to copy.

## Backlog

- Stage 3 (ADR 0139, `hold_body`, `claim_body` rewritten, `body()` idempotent) shares
  `crates/nvs-runtime/src/ctx/inbound.rs` with stage 2b but nothing else; it is the natural second group and it is
  where the one new ADR number gets spent.
- Stage 4 (the two members, the five edits each, the spec § 15 bullet, six `.nvst` cases) needs stage 2
  and stage 3 both landed. Stage 5 is one example and the reference page.
- When this goal's last check goes green the driver takes goal 17 — `Core\Test::request`. It is the last
  entry on the chain.
