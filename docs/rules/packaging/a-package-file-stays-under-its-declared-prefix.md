Authority is looked up by the enclosing namespace alone
(`rule:security/authority-is-the-enclosing-namespace`), and keying on a self-declared attribute would
be worthless if a package could declare any attribute it liked. Two checks close that.

**A package declares its namespace prefix in its manifest, and two packages claiming one prefix is
an error at fetch time naming both.** The collision is caught when the graph is assembled, not as a
redeclaration during compilation.

**Every file in a fetched package must declare a namespace under that package's declared prefix.**
This is a compile-time check needing no new information: the file's package is known from the
fetched layout, the prefix from the manifest, and the declaration is in the file. A package whose
file declares `namespace App;` in order to reach the application's grants fails to compile, naming
the file, the namespace it declared and the prefix it was required to stay inside.

The second check is *not* how authority is looked up — the lookup reads the namespace alone. It is a
separate integrity check that the fetched layout and the declared namespaces agree, so the attribute
the lookup trusts is one the package was not free to choose. Hand-vendored code has no manifest and
so no such check; what it has instead is a human writing the `autoload` line that names its prefix,
and the audit warning `rule:security/an-unmatched-namespace-holds-the-application` describes if they
did not think about it.

**Not on disk.** The diagnostic code the design named has since been issued to a configuration
refusal; the check takes the band's next free number when it lands.
