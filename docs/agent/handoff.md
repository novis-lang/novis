# Handoff

## State

**M4 — language completeness.** `mwl-ir`'s statement dispatch now reaches its catch-all for
**nothing the checker accepts**: ADR 0007 § 3.3's destructuring was the last shape, and it
lowers.

**`[int $a, string $b] = $pair;` is the subscripts it is spelled out of.**
`Lowering::lower_destructure` (`crates/mwl-ir/src/lower/stmt.rs:1291`) lowers the subject
once, then emits one `ArrayGet` per element — `AbsentKey::Throws`, so a missing key throws
exactly as `$pair["2"]` would — keyed by the element's own `key =>` where it writes one and
by its position in the pattern otherwise, a skipped slot included. A nested target reads
*through* the borrowed element rather than copying it, and a subject nothing else owns is
staged on the owned-temporaries stack for the whole statement, so a throw out of any element
read drops it. Positional, keyed, skipped, nested, widened (`array<int>` into `float $f`) and
a temporary subject all run; `tools/leak-check.sh` clean, exit 0.

**The read is at the element's representation, not the leaf's.** `mwl_types::locals`
records an `ExprInfo::Index` entry under each accepted leaf's own span — the same entry a
subscript gets, and the second and only other caller of `ExprTypeTable::record` — and
`Lowering::coerce` takes it from there to the declared type. That is what makes
`[float $f] = $ints;` ADR 0007 § 2's widening rather than a float-shaped `int`.

**The checker half was empty and is not any more.** `walk_destructure_target`
(`crates/mwl-types/src/locals.rs:1005`) declared bindings and asked nothing: a leaf's type,
the value's type and `&$x` were all unchecked. It now refuses a value with no element type
(`E0482`, that code's own rule verbatim), an element type the leaf does not declare
(`E0401`, `check_foreach_value`'s direction and covariance), and a by-reference leaf
(`E0483` — an aliasing element has no owner, the array literal's rule from the other side).
Both codes' doc comments in `mwl-diagnostics` now name the destructuring site, so neither
rule has a second home. No new `E`-code: the `E04xx` band is full at `E0499` and `E0501` is
already `mwl-ir`'s, so a genuinely new types diagnostic has only `E0500` left and the band
needs a decision before the one after that.

**Measured for the next group, so it is not re-derived**: a static property write
(`Box::$count = 5;`), a write through a `mixed` receiver and a shape-literal field write
(`$point->x = 9;`) *all* lower today. Whatever still reaches `lower_reassignment`'s
catch-all, it is none of those three.

## Next group

**What is left in the statement slice, all in one file.** The file set:
`crates/mwl-ir/src/lower/stmt.rs`, `crates/mwl-types/src/expr/assign.rs`,
`tests/conformance/lang/`. A scratch `.mwl` under `.agent-tmp/` reproduces each in one call.

- [ ] **An `unset` target whose base is not an array** — `crates/mwl-ir/src/lower/stmt.rs:1236`
      panics for every target but an element with an explicit subscript, so `unset($local)` is
      the shape to place. ADR 0007 § 5 and `E0413`'s doc comment own what a non-element target
      means; the answer is a diagnostic or a lowering, never the panic.
- [ ] **Prove `lower_reassignment`'s catch-all dead, or find what reaches it** —
      `crates/mwl-ir/src/lower/stmt.rs:1171`, the arm itself, with `lower_reassignment` at
      `:666` and `mwl_types`' `check_write_target` in `crates/mwl-types/src/expr/assign.rs`
      naming every target the checker accepts. The three obvious candidates are ruled out
      above; if the list is exhausted, the arm says so instead of listing three shapes.
- [ ] **Goal item 7 — `$x++` / `--$x` in both positions** (`mwl-ir` gap 16), the same file:
      `lower_incdec` at `:435` and `is_reevaluable_target` at `:1261`. The largest of the
      three and the one with a written plan in `docs/agent/loop-goal.md`.

## Backlog

- A pattern mixing `k =>` and positional elements is accepted; PHP refuses one outright. The
  rule this gives it (a keyed element still occupies its position) is in
  `Lowering::lower_destructure`'s doc comment — `docs/spec/00-overview.md` § 3 is where it
  belongs if it ever needs to be user-visible.
- 14 of 32 named `.mwlt` cases still unwritten — `python tools/holes.py --cases`.
- `orient.py` printed no map line for `crates/mwl-types/src/expr/assign.rs` or
  `crates/mwl-types/src/expr_table.rs`, both of which any item about a *checker* refusal
  needs; `[context] modules` wants patterns for them, alongside the `check.rs` one the last
  session asked for.
