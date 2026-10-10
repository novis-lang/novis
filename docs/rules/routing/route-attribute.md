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
  — a `Request`, a service — which the router ignores.
- **`name` is optional and never derived.** A route without one is matchable but not linkable.
- **`slotted` is optional and `false` when absent.** It is the route half of
  `rule:core-classes/html-later`'s slotted delivery, and the route table carries it.

A `#[Route]` method also carries an `#[Access]` (`rule:attributes/access-is-a-required-sibling`).

The closed list only works if every entry on it is argued: a compiler-recognized attribute that is
cheap to add is how an attribute set becomes an undocumented second language surface.

**Declined:**
- **Structural matching**, the way `rule:attributes/structural-retrieval` retrieves an attribute. A
  framework's own `Route`-shaped literal would register routes Novis does not own
  (`rule:core-classes/derive-attribute`).
- **Deriving `name` from the class and method.** A link would break silently on a rename, which is
  one of the three bugs `rule:routing/routes-are-compiled-not-registered` exists to catch.
- **A class-level prefix attribute** (`#[Route(path: "/admin")]` on a class). It interacts badly
  with inheritance and hides a route's real path from the line that declares it; a prefix is a
  framework opinion.
- **A `host` field.** A hostname in the compiled table would end the relocatability mounts buy
  (`rule:routing/a-request-reads-its-mount`).
