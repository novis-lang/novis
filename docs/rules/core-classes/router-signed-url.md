`Core\Router` keeps its own signing pair — one that mints a signed URL from a route name and its
typed parameters, and one that verifies against the match the server already made. Parsing a built
URL and signing it would reach most of this, and convenience alone would not justify two more
members.

**One property does.** A compiled route table may be served at one mount prefix or another, so a
signed *path* stops verifying the moment a mount moves. Signing the route **name** and its parameters
survives a remount, and verification reads the existing match rather than re-parsing anything.

**Verification is the application's, called by hand, wherever it keeps it.** Nothing verifies a
signature for you, because the program that renders the refusal is the program that should decide
when to ask.

**Not shipped.** `crates/nvs-stdlib/src/router.rs` carries neither member, and
`rule:core-classes/signature` — the payload half both would sign — is not built either.
