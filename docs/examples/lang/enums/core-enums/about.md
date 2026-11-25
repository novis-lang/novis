The enums that come with Novis. They work exactly like the ones you write yourself.

`Core\Arr::Order` chooses the direction of a sort. It is declared in the class `Core\Arr`, whose
methods use it. `Core\RoundMode` chooses how a number is rounded. The `Core` reference lists the
rest.

A case is written `Core\Arr::Order::Desc`. You pass it to a method that needs it. You can keep it in a
variable of type `Core\Arr::Order` and compare it with `==`. You can use it in a `match` and convert
it to its number with `as int`.

Like your own enums, they have no `->value`. To add a method to one, write an `extend` block: a block
that adds members to a type from `Core`.

`as Core\Arr::Order` or `as ?Core\Arr::Order` converts a number back into a case. A method that takes
one of these enums accepts only its cases. A wrong value is an error when the program is compiled.

**The examples below** sort with `Core\Arr::Order`, round a bill with `Core\RoundMode`, then turn the
text a visitor sent into a sort order. Example 04 shows that a number in place of a case does not
compile.
