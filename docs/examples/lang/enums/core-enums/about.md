The enums that come with Novis. They work exactly like the ones you write yourself.

`Core\Arr::Order` chooses the direction of a sort. It is declared in the class `Core\Arr`, whose
methods use it. `Core\RoundMode` chooses how a number is rounded. Part B of the reference lists the
rest.

A case is written `Core\Arr::Order::Desc`. You pass it to a method that needs it. You can keep it in a
variable of type `Core\Arr::Order`, compare it with `==`, use it in a `match`, and convert it to its
number with `as int`.

Nothing about these enums is special. They have no methods and no `->value`, the same as your own.
`as Core\Arr::Order` or `as ?Core\Arr::Order` converts a number back into a case. A method that takes
one of these enums accepts only its cases. A wrong value is an error when the program is compiled.

**The examples below** sort with `Core\Arr::Order`, round a bill with `Core\RoundMode`, then turn the
text a visitor sent into a sort order.
