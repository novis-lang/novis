# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json`, `Core\Jwe`, `Core\Jwe\Key`,
`Core\Jwt` and `Core\Jwt\KeySet` member owes nothing. The `Core\Jwt` Rust tests sit at the end of
`crates/nvs-stdlib/src/jwt.rs`'s test module. `issued` drives `verifyIssued<{sub: string}>` through
`nvs_runtime::call` with a leaked one-field shape (`subject_shape`), and `key_set` drives
`Core\Jwt\KeySet::read`. No attack found a new bug. The decoder's debug-build stack overflow stays
`# Known gaps` 1 in `crates/nvs-stdlib/src/json.rs`, owner M12. What is left is `Core\Log::write`
and `Core\Mail::send`. Each also owes a class card, which is its help proof.

## Next group

**Stage 3: `Core\Log` and `Core\Mail`**: one file set, `crates/nvs-stdlib/src/registry.rs`'s
`CLASSES_STILL_OWING_A_CARD` plus each member's module and its `Core/Log` or `Core/Mail` proof
trees. Record each bench figure after the last edit of the member's own module in the session.

- [ ] **`Core\Log::write`**: owes about, examples, hostile, perf, tests and help, meaning a
      `ClassDoc` above the class row and its name struck from `CLASSES_STILL_OWING_A_CARD`
      (`rule:testing/feature-proofs`, `rule:core-api/reference-card`).
      `crates/nvs-stdlib/src/log.rs:215`
- [ ] **`Core\Mail::send`**: owes about, examples, hostile, perf, tests and its class card, in the
      same way. An example must not reach a real SMTP host, so read how the conformance cases
      for it point `send` at an endpoint first (`rule:testing/feature-proofs`).
      `crates/nvs-stdlib/src/mail.rs:1036`

## Backlog

- The decoder's debug-build stack overflow on deep nesting: `crates/nvs-stdlib/src/json.rs`
  `# Known gaps` 1, owner M12.
