One member is the whole temporary-file surface: it hands the caller an owned directory, and the program
names files inside it with ordinary writes to paths it got back. There is no member that hands out a single
temporary file.

A program that needs one temporary file needs somewhere to put the second, so a per-file member is the
shape that gets called in a loop and leaves the cleanup question open at every call. A directory answers
both at once: it is one thing to own, one thing to sweep, and the paths inside it need no further
authority beyond the one the member already granted.

The cost is one extra join at each call site that genuinely wanted a single file.
