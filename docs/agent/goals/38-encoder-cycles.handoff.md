# Handoff

## State

**Goal 38 — An encoder ends a cycle where it closes, and every walk that can meet one is audited — has just started; nothing of it has landed yet.** Goal 25's whole list is this goal's Stage 1 floor.

The design is already settled by [0164](../../decisions/0164.md) and nothing in it is a session's to
re-decide. `Core\Json::encode` is safe today and misdiagnoses: `Encodable` carries a `depth`, both
walk arms check it against `DEPTH_CEILING`, and there is no identity anywhere — so a self-referencing
object walks to the ceiling and is then reported as *nesting*, sending the reader after a deep
structure that does not exist.

Three things a session must not re-decide, each argued in the record and restated in the goal's
*Standing decisions*: the set is the **ancestor chain and never everything seen**, because one object
held by two properties is shared rather than cyclic and must still encode by being written twice; the
answer is a **throw naming the path** and never a `{"$cycle": N}` marker in the document, because
this encoder produces somebody else's payload; and the **depth cap stays**, because it is not what is
wrong and is the only reason today's failure is safe.

## Next group

**Stage 2: the ancestor chain in `Core\Json::encode`** — one file set:
`crates/nvs-stdlib/src/json.rs`, and it opens in one `peek.py` call.

- [ ] **Carry the ancestors** — `crates/nvs-stdlib/src/json.rs:@Encodable`, beside the existing
      `depth`. Once ticked, the walk knows what it is inside and not merely how deep it is.
- [ ] **Refuse at the object arm** — `crates/nvs-stdlib/src/json.rs:@serialize_object`, before
      descending. The `DEPTH_CEILING` check stays where it is and keeps its own message.
- [ ] **Refuse at the array arm** — `crates/nvs-stdlib/src/json.rs:@serialize_array`, since a cycle
      closes through either.
- [ ] **The message names the path** — the property chain that closed the cycle, thrown as the
      `ThrownClass::Logic` this call site already throws.
- [ ] **The module doc, rewritten whole** — its refusal list gains a cycle and its depth paragraph
      stops implying the cap is the cycle answer. Rewritten, never edited to leave the old sentence
      beside the new one.

## Backlog

- **Stage 3 — the audit**, file set `crates/nvs-stdlib/src/{csv,uri,encoding}.rs`. Cheap to take in
  the same session as stage 2 only if that session is still under the ceiling: it shares no file with
  stage 2, and its answer per file may be "cannot reach an object graph", which is a finding to write
  down rather than a gap to close.
- **Stage 4 — the rulebook**, file set `docs/rules/classes.json` plus the fragment. Small, and it
  cannot run before stage 2 and stage 3 have landed the cases its `guardedBy` names.
- When this goal's last check goes green the driver takes goal 26.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
