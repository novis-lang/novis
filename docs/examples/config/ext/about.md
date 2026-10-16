The settings of a loaded extension. An extension is code that runs in a sandbox beside your program,
loaded by an `[[extension]]` entry in `nvs.toml`. An extension may declare settings, and you set them
in `nvs.toml` in a block named after the extension, such as `[ext.ledger]` for `Shop\Ledger`.

The extension declares each key, its type and its default. A key you leave out has its default.
Each type is one of `bool`, `int`, `uint`, `float`, `string` or a list of strings.

The server checks the block when it starts. A key the extension does not declare stops the start
with an error that names the extension. A block that belongs to no loaded extension stops it too.
So a typo in a setting never goes unnoticed.

Only the person who runs the server sets these values. A program cannot change them:
`Core\Config::set` returns `false` for every key in an `[ext.<name>]` block. A setting can only
change how the extension behaves. It cannot give the extension access to files or the network.

**The example below** runs a program with the test extension `Shop\Ledger` and its settings block.
