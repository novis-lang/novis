The header checksum defends against **corruption**: truncation, bit rot, an interrupted write that
landed at a final path by bypassing the runtime's own writer. It is not, and is not claimed to be, a
defense against a hostile file placed by another local principal who can write the cache directory —
that principal computes a perfectly valid header and checksum over payload bytes of their own choosing.
A checksum answers "is this the file I wrote", never "should I trust whoever wrote it"; conflating the
two would be the actual security hole.

That threat is closed by a permission and ownership check, not a hash. **A world-writable cache
directory, or one not owned by the account the runtime process runs as, is never read or written**, the
same class of check `ssh` applies to `~/.ssh`, and it is `rule:config/ownership-is-the-trust-boundary`
applied to the cache directory. A directory that does not exist yet is checked at the nearest ancestor
that does, because that is the shallowest directory an attacker would have to write in order to fill
the slot the first store will create. The default directory is `cache/` in the data folder, so its
check stops at the private data folder and never reaches the binary's own directory above it.

The check is done once, before anything is read from the directory, and never per entry: a `Cache`
value is itself the evidence it passed. A directory that fails it is not a refusal to start: the
process runs with no cache and compiles every unit, the fallback every miss already takes. It is not
silent either when somebody chose the directory — a `file_cache_dir` written in the configuration and
refused prints one `warning:` per process naming the key, the path and the reason, because a bad entry
is invisible and a breached boundary an operator named is not. A refused default directory prints
nothing of its own, because nobody chose it; an unusable data folder has its own one warning.
Ownership that changes after the process started is not re-checked mid-run, consistent with every other
`System`-class directive (`rule:config/opcache-file-cache-directives-are-system`).
