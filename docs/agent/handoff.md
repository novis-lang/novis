# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json`, `Core\Jwe` and `Core\Jwe\Key` member
owes nothing. `Core\Jwt` and `Core\Jwt\KeySet` carry class cards (`CARD` and `KEY_SET_CARD` in
`crates/nvs-stdlib/src/jwt.rs`), so every `Core\Jwt` member's help proof is green, and
`Core\Jwt::sign` owes nothing: its Rust test is
`sign_writes_the_clock_and_the_lifetime_after_the_claims_and_verifies_under_its_key`, built on the
test module's `signed` helper, which drives the member through `nvs_runtime::call` under a fixed
clock (`Ctx::set_fixed_clock`). No attack found a new bug. The decoder's debug-build stack overflow
stays `# Known gaps` 1 in `crates/nvs-stdlib/src/json.rs`, owner M12.

## Next group

**Stage 2: `Core\Jwt`** — one file set: `crates/nvs-stdlib/src/jwt.rs` and its test module, and the
`Core/Jwt` proof trees. Record each bench figure after the last `jwt.rs` edit of the session: the
ledger keys a figure on that file's text, so an earlier figure goes stale.

- [ ] **`Core\Jwt::signObject`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). Its claims are a Novis object, which Rust cannot build, so the Rust test drives a refusal path or `object_payload_of`'s name check. `crates/nvs-stdlib/src/jwt.rs:1452`
- [ ] **`Core\Jwt::verify`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). The `signed` test helper already gives it a token to read. `crates/nvs-stdlib/src/jwt.rs:1500`
- [ ] **`Core\Jwt::verifyIssued`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwt.rs:1984`

## Backlog

- `Core\Jwt\KeySet::read`, then `Core\Log` and `Core\Mail` — the rest of this goal's stage 2, each `dossier.py --verify --group <class>`.
