# Handoff

## State

**Goal 11 is whole. `///` is documentation end to end, and the last two acceptance checks are green.**
Every stage's check passes: the trivia layer, attachment, the two tags, `nvs meta --json`'s program
half, `nvs doc`, `nvs check --strict-docs`, `examples/doc-comments.nvs`, and now the conformance
suite. `python tools/verify.py` is green and the conformance corpus is 1580 cases.

- **The corpus reclassified with no edit, and a test says so** —
  `crates/nvs-syntax/tests/trivia.rs:158` walks `examples/` and `tests/` for the `///` an opening run,
  and asserts no file reports `E0127` or `E0128`. It is a sweep with a floor of seven rather than a
  list of paths, so a case added later is covered the day it lands.
- **`tests/conformance/syntax/doc-comment/` holds six cases** — one green, and one per refusal
  (`E0127`, `E0128`, `E0323`, `E0324`, `E0325`). `E0326` is not among them because `nvs test` runs a
  case with no flags; `crates/nvs-cli/tests/strict_docs.rs` pins it instead.
- Nothing is blocked. The next goal in `docs/agent/goals/chain.toml` is **12 resilient-tree**, and it
  installs its own handoff, so the group below is what that goal opens on rather than leftover work.

## Next group

**`rule:ide/one-grammar-one-tree`'s remaining half, in `nvs-syntax`'s own two files** — the tree
`Parsed` already carries trivia for, plus the recovery a consumer must not have to infer.

- [ ] **`SyntaxIndex` joins `Parsed` and answers `at(offset)`** — `rule:ide/the-index-answers-the-cursor`,
      built by one walk and rebuilt per analysis. `crates/nvs-syntax/src/parser/mod.rs:750` is the
      struct the field is added to; goal 11 deliberately did not build it, since nothing read it then.
- [ ] **Recovery says so rather than being inferable** — `rule:ide/recovery-is-explicit`:
      `MemberName::Missing(Span)` beside `MemberName::Ident` at `crates/nvs-syntax/src/ast.rs:386`, and
      the span of what it stood in for on `ExprKind::Error`. An empty span is a coincidence, not a
      contract, and completion's whole behaviour hangs on telling the two apart.

## Backlog

- Widening `--strict-docs` past a member to a class, interface, enum or `type` alias — the publisher's
  call, and `crates/nvs-hir/src/members.rs`'s module doc is that boundary's home.
- The `visibility` key on a program member has one renderer, `nvs doc`; `tools/reference.py` and the
  website still read the registry half only (`crates/nvs-cli/src/meta.rs` § *The program half*).
- `collect`/`case_source` now exist in both `tests/trivia.rs` and `tests/lossless.rs`; a third copy
  is the point at which they want a shared module.
