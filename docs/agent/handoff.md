# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json`, `Core\Jwe`, `Core\Jwe\Key`,
`Core\Jwt`, `Core\Jwt\KeySet` and `Core\Log` member owes nothing. The `Core\Html` floor check,
which had timed out under load, is green again: `Core\Html::parse` and `::sanitize` now stop at
the request's memory limit while they build the tree (they had peaked at 12 GB under a 64 MB limit).
The two quadratic costs, nesting depth and attributes per tag, are `# Known gaps` 1 and 2 in
`crates/nvs-stdlib/src/html.rs`, owner M12, and each has its own marked attack. The decoder's
debug-build stack overflow stays `# Known gaps` 1 in `crates/nvs-stdlib/src/json.rs`, owner M12.
What is left is `Core\Mail::send`, which also owes the `Core\Mail` class card.

## Next group

**Stage 3: `Core\Mail`**: one file set, `crates/nvs-stdlib/src/registry.rs`'s
`CLASSES_STILL_OWING_A_CARD`, `crates/nvs-stdlib/src/mail.rs` and the `core/Mail/send` proof trees.
Record the bench figure after the last edit of `mail.rs` in the session.

- [ ] **`Core\Mail::send`**: owes about, examples, hostile, perf, tests and help, meaning a
      `ClassDoc` above the class row and `r"Core\Mail"` struck from `CLASSES_STILL_OWING_A_CARD`
      (`rule:testing/feature-proofs`, `rule:core-api/reference-card`).
      `crates/nvs-stdlib/src/mail.rs:1036`

## Backlog

- A cap on HTML tree depth and on attributes per tag would close `Core\Html` gaps 1 and 2. That
  is a decision about `rule:core-classes/html-parsing`'s output, owner M12
  (`crates/nvs-stdlib/src/html.rs` § Known gaps).
