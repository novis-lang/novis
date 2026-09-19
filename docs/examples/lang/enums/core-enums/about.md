The enums that come with Novis, which work exactly like the ones you write yourself.

`Core\Order` chooses the direction of a sort, `Core\RoundMode` chooses how a number is rounded, and
Part B of the reference lists the rest. A case is written `Core\Order::Desc`. You pass it where a
member asks for it, keep it in a variable typed `Core\Order`, compare it with `==`, use it in a
`match`, and convert it to its number with `as int`.

Nothing about these enums is special. They have no methods and no `->value`, the same as your own,
and a number converts back into one with `as Core\Order` or `as ?Core\Order`. A member that takes
one accepts only its cases, so a wrong value is caught while the program is compiled.

**The examples below** sort with `Core\Order`, round a bill with `Core\RoundMode`, then turn the
text a visitor sent into a sort order.
