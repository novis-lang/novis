# Handoff

## State

Goal `types-enum-1-2`: 13 of its 17 enums are complete — `Core\Audience`, `Core\Charset`,
`Core\Cldr\PluralCategory`, the three `Core\Cli` ones, the three `Core\Crypto` ones, and now
`Core\Codec`, `Core\Db\ColumnType`, `Core\Db\Driver` and `Core\Db\ErrorKind`. Each has `about.md`,
three examples with blessed `.out` files, and one test attributed by a `covers:` marker. An enum owes
no bench and no attack, per `tools/data/dossier-policy.toml`'s per-kind table. Nothing is blocked.

Two facts the four `Core\Db` enums were written around, and the three left will meet again.
**An example never opens a database** — `docs/examples/README.md` § *What an example is* rules out an
example needing a database or a socket, so a `Core\Db` enum's examples use its cases as values and one
comment says what they stand for. **A `.nvst` case still can**: a `--FILE nvs.toml--` section holding a
`[capabilities.db] connect` grant and a `[db.main]` block with `driver = "sqlite"` and
`path = ":memory:"` gives the case a real engine, which is how
`tests/conformance/core/db-an-error-kind-is-the-condition-and-not-a-vendor-code.nvst` gets real
refusals to classify. A `Core\Db\DbError` thrown by the program itself carries `kind == Other` and an
empty `sqlState`, since no server classified it.

## Next group

**Stage 2: the dossier, one enum per slice** — the goal's last three `Core\Db` enums, one file set:
they are all declared in `crates/nvs-stdlib/src/db/registry.rs` and their proofs are three new
directories under `docs/examples/types/`. Each owes `about.md`, three examples and one attributed test
(`rule:testing/four-proofs`); `python tools/dossier.py --id '<name>'` prints its directory and
`--bless <file>` writes the `.out`. Grep `tests/conformance/` for the enum's name first — `Core\Codec`,
`Core\Db\ColumnType` and `Core\Db\Driver` each had a case to mark, and `Core\Db\ErrorKind` had none and
needed one written.

- [ ] **`Core\Db\Isolation`** — owes examples, tests. `crates/nvs-stdlib/src/db/registry.rs:891`
- [ ] **`Core\Db\Plan\Grade`** — owes examples, tests. `crates/nvs-stdlib/src/db/registry.rs:1912`
- [ ] **`Core\Db\Tls`** — owes examples, tests. `crates/nvs-stdlib/src/db/registry.rs:830`

## Backlog

- `Core\Digest`, the goal's seventeenth and last enum, is the one outside that file set:
  `crates/nvs-stdlib/src/hash.rs:153`.
- `Core\Db\Tls` declares three cases the runtime refuses at the call, so its examples say what is
  refused rather than calling `Core\Db::open` — `crates/nvs-stdlib/src/db/registry.rs:812` is the
  reasoning.
