`nvs pkg add acme/slug` adds a package to your application and downloads it into `vendor/`.

The command adds the newest released version to `[dependencies]` in `package.toml`. Write a version,
such as `acme/slug@1.2.0`, to add exactly that version. The packages it needs are downloaded too.

Before any file changes, the command prints each package with the capabilities it requests. Each
new package gets an empty line in `[grants]`, so it may use no capability yet. A comment on the line
says what the package requests. When you agree, you add the capability to the line yourself.

**Good to know:** a version that has a known problem stops the command, and no file changes. The
error gives the reason and the version with the fix.

**The examples below** show a package you added, a package that needs another package, and a
capability you added to a grant line.
