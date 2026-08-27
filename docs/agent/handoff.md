# Handoff

## State

**M4 — language completeness.** ADR 0069 § 5 now answers what a callback's `$key` carries: a key
handed *to* a callback is an out-flow like a key-valued return, so it is a `string` in every member
and over every array shape. `Core\Arr` was already rendering it, so this was a decision plus its
three homes — the rule in ADR 0069 § 5, one pointer sentence at spec § 2's R9, and the mechanism in
`crates/mwl-stdlib/src/arr.rs`'s module doc § *A callback that does not want a key is never handed
one*. Handing a packed list's position on unrendered is refused there because it would make a
callback's `$key` type depend on how the subject is stored, which is the integer-key/string-key
split ADR 0007 § 5 deleted — not because of what the rendering costs.

The observable half is that `fn($v, int $k)` throws `LogicError`, on a list exactly as on a map.
`tests/conformance/core/arr-a-callback-key-is-a-string-on-a-list-too.mwlt` pins both sides over a
packed list: nine members report the keys they offered, and fourteen are then asked with `int $k`
and counted, `asked` and `refused` both raised from inside the same fourteen blocks.

The tag check is by *representation*, not by declared type: `param_tag_nibble` maps `Ty::Tagged` to
`FN_PARAM_TAG_ANY` (`crates/mwl-ir/src/lower/mod.rs:2724`), so a `mixed $k` and a `?string $k` are
both unchecked. Nothing pins that yet, and it is the next group's first item.

`verify.py` green — conformance 620, differential 173.

## Next group

**The closure tag check's remaining edges, asked from MWL.** The file set:
`tests/conformance/core/`, `crates/mwl-runtime/src/closure.rs:325` (`check_param_tags`),
`crates/mwl-ir/src/lower/mod.rs:2705` (`param_tag_nibble`).

- [ ] **A `mixed` or `?T` parameter is unchecked, and that is the answer rather than a hole.**
      `FN_PARAM_TAG_ANY` is nibble 12 (`crates/mwl-ir/src/lower/mod.rs:2691`) and
      `check_param_tags` `continue`s on it (`crates/mwl-runtime/src/closure.rs:361`), so a callback
      declaring `mixed $k` or `?string $k` accepts whatever arrives. Pin it from a `Core\Arr`
      callback, and say in one sentence of `param_tag_nibble`'s own doc comment why a nullable
      declaration checks nothing — it is `Ty::Tagged`, and the representation is the check.
- [ ] **ADR 0007 § 2's one widening reaches a callback, and stops at 2^53.**
      `crates/mwl-runtime/src/closure.rs:387` widens an `int`/`uint` argument into a `float`
      parameter and throws `ArithmeticError` above 2^53. From MWL that is `Core\Arr::map` over an
      `array<int>` with `fn(float $v) => …`: assert the bound on both sides, the last accepted
      value and the first refused one, in one case.
- [ ] **`reduce` counts positions, not roles.** The carry is argument 1 and the key argument 3
      (`crates/mwl-stdlib/src/arr.rs:2670`), so a two-parameter fold callback puts the *value* in
      position 2 and a three-parameter one does not move it. A mismatch on the carry is refused by
      the same nibble walk as a mismatch on the key; the new case only asks about the key.

## Backlog

- A closure called through the variable holding it panics `mwl-ir` — `docs/agent/playbook.md`
  § *Writing MWL itself*.
- `array<T> as array<U>` does not lower (`crates/mwl-ir/src/lower/expr.rs:877`), which is ADR 0007
  § 2's one missing conversion row.
- A spread argument does not lower (`crates/mwl-ir/src/lower/call.rs:75`).
- `Core\Reflect::typeOf` is not implemented, so no case can ask a value its own type — ADR 0007 § 4
  names it.
