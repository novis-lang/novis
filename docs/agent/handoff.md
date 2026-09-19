# Handoff

## State

Goal `lang:enums`. Five of the chapter's nine features carry their full feature proofs:
`a-case-is-its-integer`, `a-union-of-cases-is-a-narrower-type`, `an-enum-as-a-type`,
`comparing-cases` and `match-and-switch-over-an-enum`. Four are still owed, all in the same file
set. Nothing is blocked.

One claim of the chapter had no case at all: a `switch` over an enum falling through a label that
has no `break`. `tests/conformance/enum/a-switch-over-an-enum-falls-through-a-label-without-break.nvst`
pins it now, together with the fall into a `default` written last, against a `match` over the same
subject, which never runs on. Every other claim of these three features was already pinned, so the
edit there was the `covers:` marker that attributes the case.

## Next group

Stage 2, the dossier. The four features this chapter still owes, sharing one file set:
`docs/reference/lang/55-enums.md` for what each one claims, plus the four proof trees under
`docs/examples/lang/enums/`, `tests/hostile/lang/enums/`, `benches/members/lang/enums/` and
`tests/conformance/enum/`. One slice is one feature with all its proofs, and the anchor below is the
section each one is read from.

- [ ] **`lang:enums/declaring-an-enum`** — the `enum` declaration itself: a case with no written
      value counting on from the one before it, and the one backing type a declaration may name.
      `rule:enums/declaration`, `rule:enums/one-backing-type`.
      `docs/reference/lang/55-enums.md:8`
- [ ] **`lang:enums/from-an-integer-back-to-a-case`** — `as E` and `as ?E` over an integer, which
      reach a case only where one names that value. `rule:enums/closed-integer-type`.
      `docs/reference/lang/55-enums.md:86`
- [ ] **`lang:enums/what-replaces-php-s-enum-members`** — an enum body declares nothing but cases,
      and what a static class does instead. `rule:enums/no-class-machinery`.
      `docs/reference/lang/55-enums.md:283`
- [ ] **`lang:enums/core-enums`** — the enums `Core` itself ships, and that they are ordinary enums.
      `rule:enums/closed-integer-type`. `docs/reference/lang/55-enums.md:349`

## Backlog

- `benches/members/lang/enums/an-enum-as-a-type.nvs` declares `allocations 0` and no `calls` figure:
  it measures 1.00 calls per op, and an inline would make a declaration of 1 a false alarm —
  `benches/members/README.md`.
- Editing `docs/reference/lang/55-enums.md` stales every perf figure in this group, so a chapter
  edit costs a `--record-perf` for all of them — `docs/agent/playbook.md`.
