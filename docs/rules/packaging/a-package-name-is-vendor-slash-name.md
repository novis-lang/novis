A package name is **`vendor/name`**: both segments lowercase ASCII with `-` permitted, and compared
**exactly** — the same rule the compiler applies to every other name, so that a name which resolves
on one operating system resolves on all three. A registry maps a name plus a minimum version to a
digest; the name is never the identity (`rule:packaging/a-package-is-its-digest`).

The vendor segment `nvs` is reserved for first-party packages, exactly as `Core` is reserved in the
language (`rule:core-api/reserved-namespace`). First-party packages are otherwise subject to every
rule here: `nvs/web` resolves, locks, is logged and is granted like anyone else's package, because a
registry whose maintainers do not depend on it does not stay good.

Names are first-come, and the squatting policy is the registry operator's own operational document.
