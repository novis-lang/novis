Every setting in `nvs.toml` is written inside a block, and Novis accepts only the blocks that it
knows.

A block is a TOML table such as `[limits]`, or a list of tables such as `[[app]]`. `[limits]` and
`[limits.hard]` contain the resource limits. `[capabilities]` contains the grants. `[[app]]`
contains the settings of one application, and `[[include]]` reads other files. Other blocks
configure a named database (`[db.<name>]`), a mail server (`[mail.<name>]`), the caches, the log,
HTTP, the web server and scheduled scripts.

A setting outside a block, an unknown block and an unknown key all give the error `E0601`. Novis
stores every key that it accepts. `nvs config dump` prints each one, and a program reads one with
`Core\Config::get`.

**Good to know:** Novis stores an accepted key even when the command you run does not use it. For
example, `nvs run` stores `wall_time` and does not stop a program that runs longer.
