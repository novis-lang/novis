`$a ??+= $v`, `$a ??-= $v` and `$a ??.= $v` mean `$a = ($a ?? d) op $v`, with `$a` evaluated once and
read the way `??` reads its left operand, so an absent key at any level and a `null` both start from
`d`. That written-out form answers every question about them: which operand types are accepted, what
overflows, what `tainted` and `secret` do, and what the expression's type is. A form that is refused
when written out refuses the operator with the same code.

`d` is the zero of the target's own type: `0` for `int` and `uint`, `0.0` for `float`, the zero
`decimal`, and `""` for `string`. For a `mixed` or union target it is the `int` `0` for `??+=` and
`??-=` and the `string` `""` for `??.=`, and the operator is answered from the runtime tag
(`rule:types/arithmetic`). A type with no zero, such as a `bool`, an enum or an array, is its own `d`,
so the operator is refused with the codes the plain `+=`, `-=` or `.=` is refused with. A target that
can never be `null` or absent takes the plain `+=`, `-=` or `.=`. The target's declared type is unchanged afterwards and nothing is narrowed (`rule:types/narrowing`).

There are exactly three, and no syntax writes another default: `($a ?? 100) -= 1` is `E0105`, and
`$a = ($a ?? 100) - 1` is that program. `$a ??-1` is a `??` and a `-1`, because each operator ends in
`=`.

They exist because `$counts[$word] += 1` throws on the first word not yet counted, and
`$lines[$day] .= $entry` on the first day, so without them a count, a running total or a grouped string
names its target twice: `$counts[$word] = ($counts[$word] ?? 0) + 1`. That form keeps working, and
neither is preferred. Each operator costs one `null`-or-absent test more than the plain one, with the
same one read and one write of the target; `??.=` copies the accumulated string on each append, as
`.=` on an element or a property does. The default is not visible where the operator is written, and
the default carries neither `tainted` nor `secret`.

**Declined:**

- **Reading an absent target as zero under `+=`, `-=` and `.=`.** A misspelled key would count from
  zero silently, which is the hole `rule:types/absent-storage-is-never-a-zero-value` closes. These
  operators say where a default is wanted.
- **A default written at the use, `($a ?? 100) -= 1`.** `($primary ?? $backup) += 1` reads as "add to
  whichever exists", so the same syntax would have two meanings.
- **`??*=`, `??/=` and the bitwise forms.** A product or a quotient that starts from zero stays zero.
- **A bare `??+` or `??-`.** `$a ??-1` already means `$a ?? -1`, so each token needs its `=`.
