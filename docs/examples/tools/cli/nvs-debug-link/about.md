`nvs debug link` prints the link to the debug viewer. It starts nothing.

The link contains the token that opens the viewer. The command reads the same settings and the same
token file as `nvs debug`, so the link works while `nvs debug` runs. If the token file does not exist
yet, the command creates it, as `nvs debug` would.

`--json` prints one JSON object with `url`, `embed`, `running` and `recording`. `embed` is the link
that an editor shows inside its own window. `running` is `true` when `nvs debug` runs for the same
data folder, the folder where Novis keeps the records and the token file. `recording` is the value
of `[debug] store`.

**Good to know:** a note on standard error says when no `nvs debug` runs, or when `[debug] store` is
off. The port `0` in `[debug] listen` means that `nvs debug` picks a free port. The command cannot
know it, so it prints an error. Then use the link that `nvs debug` printed.

**The example below** shows the options of the command.
