# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **866 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **17 unresolved of 135 named guard tests**, down from 39 of 149. Stages 5 and
6 were carried through the same three-way triage over one file set — `docs/agent/guard-name-debt.md`
and `docs/agent/loop-goal.toml` — resolving 22 of the 26 they held between them. Stage 5's two whole
`[[check]]` blocks (`nvs-types (the declared features)`, `nvs-ir (the declared features)`) are
**gone**: every one of their ten names was cause 2, so the ten `.nvst` cases that own them are named
in the `nvs-suite` `cases` list instead. Ten cases were added to that list across the two stages, each
run green first.

What is left in the two stages is cause 3 and honest, four lines with a reason each:
`a_fatal_releases_the_frames_locals` (nothing asserts what a `FATAL` does to a frame's locals, and
valgrind skips `examples/fatal.nvs` by design), `a_non_void_function_must_return_on_every_path` (the
accepting half is two cases now in the list; no diagnostic anywhere names a body that falls off its
end), `an_implicit_constructor_is_held_to_zero_arguments`, and the two nvs-syntax parse shapes
(`use A\{B, C};` and a keyword-named enum case). The debt file records the cause per line.

The debt file's own header sentence is the running count and is the thing to keep true.

## Next group

**Stage 7's remaining names, then the two Stage 4 lines still undecided.** Same shared file set,
unchanged for a third session: `docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml`, with
`grep -rhoE "fn [a-z_]+" crates/<crate>/src crates/<crate>/tests` and
`ls tests/conformance/*/ | grep -iE "<topic>"` as the two probes (the new playbook bullet is the
shape), and `python tools/loop.py --list` as the toml's parse check.

- [ ] **Stage 7's names**, at `docs/agent/guard-name-debt.md:220` onward — the testing surface
      (ADR 0079). Expect cause 3 to dominate: `loop-goal.toml`'s own comment on
      `a-test-attribute-builds-a-table-the-runner-reports.nvst` says none of that surface exists yet,
      so a name there is owed rather than misnamed and should stay unticked with the reason.
- [ ] **The one Stage 4 line still undecided**, `the_object_top_type_erases_to_the_pointer_a_class_
      does` — ADR 0036 § 4's erasure. Check `crates/nvs-ir/src/lower/expr.rs` for an
      `ExprInfo::ShapeProperty` test and `tests/conformance/lang/a-shape-read-through-a-widened-view-
      is-name-keyed.nvst` before reaching for a rename.
- [ ] **Re-derive the header count once the stages are done**, by parsing `loop-goal.toml` and
      summing `len(c["tests"])` — 135 today. The header sentence is the only running total and
      a stage that moves entries between `tests` and `cases` moves the denominator.

## Backlog

- A body that falls off its end has no diagnostic — `nvs-diagnostics` next free in the `E07xx` band
  is `E0739` (`docs/agent/loop-goal.toml`, item 38).
- `array<T> as array<U>` still panics `nvs-ir` at `lower/expr.rs:877`; three guard names wait on it.
- `Class::method(...)` first-class callable panics `nvs-ir` — `nvs-ir` gap 1, playbook § Writing a
  test case.
- Grouped `use A\{B, C};` parses no path in `parser/decl.rs:240` — ADR 0021's neighbourhood.
