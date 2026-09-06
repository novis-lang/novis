The parsed configuration is one immutable snapshot behind an `Arc`. A request clones the `Arc` when
it starts and reads from that clone for its whole life, so a reload is never visible to a request
already running and no request ever observes half of one file and half of another. The per-request
half is unchanged: `Core\Config::set` writes an overlay over the clone
(`rule:config/a-runtime-set-is-request-local`).

Replacement is **validate-then-publish**, in this order, and any failure before the last step leaves
the running configuration completely untouched: read and parse the whole tree of files, where a
syntax error, an unknown key or a duplicate key ends it; verify every `[[extension]]` pin against the
file on disk, on every reload and not only the first; register the directives those extensions
contribute; validate the assembled registry; then compute `env_hash` and publish the new `Arc`. The
tree's ownership checks re-run on every file, so a file that became group-writable since boot refuses
the swap and leaves the previous snapshot serving.

Cost: one `Arc` clone at request start and **no syscall** — unlike source revalidation, configuration
is never polled from the request path; a reload is pushed by the operator. Two snapshots live during
a swap, plus one per in-flight request still holding an older one — kilobytes each, bounded by
concurrency, never by reloads performed.
