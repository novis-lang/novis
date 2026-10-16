Checks whether an entry in the directory has a value. It returns `true` or `false`.

You give the entry, the name of an attribute and one value. The server checks the value and sends
back only the answer. The program does not read the entry, so this is a quick way to test one fact,
such as "is this person in the Sales department".

The server compares the value by the rules of the attribute. For most text attributes, upper and
lower case are the same, so `"sales"` matches `"Sales"`.

The value can be a `string`, an `int` or a `Core\Ldap\Dn`. A `Dn` is useful for an attribute that
contains the names of other entries, such as the `member` list of a group.

**Good to know:** an entry that does not exist throws `Core\Ldap\LdapError`. A list or `null` as the
value throws `LogicError`, because `compare` checks exactly one value.
