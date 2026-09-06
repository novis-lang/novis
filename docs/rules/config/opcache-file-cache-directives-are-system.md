The on-disk artifact cache is governed by five directives in `[opcache]`: `file_cache` (bool, default
on), `file_cache_dir` (a path, root-owned, defaulting to a fixed per-build location), `file_cache_max_size`
(bytes), and the `file_cache_gc_probability` / `file_cache_gc_divisor` pair, which mirrors PHP's own
`session.gc_probability`/`gc_divisor` because eviction rides the cold-compile path at a small
probability rather than costing a warm hit anything.

**All five are `System`-class**, for the identical reason `opcache.validate` is: a script that could
redirect where the process reads "already-compiled, about-to-be-trusted" native code from would be
handing itself a code-injection primitive, not a performance knob. A request cannot tighten them
either — there is no safe direction for a key that decides which bytes become executable.

The consequence is a boot-time trust: a cache directory whose ownership changes after the process
started is not re-checked mid-run, consistent with every other `System` directive. What the cache
looks like on disk, how an entry is verified before it is mapped executable, and the refusal of a
world-writable directory are the packaging chapter's; this rule is only the roster and its class.
