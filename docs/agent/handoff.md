# Handoff

## State

**M4, Stage 00, and five of its six shapes are closed.** Items 46 and 47 landed this session and
the gate's `nvs-types` block now stops at item 48.

`nvs_types::signatures::MethodSig::returns_static` records that a declaration wrote `static` as its
return type — read off the *written* annotation, because lowering is what loses the distinction
(`lower_type` interns `static` and `self` to the same class). `crate::expr::calls` substitutes the
called class at exactly two `return_ty` sites: the receiver's own type at an instance call, and
`called_class_of`'s answer at a static call — an explicitly named class for `Base::make()`, the
enclosing class for all three of `self`/`static`/`parent`, since late static binding forwards
through `parent::` where `resolve_class_expr` would not.

**The soundness question item 46 arrived carrying is decided, and it is decided as a refusal.** A
body declaring `static` must answer the called class, so `return new self();` is `E0741`. The
whitelist is `$this`, `new static(...)`, and a call forwarded through `static`/`self`/`parent`/
`$this` to a member that itself returns `static`; `crate::returns::for_each_return` is the walk.
That is stricter than PHP, which accepts the declaration and raises a `TypeError` only at a
subclass call site — the reasoning, and the three alternatives weighed against it, is at
`MethodSig::returns_static` and in `E_STATIC_RETURN_NOT_CALLED_CLASS`'s own doc. `ResolvedCall`
deliberately still records the *declared* return type: `nvs-ir` reads it for a representation, and
a class and its subclass erase to the same `Ty::Object`.

**`instanceof` narrows to an interface now.** `locals.rs`'s `instanceof_residue` accepts
`SymbolKind::Interface` and every reserved global name whose `nvs_hir::interfaces` roster entry
takes no type parameters — which leaves out only `Iterable`/`Iterator`, written with an argument
wherever they are declared. Nothing in `nvs-ir` needed teaching: an interface residue erases to the
same pointer a class one does, and the new conformance case runs the narrowed call rather than only
compiling it.

## Next group

**Item 48 and then the corpus it wants.** One file set: `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-stdlib/src/registry.rs`, with cases under `tests/conformance/class/`.

- [ ] **Item 48 — `new` on a `Core` class with no constructor is a diagnostic.**
      `crates/nvs-types/src/expr/calls.rs:322` is `infer_new`, `:450` is
      `reject_arguments_to_implicit_constructor` (which already holds a *user* class to zero
      arguments) and `:925` is `check_new_target`, whose `NewTarget::Name` arm lets a `Core` name
      through on `qname.is_core()` with no registry lookup at all. The user-class half is pinned by
      `tests/conformance/class/a-class-with-no-constructor-takes-no-arguments.nvst`, so the shape
      of the diagnostic is settled and only the `Core` side is missing. Gate name:
      `a_core_class_with_no_constructor_refuses_arguments`.
- [ ] **A `.nvst` case for the `Core` half of the same rule**, beside the user-class one named
      above — one arity rule, both sides of the `Core`/user line, which is what that gate check's
      own comment in `docs/agent/loop-goal.toml:256` asks for.
- [ ] **Item 49's shape table**, if the two above leave room:
      `every_spellable_expression_reaches_a_diagnostic_or_an_ir` — the sweep that would have found
      items 45-48 rather than having them found one at a time. `docs/agent/loop-goal.md` owns it.

## Backlog

- A class constant whose value has no constant form (`const ROWS = [1, 2];`) panics `nvs-ir`
  rather than being diagnosed — `crates/nvs-types/src/lib.rs`'s known gaps.
- ADR 0046 § 5's payload folder could now resolve a class constant, but `retrieval.rs`'s
  `fold_value` carries no namespace context to resolve the class name with — that module's docs.
- `E0741`'s whitelist ignores a `?static` or a union naming `static`; `writes_static_return` owns
  why the conservative half is to answer the declared type there.
- `refusals.rs`' recognizer matches a refusal by phrasing, so its ceiling of 4 is a floor —
  item 49 owns it.
