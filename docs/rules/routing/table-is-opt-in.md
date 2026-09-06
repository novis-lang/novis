Finding routes is the question the program enumeration was built for — *what exists that nothing
names* — and the route table is that enumeration filtered by `#[Route]`, reused wholesale with its
consequences (`rule:programs/implementing`):

- **A program containing no `Core\Router::match`/`::url` call performs no scan and builds no table**,
  the identical opt-in rule a discovery query has; a program with no `#[Route]` anywhere pays nothing,
  including no pass. A running program with no table matches nothing — `Core\Request::route()` is
  `null`, `methodsFor` answers empty — and serves the request however it likes.
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
