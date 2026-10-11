A `switch` owns `break` and nothing else, so `continue` always continues the innermost enclosing loop. A
bare `continue` inside a `switch` moves that loop to its next iteration. PHP counts a `switch` as a
looping structure for `continue`, so a bare one there behaves as `break`, and PHP warns that the
program probably meant `continue 2`. Novis takes the meaning that warning points at.

A written level counts the way PHP counts. `break N` and `continue N` name the `N`-th enclosing
statement, and a `switch` is one of them for both keywords. A level that lands on a `switch` for
`continue` then walks further out to the nearest loop. So `continue 2` inside a `switch` means here
exactly what it means in PHP, and the one divergence stays where the bare keyword put it: a level
naming a `switch` for `continue` continues the loop instead of breaking the `switch`.

A level with nothing to name is `E0475` where it is written, never a panic. That is a level that is not
a literal, a `0`, a level past the enclosing depth, and a `continue` with no loop at or outside its
frame. PHP refuses all four at compile time too, because a level resolves to a statically known
statement and one that names none has nothing to lower to.

**Declined:**
- **A bare `continue` in a `switch` behaving as `break`.** A keyword that means one thing inside a
  `switch` and another everywhere else costs simplicity, and the compatibility it buys is one PHP
  itself warns against.
- **Counting loops alone for a written level.** It would silently retarget `continue 2` inside a
  `switch` inside two nested loops from the inner loop to the outer one, which is worse than a
  diagnostic, and it would make PHP's own `continue 2` inside a `switch` in one loop name a loop that
  is not there.
