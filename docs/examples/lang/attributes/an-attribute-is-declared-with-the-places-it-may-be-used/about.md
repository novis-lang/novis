An attribute attaches fixed values to a declaration. You declare it first, with
`attribute Table on class { name: string }`. That line gives the attribute's name, the places it may
be written and its fields. Then you write it as `#[Table(name: 'users')]` in front of a class.

The words after `on` list the places: `class`, `method`, `property` and `parameter`. An attribute
written in any other place does not compile. Novis also checks your values against the fields.

An attribute declared inside a class or an interface may be used only by the classes that extend or
implement that type. Outside that type, its name is written with the owner, as in `Job::Meta`.

Write `repeatable` before `attribute` to allow more than one use on the same declaration. A `///`
comment above a field describes it, and the editor shows that text.

An attribute is not a class. Novis creates no object for it, and every value is known while the
program is compiled.

**Good to know:** `#` on its own starts a comment. Only `#[` starts an attribute.
