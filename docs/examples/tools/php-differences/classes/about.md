How a class is written in Novis, for somebody who knows PHP classes.

The constructor is named `constructor`, and `__construct` does not compile. Novis has no magic
methods. `__toString` becomes the `Stringable` interface with a `toString` method. `__get` and
`__set` become a property hook. `__invoke` becomes a closure. `__call` and `__destruct` have no
replacement. No name starts with `_`.

Every method, property and constant writes `public`, `protected` or `private`. Traits and anonymous
classes do not exist. Use an interface method with a body, or a named class. An enum has only
cases, written without the `case` keyword, and it has no methods. A class name uses `PascalCase`, a
method `camelCase` and a constant `SCREAMING_SNAKE_CASE`. Any other case is a compile error.

**Good to know:** a `readonly` property is written only in the constructor of its class.
Constructor promotion, `static::`, `abstract`, `final`, `clone` and `A::class` work as in PHP.
