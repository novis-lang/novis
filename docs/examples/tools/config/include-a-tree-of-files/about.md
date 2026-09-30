`[[include]]` reads other configuration files into the file that contains it.

`path` names one file. `dir` names a directory, and Novis reads every `*.toml` file directly inside
it, in the order of the file names. Files in subdirectories are not read. Both keys are relative to
the directory of the file that contains the block. `optional = true` allows the file to be missing.

Novis reads all the files as one list of settings, and a later setting replaces an earlier one. An
included file can therefore change a value from the file that included it. `nvs config dump
--origin` shows which file set each value and which value it replaced.

**Good to know:** a missing file without `optional = true` is the error `E0605`. Two files that
include each other are the error `E0606`. Blocks with double brackets, such as `[[app]]` and
`[[schedule]]`, are collected from all the files. A plain value is replaced.
