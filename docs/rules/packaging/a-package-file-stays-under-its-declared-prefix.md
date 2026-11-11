Every file in a package declares the package's namespace, `Vendor\Package`, or one under it, and a
file that declares anything else is a **compile error** naming the file, the namespace it declared and
the package it had to stay inside.

A file belongs to a package when a directory between it and the entry's directory holds a
`package.toml`, and that manifest gives the package's name
(`rule:packaging/a-package-name-is-vendor-slash-name`). The check needs nothing more: the package comes
from the layout, its name from the manifest, and the declaration is in the file. It needs no package
manager either, so a package copied in by hand with its manifest is checked exactly as a fetched one is.

Authority is looked up by the enclosing namespace alone
(`rule:security/authority-is-the-enclosing-namespace`), which would be worthless if a package could
declare any namespace it liked. This check is separate from the lookup: it makes the namespace the
lookup trusts one the package was not free to choose. A package file that declares `namespace App;` to
reach the application's grants does not compile.

**No fetch-time clash check exists.** A package's prefix is its name, a resolved graph holds one source
per name, and each vendor resolves from one registry
(`rule:packaging/a-vendor-resolves-from-one-registry`), so two packages in one graph cannot claim one
prefix.

Hand-copied code with no manifest has no package and so no file check. It has the grant check, a person
writing the `autoload` line that names its prefix, and the audit warning
`rule:security/an-unmatched-namespace-holds-the-application` describes.

The file's package is found from its real path: the outermost `package.toml` with a `[package] name`
between the file and the entry's directory. A manifest with no name ends nothing, and one inside a
package's directory is never read, so it makes no second package and no hole in the first: every file
under the package's directory is checked against the package's name. A package kept inside another
package's directory moves beside it. An autoloaded file, a file a `discover` glob found and a file a
`require` loaded are placed by this one rule (`crates/nvs-types/src/package.rs`). The diagnostic is
`E0660`, and a manifest whose name is not `Vendor\Package` is `E0659`. Cost: one `stat` per directory
between a loaded file and the entry's, at compile time.

**Declined:**
- **The nearest named manifest wins.** A package could then write a second `package.toml` into one of
  its own folders and hand that part of its tree to another name, or end the check there with a
  manifest that names nothing. A nested package is rare, and moving it beside the outer one costs a
  copy, where the hole costs the namespace the grant lookup trusts.
