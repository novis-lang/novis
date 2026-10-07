`Ldap\Dn` is the one launderer for the DN sink: it is built from an attribute type and a value that may be `tainted`, and escapes the value per RFC 4514. A DN is parsed by the server into a path in the directory tree, so a DN parameter is an instruction by `rule:security/sink-predicate`, and `rule:security/launderers-are-sink-named` puts its launderer in `Core`.

`Dn::of` and `$dn->child` take the attribute type, checked against RFC 4512's grammar, and the value.
Every member that takes a DN takes `Dn|string`, and the `string` arm refuses `tainted`; `Dn::parse`
refuses it too, for text an operator wrote. The launderer returns a carrier, not a string
(`rule:security/launderer-answers-a-carrier`), so a value laundered for a DN reaches only a DN
parameter. A DN the server returns comes back as a `Dn`, and `parent`, `rdn` and `isWithin` read one
without text handling. There is no `ldap_escape`, and no escaper for filter text either
(`rule:core-classes/ldap-filter-is-a-value`).
