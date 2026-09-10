# Handoff

## State

**Goal `type-test` — every row of `rule:types/type-test`'s table lowers now, and the fixture is green.**
`test_shape` in `crates/nvs-ir/src/lower/expr.rs` answers a shape for every accepted right-hand side but
a shape type, a union, an intersection, `iterable`, `callable` and an `array<T>` whose element type no
tag decides. Those reach `lower_type_test`'s panic, and each is a decision about that rule's table
rather than a slice.

**A literal type and an enum case are one tag comparison with a payload compare behind it**, `&&`-shaped
rather than folded together because a `string` literal's compare is `nvs_str_eq` through two pointers and
must not run on a value tagged anything else. The atom and its constant are `as`'s own —
`convert::LiteralAtom` and `convert::literal_constant`, now shared — so `$x as 'yay'` and `$x is 'yay'`
compare identically, and an enum case compares one representation down through
`reinterpret_enum_to_backing` for the reason that method already gives.

**One consequence is pinned rather than closed:** a `mixed` holding a backing integer answers
`is Rank::Silver` exactly as one holding the case does. `rule:enums/representation` states it — "a value
that reaches `mixed` is not distinguishable there from its backing integer" — and the tag it reserves for
an enum is what would separate the two.

**What stage 6 still owes is the cases, not the code.** `examples/type-test.nvs` matches the acceptance
`want` list line for line, `python tools/reference.py --check` is green, and no `.nvst` case pins this
rule yet. ADR 0150 § *Verification* is that list, and `[context.stage.6]` now slices it.

## Next group

**Stage 6: the conformance cases** — one file set: `tests/conformance/lang/`, `tests/conformance/reject/`.

- [ ] **Every accepted row over a `mixed` subject, and totality beside it** — one case sweeping the
      table at `docs/rules/types/type-test.md:20`, `uint` and `decimal` included, and one asserting that
      `int $n; $n is int` is `true` and `$n is string` is `false` rather than either being a diagnostic.
      `rule:types/type-test`.
- [ ] **The distinction and the two divergence rows** — `"7" is int` is `false` where `"7" as ?int` is
      `7`; a `uint` answers `is uint` and not `is int`; binary data answers `is bytes`. No differential
      case, PHP being unable to run `is` at all, which is why these are conformance cases:
      `docs/decisions/0150.md:308`.
- [ ] **One reject case per refusal** — `E0811`, `E0812` and `E0813` at
      `crates/nvs-diagnostics/src/lib.rs:3308`. § *Verification* names `E0810` and that number is stale:
      it belongs to `E_DECODED_FIELD_NOT_TAINTED`, and the registry's own comment beside the third code
      says why the trio is numbered as it is.

## Backlog

- False-edge narrowing stays out of scope for all five spellings — `docs/decisions/0150.md` § 9.
- A shape, a union, an intersection, `iterable` and `callable` on the right — `test_shape`'s doc comment
  holds the gap; closing it is a decision about `rule:types/type-test`'s table.
- `is array<Foo>` reaches the panic where `as array<Foo>` is `E0711` — same gap, same doc comment.
