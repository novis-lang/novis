Builds a new `Core\ObjectSet` with only the values that are in this set and also in `$other`. This
replaces a loop over one `SplObjectStorage` that checks `contains` on another one.

A value matches when it is `==` to a value in the other set. For an object of your own class, that is
only the same object. Two different objects with equal property values do not match, so neither is in
the result. Two `Date` values for the same day do match.

The result is a new set. `intersection` does not change this set or `$other`. The values keep the
order they have in this set. If the two sets have no value in common, the result is an empty set.

The examples show the values two sets share, and two sets that share nothing. The last one shows a
common use: finding the users who are online and are also allowed to receive an alert.
