A constructor parameter carrying no visibility is a plain parameter, complete and correct, and the
missing-visibility error does not fire on it.

Visibility on a constructor parameter is not decoration — it *is* the promotion syntax. `function
constructor(int $n)` declares a parameter; `function constructor(public int $n)` declares a property.
Requiring a keyword on every parameter would delete the distinction, so the rule
(`rule:core-api/written-visibility`) is scoped to members. A promoted parameter is a member and is
therefore already written.

This is the negative case the rule could most plausibly break, which is why it is stated rather than left
to follow from the word "member".
