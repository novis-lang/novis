# Handoff

## State

**Goal 61 — a shape a class owns is named inside it — has just started; nothing of it has landed yet.** Goal `unowned-closures`'s whole list is this goal's Stage 1 floor.

The design is settled in the goal's *Standing decisions* and needs no record before the code lands: a
`type` alias becomes a member of a class, an interface or an enum body, with no modifier, reached as
`Owner::Name` from anywhere and as a bare `Name` inside the owner's own body, never through a subclass
or an implementor. It is the same transparent alias the file-scope form declares, keyed in the alias
table by its owner rather than by a namespace path. The one record this goal opens is written in stage
5, after the code and its tests exist, and claims whatever number is free then.

## Next group

**Stage 2: the member, its key, and the two spellings** — one file set:
`crates/nvs-syntax/src/parser/decl.rs`, `crates/nvs-syntax/src/ast.rs`, `crates/nvs-hir/src/aliases.rs`,
`crates/nvs-hir/src/resolve.rs`, `crates/nvs-types/src/lower.rs`.

- [ ] **The member** — `crates/nvs-syntax/src/ast.rs:@ClassMemberKind` gains `TypeAlias(TypeAliasDecl)`;
      `crates/nvs-syntax/src/parser/decl.rs:@parse_class_member_with_attrs` dispatches on the contextual
      `type` keyword beside `const` and `function` into `@parse_type_alias_decl`. A class, an interface
      and an enum body all accept it; the parser test round-trips all three.
- [ ] **The key** — `crates/nvs-hir/src/aliases.rs:@collect_in` walks class bodies and records the
      member under the owner's `QName` plus the member name; `crates/nvs-hir/src/resolve.rs:@declare`
      declares it and `@check_alias_is_not_a_bare_class` runs on it. The `nvs-hir` tests pin the key,
      the cycle and the bare-class refusal.
- [ ] **The two spellings** — `crates/nvs-types/src/lower.rs:@lower_member_type` tries the owner's
      alias before the enum case and the constant fold, and resolves the name left of `::` through the
      alias table first (closing the known gap its doc comment records);
      `@resolve_name_type` looks a bare name up against `Ctx::current_class`'s aliases before the
      namespace's. The `nvs-types` test and the four stage 2 conformance cases prove it.
- [ ] **Stage 0's two sentences** — rewrite `lower_member_type`'s doc comment as a whole, and leave
      `docs/rules/types/type-alias.md:1-2` to stage 5's record, noting in this handoff that the code is
      ahead of the fragment until then.

## Backlog

- **Stage 3, the refusals** — the stage 2 file set plus `crates/nvs-diagnostics/src/lib.rs`: one new
  code for a modifier before `type` in a body; `E_DUPLICATE_DECLARATION` for an alias sharing a name
  with a constant or a case; `E_UNKNOWN_MEMBER` with a help naming the declaring owner for
  `Child::Meta`; a by-name refusal for `type` inside a body. Five `reject` cases. Cheap to take right
  after stage 2, since the files are already loaded.
- **Stage 4, the tooling** — `crates/nvs-fmt/src/lib.rs`, `crates/nvs-cli/src/meta.rs`, and the four
  `crates/nvs-lsp/src/` modules. Shares nothing with stages 2–3; its own session.
- **Stage 5, the rule and the record** — `docs/rules/types.json`, `docs/rules/types/`,
  `docs/decisions/`: one new record, the `types/class-scoped-alias` fragment, the amended
  `types/type-alias` fragment, `python tools/rules.py --render`. Its own session.
- When this goal's last check goes green the driver takes goal `gap-zero`.
