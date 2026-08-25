# Handoff

## State

**§ 2 is closed but for `from`.** `sortByKey` and `reduce` landed this session, leaving
`Core\Arr::from` as the section's last unregistered member — and it is the one that waits on an
`Iterable`/`Iterator` argument rather than on anything in `arr.rs`. The ratchet
(`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is at **9 keys**, of which **1 is `Core\Arr`**;
conformance is **410** of 600 and differential **89** of 150.

Two things the two slices settled, recorded where the work lives:

- **`sortByKey`'s natural order is a byte compare**, so `"10"` sorts before `"9"` — PHP's
  `ksort($a, SORT_STRING)`, not its numeric-guessing default. `mwl_core_arr_sort_by_key`'s doc owns why
  (ADR 0007 § 5 makes every key a `string`, and reading one as a number is what that ADR rejects). The row
  has no `preserveKeys` and no `by` for reasons that doc also states.
- **`reduce`'s `U` is bound by `$initial`, not by the callback**, so the fold's type is the seed's type,
  and the callback is `($carry, $value, $key)`. `docs/spec/01-core-library.md` § 2's callback-convention
  sentence was amended to say both — it is the home of that convention.

`examples/collect.mwl`'s frontier is unchanged: `Core\Out::capture` at `collect.mwl:47`, which lands with
M4S's sink work (ADR 0088 §§ 3, 5). Valgrind is clean over both new refcount edges (a key sort's borrowed
keys, and a fold whose carry is a heap string).

**Orientation gaps, both still open in `docs/agent/loop-goal.toml`:**

- `[context] modules` names four `mwl-runtime` files but not `src/array.rs`, whose
  `next_slot`/`key_at`/`value_at`/`set` contract every `Core\Arr` slice reads.
- `[context] modules`' `mwl-stdlib` patterns cover `str`/`math`/`hash`/`encoding`/`registry`/`lib` but not
  `src/time.rs` or `src/cldr.rs`, which the next group is entirely inside.

## Next group — § 4's three component-view members, in three slices off one file

`crates/mwl-stdlib/src/time.rs`'s own **gap 1** (`time.rs:63`) already specifies all three: they are a
second and third class over machinery that exists, not anything new.

**Shared file set for all three:** `crates/mwl-stdlib/src/time.rs` (`:60` the gap-1 note to strike as it
closes, `:97` the registry imports, `:127` `DURATION` — the class shape to copy, `:279` `address()`,
`:802` `UNIT` and `:826` `WEEKDAY` — the `CoreEnum` shape `Core\Month` takes, `:901` `DATETIME` where all
three members land, `:1058` `TIME`), `crates/mwl-stdlib/tests/spec-members-outstanding.txt`'s § 4 block,
and `tests/conformance/core/`.

- [ ] **`date`** — spec row at `docs/spec/01-core-library.md:495`, `$d->date(): Date`. The big one: it
      brings `Core\Time\Date` itself over `jiff::civil::Date`, with the `plus`/`minus`/`with`/`compareTo`/
      `format` shape `DURATION` and `DATETIME` already write, the `Date::at` constructor, and § 4's
      `Core\Month` enum, which `registry::ENUMS` says waits for exactly this member.
- [ ] **`timeOfDay`** — spec row at `:495`, `$d->timeOfDay(): TimeOfDay`. The same slice again over
      `jiff::civil::Time`, with `TimeOfDay::at`; whatever the `date` slice factors out is what this one
      reuses.
- [ ] **`withTime`** — spec row at `:491`, `$d->withTime(TimeOfDay $t): DateTime`. Last, because it is the
      first member to *take* a `TimeOfDay` rather than answer one; the spec calls it "the common half of
      `with`, spelled as the operation it is".

Two traps that apply: a `Core\Time` case must never assert `Zone::system()` or a wall-clock value
(playbook), and a new class narrows `Core`'s blanket trust for that name, so check the fixtures that write
`Core\Time\…`.

## Backlog

- `§5 compile`/`replaceWith` — both need `Pattern`; `regex.rs` gap 1, and `replaceWith`'s callback is the
  one that does not receive `($value, $key)` (spec `:559`).
- `§11 Random::bytes` and `Hash::stream` — unblocked since `Tag::Bytes` landed; `mwl-runtime`'s module doc.
- `§2 from` — needs an `Iterable<T>|Iterator<T>` parameter; ADR 0053 § 2.
- `§12 Out::capture` — `collect.mwl:47`'s frontier, lands with M4S (ADR 0088 §§ 3, 5).
- `uri.rs` gap 1 — an options bag that tells an omitted option from a written `null`; the same question
  `CoreTy::Union`'s widened rule answers from the other side.
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain (`tools/check-migration.py`).
