Deletes a file from a disk by its key.

The disk is a `[storage.<name>]` block in `nvs.toml`, and the `fs.write` capability must include its
folder. When `delete` returns, the file is gone.

If no file has the key, `delete` throws an `IOError`. A missing file usually means a wrong key, and
the error shows you that. If a missing file is fine for your program, catch the `IOError`.
