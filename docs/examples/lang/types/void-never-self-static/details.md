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

A method that returns `static` must return `$this` or `new static(...)`. `return new self();` is
`E0741`, because a child class may be the class the call names.

`self` and `static` are not values. `return self;` is the error `E0321`. To use a member of the
class, write `self::` and the member's name.
