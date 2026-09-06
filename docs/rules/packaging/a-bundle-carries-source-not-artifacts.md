The embedded payload is the entry file's `.nvs` source plus every file its `require` graph statically
resolves to, laid out as a flat list of `(relative_path, length, bytes)` entries, entry first — no
archive format and no compression. Paths are written with `/` separators whatever platform built the
bundle, relative to the deepest directory every bundled file sits under, so a `require` inside the
bundle resolves to the file the build resolved it to.

**Source, not precompiled artifacts, deliberately.** An artifact's key folds the whole environment into
its address (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`), which is the right
design for a shared, revalidated disk cache and the opposite of what one portable file wants: a single
executable would need one artifact per target, CPU-feature set and compiler build it meant to support,
and every one of them would go stale against the next compiler bug fix until the author rebuilt and
redistributed. Source works on any target, duplicates nothing, and feeds the ordinary cache on the
user's machine.

The costs are accepted and named. A fresh machine's first run pays exactly the cold JIT compile any
first `nvs run` of an uncached file pays, and a second run is a plain cache hit. The app's source is
recoverable from the executable by anyone who looks, the same posture as every comparable ecosystem's
CLI bundling; hiding it is not a requirement, and neither encryption nor obfuscation is applied.
