# Handoff

## State

Goal `core-io-2-2` is met. Every member of its two `Core\IO` checks has all its feature proofs,
and `stdin` was the last. `python tools/dossier.py --verify --only …` is green for both halves.

A proof program can now be fed standard input. A sibling `<name>.in` beside an example, an attack
or a bench is sent to its standard input byte for byte. `tools/dossier.py`'s `run_proof` does that,
and `proof_digest` adds the file to the green cache key. `docs/examples/README.md` § *A file for
standard input* is the rule, and the website mirror copies `.in` files.

A full `dossier.py --run all` sweep is green: 1624 examples and 537 attacks pass, with the known
gaps unchanged.

## Next group

**Stage 1: goal `core-io-file-and-1-more`, `Core\IO\File`'s members** — one file set:
`crates/nvs-stdlib/src/io.rs` (the `FILE` class rows, their bodies and the test module at the end)
and each member's new proof paths.

- [ ] **`Core\IO\File::close`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:1418`
- [ ] **`Core\IO\File::flush`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:1397`
- [ ] **`Core\IO\File::lock`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:1406`

## Backlog

- `Core\IO::stdin`'s bench runs one read per process, so its clock figure is mostly process-start
  noise. Its four counts are exact. Owner: `benches/members/README.md`.
- `Core\IO::stdin` allocates about four times its input (a doubling `read_to_end` buffer, then a
  copy into the string). Owner: `crates/nvs-stdlib/src/io.rs` `read_to_text`.
