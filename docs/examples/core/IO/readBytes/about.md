Reads the whole content of a file as `bytes`, exactly as it is on disk.

Use `Core\IO::readBytes` for a file that is not text, such as an image, a PDF or a ZIP archive.
Nothing is checked or changed, so every byte of the file is in the result. An empty file returns
empty `bytes`. To read UTF-8 text as a `string`, use `Core\IO::read`. For text in another character
set, use `Core\IO::readText`.

`readBytes` throws an `IOError` when the file does not exist, is a folder, or cannot be read. It
throws a `RuntimeError` when the file is larger than the `[limits] max_output` setting allows.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `readBytes` throws a `RuntimeError`.
