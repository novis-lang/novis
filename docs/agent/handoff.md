# Handoff

## State

Goal `types-exception` is complete. All 13 features carry their four proofs, `python
tools/dossier.py --verify --group types:exception` is green — nothing owed, 0 failed examples, 0
failed hostile — and `owners.py --closes types-exception` and `playbook.py --closes types-exception`
name nothing. `RecursionError` and `Core\Script\Finished` landed this session, each with `about.md`,
three examples with blessed output, one attack and one `.nvst` case; exceptions owe no bench.

The two classes' proofs each raise the real thing rather than describing it: a walk over data that
points back at itself reaches `RecursionError` from the runtime, and `Core\Script::finish()` raises
`Core\Script\Finished` in an example that ends where it is called. Neither needs a capability grant.

Next in the chain is goal `types-interface`, whose generated handoff the driver installs over this
file at the switch.

## Next group

**Stage 2: the dossier — one file set: the reserved interface table and the three proof trees keyed
off it** — `crates/nvs-hir/src/interfaces.rs`, `docs/examples/types/`, `tests/hostile/types/`,
`tests/conformance/`. One slice is one feature with all four proofs, `rule:testing/four-proofs`:

- [ ] **`Comparable`** — owes examples, tests. The interfaces carry no constructor and no member of
      their own, so a proof implements one on a small class of its own.
      `crates/nvs-hir/src/interfaces.rs:45`
- [ ] **`Iterable`** — owes examples, tests. Declared with a type parameter, which
      `rule:iteration/concrete-generic-implements` is the home of.
      `crates/nvs-hir/src/interfaces.rs:49`
- [ ] **`Iterator`** — owes examples, tests. `crates/nvs-hir/src/interfaces.rs:50`

## Backlog

- `rule:errors/stack-depth` says the bound is "about 65,000 frames"; measured, an ordinary static
  method stops at about 4,400 calls, because that figure is the reserved 8 MB over a minimal frame
  rather than over a real one. The playbook carries the measured number; the rule's sentence is
  worth one clause if anyone re-opens it (`docs/rules/errors/stack-depth.md:11`).
- `docs/examples/types/RecursionError/02-a-depth-limit-of-your-own.nvs` is the only proof in the
  group that raises the class from user code with a limit of its own; the reference chapter
  `docs/reference/lang/70-errors.md:369` does not mention that spelling.
