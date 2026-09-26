Builds a new `Core\ObjectSet` with only the values that are in this set and also in `$other`. This
replaces a loop over one `SplObjectStorage` that checks `contains` on another one.

A value is matched by identity. An object with equal fields that is not the same object is not in
both sets, so it is not in the result.

The result is a new set. `intersect` does not change this set or `$other`. The values keep the order
they have in this set. If the two sets have no value in common, the result is an empty set.

The examples show the values two sets share, two sets that share nothing, and a common use: finding
the users who are online and are also allowed to receive an alert.
