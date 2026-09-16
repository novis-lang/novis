# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 41`,
`untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, `retired-owner: 0` and `past-milestone: 8`; stage 6
wants the first of those at 0, and the stage's other check — `python tools/owners.py --deferrals` — is
green. Nothing is blocked.

**A cursor on a declaration's own name now resolves to the symbol it declares.**
`nvs_lsp::index::symbol_at` (`crates/nvs-lsp/src/index.rs:502`) asks `declared_at` before
`crate::definition::named_at`: the declaration side reads the type from `nvs_hir::SymbolTable` and the
member from the declaration's own node, in the order and from the sources `declarations` already reads
them, so a name it answers is a name the index is keyed on. The member half of both is one list,
`member_names`, and `member_symbol` is the one place `C::$x` is spelled. `crate::server`'s own
index-side `declared_at` is gone: the type hierarchy and `implementation` ask `symbol_at` once, like the
other three features. The cursor's file is the analysis entry, which is the only file a cursor is in
(`Analysed::index`), and `covers` includes the name's last byte because that is where a double-click
leaves the caret.

`tests/lsp/` moved with it: twelve cases that froze the empty answer are renamed to the answer they now
freeze, and four that still answer nothing say why in the terms that are now true. A `.lspt`
expectation is never edited to make a case pass (`tests/lsp/README.md`), so a case whose claim changed
is a new case under the name of its new claim.

## Next group

**Stage 6: the reference index's three remaining occurrence gaps** — one file set:
`crates/nvs-lsp/src/index.rs`, `crates/nvs-lsp/src/definition.rs` for the second item, and the `.lspt`
cases under `tests/lsp/references/` and `tests/lsp/highlight/`.

- [ ] **A name in an `extends` or `implements` clause is an occurrence** —
      `crates/nvs-lsp/src/index.rs:74`, `rule:ide/five-features-are-one-reference-index`. The written
      spans are on the declaration's own node (`ClassDecl::extends`, `ClassDecl::implements`,
      `InterfaceDecl::extends`, each entry a `Name` with its own span,
      `crates/nvs-syntax/src/ast.rs:1518`); the resolved names are `nvs_hir::ClassLinks::extends` and
      `::implements` off `analysed.module.graph` (`crates/nvs-hir/src/hierarchy.rs:60`). Pair the two
      **by position and only when the lengths match** — `crates/nvs-hir/src/hierarchy.rs:266` drops a
      name that did not resolve, and a clause paired off by one would record a use at the wrong name.
      The hook is a second pass in `occurrences` (`crates/nvs-lsp/src/index.rs:784`), per symbol
      declared in the file, reaching the node through `declared_type` the way `declarations` does.
      `tests/lsp/references/an-interface-declarations-own-name-answers-itself-alone.lspt` and
      `tests/lsp/highlight/an-interface-declarations-own-name-highlights-nothing.lspt` both move with
      it: the clause in each is at `case.nvs:7:23`.
- [ ] **Reading a class constant is an occurrence of it** — `crates/nvs-lsp/src/index.rs:68`, same
      rule. `crate::definition::Target` (`crates/nvs-lsp/src/definition.rs:187`) names a type, a method
      and a property and has no constant among them, so the variant goes there, with its arm in
      `target_of` (`crates/nvs-lsp/src/definition.rs:380`) and its spelling in `symbol_of`
      (`crates/nvs-lsp/src/index.rs:810`) — the declaration side already spells it `Cart::LIMIT`.
      `tests/lsp/references/a-class-constant-read-answers-nothing.lspt` and
      `.../a-class-constant-declaration-answers-itself-alone.lspt` move with it.
- [ ] **An enum case occurrence is recorded against the case** — `crates/nvs-lsp/src/index.rs:61`,
      same rule. This one is a change in the checker's table rather than in the walk: the read resolves
      to `nvs_types::ExprInfo::EnumCase`, whose `enum_` is what both this index and `definition` answer.
      `tests/lsp/references/an-enum-case-declaration-answers-itself-alone.lspt`,
      `.../an-enum-case-read-answers-the-enum-it-was-recorded-against.lspt` and their two `highlight`
      twins move with it.

## Backlog

- The other 41 `unowned` module-doc gaps stage 6 still wants at 0 — `python tools/owners.py` lists
  each with its file and number.
- `crates/nvs-hir/src/hierarchy.rs:38`'s own unowned gap (a `Core` link target is trusted to exist) is
  in this goal and carries a `Decided:` sentence already.
- What a shipped feature still owes across a goal switch is `docs/agent/carried-gaps.md`, not here.
