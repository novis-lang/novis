# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open at P4** — its `[[check]]`'s first named case,
`tests/conformance/core/a-first-class-callable-lowers.nvst`, is written and green, so the driver's
acceptance run now fails on a later case in the same block rather than on that one.

**A first-class callable lowers, and it lowers to the closure representation that already existed.**
`nvs_ir::lower::closure`'s `lower_callable` builds one forwarding thunk per written `(...)` — a
synthesized class carrying `FN_ARITY`, `FN_PARAM_TAGS` and, for an instance target, `FCC_RECV`, with
an `invoke` that passes its own parameters straight to the target. That module's doc comment owns
the whole representation and the reason it is a thunk rather than a second closure shape; ADR 0031
§ 1 is why it had to be the same object, and it is what lets `Core\Arr::map($xs, Core\Math::abs(...))`
work with no native change at all. The site half is `Lowering::lower_callable_ref`
(`crates/nvs-ir/src/lower/expr.rs:2310`), and both `ExprInfo::CallableRef` panics are gone.

**One divergence is stated rather than hidden: `static::method(...)` binds the declaring class.** The
checker records `ResolvedCall::static_class` only for a written class name, and a class descriptor is
not a value a field slot can hold, so the one spelling whose late static binding would have to
survive past the site is bound early. `self::`/`parent::` mean the declaring class anyway.
`lower_callable`'s doc comment is that fact's home; the redesign is in `## Backlog`.

**`E0793` refuses the two parameter lists a `callable` cannot carry.** `inout` and a variadic tail
both reach the callee as a type confusion — ADR 0031 § 4 gives `callable` no parameter list, so
nothing at a call through one could stage a cell or collect a tail. Refused where the `(...)` is
written (`nvs_types::expr::calls::reject_unforwardable_first_class_callable`), pinned by
`tests/conformance/reject/a-first-class-callable-forwards-every-argument.nvst`. The receiver capture
is a new refcount edge and `tools/leak-check.sh` is clean over it.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is now `E0794`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1058,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's
own ADRs: this session needed **0027 § 1** and **0031 §§ 1-2, 4** and the pack printed 0103, 0078,
0042 and 0119 instead — add those two, plus **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6** and
**0096 §§ 1-1a** from the sessions before. `[context] modules` still misses
`crates/nvs-types/src/attributes.rs`, `routes.rs`, `commands.rs`, `derive.rs`, `testing.rs`,
`defaults.rs`, `consts.rs`, `signatures.rs`, `retrieval.rs`, `crates/nvs-stdlib/src/serialize.rs`
and `secret.rs`, `crates/nvs-types/src/expr/args.rs`, and — new this session —
`crates/nvs-ir/src/lower/closure.rs`, `expr.rs` and `call.rs`, which item 34's own anchors name and
which the map did not print a line for. `orient.py` itself still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**The rest of item 34's lowering gaps. P4 is a `nvs-ir` type-translation gap and D23 a `nvs-types`
resolution one, so they share no file — take P4 first, since it is the next case the acceptance
check names.**

- [ ] **P4 a `: never` method is a terminator** — findings.md § *Panics* P4, item 34. A method
      declared `: never` panics the lowerer even when it is never called: `lower_checked_ty`
      (`crates/nvs-ir/src/lower/mod.rs:2872`) has no arm for `CheckedTy::Never`, and its own
      comment at `crates/nvs-ir/src/lower/mod.rs:2860` already names that as what is left. A `never`
      return means the callee does not come back, so the call site's `Terminator` is the question,
      not the value's representation. Case:
      `tests/conformance/core/a-never-method-is-a-terminator.nvst`, named by the item-34 `[[check]]`
      at `docs/agent/loop-goal.toml:329`.
- [ ] **D23 a named closure recurses** — findings.md § *Divergences* D23, ADR 0031 § 3. `fn fact(int
      $n): int => … fact($n - 1)` parses, and the recursive call resolves as a free function
      (`E0320`): the name is not bound while the body is checked. `check_fn_literal`
      (`crates/nvs-types/src/expr/calls.rs:1241`) is where the binding would go in, and the
      lowering's own capture list is `crates/nvs-ir/src/lower/closure.rs:32`'s `PendingClosure`.
      Case: `tests/conformance/core/a-named-closure-recurses.nvst`, `docs/agent/loop-goal.toml:330`.
- [ ] **D22 `inout` takes only a local** — findings.md § *Divergences* D22, item 34.
      `M::bump(inout $a["k"])` is `E0439`, "cannot be passed to an `inout` parameter **yet**" — the
      staging that would copy an element back is what the word `yet` stands for, and
      `crates/nvs-ir/src/lower/call.rs:975`'s `forget_transferred_since` sits beside the
      `pending_refs` window that already does it for a local. Decide whether ADR 0107 § 2 wants the
      element form at all before writing it; the rest of item 34 is the `[[check]]` block at
      `docs/agent/loop-goal.toml:322`, in the order it is judged.

## Backlog

- `static::method(...)` as a first-class callable binds the declaring class rather than the frame's
  called class — `nvs_ir::lower::closure`'s `lower_callable` doc comment owns the fact; a redesign
  needs a class descriptor a field slot can hold.
- Item 35, the reference cards and chapters — `docs/reference/findings.md` § *Triage*.
- Stage 9's expression `catch`, items 21–23 — ADR 0119, anchors already written.
- Stage 8's two remaining differential cases — `docs/agent/loop-goal.md` § *Stage 8*.
- Stage 5's cache payload — `crates/nvs-cli/src/cache.rs` § *Known gaps*.
