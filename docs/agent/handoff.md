# Handoff

## State

**§ 2 owes three structure members.** `column`, `mapKeys` and `groupBy` are built, tested and committed,
leaving `from`, `reduce` and `sortByKey`. Two design calls they forced are recorded where the work lives:

- **A union option is restricted by `null`, not by literals.** `registry::CoreTy::Union`'s doc comment
  and `a_union_option_excludes_null` (renamed from `a_union_option_is_a_closed_set_of_literals`) now say
  the real invariant: an omitted option passes `Const::Null`, so the rule is only that no *written* value
  can arrive as a `Tag::Null` too. `{indexBy?: int|string}` is the row that showed
  `Core\Validate::isIp`'s `{version?: 4|6}` was one instance of that rule rather than the rule itself.
- **A `groupBy` bucket keeps each entry's own key**, where `flatten`/`appendAll`/`values` renumber. A
  partition cannot collide two entries, so ADR 0069 § 3's reason for renumbering does not apply and
  preserving is lossless; `docs/spec/01-core-library.md` § 2's key-rule sentence was amended to say so,
  and `arr.rs`'s member doc owns the reasoning. `Core\Arr::values` over a bucket recovers a list.

Conformance is **408** of 600, differential **89** of 150, and the ratchet
(`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is at **11 keys**, of which **3 are `Core\Arr`**.
`examples/collect.mwl`'s frontier is unchanged: `Core\Out::capture` at `collect.mwl:47`, which lands with
M4S's sink work (ADR 0088 §§ 3, 5). `uri.rs` gap 1 (an options bag that can tell an omitted option from a
written `null`) is still open — and the widened union rule above is the same question from the other side.

A valgrind run over a scratch file calling all three members is clean; the new refcount edges are a
retained subject value stored under a callback-named key, and a bucket array built beside the result.

**Orientation gap:** `orient.py`'s `[context] modules` names four `mwl-runtime` files but not
`src/array.rs`, whose `get`/`set`/`append`/`key_at` contract every `Core\Arr` slice reads. Add
`mwl-runtime/src/array.rs` to that manifest field.

## Next group — § 2's last three structure members, in three slices off one file

**Shared file set for all three:** `crates/mwl-stdlib/src/arr.rs` (`:55` `CLASS`, `:332` the `sort`
registry row, `:572` `BY_OPTION`, `:631` `SORT_OPTIONS`, `:659` `address()`, `:974`
`mwl_core_arr_map_keys` — the newest callback-calling shape, `:1168` `key_bytes`, `:2439` `Extracted`,
`:2513` `mwl_core_arr_sort`, `:2595` its `compare` closure, `:2666` `comparator_sign`, `:2693`/`:2729`
the stable merge sort, `:2761` `subject`), `crates/mwl-stdlib/tests/spec-members-outstanding.txt`'s § 2
block, and `tests/conformance/core/`.

- [ ] **`sortByKey`** — spec row at `docs/spec/01-core-library.md:345`,
      `(array<T> $a, {order?: Order, comparator?: callable}): array<T>`. `mwl_core_arr_sort` (`:2513`) is
      the whole machinery — the same stable merge over a permutation, with the *keys* extracted instead of
      a `by` closure's answers, so the slice is a second entry point rather than a second sort. Keys are
      always strings (`arr-keys-are-always-strings.mwlt`), so the default order is a byte compare and the
      member preserves keys unconditionally — there is no `preserveKeys` option in the row, which is the
      one thing to state in the member doc. A `comparator` receives two keys; `by` is deliberately absent.
- [ ] **`reduce`** — spec row at `:317`, `(array<T> $a, callable $fn, U $initial): U`. The first member
      whose `U` is bound from an ordinary *parameter* rather than a callback's return
      (`CoreTy::Var("U")` as both a whole parameter and the return type — `fill`'s `T` is the shape to
      copy at `:332`-ish rows). The callback receives `($carry, $value, $key)`, which is one argument
      more than every other `Core\Arr` callback: say why in the member doc, and check
      `mwl_runtime::call_closure`'s trimming still lets a two-parameter closure bind.
- [ ] **`from`** — spec row at `:256`, `(Iterable<T>|Iterator<T> $items, {limit?: uint}): array<T>`.
      **Check first whether this is reachable**: § 9 still owes the `Iterable` interface, and the row's
      parameter names it. If `CoreTy` cannot state that parameter yet, leave the ratchet key and move the
      slice to the backlog with one line saying which half is missing, rather than registering a weaker
      signature.

## Backlog

- `Core\Out::capture` — the `collect.mwl` frontier; lands with M4S's sink work (ADR 0088 §§ 3, 5).
- ADR 0069's four combination members and the `diff`/`intersect` set half — `docs/spec/01-core-library.md` § 2.
- `uri.rs` gap 1 — an options bag that distinguishes an omitted option from a written `null`.
- § 9's `Core\Heap` and the `Iterable` its three rows declare — `mwl_stdlib::objmap`'s neighbours.
- § 4's `Date`/`TimeOfDay`/`Core\Month` — `mwl_stdlib::time` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
