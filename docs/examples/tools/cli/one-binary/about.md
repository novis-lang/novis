Novis is one program, `nvs`. You give it a command, then the options of that command, and usually
one file.

`nvs run` checks, compiles and runs a program. `nvs check` reports every error and runs nothing.
`nvs test` runs tests. `nvs build` writes a single executable file or an OpenAPI document.
`nvs config check` and `nvs config dump` read the configuration. `nvs info` prints information about
the binary, and `nvs --version` prints the version. `nvs --help` lists every command.

Every command takes `--config <PATH>` and `--help`.

**Good to know:** the short options `-i`, `-a`, `-r` and `-f` do not exist. Each of these
operations is a command.
