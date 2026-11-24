Finds the entries that changed since an earlier call. A program uses it to keep its own copy of the
directory up to date, without reading every entry each time. It uses DirSync, a feature of Active
Directory.

The first call has no `cookie`, so it returns every entry that matches the filter. After the
`foreach` loop, `cookie()` returns a cookie. Store it, for example in a database. The next call
gives that cookie, and returns only the entries that changed after it.

A changed entry has only the attributes that changed. `select` names the attributes to watch. A
deleted entry is in the result too, and its `isDeleted()` returns `true`.

**Good to know:** the cookie covers only the entries the loop read. If the loop stops early, the
next call returns the other entries again. `base` must be the root of the domain. A server without
DirSync throws `Core\Ldap\LdapError` with the kind `Unsupported`.
