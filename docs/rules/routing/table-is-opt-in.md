Finding routes is the question the program enumeration was built for — *what exists that nothing
names* — and the route table is that enumeration filtered by `#[Route]`, reused wholesale with its
consequences (`rule:programs/implementing`):

- **A program containing no call to a member whose answer comes from the table performs no scan of
  the autoload roots**, the identical opt-in rule a discovery query has, and any one of them alone
  opts it in. Those members are `Core\Request::route()` and `Core\Router`'s `match`, `methodsFor`,
  `url`, `urlAbsolute`, `urlSigned` and `signedRoute`. `route()` and `signedRoute` read the match the
  door took, and that match is taken against the table, so without the scan a front controller
  reading `route()` over autoloaded routes would get `null` on every request. `Core\Request::mount()`
  reads the mount table and does not opt in. A program with no `#[Route]` anywhere pays nothing,
  including no pass. A running program with no table matches nothing and serves the request however
  it likes. The table still holds every `#[Route]` of every class the program compiles, named or
  required, so the door's CSRF check covers every handler that can run, scan or no scan.
- **The scan makes the compiled unit depend on directory contents**, so every listed directory joins
  the revalidation set: no new dependency kind and no new directive. Zero under `validate = never`,
  which is what production runs.
- Routes in files reached by an ordinary `require` are included too; they are already in the graph.
  Rows accumulate across every file into one program-wide table, which is where a duplicate between
  two files is caught (`rule:routing/routes-are-compiled-not-registered`).

The table is a compile product carried by the unit and installed before the program runs, beside the
command table and the configuration snapshot; the match against it is taken once at the door and
nothing dispatches (`rule:routing/matching-is-not-dispatching`). Its cost is O(routes in compiled
code) in the artifact, not per request and not per object. A link to a route declared in another
module needs that module compiled — the consequence `rule:programs/no-runtime-autoload` already
records for a hard cross-module reference, arriving in a second place.
