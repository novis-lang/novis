# Handoff

## State

**ADR 0007 § 2's boundaries and the three tag-decided families' agreement are both
pinned**, which is two corpus cases over already-landed work and takes `python
tools/holes.py --cases` from 9 named cases down to 7. The plan's `Open now` carries what
each asserts; nothing in `crates/` changed this session.

- **`tests/conformance/lang/every-remaining-conversion-row-runs-or-throws.nvst`** names
  the last accepted and first refused value of every `int`/`uint`/`float` row, the
  operand arriving as a parameter so that ADR 0047 § 6 does not refuse it a phase early.
- **`tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.nvst`**
  asks one `mixed` value an operator, a condition and a subscript, and counts its
  agreements with the declared-type twin rather than reading a line off each.
- **`require` does not run the required file's own top-level statements** — that is
  `nvs-ir`'s known gap 22 and `crates/nvs-ir/src/lower/stmt.rs:347` lowers
  `ExprKind::Require` to nothing at all, while the declarations it carries do resolve
  (a class in the required file is callable from the entry file). So the named case
  `a-required-file-runs-its-own-top-level-statements.nvst` is a **hole**, not a case.
- **`orient.py`'s pack was complete for this item.** The two standing manifest gaps are
  unchanged — `[context] modules` has no `nvs-runtime` and no `nvs-diagnostics` entry.

## Next group

**The two file-scope statement shapes `holes.py --cases` still names, over the statement
lowering that owns both.** The files: `crates/nvs-ir/src/lower/stmt.rs` and
`tests/conformance/lang/`. The first is a case over landed work and the second is the
gap behind it, so they belong in one session.

- [ ] **`tests/conformance/lang/inline-html-at-file-scope-is-echoed-in-place.nvst`** —
      ADR 0021, the raw-span echo. Verified working on a scratch this session: text
      outside `?>` … `<?nvs` prints in place, in order, between the statements around it.
      `crates/nvs-ir/src/lower/stmt.rs:239` (`lower_inline_html`) and
      `crates/nvs-ir/src/lower/expr.rs:464` are the anchors.
- [ ] **`require` runs the required file's own top-level statements** — `nvs-ir` gap 22,
      `crates/nvs-ir/src/lower/stmt.rs:347`, with the two sites that already say so in
      `crates/nvs-types/src/expr/mod.rs:738` and `:750` and the `E07xx` code at
      `crates/nvs-diagnostics/src/lib.rs:1213`. Close it, then write the named case; if
      the design is bigger than one slice, pin the divergence and say so in ADR 0021.
- [ ] **`python tools/holes.py --cases`** once both land, to pick the next pair.

## Backlog

- `PropertyObserver` may not be a reserved interface at all yet —
  `crates/nvs-hir/src/interfaces.rs:106` asserts it is *not* one — so check before
  taking `class/a-property-observer-sees-every-write-its-class-makes.nvst` (ADR 0014).
- The other named cases still owed: a delegated interface (ADR 0043), an attribute
  retrieved by its own type (ADR 0046), a dump that redacts a secret (ADR 0033), and the
  test-attribute table (`python tools/holes.py --cases` is the list).
- `[context] modules` in `docs/agent/loop-goal.toml` has no `nvs-runtime` and no
  `nvs-diagnostics` entry; both were wanted by earlier sessions.
