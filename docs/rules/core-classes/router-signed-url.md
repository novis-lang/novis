`Core\Router` keeps its own signing pair — `urlSigned`, which mints a signed URL from a route's method
reference and its typed parameters, and `signedRoute`, which verifies against the match the server
already made and answers that `Router\Match` or throws. Parsing a built URL and signing it would reach
most of this, and convenience alone would not justify two more members.

**One property does.** A compiled route table may be served at one mount prefix or another, so a
signed *URL* stops verifying the moment a mount moves. `urlSigned` signs the route's **path pattern**
— the mount-relative `#[Route]` `path`, such as `/unsubscribe/{reader}` — and its parameter values,
and verification reads the existing match rather than re-parsing anything. A remount and a rename of
the handler method leave old links valid. A changed path breaks them, which a changed URL does anyway.

**Verification is the application's, called by hand, wherever it keeps it.** Nothing verifies a
signature for you, because the program that renders the refusal is the program that should decide
when to ask.

**Declined:**
- **Signing the handler's `Class::method`.** A refactor that renames a method would invalidate every
  link already sent, such as an unsubscribe link in a mail, with no change to the URL a reader sees.
- **A signed link to another module.** `urlSigned` given a route under an `autoload links` root is
  `E0898`. A signature is checked with the key of the program that verifies it, so a program able to
  mint one for another program's route would cross the isolation boundary between the two. The
  module that declares the route signs its own links. `signedRoute` takes no route, so it has no such
  call.
