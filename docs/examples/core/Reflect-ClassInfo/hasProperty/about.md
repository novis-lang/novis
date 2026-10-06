Checks whether a class has a property with a given name.

`hasProperty($name)` returns `true` when the described class has a property called `$name`, and
`false` when it has none. Properties the class inherits from a parent class count too. The name
must match exactly, so `Owner` does not find a property called `owner`.

`hasProperty` also returns `true` for a private property. That property still cannot be read from
outside its class. `get` throws a `RuntimeError` for a private property and a `LogicError` for a
name that does not exist, so `hasProperty` is how a program tells the two cases apart.

**The examples below** check a few names, tell a private property from a missing one, and check
the field names a client asked for before the program reads them.
