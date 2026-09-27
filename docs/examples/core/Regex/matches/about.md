Checks whether a pattern matches a text, and returns `true` or `false`.

The pattern can match at any position in the text. To check the whole text, start the pattern with
`^` and end it with `$`. The pattern is a `string` or a `Core\Regex\Pattern` from
`Core\Regex::compile`. Use a `Pattern` when you need options, such as matching upper and lower case
letters the same way. This replaces PHP's `preg_match` when you only need to know if there is a
match.

**Good to know:** `matches` only returns `true` or `false`. To read the matched text or its groups,
use `Core\Regex::match`.
