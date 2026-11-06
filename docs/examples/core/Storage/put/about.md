Saves data as a file with a name, on a disk that `nvs.toml` sets up.

The data can be `bytes`, an open file, an uploaded file, `Core\Request::bodyStream()`, or a list or
loop that gives `bytes` pieces. A file or an upload is written one piece at a time. If reading the
data fails, `put` deletes the file it started.

A disk is a `[storage.<name>]` block in `nvs.toml`. Its `root` is the folder that has the files,
and on a server the `fs.write` capability must include that folder. A program you start from the command line needs no
capability. The key is the name of the file.
It can use ASCII letters, digits, `.`, `-` and `_`, and it is at most 255 bytes long. A key with a
`/` or a `\` throws an error, so a program cannot write outside the folder.

If a file already has the key, `put` replaces it. With `overwrite: false`, `put` saves the file only
when the key is free. Otherwise it throws an `IOError`.

**Good to know:** the key may come from user input, but the disk name may not. You write the disk
name in the code.
