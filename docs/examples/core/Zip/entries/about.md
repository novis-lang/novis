Lists the names of the files and folders in a zip archive, in the order the archive stores them.

The archive is checked before any name is returned. If one name is unsafe, the whole call throws a
`ParseError` and you get no list at all. A name is unsafe when it starts at the root, such as
`/etc/passwd`, or when it goes up a folder with `..`. It is also unsafe when it is a symbolic link,
when it appears twice, or when it is inside more than 64 folders. Bytes that are not a zip archive
also throw a `ParseError`.

Each name is a `tainted string`. The person who made the archive chose it, so you check it before
you use it.

The archive can be `bytes`, a file that `Core\IO::open` opened for reading, an uploaded
`Core\Request\Part` or a `Core\Request\BodyStream`. A file is read in small pieces, so the whole
archive is never in memory. An upload is kept in memory whole while it is read.

**Good to know:** listing reads only the table of contents at the end of the archive. A large
archive lists as quickly as a small one with the same number of files.

**The examples below** list an archive file, catch an unsafe archive, and check an upload before
accepting it.
