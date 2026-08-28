# Handoff

## State

**M4's frontier past ADR 0021 entirely.** Gap 22 is closed at both halves and deleted from
`nvs-ir`'s known-gaps list; `holes.py --cases` names five Stage 8 cases and no `require` shape.

- **`require` used for its value lowers.** The mechanism has one home and is not restated here:
  `nvs_ir::lower::Lowering::lower_expr`'s `ExprKind::Require` arm (why it is the statement form's
  own call with the result kept), `nvs_ir::lower::lower_script` (why the fall-through seal hands
  back a tagged `1` rather than nothing), and `nvs_types::expr::infer`'s arm (why `mixed` is the
  whole answer). `E0704` is retired at `crates/nvs-diagnostics/src/lib.rs`, where a comment holds
  the number so it is never reused.
- **A file-scope `return expr;` needed no code at all.** It reaches `lower_stmt`'s ordinary
  `StmtKind::Return` arm through `lower_script_stmts`, against the `Ty::Tagged` `lower_script`
  declares — the frame that landed last session carried it. The slice is the seal and the cases.
- **A dynamic `require` runs nothing, silently, in both forms.** `nvs_hir::requires`' own known gap
  recognises only a plain string literal, so a non-literal path records no target, the statement
  form emits nothing and the value form answers § 3's `1`. Backlogged: closing it is a
  constant-folding or a run-time-resolution question, not a lowering one, and it is not this
  milestone's.
- **`orient.py`'s pack was complete for this item.** The two standing manifest gaps are unchanged —
  `[context] modules` has no `nvs-runtime` and no `nvs-diagnostics` entry.

## Next group

**Stage 8's three declaration-shaped named cases, from `python tools/holes.py --cases`.** No Rust
changes: the file set is `tests/conformance/class/` and `tests/conformance/lang/`, and each slice is
one `.nvst` written against the ADR § *Verification* that names it. Author each against a scratch
`.agent-tmp/*.nvs` under `nvs run` first, then `./target/debug/nvs.exe test <path>` for the one file.

- [ ] **`tests/conformance/class/a-property-observer-sees-every-write-its-class-makes.nvst`** —
      ADR 0014 § 2 (`docs/adr/0014-property-observer.md:90`) declares the reserved interface and
      § *Verification* names the case. A hook runs first, then the declared observer; an undeclared
      property is `E0405` on every class kind and is not this case's subject.
- [ ] **`tests/conformance/class/a-delegated-interface-forwards-to-the-object-it-names.nvst`** —
      ADR 0043's `implements Interface by $field;`, which is what replaces a `trait`'s shared state.
      Same file set as the slice above and the same shapes (a class, an interface with a body).
- [ ] **`tests/conformance/lang/an-attribute-is-retrieved-by-its-own-type.nvst`** — ADR 0046's
      `#[...]` shape-literal metadata, never a declared attribute class. The parser side is
      `crates/nvs-syntax/src/parser/decl.rs:53` and `parser/expr.rs:1555`.

## Backlog

- A dynamic `require` path runs nothing in either form — `nvs_hir::requires`' own known gaps.
- An autoloaded file's uncalled script frame is compiled dead code — `nvs-ir` `lower_program`.
- `[context] modules` still names no `nvs-runtime` and no `nvs-diagnostics` — `loop-goal.toml`.
- The other two Stage 8 cases (`a-dump-renders-one-record-and-redacts-a-secret`,
  `a-test-attribute-builds-a-table-the-runner-reports`) — `holes.py --cases`.
- ADR 0024 § 5's `string as Core\Html\Markup` waits on `Core\Html` (M7) — `nvs-ir` catch-all roster.
