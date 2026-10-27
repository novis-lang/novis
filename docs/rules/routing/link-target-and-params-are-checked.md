`Core\Router::url(callable $route, array<string, mixed> $params = []): string` takes the route by its
method reference — `Router::url(App::show, ['id' => 3])` — and reverses the table from that method.
The compiler resolves the call against the whole program's table after every file has been walked, so
the route may be declared in a file the scan reaches later.

**A reference written at the call is checked while compiling**, and each failure is an error reported
at the argument: **a method that does not exist**, **a method with no `#[Route]`**, and **a `$params`
array literal that does not cover the route's captures** — every `{name}` and `{name...}` the path
writes needs a key of that name, and only a `{name?}` may be left out, whose whole segment is then
dropped. A key in that array literal that is neither a capture nor one of the route's declared
`#[Query]` parameters is refused in the same shape, so a typo cannot silently become a query
parameter. `$params` defaults to `[]`, so a route with no captures is linked as `Router::url(App::home)`.

**A reference passed as a value**, such as a menu entry built elsewhere, is resolved when the call
runs, and the call throws when the method has no route. A computed `$params` is not checked at all —
there are no keys to read.

**A link calls nothing**, so an instance method needs no object (`App::show` names it whether or not
it is `static`), and its visibility is not checked: a `private` handler is linked to like a `public`
one.

**A literal value is held to the conversion of the parameter it supplies**, a capture's or a
`#[Query]` parameter's, and one the conversion refuses is `E0772` at the call: `"x"`, `-3` and `2.5`
for a `uint`. The check runs the matcher's own conversion on the segment the value builds, for the
types the matcher converts itself (`int`, `uint`, `decimal`, `Core\Uuid`); a closed set is
`rule:routing/a-capture-narrows-to-a-closed-set`'s. A `float` literal is refused for each of those but
`decimal` without being turned into text. A computed value, a `string` parameter and a class built from
text are not checked, so a link is a path its route matches wherever the value is written at the call.

The result is a laundered URL path (`rule:security/launderers-are-sink-named`): each substituted
value is percent-encoded into its own segment under RFC 3986's component rules, so a `tainted`
parameter produces a plain `string` that is safe *as a path* and for nothing else. `urlAbsolute` is
the same link with a configured origin in front, and refuses exactly the same targets. The path a link
is built on is the mount-relative one (`rule:routing/link-carries-the-mount-prefix`).

**Declined:**
- **Checking the handler's visibility.** A link builds a path and invokes nothing, so a `private`
  handler is no less reachable for being linked to; the route's `#[Access]` is what guards it.
