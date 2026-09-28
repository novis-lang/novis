Reads a file from a disk by its key, and returns its bytes.

The disk is a `[storage.<name>]` block in `nvs.toml`, and the `fs.read` capability must include its
folder. If no file has the key, `get` returns `null`. This is not an error, so you can handle a
missing file with `??` or `== null`. A key that is not valid, such as one with a `/`, throws an
error.

**Good to know:** `get` reads the whole file into memory. To see which files exist without reading
them, use `Core\Storage::list`.
