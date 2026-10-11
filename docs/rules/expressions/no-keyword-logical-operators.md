`and`, `or` and `xor` are rejected at parse time, in every position, as `E0226`. Each keyword is
still recognised where an infix logical operator would go, so the diagnostic can name the fix, but
the expression becomes an error node: there is no low-precedence AND or OR in the tree at all.

`&&` and `||` are the only logical connectives, at every precedence, and `!` is untouched. The
diagnostic for `and` and `or` names the one-token replacement.

`xor` has none. There is no `^^` operator, so its diagnostic names the two rewrites instead —
`(a || b) && !(a && b)` for the general truthy case, or `a != b` when both operands are already
`bool`. That is a real, narrow capability subtraction rather than a spelling change.

All three keywords stay reserved words. They are not freed for use as identifiers, which is what lets
the diagnostic fire at all. A chain such as `$a and $b or $c` reports once per keyword: each keyword
consumes its own right-hand operand, so the error does not cascade into unrelated token errors.

The word forms are not synonyms for `&&` and `||`: they bind looser than `=`, so `$x = $a and $b`
assigns `$a` and discards `$b`. That precedence gap is a trap with nothing on the other side of the
scale, and keeping a second spelling of each connective is what `rule:statements/nothing-gets-a-second-name`
rules out everywhere else. Compatibility is with what a PHP program observably does, not with every
way PHP lets it be written.

**Declined:**
- **The keywords with PHP's precedence.** It would take a second short-circuit lowering at a third
  precedence tier, built and tested only to reproduce the trap above.
- **`and` and `or` as plain aliases of `&&` and `||`, dropping only `xor`.** A second spelling buys
  nothing once the meaning matches, the same reason `rule:types/no-legacy-cast` gives for `(int)$x`.
- **A deprecation warning first, removal later.** A refused PHP spelling is a hard error with no
  warning tier, as `rule:core-api/casing-has-no-suppression` reasons.
