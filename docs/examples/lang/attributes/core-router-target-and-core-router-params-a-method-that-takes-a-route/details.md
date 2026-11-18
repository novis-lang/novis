You write `#[Core\Router\Target]` and `#[Core\Router\Params]` on the parameters of a method or a
constructor. On a method, a class, a property or a promoted constructor parameter, they are `E0874`.

A `Target` parameter has the type `callable`, a `callable(...)` type, or either one with `?`. A
`Params` parameter has the type `array` or `array<...>`, with or without `?`. Any other type is
`E0899`, and so is a variadic parameter.

A method has at most one `Target` and at most one `Params`. A second one is `E1000`. A `Params`
needs a `Target` in the same method, or it is `E1000` too. A `Target` alone is allowed, for a route
with no captures. Neither attribute takes arguments.

These are the checks at a call that writes the route method directly:

- A method with no `#[Core\Route]` is `E0754`. A method the class does not have is `E0309`.
- An array that leaves out a capture is `E0755`.
- A key that is neither a capture nor a `#[Core\Query]` parameter is `E0759`.
- A written value that the capture cannot convert is `E0772`, such as `"x"` for a `uint`. A value
  outside a set of allowed values, such as `"fr"` for `"en"|"de"`, is `E0772` too.

Each error names your method. A named argument, such as `route: Shop::show`, is checked in the same
way. An array in a variable is not checked. When the call leaves out the `Params` argument, only the
route is checked.

A route method in a variable is not checked while the program compiles. This is also true when one
`Target` method passes its parameter on to another one. `Core\Router::url` finds the route when the
program runs. It throws a `RuntimeError` when the method has no route.

A `private` route method passed from another class is `E0471`. An instance route method needs an
object. Inside an instance method of its own class, `Shop::show` uses `$this`. From anywhere else,
the route method must be `static`.

The route must be in your own program. A class from another module, under `autoload links`, is
`E0897`. Write that link with `Core\Router::url`.
