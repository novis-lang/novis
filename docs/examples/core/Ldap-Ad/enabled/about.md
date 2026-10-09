Returns a filter that matches the entries whose account is not disabled. Use it with `search`, to
find the users who can still log in.

The filter tests the bit with the value `2` in `userAccountControl`. It matches when that bit is
not set.

**Good to know:** an entry with no `userAccountControl`, such as a group, also matches. Combine the
filter with `Filter::equals('objectClass', 'user')` to find only users.
