A method parameter and an instance property may be declared again at a narrower type when the
declaration writes `covariant` after its modifiers and before its type:
`public function feed(covariant Dog $a)`, `inout covariant Dog $a`, `covariant Dog ...$all`,
`public covariant Dog $pet;` and a promoted `constructor(public covariant Dog $pet)`. A call or a
write through the ancestor's type can still pass the wider type, so each value that enters is
tested where it enters, and a value of the wrong type throws a `LogicError` that names both types.
A body and a read never see a value of another type.

`covariant` lexes as an identifier and is recognised only before a parameter's or a property's type,
like `spawn` and `type`, so a method or a variable named `covariant` keeps compiling.

**Where it is allowed.** On a parameter of a method that overrides a class's method or implements an
interface's, a static method and `Comparable::compareTo` included, and on an instance property that
redeclares a superclass's. Anywhere else it is a compile error with one code of its own, `E0882`: a method
that replaces nothing, a constructor parameter that is not promoted, an anonymous function, a static
property, a constant and a `readonly` property. A static property needs no keyword, because each
class names its own storage. A constant needs none, because it is only read. A `readonly` property is
not declared again at all (`rule:classes/a-redeclared-property-keeps-its-parents-type`): a subclass
passes the narrower value to the parent's constructor and adds a method with a narrower return type.

**Which types it takes.** Every type `is` tests in constant time: a class, an interface, an enum, a
scalar, and a nullable or a union of those. An `array<T>`, a shape, a callable, a `class<T>` and a
`property<T>` are refused with a code of their own, `E0883`, because their test walks the value and each call
would cost its size. The help says to take the wider type and test inside the body.

**Narrower is measured against the topmost declaration, and the keyword is written at every level.**
A parameter or property whose type is narrower than the first ancestor that declares it needs
`covariant`, also in a grandchild that only repeats its parent's narrowed type. Without it the
override is `E0404` and the property is `E0866`, and both helps name `covariant`. A `covariant` whose
type the topmost declaration's type fits is `E0882`, because it would cost a test that tests
nothing, and one that is unrelated to it is `E0404`. A grandchild's `covariant` type is also
measured against its parent's narrowed type: one that has no value in common with it is `E0404`,
because every call through the parent's type would throw. A `tainted` or `secret` qualifier is kept: the
keyword narrows the type, never a qualifier, and dropping one is `E0404`. A default the topmost
declaration gives a parameter must pass the narrower type, or the override is `E0884`, since a call
through the ancestor fills that default. `Comparable::compareTo` takes the class that names
`Comparable` (`rule:classes/comparable`), so that class's `compareTo` is the topmost one a
subclass narrows.

**A parameter is tested when the method is entered, on every call.** Each covariant parameter gets
one type test before the body runs: an `inout` one on entry, and a variadic one on each element. A
generator method is tested when it is called, before it returns its iterator. The test is the one
`is` compiles to. Its message names the method, the parameter, the type it takes and
the class it got. A call through the subclass's own type is tested too: skipping it saves one
compare and costs a second calling convention.

**A property is tested by each write that can reach it with a wider value.** The compiler sees every
class of a program (`rule:programs/no-runtime-autoload`), so it knows which properties some class
narrows, and a write to any other property is unchanged. A write to a narrowed one is tested unless
the receiver's static class is the narrowing class or below it and the value's static type is the
narrowed one. The class descriptor carries, for each slot some class narrows, the type that class's
instances take, and the tested write loads it from the receiver's descriptor. The slot stays the one
the topmost declaration has. An erased write through `object` or `mixed`, a `property<T>` key write
and a reflected write run the same test in the runtime. A hooked property tests before its `set`
hook runs, and the guard and observer pipeline runs after the test. A read is never tested.

**What it costs.** One class or scalar test per call of a method with a covariant parameter, and one
descriptor load and test per write that can reach a narrowed property from a wider type. Memory is
one type per narrowed slot in the class descriptor, held once per process, never per request.

Declined: Eiffel's narrowing with no keyword, which hides the run-time test at the override.
Declined: generic classes (`rule:types/no-user-generics`), and Scala's abstract type members, which
are generics under another name. Declined: a covariant `array<T>`, whose test costs the array's
length on every call. Declined: skipping the entry test for a call through the subclass's own type,
which needs a second calling convention to save one compare.
