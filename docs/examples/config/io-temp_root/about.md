The directory in which Novis creates every temporary directory that a program requests.

A program that needs temporary files requests a temporary directory. Novis creates all of them in
one root directory, and `[io] temp_root` sets that root. You can use a fast disk, a volume with
free space, or a path that a container mounts.

When the key is not set or is an empty string, Novis uses the `tmp` folder in its data folder,
`.nvsdata` next to the `nvs` program. Novis creates it, and only the account that runs `nvs` can
open it.

When a script ends, Novis deletes the temporary directories that the script created. When the
server starts, Novis deletes what a crashed process left in the root. Do not set the root to a
directory that other programs write to.

**Good to know:** only the configuration file can set this key. You can change it while the server
runs, and the next temporary directory is created in the new root.
