# Handoff

## State

**Goal `m4-refusals` — Stage 3's six coded sites are guarded; its seventh, the statement roster, is
open on a decision named below.** `python tools/holes.py --guarded` lists `E0439`, `E0723`, `E0443`,
`E0497`, `E0700`, `E0234` in that order, which is Stage 3's first acceptance check, and the stage's
`.nvst` check passes.

- `python tools/holes.py`: **10** refusal sites, `UNATTRIBUTED: 0`, **6** guarded.
  `crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **10** to match.
- Each of the six was probed under `.agent-tmp/` before conversion and is genuinely refused by the
  front end: a `?array<T>` subject and an `array<string>|null` spelling both answer `E0443`, an `int`
  key binding `E0723`, and an array element, a static property, a call's result and a literal all
  answer `E0439` at an `inout` argument.
- Two new reject cases carry the codes that had none:
  `tests/conformance/reject/a-foreach-over-a-nullable-array-is-refused-at-the-subject.nvst` and
  `tests/conformance/reject/an-inout-argument-names-storage-that-outlives-the-call.nvst`.
  `E0497`, `E0234`, `E0723` and `E0700` were already expected by cases on disk.
- **`StmtKind` is `#[non_exhaustive]`** (`crates/nvs-syntax/src/ast.rs:1271`), so Stage 3's last bullet
  — "a new `StmtKind` then fails to compile here" — is unreachable from `nvs-ir` as written. The
  playbook bullet holds the two ways out. Nothing else is blocked.
- `E0101` is the one code the roster's arms would name that **no** conformance case expects yet
  (`grep -rl 'error\[E0101\]' tests/conformance/` is empty), so that arm owes a case before
  `refusals.rs`'s guard gate will take it.
- Line numbers in `docs/agent/loop-goal.md` § *Stage 3*'s table are stale for every row now, the
  guards being longer than the panics they replaced. The anchors below are what `holes.py --sites`
  reports now.

## Next group

**Stage 3: the statement roster, the last site** — one file set:
`crates/nvs-ir/src/lower/stmt.rs`, `crates/nvs-syntax/src/ast.rs`, plus one reject case under
`tests/conformance/reject/`. Then Stage 4, which is one site in a file this group already opens.

- [ ] **The `E0101` reject case, first** — a `var $x;` with no initializer, anchored at the roster
      comment that names it, `crates/nvs-ir/src/lower/stmt.rs:249`. `refusals.rs`'s
      `every_guarded_site_names_a_code_a_conformance_case_expects` refuses a guard naming a code no
      case expects, so writing this before the arm keeps the tree green at each commit.
- [ ] **The statement roster's arms** — `crates/nvs-ir/src/lower/stmt.rs:263`, whose `:240-262`
      comment is the roster. Spell each shape it lists as its own `guarded_by!` arm naming its own
      code (`E0101`, `E0215`/`E0216`, `E0233`, and the parser's codes for `global`, `goto`, a
      function-scope `static`). The wildcard **stays**: `crates/nvs-syntax/src/ast.rs:1271` makes
      `StmtKind` `#[non_exhaustive]`, so reword it as the engine invariant it then is — `holes.py`'s
      `ENGINE` regex takes "this is a bug" — rather than leaving a `REFUSAL` the count still charges
      for. Taking the attribute off `StmtKind` instead is the alternative, and it is a change to
      `nvs-syntax`'s surface that this goal's § *Standing decisions* does not pre-authorize.
- [ ] **Stage 4 — `$f(...)` through a callable** — `crates/nvs-ir/src/lower/call.rs:793`
      (`lower_closure_call`). `rule:types/callable-is-a-closure` says a `callable` holds only a
      closure, so the shape lowers to that closure retained once and makes no call; the differential
      case pinning `$f(...) === $f` is what decides it if PHP disagrees.

## Backlog

- `docs/agent/loop-goal.md` § *Stage 3*'s table carries stale `file:line` anchors for all seven rows;
  `python tools/holes.py --sites` is the live list — that file owns the rows.
- Stage 5 onward is untouched; `docs/agent/loop-goal.md` § *Stage 5* owns what it needs.
