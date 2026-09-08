# Handoff

## State

**Goal 18. Stage 5's group is closed — all three items landed and verified.**

A value that does not fit a shape now names the key at fault. `crates/nvs-types/src/expr/assign.rs`'s
`mismatch` computes it, so **every** position that assigns into a shape gets it — a `return`, an
attribute payload, an argument, a `foreach` binding — rather than the one arm a caller-side help would
have reached. It stays `E0401`: the mistake is unchanged, only the explanation was missing, and a code
of its own would have fired on one arm and said nothing the help line does not. **`E0811` is still
free.** The naming test is `shape_satisfied`'s own, not a weaker one written for the message, so a
source field that is itself optional is named as unsupplied exactly where the relation refuses it.

**A parenthesised group reported one mismatch twice, a column apart, and now reports it once.**
`infer`'s `Paren` arm at `crates/nvs-types/src/expr/mod.rs:885` re-enters `check_expr` with the same
`expected`, so the inner expression already had the whole of that function applied at its own narrower
span; the enclosing `check_expr` was repeating it. The surviving report is the inner one. Asked of a
scalar as well as a shape, because the fix is `check_expr`'s and not the shape arm's.

**Stage 2's acceptance check is red for missing tests, not missing behaviour — this is the finding the
next group turns on.** The optional marker and the shape qualifier are both landed and exercised:
`{x?: int}` parses, interns apart from `{x: ?int}` and relaxes presence only, and `tainted {…}`
distributes over every text field and is refused over a shape carrying none. What no crate holds is a
test under any of the nine names the check lists. Nothing needs designing there.

## Next group

**Stage 2: the grammar — the nine names its two checks list, over landed behaviour** — one file set:
`crates/nvs-syntax/src/parser/tests/ty.rs`, `crates/nvs-types/tests/objects_and_shapes.rs`,
`crates/nvs-types/tests/tainted.rs`. `rule:types/shape-type` and `rule:security/tainted-qualifier` are
the specification; both are already implemented, so every item here is a test and none is a design call.

- [ ] **The four `nvs-syntax` names** — `crates/nvs-syntax/src/parser/tests/ty.rs:417`, beside
      `tainted_qualifies_string_and_bytes`, which is the shape these copy. Nothing in that crate names
      a shape test today. The grammar they ask about is `parse_shape_type` at
      `crates/nvs-syntax/src/parser/ty.rs:712` (the `name?:` marker) and
      `crates/nvs-syntax/src/parser/ty.rs:334` (`tainted` distributing over a shape's text fields, and
      refusing a shape that carries none). `rule:types/shape-type`, `rule:security/tainted-qualifier`.
- [ ] **The five `nvs-types` names, three of which are existing tests under other spellings** —
      `crates/nvs-types/tests/objects_and_shapes.rs:92` is
      `a_source_with_extra_fields_still_satisfies_the_shape`, `:100` is
      `a_source_missing_a_required_field_does_not`, and `:132` is
      `a_source_missing_an_optional_field_satisfies_the_shape`. **Rename rather than duplicate, and
      check no other `[[check]]` in `docs/agent/loop-goal.toml` names the old spelling first** — that
      is exactly how `the_pipeline_codes_…` broke stage 0's check two sessions ago.
- [ ] **The tainted pair** — `crates/nvs-types/tests/tainted.rs:420` is
      `tainted_over_a_shape_carrying_no_text_is_refused`, which is
      `a_tainted_shape_naming_no_text_field_is_refused` under another name. Its sibling
      `a_tainted_shape_is_not_assignable_to_the_same_shape_unqualified` has no test at all; the
      relation that should already refuse it is the qualifier axis in `crates/nvs-types/src/expr/quals.rs`,
      reached from `crates/nvs-types/src/expr/assign.rs:114` — verify it refuses before writing the name.

## Backlog

- Stage 5's remaining clauses, once stage 2 is green — `docs/agent/loop-goal.md` § *Stage 5*.
- `E0811` is unspent and the `E08xx` band's next free number; `crates/nvs-diagnostics/src/lib.rs`.
- Per-file known gaps stay in each crate's module doc, never the plan — `AGENTS.md` § *Keep each slice small*.
- Carried across a goal switch: `docs/agent/carried-gaps.md`.
