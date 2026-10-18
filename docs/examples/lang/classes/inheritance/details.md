`extends` names one class. `extends A, B` is a syntax error. A class that is not declared is the
error `E0303`, and so is an interface after `extends`. A class that extends itself, directly or
through another class, is the error `E0305`.

When the parent has a constructor, every path through the subclass constructor must call
`parent::constructor(...)`. A call inside one `if` branch only is the error `E0410`. Code may run
before the call, and a second call runs the parent's constructor again. When the parent has no
constructor, the subclass does not call one, and `parent::constructor()` is the error `E0309`. A
subclass with no constructor uses the parent's constructor.

A subclass reaches the parent's `public` and `protected` members. A `private` member is the error
`E0471` in the subclass. `parent::` in a class with no parent is `E0303`. `parent::f()` runs the
nearest version above the class, so it reaches a grandparent when the parent does not declare `f`.
A default parameter value comes from the declared type of the variable. Say `A::f(int $x = 1)` is
replaced by `B::f(int $x = 2)`. A call on an `A` variable uses `1`, even when it contains a `B`.

A class that does not write every `abstract` method of its parent is the error `E0449`. An
`abstract` class that extends another one does not need to write them. `new` on an `abstract` class
or on an interface is the error `E0785`. `new static()` inside an `abstract` class compiles, because
it makes an object of the subclass the call was made on. A method without a body in a class that is
not `abstract` is the error `E0786`, and so is an `abstract` method there. A method cannot be both
`abstract` and `final`: that is the error `E0106`.

An `abstract static` method has no body, so a call to it must reach a subclass. `self::title()` and
`static::title()` in a static method call the class the method was called on. `Page::title()` on the
`abstract` class is the error `E0835`. `self::title()` inside an anonymous function is `E0835` too,
because it calls the class the function is written in.

A class that extends a `final` class is the error `E0783`. A method that replaces a `final` method
is the error `E0784`, also in a grandchild class. Other methods of the same class can be replaced as
usual.

`$x is T` is true for the object's own class, every parent class, and every interface they
implement. It is false for any other class, and false when `$x` is `null`. Inside the `if`, the
variable has the type `T`. The right side of `is` is a class name or a `class<T>` value. A `string`
there is the error `E0496`.

A `class<T>` value works in three places: `new $cls(...)`, `$cls::f(...)` and `$x is $cls`. A
`string` in any of them is the error `E0496`. `$obj->$name` with a string is the error `E0235`. A
static call through a `class<T>` value runs the subclass's method, and `static::` in it is that
subclass. `new $cls(...)` checks the arguments against the constructor of `T`. When a subclass of `T`
has a constructor that cannot take those arguments, the `new` is the error `E0794`. Convert to the
subclass, as in `as class<Strict>`, or give the subclass a constructor that fits. A class name
written in the code that is not a subclass of `T` is the error `E0708` at `as class<T>`.
