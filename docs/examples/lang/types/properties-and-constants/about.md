A property is a variable that belongs to an object. A class constant is a value that belongs to the
class and never changes.

You write a property as `visibility [static] type $name`, with an optional default value:
`public int $size = 8;`. Each object has its own copy of every property that is not `static`.
A property must have a value before a program reads it. So the constructor must set every property
that has no default, on every path. If it does not, the program does not compile.

You write a constant as `visibility const type NAME = value;`, for example
`public const uint WIDTH = 5;`. Reading `Limits::WIDTH` gives a value of the type you wrote, here
a `uint`.

**Good to know:** the compiler works out a default value once, before the program runs. So it must
be a value written directly in the code, `[]`, an enum case or a class constant. Code that computes
a value belongs in the constructor.

The examples show defaults and a constructor, constants and a `static` property, and a small retry
policy. Example 04 shows a default that is computed, which does not compile.
