Every object carries the values its class declares, and every class can name the values that never
change.

A property is a typed slot on the object: who may see it, what type it holds, its name, and
sometimes a starting value. Every property must hold something before anything reads it, so one
without a starting value has to be assigned by the constructor — on every path out of it. That is
settled while the program is compiled, rather than turning up later as a read that found nothing
there.

A constant is a name for a value that is fixed when the program is built. It belongs to a class,
the way everything else does, and it is written with the type it has, so reading one hands you
exactly that type.

**Good to know:** a starting value is worked out once, before the program runs, so it has to be
something already known: a value written directly in the code, an empty list, an enum case, or
another class's constant.
Anything that has to be computed belongs in the constructor.
