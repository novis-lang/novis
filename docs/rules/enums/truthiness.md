An enum case in a condition is **always truthy**, whatever integer backs it. `Signal::Idle` backed
by `0` is truthy; so is a case backed by a negative value.

An enum follows the object row of the truthy table rather than the integer row its backing type
might suggest (`rule:enums/representation`). A case is a value of its own type, not a number, and
judging it by its backing integer would make the branch `if ($status)` takes depend on which integer
a declaration happened to give the case.

Lowering reads this statically: a condition whose static type is an enum needs no truthiness helper
and no comparison at all.
