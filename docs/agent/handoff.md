# Handoff

## State

**§ 2's four positional structure members are whole.** `slice`, `chunk`, `append` and `prepend` are
built, tested and committed, so `Core\Arr` now owes nine rows rather than thirteen. The design call
they forced is `append`'s keys, and `crates/mwl-stdlib/src/arr.rs`'s member doc comments own it: it
**keeps** the subject's keys and puts each added value under the next free integer key, because
being `$a[] = $v`'s expression form is the whole claim of the row; `prepend` cannot, so it
renumbers everything, which is `padStart`'s answer for `padStart`'s reason. Neither is key-*type*
dependent in ADR 0069 § 3's sense. `slice`'s new `window` (`arr.rs:1178`) is deliberately
`crate::str::window`'s sign rule one unit up, in entries rather than characters.

Conformance is **404** of 600, differential **89** of 150, and the ratchet
(`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is at **17 keys**, of which **9 are
`Core\Arr`**. `examples/collect.mwl`'s frontier is unchanged: `Core\Out::capture` at
`collect.mwl:47`, which lands with M4S's sink work (ADR 0088 §§ 3, 5). `uri.rs` gap 1 (an options
bag that can tell an omitted option from a written `null`) is still open.

Both slices landed in **one commit** rather than one each: they share `arr.rs`'s one contiguous
row insertion and `carry_entry` is written on `append_borrowed`, extracted by the first. The commit
message says so.

## Next group — § 2's remaining structure members, in three slices off one file

**Shared file set for all three:** `crates/mwl-stdlib/src/arr.rs` (`:55` `CLASS`, `:197` the
`chunk` row to insert beside, `:498` `PRESERVE_KEYS`, `:883` `preserve_keys`, `:916` `key_bytes`,
`:976` `append_borrowed`, `:1178` `window`, `:1215` `carry_entry`, `:2745` `copy_all`, and
`address()` below the rows), `crates/mwl-stdlib/tests/spec-members-outstanding.txt`'s § 2 block,
and `tests/conformance/core/`. `arr.rs:821` `mwl_core_arr_map` is the closure-calling shape slice 3
does not need but slices after this group will.

- [ ] **`replaceRange`** — spec row at `docs/spec/01-core-library.md:240`,
      `(array<T> $a, int $offset, ?int $length, array<T> $replacement = []): array<T>`. It is
      `slice`'s window with the entries *substituted*, so read both positions through `window`
      (`arr.rs:1178`) and the pair is exact for every sign, as `Core\Str`'s two already are. **Check
      this first:** `registry::Const` has no array variant (`registry.rs:372-383`), so the written
      `= []` default has nowhere to come from — either add `Const::EmptyArray` and its arm in
      `mwl-types/src/core_lib.rs:220`'s `lower_const` (whose `panic!` arm is explicit that a
      missing variant fails at build time), or make the parameter required and amend the spec row.
      Adding the variant is the smaller change and the spec already wrote the default. The result
      renumbers, like `slice`'s; no `preserveKeys` option, because the spec row declares none.
- [ ] **`flatten` and `flattenDeep`** — spec rows at `:250` and `:251`, both
      `(array<T> $a): array<T>`, one level and all levels. **The signature is the open question:**
      `T` binds from the subject's element type, so over an `array<array<string>>` the row promises
      `array<array<string>>` back, which is not what either member returns. Decide it under
      loop-goal.md § *Standing decisions* — `array<mixed>`, or amending the spec rows — and record
      it in `arr.rs`'s module doc beside the `array<T|U>` call the combination members made. A
      non-array element passes through rather than throwing (that is what "one level" means over a
      mixed array), and keys are discarded: there is no non-arbitrary key for an entry lifted out
      of a nested array, which is `prepend`'s reasoning again.
- [ ] **`column`** — spec row at `:257`,
      `(array<array<T>> $a, int|string $column, {indexBy?: int|string}): array<T>`. `key_bytes`
      (`arr.rs:916`) is the `int|string` key reader both arguments want; `ARRAY_KEY` beside it is
      the union `flip`'s row already names. A row missing the column key is skipped, as
      `array_column` does; `indexBy` naming a missing key falls back to the appended position.

## Backlog

- `Core\Arr`'s four remaining § 2 rows after this group — `from`, `groupBy`, `mapKeys`, `reduce`,
  `sortByKey` — spec `:256`, `:317`, `:310`, `:312`, `:338`; `from` needs `Iterable` (§ 9's gap).
- `Core\Out::capture` at `examples/collect.mwl:47` — the gate's frontier, M4S sink work, ADR 0088.
- § 4's `Date`/`TimeOfDay`/`Core\Month` — `mwl_stdlib::time` gap 1.
- § 10's constructor `{previous: $e}` shape and `$e->location` — ADR 0071 § 5 needs them.
- Stage 4's counts are their own work: conformance 404/600, differential 89/150.
- `uri.rs` gap 1 — an options bag that distinguishes an omitted option from a written `null`.
