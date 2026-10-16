Returns the whole number of `userAccountControl`, exactly as Active Directory stores it. Each flag of
the account is one bit of this number. A normal account that can log in is `512`, and the same
account when it is disabled is `514`.

`bits` returns every stored bit, also the bits no function names. It does not include `lockedOut`
and `passwordExpired`. Active Directory computes these two flags and does not store them.

Use `bits` when you need the number itself, for example to write changed flags back to the
directory. To check one flag, call the function for that flag, such as `disabled()`.
