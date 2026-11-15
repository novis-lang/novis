A writer compiles, writes header and payload to `<cache_dir>/<key[0:2]>/.tmp-<random>` in the shard
directory the final name lives in, closes the temp file, and does one `rename` onto
`<key[2:]>.nvsc`. Rename is atomic on every platform Novis ships for, so no reader ever observes a torn
or partial file under the final name; the same directory is used so the rename cannot cross a
filesystem and degrade into a copy.

**If the final path already exists, this writer discards its own temp file and never overwrites.** The
key already includes the content hash, so another writer has by construction published byte-identical
content: there is nothing to reconcile because there is nothing that could differ. Two processes
racing to compile the same content each produce a valid entry and the survivor is whichever rename wins.

**No lock file, anywhere, ever, and no `fsync` of the file or its directory.** The store is read and
written by independent, non-communicating processes, and correctness never depends on durability
across a crash. A crash can leave the final name over a short or empty file whose rename reached the
disk before its data did. That file fails its length or checksum check, and
[`packaging/a-bad-cache-entry-is-a-miss-never-an-error`](packaging.md#packaging-a-bad-cache-entry-is-a-miss-never-an-error)
deletes it and recompiles, so a crash costs one compile. A sync on every write would make each first
run wait on the disk for that durability, and buy nothing. A failed write is likewise nobody's error: the caller drops it and the
run continues on the compile it just did.
