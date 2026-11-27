Novis stores compiled code on disk, so a program is not compiled again at every start.

`nvs run` and `nvs serve` both read and write the cache. It has one entry for each source file, with
the code of each function in that file. The name of each entry is a hash of that code and of the
build environment. A changed function gives a new name, so you never clear the cache. After an edit
inside one function, only that function is compiled again.

`[opcache] file_cache_dir` sets the directory. The default is the `cache` folder in the data folder,
`.nvsdata` next to the `nvs` program. `[opcache] file_cache = false` turns the cache off.

**Good to know:** the directory and its parent directory must belong to the account that runs
`nvs`, and no other ordinary account may write to them. If a directory that you set fails this
check, `nvs` prints one warning and compiles the program at every start.
