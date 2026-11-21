Novis is one program, `nvs`. You give it a command, then the options of that command, and usually
one file.

`nvs run` checks, compiles and runs a program. `nvs check` reports every error and runs nothing.
`nvs test` runs tests. `nvs serve` runs a web application, and `nvs reload` makes a running server
use the code you changed. `nvs build` writes a single executable file or an OpenAPI document.
`nvs config check` and `nvs config dump` read the configuration. `nvs info` prints information about
the binary, and `nvs --version` prints the version. `nvs --help` lists every command.

Every command takes `--config <PATH>`, `--data <PATH>` and `--help`. `--data` sets the data
folder, where Novis keeps its own files.

**Good to know:** the short options `-i`, `-a`, `-r` and `-f` do not exist. Each task has its own
command, such as `nvs run` or `nvs info`.
