The extension registers `.nvs` and does not claim `.php`, even though `nvs-syntax` parses it. Claiming it
would fight every PHP extension a user already has, and losing that fight silently looks like Novis being
broken.

A file named `nvs.toml` gets one completion provider and nothing else: no language id, file extension or
grammar for TOML. The provider's selector is the file name, so a TOML extension keeps the file and the
editor merges its completion with this one, and nothing is fought over. The provider asks the server's
`nvs/directives`, which answers the keys of the block the cursor is in, read from the default file the
binary ships. A workspace holding an `nvs.toml` activates the extension, so the answer is there before any
`.nvs` is opened. `nvs.toml` is where most users make their first mistakes, and an activation is not a
claim.

The extension-host run proves activation on `.nvs`, its absence on `.php`, and completion in an `nvs.toml`
whose language stays the editor's own. The manifest test proves no TOML file type is claimed.
