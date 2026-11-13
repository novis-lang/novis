A link to another module (another program on the same server) names that module's route the way a
local link does: `Core\Router::url(Shop\View\Index::render)`. The route's class sits under a root the
linking program declares with `autoload links` (`rule:programs/autoload`), which is read for its
routes and never compiled.

**The compiler checks which method, and the server picks which mount.** The call is checked exactly
as a local link is (`rule:routing/link-target-and-params-are-checked`): the method, its `#[Route]`,
the captures and the keys. The compiler reads only the file that declares the target class, and only
its attributes and capture types. A capture whose type is a class or enum of the links root is not
value-checked, like a computed value. The site lowers to the mount-relative path and the target's
handler name, never to a finished URL, so compiling never reads `nvs.toml`. A path belongs to the
source and a mount belongs to the deployment, the same split as
`rule:routing/link-carries-the-mount-prefix`.

At boot, after every row of the mount table has compiled, each row whose unit links to another
module gets a table from handler to prefix, with an origin where the target is on another host. The
three steps and the tie are `rule:http-server/a-canonical-mount-breaks-a-tie`'s. Every request of
that row reads the table through one shared reference, so a link costs one hash lookup.
`urlAbsolute` uses the same order and always writes an origin: the target row's on another host,
and the linking row's own otherwise.

**A missing or broken target is a compile error in the linking program.** That is the dead link,
found before it ships. The target file joins the linking unit's revalidation set, so a changed or
removed route recompiles the linker on its next validation, and a deploy recompiles it under
`validate = never`. A module that only some deployments ship is not linked to from code every
deployment compiles. **A target row that did not compile does not stop the server:** the link is
logged at boot and throws when it runs, naming the handler and the mount, until a pass compiles the
target (`rule:http-server/a-mount-table-expands-at-boot`).

The one place a links-root name may be written is the route argument of `url` and `urlAbsolute`.
Anywhere else it is `E0897`, and `urlSigned` and `signedRoute` never take one
(`rule:core-classes/router-signed-url`).

**What it spends:** O(rows × links-root handlers) per expansion of the mount table, shared by every
request of a row. Nothing is held per request.

**Declined:**
- **A link that lowers to a finished URL.** The compiler would read the deployment's mount table,
  and one compiled unit could no longer serve under two mounts.
- **Compiling the linked module into the linking program.** Its classes would join the route table
  and `implementing<T>()`, and one program would run another's code inside its own isolate.
