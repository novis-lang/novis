Which part of an entry is compared when you ask what two arrays have in common, or what only one of
them has.

`Core\Arr::diff` and `Core\Arr::intersect` compare the values by default. `Values` compares values
alone, `Keys` compares names alone, and `Both` counts an entry as matching only when the name and the
value both match.

The answer always keeps the first array's names and its order, so the result reads like the list you
started with, minus or plus whatever the question asked about.

**Good to know:** the option decides what is compared, and a key function passed alongside it maps
that same part. Under `Keys` a key function is handed the name, not the value.
