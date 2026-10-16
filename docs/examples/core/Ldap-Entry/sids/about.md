Returns every value of an attribute as a `Core\Ldap\Sid`, such as a user's `tokenGroups`.

A SID (security identifier) is the id Windows uses for a user, a group or a computer. `tokenGroups`
has the SID of every security group the user is in, also the groups the user is in through another
group. The directory computes it only when you `read` one entry, not in a `search`.

`sids` returns the SIDs in the order the server sent them. That order is not fixed, so sort them
when the order matters. If the entry has no value for the attribute, the result is `null`. If a
value is not a SID, `sids` throws a `LogicError` that names the attribute.

`sids` also reads an attribute with only one value, and returns an array with one item.
