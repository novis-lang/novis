Authority is the namespace **enclosing the code**, not the nesting depth of the statement and not the
name being called.

A top-level statement in a file declaring a namespace runs under that namespace's grants exactly as a
method body in the same file would. **File-scope code is not exempt**, and it must not be: an
exemption would be a one-line bypass of the whole system. A file with no namespace declaration is in
the global namespace, which is the application, so an entry point is unrestricted up to the operator's
ceiling. An anonymous function carries the namespace it was **declared** in, not the one that calls it, so
one written in the application and invoked from a package runs under the application's authority.
A `use` import transfers nothing: authority is a property of where code *is*, never of what it names.

Keying on the namespace is what closes the override hole — a file overriding one class of a dependency
keeps that dependency's authority, because it must keep its namespace to be an override at all — and
it reaches code no package manager ever touched.

**Not on disk.** Nothing in the tree reads a per-namespace grant table.
