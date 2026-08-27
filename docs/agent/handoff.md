# Handoff

## State

**M4 — language completeness.** The closure tag check is now pinned on both of its edges, from MWL.
`mwl_runtime::closure::check_param_tags` compares the *representation* a parameter erases to and never
its declared type, so `mixed` and every `?T` are `Ty::Tagged` → `FN_PARAM_TAG_ANY` and admit any
argument while the bare twin of each is refused with `LogicError`; `param_tag_nibble`'s doc comment
(`crates/mwl-ir/src/lower/mod.rs:2705`) is the one home for why a nullable declaration checks nothing.
ADR 0007 § 2's one implicit widening is applied inside that same check
(`crates/mwl-runtime/src/closure.rs:387`) and stops at 2^53 on both signs and for `uint` as well as
`int` — 2^54 is refused although an `f64` represents it exactly, because the bound is the range over
which every integer is representable rather than this integer's own luck. Both new cases assert an
ordered accept/refuse string rather than a total, so a pair that flipped fails rather than balancing.

Not yet asked anywhere: whether the `object` nibble distinguishes classes, and whether the wrong
*position* in `reduce`'s three arguments is what its refusal names. Those are the next group.

`verify.py` green — conformance 622, differential 173.

## Next group

**The tag check's last two edges, and what a refusal names.** The file set:
`tests/conformance/core/`, `crates/mwl-runtime/src/closure.rs:325` (`check_param_tags`),
`crates/mwl-ir/src/lower/mod.rs:2705` (`param_tag_nibble`), `crates/mwl-stdlib/src/arr.rs:2611`.

- [ ] **An `object` parameter is checked as an object and not as its class.** `Ty::Object` is nibble 7
      (`crates/mwl-ir/src/lower/mod.rs:2718`) and the tag carries no class label, so a callback
      declaring the *wrong* class accepts the argument and only fails later, at a member it does not
      have. Reach the check with an object argument through `Core\Out::capture($body, {through: $fn})` —
      the playbook's § *Writing MWL itself* owns that spelling — and pin both halves: a `string $c`
      refuses, a `?Core\Cli\Text $c` accepts, and so does a different class. ADR 0007 § 2, ADR 0031 § 1.
- [ ] **`reduce` counts positions, not roles.** The carry is argument 1, the element 2 and the key 3
      (`crates/mwl-stdlib/src/arr.rs:2611`), and `check_param_tags` names the *position* in its message
      (`crates/mwl-runtime/src/closure.rs:400`) because a `callable` has no parameter names to name.
      Pin that a wrong declaration in each of the three positions is refused independently, and that the
      seed's type is what argument 1 has to match rather than the element's. ADR 0069 § 5, ADR 0031 § 1.
- [ ] **An enum rides as its backing integer, so the check cannot tell them apart.**
      `Ty::Int | Ty::Enum(EnumRepr::Int) => 2` (`crates/mwl-ir/src/lower/mod.rs:2713`) is the decision;
      what is unproven is whether an `array<SomeEnum>` reaches a `Core\Arr` callback at all. One scratch
      run under `.agent-tmp/` settles it — if it does not lower, put it in the backlog and take the
      first two. ADR 0010, ADR 0007 § 4.

## Backlog

- A closure called through the variable holding it panics `mwl-ir` — `docs/agent/playbook.md`
  § *Writing MWL itself*.
- `array<T> as array<U>` does not lower (`crates/mwl-ir/src/lower/expr.rs:877`), which is ADR 0007
  § 2's one missing conversion row.
- A spread argument does not lower (`crates/mwl-ir/src/lower/call.rs:75`).
- `Core\Reflect::typeOf` is not implemented, so no case can ask a value its own type — ADR 0007 § 4
  names it, and the absence is why the two cases landed here observe a representation through a
  *boundary* (a value `float` refuses and `?float` takes) rather than by asking.
