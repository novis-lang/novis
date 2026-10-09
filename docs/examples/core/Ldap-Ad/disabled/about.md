Returns a filter that matches the entries whose account is disabled. Use it with `search`, to find
the accounts that cannot log in, for example before you clean them up.

The filter tests the bit with the value `2` in `userAccountControl`. It matches when that bit is
set.

**Good to know:** `Ad::enabled` returns the opposite filter.
