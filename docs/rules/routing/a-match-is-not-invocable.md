`Core\Router\Match` carries the matched row, which `is` compares a method reference against, the
typed captures the path filled, the declared verb and the access decision's name, and nothing
invocable: no `->invoke()`, no callable, no class-and-method
strings, and the handler's `Class::method` label never crosses. A first-class reference to the
matched method would need `callable` to carry a signature, which `rule:types/grammar` defers — but
routing is not a forcing case for that deferral, and a typed `callable` on the match would not be the
way to dispatch, for three independent reasons.

What a match returns beyond its captures, and why:

- **`is(Shop::show)` returns whether that method's route matched.** It takes a method reference and
  returns a `bool`, so it identifies a route the way a link names one without handing anything
  invocable back. A written reference folds to the row index while compiling; a reference passed as
  a value is compared by row when the call runs.

- **`access()` returns the decision's name**, such as `App\Role::Admin`, never the constant's value.
  The compiler checks that the name resolves and promises nothing about what it means, so the name is
  the one thing the row carries without interpreting it. `null` comes only from a table built by hand,
  because a route with no decision does not compile, and a dispatcher treats `null` as denied.
- **`method()` returns the verb the matched row declares.** Two `#[Route]`s on one method share
  its path, and `is` is `true` for both, so only the verb tells a `Get` match from a `Post` match.
- **The handler's `Class::method` label stays off the match**, because it is the one field of the row
  a program could turn into a call.
- **`attribute<T>()` returns the matched method's attribute payload of the shape `T`**, folded while
  compiling and looked up by the label on the runtime's side, so the label still does not cross
  (`rule:routing/a-match-reads-an-attribute-as-data`).
- **Memory:** each read of `Core\Request::route()` or `Core\Router::match` holds one string, the length
  of the decision's name, and two integers, the verb and the row's place in the table. A program that
  never reads a match pays nothing.

Routes do not share a signature: `show(uint $id)`, `index()` and `blog(uint $y, uint $m, string $slug)`
cannot inhabit one `handler` field, and pre-binding the converted parameters into a zero-argument callable
is dispatch. Handlers have no common return type, and a `callable(): Response` would force one, which is
a framework opinion `Core` refuses to hold. And a callable on the match would be a second way to reach a
handler, around the gate `Core\Router::dispatch` puts in front of every one
(`rule:routing/a-gate-answers-around-every-handler`). That call is generated code in the unit, indexed
by the matched row, so dispatching needs nothing from the match a program could call. The roster of
compiler-recognised attributes this table draws on has one home
(`rule:core-classes/derive-attribute`), and no rule restates its size.

**Declined:**
- **The server calls the matched method.** The program calls `dispatch`, and a program that does not
  is served as before (`rule:routing/the-servers-match-dispatches-nothing`).
- **`params` as a shape per route, narrowed by `is`.** The checker has no such narrowing,
  and it would save only one `as`. The generated jump binds typed parameters without it.
