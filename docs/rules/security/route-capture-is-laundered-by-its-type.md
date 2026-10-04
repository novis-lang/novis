Every capture in a route path corresponds to a parameter of the same name, and the parameter's
declared type is what the segment is converted to during matching. A failed conversion is **not a
match**: matching continues, and if nothing else matches the result is a 404 — which is what every
framework otherwise writes by hand as a digit constraint on the placeholder.

**A capture typed at a class built from text is the one exception, and it fails the other way.** The
matcher narrows on the conversions it reads natively — `int`, `uint`, `decimal`, `Core\Uuid` and a
closed set — and a capture declared at any *other* class implementing `Parses` matches on **shape**.
Matching runs at the door with no program installed, so calling that class's `parse` there would put an
implementor's body over every request URL, including the ones that match no route, ahead of everything
that rate-limits it — the priority-1 objection this rule already makes to a regex, and a `parse` body is
strictly more than a regex. The class's `parse` runs at the binding site instead, where the match
crosses into the program, so a segment it refuses is a **`400`** over a route that did match rather than
a `404` (`rule:routing/a-bad-query-value-is-a-400`). Two captures spelled the same way therefore fail
two ways, and what decides which is who runs at the door.

**A converted capture arrives unqualified.** A checked conversion launders
(`rule:security/taint-propagation`), so an integer or enum capture is a plain value, while a `string`
capture and a trailing-segment capture stay `tainted string`, because nothing about them was checked.
A capture at a class built from text arrives as an instance of that class, which carries no qualifier
because `tainted` is a property of `string` and `bytes` and never of a class
(`rule:security/tainted-qualifier`) — an implementor that keeps the text in a plain `string` field is
refused by the assignability rule that already stands. No new sink, no new launderer, no new rule — the
existing one arriving somewhere useful.

**A regex constraint is not among the admitted types and never will be.** An application-authored
pattern over the request path runs before any rate limiting, which makes catastrophic backtracking an
unauthenticated denial of service; a closed set is spelled as a set of allowed values or a subset of
an enum's cases instead.
