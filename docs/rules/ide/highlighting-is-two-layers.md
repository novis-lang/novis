"Colour" is two deliverables because the two layers fail differently: the grammar has to be right before
the server has started, and the server has to be right about things a regex cannot see.

**Layer one, the TextMate grammar** (`editors/vscode/syntaxes/nvs.tmLanguage.json`), is what a file looks
like the instant it opens. It must cover, because each is a way Novis is not PHP and a borrowed PHP grammar
gets wrong: the dual-mode lexer's openers `<?nvs`, `<?php`, `<?=` and `?>`, with inline HTML outside them
highlighted as HTML; heredoc and nowdoc, with interpolation only in the former; type annotations everywhere
the grammar allows one, including the inline shape `{x: int}` (`rule:types/object-top`); the qualifiers
`tainted` and `secret`, and `decimal` as a scalar keyword (`rule:types/decimal`); `spawn`, `spawn script`,
`autoload`, `type`, `by`-delegation, property hooks and their `get`/`set` bodies; durations
(`rule:types/duration`); `#[...]` attributes distinguished from a `#` comment; and nothing Novis
rejects (`rule:ide/rejected-syntax-gets-no-colour`). Its test needs no editor: `vscode-textmate` plus
`vscode-oniguruma` tokenize a fixture and a snapshot freezes the scope of every span.

**Layer two, semantic tokens**, is where a compiler colours what a regex cannot know
(`rule:ide/semantic-tokens-carry-the-qualifiers`). Its test is a `.lspt` case per token type, plus the
extension-host run confirming the client's legend matches the server's.
