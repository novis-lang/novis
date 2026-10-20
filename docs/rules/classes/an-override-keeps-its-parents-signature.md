A method a class or an interface declares again over one an ancestor declares is an override, and it
keeps a signature a call through the ancestor can use. Otherwise it is `E0404`, at the override:

- **The return type is the ancestor's or a subtype of it**, and a `: static` stays `: static`.
- **Each parameter accepts the ancestor's type of it or a wider one.** A `tainted` or `secret`
  qualifier is part of the type, so it is kept.
- **The parameter count is the ancestor's**, with `inout` and a variadic tail where the ancestor has
  them. A further parameter is refused even with a default, because a call fills a default from the
  signature it resolved, which is the ancestor's: the override would read a parameter nothing passed.
  A default the override writes on one of the ancestor's parameters is not used through the ancestor
  either; the ancestor's is.
- **`static` is the same, and the visibility is no narrower.**

A call through the ancestor's type is checked against the ancestor's signature and runs the override,
which reads its arguments and returns its result unconverted. So "fits" is assignability without the
`int` → `float` row, which converts where a value is stored: an override of `f(): float` that returns
`int` is refused, because the caller would read the `int`'s bits as a `float`.

The reserved interfaces are held to it the same way: `Stringable::toString(): string`,
`Comparable::compareTo`, `Parses::parse(tainted string $s): static` and `Iterator<T>::current(): T`,
whose `T` is the argument the class's own `implements` clause writes. `compareTo`'s parameter is
`self` (`rule:classes/comparable`), so the class itself is accepted there, and an ordering compares
the two run-time classes before it calls it (`rule:classes/comparable-is-same-class-only`). A subclass
that overrides `compareTo` again keeps its parent's parameter type. A direct `$price->compareTo($other)`
through the parent passes the parent's class, and no run-time check runs there.

A class that answers an interface's method with one it inherits from its superclass is checked the
same way, at the class's name, since that is the only declaration it wrote. A constructor is not an
override: `rule:classes/constructor-compatibility` owns what a constructor keeps.

**An `abstract` method has no body.** A body there is `E0106`: `abstract` says each subclass writes
the method, and a body would make it an ordinary method no subclass has to write.

Declined: PHP's rule, which lets an override add a required parameter in some versions and checks
others only at run time. Novis has no run-time check below the type system, so a call through the
ancestor would read the wrong type. Declined too: invariant parameters, which would refuse the widened
parameter a caller never notices.

`crates/nvs-types/src/conformance.rs`'s `check_method_overrides` is the check.
