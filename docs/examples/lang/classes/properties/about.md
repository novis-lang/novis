A property is a named value that each object of a class has.

Every property declares its type and its visibility. A property without a default value must be
assigned in the constructor, on every path out of it. The compiler checks that, so every object has a
value for every property from the moment it is created. A default value must be a constant the
compiler already knows: a number, a string, an enum case, `[]` or `null`.

Three keywords change when a property gets its value. `readonly` means the constructor sets it once
and no later code can change it. `lateinit` means another part of the program sets it after `new`
returns. Reading a `lateinit` property before that first write throws an error. `static` means there
is one value for the whole class instead of one per object, and it starts every request at the value
you declared.

**The examples below** show the three shapes in that order: defaults and a constructor, then
`readonly`, then a `static` counter with a `lateinit` property beside it.
