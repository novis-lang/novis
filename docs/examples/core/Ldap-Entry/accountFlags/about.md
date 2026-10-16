Returns the flags of an Active Directory account as a `Core\Ldap\Ad\AccountFlags`. Each flag is a
function that returns a `bool`, such as `disabled()` or `passwordNeverExpires()`.

Active Directory stores these flags as one number in `userAccountControl`. Select that attribute in
the search. The search then also reads two more values: `msDS-User-Account-Control-Computed`, which
says if the account is locked out or its password has expired, and `pwdLastSet`, which says if the
user must change the password.

If the entry has no `userAccountControl`, the result is `null`. A group and an organizational unit
have none. If the value is not a number, `accountFlags` throws a `LogicError` that names the
attribute.

**Good to know:** `bits()` returns the whole number, and `with` returns a copy with some flags
changed.
