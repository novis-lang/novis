`Core\Router\Match` carries a declared name, the typed captures the path filled, the declared verb and
the access decision's name, and nothing invocable: no `->invoke()`, no callable, no class-and-method
strings, and the handler's `Class::method` label never crosses. A first-class reference to the
matched method would need `callable` to carry a signature, which `rule:types/grammar` defers — but
routing is not a forcing case for that deferral, and typed `callable` would not remove the dispatch
`switch`, for three independent reasons.

What a match returns beyond its name and captures, and why:

- **`access()` returns the decision's name**, such as `App\Role::Admin`, never the constant's value.
  The compiler checks that the name resolves and promises nothing about what it means, so the name is
  the one thing the row carries without interpreting it. `null` comes only from a table built by hand,
  because a route with no decision does not compile, and a dispatcher treats `null` as denied.
- **`method()` returns the verb the matched row declares.** Two `#[Route]`s on one method may share a
  `name` when they share a path, so only the verb tells a `Get` match from a `Post` match.
- **The handler's `Class::method` label stays off the match**, because it is the one field of the row
  a program could turn into a call.
- **Memory:** each read of `Core\Request::route()` or `Core\Router::match` holds one string, the length
  of the decision's name, and one integer. A program that never reads a match pays nothing.

Routes do not share a signature: `show(uint $id)`, `index()` and `blog(uint $y, uint $m, string $slug)`
cannot inhabit one `handler` field, and pre-binding the converted parameters into a zero-argument callable
is dispatch. Handlers have no common return type, and a `callable(): Response` would force one, which is
a framework opinion `Core` refuses to hold. And the boundary ground survives regardless: invoking would
be dispatch, which the table refuses on its own account
(`rule:routing/the-servers-match-dispatches-nothing`).

The ergonomic cost is paid where the framework layer already sits: `Web\Controller` generates the
`switch` from the route table while compiling (`rule:programs/framework-web-package`), and an application
declining the framework writes it itself, permanently. The roster of compiler-recognised attributes this
table draws on has one home (`rule:core-classes/derive-attribute`), and no rule restates its size.

**Declined:**
- **The server calls the matched method.** Calling it would be dispatch
  (`rule:routing/the-servers-match-dispatches-nothing`).
- **`params` as a shape per route, narrowed by comparing `name`.** The checker has no such narrowing,
  and it would save only one `as`. A generated dispatch `switch` binds typed parameters without it.
