Writes one log record. A record is one line of JSON. It has a level, a message and the file and line
that wrote it. You can also give `fields`: an array of extra values, such as an order number. They
are written as their own object next to the message, so a search for that order finds every record
about it. Inside a web request, the record also has the time and the id of the request.

Whoever runs the server decides where records go, with `[log] target`. When nothing is set, the
record is printed to the program's output. `[log] level` decides which levels are kept, and
`[log] format` can choose plain text instead of JSON.

The message may be text that a user sent. A line break in it is written as `\n`, so one record is
always one line. A `secret` value cannot be the message, and that does not compile.

**The examples below** write a record with and without fields, show how each kind of field value is
written, and log the rows an import job cannot read.
