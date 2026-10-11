`nvs-lsp` and `nvs-fmt` are the only place completion, hover, diagnostics, rename, go-to-definition,
references and formatting of Novis are implemented. Every client of them is a thin adapter that decides
nothing about the language — not what a name resolves to, not where a line breaks, not even which range
is a `secret` (`rule:ide/redaction-ranges-come-from-the-server`). An editor client starts the server or
the formatter, translates its own editor's events into LSP requests, and renders what comes back.
`nvs code` builds `nvs-lsp`'s workspace index for one call and renders what `nvs_lsp::query` returns
(`rule:tooling/nvs-code-answers-about-the-project`).

The reason is the same one that gives every fact one home in the documentation, applied to executable
behaviour: two implementations of the formatting rules drift the first time one editor's plugin fixes a
bug the other's has not, and the verification that both editors produce byte-identical diagnostics and
formatted output for one file only holds while there is one implementation to agree with. A command
line that answered "who calls this" with a walk of its own would drift from the editor's references the
same way.

**Markup is the one exception, and it belongs to the editor rather than to the client.** Inside an
inline-HTML region the services and the formatter are the editor's own HTML ones
(`rule:ide/a-template-region-gets-the-editors-services-and-formatter`): the client forwards to them on
boundaries the server reports and implements neither. So byte-identical formatted output across the two
editors holds for the Novis bytes of a file; its markup bytes are each editor's HTML formatter's. The
client's format pass is its most intricate code, chunks, stand-ins, the base column and refusing an edit
that reaches a hole, and all of it is over text the server delimited and none of it is language logic.

The VS Code extension (`rule:ide/vscode-is-the-reference-client`) and the PhpStorm plugin
(`rule:ide/phpstorm-bridges-to-the-same-server`) are the two editor clients, and `nvs code` is the
third client, in the `nvs` binary. A dependency-allowlist test on the extension is what enforces "holds
no language logic" for an editor client rather than review.
