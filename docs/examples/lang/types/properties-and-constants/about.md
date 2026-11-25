A property is a variable that belongs to an object. A class constant is a value that belongs to the
class and never changes.

You write a property as `visibility [static] type $name`, with an optional default value:
`public int $size = 8;`. Each object has its own copy of every property that is not `static`.
The constructor must set every property that has no default, on every path. If it does not, the
program does not compile.

You write a constant as `visibility const type NAME = value;`, for example
`public const uint WIDTH = 5;`. Reading `Limits::WIDTH` gives a `uint`, the type you wrote.

**Good to know:** the compiler works out a default value once, before the program runs. So it must
be a value written directly in the code, an enum case, a class constant, or an array of these.
Code that computes a value belongs in the constructor.

**The examples below** show defaults and a constructor, constants and a `static` property, and a
small retry policy. Example 04 shows a computed default, which does not compile. Example 05 shows
constants that contain enum cases.
