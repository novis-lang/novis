`nvs pkg pack` builds the archive of a package, the file that other applications download.

The archive contains only these files: every `.nvs` file under `src`, `package.toml`, the license and
readme files next to `package.toml`, and each file that `[package] files` lists. Any other file in the
folder is not added. The command prints each file with its size, the total and the digest of the
package. The digest is the hash that `package.lock` records.

The same files always give the same archive and the same digest, on every system. The time, the owner
and the permissions of a file are not in the archive.

A package must have a license file, such as `LICENSE`. It has at most 5,000 files and at most 16 MiB.
A symbolic link is not allowed. For a larger file, use `nvs pkg download`.

**Good to know:** the package's own code may read every file in its archive with no grant.

**The examples below** show a package reading a file it lists, its license file, and its readme file
next to its code.
