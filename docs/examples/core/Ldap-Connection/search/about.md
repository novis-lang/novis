Finds the entries in the directory that match a filter. It returns a `Core\Ldap\Entries`, and a
`foreach` loop over it gives one `Core\Ldap\Entry` at a time.

You build the filter with `Core\Ldap\Filter`, such as `Filter::equals('sAMAccountName', $login)`.
A value from a request can go into a filter as it is. The filter escapes it, so the value cannot
change what the filter matches.

The options say where and how to search. `base` is the DN the search starts from, and `scope` is
how far below it the search looks. `select` names the attributes each entry has. `sort`, `offset`
and `window` return one sorted page of a long list, and `total()` then returns the size of the
whole list.

The server sends the entries in pages of up to `pageSize` entries. The loop reads the next page
only when it reaches it, so a large result never needs all of its entries in memory at once.

**Good to know:** the values of an entry are `tainted`. A search that finds more than `sizeLimit`
entries throws `Core\Ldap\LdapError` inside the loop.
