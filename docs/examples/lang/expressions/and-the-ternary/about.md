Five short operators for values that may be missing, and for choosing between two values.

`$a ?? $b` gives `$a`, or `$b` when `$a` is `null` or the array key is not there. `$a ??= $b` writes
`$b` into `$a` only when `$a` is `null` or missing. `$a ?: $b` gives `$b` when `$a` counts as
nothing, such as an empty string or a zero. `$o?->name` gives `null` when `$o` is `null`.
`$c ? $a : $b` gives `$a` when `$c` is true and `$b` when it is not.

**Good to know:** only the side you end up with is run. In `$o != null ? $o->name : "none"`, the
first side runs only when `$o` is not `null`. `&&` works the same way.

**The examples below** show a missing value, a choice between two values and a settings
file with its gaps filled in. The last two show `??-=` and `??.=`, and the error for a write through
`?->`.
