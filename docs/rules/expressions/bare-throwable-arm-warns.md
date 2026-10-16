`W1006` warns on an expression-level arm that names `Throwable`, **binds no variable**, and whose body
is not a `throw`.

In the block form a `catch (Throwable)` has a body with room to log or re-raise. In the expression
form the body *is* the value, so an unbound arm over the root of the exception tree is by construction
*discard every failure, including the ones this site never anticipated* — the suppression operator
this language removed, regrown as a one-liner.

The warning names the two honest spellings: name the class the site expects, or bind `$e` and carry
it. It is a warning rather than an error because the hazard is a habit and not a type error, and
`nvs check` is where habits are named.
