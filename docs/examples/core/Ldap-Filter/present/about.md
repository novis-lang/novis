Builds a filter that finds the directory entries that have any value for an attribute.

Use it to find every account that has a mail address, or, together with `Core\Ldap\Filter::not`,
every account that has none. The value itself does not matter, only that the attribute is there.

A name with a character that an attribute name cannot have throws a `LogicError`.

**Good to know:** `present("objectClass")` matches every entry, because every entry has an object
class. It is the usual filter when you want all entries below a place in the directory.
