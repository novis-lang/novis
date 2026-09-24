# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json`, `Core\Jwe` and `Core\Jwe\Key` member
owes nothing: `dossier.py --verify --group 'Core\Jwe\Key'` is green over all four statics. The Rust
tests for the two agreement statics are
`recipient_seals_to_one_public_key_and_refuses_a_key_that_only_signs` and
`own_opens_what_was_sealed_to_its_pair_and_refuses_a_pair_that_only_signs`, built on the test
module's `pair` and `public_of` helpers in `crates/nvs-stdlib/src/jwe.rs`. No attack found a new
bug. The decoder's debug-build stack overflow stays `# Known gaps` 1 in
`crates/nvs-stdlib/src/json.rs`, owner M12.

## Next group

**Stage 2: `Core\Jwt`** — one file set: `crates/nvs-stdlib/src/jwt.rs` and its test module, the
`Core/Jwt` proof trees, and the class-card list in `crates/nvs-stdlib/src/registry.rs`.

- [ ] **`Core\Jwt`'s class card** — the `help` proof of all four `Core\Jwt` members: a `ClassDoc` above the class row, and the class deleted from the list (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/registry.rs:5248`
- [ ] **`Core\Jwt::sign`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwt.rs:1395`
- [ ] **`Core\Jwt::signObject`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwt.rs:1438`

## Backlog

- `Core\Jwt::verify`, `::verifyIssued`, then `Core\Jwt\KeySet`, `Core\Log`, `Core\Mail` — the rest of this goal's stage 2, each `dossier.py --gate --group <class>`.
