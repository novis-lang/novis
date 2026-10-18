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

**A card is text an end user reads**, because `nvs help`, a hover and the website print it, so it is
written to `AGENTS.md` § *Text an end user reads*. A card added or changed is written that way. An older
card in another voice is rewritten when its class is next touched, and no session sweeps every card.

Extended prose — long descriptions, examples, tips — stays in the website's pages, and only the card
moves into Rust.

- **Memory:** per process, never per request: static strings in read-only data, on the order of a few
  hundred bytes per documented member and a few tens per enum case or constant, plus each compiler
  attribute's card and each compiled-in intro page. That is the cheap side
  of the trade, and a build feature can strip it if a deployment ever cares.

**Declined:**
- **A design document as the documentation's only home.** It keeps the documentation far from the code it
  describes, and the registry has to carry every parameter's name anyway
  (`rule:core-api/parameters-are-callable-by-name`), so the names would live in two places.
- **Extended prose in Rust too.** Long markdown inside string literals, with its escaping and review
  noise, is worse to edit than a markdown page. The card is the part worth keeping beside the code.
- **Doc comments extracted through rustdoc's JSON output.** That format is unstable outside nightly, and
  a registry row is a struct literal rather than an item, so a doc comment does not attach to it.
- **A sidecar documentation file in the crate.** It is a second home beside the declaration, the same
  problem at a shorter distance.
- **An `intro` field on every class literal.** Each literal would carry `None` or a hand-typed path
  to its own page, a copy kept by hand. A lookup in a table the build generates from the pages
  keeps the literals and the pages as they are.
- **A class's card taken from its page under `docs/reference/core/`.** Not every class has a page, and a
  page is not data the runtime dispatches on. The card goes where the members' cards already are, and
  the page is shown beneath it.
