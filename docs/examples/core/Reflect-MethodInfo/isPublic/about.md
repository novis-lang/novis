Tells you whether code outside the class may call a method.

`isPublic()` returns `true` for a `public` method. It returns `false` for a `protected` or
`private` method. Those methods are still in the list that `Core\Reflect\ClassInfo::methods`
returns, so you can see that they exist.

When `isPublic()` returns `false` and code outside the class passes that name to
`Core\Reflect\ClassInfo::call`, `call` throws a `LogicError`. The method does not run.

**The examples below** show which methods of a class are public, what happens when you call one
that is not, and how a small web router finds the pages a controller offers.
