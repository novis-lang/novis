A property key is a value that names one public property of a class.

You use a key when the name of a property is known only while the program runs, such as a sort
column from a URL. A value of type `property<User>` is the name of a public property of `User`,
including inherited ones. `as` converts a string to a key, and throws an error if the string names
no public property. `as ?property<User>` gives `null` instead. `$user->$key` reads or writes the
named property.

An attribute is data written above a class or a property, as in `#[Search(fields: ['title'])]`.
When you declare an attribute, a field typed `property<static>` takes the name of a public property.
`static` is the class the attribute is used in. A wrong name does not compile.

**Good to know:** a `private` or `protected` property never converts to a key. If a class has a public
`readonly` property, no write through a key of that class compiles, whatever property the key names.

**The examples below** sort a table and update properties by name. Then they show two wrong
writes, a key of a parent class on a child object, and an attribute.
