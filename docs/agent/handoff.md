# Handoff

## State

Goal `lang-concurrency`. Two of its eleven features now carry every feature proof:
`a-child-s-failure-is-a-value` and `a-child-s-throw`. Eight are still owed — `python
tools/dossier.py --owed --group 'lang:concurrency'` is the list, and `what-does-not-exist` is the one
that may want a `[skip]` rather than proofs.

Nothing is blocked. Two facts worth holding before the next feature: a `spawn script` path resolves
against the working directory (the new playbook bullet), and the chapter itself is the implementing
file every `lang:concurrency` perf figure is hashed against, so an edit to
`docs/reference/lang/80-concurrency.md` re-stales every figure in the group and each has to be
re-measured with `--record-perf`.

## Next group

The spawn and isolate half of the chapter — one file set: `docs/reference/lang/80-concurrency.md`,
`docs/examples/lang/concurrency/`, `tests/hostile/lang/concurrency/`,
`benches/members/lang/concurrency/` and `tests/conformance/isolate/`. The first two share the
companion-child idiom this session landed; take them in order.

- [ ] **`lang:concurrency/a-child-shares-nothing`** — owes examples, hostile, perf, tests.
      `tests/conformance/isolate/a-child-shares-nothing-with-its-parent.nvst` already pins it and
      needs the `covers:` marker. `docs/reference/lang/80-concurrency.md:276`
- [ ] **`lang:concurrency/isolates-spawn-script-and-await`** — owes examples, hostile, perf, tests.
      `tests/conformance/isolate/a-handle-is-collected-once-and-the-second-await-throws.nvst` is one
      of its two cases. `docs/reference/lang/80-concurrency.md:192`
- [ ] **`lang:concurrency/what-does-not-exist`** — owes examples, hostile, perf, tests, and every
      construct in it is a compile error, so it reads like the `[skip]` entries
      `tools/data/dossier-policy.toml` already holds for
      `lang:expressions/refused-in-expression-position`. `docs/reference/lang/80-concurrency.md:376`

## Backlog

- A group of 500 throwing tasks prints one `uncaught in a cancelled sibling: …` line per losing
  sibling to stderr; nothing in `docs/reference/lang/80-concurrency.md` § *A child's throw* says so,
  and a request can make that a log flood. Owner: that section, or `nvs-runtime`'s module doc.
- `docs/reference/lang/80-concurrency.md:214` says `limits:` and `grants:` at the spawn site are
  refused at compile time, and `tests/conformance/isolate/a-child-given-limits-is-stopped-at-its-own-ceiling.nvst`
  passes using both. Tested code wins, so the chapter line is stale — and fixing it re-stales every
  `lang:concurrency` figure, so it belongs with the last feature of this goal.
- `docs/examples/README.md:103` asks each companion to do nothing and succeed when run on its own,
  which a feature about a child that fails cannot satisfy. Owner: that README.
