`inout` is a reserved word, written **before the type**, in every position a by-reference binding is legal:

```nvs
public static function bump(inout int $slot): int { … }

foreach ($xs as inout int $v) { … }
[inout int $a, inout int $b] = $pair;
```

It sits in the modifier slot the parser already reads ahead of the type, so `inout int $x` reads like the
`public readonly int $x` beside it and costs the grammar nothing.

The word names the mechanism. A parameter is copy-in/copy-back through a cell the call stages: the callee
gets the cell's address, and the value is copied back after the call returns — a callee that throws is the
one path that does not reach the write-back, so a caller sees no partial write. A `foreach` value binding
is the other mechanism, pushing each assignment through to the array element as it happens. Neither is an
alias. Which declarations may carry one is unchanged: an anonymous function may not, and a generator may not.
