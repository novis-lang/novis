Compiles a regular expression and returns a `Core\Regex\Pattern` that you can use many times.

Every `Core\Regex` method takes a pattern as a string or as a `Pattern`. Use `compile` when you need
an option, or when you want to check a pattern at one place in your program. There are four options:
`caseInsensitive`, `multiline`, `dotAll` and `ungreedy`. All four are `false` by default. A pattern has
no delimiters, so you write `^\d+$` and not `/^\d+$/i`. PHP writes the options as letters after the
last `/`.

**Good to know:** `compile` checks the pattern at once. An invalid pattern throws a `RuntimeError` at
that line, not later when you use it. A pattern cannot contain text from a request or another outside
source. To search for such text, pass it through `Core\Regex::quote` first.
