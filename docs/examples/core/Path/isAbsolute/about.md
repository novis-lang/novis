Checks whether a path starts at a root. It returns `true` or `false`, and it never looks at the
disk.

A root is where a path starts on its own, without a current folder. There are three kinds: a
separator such as `/srv`, a drive followed by a separator such as `C:\Users`, and a network share
such as `\\server\share`. A path that starts at a root is absolute. Every other path is relative,
and the empty path is relative too.

The check is the same on every platform, and it reads both `/` and `\` as separators. So
`C:\Users` is absolute on Linux too. `C:report.txt` is relative, because no separator follows the
drive.

**The examples below** check a few paths, show the Windows forms, and turn a relative path from a
configuration file into a full one.
