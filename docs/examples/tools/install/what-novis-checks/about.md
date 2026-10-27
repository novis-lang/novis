Novis checks who owns a configuration file or a cache folder, and who can write to it.

A configuration file can allow a program to read files and start programs. An account that can
change the file can allow itself all of that.

A path passes the check when its owner is the account the server runs as or an administrator, and
no group of ordinary accounts can write to it. A failed check is the error `E0607`.

- `nvs serve` checks every configuration file and its folder at the start and after each save.
- `nvs service install` and `nvs service status` check every `--config` file for the service's
  account. A failed check stops the install before it changes anything.
- `nvs config check --as <account>` checks for the account you name. Run it before a service starts.
- `nvs init` checks the folder it writes into and the folder above it.

The cache and data folders get the same check. When it fails, `nvs` prints a `warning:` and the
program still runs.

**Good to know:** Novis never checks who can read or run something.

**The example below** prints the paths that each command checks on one host.
