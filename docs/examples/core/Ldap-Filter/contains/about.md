Builds a filter that finds the directory entries where a value of an attribute contains this text
anywhere.

Use it when a person searches for a part of a name, a department or a description. The text is sent to
the server as data. A `*` in it is a normal character and not a wildcard, so a value from a form
cannot widen the search.

An empty text throws a `LogicError`, and so does a name with a character that an attribute name cannot
have.

**Good to know:** a search like this is often slow on a large directory, because many servers cannot
use an index for it. Combine it with a narrower filter in `Core\Ldap\Filter::all` where you can.
