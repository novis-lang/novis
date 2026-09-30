Returns the names of the fields of an attribute, in the order they are written.

An attribute has fields, and each field has a name and a value. For an attribute written as
`#[Cache(seconds: 600, shared: true)]`, `fields` returns `seconds` and `shared`. The result is an
array of strings. It has the names only. Use `field` to read a value.

`field` throws a `LogicError` for a name the attribute does not have. When a field is optional,
check the list from `fields` first. A framework can also compare the list with the names it knows,
to find a setting that somebody typed wrong.

**Good to know:** the list has every name, also the name of a field whose value `field` cannot read.
