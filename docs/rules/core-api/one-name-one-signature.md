One name has one signature. There is no overloading, and optional arguments are the only variance a
member's parameter list may have. Two behaviours therefore need two names, and which two is decided by the
verb lexicon (`rule:core-api/verb-lexicon`) and the symmetric-name rule
(`rule:core-api/symmetric-names`) rather than by whoever writes the second one.

This is the rule that keeps a family from regrowing. Where each new behaviour can be a new name in the same
shape, a library ends up with a dozen sort functions and a dozen substring searches; here it has to be a name that
predicts its own return type, or an enum-typed option on the member that already exists
(`rule:core-api/no-mode-strings`). Where two members genuinely take different things and answer different
things, that is not a second spelling (`rule:core-api/each-door-takes-a-different-thing`).
