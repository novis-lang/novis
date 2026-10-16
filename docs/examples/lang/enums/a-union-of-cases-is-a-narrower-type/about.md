Writes a type that accepts only some of an enum's cases.

Where a type is written you can name two or more cases of one enum with `|` between them, as in
`Mode::Read|Mode::Write`. A variable, a parameter, a property or a return type declared that way
accepts those cases and nothing else. A value typed as the whole enum does not fit, even when it is
one of the named cases while the program runs.

Such a value enters the smaller type with `as Mode::Read|Mode::Write`, which throws an error when
the value is not one of the named cases. `as ?Mode::Read|Mode::Write` returns `null` instead of
throwing.

**Good to know:** the smaller type fits everywhere the whole enum is wanted, so you never need a
conversion in that direction.

**The examples below** show the type on a parameter, the two conversions into it, and a web handler
that takes only the two requests it is written for.
