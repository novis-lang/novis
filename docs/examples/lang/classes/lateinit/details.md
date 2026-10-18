In a class that implements `LateInit`, a method may read a `lateinit` property before any write.
This compiles, even in the constructor, because the read calls `initProperty`. In a class without
`LateInit`, the same read in the constructor is the error `E0428`. A class that implements
`LateInit` and has no `initProperty` method is the error `E0449`.

`initProperty` runs only for a `lateinit` property with no value. A read of a plain property never
calls it. A write before the first read gives the property its value, so a later read does not call
`initProperty`. `isinit($p->views)` tests the property and calls nothing. A compound assignment such
as `$p->views += 1` reads first, so it calls `initProperty` and then adds `1`.

`initProperty` may set more properties than the one it was called for. A later read of another
property it set does not call it again. A `lateinit` property that a subclass declares calls the
parent's `initProperty` too, with the subclass property's name.

Example 04 shows the message of the `LogicError` when `initProperty` returns and the property still
has no value. A read of a property `$v` inside its own `initProperty` call throws a `LogicError`
too. Its message is `` `$v` is read while `initProperty` initializes it. ``

Inside `initProperty`, only the first write of a property is plain. A second write of the same
property in the method runs `PropertyGuard`, a `set` hook and `PropertyObserver`, as every other
write does.
