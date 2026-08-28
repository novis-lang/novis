# Handoff

## State

**M4's frontier is ADR 0043 § 4, whose bullets 1, 3 and 4 all run.** A promoted
constructor parameter is an ordinary property in every table that answers the
question, so § 4's own parenthetical holds and a delegate field may be one.

- **The mechanism has one home each and is not restated here**: which parameters
  promote is `nvs_syntax::ast::Param::is_promoted`
  (`crates/nvs-syntax/src/ast.rs:505`), the type and visibility are
  `record_promoted_properties` (`crates/nvs-types/src/signatures.rs:857`), the
  slot is `own_properties` (`crates/nvs-types/src/layout.rs:283`), and the store
  is `nvs_ir::lower::promoted_stores` (`crates/nvs-ir/src/lower/mod.rs:880`).
- **§ 4 bullet 2 is the open one, and it is bigger than it reads.** ADR 0022 § 3's
  "never written" storage state does not exist anywhere in the tree, so there is
  no checked throw for the forward to reuse: `lateinit Tracker $t;` read before
  its first write hands back a null receiver today, and under a delegation clause
  the forward dispatches on that and overflows the stack — eight lines reproduce
  it, a `lateinit` delegate field and one call.
- **`orient.py`'s pack was complete for this item.** `[context] modules` still has
  no `nvs-diagnostics` entry, and the pack prints no map line for `nvs-types`'
  `signatures`/`layout` or for `nvs-hir`, all three of which this session edited.
- **`docs/` said the WSL mount is `/mnt/d/nvs`** and it is `/mnt/<drive>/<repo>`; the four
  files that said so, `commands.md`'s leak-check line among them, are corrected.

## Next group

**ADR 0022 § 3's never-written property state — the one storage fact ADR 0038 and
ADR 0043 § 4 bullet 2 both wait on.** The file set is
`crates/nvs-runtime/src/object.rs`, `crates/nvs-ir/src/lower/expr.rs` and
`crates/nvs-ir/src/lower/call.rs`.

- [ ] **A slot that was never written reads as a throw**, ADR 0022 § 3: the tag is
      free by that section's own paragraph — one more discriminant on the existing
      value representation — and the read is `crates/nvs-ir/src/lower/expr.rs`'s
      `InstKind::FieldGet` site. Only a `lateinit` property can reach it, every
      other non-nullable one being discharged by ADR 0022 § 2, so the check is
      owed on that declaration alone.
- [ ] **A delegate reached before `$field` is written throws it**, ADR 0043 § 4
      bullet 2, and stops overflowing the stack. The read is
      `crates/nvs-ir/src/lower/call.rs:911`'s `delegation_forward`, which retains
      the slot and dispatches on it with no fallback.
- [ ] **The two `.nvst` cases**, one per bullet above, beside
      `tests/conformance/class/a-delegate-field-must-be-able-to-answer-the-interface.nvst`.

## Backlog

- A visibility keyword on a non-constructor parameter declares nothing and is
  silently ignored; PHP refuses it — `nvs_syntax::ast::Param::is_promoted`.
- `null` is refused as a parameter default (`E0451`), so `public ?string $t = null`
  does not compile — `crates/nvs-types/src/defaults.rs`.
- A `require` whose path is not a string literal runs nothing, silently —
  `nvs_hir::requires`' own known gap.
- `python tools/holes.py` is the rest of M4's worklist, `--item N` for one in full.
