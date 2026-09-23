Makes a second file with the same content as the first. The first file does not change.

`Core\IO::copy` takes the file to read and the path of the new file. If a file already exists at
the new path, `copy` replaces it. The copy is a separate file: a later change to one of the two
files does not change the other.

The program needs two capabilities: `fs.read` for the file it reads and `fs.write` for the new
path. Without one of them, `copy` throws a `RuntimeError` and copies nothing. When the system cannot
copy, for example because the first file does not exist, `copy` throws an `IOError`. A copy of a
file onto itself also throws an `IOError`, and the file stays as it was.

This replaces PHP's `copy`.
