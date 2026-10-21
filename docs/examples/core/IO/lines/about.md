Reads a text file one line at a time.

`Core\IO::lines` reads the file at the path and returns its lines. You walk them with `foreach`,
from the first line to the last. Each line comes without its line break.

A line ends at `\n` (Linux and macOS), at `\r\n` (Windows) or at a single `\r` (old Mac), so a
file from any system reads the same way. An empty line in the middle of the file is kept as an
empty string. A line break at the very end of the file does not add an empty last line, and an
empty file has no lines. You can walk the result more than once.

`lines` throws an `IOError` when the file does not exist or cannot be read. It throws an error
when the file is not valid UTF-8 text, because each line is a `string`.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `lines` throws a `RuntimeError`.
