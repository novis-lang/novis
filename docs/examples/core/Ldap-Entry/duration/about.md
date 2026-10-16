Returns the one value of an attribute as a `Core\Time\Duration`, such as the domain's `maxPwdAge`
or `lockoutDuration`.

Active Directory writes a length of time as a negative number of 100-nanosecond steps. For example,
30 minutes is `-18000000000`. `duration` reads that number and returns a positive `Duration`, so
`toString` prints `30m`.

The result is `null` when the entry has no value for the attribute. It is also `null` for the
smallest possible number, because Active Directory writes it for "never". For example, a domain
where passwords never expire has `maxPwdAge` with that value.

For an attribute with several values, `duration` throws a `LogicError`. It also throws a
`LogicError` when the value is not a length of time, and the message names the attribute.
