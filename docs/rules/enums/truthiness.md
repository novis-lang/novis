An enum case in a condition is **always truthy**, whatever integer backs it. `Signal::Idle` backed
by `0` is truthy; so is a case backed by a negative value.

An enum follows the object row of the truthy table rather than the integer row its backing type
might suggest (`rule:enums/representation`). PHP's enum cases are objects, PHP has no mechanism for
making an object falsy, and so every ported `if ($status)` over an enum has only ever meant "always
true" — judging a case by its backing integer instead would silently change the branch that
condition takes.

Lowering reads this statically: a condition whose static type is an enum needs no truthiness helper
and no comparison at all.
