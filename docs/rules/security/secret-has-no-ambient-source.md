Unlike `tainted`, **nothing in Novis grants `secret` ambiently.** The five host accessors are why
`tainted` can attach itself automatically — every one is a named, enumerable place untrusted data
enters (`rule:security/tainted-sources`). There is no equivalent list for secrecy: an environment
read, a session value, a database row and a string literal all look identical to the type checker
whether or not their content happens to be a credential.

`secret` therefore appears only where a developer spells it on a declaration — a parameter, return
type, property or local. A helper that loads an API key is expected to declare its own return type
`secret string`; the language gives no free ride.

**One member originates the qualifier**, and it is the exception that proves the rule: reading a
password at a terminal prompt with echo disabled answers `secret tainted string`, because the
*member's contract* is confidentiality and there is no reading of its result that is not a credential.
That is a declared return type, not an ambient grant.

The cost is stated: `secret` protects only what a developer remembers to annotate. That is a real
coverage gap relative to `tainted`, not an oversight.
