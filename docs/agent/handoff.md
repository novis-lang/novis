# Handoff

## State

Goal `core-db-row`: thirteen of the fourteen members are complete. `float`, `has` and `get` landed
this session, each with all of `rule:testing/feature-proofs`'s proofs. `instant` carries its two
tests and its `about.md`, with its three program proofs skipped for a reason each in
`tools/data/dossier-policy.toml`. Only `toArray` is left, and `python tools/dossier.py --group
'Core\Db\Row'` is the board.

A language bug the `get` attack found is fixed in the same session. The tag-dispatched conversion
rows in `crates/nvs-runtime/src/helpers.rs` carried no `Tag::Decimal` arm, so `$d as ?int` answered
`null` where `$d as int` answered the number — against `rule:types/conversion`, whose `as ?T` yields
`null` exactly where `as T` throws. The three decimal rows now sit in that file's `mod row` and both
spellings read them, so the invariant that module's own doc states is true rather than promised.

`target/release/nvs.exe` is rebuilt by `--bless` and again by `--record-perf` whenever a `crates/`
edit landed between them, so make every Rust edit of a group before the first `--bless`.

## Next group

The last member of the group, and then the goal's own end. `toArray` is the only member that
answers the whole row rather than one column, so its examples are not the shape the thirteen landed
ones take. One file set: `crates/nvs-stdlib/src/db/row.rs` (the member and the `mod tests` its Rust
proof goes in), `nvs.toml` (one `[[app]]` grant per proof program) and the four trees under
`core/Db-Row/toArray/`.

- [ ] **`Core\Db\Row::toArray`** — owes about.md, examples, hostile, perf, tests. It answers the
      whole row as `array<string, mixed>`, and answers the row's own array under a second reference
      rather than a copy, so the claim only a Rust proof can carry is that a caller writing to what
      it got separates it and leaves the row untouched.
      `crates/nvs-stdlib/src/db/row.rs:1022`
- [ ] **Reach the goal** — with `toArray` landed the group's board is clean, so run the goal-end
      gates the session prompt names (`verify.py --doc`, `owners.py --closes`, `playbook.py
      --closes`) and close or re-owner every gap they name before writing `DONE`.
      `docs/agent/loop-goal.md:1`

## Backlog

- `Core\Arr::shapeAs`'s hydration reads the same conversion rows (`crates/nvs-stdlib/src/json.rs`),
  so a `DECIMAL` field now coerces into an `int` field; no case pins that half —
  `rule:types/conversion`.
- `tests/hostile/core/Db-Row/get` step 5 holds a 16 MB value and is the heaviest attack in this
  group; it passes well inside the default limit.
