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

**The server checks its own configuration files, and a saved file is how a reload starts.** Every two
seconds, one thread off the request path takes the stamp (`mtime` and size) of every path the serving
tree read or probed: each root, each include, each included directory and each optional include that
was absent. A stamp that moved and then holds for one more check is a saved file, and the tree is
resolved and published under one lock, so two reloads never interleave two snapshots. A tree equal to
the one serving publishes nothing. A tree that does not validate is logged once for each distinct
refusal, with its file and line, and the running configuration stays. Nothing else starts a reload:
no signal and no service manager's control (`rule:packaging/a-service-answers-its-manager`). A server
whose roots are the shipped defaults read no file, and has nothing to check.

Cost: one `Arc` clone at request start and **no syscall** on the request path, and one `stat` per
configuration path every two seconds on the checking thread. Two snapshots live during a swap, plus
one per in-flight request still holding an older one — kilobytes each, bounded by concurrency, never
by reloads performed.
