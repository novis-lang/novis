# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** The floor check `no module-doc gap names a goal
that walked without closing it` is green: `python tools/owners.py` reports `retired-owner: 0`, since
`nvs-render`'s `#[Test]`-result producer (`crates/nvs-render/src/lib.rs:39`) is M10's — the milestone
that makes the identical move for a compiler diagnostic and lands the surfaces that read a record.
The tool's other counts are `unowned: 42`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, and
`past-milestone: 8`; stage 6 wants the first of those at 0.

**The two definite-assignment scanners now share one descent and nothing else.**
`nvs_syntax::visit::each_child_expr` (`crates/nvs-syntax/src/visit.rs`) is the one match over the
productions `ExprKind` holds, written where the enum is because it is `#[non_exhaustive]` and a
wildcard arm in another crate would walk a new form as a leaf. It yields what an expression
*evaluates*: a closure's body and an anonymous class's members are not children of the expression
that writes them. `crate::ctor_init` joins the forms whose operands do not all run
(`scan_branches`) by intersection, so `$flag ? ($this->count = 1) : 0` no longer counts; `crate::lateinit`
threads one set straight through them, because a write seen on any branch suppressing a later read
is the silence `rule:classes/lateinit-read-before-write` asks for. Both module docs state the
closure bound as prose rather than a gap. Nothing is blocked.

## Next group

**Stage 6: the reference index's four occurrence gaps** — one file set:
`crates/nvs-lsp/src/index.rs`, and `crates/nvs-lsp/src/definition.rs` for the second item, which is
where the missing variant goes.

- [ ] **A cursor on a declaration's own name resolves to the symbol it declares** —
      `crates/nvs-lsp/src/index.rs:81`, `rule:ide/five-features-are-one-reference-index`. `symbol_at`
      reads a name off a recorded expression and a declaration is not one, so
      `textDocument/references` answers empty exactly where a reader asks it; `tests/lsp/references/`
      freezes that empty answer at a class, an interface, an enum, a method and a property, and each
      of those fixtures moves with the fix.
- [ ] **A name in an `extends` or `implements` clause is an occurrence** —
      `crates/nvs-lsp/src/index.rs:74`, same rule. A use is read off `Analysed::exprs` and a clause is
      not an expression, so an interface every class implements counts none; `SymbolIndex::subtypes`
      already reads the resolved graph, which is the shape the count above the name should take too.
- [ ] **Reading a class constant is an occurrence of it** — `crates/nvs-lsp/src/index.rs:68`, same
      rule. `crate::definition::Target` names a type, a method and a property and no constant, so
      `self::GREETING` resolves to nothing this walk can record — the variant is that module's to add
      before the index can record one.
- [ ] **An enum case occurrence is recorded against the case** — `crates/nvs-lsp/src/index.rs:61`,
      same rule. Take this one last and only if the first three left room: the checker resolves the
      site to the enum (`nvs_types::ExprInfo::EnumCase`) and `definition` answers the same way, so
      closing it is a change in the checker's table rather than in this walk.

## Backlog

- The `nvs-ir` lowering gaps are the largest unowned block left — `crates/nvs-ir/src/lib.rs` gaps 2–20,
  each its own decision; that crate's own module doc holds them.
- `crates/nvs-cli/src/openapi.rs` gaps 1–5 are one unowned block over one file, and the REST-client
  note (`docs/agent/carried-gaps.md`) says OpenAPI is skipped on purpose — that reason may already
  settle all five.
- `crates/nvs-types/src/signatures.rs` gaps 1–2 and `locals.rs` gap 1 sit in files this session did
  not open but the same pass owns.
- `python tools/owners.py`'s `past-milestone: 8` is a separate finding from `unowned`, and no check
  names it yet.
