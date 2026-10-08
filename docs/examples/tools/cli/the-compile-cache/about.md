Novis stores compiled code on disk, so a program is not compiled again at every start.

`nvs run` and `nvs serve` both read and write the cache. The name of each entry comes from a hash
of the source and of the build environment. A changed source gets a new entry, so you never clear
the cache.

`[opcache] file_cache_dir` sets the directory. The default is the `cache` folder in the data folder,
`.nvsdata` next to the `nvs` program. `[opcache] file_cache = false` turns the cache off.

**Good to know:** the directory and its parent directory must belong to the account that runs
`nvs`, and no other ordinary account may write to them. If a directory that you set fails this
check, `nvs` prints one warning and compiles the program at every start.
