The extension forwards requests inside an inline-HTML region to VS Code's built-in HTML, CSS and
JavaScript language services, so the half of a `.nvs` file that is markup gets Emmet expansion, tag
closing and renaming, the colour picker, hover and validation. Since
`rule:programs/first-party-framework` makes inline HTML the template engine, that region is where a web
application's markup is written, not an edge case.

**The region list comes from the server**, as one request of Novis's own, `nvs/regions`, beside
`nvs/redactions`. The lexer already knows where a mode ends; the client does not re-derive it from a
grammar, for the reason `rule:ide/redaction-ranges-come-from-the-server` gives for redaction
ranges — a client that guesses is a second implementation of the lexer. Forwarding a request to a service
the extension did not write is not language logic in the client.

**Formatting is excluded, and this is the load-bearing half.** The embedded services are not registered as
formatting providers, and `editor.formatOnSave` in a `.nvs` file runs `nvs fmt` over the whole file and
nothing else. A second, configurable formatter inside a file whose formatter is unconfigurable by
decision would make `nvs fmt --check` fail for a second reason. `nvs fmt` treats an inline-HTML region as
any other span it does not reflow, so formatting a `.nvs` file with markup in it is byte-identical to
`nvs fmt`.

`nvs.template.services` (default `true`) disables the forwarding, because a user with their own HTML
tooling has to be able to get out of the way of ours.
