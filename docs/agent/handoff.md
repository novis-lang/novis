# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json`, `Core\Jwe` and `Core\Jwe\Key` member
owes nothing. `Core\Jwt` and `Core\Jwt\KeySet` carry class cards, so every `Core\Jwt` member's help
proof is green, and `Core\Jwt::sign`, `::signObject` and `::verify` owe nothing. Their Rust tests
sit at the end of `crates/nvs-stdlib/src/jwt.rs`'s test module, built on the `signed`,
`signed_object` and `verified` helpers, which drive each member through `nvs_runtime::call` under
a fixed clock (`Ctx::set_fixed_clock`). `signed_object` hands the member a text-keyed array in place
of a shape, because Rust cannot build one and the member reads only the encoder's text. No attack
found a new bug. The decoder's debug-build stack overflow stays `# Known gaps` 1 in
`crates/nvs-stdlib/src/json.rs`, owner M12.

## Next group

**Stage 2: `Core\Jwt` and `Core\Jwt\KeySet`** — one file set: `crates/nvs-stdlib/src/jwt.rs` and its
test module, and the `Core/Jwt` and `Core/Jwt-KeySet` proof trees. Record each bench figure after
the last `jwt.rs` edit of the session: the ledger keys a figure on that file's text, so an earlier
figure goes stale.

- [ ] **`Core\Jwt::verifyIssued`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). The Rust test signs with `signed_object` under an Ed25519 pair and checks with the pair's public half; `tests/conformance/core/jwt-verify-issued-answers-the-written-type-from-either-key-spelling.nvst` is the shape of a call. `crates/nvs-stdlib/src/jwt.rs:1984`
- [ ] **`Core\Jwt\KeySet::read`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`); its proof trees are `Core/Jwt-KeySet/read`. `crates/nvs-stdlib/src/jwt.rs:2364`

## Backlog

- `Core\Jwt\KeySet`'s other members, after `read` — `python tools/dossier.py --id` names what each owes.
