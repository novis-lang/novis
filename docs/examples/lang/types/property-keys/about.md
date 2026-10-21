A property key is a value that names one public property of a class.

You use a key when the name of a property is known only while the program runs, such as a sort
column in a web address. The type `property<User>` contains the names of the public properties of
`User`, and inherited ones. `as` converts a string to a key, and throws an error if the string
does not name a public property. `as ?property<User>` gives `null` instead. `$user->$key` reads or
writes the named property.

**Good to know:** a `private` or `protected` property never converts to a key. A write through a key
does not compile when the class has a public `readonly` property.

**The examples below** sort a table by a column from the web address and update the properties a list of
changes names. Then they show a write of the wrong
type and a key of a parent class used on a child class.
