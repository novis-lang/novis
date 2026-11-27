`void` and `never` are allowed only as a return type. A parameter or a variable of type `void` or
`never` is the error `E0742`. `var $x = Helper::nothing();` is `E0742` too, because the
call returns `void`.

A `void` result is not a value. `echo Helper::nothing();` is the error `E0707`. Passing it to an
`int` parameter is `E0401`. A `void` method that returns a value is `E0403`.

A `never` method must not reach the end of its body. That is the error `E0739`. A `return;` in it is
`E0822`. Every path must throw, call `exit` or call another `never` method.

A call to a `never` method ends the caller's path. The caller needs no `return` after it. This is
true for a static method and for a method no child class overrides. A call to an overridden method
does not end the path, so the caller still needs a `return` there. Otherwise it is `E0739`.

`self` and `static` are only allowed inside a class. Outside one, `self` is `E0303`. `self` also
works as a parameter type and inside another type, such as `array<self>` or `?static`. `static` is
not allowed as a parameter type.

`static` is allowed inside a callable parameter type, such as `callable(static): bool`. Then
`Order::check(...)` takes a function for `Order`. Inside the method, `$rule($this)` and
`$rule(new static())` are checked when the program compiles. Any other argument to `$rule` is the
error `E1015`. If the method stores `$rule` or passes it to another function, that copy is a
`callable` with no parameter types. Its arguments are checked when it runs.

A method that returns `static` must return `$this` or `new static(...)`. `return new self();` is
`E0741`, because a child class may be the class the call names.

`$o is static` checks the class the call names when the program runs. In `Model`, called as
`Order::findAll()`, it is `true` for an `Order` and `false` for a `Customer`. Inside
`if ($o is static)`, the method may return `$o` as `static`. After a write to `$o`, it may not.
`static` inside a larger tested type, such as `$xs is array<static>`, is the error `E1021`. Test each
element with `is static` instead. Inside an anonymous function, `is static` is the error `E0834`.

A variable may also have `static` in its type, such as `array<static> $found = [];`. The method may
return it. Each value written to it must be `$this`, `new static(...)`, `null`, a variable inside
`if ($x is static)`, or a call that returns `static`. Any other value is the error `E1022`. Passing
the variable to an `inout` parameter is `E1022` too, because that method may write any object to it.
In a `final` class, `static` is that class, so these variables accept any object of it.

`self` and `static` are not values. `return self;` is the error `E0321`. To use a member of the
class, write `self::` and the member's name.
