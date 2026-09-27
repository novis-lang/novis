Returns the names of the properties that your code can read at the place where it calls this method.

`readableProperties()` returns an array of strings. Outside the class, the list has only the public
properties. Inside the class, the list has every property of the class. Inside a child class, the
list also has the protected properties. These are the same rules as for a normal property read, so
`get` can read every name in the list at the same place. It replaces PHP's `get_object_vars`.

The answer depends on where you call it, so the same description can return two different lists.
`properties` lists every property of the class, wherever you call it.

**The examples below** compare the list outside and inside a class, show what a child class can
read, and write a debug line with the readable values of an object.
