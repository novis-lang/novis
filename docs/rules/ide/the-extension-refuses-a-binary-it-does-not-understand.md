`nvs lsp` reports its version at `initialize`. On a mismatch with the extension's own, the
`LanguageStatusItem` says so and the client does not start, rather than running and producing confusing
answers.

An old `nvs` earlier on `PATH` than the intended one is the single most likely support question this
extension will ever get, and it costs one comparison to answer it out loud. A client reporting a mismatched
version gets a refusal and a status item, not a session.
