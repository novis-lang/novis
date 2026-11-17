The body of a doc comment is Markdown. `nvs meta --json` includes it, `nvs doc` shows it on a page,
and the editor shows it when you hover over the name.

A tag counts only at the start of a line. `@see` must name a class or member that exists, or the
program does not compile (`E0323`). `@example` must name a file under `examples/` or `tests/`
(`E0325`). Any other tag, such as `@since`, does not compile (`E0128`). `@returns` is an error too,
and the fix rewrites it to `@return`.

`@param`, `@return` and `@throws` need a sentence after them (`E0139`). The sentence runs to the end
of the line. The next `///` line continues it, unless that line is blank or starts with a tag. A
continuation line that starts with `- ` is a list item. Use it to describe each key of a shape
parameter. The editor and `nvs doc` show the list under that parameter. The compiler checks each tag
against the method under it:

- `@param $name`: `$name` is a parameter of this method (`E0335`). Only one `@param` describes it
  (`E0331`). Write `$name` without a type (`E0138`).
- `@return`: the method returns a value. A `void` or `never` method and a constructor take no
  `@return` (`E0333`).
- `@throws Name`: `Name` is a class that extends `Throwable` (`E0334`). It is found the same way a
  type name is, so `use` lines apply.

A tag must not start its sentence with the type from the signature, such as `@param $x int the
count` (`E0332`). `@param`, `@return` and `@throws` go only on a function, a method or a
constructor (`E0333`). The tags may be in any order. `nvs doc` and the editor show the parameters in
the order of the signature.

You do not have to describe every parameter. If the comment has an `@param` for some parameters but
not for all, the compiler prints the warning `W1030`. Its fix adds an empty `@param` line for the
missing parameter.

A `///` comment must be directly above a class, a constant, a property, a method or an enum case. If it
is above a statement, above a blank line or at the end of the file, the program does not compile
(`E0127`). Write `//` for a note to the reader.

`/** … */` is an ordinary comment. Directly above a declaration it documents nothing, and the compiler
prints the warning `W1011`. A line of four or more slashes is an ordinary comment too.
