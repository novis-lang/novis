# Handoff

## State

Goal types:exception, 8 of its 13 features complete. `ArithmeticError`, `Core\Cli\NotInteractive`,
`Core\Db\DbError`, `Core\Db\RolledBack` and `Core\Test\Failure` landed earlier; the three trunk
classes — `LogicError`, `RuntimeError` and `Throwable` — landed in this session. Each carries
`about.md`, three examples with blessed output, one attack and one `.nvst` case; exceptions owe no
bench. `python tools/dossier.py --verify --group types:exception` names the five that are left.

The trunk vocabulary the rest of the tree is now defined against: `LogicError` is *the mistake is
ours*, `RuntimeError` is *the world said no*, the two are disjoint from both sides, and `Throwable`
is the root a program's own class extends directly. The three new `.nvst` cases pin each of those
by **counting** over the whole tree rather than reading one arm, so a class that changes parent
moves a number. `crates/nvs-hir/src/errors.rs:96` is the tree; `:131` is the four properties every
class in it inherits at the same slots.

A `Core` member is the cheapest way to raise a real one in an example with no capability and no
grant: `Core\BigInt::format({radix: …})` and `Core\Json::decode(…, {maxDepth: 0})` throw
`LogicError`, `Core\Json::decode` on a malformed document throws `ParseError`.

## Next group

**Stage 2: the dossier — one file set: the exception tree and the three proof trees keyed off it** —
`crates/nvs-hir/src/errors.rs`, `docs/examples/types/`, `tests/hostile/types/`,
`tests/conformance/error/`. One slice is one feature with all four proofs, in this order:

- [ ] **`ParseError`** — owes about, examples, hostile, tests. Take it first of the three: it is the
      one narrow class that declares a property of its own, the `issues` list that makes a decode
      report every bad field rather than the first, so it has something to pin that the trunk cases
      did not. `rule:testing/four-proofs`. `crates/nvs-hir/src/errors.rs:101`, own property at
      `crates/nvs-hir/src/errors.rs:166`.
- [ ] **`IOError`** — owes about, examples, hostile, tests. The plainest member of the branch, and
      the one a reader meets first; `tests/conformance/error/a-runtime-error-clause-takes-the-whole-branch-under-it-and-nothing-beside-it.nvst`
      already pins its place, so its own case takes the boundary instead.
      `rule:testing/four-proofs`. `crates/nvs-hir/src/errors.rs:100`.
- [ ] **`TimeoutError`** — owes about, examples, hostile, tests. Same shape as `IOError` and the
      same file set, so it is cheap taken straight after it. `rule:testing/four-proofs`.
      `crates/nvs-hir/src/errors.rs:102`.

## Backlog

- `RecursionError` — the last of the branch; `crates/nvs-hir/src/errors.rs:103`, and the depth the
  stack allows sits between 2000 and 5000 frames, measured this session.
- `Core\Script\Finished` — the odd one out: it is a root of its own and **no** `catch` admits it, so
  its case asserts an absence and its example ends the script. `crates/nvs-hir/src/errors.rs:122`.
- `tests/conformance/error/a-catch-matches-every-supertype-of-what-was-thrown.nvst` names three of
  `RuntimeError`'s subclasses and not `RecursionError`; the new branch case counts all seven, so
  that older case is narrower than its title suggests rather than wrong.
