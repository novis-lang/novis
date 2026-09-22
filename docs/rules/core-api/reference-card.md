An implemented `Core` member's reference documentation lives in its registry declaration, next to the code
it documents: a short description of one or two sentences, a name and description per parameter — and for a
shape-typed parameter, each key's type and description — a return description, and a list of thrown errors,
each described. An enum carries a card of its own with one line per case, and a constant carries one
sentence, since a constant has a value and no signature. **A class carries a card of its own too**: one
or two sentences saying what the class is for, which is what a completion list shows beside the class's
name — the one place a reader meets a class before any of its members. A class that landed before
classes carried one is named in the registry test's list of classes still owing a card, and is deleted
from that list the session it gains one; a class added since lands with its card, as a member does.

The registry is the one artifact that provably matches shipped behaviour, because it is the data the
runtime dispatches on; and it already has to carry every parameter's name
(`rule:core-api/parameters-are-callable-by-name`), so a documentation scheme that put descriptions anywhere
else would create the duplicate that name guard exists to prevent. **Every row carries its card** — a
member without one fails the crate's tests, so a member lands documented or does not land.

**A compiler attribute carries a card too**, beside the checker's roster of recognized names: one
sentence saying what it does, the declaration it is written above, and its payload's fields at their
types — the card a hover on `#[Core\Route]`, the attribute's stub and a completion row all read. And a
class's hand-written intro page under `docs/reference/core/` is compiled in at build time and shown
under the class's own card by a hover and at the head of its stub; the page stays the website's, and
nothing about it is copied into Rust by hand.

Extended prose is deliberately excluded. Long-form text inside Rust string literals is the worst reading
surface available, so anything beyond the reference card stays in the website's pages. The cost is static
strings in the binary — per process, not per request, on the order of a few hundred bytes per documented
member — which is the cheap side of the trade and strippable behind a build feature if a deployment ever
cares.
