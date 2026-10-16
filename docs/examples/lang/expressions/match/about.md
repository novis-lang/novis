`match` chooses one value out of several by comparing a subject against a list of values.

You write the subject in brackets, then one arm per line. Each arm has one or more values on the
left and a result on the right. The first arm with a value equal to the subject gives the result of
the whole `match`, and `default` is used when none of the other arms matched — it is tried last
wherever you put it in the list. A `match` is an expression, so you can assign it to a variable,
pass it to a function or join it to a string.

**Good to know:** if no arm matches and there is no `default`, `match` throws a `LogicError`. The
comparison is `==`, so an arm value of a type the subject can never equal — text against a number —
does not compile.

**The examples below** take these in turn: choosing a label for a value, choosing by a range with
`match (true)`, and writing the line a customer reads for each order status.
