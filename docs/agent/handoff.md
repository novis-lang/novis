# Handoff

## State

**ADR 0086 § 2's `Text + Text` is closed, and stage 3 with it.** Two `Cli\Text` compose into a
`Cli\Text`; every other pairing — a `string` operand, and the cross-carrier `Text + Markup` — is
refused with a help line naming § 2. `Core\Cli::write` and `echo` take the sum as the carrier, so
the raw path survives composition.

**The two sink carriers are one rule, in one function.** `carrier_of` in
`crates/nvs-types/src/expr/operators.rs` answers which carrier a type is, and
`carrier_composition_result` admits a carrier beside its own kind and nothing else — replacing the
`Markup`-only `markup_composition_result`. `nvs_runtime::is_carrier` is the same pair of names on
the render side; the playbook's *"`Core` is two rosters, not one"* bullet is why they are joined
rather than each stated where it is used.

**Which carrier is the checker's answer, not the lowering's.** `nvs_ir::ty::Ty` erases a class to
`Ty::Object`, so both carriers reach `lower_binary` as the same pair of representations. The
checker records `ExprInfo::CarrierComposition { symbol }` at the `+` span and `lower_carrier_concat`
emits that symbol — `ExprInfo::SecretEquality`'s shape, one step further along because there are two
answers now rather than one bit. `nvs_stdlib::cli` is `pub` for exactly two constants, `NAME` and
`TEXT_CONCAT_SYMBOL`, reached as `nvs_types::CORE_CLI_TEXT_CLASS`/`CORE_CLI_TEXT_CONCAT`.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs §§ 15-19. Those are `Core\Request` and its neighbours and are
goal 6's, so this check cannot pass inside this goal and is not a regression.

**Read the plan's stage 7 clause before taking it.** *"`write`'s `fields` refuses a `secret`"*
scans as owed, but `reject_secret_logged_argument` and `is_fields_argument` are already on disk
(`crates/nvs-types/src/expr/quals.rs:658`, `:708`) under `E_SECRET_LOGGED`. What is owed there is
reading `[log] target`.

**Two pack gaps, both still open.** `[context] modules` does not select
`nvs-runtime/src/terminal.rs`. And `[context] playbook` filters to the *item's* anchor paths, so a
group that also writes `.nvst` cases never sees the case-authoring bullets — this session wrote two
cases and paid for the `--EXPECTF-ERROR--` indentation rule out of `conventions.md` instead.

## Next group

**`Core\Storage::list` — ADR 0082 § 2's third member and the last of stage 9's three owed pieces
that has its whole file set in one place — over `crates/nvs-stdlib/src/storage.rs` and
`tests/conformance/core/`. `put`/`get`/`delete` are the worked precedent for every step.**

- [ ] **The row and its card.** A `list` beside `delete` in `CLASS`, with the reference card in the
      block after it, in row order — `crates/nvs-stdlib/src/storage.rs:122`, `delete`'s own row at
      `crates/nvs-stdlib/src/storage.rs:149`. Read ADR 0082 § 2 for what it answers over and whether
      a prefix is a parameter before writing the signature.
- [ ] **The body and the `address()` arm**, over the same `fs.*` capabilities the other three reach
      the disk through and no capability of its own — `crates/nvs-stdlib/src/storage.rs:396` is
      `get`'s body, `crates/nvs-stdlib/src/storage.rs:253` the arm a miss turns into a runtime panic.
- [ ] **Three `.nvst` cases** under `tests/conformance/core/`, one per question:
      `crates/nvs-stdlib/src/storage.rs:122` names the members they have to agree with. The floor in
      `crates/nvs-stdlib/tests/conformance_coverage.rs` is three, each asking something different.

## Backlog

- Redis `AUTH` — `crates/nvs-stdlib/src/cache/redis.rs`, `docs/agent/loop-goal.md` stage 9.
- `Core\Mail`'s TLS — `crates/nvs-stdlib/src/mail.rs`, same stage.
- Reading `[log] target` — `crates/nvs-stdlib/src/log.rs`, stage 7's one remaining piece.
- `[context] modules` is missing `nvs-runtime/src/terminal.rs` — `docs/agent/loop-goal.toml`.
- `[context] playbook` filters to the item's anchors, so a group writing cases never sees the
  case-authoring bullets — `docs/agent/loop-goal.toml`.
- §§ 15-19 are goal 6's, so stage 10's registration gate cannot pass in this goal —
  `docs/agent/loop-goal.toml`.
