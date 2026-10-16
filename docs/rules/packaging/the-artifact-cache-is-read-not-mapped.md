The compiled-unit cache is **read into memory**, never mapped. A reader reads the whole file into a
buffer of its own with one read and closes it before the first check runs. A mapped file that is
truncated or replaced underneath a reader raises `SIGBUS` in the reader — a signal, which no
in-process mechanism contains: `catch_unwind` catches a panic, not a signal, and a signal in a worker
is the one thing left that can cost more than the request that provoked it
(`rule:errors/panics-bypass-user-code` is the boundary this sits outside of). A buffer is the
process's own, so nothing done to the file after the read reaches the unit, and reading the cache
needs no `unsafe` block.

The cost is one copy per unit at load, on the compile pool that single-flight compilation already
keeps off every request core, against a whole class of fault that has no answer once it fires. Memory
buys isolation, which is the direction the priority ordering exists to permit.

What the entry contains, how it is verified before it becomes executable, and why its key carries the
extension set (`rule:config/the-extension-set-is-in-every-unit-key`) are unchanged by how its bytes
arrive; only the read path is. The pages that run stay an anonymous mapping of the loader's own,
which no file can truncate.

**Memory:** one in-memory copy of each loaded unit's bytes for the process. It grows with the units a
program loads, never with requests served.
