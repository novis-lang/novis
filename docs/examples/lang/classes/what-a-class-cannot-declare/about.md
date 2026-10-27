Novis classes do not have magic methods, anonymous classes or nested classes. Each one is a compile
error, and the message names what to write instead.

No method name may start with `_`, so `__get`, `__set`, `__call`, `__invoke`, `__clone`,
`__destruct`, `__toString` and the rest cannot be written. A property hook does the work of `__get`
and `__set`. `PropertyObserver` sees every read and write of a class. `Stringable` gives a class
its text form. A `callable` comes from `fn` or from a method reference such as `Core\Str::length`,
so there are no callable objects. There is no destructor.

`new class { … }` does not exist. A `class` or an `interface` inside a class body does not
compile. Every class is declared at the top level of a file, with a name. An `enum` and a `type`
alias are the only types a class body can declare.

**The examples below** show a hook in place of `__get`, `PropertyObserver` in place of `__set`, a
named class in place of an anonymous one, and two classes that do not compile.
