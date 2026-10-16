Reads a filter written as LDAP filter text, such as `(&(objectClass=user)(mail=*))`, and returns it as
a `Core\Ldap\Filter`.

Use it for a filter that you or an operator wrote, for example in a configuration file or a constant.
The text cannot be `tainted`, so text from a request does not compile here. To search for a value from
a user, build the filter with `Core\Ldap\Filter::equals` or one of the other functions of this class.

Text that is not a filter throws a `LogicError`. The message gives the position of the first wrong
character, counted from 1.

**Good to know:** the result works the same as a filter you built step by step. Its `toString` returns
text that `parse` reads back as the same filter.
