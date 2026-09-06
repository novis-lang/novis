The PhpStorm plugin drives the same `nvs lsp` binary the VS Code extension does, through JetBrains
Platform's LSP client support — falling back to the community LSP4IJ plugin if the bundled API lacks a
needed feature; which of the two is left to a concrete evaluation when the work starts. Completion,
hover, diagnostics, rename and go-to-definition route through that bridge, and so does formatting:
"Reformat Code" runs `nvs fmt`, and PhpStorm's native Formatter framework and Code Style settings page
do not apply to `.nvs` files. That is the price of not forking the formatter into a second
implementation (`rule:ide/one-server-two-thin-clients`), named up front.

The plugin registers `.nvs` as its own file type (`rule:ide/nvs-is-its-own-file-type`) and ships a
TextMate-or-equivalent baseline grammar for the same instant-colour reason VS Code does.

It explicitly does not build a native PSI tree, PhpStorm-grade refactoring beyond what the LSP `rename`
request gives, structural search and replace, intention actions backed by its own inspector, or
PhpStorm's native debugger UI (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). Those are the
real quality gap between an LSP bridge and PhpStorm's PHP support, and a full native plugin is a later,
explicit decision if usage justifies it — kept open, not silently skipped, and not scheduled.
