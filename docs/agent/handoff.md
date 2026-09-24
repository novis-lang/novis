# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json` member owes nothing. `Core\Jwe::encrypt`
now has `about.md`, three examples, a hostile case, a bench and the Rust test
`encrypt_seals_under_the_algorithm_its_key_names_and_draws_a_fresh_iv_per_call`, which calls the
member itself under `dir`, PBES2 and `ECDH-ES`. `Core\Jwe` and `Core\Jwe\Key` both carry their class
cards now, so no `Jwe` member owes help. The attack found no new bug. The decoder's debug-build stack
overflow stays `# Known gaps` 1 in `crates/nvs-stdlib/src/json.rs`, owner M12. `encrypt`'s bench
still needs its first recorded figure (`--record-perf --id 'Core\Jwe::encrypt'`) if the sweep asks.

## Next group

**`Core\Jwe::decrypt` and the four `Core\Jwe\Key` statics** — one file set:
`crates/nvs-stdlib/src/jwe.rs` and the `Core/Jwe` and `Core/Jwe-Key` proof trees.

- [ ] **`Core\Jwe::decrypt`** — owes about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). The Rust test can reuse the `encrypted` helper in the test module and assert a ring's order and its three `LogicError`s through `nvs_runtime::call`. `crates/nvs-stdlib/src/jwe.rs:957`
- [ ] **`Core\Jwe\Key::shared` and `::password`** — owe about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwe.rs:808`
- [ ] **`Core\Jwe\Key::recipient` and `::own`** — owe about, examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwe.rs:855`

## Backlog

- `Core\Jwt` members and `Core\Jwt\KeySet::read`, then `Core\Log::write` and `Core\Mail::send`, are the rest of this goal (`docs/agent/loop-goal.md`).
