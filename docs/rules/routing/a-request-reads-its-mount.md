`Core\Request::mount(): Core\Request\Mount` answers the two facts about the door a request came through:
`prefix(): string` is what the mount stripped from the path before the program saw it, and
`captures(): array<tainted string>` are the mount's glob captures in order — `{1}` is `captures[0]`.
The captures are `tainted` because they came off the wire (`rule:security/tainted-qualifier`), so
feeding one to a query launders normally; the prefix is the deployment's own plain text. One member for
the pair, because a request holding one mount's prefix and another's captures is a bug the shape rules
out.

**`mount()` is never `null`**, where `route()` is. A request the table does not claim is an ordinary
served request, but every request that reached a program reached it *through* a mount, and a door that
strips nothing answers `""` and an empty array rather than an absence. With no request at all it
refuses, like every other reader of the class (`rule:security/request-state-throws-in-an-isolate`).

This is how one compiled table serves many tenants, and why `#[Route]` gains no `host` field: a path is
source state and a hostname is deployment state, so each lives where it changes, and the table stays
relocatable to `/ModuleA`, `/ModuleB` or `/` with no recompile. A program never derives its own prefix
from the request target.
