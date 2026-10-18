An interface has no properties. `public string $path;` in an interface is the error `E0254`. A
constant in an interface needs a type, as in any class: `public const MAX = 3;` is the error
`E0246`. A class may declare the constant again with its own value. A default method reads the
value of the class with `static::MAX`, and the interface's value when the class has none.

A `private` method with a body is a helper. Only the interface's own methods can call it. A call
from anywhere else is the error `E0435`. A class that replaces a default method can still call the
interface's version with `Greets::greet()`. A subclass calls its parent's version with `parent::`.
A default method cannot read a property, because the interface has none: `$this->name` there is the
error `E0313`. It calls a method instead.

A `static` method with a body is called as `Greets::kind()` or `Person::kind()`, and a class may
replace it. A `static` method without a body must be written by every class, like any other
method.

A class that does not write a required method is the error `E0449`. The message names the method
and the interface. A name after `implements` that is a class is the error `E0303`. A name after an
interface's `extends` that is a class is `E0303` too. An interface may extend several interfaces,
separated by commas. Two interfaces that extend each other are the error `E0305`. When a class
extends a parent class and also gets a default method of the same name from an interface, the
parent's method is used.

When two interfaces give a class default methods with the same name, the class must declare that
method itself. Otherwise it is the error `E0859`. Its body can call `A::h()` and `B::h()`.

`implements Greets by $voice` needs a property named `$voice`. A missing property is the error
`E0720`. The property's type must be the interface, or a class that implements it. A `?Greets`
property, an `int` property and a class that only has the same methods are all the error `E0720`.
The forward covers the methods without a body. A default method of the interface still runs its own
body, and calls the class's methods. The forward reads the property on every call, so assigning a
new object to it changes where later calls go. A `lateinit` property that is not set yet throws
`LogicError` on the first forwarded call. A `static` method, a method with a `...` parameter and a
method with an `inout` parameter cannot be forwarded. Each is the error `E0721`, and the class must
write that method itself. One `implements` list may mix interfaces with `by` and without it.

`trait`, `use T;` in a class body and `insteadof` are all the error `E0227`.
