# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json`, `Core\Jwe`, `Core\Jwe\Key`,
`Core\Jwt`, `Core\Jwt\KeySet` and `Core\Log` member owes nothing. `Core\Log` now has its class card
in `crates/nvs-stdlib/src/log.rs`, and its Rust test is the `covers:`-marked
`application_code_and_the_engine_floor_produce_schema_identical_records`. The attack on
`Core\Log::write` found no bug: a forged second line, 100,000-deep fields, a self-referencing object
and floods of distinct and identical records all hold. The decoder's debug-build stack overflow
stays `# Known gaps` 1 in `crates/nvs-stdlib/src/json.rs`, owner M12. What is left is
`Core\Mail::send`, which also owes the `Core\Mail` class card.

## Next group

**Stage 3: `Core\Mail`**: one file set, `crates/nvs-stdlib/src/registry.rs`'s
`CLASSES_STILL_OWING_A_CARD`, `crates/nvs-stdlib/src/mail.rs` and the `core/Mail/send` proof trees.
Record the bench figure after the last edit of `mail.rs` in the session.

- [ ] **`Core\Mail::send`**: owes about, examples, hostile, perf, tests and help, meaning a
      `ClassDoc` above the class row and `r"Core\Mail"` struck from `CLASSES_STILL_OWING_A_CARD`
      (`rule:testing/feature-proofs`, `rule:core-api/reference-card`).
      `crates/nvs-stdlib/src/mail.rs:1036`

## Backlog

- `nvs agent show 'Core\Log'` prints the member list and not the class card's `short`, the same as
  for `Core\Jwt`; whether a class card should print there is `rule:core-api/reference-card`'s.
