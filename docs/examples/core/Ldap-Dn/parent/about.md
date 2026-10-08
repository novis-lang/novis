Returns the DN one level up, without its first part. For `CN=Ann,OU=Staff,DC=example,DC=test`
the result is `OU=Staff,DC=example,DC=test`.

Use it to find the group or the unit that an entry is in.

**Good to know:** a DN with only one part has no parent, and the result is `null`.
