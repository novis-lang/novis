Every expression kind and every control structure the grammar has is in exactly one of two lists in
`nvs fmt`. One list has the constructs with a broken layout, each naming the rule that gives it. The
other has the constructs whose line breaks are the author's. A test reads the node kinds from
`crates/nvs-syntax/src/walk.rs` and fails naming each kind that is on neither list, so a new construct
is never laid out by accident and never forgotten. What our own corpus happens to contain decides
nothing: a user writes what the grammar accepts.

The constructs with a broken layout are lists
(`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`), `->` call chains
(`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`), binary operator chains
(`rule:tooling/fmt-every-binary-operator-breaks-operator-first`), ternaries
(`rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch`), control headers
(`rule:tooling/fmt-every-control-header-breaks-like-a-condition`) and `match` arm lists
(`rule:tooling/fmt-novis-constructs`).

The constructs whose line breaks are the author's are assignment and every compound assignment, `=>`
between a key and its value, `is`, `as`, a unary operator, `new`, `clone`, `throw`, `return`, `yield`,
`await`, an index, and everything inside a string, a heredoc, a template or inline HTML. A line that
continues one of them starts one level in from the line the construct starts on. A construct that
needs a layout later moves from the second list to the first with the rule that gives it.

A kind that is not an expression or a control structure at all, such as a literal or a variable, is
on the second list too, because it has no line break of its own to lay out.

**Declined:**
- **Rules written only for the shapes the corpus contains.** The corpus has no broken `match` or
  `foreach` header, and users write both.
