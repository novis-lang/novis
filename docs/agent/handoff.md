# Handoff

## State

Goal `types-enum-1-2`: 9 of its 17 enums are complete — `Core\Audience`, `Core\Charset`,
`Core\Cldr\PluralCategory`, the three `Core\Cli` ones, and now the three `Core\Crypto` ones. Each has
`about.md`, three examples with blessed `.out` files, and one test attributed by a `covers:` marker.
An enum owes no bench and no attack, per `tools/data/dossier-policy.toml`'s per-kind table.

**Item 7, `Core\Codec`, is still open** — the last handoff's group named items 8 to 10 and walked past
it, so it is the first item below rather than the earliest one left behind. Nothing is blocked.

Two facts the crypto examples were written around, both of which the next enum in a random-keyed class
will meet again. A generated key makes an example's output non-deterministic, so anything printed is a
length, a round trip or a refusal sentence — never key material; where the printed thing has to be the
key itself, the frozen WebCrypto vectors inside
`tests/conformance/core/crypto-public-key-round-trips-every-kind-through-every-encoding.nvst` are the
source, and a P-256 and an Ed25519 `SubjectPublicKeyInfo` from it now carry the two `Core\Crypto\KeyFormat`
examples. And a `foreach` key binding is a `string` whatever the array holds
(`tests/conformance/lang/a-foreach-key-binding-is-a-string-and-nothing-else.nvst`), so an example that
walks two parallel arrays indexes them off `Core\Arr::range(0, N)` rather than off a keyed `foreach`.

## Next group

**Stage 2: the dossier, one enum per slice** — the goal's items 7, 11 and 12. Each owes `about.md`,
three examples and one attributed test (`rule:testing/four-proofs`); `python tools/dossier.py --id
'<name>'` prints its directory and `--bless <file>` writes the `.out`. The first is its own file set and
the other two share one, so take the first and then as much of the pair as the ceiling allows.

- [ ] **`Core\Codec`** — owes examples, tests. `crates/nvs-stdlib/src/compress.rs:132`
- [ ] **`Core\Db\ColumnType`** — owes examples, tests. `crates/nvs-stdlib/src/db/registry.rs:1073`
- [ ] **`Core\Db\Driver`** — owes examples, tests. `crates/nvs-stdlib/src/db/registry.rs:766`

## Backlog

- The five remaining `Core\Db` enums and `Core\Digest` close this goal's check — `docs/agent/loop-goal.toml:12140`.
- A `Core\Db` enum's examples may need a live statement; `nvs_db::sqlite::open` is the one engine a test
  reaches with no container (playbook, *Writing a test case*).
