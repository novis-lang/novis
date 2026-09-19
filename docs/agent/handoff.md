# Handoff

## State

Goal `lang:enums`. Two of the chapter's nine features carry their full feature proofs:
`a-case-is-its-integer` and `a-union-of-cases-is-a-narrower-type`. Seven are still owed, all in the
same file set. Nothing is blocked.

Two findings landed with them. The reference chapter claimed "a comparison does not narrow", which
is true only of an `||` of two comparisons (`docs/reference/findings.md` D19) — a single `==` does
narrow, as `tests/conformance/lang/an-enum-case-comparison-narrows-its-subject.nvst` has pinned all
along, and the chapter now says both halves. And `1 as Mode::Read|Mode::Write` was refused at
compile time while the same value in a variable converted at run time; the membership test in
`reject_impossible_literal_conversion` now reads the cases' values rather than their names.

## Next group

Three more features of the same chapter, sharing one file set: `docs/reference/lang/55-enums.md`
for what each one claims, plus the four proof trees under `docs/examples/lang/enums/`,
`tests/hostile/lang/enums/`, `benches/members/lang/enums/` and `tests/conformance/enum/`. One slice
is one feature with all its proofs, and the anchors below are the section each one is read from.

- [ ] **`lang:enums/an-enum-as-a-type`** — an enum's name at every binding site, and a case as an
      array index through `as int`. `rule:enums/closed-integer-type`.
      `docs/reference/lang/55-enums.md:193`
- [ ] **`lang:enums/comparing-cases`** — `==`/`!=` by value within one enum, and the disjointness
      refusal against an `int` or another enum. `rule:enums/closed-integer-type`.
      `docs/reference/lang/55-enums.md:121`
- [ ] **`lang:enums/match-and-switch-over-an-enum`** — source-order comparison, the throw on an
      unmatched `match`, fall-through in a `switch`. `rule:enums/closed-integer-type`.
      `docs/reference/lang/55-enums.md:151`

## Backlog

- `lang:enums/declaring-an-enum`, `from-an-integer-back-to-a-case`, `what-replaces-php-s-enum-members`
  and `core-enums` are the rest of this goal — `python tools/dossier.py --owed --group 'lang:enums'`.
- An `int`-backed enum cannot carry `int`'s own lowest value: the literal is refused (E0429) and a
  case value must be a literal (E0436), so there is no spelling for it —
  `docs/reference/lang/55-enums.md`.
- A ternary joins at the base type, so `return $first ? Mode::Read : Mode::Write` does not satisfy a
  `Mode::Read|Mode::Write` return, and `"a"|"b"` behaves the same — `rule:types/literal-types`.
