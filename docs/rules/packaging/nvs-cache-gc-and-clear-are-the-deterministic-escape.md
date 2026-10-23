An operator wanting deterministic control instead of a probability gets two explicit commands.
`nvs cache gc` is the same walk `rule:packaging/eviction-rides-the-cold-miss-at-a-probability` performs
on a miss, without the roll: it deletes oldest-first down to the floor if the store is over its cap, and
otherwise does nothing. `nvs cache clear` deletes every entry in the store.

Neither is ever needed for correctness. A content-addressed entry cannot be stale — a changed source, a
changed toolchain or a changed extension set is a different key — so the only thing either command
changes is disk usage, and the next run of anything they removed pays one cold compile.

**The store is the one a run reads.** Both commands resolve the configuration tree and act on the
directory the artifact cache resolves from it, `opcache.file_cache_dir` or the data folder's `cache/`.
A written directory the ownership check refuses prints the same warning a run prints, and the command
removes nothing, because a run then uses no store either. A tree that turns the cache off is the same
empty store.

**Only published entries go.** A `.nvsc` file is an entry. A `.tmp-` file belongs to a writer that may
still be running, so it is neither counted toward the cap nor deleted: deleting another process's
half-written entry is a race.

**An entry the platform keeps is skipped.** On Windows a file another process has open cannot be
deleted. The walk counts it in `skipped`, the command still exits `0`, and the next `gc` takes it, so
a scheduled job never fails over it.

**The output is one line**, `removed <n> entries, freed <bytes> bytes, skipped <k>`, and with
`--json` the document `{"schema":1,"removed":n,"freed_bytes":b,"skipped":k}` in that key order. Both
exit `0`, also when nothing was removed. Only a cache directory that cannot be read, or a tree that
does not resolve, exits non-zero.
