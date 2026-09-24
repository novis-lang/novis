# Handoff

## State

Goal `core-json-and-6-more` is under way. `Core\Json::decode` owes nothing now: `about.md`, three
examples, a hostile case, a bench with a recorded figure, a Rust test carrying its `covers:` marker,
and `Core\Json`'s class card (deleted from `CLASSES_STILL_OWING_A_CARD`). The attack found that a
debug build overflows its stack decoding about 800 nested levels; release is fine even at the
ceiling from the soft stack limit, so the hostile case passes and the finding is `# Known gaps` 1
in `crates/nvs-stdlib/src/json.rs`, owner M12.

## Next group

The same file set: `crates/nvs-stdlib/src/json.rs`, and the `docs/examples/core/Json/`,
`tests/hostile/core/Json/` and `benches/members/core/Json/` trees. The class card is already done.

- [ ] **`Core\Json::decodeAs`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/json.rs:309`
- [ ] **`Core\Json::encode`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/json.rs:288`

## Backlog

- `Core\Json::decode` turns an integer literal below `i64::MIN` (and past `u64::MAX`) into a `float` without an error, while `DECODE_DOC` says an integer too large for `int` throws; the module doc's § *The refusals* names only the positive band — `crates/nvs-stdlib/src/json.rs`.
- The debug-build stack overflow in the decoder — `crates/nvs-stdlib/src/json.rs` § *Known gaps* 1.
- A `Core\Json` example calling `Core\IO::temporaryDir` was refused `fs.write` under `dossier.py --bless`, while the `Core\IO` examples making the same call pass; why was not checked, so the Json examples read string literals — `tools/dossier.py`.
