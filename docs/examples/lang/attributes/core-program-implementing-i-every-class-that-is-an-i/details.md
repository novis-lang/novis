`I` can also be a class. Then the list has every class that extends it, and `I` itself when it is
not `abstract`. An `abstract` base class is a good `I` when the classes share code. A class that
implements an interface which extends `I` is in the list too. The test for each class is the same
as `$object is I`.

`I` must be an interface or a class that your program declares. An enum, a shape, a scalar,
`mixed` and `object` are `E0743`. A built-in interface or class, such as `Stringable` or
`RuntimeError`, is `E0743` as well. A name that nothing declares is `E0303`.

The call takes exactly one type argument, so `implementing()` and `implementing<I, J>()` are
`E0442`. It takes no value argument, so `implementing<I>(1)` is `E0402`.

A class whose constructor has a parameter without a default is `E0744`, and the message names the
class. A constructor whose parameters all have defaults is allowed, and each parameter gets its
default. To pass arguments to the constructors, use `Core\Program::constructors` instead.

The result is an `array<I>`. Storing it in an `array<J>` of another type is `E0401`, and `var`
gives the variable the type `array<I>`.

Each call runs every constructor again. Two calls return two different objects for each class, so
`==` between them is `false`.

The full class name includes the namespace, so `Alpha\Zebra` comes before `Middle`, and `Middle`
before `Zed\Apple`. The list has the classes of every file in the program. It also has the classes
under each `autoload` folder, even when no code names those classes or their files.
