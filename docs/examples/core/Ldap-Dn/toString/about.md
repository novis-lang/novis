Returns the DN as text, such as `CN=Ann,OU=Staff,DC=example,DC=test`.

Use it to write a DN to a log or to show it on a page. Each special character in a value is
escaped, so `Lee, Ann` is written as `Lee\, Ann`.

**Good to know:** the result is `tainted`, because a value in the DN can come from user input.
Spaces around `,` and `=` are not in the text.
