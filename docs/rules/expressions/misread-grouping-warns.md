`nvs check` warns with `W1022` at two operator mixes whose grouping a reader misreads, and at no others:
a `??` whose right operand is an arithmetic or `.` expression written without parentheses
(`$n ?? 0 + 10` is `$n ?? (0 + 10)`), and a `?:`, long or short, whose condition is one of those or a
`??` (`"n=" . $ok ? "x" : "y"` tests `"n=" . $ok`, so it is always `"x"`).

The list is closed because every shape on it is valid code, and a warning on valid code is worth its
noise only where the misreading is common. `&&` beside `||`, `-2 ** 2` and a nested ternary are left
alone, and a parenthesized operand never warns. The grammar is unchanged, so the warning costs nothing
when the program runs.

Each warning carries two edits. Parentheses around what the code does now change nothing, so
`source.fixAll.nvs` may apply them on save. The grouping the author most likely meant, the `??` or `?`
taking the operand nearest it, changes what the line does, so it is an alternative a person picks from
the light bulb and no batch applies (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`).

Two errors name the same kind of parentheses. `E0105` on `$ok && $row = $next`, which PHP reads as
`$ok && ($row = $next)`, offers that grouping and recovers the parse as it, so no second error follows
about a target nobody meant. `E0706` on `$flags & 4 == 4`, which is `$flags & (4 == 4)`, offers
`($flags & 4) == 4`.
