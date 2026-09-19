# Handoff

## State

Goal `lang-classes` has 14 of its 15 features complete: `comparable`, `constants-and-class`,
`declaring-a-class`, `properties`, `methods-self-static-and-parent`, `inheritance`, `interfaces`,
`property-hooks`, `propertyobserver`, `stringable`, `parses`, `objects-are-handles`,
`nullable-objects-and` and `object-the-top-of-every-class-type`. Only
`what-a-class-cannot-declare` still owes anything, and nothing is blocked.

**A proof found a memory-safety bug and the fix is on disk.** `as <Class>` ran its class test only
when the operand was `Ty::Tagged`, so `$mixed as Other` threw while `$object as Other` and
`$base as WrongSibling` fell through to `convert`'s free `from == to` row and handed the operand
through under a declaration it does not satisfy; the next property read off that handle loaded the
target class's field offset out of another class's storage. The gate now takes `Ty::Object` too, and
`lower_checked_downcast` skips the `Untag` for an operand that has no tag.
`tests/conformance/class/a-conversion-to-a-class-the-object-is-not-throws-from-every-source.nvst`
pins all three sources refusing and all three still converting.

Three figures, and the spread is the story. A write through one handle plus a read through another
is 8.4 ns (2.7 units, 4 statements, 1 call, 0 allocations), so passing an object really is free. A
`?->` read on a present receiver is 2.3 ns (0.73 units, 0 calls, 0 allocations), so the null test is
close to nothing. A property read through an `object` receiver is 26.3 ns (9.1 units, 0 calls, 0
allocations): the name-keyed lookup is an order of magnitude over either static access, and it is the
figure to re-measure if erased access is ever made faster.

## Next group

**The last section of the chapter, then the goal's own end — one file set:**
`docs/reference/lang/50-classes.md`, `docs/examples/lang/classes/what-a-class-cannot-declare/`,
`tests/hostile/lang/classes/what-a-class-cannot-declare/`,
`benches/members/lang/classes/what-a-class-cannot-declare.nvs` and `tests/conformance/reject/`. One
slice is one feature with all of its feature proofs (`rule:testing/one-slice-is-one-feature`), and
`python tools/dossier.py --verify --group lang:classes` is the acceptance check.

- [ ] **`lang:classes/what-a-class-cannot-declare`** — owes examples, hostile, perf, tests. The
      section is three refusals: magic methods, anonymous classes and nested classes. Every one is a
      compile error, so the examples show what stands in their place — hooks and `PropertyObserver`
      for interception, a named class for the other two — and the attack carries
      `// hostile: expect-refusal`, which is what makes a clean compile the failure.
      `tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst` already
      holds the magic-method half and needs only a `covers:` marker, below its last refused line if
      it carries an `--EXPECTF-ERROR--` anchor. A bench over three compile errors may have nothing to
      measure; if so it is a `[skip]` entry in `tools/data/dossier-policy.toml` with the reason, not
      an empty file. `docs/reference/lang/50-classes.md:1249`
- [ ] **Run the goal's end gates before claiming it.** `python tools/verify.py --doc`, then
      `python tools/owners.py --closes lang-classes` and `python tools/playbook.py --closes
      lang-classes`, closing or re-ownering every gap they name. The driver does not reach the goal
      while one is red. `docs/agent/loop-goal.toml:12290`

## Backlog

- A perf figure for any `lang:classes/*` feature goes stale the moment
  `docs/reference/lang/50-classes.md` is edited, because that file is what the group's currency is
  read from — `rule:testing/member-perf-ledger`. Edit the chapter and the whole group re-measures.
- `rule:types/conversion`'s table has no row for a class target at all, though `as <Class>` is a
  conversion the chapter documents and the corpus now pins from three sources. Worth a row.
