# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it — is closed**, and this session
proved it rather than inferring it: `python tools/loop.py --goal-only` reports `GOAL REACHED: every
acceptance check passes`, 485 checks in 414s. The one check the driver reported red after session 0001,
`nvs-server (the connection seam) [1 floor]`, is green — `3c60dcf60` landed after that run, and
`serve::tests::a_connection_over_its_budget_is_closed_with_the_defined_code_not_oom` passes at
`crates/nvs-server/src/serve.rs:2719`. Nothing is blocked; the driver's next step is the next chain
entry, whose seed handoff replaces this file.

**The interrupted commit `b36ae3487` stands as it is.** It is a comment rewrite in `tools/loop.py:3081`
and `docs/agent/coordinator.md:230` — the sweep's docstrings stop reading as a changelog — with no code
change in it, so there was nothing to continue or revert.

**The status block's three stale fields are rewritten.** `Open now` was 2042 B against its 2000 B
ceiling and every clause in it belonged to a goal the chain has already walked; it now points at
`chain.toml`, which is the one home of what is open. `Status` no longer calls the parity program the
next target, and `Blocking` no longer files goals 4 and 5's Docker daemon as a live wall.

## Next group

**No stage — the next group is the next chain entry's, installed by the driver with its own seed
handoff.** If you are reading this because the switch has not happened, this is the item worth taking —
one file set: `docs/implementation-plan.md`.

- [ ] **`Done` is 755 B and still says M4's 1000-case corpus figure "is the one thing left"**, which
      the corpus met several goals ago, and it dates that to "goals 1–5" the chain has walked. Replace
      the clause with what M4 still owes, or drop it. `docs/implementation-plan.md:19` is the field,
      `docs/agent/conventions.md` § *A status-block field* is the shape, and `python tools/plan.py
      --check` is the gate.

## Backlog

- The plan's prose under the block links `docs/adr/` where records live in `docs/decisions/`;
  `python tools/check-links.py` says it resolves, so it is a naming question only —
  `docs/implementation-plan.md:79`.
