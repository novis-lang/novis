`fields` is a **named, typed** structure inside `rule:errors/diagnostic-record`, never
`array<string, mixed>` at the core. The open type on `Core\Log::write`'s parameter is a call-site
convenience over it.

That matters for what it keeps possible. Because the names and their types exist in the model, a
later decision can have `nvs check` collect every call site, build the program's whole log schema at
compile time, and refuse two call sites that use one field name with two types. No mainstream logger
can do this, and the shape of the model is what leaves the door open.

Scoped context fields — a record inheriting an enclosing scope's fields — fit the same envelope
without changing it.

A record is charged to the request's budget, and a record shed under burst pressure **increments a
counter that is exported**, because a silently dropped log line is worse than a counted one.
