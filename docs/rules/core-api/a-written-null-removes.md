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

`crates/nvs-stdlib/src/uri.rs`'s `removable` helper is the first reader of the three states, and
`Core\Uri::with` the first member to spend them: `{port: null}`, `{query: null}` and
`{fragment: null}` each clear their component, where omitting the key carries it over.

This is what lets a bag express a partial update, the shape every `with`-style member and every
PATCH-shaped API needs. It costs one new way to be wrong, the standing trade of merge-patch semantics:
`$u->with({fragment: $maybe})` with a `?string` computed elsewhere removes the fragment when that value
happens to be `null`. The trade is accepted. The nullability is written in the signature, so the
possibility is visible at the call site, and a field stays non-nullable wherever an accidental removal
would be dangerous rather than merely surprising. A member may still make a nullable key *required*
where a `null` has to be typed on purpose, as a signature's lifetime is
(`rule:core-api/a-lifetime-is-written`).

**Declined:**
- **A reserved constant per class meaning remove.** It is still in-band, still a value of the field's
  type, and one more name to learn per class. Nothing stops it being written into a field where removal
  means nothing.
- **A second bag key listing what to clear, such as `{clear: ["fragment"]}`.** It is stringly-typed against
  a key set the shape already declares, so a typo becomes a run-time throw where the bag gives a compile
  error. It also puts one component's fate in two places when a call both sets and clears.
- **Making `with` mutate the receiver.** Nothing mutates (`rule:core-api/nothing-mutates`), and removal is
  a syntax problem: an in-place setter taking the same bag could not express it either.
