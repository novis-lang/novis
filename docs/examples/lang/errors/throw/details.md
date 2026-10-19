`throw "x"`, `throw 5` and `throw new Foo()` for a class not under `Throwable` are each the error
`E0780`. A variable whose type is an interface is `E0780` too, even when the object in it extends
`RuntimeError`.

A `?LogicError` variable may be thrown. When it is `null`, the `throw` throws a `LogicError` with the
message `Can only throw objects`. A `mixed` variable may be thrown as well. When its value is not a
`Throwable`, the result is the same `LogicError`.

`throw` as a value can stand after `??` and `?:`, in a `match` arm, in either arm of a ternary, after
`||`, and after `echo`. Its type is `never`. At a typed
variable or a method argument, `int $x = throw …;` is the error `E0401`. In arithmetic such as
`1 + throw …`, the result is `mixed`.

A method whose return type is `int` may end with a `throw` and no `return`. A method may also have the
return type `never` and always throw.

`throw LogicError("x")` without `new` is the error `E0320`.
