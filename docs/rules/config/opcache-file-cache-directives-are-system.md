The on-disk artifact cache is governed by five directives in `[opcache]`: `file_cache` (bool, default
on), `file_cache_dir` (a path, root-owned, defaulting to `cache/` in the data folder, which Novis creates
private to the account running the compile), `file_cache_max_size`
(bytes), and the `file_cache_gc_probability` / `file_cache_gc_divisor` pair, a probability because
eviction rides the cold-compile path at a small probability rather than costing a warm hit anything.

`file_cache_dir` is the **only** spelling of where that cache lives, so every directive about the
artifact cache is a key of one block. `[cache]` is `Core\Cache`'s tiers and holds nothing about
compiled artifacts, so `[cache] dir` is `E0601` like any other key the registry does not know
(`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`). The directory is a trust
boundary, and a key that was accepted and read by nobody would leave an operator believing they had
moved, and checked, a directory they had not; the error names the file and the line instead. Like the
rest of `[opcache]` it reloads, and a new directory takes the next compile
(`rule:config/reloadability-is-its-own-field`).

**All five are `System`-class**, for the identical reason `opcache.validate` is: a script that could
redirect where the process reads "already-compiled, about-to-be-trusted" native code from would be
handing itself a code-injection primitive, not a performance knob. A request cannot tighten them
either — there is no safe direction for a key that decides which bytes become executable.

The consequence is a boot-time trust: a cache directory whose ownership changes after the process
started is not re-checked mid-run, consistent with every other `System` directive. What the cache
looks like on disk, how an entry is verified before it is mapped executable, and the refusal of a
world-writable directory are the packaging chapter's; this rule is only the roster and its class.

**Declined:**
- **`[cache] dir` as the primary spelling, with `opcache.file_cache_dir` as its alias.** Two names
  for one directory need a precedence an operator has to know, and the directory would sit in a
  different block from the keys that switch the same cache on and bound it. Neither name is one a PHP
  operator already has, since `[opcache]`'s keys are not `php.ini`'s either.
- **Both keys, documented as equal.** Two keys that can disagree need a rule for which one wins, which
  is the alias above with its precedence left unwritten.
