`stream` and `streamAs<T>` read a result set in constant memory, and they **hold the connection until
drained**. A second statement attempted on a streaming connection throws `LogicError` naming both
fixes: `->all()`, or a `{shared: false}` connection.

There is no `{chunk?: uint}` option, and its absence is a refusal rather than an unlanded feature.
The PostgreSQL portal is opened with a row count of *every row* and one `DataRow` is read per step, so
a streamed result already crosses on a single round trip while the client holds one row — the memory
a chunk size exists to bound is already one row, and the option could only spend latency to buy
nothing.

**Both members answer on all five drivers**, and a driver never substitutes a buffer for a walk it
cannot park. This rule is the member's contract — constant memory, the connection held, the second
statement refused — and the read state each driver leaves between two steps is
`rule:core-classes/a-stream-parks-its-read-on-the-connection`, which is the mechanism under it.
