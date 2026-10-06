Returns the name of the class that a description is about.

`name()` returns the class name as a string, the same way the class declaration writes it. A class
in a namespace keeps its namespace, so the result is `Shop\Order` and not `Order`.

When the description comes from `Core\Reflect::forObject`, the name is the class of the object
itself. A variable with the type of a parent class can contain an object of a child class. `name()`
then returns the name of the child class. You can give the result to `Core\Reflect::forClass` to get
the same description again.

**The examples below** print the class of a few objects, show that the object decides the name and
not the variable, and write a log line that names the class of each job.
