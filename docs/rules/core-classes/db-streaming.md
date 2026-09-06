`stream` and `streamAs<T>` read a result set in constant memory, and they **hold the connection until
drained**. A second statement attempted on a streaming connection throws `LogicError` naming both
fixes: `->all()`, or a `{shared: false}` connection.

There is no `{chunk?: uint}` option, and its absence is a refusal rather than an unlanded feature.
The PostgreSQL portal is opened with a row count of *every row* and one `DataRow` is read per step, so
a streamed result already crosses on a single round trip while the client holds one row — the memory
a chunk size exists to bound is already one row, and the option could only spend latency to buy
nothing.

**Not shipped whole.** `stream` lands on PostgreSQL alone: the read needs the portal left open with
its state parked off the borrow, and the other four drivers have no such state, so the member throws
a `RuntimeError` naming `query` on each of them rather than buffering behind the caller's back.
`streamAs` is owed entirely. `crates/nvs-stdlib/src/db/mod.rs` is where that gap is recorded, and
`crates/nvs-db/src/pg.rs` holds the one driver that has it.
