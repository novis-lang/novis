Wherever a `Core` member admits a written `null` — a bag field or an ordinary argument — it means
**remove**. Clear, delete, not present in the result. It never means "restore a default", never selects an
alternative behaviour, and is never a flag under another name. An **omitted** key means the receiver's
existing value is carried through unchanged.

The position the `null` arrives in is incidental to what it means, which is why the rule is stated over a
member rather than over a bag field. A query builder that drops a pair whose value is `null` already
behaved this way before the rule existed — the only behaviour that round-trips, since a query string cannot
spell an absent value — and that it was arrived at independently is the best evidence it is right.

The rule's teeth are that **an input is made nullable only where removal is something the member can
actually do.** A member with no removal to offer keeps its non-nullable field, and writing `null` into it
stays the compile error it is today, so the nullability in a signature *is* the announcement that the thing
can be cleared and a caller reads it off the type rather than off prose. `""` is never a removal spelling
on any member: it is a legal value of most of these fields and is already distinguishable — an empty query
is not an absent one — so overloading it would reinstate the in-band sentinel this removes.

**Designed, not shipped.** `crates/nvs-stdlib/src/uri.rs`'s `written` helper still records the opposite:
with no second null to spend, `with` replaces and never removes.
