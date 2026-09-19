# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Nine are complete: the six that were, plus `nullable-union-literal-and-enum-case-types`,
`void-never-self-static` and `type-aliases`. Nine still owe;
`python tools/dossier.py --owed --group lang:types` is the list. Nothing is blocked.

`void-never-self-static`'s proof found a checker hole and it is fixed in the same group: `void` and
`never` were accepted as a binding's declared type, and a `var` took its type from a call that hands
nothing back, which left the name standing for nothing and made `nvs-ir` report an operand used
before it is defined. `nvs_types::locals`' `reject_void_or_never_binding` refuses all three now,
under `E0742`, whose constant is `E_VOID_OR_NEVER_OUTSIDE_RETURN` rather than `..._PARAMETER`.

Two sentences in the chapter were stale against pinned behaviour and are rewritten: a `never` method
does run (`tests/conformance/core/a-never-method-is-a-terminator.nvst` has pinned that all along),
and a `type` alias *is* a member of a class, interface or enum body (`rule:types/class-scoped-alias`).

`target/release/nvs.exe` was rebuilt this session, because the checker changed and `dossier.py`
prefers it.

`docs/agent/loop-goal.toml`'s `[context] rules` and its copy at
`docs/agent/goals/dossier/80-lang-types.toml` now carry the next group's chapter rules; keep swapping
them per group rather than naming the chapter's whole `types/` set, which is 47 fragments.

## Next group

**Stage 2: the dossier** — one file set: `docs/reference/lang/20-types.md` and the four proof trees
under `docs/examples/lang/types/`, `tests/hostile/lang/types/`, `benches/members/lang/types/` and
`tests/conformance/`. One slice is one feature with all five artefacts.

- [ ] **`lang:types/literals`** — owes all five. `rule:types/literal-types` and
      `rule:types/duration-literal` specify it. `docs/reference/lang/20-types.md:407`
- [ ] **`lang:types/widening-without-as`** — owes all five. `rule:types/implicit-widening`
      specifies it. `docs/reference/lang/20-types.md:486`
- [ ] **`lang:types/the-conversion-operator-as`** — owes all five. `rule:types/conversion`,
      `rule:expressions/nullable-conversion-availability` and
      `rule:expressions/conversion-keeps-qualifiers` specify it.
      `docs/reference/lang/20-types.md:527`

## Backlog

- `E0233`'s help still says "move it out to file scope" now that a class body takes a `type` too —
  `crates/nvs-types/src/locals.rs:1448`.
- `E0307` refuses an alias that names another alias while naming class kinds — the rule's fragment
  `docs/rules/types/alias-is-never-a-bare-class.md` lists only class-shaped atoms.
- Nine `lang:types` features still owe every artefact; `python tools/dossier.py --owed --group
  lang:types` is the list, and `docs/implementation-plan.md` § *Open now* owns the ordering.
