A route is declared where its handler is: `#[Route(path: "/users/{id}", method: Http\Method::Get,
name: "user.show")]` on a method. The compiler reads every such attribute, through the program
enumeration `rule:programs/implementing` already built, into a route table baked into the compiled
unit. Nothing registers a route at run time — there is no routes file, no `$router->get(...)`, and no
generated cache to warm or go stale, because the work that cache exists to save is done while compiling.

Three bugs every runtime router discovers late are compile errors here, each naming both sites. **Two
routes claiming the same verb and the same path shape** — `/users/{id}` and `/users/{userId}` are one
shape, and a capture's type never disambiguates two declarations, because a rule under which it did
would make matching depend on declaration order after all. **A `{name}` capture with no parameter of
that name** on the annotated method. **A link naming a route that does not exist**
(`rule:routing/link-name-and-params-are-checked`). A duplicate `name` is refused the same way, with
one exception: repeated `#[Route]`s on one method may share a `name` when they also share a `path`.

The limit is the one the design buys. A route cannot be added at run time, so a CMS with
database-defined URLs matches its own way and uses none of this — and because the router stops at
matching (`rule:routing/matching-is-not-dispatching`), using none of it, half of it, or `url` alone
costs nothing.

**Declined:**
- **A runtime registry** (`$router->get("/users/{id}", …)`). It moves all three checks to run time,
  needs a generated cache file to be fast, and makes registration order part of what a route means.
- **A route table in `nvs.toml`.** A URL is source state: the code that handles a path and the path
  itself change together, in the same commit. A schedule is deployment state, which is why it is
  the opposite case (`rule:config/scheduled-work-is-a-config-block`).
- **Building the table from a `Core\Ast` walk at run time**
  (`rule:tooling/reflection-and-source-parsing-are-core-features`). It parses the program again
  inside a request, gets none of the three compile-time checks, and puts the parser on the request
  path for something the compiler already knows.
- **Captures' types disambiguating two routes at the same path**, `/{id}` on `uint` beside `/{slug}`
  on `string`. It brings order-dependence back and makes the duplicate check partial rather than
  total.
