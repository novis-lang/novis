Novis checks who owns a configuration file or a cache folder, and who can write to it.

A configuration file can allow a program to read files, open network connections and start other
programs. An account that can change the file can allow itself all of that. The same is true for
the compile cache, because Novis runs the compiled programs it finds there.

A path passes the check when its owner is the account that runs `nvs` or an administrator, and no
group of ordinary accounts can write to it. `nvs serve` checks every configuration file and its
folder when it starts, and again after each save. When that check fails, the error is `E0607`.
`nvs init` checks the folder it writes into and the folder one level up. Every command that
compiles a program checks the cache folder in the same way. When that check fails, the command
prints a `warning:` line and the program still runs.

**Good to know:** Novis never checks who can read or run something. It does not check the log
folder.

**The example below** prints the paths that each command checks on one host.
