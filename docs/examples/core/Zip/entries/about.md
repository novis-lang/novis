Lists the names of the files and folders in a zip archive, in the order the archive stores them.
It replaces PHP's `ZipArchive::statIndex` and `zip_entry_name`.

The archive is checked before any name is returned. If one name is unsafe, the whole call throws a
`ParseError` and you get no list at all. A name is unsafe when it starts at the root, such as
`/etc/passwd`, when it goes up a folder with `..`, when it is a symbolic link, or when it appears
twice. Bytes that are not a zip archive also throw a `ParseError`.

Each name is a `tainted string`. The person who made the archive chose it, so you check it before
you use it.

**Good to know:** listing reads only the table of contents at the end of the archive. A large
archive lists as quickly as a small one with the same number of files.

**The examples below** list an archive, catch an unsafe archive, and check an upload before
accepting it.
