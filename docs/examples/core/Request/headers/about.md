Returns every header of the request in one array. Each key is the name of a header, written in lower
case, so `Content-Type` arrives as `content-type`. The keys are in the order the headers first
arrived.

The value of each key is a list of strings. A request can have the same header on more than one
line, and the list has one entry for each line, in the order they arrived. So no value is lost.
`Core\Request::header` returns one header with its lines joined into one string.

When the request has no headers, the array is empty. Every string in it is tainted, which means it
came from outside your program. A command-line program answers no request, so `headers` throws a
`LogicError` there.
