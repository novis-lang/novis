A scalar and an `array<T>` never gain methods. There is no `$s->length()` and no `$a->map()`; `Core\Str`
and `Core\Arr` are the one spelling for both.

The alternative is a second surface that grows in parallel with the first forever, which is how the twin
problem regrows after it has been removed once. Keeping the operations on the domain class also keeps them
where the subject-first rule (`rule:core-api/subject-first`) puts them, so a reader learns one call shape
rather than two. Member access on a value that has no members is refused where it is written
(`rule:types/erased-member-access`).

`array<T>` is list, stack, queue and dictionary at once, so the collection classes that restate it are
absent. Only three structures exist beside it, each expressing what an ordered `int|string`-keyed hash
cannot: `Core\ObjectMap<K, V>` and `Core\ObjectSet<T>`, keyed by object identity, and `Core\Heap<T>`,
ordered by priority through a comparator or `Comparable` (`rule:classes/comparable`).

**Declined:**
- **A fuller typed collections library, `List<T>`, `Map<K, V>` and `Set<T>`, beside `array<T>`.** It
  gives two ways to hold a sequence, a conversion at every library boundary, and a standing question of
  which one a signature should declare.
