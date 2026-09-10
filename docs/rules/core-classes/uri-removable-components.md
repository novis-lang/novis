Three of a URI's components become nullable — `port`, `query` and `fragment` — and writing `null` for
one in a `with` bag **removes** it, where omitting the key leaves it alone. The other three stay
non-nullable, because in each case what looks like a removal is a different operation wearing its
clothes: every reference has a path, and an empty one is `""`; clearing a host would silently clear
the port and user-info a caller did not mention, which is a cascade rather than a removal; and
dropping a scheme turns an absolute URI into a relative reference, which is a member's worth of
behaviour rather than a field's. A removal still goes through the same recompose-and-reread path
every other `with` takes, so a component that would move is the same throw a bad replacement already
is.

**A query parameter is the second level.** A URI's components are fixed and few; its parameters are
dynamic and many, and replacing the whole query string is what makes every call site parse, edit and
rebuild — which is where this area's two real bugs come from. A singular reader and a singular writer
close it, composing the three existing members so no second canonicalization exists to drift. A
`null` value removes the parameter, the bracket convention comes free because a value may itself be
an array, and removing the last parameter leaves no query at all rather than a bare `?`.

**The first level is shipped and the second is not.** `crates/nvs-stdlib/src/uri.rs`'s `with` takes
`port`, `query` and `fragment` as `?T` and its `removable` helper reads the three states; the two
query-parameter members do not exist yet, and that module's known gap 1 is what records them.
