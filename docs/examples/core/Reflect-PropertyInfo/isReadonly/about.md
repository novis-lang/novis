Tells you whether a property is declared `readonly`.

`isReadonly()` returns `true` for a property declared `readonly`. It returns `false` for every
other property. A `readonly` property that a class inherits from its parent class returns `true`
too.

Only the constructor may write a `readonly` property. After that, `Core\Reflect\ClassInfo::set`
throws a `RuntimeError` for it. Checking `isReadonly()` first lets your program skip those
properties.

**The examples below** print which properties of a class are `readonly`, show a `readonly`
property from a parent class, and write new values into an object without its `readonly`
properties.
