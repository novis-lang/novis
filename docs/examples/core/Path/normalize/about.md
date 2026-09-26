Removes the `.` and `..` parts from a path, so the same place is always written the same way. It
reads both `/` and `\` as separators, and it never looks at the disk.

A `.` part means "this folder", so it is removed. A `..` part means "the parent folder", so it
removes the folder before it. `/var/www/../log/./app.log` gives `/var/log/app.log`. Repeated
separators are removed too.

A `..` at the start of a relative path stays, so `../shared` stays `../shared`. A `..` above a
root is removed, because a root has no parent. The empty path gives `.`. The result is written
with `Core\Path::SEPARATOR`.

The result can still start with `..`, so `normalize` does not make a path safe.
`Core\IO::within` checks that.

**The examples below** clean up a path, show the `..` parts that stay, and find the paths in a
list that name the same folder.
