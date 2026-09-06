`and`, `or` and `xor` are rejected at parse time, in every position, as `E0226`. Each keyword is
still recognised where an infix logical operator would go, so the diagnostic can name the fix, but
the expression becomes an error node: there is no low-precedence AND or OR in the tree at all.

`&&` and `||` are the only logical connectives, at every precedence, and `!` is untouched. The
diagnostic for `and` and `or` names the one-token replacement.

`xor` has none. There is no `^^` operator, so its diagnostic names the two rewrites instead —
`(a || b) && !(a && b)` for the general truthy case, or `a != b` when both operands are already
`bool`. That is a real, narrow capability subtraction rather than a spelling change.

All three keywords stay reserved words. They are not freed for use as identifiers, which is what lets
the diagnostic fire at all.
