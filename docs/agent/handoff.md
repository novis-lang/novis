# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 is closed. Item 32 has nine findings left** — P6, P9, P10, P11, P13, P14, U18, U19 and
U20; P12, P16, M5, M8, M9, D30, P2, P3, P7 and P8 are closed, the last four this session.

**A receiver is a thing the checker now asks about, in one question asked two ways.** A non-static
method reached through a class name is `E0778` where the frame holds no `$this` — file scope, or a
`static` method — so `self::f()` and `parent::f()` from an instance method stay legal, because they
forward that frame's own. `Core`'s half stays `E0458` and stays unconditional: ADR 0063 R20 gives a
`Core` instance member one spelling, so the static one is refused wherever it is written.
`crates/nvs-types/src/expr/members.rs`'s two reporters sit beside each other and their doc comments
own the difference. `$this` itself is `E0779` where no receiver is in scope, and `crate::check` no
longer seeds it into a `static` method's scope at all.

**`throw` and `clone` are held to their operands** — `E0780` and `E0781`, both asked through
`can_hold_an_object`, so only what provably cannot be an object is refused and `mixed`, `object` and
a type variable keep the behaviour they had. `throw` additionally refuses a *class* outside spec
§ 10's tree, through `is_throwable_shaped` (`crates/nvs-types/src/expr/quals.rs:331`), which is the
one predicate that already answered that question. All four codes closed a `nvs-ir` panic; the
lowerer's own comments already asserted the checker refuses these.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1033, six of
eight named cases written.

**`orient.py` still prints four dead `[context] modules` patterns** —
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`. This session also needed `crates/nvs-types/src/expr/assign.rs`,
`crates/nvs-types/src/expr/members.rs`, `crates/nvs-types/src/locals.rs`,
`crates/nvs-diagnostics/src/lib.rs` and `crates/nvs-hir/src/hierarchy.rs`, none of which
`[context] modules` names, and ADR 0008 §§ 1 and 5 and ADR 0023 § 1 — an `[context] adrs` gap.

## Next group

**Item 32's parser half — four constructs that parse today and should not.** They share the syntax
crate's own front end and the `E01xx`/`E02xx` bands of `crates/nvs-diagnostics/src/lib.rs` (next
free: `E0126` and `E0241`), plus the `tests/conformance/reject/` cases
`docs/agent/loop-goal.toml`'s item-32 check names. Check each finding against the tree before
writing: the playbook's bullet on a stale `loop-goal.toml` comment applies to `findings.md` too.

- [ ] **U18 and U19** — `<>` is lexed as `!=` where ADR 0090 § 1 makes `==`/`!=` the whole set, and
      a `try` with neither `catch` nor `finally` is accepted where PHP refuses it
      (`crates/nvs-syntax/src/lexer.rs:855` for the `<>` branch,
      `crates/nvs-syntax/src/parser/stmt.rs:645` for `parse_try`). Cases:
      `tests/conformance/reject/not-equal-is-spelled-bang-equals.nvst` and
      `tests/conformance/reject/a-try-has-a-clause.nvst`.
- [ ] **U20 and P10** — the braced `namespace A { … }` form parses and resolves, and an anonymous
      `new class { … }` passes the checker and panics in `nvs-ir`
      (`crates/nvs-syntax/src/parser/decl.rs:213` for `parse_namespace_decl`,
      `crates/nvs-syntax/src/parser/expr.rs:1384` for `parse_new_anon_class`). Cases:
      `tests/conformance/reject/a-namespace-statement-is-unbraced.nvst` and
      `tests/conformance/reject/an-anonymous-class-is-refused.nvst`.
- [ ] **P13 and P14** — a `catch` binding read after its clause panics as an undeclared local rather
      than being refused, and `catch (A | B $e)` checks and then fails in codegen; both are the
      checker's, not the parser's (`crates/nvs-types/src/locals.rs:1106` is the `Try` arm that
      declares the binding, `crates/nvs-types/src/locals.rs:671` the reuse rule it lands in). Case:
      `tests/conformance/reject/a-catch-binding-ends-with-its-clause.nvst`.

## Backlog

- Items 33–35 of stage 0c — the modifiers, the lowering gaps and the docs (`docs/agent/loop-goal.md`).
- Item 32's remaining panics P6 and P9 — an untyped interface constant, and `new $name()`.
- Stage 9's items 21–23: ADR 0119's expression `catch`, accepted and unimplemented.
- Stage 8's last two named cases (`docs/agent/loop-goal.toml`'s stage 8 check).
- ADR 0011 § 3 maps `PHP_EOL` to `Core\Env::EOL`; `docs/reference/tools/30-php-differences.md:31`
  says `"\n"` is the newline and there is no such class. One of the two is the bug — item 35's.
