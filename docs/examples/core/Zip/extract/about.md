Writes every file of a zip archive into a folder and returns the number of files it wrote. The
folder is created if it does not exist, and so is every folder the archive names inside it. The
program needs the `fs.read` and `fs.write` capabilities for the folder.

The whole archive is checked before anything is written. An archive with one unsafe name, such as
`../config.ini` or a file inside more than 64 folders, throws a `ParseError` and writes no file at
all. `extract` never replaces a file:
a name that already exists throws an `IOError`.

A small archive can decompress to gigabytes. This is called a zip bomb. So `extract` has the same
two limits as `Core\Zip::read`, and the limits count all files together. When a file goes over
them, it throws a `ParseError`. The files written before it stay on the disk.

**The examples below** unpack an archive, show the limit for the whole archive, and install themes
that users upload.
