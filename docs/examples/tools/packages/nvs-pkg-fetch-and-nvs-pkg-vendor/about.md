`nvs pkg fetch` downloads every package your application needs into `vendor/`, and `nvs pkg vendor`
downloads them all again.

Run the command next to `package.toml`. It downloads each package in `[dependencies]` and each package
those packages need. It writes `vendor/packages.nvs`, so one `require` line loads every package. It
also writes `package.lock` with the version and the digest of each package. Commit that file. The
next fetch stops with an error when a download does not match its digest. With `--locked`, it also
stops when `package.lock` has to change.

When two packages need different versions of a third package, the higher version is used. No code of
a package runs during the fetch.

`nvs pkg fetch` keeps a package folder whose files still match `package.lock`. Use `nvs pkg vendor`
when a file in `vendor/` was changed.

**Good to know:** a package may read the files it came with, even when its grant line is empty. A
package that contains an `nvs.toml` file stops the fetch with an error, and nothing is vendored.

**The examples below** show the one `require` line, the version two packages share, and a package
reading its own file.
