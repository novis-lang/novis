# Handoff

## State

Goal `core-json-and-6-more` is under way. Every `Core\Json` member now owes nothing:
`decode`, `decodeAs`, `encode` and `isValid` each have `about.md`, three examples, a hostile case,
a bench with a recorded figure and a Rust test carrying its `covers:` marker, and `Core\Json`'s
class card is done. The attacks found no new bug. The decoder's debug-build stack overflow stays
`# Known gaps` 1 in `crates/nvs-stdlib/src/json.rs`, owner M12. The `Core\Jwe` members come next
and start a new file set. `Core\Jwe` also owes its class card.

## Next group

**`Core\Jwe::encrypt` and `decrypt`, and the `Core\Jwe` class card** — one file set:
`crates/nvs-stdlib/src/jwe.rs`, `crates/nvs-stdlib/src/registry.rs`'s
`CLASSES_STILL_OWING_A_CARD`, and the `Core/Jwe` proof trees.

- [ ] **`Core\Jwe::encrypt`** — owes examples, hostile, perf, tests, and help: `Core\Jwe` has no `ClassDoc` yet (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwe.rs:889`
- [ ] **`Core\Jwe::decrypt`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/jwe.rs:941`

## Backlog

- The rest of goal `core-json-and-6-more`'s members after `Core\Jwe`: `python tools/dossier.py --id '<member>'` prints what each owes.
