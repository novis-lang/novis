The extension may hold no language logic — no parser, no formatter, no type table — and this is enforced
rather than intended: its `package.json` `dependencies` are checked against an allowlist by its own test
suite, so a second implementation cannot arrive as a dependency, and the reviewer is not the only thing
standing between the repository and one.

The same allowlist is what keeps the client free of language logic when a feature is added. The redaction
of `rule:security/redaction-ranges-come-from-the-server` is a range list from the server and a decoration;
there is nothing in it a parser would help with, and the test is unchanged by it.
