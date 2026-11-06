Writes every file of a zip archive into a folder and returns the number of files it wrote. The
folder is created if it does not exist, and so is every folder the archive names inside it.

A program you start from the command line may extract into any folder. On a server, a program may
write only in the folders that `Core\IO::temporaryDir` made for it. Any other folder needs the
`fs.read` and `fs.write` capabilities.

The whole archive is checked before anything is written. An archive with one unsafe name, such as
`../config.ini` or a file inside more than 64 folders, throws a `ParseError` and writes no file at
all. `extract` never replaces a file:
a name that already exists throws an `IOError`.

The archive can be `bytes`, a file that `Core\IO::open` opened for reading, an uploaded
`Core\Request\Part` or a `Core\Request\BodyStream`. A file is read in small pieces. `extract` reads
the table of contents and then one file at a time, so the whole archive is never in memory. An
upload is kept in memory whole while it is read.

A small archive can decompress to gigabytes. This is called a zip bomb. So `extract` has the same
two limits as `Core\Zip::read`, and the limits count all files together. When a file goes over
them, it throws a `ParseError`. The files written before it stay on the disk.

**The examples below** unpack an archive file, show the limit for the whole archive, and install themes
that users upload.
