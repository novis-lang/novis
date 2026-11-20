A property key is a value that names one public property of a class.

You use a key when the name of a property is known only while the program runs. An example is a
sort column in a web address. The type `property<User>` contains the names of the public properties of
`User`, and inherited ones. `as` converts a string to a key, and throws an error if the string
does not name a public property. `as ?property<User>` gives `null` instead. `$user->$key` reads or
writes the named property.

In an attribute declaration, a field typed `property<static>` takes the name of a public property
of the class the attribute is written on. A wrong name does not compile.

**Good to know:** a `private` or `protected` property never converts to a key. A write through a key
does not compile when the class has a public `readonly` property.

**The examples below** sort a table by a column from the web address and update properties by name.
Then they show a wrong write, a parent key on a child object, and an attribute that names properties.
