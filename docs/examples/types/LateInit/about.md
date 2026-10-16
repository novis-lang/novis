`LateInit` is an interface. A class implements it to give a `lateinit` property its value the first
time a program reads it.

The class writes one method, `initProperty(string $name): void`. A read of a `lateinit` property that
has no value calls `initProperty` with the name of the property. The method sets the property, and
then the read returns the new value. A later read does not call the method again.

If the property still has no value when `initProperty` returns, the read throws a `LogicError`. If
the method throws an error, the next read calls it again.

**Good to know:** one call to `initProperty` may set several properties. A write that gives a
property its first value inside `initProperty` does not run a `set` hook, `PropertyGuard` or
`PropertyObserver`.
