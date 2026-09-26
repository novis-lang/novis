Describes a class when you have only its name as a string.

`Core\Reflect::forClass` takes a class name, such as `"Shop\Order"`, and returns a
`Core\Reflect\ClassInfo`. That description has the class's name, its properties, its methods, its
constants and its attributes. When the running program has no class of that name, the result is
`null`. So you can use the same call to check whether a class exists.

Write the name the way the class declaration writes it, with its namespace and without a leading
backslash. The classes of `Core` itself are not described: the result for them is `null`.

**The examples below** show a class that exists and one that does not, a check of class names read
from a setting, and a header line built from a class's public properties.
