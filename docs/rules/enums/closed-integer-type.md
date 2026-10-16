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

A closed set of named values is checked at the same boundary as any other typed input: untrusted
input becomes a case only through the one throwing `as` every other conversion uses
(`rule:types/conversion`), so there is no second per-enum API for a reviewer or an analyzer to know
about.

**Declined:**
- **A case as a singleton object, with methods and interfaces.** It allocates a case and carries
  identity, methods and interfaces for something an integer and a `match` already do
  (`rule:enums/no-class-machinery`).
- **Singleton case objects with no methods and no interfaces.** That keeps the allocation and drops
  the only features that would pay for it.
- **A conversion into an enum that always succeeds.** Turning any integer into an enum value, even one
  no case names, is the silent coercion `as` never performs; a value no case names throws.
