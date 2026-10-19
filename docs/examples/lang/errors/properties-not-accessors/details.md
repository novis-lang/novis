`getMessage()`, `getPrevious()`, `getTrace()`, `getTraceAsString()`, `getFile()`, `getLine()` and
`getCode()` are each the error `E0405`. The help names the property to read instead. For `getCode()` it
lists the four properties, because there is no error code. `$e->code` is the error `E0405` too.

All four properties are `readonly`. `$e->message = "…"` is the error `E0782`.

`location` is the empty string until the object is thrown. When the same object is thrown a second
time, `location` changes to the line of the second `throw`.

`backtrace` has one line for each method the throw left. A throw in the top-level code of a file has
one line, `<script>() at main.nvs:2`. An object that was never thrown has an empty `backtrace`.

`previous` is a `?Throwable`. `$e->previous->message` is the error `E0459`, because `previous` may be
`null`. Write `$e->previous?->message`.

`issues` exists on `ParseError` and its subclasses. On any other class, `$e->issues` is the error
`E0405`. A `ParseError` that your own code creates has an empty `issues` list.

`Core\Debug::dump($e)` prints the four properties. `echo $e` is the error `E0412`, and
`Core\Json::encode($e)` throws a `RuntimeError`. Build the text from `$e->message` and the other
properties.
