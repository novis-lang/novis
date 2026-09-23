The one directory every temporary directory a program asks for is created inside.

A program that needs scratch space asks for a temporary directory and puts its files in there. Every
one of those is created under a single root the runtime owns, and `[io] temp_root` is where an
operator says which directory that is — a fast disk, a volume with room on it, a path a container
mounts. When it is not set, Novis uses a `novis` subdirectory of the operating system's temporary
directory: `$TMPDIR/novis` (usually `/tmp/novis`) on Linux and macOS, `%TEMP%\novis` on Windows.
Novis creates that directory on first use, readable by the server's user only. An empty string
means the same as not set.

The cleanup is safe because nothing else writes in this directory. When a script ends, Novis deletes
the temporary directories that script created. When the server starts, Novis deletes what a crashed
process left there. Do not set the root to a shared directory that other programs write to.

Only the configuration file can set this key. A program cannot change it. You can change it while
the server runs. The next temporary directory is then created in the new root. A directory created
before the change is still deleted when its script ends.
