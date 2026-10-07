An LDAP filter is an immutable `Ldap\Filter` value, encoded straight to BER, whose values are data and whose attribute names are a sink. It is made by static functions — `equals`, `startsWith`, `endsWith`, `contains`, `present`, `atLeast`, `atMost`, `approx` — and combined with `all`, `any` and `not`; there is no fluent chain, and no `<` or `>` because RFC 4511 has none. Base, scope, attributes and paging are search options, never part of the filter.

**The split follows `rule:security/sink-predicate`.** A filter on the wire is a BER structure, not
text, so a value is a framed octet string the server compares against and never parses: it accepts
`tainted`, and `*)(objectClass=*` or a NUL byte matches only itself. An attribute name selects what the
server evaluates, so it is a sink, checked against RFC 4512's attribute-description grammar, and
anything else throws `LogicError`. A value may be a string, bytes, an `int`, a `bool`, a `Core\Uuid`, an
`Ldap\Sid` or an `Instant`, encoded at search time by the table `rule:core-classes/ldap-value-types`
reads with, so a GUID filter matches the server's byte order.

**No filter escaper exists.** The builder is the protocol's own shape, the way a bound parameter is
SQL's, so an escaper would be a second, weaker answer. `rule:core-classes/db-one-api` keeps query
builders out of `Core` because SQL has a text form the server executes; LDAP has none on the wire, and
that rule is about SQL. `Filter::parse` reads RFC 4515 text an operator wrote, and its parameter refuses
`tainted`. `toString` renders that text for a log and is itself `tainted`. `Ldap\Ad` adds AD's matching
rules as filters: `memberOf` with `nested: true` (`1.2.840.113556.1.4.1941`), `bitAnd` and `bitOr`
(`.803`, `.804`), `enabled` and `disabled`.
