An anonymous function written inside a method keeps that method's `$this`. It may read the class's
`private` properties and call its `private` methods.

A `static` method may also be called on an object: `$a->s()` calls `A::s()`. `$a::s()` is the error
`E0496`, because the left side of `::` must be a class name. An instance method called through the
class, such as `A::i()`, is the error `E0778`. `$this` inside a `static` method is the error `E0779`.

`self`, `static` and `parent` work only inside a class. Outside one, `self::class` and
`static::class` are the error `E0702`. `parent::` in a class that extends no class is the error
`E0303`. When the parent declares a constructor, the child's constructor must call
`parent::constructor(...)` on every path. A path without the call is the error `E0410`.

A method that returns `self` returns the class the method is written in. On a subclass object, the
result has the parent's type, so a method only the subclass has is the error `E0405`. Declare the
return type `static` to keep the subclass. `self` also works as a parameter type.

A method that returns `static` may return `$this`, `new static(...)` or the result of a `static::`
call. `new self()` or `new A()` there is the error `E0741`. `new self()` in a `static` method
always creates the class the code is written in, even when the call was made on a subclass.
`static::method()` reaches an `abstract static` method that a subclass writes.

`static::` inside an anonymous function is the error `E0834`. `self::m()` and `parent::m()` inside
one call the class the function is written in.
