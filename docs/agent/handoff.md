# Handoff

## State

Goal `types-enum-1-2`: 3 of its 17 enums are complete — `Core\Audience`, `Core\Charset` and
`Core\Cldr\PluralCategory`. Each has `about.md`, three examples with blessed `.out` files, and at
least one test attributed to it. An enum owes no bench and no attack, per
`tools/data/dossier-policy.toml`'s per-kind table.

`Core\Audience` had nothing on the tree, so it got two new cases — the open route and the refusal of
a text `allow`. The other two already had the case that pins their roster, so their test was a
`covers:` marker added to it, which is the whole edit `docs/agent/conventions.md` § *A feature's four
proofs* describes. Nothing is blocked.

## Next group

**One file set: `crates/nvs-stdlib/src/cli.rs`** — the goal's items 4 to 6, the three enums the
terminal members take. Each owes `about.md`, three examples and one attributed test
(`rule:testing/four-proofs`); `python tools/dossier.py --id '<name>'` prints its directory.

Watch one thing these three have that the first three did not: an example calling a member that
*asks the terminal* answers differently under the blesser than in a terminal, so keep an example's
printed lines to values the program itself decides.

- [ ] **`Core\Cli\Stream`** — owes examples, tests. `crates/nvs-stdlib/src/cli.rs:939`
- [ ] **`Core\Cli\ColorDepth`** — owes examples, tests. `crates/nvs-stdlib/src/cli.rs:973`
- [ ] **`Core\Cli\Shell`** — owes examples, tests. `crates/nvs-stdlib/src/cli.rs:1018`

## Backlog

- Items 7 to 17 of this goal: `Core\Codec`, the three `Core\Crypto` enums, the six `Core\Db` ones and
  `Core\Digest` — `docs/agent/goals/dossier/75-types-enum-1-2.md` lists them with anchors.
- A `Core\Charset` written in source does not unify with the registry's own enum type (`E0401`), so
  no program can put one in an `array<…>` or in a parameter. Two conformance cases state it in their
  comments; no `# Known gaps` entry carries it, and which crate owns it is not checked.
- `Core\Hash::hmac` takes `secret bytes` for its key, so a plain string literal is `E0401` — worth a
  line in that member's own examples when goal `dossier` reaches `Core\Hash`.
