`enum Status { Active, Banned }` declares a new named integer type with a fixed set of cases, and
the enum's name is then a type like any other — a property, a parameter, a return, a local, a class
constant, an `array<T>` element, an `Iterator<T>` yield, a `foreach` binding. The `enum` keyword
appears only at the declaration site; every use site spells the enum's own name, exactly as a class
does.

A case is a compile-time constant of that type, never an object. There is no singleton to allocate,
no identity distinct from the value (`rule:enums/no-class-machinery`), and no per-isolate storage
slot to build or tear down (`rule:enums/representation`).

The set is closed, and closed means checked: a value no case names never becomes a case by default
or by silent coercion. `EnumName` is its own kind of type atom rather than a class reference, so a
name that resolves to an enum is not a class name and is never treated as one.
