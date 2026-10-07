`nvs ext inspect`, `nvs ext verify` and `nvs ext pin` read a `.nvsx` file before you load it.

`nvs ext inspect <file>` prints the class of the extension, its methods with their help, and the
folders and hosts it asks for. `--source` also prints the Novis source files inside it.
`nvs ext verify <file>` runs every check that loading the file runs, and says if the file passes.
`nvs ext pin <file>` prints the `[[extension]]` entry for `nvs.toml`, with the file's `sha256`.

None of the three commands runs code from the file. `nvs ext inspect` also works for a file that
does not load.

**Good to know:** the entry from `nvs ext pin` has no `grants`, so the extension may not read files
or connect to hosts. You add the folders and hosts you allow after you read `nvs ext inspect`.
`nvs ext pin` prints nothing for a file that does not pass the checks.
