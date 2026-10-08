Builds a filter that finds the directory entries where an attribute has exactly this value.

You give the attribute's name, such as `sAMAccountName`, and the value to look for. Pass the filter to
`search` on a connection to get the entries it matches. The value is sent to the server as data. A `*`,
`(` or `)` in it is a normal character, so a value from a form cannot change what the filter finds.

The attribute's name must be written in your program, because it is not allowed to be `tainted`. A name
with a character that an attribute name cannot have throws a `LogicError`.

**Good to know:** the server decides if upper and lower case count. In Active Directory, most text
attributes ignore case, so `anna` also finds `Anna`.
