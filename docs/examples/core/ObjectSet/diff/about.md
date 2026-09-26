Builds a new `Core\ObjectSet` with the values that are in this set and not in `$other`. This replaces
a loop over one `SplObjectStorage` that skips each object another one contains. The name is `diff`,
the same as `Core\Arr::diff`.

The order of the two sets matters. `$a->diff($b)` gives the values that only `$a` has.
`$b->diff($a)` gives the values that only `$b` has.

A value is matched by identity. The result is a new set, and `diff` does not change this set or
`$other`. The values keep the order they have in this set. If `$other` has every value of this set,
the result is an empty set.

The examples show the values that only one set has, why the order of the two sets matters, and a
common use: finding the guests who have not answered an invitation yet.
