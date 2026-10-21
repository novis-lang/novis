Reads the whole content of a text file into one string.

`Core\IO::read` takes a path and returns everything in the file. Nothing is changed: line breaks,
spaces and a line break at the end are all kept. An empty file returns an empty string.

The file must be UTF-8 text. For a file in another character set, use `Core\IO::readText`. To go
through a large file one line at a time, use `Core\IO::lines`.

`read` throws an `IOError` when the file does not exist, is a folder, or cannot be read. It throws
a `RuntimeError` when the file is not valid UTF-8, or when the file is larger than the
`[limits] max_output` setting allows.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `read` throws a `RuntimeError`.
