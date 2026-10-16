Returns a description of an enum, found by the name of the enum.

An enum has no methods of its own. To read the list of its cases, or the value of one case, you
first get its description with `Core\Reflect\EnumInfo::of`. You pass the full name of the enum, and
`Status::class` gives that name. The result is a `Core\Reflect\EnumInfo`, or `null` if the program
has no enum with that name. The name of a class also gives `null`, because a class is not an enum.
The type of the result is `?Core\Reflect\EnumInfo`, so check it for `null` before you use it.

**Good to know:** `of` only needs a string. One function can work with every enum in a program.

**The examples below** describe an enum by its name, show the names that give `null`, and build the
choices of a form from any enum.
