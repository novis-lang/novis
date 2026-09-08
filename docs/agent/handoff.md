# Handoff

## State

**Goal 18, stage 2 is half landed.** `Ty::Shape` now carries a required bit per field
(`crates/nvs-types/src/ty.rs:390`), `{name?: T}` parses into it, and `is_assignable` lets an
optional key be absent. The other half of stage 2 — `tainted {…}`, and the record that carries both
— is untouched, and stages 3 and 4 (the converter and the two request members) have not started.

**One thing is knowingly out of step and it is the next group's first job.**
`rule:types/shape-type`'s fragment still says a field reached through a shape "is proven present, so
the read cannot fail". That is now false for an optional field: `({a?: int, x: int}) $p = {x: 1};
echo $p->a;` compiles and throws a catchable `RuntimeError` at run time (memory-safe, the same
mechanism `rule:types/erased-member-access` already specifies for an unnamed field). The fragment was
deliberately **not** edited, because doing so needs the goal's one record — see the next group.

## Next group

**Stage 2: the qualifier grammar and the stage's one record** — one file set:
`crates/nvs-syntax/src/parser/ty.rs`, `docs/rules/security/tainted-qualifier.md`,
`docs/rules/types/shape-type.md`, `docs/decisions/0157.md`.

- [ ] **ADR 0157 is the goal's one record and it covers stage 2 whole** — `docs/decisions/0157.md`
      (new; re-derive the next free number before claiming it), and the fragment it has to make true
      again is `docs/rules/types/shape-type.md:1`. *Standing decisions* allows one new
      number for the entire goal, so it cannot be spent on half a stage — that is why this session
      landed code ahead of its rule rather than opening it early. Its `changes.modifies` names
      **both** `types/shape-type` and `security/tainted-qualifier`, and both fragments are edited in
      the same commit as the record (`rule:` fragment shape, conventions § *A decision record*).
      It must decide the open question above: whether `$p->a` on an optional field is `?int`, a
      refusal, or the throw it currently is.
- [ ] **`tainted {…}` desugars at parse time** — `crates/nvs-syntax/src/parser/ty.rs:327`.
      `rule:security/tainted-qualifier`. **`tainted` is not a wrapper**: the arm at that anchor
      rewrites the inner *atom* to `TypeAtom::TaintedString`/`TaintedBytes` and reports
      `E_TAINTED_NON_SCALAR` for anything else, so admitting a shape means rewriting each field's
      atom, not wrapping the shape. `{a?: T}`'s required bit rides through untouched
      (`crates/nvs-syntax/src/parser/ty.rs:687`).
- [ ] **`tainted mixed` is settled in the same record** — goal 16's `Core\Request::json(): tainted
      mixed`, whose registry row is `crates/nvs-stdlib/src/request.rs:288`. There is no
      `TaintedMixed` atom today — `crates/nvs-types/src/expr/quals.rs:77` is the closed list of what
      carries taint — so this is either a third atom threaded through that list or a correction to
      goal 16's signature; *Standing decisions* pre-authorizes either and forbids `BLOCKED`.

## Backlog

- Reading an optional field is unanswered in `crates/nvs-types/src/expr/members.rs:1183` — it still
  types the field as present. Owned by ADR 0157 above, not by a separate decision.
- Stage 3: the `array<mixed>` → declared shape converter — `docs/agent/loop-goal.md`.
- Stage 4: the two `Core\Request` members over it — `crates/nvs-stdlib/src/request.rs`.
- `rule:core-api/shape-rules` R15's worked list and spec §§ 6 and 15's rosters still name no optional
  shape key — `docs/agent/loop-goal.md` § *Standing decisions* lists both as this goal's to change.
