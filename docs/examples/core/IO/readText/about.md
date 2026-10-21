Reads the whole content of a text file that may be saved in another character set.

A character set says how the letters of a text are stored as bytes. Older programs often save
files as Latin-1 or Windows-1252, and some Windows programs save them as UTF-16.

`Core\IO::readText` takes a path and a `charset` option with a `Core\Charset` case. It reads the
file and converts it to a normal Novis `string`. Without the option, the file is read as UTF-8.

The conversion is exact. When a byte in the file is not valid in that character set, `readText`
throws a `RuntimeError`. The message gives the position of that byte. No character is replaced or
left out.

`readText` throws an `IOError` when the file does not exist or cannot be read.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `readText` throws a `RuntimeError`.
