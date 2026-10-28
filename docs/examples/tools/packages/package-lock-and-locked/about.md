`package.lock` lists the exact version and the digest of every package your application uses.

`nvs pkg fetch`, `nvs pkg vendor`, `nvs pkg add` and `nvs pkg update` write the file. Commit it with
your code, and do not edit it. Each entry has the package's name, its version, its registry, its
digest and its source code repository. It also has the accounts that may publish it and the
capabilities it requests. The digest is a hash of the package's archive.

Every command checks each archive against its digest. When one does not match, the command stops with
an error and writes no file. So a changed archive is never used.

With `--locked`, the command also stops when `package.lock` has to change. That is when a package is
missing, has another version or is no longer needed. Use `--locked` in CI, so the build uses exactly
the versions you tested.

**The examples below** show the locked version in use, a grant line next to the capabilities the lock
lists, and a program reading the lock.
