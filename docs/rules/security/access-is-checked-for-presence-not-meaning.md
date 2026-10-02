The compiler verifies that a route's access attribute is there and that the name inside it resolves.
It never asks what the name means, never calls anything, and has no opinion about roles, policies or
sessions. An omission is an error rather than a public default — that half is
`rule:attributes/access-is-a-required-sibling`, and the payload's shape is
`rule:attributes/access-payload`.

**Interpretation belongs to whoever dispatches.** The declared name rides on the match the server made
as uninterpreted data, and the application's own dispatch reads and enforces it, once, above the
`switch` that calls a handler, so a route whose arm never mentions access is still checked.
**A dispatcher compares a case, never text:** `Core\Router\Match::accessAs<E>()` answers the decision
as a case of the enum `E` and `null` for any other decision, so a renamed case is a compile error at
the comparison and an admin route never reads as a public one. A case reaching `mixed` is only its
integer (`rule:enums/representation`), which is why the typed member names its enum rather than
answering `mixed`. `access()` answers the resolved name as a `string`, for a log line and for a class
constant, which has no enum. A type argument that is not an enum is **`E0841`**. **The server enforces
CSRF and nothing else** (`rule:security/csrf-is-on-by-default`): interpreting a role would need a
session, a user model and a role source, all three of which are deliberately outside the binary. Two
enforcement points is how a route ends up checked twice in development and not at all in production,
so there is one.

**The limit is stated rather than implied: the compiler guarantees the decision was *written*, not
that it was *honoured*.** An application that hand-rolls dispatch and never reads the access name gets
no enforcement from anyone. Closing that would require recognising a dispatch site, which is an
opinion the route table refuses to hold.
