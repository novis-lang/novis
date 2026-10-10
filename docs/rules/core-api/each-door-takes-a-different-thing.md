Two members are not a second spelling of one operation when **each takes a different thing and answers a
different thing.** A payload signer takes a map and answers a token; a URL's own signing member takes a URL
and answers a URL; a router's signing pair takes a route identity and answers a path. None substitutes for
another, so none is the duplication `rule:core-api/one-paradigm-per-operation` refuses.

The test is on the *types at the boundary*, not on what the implementation shares underneath. Two members
may compile to the same construction and stay two members, and a class may be implemented as another plus
one more step without becoming the same member. Conversely, two members that take the same thing and answer
the same thing are one member with a flag, whatever they are named — that is the case R17 exists to catch.

A program that assembles a URL by hand, signs the text and hand-rolls the parameter walk has written the
URL member badly rather than reached it twice, which is the difference between a second door and a second
API.

Signing and sealing are two operations for the same reason. The signed cookie is authenticated
encryption, so its payload is **hidden**; a signed URL's payload must be **visible**, because the
parameters in the link are the point of it. One member with a flag would mean either readable cookies
or opaque URLs, so neither class loses a member to the other.

**Declined:**
- **The payload signer alone, with no doors.** It is the smallest surface, but the join from "I have a
  URL" to "I have a canonical payload of it" is the step that goes wrong, and leaving it to every caller
  leaves the bug class in place (`rule:core-api/signing-is-over-a-payload`).
- **The URL member alone, with no general signer.** Signing would then depend on the thing being a URL,
  which a cross-service token is not, and a program that uses neither `Core\Router` nor `Core\Uri` would
  have no way to sign at all.
- **Extending the signed cookie to cover URLs.** It keeps the roster one shorter, but its payload is
  encrypted and a URL's must not be.
