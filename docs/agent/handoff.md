# Handoff

## State

Goal `core-json-and-6-more` is under way. `Core\Json::decode`, `decodeAs` and `encode` owe
nothing now: each has `about.md`, three examples, a hostile case, a bench with a recorded figure
and a Rust test carrying its `covers:` marker, and `Core\Json`'s class card is done. The attacks
found no new bug. The decoder's debug-build stack overflow stays `# Known gaps` 1 in
`crates/nvs-stdlib/src/json.rs`, owner M12. `Core\Json::isValid` is the last `Core\Json` member,
then the `Core\Jwe` members start a new file set.

## Next group

**`Core\Json::isValid`, then `Core\Jwe`** — `isValid` shares `crates/nvs-stdlib/src/json.rs` and
the `Core/Json` proof trees with the landed members; the `Core\Jwe` pair is `crates/nvs-stdlib/src/jwe.rs`.

- [ ] **`Core\Json::isValid`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/json.rs:321`
- [ ] **`Core\Jwe::encrypt`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwe.rs:176`
- [ ] **`Core\Jwe::decrypt`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwe.rs:187`

## Backlog

- `Core\Json::decode` turns an integer literal below `i64::MIN` (and past `u64::MAX`) into a `float` without an error, while `DECODE_DOC` says an integer too large for `int` throws; the module doc's § *The refusals* names only the positive band — `crates/nvs-stdlib/src/json.rs`.
- The debug-build stack overflow in the decoder — `crates/nvs-stdlib/src/json.rs` § *Known gaps* 1.
- A `Core\Json` example calling `Core\IO::temporaryDir` was refused `fs.write` under `dossier.py --bless`, while the `Core\IO` examples making the same call pass; why was not checked, so the Json examples read string literals — `tools/dossier.py`.
- There is no global `NAN` or `INF`, and `Core\Math` has no constant for either; a proof builds them with `Core\Math::fdiv(0.0, 0.0)` and `fdiv(±1.0, 0.0)` — `docs/spec/02-php-migration.md`.
