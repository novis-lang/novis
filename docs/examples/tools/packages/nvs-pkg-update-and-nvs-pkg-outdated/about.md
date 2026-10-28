`nvs pkg outdated` lists each package that has a newer version, and `nvs pkg update` changes your
application to use it.

`nvs pkg outdated` changes no file. `nvs pkg update acme/slug` raises the minimum of that package in
`[dependencies]` to its newest version. Without a package name, it raises every dependency. Then it
does what `nvs pkg fetch` does.

Before it changes a file, the command prints what changed since `package.lock`. The report lists each
new version, each capability a package now requests, and new owners or a new repository. The command
does not change a grant line. So a new capability does not work until you add it yourself.

When two packages need different major versions of a third package, the higher version is used. The
report names the packages that asked for the older one. If the new version breaks your code, your
program does not compile.

**Good to know:** a git dependency is not updated. Change its `rev` in `package.toml` instead.

**The examples below** show the raised version in use, a new capability after you add it, and the
higher of two major versions.
