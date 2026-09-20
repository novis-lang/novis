Stores values in the memory of one CPU core, and reads them back without a network or a disk.

You write a value with `put` and read it back with `get`. A key that nothing wrote returns `null`, and
any value may be gone at any time. The store has a size limit, 32 MB for each core by default. When a
new value does not fit, the value written longest ago is forgotten.

Every core of the server has a store of its own, and one request runs on one core. The request after
yours usually runs on another core, where the value is not there. So write here only what your program
can build again at any time.

**Good to know:** if a `null` would make your program wrong, use `Core\Cache::shared`. If every core of
this server should find the same value, use `Core\Cache::process`.

**The examples below** show a write and a read first, then a value with a lifetime, then a table one
request builds and later requests find ready.
