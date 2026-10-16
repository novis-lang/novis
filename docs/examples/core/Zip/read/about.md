Reads one file out of a zip archive and returns its content as bytes, already decompressed.

You name the file exactly as `Core\Zip::entries` lists it. A name that
is not in the archive throws a `ParseError`. The whole archive is checked first, so an archive with
one unsafe name throws even when you ask for a safe file.

A small file in an archive can decompress to gigabytes. This is called a zip bomb. So `read` always
has two limits: the most bytes the file may have, and the most bytes it may have for each byte it
takes in the archive. A file over either limit throws a `ParseError`, and you get none of it. You
can make both limits smaller, but not larger than the server's settings.

The content is `tainted bytes`, because the person who made the archive chose it.

**The examples below** read two files, show both limits, and import the text files of an upload.
