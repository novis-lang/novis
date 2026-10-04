`type Core\Route = {path: string, method: Core\Http\Method, name?: string, slotted?: bool};` — an ordinary
`rule:attributes/inert-metadata` alias, attached the ordinary way, with one difference: the compiler
acts on it only when the name **resolves** to `Core\Route`. It sits on
`rule:core-classes/derive-attribute`'s closed list of compiler-recognized attributes and is matched
nominally, so a userland `type Route = {...};` and a framework's own `Route`-shaped anonymous object declare
no route however they are spelled.

- **`method` is an enum case**, the same `Core\Http\Method` that `Core\Request::method` answers,
  never a string (`rule:attributes/payload-is-a-compile-time-constant`). `path` and `method` are
  required; an attribute naming neither declares nothing and is refused.
- **The attribute is repeatable** (`rule:attributes/repeatable`), which is how one method serves two
  verbs. There is no `methods:` list and no wildcard verb.
- **Methods only**, `static` or instance, and the method may take parameters the path does not name
  — a `Request`, a service — which the router ignores. There is no class-level prefix attribute: a
  prefix interacts badly with inheritance and hides a route's real path from the line that declares
  it.
- **`name` is optional and never derived.** A route without one is matchable but not linkable.
  Deriving one from the class and method would make a link break silently on a rename, which is one
  of the three bugs `rule:routing/routes-are-compiled-not-registered` exists to catch.
- **`slotted` is optional and `false` when absent.** It is the route half of
  `rule:core-classes/html-later`'s slotted delivery, and the route table carries it.

A `#[Route]` method also carries an `#[Access]` (`rule:attributes/access-is-a-required-sibling`).
