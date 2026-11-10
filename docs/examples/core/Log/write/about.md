Writes one log record with a level, a message, and the file and line that wrote it. Inside a web
request, the record also has the time and the request id. `fields` adds extra values as their own
object, so a search for an order number finds every record about that order.

`[log] target` sets where records go. With no target, they go to standard error. A server that runs
as a service writes them to `logs/nvs.log` in its data folder. A record is never part of the
program's output or of a web response. `[log] format` is `"json"` in production and `"text"` in
development.

In JSON, a line break in the message is written as `\n`, so a record is always one line. A `secret`
value as the message does not compile.

**The examples below** write records with and without fields, show each kind of field value, and log
the rows an import cannot read.
