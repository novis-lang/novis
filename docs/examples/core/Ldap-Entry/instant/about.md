Returns the one value of an attribute as a `Core\Time\Instant`, such as `whenCreated` or
`pwdLastSet`.

Active Directory writes a time in two ways. `whenCreated` and `whenChanged` are text, such as
`20240101120000.0Z`. `pwdLastSet`, `lastLogon` and `accountExpires` are a large number of
100-nanosecond steps since the year 1601. `instant` reads both and returns an `Instant`, a point in
time in UTC.

The result is `null` when the entry has no value for the attribute. It is also `null` for `0` and
for the largest possible number, because Active Directory writes these for "never". For example, an
account that never expires has `accountExpires` with one of these values.

For an attribute with several values, `instant` throws a `LogicError`. It also throws a
`LogicError` when the value is not a time, and the message names the attribute.
