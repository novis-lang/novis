Decides whether a debugging dump also appears in the page, or only in the log.

`Core\Debug::dump` always writes its value to the log. With this key on, a dump made while an HTML
page is built is also added to the end of that page. A JSON response is never changed, so an API
returns the same data in both cases.

The default follows the mode the server runs in. Development adds the dumps to the page, and
production does not. A program cannot turn the key on for itself. So a dump that you forgot in
production code is only a line in the log. A program can turn it off for its own request with
`Core\Config::set('debug.inline', 'false')`.

When `[debug] store` is also on, the block in the page starts with the number of queries the
request ran. It also has a link that opens this request in the debug viewer.

**Good to know:** on the command line, a dump always goes to the error stream. This key has no
effect there, and the output of your program stays clean.
