Novis reads its configuration from one TOML file named `nvs.toml`.

`nvs` reads `./nvs.toml` from the working directory. This is the directory where you start the
command, which can be different from the directory of the program. If there is no `./nvs.toml`,
`nvs` reads `nvs.toml` in the data folder, `.nvsdata` next to the `nvs` program. If there is no file
at all, the configuration is empty and valid: nothing is limited and nothing is granted.
`--config <path>` reads the named file, and neither `nvs.toml` is then read. Repeat the option to
read several files in order.

A relative path inside a file starts from the directory of that file. This is true for an include,
a path in a capability grant, an `[[app]]` key and a secret file.

**Good to know:** a key that Novis does not know stops the run before the program starts. The error
is `E0601`, and it names the block and the keys that the block accepts. A setting is always written
in a file. No environment variable and no command-line option sets one.
