Writes every file of a zip archive into a folder and returns the number of files it wrote. The
folder and every folder the archive names inside it are created if they do not exist.

On a server, a program may write only in the folders that `Core\IO::temporaryDir` made for it. Any
other folder needs the `fs.read` and `fs.write` capabilities.

The whole archive is checked before anything is written. An archive with one unsafe name, such as
`../config.ini`, throws a `ParseError` and writes no file. A name that already exists on the disk
throws an `IOError`.

The archive can be `bytes`, an open file, an uploaded `Core\Request\Part` or a
`Core\Request\BodyStream`. `extract` reads one file of the archive at a time. An upload is kept in
memory while it is read.

A small archive can decompress to gigabytes, which is called a zip bomb. So `extract` has the same
two limits as `Core\Zip::read`, counted over all files together. A file over them throws a
`ParseError`, and the files written before it stay on the disk.

**The examples below** unpack an archive file, show the limit for the whole archive, and install
themes that users upload.
