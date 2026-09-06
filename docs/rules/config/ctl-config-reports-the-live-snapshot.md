`nvs ctl config --origin` answers the question the offline pair cannot: **what the running process
actually holds** — what the last reload published, including an `optional` include that has appeared
since boot, and every `Boot` key whose changed value was reported and left unapplied. It is the second
operation on the control socket, which the reload rule reserved for exactly this kind of read.

The output is `nvs config dump --origin`'s, taken from the live snapshot rather than from the files on
disk, so the two can be diffed: a difference between them is a reload that has not happened, a file
that changed since the last one, or a directory-mode change that will refuse the next one.
