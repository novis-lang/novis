# Handoff

## State

**`Core\Password` is off `gaps.py`'s thinnest list.** Its three members went from three case files
asking one question each to six, and the three added here are the three shapes the class had no
case for: agreement under a second call, its cost floor named one unit apart on both axes, and
`verify`'s whole refusal surface counted as one shape. `python tools/gaps.py` no longer ranks the
class in its first twenty-five; `Core\Ast\Node`, `Core\Crypto` and `Core\Script\ExitReport` are the
floor now.

- **The third slice's premise as the handoff wrote it was wrong, and the case pins the corrected
  thing.** The item asked for a stored value that is not a hash to be "a `false` rather than a throw
  or a fatal"; the module throws `LogicError` there on purpose, and `password.rs`'s own § *A stored
  hash that will not parse throws* is the home of why. So the case pins the **line between the two
  refusals** instead — every way the *password* is wrong is one `false`, every way the *stored hash*
  is wrong is a throw — and counts both sides together, which is the assertion the item was reaching
  for.
- **The bound case reads its own floor back off a fresh hash.** Five lines assert against the
  literal `m=19456,t=2`, and the sixth asserts `Core\Password::hash()` still writes exactly that
  prefix, so a parameter change fails on that line rather than leaving five assertions about a
  number nothing uses. The adjacency is the point: the landed
  `password-needs-rehash-answers-weaker-not-different.nvst` puts its stale hash 11 MiB below the
  floor and its current one 45 MiB above, so a `<=` where the module writes `<` passes it either
  way round.
- Nothing in `crates/nvs-stdlib/src/password.rs` changed — three `.nvst` files and nothing else — so
  there is no new refcount edge and no valgrind run behind them. `python tools/verify.py` is 8 of 8
  green at conformance 1296.

**The two `orient.py` warnings the last session reported are still there**: the `[context] modules`
patterns `crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as
matching no module and are then printed in the scoped map below. The manifest is right; the matcher
is what to check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**`Core\Ast\Node` is the thinnest class left with every member on the floor — `kind`, `children` and
`nodes`, three cases each — and all three are bound by one rule, that the tree is *inert*: it is
built once by `Core\Ast::parse` and nothing a program does to it re-enters the compiler. The file
set is `crates/nvs-stdlib/src/ast.rs` and `tests/conformance/core/`; the three helper bodies are
within twenty-five lines of each other. The landed cases are
`core-ast-parse-answers-the-compilers-own-tree.nvst`,
`core-ast-parse-stops-where-the-compiler-stops.nvst` and
`core-ast-nodes-is-children-closed-transitively.nvst` — read those first, because the transitive
closure question is already asked and must not be asked twice.**

- [ ] **A case that asks each member twice and asserts agreement** — `kind`, `children` and `nodes`
      on one node must answer the same thing on a second call, since an inert tree holds its answer
      rather than recomputing it, and two `parse` calls over one source must agree as well.
      `crates/nvs-stdlib/src/ast.rs:300`, `crates/nvs-stdlib/src/ast.rs:308` and
      `crates/nvs-stdlib/src/ast.rs:321`.
- [ ] **A case that names the leaf edge on both sides** — a node with no children answers an empty
      list rather than `null`, and `nodes` on that same leaf answers a list of exactly one, beside
      an interior node where both are larger, so a member confusing "no children" with "not a node"
      fails. `crates/nvs-stdlib/src/ast.rs:308` and `crates/nvs-stdlib/src/ast.rs:321`.
- [ ] **A case that asserts an invariant over the whole tree by counting** — every node `nodes`
      reaches has a non-empty `kind`, and the count of nodes with children plus the count without
      equals the whole, so a tree that dropped a subtree fails on the arithmetic rather than on a
      line. `crates/nvs-stdlib/src/ast.rs:321`.

## Backlog

- `Core\Crypto`'s agreement gap: `open` twice on one sealed message, two `seal`s differing, two
  `generateKey`s differing — `crates/nvs-stdlib/src/crypto.rs:431`, `:447`, `:463`. Its refusal and
  length-sweep questions already have cases.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case —
  `crates/nvs-stdlib/src/task.rs:561`, `gaps.py`'s differential section.
- `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's.
- `orient.py`'s `[context] modules` matcher reports `fatal.rs` and `script.rs` unmatched while
  printing both.
- `Core\Csrf`, `Core\Jwt` and `Core\Totp` are ADR 0060's remaining floor classes, one file each.
