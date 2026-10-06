`language-configuration.json` is content, not a checkbox: comments (`//`, `#`, `/* */`), brackets,
auto-closing and surrounding pairs, `indentationRules`, `onEnterRules` continuing a `///` run
(`rule:ide/doc-comment-authoring-is-the-editors-own`), and folding markers.

Two entries are where a borrowed configuration file goes wrong. The first is `onEnterRules`, which
in a C-family configuration continues a `/** */` block — in Novis an ordinary comment nothing reads
(`rule:tooling/doc-comment-is-three-slashes`) — so it carries a shape the language does not document
with and leaves the shape it does uncontinued.

The second is that **`wordPattern` must include `$`**. Without it, double-clicking `$total` selects
`total`, every rename-adjacent interaction is off by one character, and word-based completion suggests
the wrong token.
