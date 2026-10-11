`namespace X;` is the only namespace statement, written once per file and before any declaration. The
braced form `namespace X { … }`, a second `namespace X;` in one file, and a `namespace X;` after a
declaration it would not cover are all `E0243` at parse time.

`rule:security/authority-is-the-enclosing-namespace` keys authority on the namespace enclosing the code,
and `rule:config/an-application-is-its-entry-file-path` keys an application on its entry file. A file
that is two namespaces is a file whose authority depends on the line number, and neither rule has a
reading to give that. The rewrite is one file per namespace.

The braced form is still parsed after it is refused, so the declarations inside it report their own
problems in the same run.
