Every failure the directory or the wire reports is one class, `Ldap\LdapError`, with `$kind`, an `Ldap\LdapError::Kind` a program branches on, and `$code`, the LDAP result code, or `null` where no server sent one. A malformed attribute name, DN or URL is the program's own `LogicError` or `RuntimeError`, and a pool with no free slot is an `IOError`, as for `Core\Db`.

The kinds are `InvalidCredentials`, `AccountDisabled` (AD `data 533`), `AccountLocked` (775),
`PasswordExpired` (532), `MustChangePassword` (773), `AccountExpired` (701), `NotAllowedNow` (530,
531), `EncryptionRequired`, `PasswordPolicy`, `NoSuchObject`, `AlreadyExists`, `InsufficientAccess`,
`ConstraintViolation`, `SizeLimitExceeded`, `TimeLimitExceeded`, `Referral`, `ReadOnly`,
`Unsupported`, `Unavailable`, `Timeout` and `Protocol`. One class with a kind lets a program branch on
the condition rather than on a result code, for the reason `rule:core-classes/db-error` gives
`Core\Db\DbError`.

An unknown user (525) and a wrong password (52e) are both `InvalidCredentials`
(`rule:security/ldap-pool-is-bound-as-its-block`). AD's policy refusal of a new password is
`PasswordPolicy`. **No message carries a password or a value of a filter**: a message is built from
the operation's name, the result code and the server's own diagnostic.

**Declined:**
- **`false` and a separate error number, as PHP's `ldap_errno` gives.** A failure throws, so a
  caller that forgets to test a return value cannot read a failure as a result.
- **Distinct kinds for 525 and 52e.** A user-enumeration oracle handed to every login form.
