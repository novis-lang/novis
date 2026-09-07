# Handoff

## State

**Goal 14, stage 6: `completion` answers both halves of an access.** `->` reaches what a value's
class declares for an instance, `::` reaches the static members, the class constants and an enum's
cases, and `crates/nvs-lsp/src/completion.rs:133`'s `Reach` is the only thing the two lookups
differ by — one roster of node kinds (`crates/nvs-lsp/src/completion.rs:121`) carries which half
each access shape reaches, so `MEMBER_ACCESS` is gone.

**A `::` receiver names a class rather than holding one**, so there is no type to ask for.
`crates/nvs-lsp/src/completion.rs:204`'s `named_class` takes the checker's own answer first —
`nvs_types::ExprInfo::EnumCase`, a `ResolvedCall`, a `StaticProperty` keyed by the whole access —
and, for the member half still being written, which is exactly what resolved to nothing, puts the
written name through `nvs_hir::resolve_ref` with the namespace the cursor sits in and the entry
file's own imports. `rustc-hash` is a new `nvs-lsp` dependency, for that function's map parameter
and nothing else.

**`nvs lsp-test tests/lsp/` reports `55 passed, 0 failed`**, against the goal's floor of 160;
`verify.py` is 7 of 7 green. The known gaps are the module doc's: an inherited member, visibility,
and `self::`/`static::`/`parent::` whose member half is still empty.

## Next group

**Stage 6: `completion`'s last arm, the bare position** — one file set:
`crates/nvs-lsp/src/completion.rs`, with `crates/nvs-syntax/src/walk.rs` and
`crates/nvs-types/src/expr_table.rs` read only, plus new cases under `tests/lsp/completion/`.

- [ ] **What position a bare cursor is in, and the variables in scope there** —
      `rule:ide/the-request-set-is-closed`. `crates/nvs-lsp/src/completion.rs:150`'s `at` answers
      nothing when the cursor is in no access, and that is where this arm goes;
      `crates/nvs-lsp/src/completion.rs:324`'s `local_ty` already reads
      `nvs_types::ExprTypeTable::local_scopes` innermost-body-first and is the half of the work
      that exists. **The position test is the slice.** One checked fact to start from:
      `crates/nvs-syntax/src/walk.rs:364` gives a half-written `$na` its own `Variable` node, so
      "the innermost node is a `Block`" is not the test — a cursor inside a class body is inside
      the file's own script frame too, and offering it locals would be wrong.
- [ ] **Keywords, filtered by that position** — same rule, same file, on the position the item
      above settles. **The parser's dispatch is not the roster**:
      `crates/nvs-syntax/src/parser/stmt.rs:222` parses `unset`, `global`, `goto`, `trait` and
      `list`, which Novis rejects in the `E02xx` band so the diagnostic can be precise. The list to
      offer is what the language accepts, and the decision belongs in this module's doc per the
      goal's § *Standing decisions*.

## Backlog

- `nvs/redactions`, the goal's stage 8 and the driver's earliest failing acceptance check —
  `docs/agent/loop-goal.toml`'s `8 redactions` stage names its three tests.
- The `.lspt` corpus is at 55 cases against M4B's floor of 160 — `docs/plan/m4b.md` *Verify*.
- An inherited member is not offered by `completion` — `crates/nvs-lsp/src/completion.rs`'s module
  doc, and the widening is `rule:ide/five-features-are-one-reference-index`'s index.
