# Handoff

## State

**§ 2 owes six structure members.** `replaceRange`, `flatten` and `flattenDeep` are built, tested and
committed, leaving `column`, `from`, `groupBy`, `mapKeys`, `reduce` and `sortByKey`. Two design calls
they forced are recorded where the work lives:

- **`registry::Const::EmptyArray` is new**, with its `ConstArg` twin and the `InstKind::ArrayNew`
  arm a written `[]` already lowers to. It is the only array constant the registry can state, and
  its own doc comment in `registry.rs` says why a populated one is not worth a second literal
  grammar. It costs one allocation per use site, like `Const::Str`.
- **`flatten` and `flattenDeep` do not share a signature**, and `docs/spec/01-core-library.md`'s two
  rows were amended to say so: `flatten(array<array<T>>): array<T>` is exact, while `flattenDeep`'s
  depth is the caller's data, so a nested `T` would bind one level too shallow over a three-deep
  argument and declare a nesting its answer does not have. `array<mixed>` both ways is the *sound*
  spelling; `arr.rs`'s member doc owns the reasoning.

Conformance is **406** of 600, differential **89** of 150, and the ratchet
(`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is at **14 keys**, of which **6 are
`Core\Arr`**. `examples/collect.mwl`'s frontier is unchanged: `Core\Out::capture` at
`collect.mwl:47`, which lands with M4S's sink work (ADR 0088 §§ 3, 5). `uri.rs` gap 1 (an options
bag that can tell an omitted option from a written `null`) is still open.

A valgrind run over the new default covers the one new refcount edge — an omitted `array<T>`
argument materializes a fresh refcounted value inside an argument list — and it is clean.

## Next group — § 2's key-shaped structure members, in three slices off one file

**Shared file set for all three:** `crates/mwl-stdlib/src/arr.rs` (`:55` `CLASS`, `:590`
`SORT_OPTIONS`, `:618` `address()`, `:850` `mwl_core_arr_map` — the closure-calling shape, `:945`
`key_bytes`, `:1005` `append_borrowed`, `:1023` `append_values`, `:1051` `copy_entry`, `:2072`
`mwl_core_arr_count_by` — a callback keyed by its answer, which is `groupBy` minus the bucket,
`:2215` `mwl_core_arr_sort`, `:2463` `subject`, `:2481` `array_at`),
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`'s § 2 block, and `tests/conformance/core/`.

- [ ] **`column`** — spec row at `docs/spec/01-core-library.md:257`,
      `(array<array<T>> $a, int|string $column, {indexBy?: int|string}): array<T>`. The nested-`T`
      parameter is `flatten`'s, already proven to bind (`generics.rs:126` recurses through
      `Ty::Array`), and `array_at` (`:2481`) is how each row is borrowed. Two option questions the
      spec does not answer and this slice must: a row **missing** the named column — skip it, as
      PHP's `array_column` does — and a missing `indexBy` key on a row that *has* the column. Take
      PHP's answer there too (fall back to the next integer key) and say so in the member doc.
      `key_bytes` (`:945`) normalizes both option values, since either may be an `int`.
- [ ] **`mapKeys` and `groupBy`** — spec rows at `:315` and `:322`, both a `callable` over each
      value. `mwl_core_arr_map` (`:850`) is the closure-calling shape to copy — note it takes `ctx`,
      not `_ctx` — and `mwl_core_arr_count_by` (`:2072`) is nearer still: it already keys a result
      by what a callback answered, so `groupBy` is that with an `array<T>` bucket instead of a
      counter. `mapKeys`'s answer collides when two values map to one key; last-wins matches
      `flip`'s recorded rule, so say so rather than inventing a third.
- [ ] **`sortByKey`** — spec row at `:343`,
      `(array<T> $a, {order?: Order, comparator?: callable}): array<T>`. It is
      `mwl_core_arr_sort` (`:2215`) with the *key* compared instead of the value, and `SORT_OPTIONS`
      (`:590`) is the bag to model the new one on — minus `by`, which a key has no use for. Keys
      preserved, necessarily: sorting by key and then discarding them answers nothing.

## Backlog

- `§2 from` needs `Iterable<T>`/`Iterator<T>` and `{limit}` — spec `:256`; ADR 0053 § 5 owns the
  no-keys rule, and § 9's `Core\Heap` waits on the same `Iterable`.
- `§2 reduce` needs a `U` return bound from a third argument — spec `:317`; check
  `generics.rs`'s `callback_result_var` before budgeting it.
- `§4 date`/`timeOfDay`/`withTime` — `mwl_stdlib::time` gap 1's component views.
- `§5 compile`/`replaceWith` need a `Pattern` instance — `mwl_stdlib::regex` gap 1.
- `§11 Random::bytes` and `Hash::stream`; `§12 Out::capture` lands with M4S's sinks (ADR 0088 § 3).
- ADR 0088's registry-wide qualifier classification for every member row — plan's `Open now`.
