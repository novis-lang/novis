Decides whether a regular expression may use the backtracking engine. You set it in `nvs.toml` as
the key `backtracking` in the block `[regex]`.

Most patterns run on a linear engine. Its time grows only with the length of the text. Lookaround,
such as `(?<=a)`, and backreferences, such as `\1`, need the backtracking engine. On some input, that
engine can take a very long time.

There are three values:

- `allow` runs these patterns on the backtracking engine. This is the default.
- `warn` does the same, and the compiler prints a warning for each such pattern it finds.
- `deny` does not allow these patterns at all.

With `deny`, a pattern written directly in `Core\Regex::compile` does not compile. A pattern built
while the program runs throws a `RuntimeError` at the call that uses it. Patterns without lookaround
and backreferences run as usual with every value.

A program cannot change this setting. `Core\Config::set('regex.backtracking', 'allow')` returns
`false`.

**The example below** runs a simple pattern, then catches the error from a pattern with lookaround.
