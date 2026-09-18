# Handoff

## State

Goal `types-enum-1-2`: 6 of its 17 enums are complete — `Core\Audience`, `Core\Charset`,
`Core\Cldr\PluralCategory` and now the three `Core\Cli` ones. Each has `about.md`, three examples
with blessed `.out` files, and at least one test attributed to it. An enum owes no bench and no
attack, per `tools/data/dossier-policy.toml`'s per-kind table.

All three `Core\Cli` enums already had the test that pins their roster, so each attribution was one
`covers:` marker — `Core\Cli\Stream`'s on
`tests/conformance/core/cli-a-stream-that-is-no-terminal-shows-no-colour.nvst`, the other two on the
`crates/nvs-stdlib/src/cli.rs` tests that hold each roster against the runtime's own. Nothing is
blocked.

Two facts the examples had to be written around. An enum case is a name and not a number, so
`>=` between two cases does not compile and *at least this much colour* is
`($have as int) >= ($want as int)` — `rule:enums/closed-integer-type`. And the terminal answers
differently under `--bless` than in a terminal, so `Core\Cli::colorDepth()` is called in an example
but its answer is written to standard error, never to the lines the `.out` pins.

## Next group

**One file set: `crates/nvs-stdlib/src/crypto.rs`** — the goal's items 8 to 10, the three enums the
sealing and key members take. Each owes `about.md`, three examples and one attributed test
(`rule:testing/four-proofs`); `python tools/dossier.py --id '<name>'` prints its directory.

- [ ] **`Core\Crypto\Cipher`** — owes examples, tests. `crates/nvs-stdlib/src/crypto.rs:492`
- [ ] **`Core\Crypto\KeyKind`** — owes examples, tests. `crates/nvs-stdlib/src/crypto.rs:546`
- [ ] **`Core\Crypto\KeyFormat`** — owes examples, tests. `crates/nvs-stdlib/src/crypto.rs:617`

An example here prints key material, so give every one of them a fixed input rather than a fresh
key, or the `.out` differs on every run.

## Backlog

- `Core\Codec` is the goal's item 7 and the only one in `crates/nvs-stdlib/src/compress.rs`, so it
  has no group to share a file set with — take it beside whichever neighbour is cheapest.
- The goal's remaining seven are the `Core\Db` enums and `Core\Digest`, items 11 to 17.
