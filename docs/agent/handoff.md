# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json` and `Core\Jwe` member owes nothing, and
`dossier.py --verify --group 'Core\Jwe'` is green. `Core\Jwe\Key::shared` and `::password` owe
nothing either: `about.md`, three examples, an attack, a bench with a recorded figure, and the Rust
tests `shared_builds_a_dir_key_of_exactly_the_key_length_and_never_quotes_a_wrong_one` and
`password_builds_a_pbes2_key_from_any_text_and_only_that_text_opens_it`. `Core\Jwe::decrypt`'s Rust
test is `decrypt_tries_the_ring_in_order_and_refuses_a_ring_of_the_wrong_shape_first`. No attack
found a new bug. The decoder's debug-build stack overflow stays `# Known gaps` 1 in
`crates/nvs-stdlib/src/json.rs`, owner M12.

## Next group

**Stage 2: `Core\Jwe\Key`'s agreement statics, then `Core\Jwt`** — one file set:
`crates/nvs-stdlib/src/jwe.rs`'s test module and the `Core/Jwe-Key` proof trees first, then
`crates/nvs-stdlib/src/jwt.rs` and the `Core/Jwt` proof trees.

- [ ] **`Core\Jwe\Key::recipient` and `::own`** — owe about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). The test module's `constructed` helper calls a one-argument constructor and returns its sentence; an Ed25519 key is the `LogicError` from `not_an_agreement_key`. `crates/nvs-stdlib/src/jwe.rs:855`
- [ ] **`Core\Jwt`'s class card** — the `help` proof of all four `Core\Jwt` members: a `ClassDoc` above the class row, and the class deleted from the list (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/registry.rs:5189`
- [ ] **`Core\Jwt::sign`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwt.rs:1395`

## Backlog

- `Core\Jwt::signObject`, `::verify`, `::verifyIssued`, then `Core\Jwt\KeySet`, `Core\Log`, `Core\Mail` — the rest of this goal's stage 2, each `dossier.py --gate --group <class>`.
