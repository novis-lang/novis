Lists the names of the files and folders in a zip archive, in the order the archive stores them.

The archive is checked before any name is returned. If one name is unsafe, the call throws a
`ParseError` and returns no list. A name is unsafe when it starts at the root, such as
`/etc/passwd`, or goes up a folder with `..`. A symbolic link, a repeated name and a name inside
more than 64 folders are unsafe too. Bytes that are not a zip archive also throw a `ParseError`.

Each name is a `tainted string`, because the person who made the archive chose it.

The archive can be `bytes`, an open file, an uploaded `Core\Request\Part` or a
`Core\Request\BodyStream`. A file is read in small pieces. An upload is kept in memory while it is
read.

**Good to know:** listing reads only the table of contents at the end of the archive, so a large
archive lists as quickly as a small one.

**The examples below** list an archive file, catch an unsafe archive, and check an upload.
