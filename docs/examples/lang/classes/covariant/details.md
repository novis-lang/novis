`covariant` is allowed on a parameter of a method that replaces a parent's method or implements an
interface's method. Static methods and `compareTo` are included. It is also allowed on an instance
property that the parent class already declares. Anywhere else it is the error `E0882`. That
includes a method that replaces nothing and a constructor parameter that is not promoted. It also
includes an anonymous function, a static property, a constant and a `readonly` property.

The type must be one that `is` can test quickly: a class, an interface, an enum, a scalar, or a
nullable type or union of those. An array, a shape, a callable, a `class<T>` and a `property<T>` are
the error `E0883`. For those, take the wider type and test the value inside the method.

The new type must be narrower than the type in the first class that declares the parameter or the
property. The same type or a wider type with `covariant` is `E0882`, because the check would test
nothing. A grandchild class that repeats its parent's narrower type writes `covariant` too. A
`tainted` or `secret` type keeps that word. If the first class gives the parameter a default value,
the narrower type must accept that value, or the method is the error `E0884`.

An `inout` parameter is checked when the method is entered. A variadic parameter checks each value,
and the error names the first one that fails. A property is checked on every write. That includes a
write through `object` or `mixed`, a `property<T>` key, `Core\Reflect` and the parent's
constructor. A property with a `set` hook is checked before the hook runs. Reading a value is never
checked.
