Returns the size of a file in bytes.

`Core\IO::size` returns the number of bytes in the file at the path, as a `uint`. The operating
system gives this number, so the program does not read the file to get it.

The size counts bytes, not characters. Some characters take more than one byte. For example, "Café"
has four characters and five bytes. To count the characters in a string, use `Core\Str::length`.

When there is no file at the path, `size` throws an `IOError`. Check with `Core\IO::exists` first
when a missing file is a normal case.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `size` throws a `RuntimeError`.

**Good to know:** use `size` to reject a file that is too large before you read it into memory.
