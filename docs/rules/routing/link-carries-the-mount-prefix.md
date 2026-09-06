`Core\Router::url` prepends the prefix of the mount the request arrived through, in front of the
substituted path. One compiled table then serves the same module at `/ModuleA`, at `/ModuleB` or at
`/` with no recompile — a path is source state and a mount is deployment state, so the prefix is
known only where the request is — and it is why link generation goes through this member rather than
concatenating a declared path: a link assembled from the declaration is wrong the day the module is
mounted anywhere but the root.

A program run off the command line is mounted nowhere and the prefix is empty; nothing else about the
link changes (`rule:routing/link-name-and-params-are-checked`). `urlAbsolute`'s configured origin
is the other half of an absolute link, and is read from the resolved unit rather than from any header.

**Not shipped.** `crates/nvs-stdlib/src/router.rs` substitutes and percent-encodes but joins no
prefix in front, and the server hands it none: the prefix `crates/nvs-server/src/mount.rs` strips
from the request path does not reach the link.
