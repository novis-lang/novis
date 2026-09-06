`language-configuration.json` is content, not a checkbox: comments (`//`, `#`, `/* */`), brackets,
auto-closing and surrounding pairs, `indentationRules`, `onEnterRules` continuing a `/** */` block, and
folding markers.

The one entry that is Novis-specific, and that a file borrowed from a PHP extension gets wrong, is that
**`wordPattern` must include `$`**. Without it, double-clicking `$total` selects `total`, every
rename-adjacent interaction is off by one character, and word-based completion suggests the wrong token.
