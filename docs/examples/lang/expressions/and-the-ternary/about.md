Five short operators for values that may be missing, and for choosing between two values.

`$a ?? $b` gives `$a`, or `$b` when `$a` is `null` or the array key is not there. `$a ??= $b` writes
`$b` into `$a` only when `$a` is `null`. `$a ?: $b` gives `$a` when it counts as something and `$b`
when it counts as nothing, so an empty string or a zero picks `$b`. `$o?->name` gives `null` when
`$o` is `null`. `$c ? $a : $b` gives `$a` when `$c` is true and `$b` when it is not.

**Good to know:** only the side you end up with is run, so a method call on the side you do not take
never runs. `??` looks only for `null`, while `?:` and `? :` ask whether a value counts as
something.

**The examples below** show a value that may be missing, then a choice between two values, then a
settings file with its gaps filled in.
