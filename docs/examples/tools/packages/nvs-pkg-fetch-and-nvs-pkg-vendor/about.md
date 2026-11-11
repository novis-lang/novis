`nvs pkg fetch` downloads every package your application needs into `vendor/`, and `nvs pkg vendor`
downloads them all again.

Run the command next to `package.toml`. It downloads each package in `[dependencies]` and the packages
they need. It writes `vendor/packages.nvs`, so one `require` line loads every package. It
also writes `package.lock` with the version and the digest of each package. Commit that file. A
download that does not match its digest is an error. With `--locked`, a change to `package.lock` is
an error too.

When two packages need different versions of a package, the higher version is used. No code of
a package runs during the fetch.

`nvs pkg fetch` keeps a folder that still matches `package.lock`. Use `nvs pkg vendor` after you
change a file in `vendor/`.

For a package that contains an extension, the fetch prints the `[[extension]]` entry for `nvs.toml`.

**Good to know:** a package may read the files it came with, even when its grant line is empty. The
fetch stops with an error when a package contains an `nvs.toml` file or a second `package.toml`.

**The examples below** show one `require` line, a shared version, a package reading its own file,
and an extension.
