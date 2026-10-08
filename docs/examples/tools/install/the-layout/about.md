On a server you can choose four folders yourself: one for the `nvs` binary, one for the configuration, one for the compile cache and one for the logs.

Without your own folders, Novis keeps its configuration file and its compile cache in the data
folder, `.nvsdata` next to the `nvs` program. The folders can have any names and be anywhere.
Administrators write to the binary folder and the
configuration folder. The account that runs `nvs` writes to the cache folder and the log folder.
The simplest layout puts the other three folders inside the binary folder.

Put the binary folder on the `PATH`. Then `nvs` runs from any directory, and you give it the path
of `nvs.toml` with `--config`. The command `nvs init` writes an `nvs.toml` in which every key is
present and commented out. It never overwrites an existing file. Two keys in that file set the
other two folders: `file_cache_dir` in the `[opcache]` block and `target` in the `[log]` block.

**Good to know:** a path in `nvs.toml` is written with `/`, on Windows too.

**The example below** prints those two settings for each binary folder in its input.
