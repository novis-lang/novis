A writer compiles, writes header and payload to `<cache_dir>/<key[0:2]>/.tmp-<random>` in the shard
directory the final name lives in, `fsync`s the temp file, and does one `rename` onto
`<key[2:]>.nvsc`. Rename is atomic on every platform Novis ships for, so no reader ever observes a torn
or partial file under the final name; the same directory is used so the rename cannot cross a
filesystem and degrade into a copy.

**If the final path already exists, this writer discards its own temp file and never overwrites.** The
key already includes the content hash, so another writer has by construction published byte-identical
content: there is nothing to reconcile because there is nothing that could differ. Two processes
racing to compile the same content each produce a valid entry and the survivor is whichever rename wins.

**No lock file, anywhere, ever, and no directory `fsync`.** The store is read and written by
independent, non-communicating processes, and correctness never depends on durability across a crash —
losing an un-synced entry means the next process recompiles it, which is a miss and not a defect, so a
sync on every write buys nothing. A failed write is likewise nobody's error: the caller drops it and the
run continues on the compile it just did.
