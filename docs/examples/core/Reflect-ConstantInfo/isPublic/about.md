Tells you whether code outside the class may read a class constant.

`isPublic()` returns `true` for a `public` constant. It returns `false` for a `protected` or
`private` constant. Those constants are still in the list that `Core\Reflect\ClassInfo::constants`
returns, so you can see that they exist.

When `isPublic()` returns `false` and code outside the class passes that name to
`Core\Reflect\ClassInfo::constant`, `constant` throws a `RuntimeError`. Code inside the class can
still read the constant.

**The examples below** show which constants of a class are public, what happens when you read one
that is not, and how to print only the public settings of a class.
