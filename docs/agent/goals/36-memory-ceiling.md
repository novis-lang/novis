# Loop goal 36 — The memory ceiling stops a single operation

TODO: the target, in two or three sentences — what is different about the language, the runtime or
the tooling once this goal is green. Not the work; the outcome. Then one sentence on why this goal
sits where it does on the chain, which is the same sentence as the comment above its `[[goal]]`
block in [chain.toml](chain.toml).

Goal 35's whole acceptance list is this goal's floor, and it is never traded.

## Stage 0 — the catch-up

TODO: what is already on disk that contradicts this goal's rule, and is therefore rewritten before
anything new is written. `Nothing. No fixture predates the rule.` is a complete answer.

## Stage 1 — the floor

Goal 35's whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: TODO

TODO: the item list, **already grouped by file set** ([loop-authoring.md](../loop-authoring.md)
§ 7) — one numbered item per edit, each naming the file and the symbol it lands at
(`crates/<crate>/src/<module>.rs:@symbol`), so a session can open the group in one `peek.py` call.

## Standing decisions

- TODO: every tradeoff this goal will meet, decided here rather than by a session at 2am
  ([loop-authoring.md](../loop-authoring.md) § 5). Anything not pre-authorized is what makes a run
  stop.
- TODO: **this goal opens no ADR number**, or **opens ADR NNNN, whose `changes:` block names the
  rules it creates and modifies** — a record is frozen on acceptance and never amended in place,
  and the chain contract in [README.md](README.md) is that each goal names its slots.
- TODO: where ambiguity resolves to, so it is decided-and-recorded and never `BLOCKED`.
