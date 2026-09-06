**Both sides are canonicalized before they are matched** — symlinks resolved, `.` and `..` removed —
and the prefix test compares whole components. Without that, `/srv/www/shop/../other/x.nvs` matches
`root = "/srv/www/shop"` and a symlink planted inside an application's tree inherits that
application's capabilities. This is the same canonicalize-then-prefix rule
`rule:security/path-scope-canonicalise-then-prefix` applies to `script.spawn`'s roots, for the same
reason, and it is written once in the trust module rather than three times.

A block's key is canonicalized in place at boot, replacing what the operator wrote, so `nvs config
dump` prints the path the match will actually use rather than a relative fragment.

**A key naming something that cannot be examined refuses the boot** (`E0605`) rather than quietly
matching nothing. A block that covers no file hands every application it was written for the global
configuration instead, so a typo in the `root` of a block that narrows limits or grants a capability
would change what runs and report nothing. A missing directory is a loud refusal instead.
