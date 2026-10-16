Returns every attribute of an entry as an array. Each key is an attribute name, and each value is a
list of the attribute's values as `tainted bytes`.

Use `toArray` when you do not know the attribute names in advance, for example to show every field
of an entry or to save the entry as it is. When you know the name, `string`, `strings` and `bytes`
are easier.

A key is written the way the server wrote the name, which can differ from the way you wrote it in
`select`. Active Directory writes `mail` for `MAIL`, for example. The attributes are in the order
the server sent them, and that order is not fixed.

**Good to know:** the DN is not in the array, so use `dn` for it. The array is a copy. A change
to it does not change the entry.
