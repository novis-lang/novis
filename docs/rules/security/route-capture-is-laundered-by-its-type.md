Every capture in a route path corresponds to a parameter of the same name, and the parameter's
declared type is what the segment is converted to during matching. A failed conversion is **not a
match**: matching continues, and if nothing else matches the result is a 404 — which is what every
framework otherwise writes by hand as a digit constraint on the placeholder.

**A converted capture arrives unqualified.** A checked conversion launders
(`rule:security/taint-propagation`), so an integer or enum capture is a plain value, while a `string`
capture and a trailing-segment capture stay `tainted string`, because nothing about them was checked.
No new sink, no new launderer, no new rule — the existing one arriving somewhere useful.

**A regex constraint is not among the admitted types and never will be.** An application-authored
pattern over the request path runs before any rate limiting, which makes catastrophic backtracking an
unauthenticated denial of service; a closed set is spelled as a union of literal types or a subset of
an enum's cases instead.
