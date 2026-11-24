The body of a doc comment is Markdown. `nvs meta --json` includes it, `nvs doc` shows it on a page,
and the editor shows it when you hover over the name.

A tag counts only at the start of a line. `@see` must name a class, method, property or constant that
exists, or the program does not compile (`E0323`). `@example` must name a file under `examples/` or `tests/`
(`E0325`). Any other tag, such as `@since`, does not compile (`E0128`). `@returns` is an error too,
and the editor's quick fix rewrites it to `@return`.

`@see` can also name a file. When the text after `@see` contains `/` or `.`, it is a
file, such as `@see notes/refunds.md`. The path starts at the folder of the file that contains the comment. The
file must exist, and it must be inside your project. The project is the folder of the file, or the nearest
folder above it, that has an `nvs.toml` or a `package.toml` file. An absolute path, or a `..` that
leaves the project, does not compile (`E0323`).

After a Markdown file, `#` and a heading name one heading:
`@see notes/refunds.md#who-can-get-a-refund`. The part after `#` is the heading text in lowercase.
Each space is written as `-`, and other punctuation is removed. Only a `.md` or `.markdown` file
takes a heading. If the heading is not in the file, the error suggests the most similar heading. The editor opens the file at that heading when you click the path.
A package must list in `[package] files` every file that a `@see` or `@example` names. If one is
missing, `nvs pkg pack` stops with an error.

`@param`, `@return` and `@throws` need a sentence after them (`E0139`). The sentence runs to the end
of the line. The next `///` line continues it, unless that line is blank or starts with a tag. A
continuation line that starts with `- ` is a list item. Use it to describe each key of a
parameter whose type lists named keys. The editor and `nvs doc` show the list under that parameter. The compiler checks each tag
against the method under it:

- `@param $name`: `$name` is a parameter of this method (`E0335`). Only one `@param` describes it
  (`E0331`). Write `$name` without a type (`E0138`).
- `@return`: the method returns a value. A `void` or `never` method and a constructor take no
  `@return` (`E0333`).
- `@throws Name`: `Name` is a class that extends `Throwable` (`E0334`). The compiler finds `Name` the
  same way it finds a type name, so `use` lines apply.

A tag must not start its sentence with a type. `@param $x int the count` does not compile
(`E0332`), because the method already declares the type. `@param`, `@return` and `@throws` go only on a function, a method or a
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
