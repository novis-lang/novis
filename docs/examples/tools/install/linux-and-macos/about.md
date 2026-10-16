On Linux and macOS, Novis reads the owner and the mode of a file or folder to check who can write to it.

A path passes the check when its owner is `root` or the account that runs `nvs`, and its mode has no
write bit for the group or for others. The modes `0755`, `0750` and `0640` pass. The modes `0775`,
`0777` and `0664` fail. The command `chmod go-w` removes both write bits, and then the path passes.

The `install` command creates a folder with its owner, its group and its mode in one step. The
binary folder and the configuration folder are owned by `root`, so the account that runs `nvs`
cannot change them. The cache folder and the log folder are owned by that account, because it
writes to them.

**Good to know:** Novis never checks who can read a path. The mode `0750` on the configuration
folder stops other accounts from reading it, and that is your choice.

**The example below** applies the same rule to a list of folders and prints which of them fail.
