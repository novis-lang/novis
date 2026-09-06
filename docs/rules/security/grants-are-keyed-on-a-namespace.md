The application's grant table is keyed on namespace prefixes and lives in the application's own
configuration rather than in a package manifest. A package still declares what it requests and that
declaration still grants nothing; the application grants explicitly, one line at a time.

A capability named in a grant line must appear on the roster
(`rule:security/capability-roster-is-closed`); an unknown name is an error at boot, because a
directive that silently means nothing is worse than one that refuses.

The effective set at a call site is the **intersection** of the operator's configuration, the
application's grant, the package's own declaration and any narrowing an enclosing isolate applied —
every one of which may only tighten (`rule:security/no-runtime-grant`).

**Not on disk.** The tree reads the deployment's capability block; it has no per-namespace grant
table, and the diagnostics this rule needs do not exist.
