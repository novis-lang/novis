A caught error gives you what it knows as properties, and there are no accessor methods.

Coming from PHP: `getMessage()`, `getPrevious()`, `getTrace()`, `getFile()`, `getLine()` and
`getCode()` do not exist. Write `$e->message` instead.

Every error has the same four properties. `message` is the text it was created with. `previous` is
the error that caused it, or `null`. `backtrace` is one line per function the throw came out of,
innermost first. `location` is the file and the line of the `throw`. A `ParseError` adds `issues`,
one entry per problem the parser found, each with a `path` and a `message`.

**Good to know:** there is no error code anywhere. The class of the error is what says what kind of
problem it is.

**The examples below** show where a throw came from, the list of problems in a parse error, and one
log line built out of a caught error.
