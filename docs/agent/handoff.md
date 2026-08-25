# Handoff

## State

**§ 2's combination and set halves are both whole.** ADR 0069's four combination members
(`overlay`, `overlayDeep`, `underlay`, `appendAll`) and the set half (`diff`, `intersect`, with
`Core\SetOn { Values, Keys, Both }` in `registry::ENUMS`) are built, tested and committed.
`crates/mwl-stdlib/src/arr.rs`'s module doc owns the two design calls they forced, both taken
under loop-goal.md § *Standing decisions* rather than as ADRs:

- **`array<T|U>` is written literally.** The handoff that opened this group said a union cannot be
  a return type; that was wrong — `CoreTy::Union`'s own doc says "legal in **either** direction",
  and `Core\Arr::sum` already returns one. So the combination members answer the spec's
  `array<T|U>` rather than the safe `array<mixed>`. The cost is a shape rule: `T` binds from the
  base and `U` from the *first* layer, first-occurrence-wins, so every later layer is checked
  against the first one's element type.
- **`on` selects, `by` maps what it selected.** A `by` under `SetOn::Keys` maps the key, not the
  value — the alternative puts an option in the language that cannot change the answer.

Conformance is **402** of 600, differential **89** of 150, and the ratchet
(`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is at **21 keys**, of which **13 are
`Core\Arr`** — the structure members, which are the next group. `examples/collect.mwl`'s frontier
is unchanged: `Core\Out::capture` at `collect.mwl:47`, which lands with M4S's sink work
(ADR 0088 §§ 3, 5).

`uri.rs` gap 1 (an options bag that can tell an omitted option from a written `null`) is still
open and still the model problem for `$uri->with`.

## Next group — § 2's structure members, in three slices off one file

**Shared file set for all three:** `crates/mwl-stdlib/src/arr.rs` (`:55` `CLASS`, `:199` the
`padStart` row for a `T $value` parameter, `:221` the `reverse` row for a `{preserveKeys}` bag,
`:455` `PRESERVE_KEYS`, `:549` `address`), `crates/mwl-stdlib/tests/spec-members-outstanding.txt`'s
§ 2 block, and `tests/conformance/core/`. `arr.rs`'s own `for_each_layer` (just landed, beside the
combination helpers) is the variadic-tail walk all of slice 1 needs; `Core\Str::slice` at
`str.rs:119` is the `?int $length = null` model.

- [ ] **`append` and `prepend`** — spec rows at `docs/spec/01-core-library.md:241-242`, both
      `(array<T> $a, T ...$values): array<T>`. A `CoreTy::Variadic(&CoreTy::Var("T"))` tail is one
      ABI argument holding the trailing values under `"0"`, `"1"`, …, so both are `args: [2]`.
      `prepend` keeps the values in written order ahead of the subject, and both discard the
      subject's own keys only where `array_unshift` would — check `reverse`'s note first.
- [ ] **`slice` and `chunk`** — spec rows at `:238` and `:240`. `slice` is
      `(array<T> $a, int $offset, ?int $length = null, {preserveKeys?: bool})`, so it is
      `args: [4]`; `chunk` is `(array<T> $a, uint $size, {preserveKeys?: bool}): array<array<T>>`,
      `args: [3]`. A negative offset counts from the end, as `Core\Str::slice` already does.
- [ ] **`replaceRange`** — spec row at `:239`,
      `(array<T> $a, int $offset, ?int $length, array<T> $replacement = []): array<T>`.
      **First question to settle:** `registry::Const` has no empty-array variant (its variants are
      `Null`, `Bool`, `Int`, `Uint`, `Float`, `Str`, `Bytes`, `EnumCase`, `Built`), so that default
      cannot be spelled today. Either grow `Const` and `mwl_types::defaults::ConstArg` together —
      that enum's own doc calls the friction deliberate — or make `$replacement` required and say
      so in the spec row. Pick one and record it in `arr.rs`'s module doc.

## Backlog

- `Core\Out::capture` — the last § 12 key; blocked on M4S's sink work (ADR 0088 §§ 3, 5).
- § 2's remaining ratchet keys after this group: `column`, `flatten`, `flattenDeep`, `from`,
  `groupBy`, `mapKeys`, `reduce`, `sortByKey`.
- The `?array<T>` index hole — `mwl-ir` panics at `crates/mwl-ir/src/lower/expr.rs:2952`; playbook
  § *Writing a test case* has the three spellings that work around it.
- § 4's `Date`, `TimeOfDay` and `Core\Month` — `mwl_stdlib::time` gap 1.
- § 5's `compile`/`replaceWith`, which need `Pattern` — `mwl_stdlib::regex` gap 1.
- § 10's `{previous: $e}` options shape and `$e->location` — ADR 0071 § 5 needs them.
- § 11's `Random::bytes` and `Hash::stream` — the `bytes` tag they waited on exists now.

## Gap in `orient.py`'s pack

`loop-goal.toml`'s `[context] adrs` printed no section of **ADR 0069** even though `[context] rules`
names it. § 1's rule table — the per-member replace/ignore/append rule, the key-order rule and
`overlayDeep`'s list test — is what this group was written against, and it cost a `sed -n` to get.
Add `0069` §§ 1, 3 to `[context] adrs`.
