The one directory every temporary directory a program asks for is created inside.

A program that needs scratch space asks for a temporary directory and puts its files in there. Every
one of those is created under a single root the runtime owns, and `[io] temp_root` is where an
operator says which directory that is — a fast disk, a volume with room on it, a path a container
mounts. When it is not set, Novis uses a `novis` subdirectory of the operating system's temporary
directory: `$TMPDIR/novis` (usually `/tmp/novis`) on Linux and macOS, `%TEMP%\novis` on Windows.
Novis creates that directory on first use, readable by the server's user only. An empty string
means the same as not set.

Owning the root outright is what makes the cleanup safe. When a script ends the runtime deletes the
directories it handed out, and when the server starts it clears whatever a crashed process left
behind — both of which it can only do because nothing else writes there. Pointing the root at a
shared directory full of other people's files and other people's symlinks is the mistake this key
exists to let an operator avoid, so it is the operator's alone, and a new root takes a restart
rather than a reload.
