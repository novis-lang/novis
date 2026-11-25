An `extend` block is declared at the top level of a file. Inside a class it is the error `E0233`. The name after `as` is the name of the block, in the namespace of the file. An autoloaded file declares one block and nothing else. Its file name is the name of the block, such as `BusinessDays.nvs`. `extend` is a keyword only at the start of a top-level statement. A method, a variable or a property may still be named `extend`.

The type after `extend` must be a class, an interface or an enum. A type alias, an attribute or another block is the error `E0336`. The type must come from `Core` or from another package. A type of your own package is the error `E0337`: add the member to the type itself.

A file sees the added members only when it has a `use` line for the block. An alias in the `use` line works too. Without it, a call of an added method is the error `E0405`, and a call with `::` is `E0309`. The help of the error names the `use` line to add.

A block may add these members:

- `public` and `private` methods, static or not. On an enum, `$this` is the case.
- Constants.
- A property with a `get` hook and no stored value. Every read runs the hook.
- A stored property. It needs a default value, a `?` type or `lateinit`. Only a class can store one.
- A nested `enum`, and a nested `attribute`. A block on an enum cannot add these two.

A block cannot add these members, and each one is the error `E0338`: a constructor, a `protected`, `abstract` or `final` member, a `static` property, a `readonly` property, a stored property with no default, and a new enum case. Inside a block, `parent`, `static::` and `new static` are `E0338` too. Write `self` instead of `static`. A static method of a block cannot use `yield`, and on an enum no method of a block can.

Inside a block, `$this` has the type that the block extends, and `self` names that type. The body sees the `public` members of the type, the members of its own block, and the members of other blocks its file uses. A `private` member of the type is the error `E0471`. A `private` added member is visible only inside its own block.

An added name must be new. Each of these is the error `E1011`, and the error names both places:

- The type or a parent class already has a member with that name.
- Two blocks add the same name to the same type.
- A class or an interface of the program extends or implements the type and declares the name.
- A later version of Novis adds the name to a `Core` type.

Some methods are called by name by the runtime. Examples are `compareTo`, `toString`, `parse`, `tryParse`, `format`, `iterate`, `current` and `key`. A block cannot add a method with such a name, and that is the error `E1012`. Write a class of your own that implements the interface instead. An added method never makes a type implement an interface, so `$x is Comparable` gives the same result as before.

A value whose type is a union finds an added method only when every type in the union gets it from the same block. Otherwise the call is the error `E1013`. Use `is` to check the type first.

A stored property cannot be added to an interface, to an enum, or to a `Core` class that does not allow it. That is the error `E1014`. The `Core\Time` classes, such as `Core\Time\Date`, do not allow it. Wrap the value in a class of your own instead. Before its first write, a stored property reads its default. A `?` property with no default reads `null`. A `lateinit` property throws an error when it is read before a write. A write of one array element through an added property, such as `$order->tags[] = 'x'`, is the error `E0478`. Read the array into a variable, change it there, and assign it back.

`clone` gives a copy whose added properties have their defaults again. A copy sent to a task starts the same way. JSON, `Core\Reflect`, `==` and the session store do not read added properties. `Core\Reflect` does not list an added method. A call of one through a `mixed` value or `object` throws an error, as for a method that does not exist.

A block for a generic `Core` class gives a new name to each type parameter, such as `extend Core\ObjectMap<K, V> as Index { … }`. A list that does not match the type parameters is the error `E0338`. So a block for one exact type, such as `Core\ObjectMap<Tag, int>`, does not compile.

A `[[restrict]]` entry in `package.toml` names an added method by its block, such as `'Acme\Billing\InvoiceDates::lateFeeFrom'`. A use where the entry does not allow it is the error `E0664`. An entry for the extended type does not cover the members a block adds. The code inside a block is checked as code in the namespace of the block. So a block cannot call a member that is restricted to its type.

`#[Core\Deprecated]` on a block or on an added member gives a warning at each use.
