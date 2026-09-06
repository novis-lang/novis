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
