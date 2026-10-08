`nvs build --compile <file>` writes one executable file that contains your program.

The file is a copy of the `nvs` binary with the source of the program added. It contains the entry
file and every file that the program loads with `require`. The program is compiled when the
executable starts, as with `nvs run`. `-o <path>` sets the output path. The default is the name of
the entry file without its extension, in the current directory.

All arguments of the executable go to your program. The executable does not read `run`, `check` or
`--help` as `nvs` commands.

**Good to know:** the executable reads `./nvs.toml` from the directory where it runs. Files
that the program finds only through `autoload` at run time, or opens with `Core\IO`, are not in the
executable.
