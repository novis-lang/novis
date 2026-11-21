Stores values in the memory of this server process, so a later request can use what an earlier one built.

You write a value with `put` and read it back with `get`. Every request this server process runs
sees the same values. The store needs no configuration and no network, so it is fast. Its size
limit is 32 MB by default. When the store is full, it deletes the value whose last write is the
oldest.

Any value may be gone at any time, and a key that nothing wrote returns `null`. So write here only
what your program can build again.

**Good to know:** a restart empties the store. A second server process has a store of its own. If
another server must find the value, or your program needs it to be correct, use `Core\Cache::shared`.
In a test with `#[Test(at: ...)]`, a value's lifetime uses the test clock, so `Core\Test::advance`
ends it.

**The examples below** show a write and a read, a value with a lifetime, and a slow result that one
request fetches and later requests find ready.
