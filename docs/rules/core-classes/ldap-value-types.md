Each LDAP attribute has one natural type, given by the server's schema plus a fixed table of Active Directory names, and a typed reader on `Ldap\Entry` converts only where nothing is lost. The principle is `rule:core-classes/db-column-types`'s: anything else throws, and text and bytes are `tainted`.

The schema gives the base type — AD's `attributeSyntax`/`oMSyntax`, or the subschema's `SYNTAX` OID on
any other server — read lazily, once per pool, and immutable after. The fixed table gives what the
schema cannot say:

| Attribute | Natural type |
|---|---|
| `objectGUID` and every GUID-valued AD attribute | `Core\Uuid`, the first three groups byte-swapped from AD's mixed-endian form |
| `objectSid`, `tokenGroups`, `sIDHistory` | `Ldap\Sid` (`S-1-5-21-…`) |
| FILETIME integers (`pwdLastSet`, `lastLogonTimestamp`, `accountExpires`, `lockoutTime`, …) | `?Instant`; `0` and `0x7FFFFFFFFFFFFFFF` are `null` |
| negative intervals (`maxPwdAge`, `lockoutDuration`, …) | `Duration` |
| GeneralizedTime | `Instant` |
| `TRUE`/`FALSE` | `bool` |
| `userAccountControl` with `msDS-User-Account-Control-Computed` | `Ad\AccountFlags` |
| `groupType` | `Ad\GroupType` |
| `sAMAccountType` | the enum `Ad\AccountType` |

**A flag field is a readonly object with one `bool` reader method per flag** and an immutable `with`
taking one `bool` option per flag it may set, where an option left out keeps its bit; `Core` has no
general flag-set type. Bits the object does not name are kept, so a write never drops them. Lockout and
an expired password are read from the computed attribute, and `mustChangePassword` from `pwdLastSet =
0`, both of which `search` requests whenever `userAccountControl` is selected, because AD does not
store them in `userAccountControl`; `with` cannot set them. A value is written in the form it is read, so every type above round-trips through
`modify` and is encoded the same way inside a filter. A ranged attribute (`member;range=0-1499`) is
fetched to its end and returned whole under its plain name. An attribute name matches without case and
keeps the server's case.
