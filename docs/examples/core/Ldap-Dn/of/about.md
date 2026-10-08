Returns a DN with one part, `attribute=value`, such as `DC=test`.

The value can come from user input. Each special character in it, such as `,`, `+` or `=`, is
escaped. So the value stays one value, and it cannot add a part to the DN. Use `child` to add more
parts below this one.

**Good to know:** the attribute name cannot be `tainted`. An empty value throws a `LogicError`, and
so does an attribute that is not a name.
