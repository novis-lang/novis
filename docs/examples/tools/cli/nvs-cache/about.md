`nvs cache` deletes compiled programs from the compile cache.

`nvs cache gc` checks the size of the cache. If the cache is larger than
`[opcache] file_cache_max_size`, it deletes the oldest entries until the cache is at 80% of that
size. `nvs cache clear` deletes every entry. The next run of a deleted program compiles it again.

Both commands print one line with the number of deleted entries and the bytes they used. `--json`
prints the same result as one JSON document.

**Good to know:** a file that a run is still writing is not an entry, so neither command deletes it.
