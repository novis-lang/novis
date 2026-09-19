# Handoff

## State

Goal `lang-concurrency` is reached. `python tools/dossier.py --verify --group lang:concurrency`
prints `nothing owed` over all ten features of the chapter and `0 failed` for both suites (30
examples, 10 attacks). The last three features landed this session with every feature proof: the
channel, `limit` and `deadline`, and the no-colouring claim. `python tools/owners.py --closes
lang-concurrency` and `python tools/playbook.py --closes lang-concurrency` each report nothing
owned, and `python tools/verify.py --doc` is green.

Nothing is blocked. `lang:concurrency/what-does-not-exist` stays `[skip]`ped for perf alone in
`tools/data/dossier-policy.toml:64`; the other nine features each carry a measured figure in
`docs/perf/members.ndjson`, and the three new ones sit inside the group's existing spread.

## Next group

The chain's next goal is `lang-attributes`, whose own handoff `goal-switch.py` installs, so this
group is only what to take if the switch has not happened yet.

**Stage 2: the dossier** — one file set: `docs/reference/lang/90-attributes.md`,
`docs/examples/lang/attributes/`, `tests/hostile/lang/attributes/`,
`benches/members/lang/attributes/` and `tests/conformance/`.

- [ ] **`lang:attributes/an-attribute-is-a-shape-literal-attached-to-a-declaration`** — owes
      examples, hostile, perf, tests (`rule:testing/feature-proofs`).
      `docs/reference/lang/90-attributes.md:9`
- [ ] **`lang:attributes/reading-attributes-back-core-attributes-get-and-all`** — owes the same
      four proofs, and shares the reading half with the item above.
      `docs/reference/lang/90-attributes.md:93`
- [ ] **`lang:attributes/the-names-the-compiler-acts-on`** — owes the same four proofs.
      `docs/reference/lang/90-attributes.md:116`

## Backlog

- A task's `echo` is held until its group returns and is then printed one task at a time; the
  chapter never says so — `docs/reference/lang/80-concurrency.md` § *`Core\Task::all`*.
- Tasks do not start in the order their fields are written, which no chapter or rule states —
  `docs/reference/lang/80-concurrency.md` § *`Core\Task::all`*.
- `Core\Arr` has no `push`, so a trace across tasks is built by string concatenation — a gap only
  if `docs/reference/core/` means to offer one.
