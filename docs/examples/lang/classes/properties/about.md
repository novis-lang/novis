A property is a named value that each object of a class has.

Every property declares its type and its visibility. A property without a default value must be
assigned in the constructor, on every path out of it. A default value must
be a constant: a number, a string, a `bool`, an enum case, a class constant, `[]` or `null`.

Three keywords change when a property gets its value. `readonly` means the constructor sets it once
and no later code can change it. `lateinit` means another part of the program sets it after `new`
returns. Reading it before that first write throws an error. `static` means there is one value for
the whole class, and it starts every request at the value you declared.

**The examples below** show defaults, a constructor, `readonly`, `static` and `lateinit`. Others
show a constructor that does not compile, a `lateinit` read that throws, `isinit`, a `readonly`
write and `static::` on a property.
