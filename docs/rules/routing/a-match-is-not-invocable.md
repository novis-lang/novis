`Core\Router\Match` carries a declared name, the typed captures the path filled, the declared verb and
the access decision's name, and nothing invocable: no `->invoke()`, no callable, no class-and-method
strings, and the handler's `Class::method` label never crosses. A first-class reference to the
matched method would need `callable` to carry a signature, which `rule:types/grammar` defers — but
routing is not a forcing case for that deferral, and typed `callable` would not remove the dispatch
`switch`, for three independent reasons.

Routes do not share a signature: `show(uint $id)`, `index()` and `blog(uint $y, uint $m, string $slug)`
cannot inhabit one `handler` field, and pre-binding the converted parameters into a zero-argument closure
is dispatch. Handlers have no common return type, and a `callable(): Response` would force one, which is
a framework opinion `Core` refuses to hold. And the boundary ground survives regardless: invoking would
be dispatch, which the table refuses on its own account
(`rule:routing/the-servers-match-dispatches-nothing`).

The ergonomic cost is paid where the framework layer already sits: `Web\Controller` generates the
`switch` from the route table while compiling (`rule:programs/framework-web-package`), and an application
declining the framework writes it itself, permanently. The roster of compiler-recognised attributes this
table draws on has one home (`rule:core-classes/derive-attribute`), and no rule restates its size.
