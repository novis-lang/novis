# Handoff

## State

**Both acceptance checks that were failing are closed.** `nvs-types (attribute retrieval folds)`
named four tests that did not exist; `crates/nvs-types/tests/attributes.rs` is all four, over ADR
0046 §§ 4-5's fold — the answer recorded at the call's own span as an `ExprInfo::CoreConst`, which
is what "no runtime lookup" means and what no conformance case can see, since a fold prints nothing.

**The fourth of them needed a rule that was prose and nothing else.** ADR 0046 § 4's last paragraph
— a *written* `$member` is checked against the target's real declarations, and only a computed one
falls back to an empty result — is now `E0798` in `crates/nvs-types/src/retrieval.rs`'s
`declares_member`, asked of `crate::signatures` rather than of the attach table so that an inherited
declaration counts and an unattributed one does too. The module doc owns why it has to be a refusal:
the `null` a misspelling folds to is the answer a correct retrieval of an absent attribute gives.

**One conformance case was pinning the absence of that rule** and now pins the rule instead — see
the playbook bullet. The refusal's rendered text lives in
`tests/conformance/reject/an-attribute-retrieval-is-refused-where-it-cannot-be-folded.nvst`, which
is five refusals now rather than four.

`E0797` has its case: `tests/conformance/reject/log-write-refuses-a-secret-field.nvst` covers all
three arms of `reject_secret_logged_argument` — the element read off the written literal, the same
bag as a named argument, and the argument whose own declared type carries the qualifier.

`Core\Cli::displayWidth` is untouched for the third session running; the acceptance failure outranked
it each time, and nothing outranks it now. `orient.py` printed no section of ADR 0046 although the
work lived entirely inside § 4: `[context] adrs` needs `0046:4` and `0046:5`, and still needs
`0019:1`, `0019:2`, `0014:3`, `0086:1`, `0086:3`, `0033:4` and `0054:3`; `[context] spec` still
misses `docs/spec/01-core-library.md` § 15.

## Next group

**`Core\Cli`'s last member and the case that measures it against its neighbour, over
`crates/nvs-stdlib/src/cli.rs` and `tests/conformance/core/`.**

- [ ] **`Core\Cli::displayWidth`** — ADR 0086 § 3's `displayWidth(string $value): uint`, UAX #11
      columns rather than `Core\Str::length`'s graphemes. The five edits of a `Core` member, at
      `crates/nvs-stdlib/src/cli.rs:213` (the rows — it goes after `colorDepth`, spec order),
      `crates/nvs-stdlib/src/cli.rs:362` (the cards, in row order beside `WIDTH_DOC`),
      `crates/nvs-stdlib/src/cli.rs:1020` (the `address()` arm) and
      `crates/nvs-stdlib/src/cli.rs:1286` (`nvs_core_cli_width`, the body to write beside). The
      decision to make first is where the width table comes from: a dependency is pre-authorized
      under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment, `cargo deny check` and
      `python tools/gen-attribution.py`.
- [ ] **The case that pins the other half** — one `.nvst` asking `displayWidth` and
      `Core\Str::length` the *same* question over a wide CJK run, a combining mark and a ZWJ
      sequence, and asserting they disagree exactly where UAX #11 says they must. The agreement
      shape from `docs/agent/conventions.md`, over the body written beside
      `crates/nvs-stdlib/src/cli.rs:1286`; the three cases the member's own five edits owe are not
      this, since each of those asks one member one question.

## Backlog

- `Core\IO::truncate` and `lock` — stage 2's handle half, `crates/nvs-stdlib/src/io.rs`.
- Reading `[log] target` — stage 7's owed half, `crates/nvs-stdlib/src/log.rs`.
- `Core\Decimal::allocate`, `pow`, `floor`, `ceil`, `round` — gap 1 of
  `crates/nvs-stdlib/src/decimal.rs`'s module doc.
- `Core\Cache`'s shared tier needs a reachable Docker daemon — the plan's `Blocking` field.
