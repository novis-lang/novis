A property key is a value that names one public property of a class.

You use a key when the name of a property is known only while the program runs. A sort column in a
web address and a field in a form are two examples. The type `property<User>` contains the names of
the public properties of `User`, and of the public properties it inherits. `as` converts a string to
a key. If the string does not name a public property, `as` throws an error. `as ?property<User>`
gives `null` instead. With a key, `$user->$key` reads or writes the property it names.

**Good to know:** the name of a `private` or `protected` property never converts to a key. A write
through a key does not compile when the class has a `readonly` property.

**The examples below** sort a table by a column from the web address, show a function that accepts a
key or any string, update the fields that a list of changes names, and update an object from a JSON
request body.
