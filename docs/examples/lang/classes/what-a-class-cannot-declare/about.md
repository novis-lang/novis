Novis classes do not have magic methods, anonymous classes or nested classes. Each one is a compile
error, and the message names what to write instead.

No method name may start with `_`, so `__get`, `__set`, `__call`, `__invoke`, `__clone`,
`__destruct`, `__toString` and the rest cannot be written. A property hook does the work of `__get`
and `__set`. `PropertyObserver` watches every read and write of a class. `Stringable` gives a class
its text form. A `callable` comes from `fn` or from a method reference such as `Core\Str::length(...)`,
so there are no callable objects. There is no destructor.

`new class { … }` does not exist, and a `class` written inside another class body is refused. Every
class is declared at file scope with a name.

**Good to know:** you meet all of these when you compile, never while the program is running.

**The examples below** show a hook in place of `__get`, `PropertyObserver` in place of `__set`, and
a named class in place of an anonymous one.
