Two grammatical refusals, each raised once and each naming the spelling that works:

- **`E0124`** — a `for` init clause holds one declaration or a list of expressions, not both. Raised
  where the second item begins, on `for (int $i = 0, $j = 1; …)` and on `for ($j = 1, int $i = 0; …)`
  alike. The help names both fixes: move the extra initialiser above the loop, or make both
  expressions.
- **`E0125`** — a `for` init clause declares at most one binding. Raised on the second declaration of
  `for (int $i = 0, int $j = 0; …)`. Separate from `E0124` because the fix differs: the second
  declaration goes above the loop.

Each refused header reports **one** diagnostic and no second one, and keeps whatever declaration it
held so no later phase reports an undeclared counter on top of it.
