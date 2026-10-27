`type Core\Route = {path: string, method: Core\Http\Method, slotted?: bool};` — an ordinary
`rule:attributes/inert-metadata` alias, attached the ordinary way, with one difference: the compiler
acts on it only when the name **resolves** to `Core\Route`. It sits on
`rule:core-classes/derive-attribute`'s closed list of compiler-recognized attributes and is matched
nominally, so a userland `type Route = {...};` and a framework's own `Route`-shaped anonymous object declare
no route however they are spelled.

- **`method` is an enum case**, the same `Core\Http\Method` that `Core\Request::method` answers,
  never a string (`rule:attributes/payload-is-a-compile-time-constant`). `path` and `method` are
  required; an attribute naming neither declares nothing and is refused.
- **The attribute is repeatable** (`rule:attributes/repeatable`), which is how one method serves two
  verbs. There is no `methods:` list and no wildcard verb. Every `#[Route]` on one method writes the
  same `path` (`rule:routing/routes-are-compiled-not-registered`).
- **Methods only**, `static` or instance, and the method may take parameters the path does not name
  — a `Request`, a service — which the router ignores.
- **A route is identified by its method.** There is no `name` field: a link, a match check, a signed
  link and the API document all name the route with a method reference such as `Shop::show`
  (`rule:routing/link-target-and-params-are-checked`), so every route is linkable.
- **`slotted` is optional and `false` when absent.** It is the route half of
  `rule:core-classes/html-later`'s slotted delivery, and the route table carries it.

A `#[Route]` method also carries an `#[Access]` (`rule:attributes/access-is-a-required-sibling`).

The closed list only works if every entry on it is argued: a compiler-recognized attribute that is
cheap to add is how an attribute set becomes an undocumented second language surface.

**Declined:**
- **Structural matching**, the way `rule:attributes/structural-retrieval` retrieves an attribute. A
  framework's own `Route`-shaped literal would register routes Novis does not own
  (`rule:core-classes/derive-attribute`).
- **A `name` field.** It is a second identity beside the method, with a mapping to learn and a
  duplicate-name error to keep. A link written by name is a string the editor cannot follow, while a
  method reference is checked while compiling and followed like any other. A method rename changes
  every reference with it, so the rename that a name existed to survive is a refactor, not a break.
- **A class-level prefix attribute** (`#[Route(path: "/admin")]` on a class). It interacts badly
  with inheritance and hides a route's real path from the line that declares it; a prefix is a
  framework opinion.
- **A `host` field.** A hostname in the compiled table would end the relocatability mounts buy
  (`rule:routing/a-request-reads-its-mount`), and the duplicate-route compile error would grow a host
  dimension.
