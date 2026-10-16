`nvs check` warns with `W1022` at two operator mixes whose grouping a reader misreads, and at no others:
a `??` whose right operand is an arithmetic or `.` expression written without parentheses
(`$n ?? 0 + 10` is `$n ?? (0 + 10)`), and a `?:`, long or short, whose condition is one of those or a
`??` (`"n=" . $ok ? "x" : "y"` tests `"n=" . $ok`, so it is always `"x"`).

The list is closed because every shape on it is valid code, and a warning on valid code is worth its
noise only where the misreading is common. `&&` beside `||`, `-2 ** 2` and a nested ternary are left
alone: the first is common, deliberate and well understood, the second is pinned by the reference's
own precedence table, and the third groups to the right as documented. A parenthesized operand never
warns. Neither shape is a type error, which is why nothing else reports it: `??` binds looser than
every arithmetic operator and `.`, and a ternary condition accepts any type and tests its truthiness
(`rule:expressions/truthy-positions`).

The grammar is unchanged, so the warning costs nothing when the program runs. The parser raises it,
seeing each written operator once with its source at hand: one `match` per `??`, per `?:` and per
invalid assignment target, allocating only when it warns, and nothing is held past the diagnostic.

Each warning carries two edits. Parentheses around what the code does now change nothing, so
`source.fixAll.nvs` may apply them on save. The grouping the author most likely meant, the `??` or `?`
taking the operand nearest it, changes what the line does, so it is an alternative a person picks from
the light bulb and no batch applies (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`).

Two errors name the same kind of parentheses. `E0105` on `$ok && $row = $next` offers the grouping
`$ok && ($row = $next)` and recovers the parse as it, so no second error follows
about a target nobody meant. `E0706` on `$flags & 4 == 4`, which is `$flags & (4 == 4)`, offers
`($flags & 4) == 4`, with a help line about precedence in place of its general one about converting
the operand. Each of these two fixes is the only grouping of its line that compiles, so neither is an
alternative. Both errors keep their codes and messages.

**Declined:**

- **An error for the warned shapes.** Each is valid code with one meaning, and refusing it would
  break programs that meant exactly that meaning.
- **A hint in the editor alone.** `nvs check` and CI would never see it, and the language server would
  carry a second copy of a check the compiler makes once.
