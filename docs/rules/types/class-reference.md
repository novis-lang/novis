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

The literal is the whole name because the same text built at run time gets no namespace and no
imports, and the two must name the same class; `E0303`'s help says so. It is read parenthesized or
not, as identifier segments separated by `\`, and a leading `\` does not parse
(`rule:statements/a-leading-separator-does-not-parse`). Loading it while compiling is what makes the
literal work: `rule:programs/no-runtime-autoload` closes the file graph before the program runs, so a
literal looked up at run time finds its class only when another line names it, and compiles cleanly
before it throws on its first run. Under `as ?class<T>` a literal is refused the same way, because a
decided operand has no run-time failure for the `?` to turn into `null`
(`rule:expressions/nullable-conversion-availability`).

```nvs
class<Shop\Animal> $c = 'Shop\Dog' as class<Shop\Animal>;   // loads src/Dog.nvs while compiling
?class<Shop\Animal> $d = $name as ?class<Shop\Animal>;       // null unless that class is loaded
```

A qualifier is stripped, as every checked conversion strips one, and here for a narrower reason than
that row's: the conversion's whole output range is the set of classes declared to be `Animal`s in this
program's own source. A tainted string cannot widen that set, name a class the source does not
declare, or reach outside the hierarchy the author wrote down. What survives is a choice among the
author's own subclasses, which is what a discriminator field is for. That bound is narrower than a
hand-written `match` over string literals gives, because the compiler cannot tell whether one arm of
the `match` was mistyped.

The type exists because the refusal alone is a dead end. A driver chosen by configuration, a handler
chosen by a discriminator field and a factory over a closed set of subclasses are ordinary programs.
Without a type, each falls back to a `match` the compiler cannot check is exhaustive, or to a table of
closures that erases the constructor. With one, the dynamic `new` and the dynamic static call are
typed call sites: arguments, arity and return type are checked. The cost is that `class<T>` and
`Foo::class` are visibly different things, and that distinction is what the safety rests on.

A run-time conversion is a walk up the named class's parent chain and interface set, as deep as the
program's own inheritance, once per conversion at the place the string arrives. A written-out name
skips it.

It is its own **equality domain**: two class references compare by descriptor identity, nothing else is
ever equal to one — `$cls == "Dog"` is exactly the string-as-a-class confusion this type keeps out —
and ordering one is refused with the other unordered types (`rule:types/ordering`).

- **Memory:** a class reference is one word pointing at a descriptor that already exists for the life
  of the process, so holding one frees nothing. Nothing is spent per request or per instance.

**Declined:**
- **Keep the refusal and offer nothing.** It makes ordinary programs unwritable and pushes authors to a
  closure table, which checks less than the syntax it replaces.
- **A `Core` function that takes a class name and a base as two strings.** It checks only at run time
  and gives back a value the checker cannot type, so the `new` site stays untyped. A type is the only
  shape that reaches the call site.
- **A bare `string` accepted at the three sites when the base is written beside it.** The check moves
  from the source to every use, so a string flows through the program unchecked and one forgotten
  annotation is PHP's behaviour (`rule:types/class-reference-sites`).
- **Resolving a string literal through the file's namespace and imports**, as `::class` resolves. The
  same text built at run time would name a different class, and the run-time lookup has no imports to
  apply.
- **A diagnostic code of its own for the literal.** Its mistakes are the ones `::class` already
  reports, and one code per mistake keeps `nvs agent show` to one page.
