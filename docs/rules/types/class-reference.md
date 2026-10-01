`class<T>` is a type atom, written in every position a type is written, whose **value** is the run-time
class descriptor — the same word a `new static(...)` already allocates from, carrying the class's
name, parent chain, interface set and method table. `T` names one class or one interface; anything
else is `E0795` where it is written, and a `T` resolving to nothing is `E0303`.

**There is no other way to obtain one.** `Foo::class` is a `string` and stays one
(`rule:types/class-constant`), so no program acquires a class reference by accident. `as` is the only
source, and its rows sit in the one conversion grid (`rule:types/conversion`): a `string` must name
`T` or a class that is one, or it throws; a `class<U>` narrows against the descriptor at run time; and
`class<T> → string` is total. A name that resolves to nothing and a name outside `T`'s hierarchy are
the same failure and throw the same way, with the class named in the message. `as ?class<T>` yields
`null` exactly where it would throw.

A class name written out in the source is decided at **compile time** — `Dog::class as
class<Animal>` is a compile-time yes when `Dog` is an `Animal` and a compile-time refusal when it is
not — so the ordinary factory shape pays nothing at run time. A plain string literal under `as
class<T>` or `as ?class<T>` is written out too: its text is the class's whole name, with no `namespace`
or `use` applied, and the compiler loads that class through `rule:programs/autoload`'s map as it loads
`Dog::class`. A literal that names nothing is `E0303` and one outside `T`'s hierarchy is `E0708`, the
codes `Bogus::class` and `Rock::class` get. A class constant, a concatenation and a variable are values
built at run time, and they are checked when they arrive against the classes the program loaded.

```nvs
class<Shop\Animal> $c = 'Shop\Dog' as class<Shop\Animal>;   // loads src/Dog.nvs while compiling
?class<Shop\Animal> $d = $name as ?class<Shop\Animal>;       // null unless that class is loaded
```

A qualifier is stripped, as every checked conversion strips one, and here for a narrower reason than
that row's: the conversion's whole output range is the set of classes declared to be `Animal`s in this
program's own source. A tainted string cannot widen that set, name a class the source does not
declare, or reach outside the hierarchy the author wrote down.

It is its own **equality domain**: two class references compare by descriptor identity, nothing else is
ever equal to one — `$cls == "Dog"` is exactly the string-as-a-class confusion this type keeps out —
and ordering one is refused with the other unordered types (`rule:types/ordering`). A descriptor is
immortal and process-wide, so holding one costs a word and frees nothing.
