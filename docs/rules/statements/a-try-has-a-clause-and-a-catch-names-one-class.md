A `try` has at least one `catch` clause or a `finally`, and a `catch` clause names exactly one class.
A bare `try { … }` guards nothing and is the shape a deleted clause leaves behind: it is `E0242`. PHP
refuses it too.

`catch (A | B $e)` is `E0245`. The binding carries one static type (`rule:types/declaration`'s `catch`
position), and a clause naming two classes has no single type to give it. A body shared by two classes
is written as two clauses, or as one clause naming a class they both extend. That is the shape
`rule:expressions/catch-expression` already requires of an expression arm, and the two refusals use
the same words.

Both are parse-time refusals in the rejected-PHP band, and each diagnostic names its rewrite.
