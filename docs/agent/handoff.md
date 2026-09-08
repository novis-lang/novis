# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 2 is closed: both halves are on disk and
its four `nvs-types` checks pass.**

`Parses` carries `parse(tainted string $s): static` as its one required member and
`tryParse(tainted string $s): ?static` as a `rule:classes/interface-default-methods` default, seeded in
`crates/nvs-types/src/iter_lib.rs`. `?static` is not `returns_static` on purpose
(`crates/nvs-types/src/signatures.rs:1560`), so that member interns a nullable over the interface's own
class. `E0404` reports for the first time: an interface member declared `: static` answered by a
declaration that writes a class of its own is refused
(`crates/nvs-types/src/conformance.rs:@reject_dropped_static_return`).

**A named gap, and refusing is the safe half.** An inherited `tryParse` cannot be called from source —
the playbook bullet has the mechanism. There is no compiled body behind that default yet, so the
refusal is correct today and the repair is body-first, not visibility-first.

The goal's one record is still unwritten; `docs/agent/loop-goal.md`'s stage 5 schedules its prose half,
and it now owes `rule:core-api/reserved-namespace` and this `E0404` refusal in its `changes` block.
The narrowing in *Standing decisions* — this goal gives `as` no class-building meaning — is unchanged.

## Next group

**Stage 3: the predicate, the `nvs-stdlib` half** — one file set: `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/uuid.rs`, `crates/nvs-types/src/core_lib.rs`.

- [ ] **`implements_parses` beside `implements_comparable`** — `crates/nvs-stdlib/src/registry.rs:2553`
      is the precedent, and the structural question is the same shape: a `static` row named `parse`
      taking one `Text` and answering `CoreTy::Instance(self)`, and a `static` `tryParse` answering the
      nullable of it. `rule:expressions/try-parse` is the contract, and its three conditions are what
      the predicate encodes.
- [ ] **`Core\Uuid` satisfies it with not one line changed** — its rows are already exactly that shape
      at `crates/nvs-stdlib/src/uuid.rs:142` and `:151`. The two checks that pin it are named at
      `docs/agent/loop-goal.toml:5867`: `implements_parses_is_true_for_core_uuid_and_false_for_a_class_without_both_members`
      and `implements_parses_requires_try_parse_to_answer_the_nullable_self`.
- [ ] **The conformance edge is seeded from the predicate** — `crates/nvs-types/src/core_lib.rs:83` is
      the `implements_comparable` seed, two lines above where the `Parses` one goes, so a `Core` class
      that carries the members claims the interface the way a user class claims it with `implements`.

## Backlog

- Stage 3's `nvs-types` half: `converts_from_string` at `crates/nvs-types/src/commands.rs:771` asks the
  predicate instead of naming `Core\Uuid`, and the three diagnostics plus two doc comments in
  `crates/nvs-types/src/routes.rs` stop reciting the roster. Goal prose stage 3, items 1, 3 and 4.
- The default body has no code: nothing lowers `tryParse`, so `crates/nvs-types/src/layout.rs:213`'s
  comment is true only because there is nothing to enter in a method table. Goal prose stage 4 is where
  the runtime arm lands.
- `class Core {}` is still accepted: `crates/nvs-syntax/src/parser/decl.rs:194` guards the `Core\…`
  spelling and nothing guards the bare one. `rule:core-api/reserved-namespace` owns it.
- The goal's one new record is unwritten; `docs/agent/loop-goal.md`'s stage 5 is where its prose half
  is scheduled.
- Stages 4 and 5 as `docs/agent/loop-goal.md` lists them.
