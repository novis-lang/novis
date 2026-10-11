A method carrying `#[Route]` must also carry `#[Access]`. A `#[Route]` without one is `E0762`, naming
the method and suggesting `#[Access(allow: Audience::Public)]` for a route that is genuinely open. The
same sentence read from the other side is `E0788`: an `#[Access]` on a method that declares no route is
refused rather than ignored, because the route table is the only thing that ever asks the decision.

```nvs
#[Route(path: "/admin/users", method: Http\Method::Get)]
#[Access(allow: Role::Admin)]
public function listUsers(): Response { … }

#[Route(path: "/", method: Http\Method::Get)]
#[Access(allow: Audience::Public)]      // written, because an omission is not a default
public function home(): Response { … }
```

There is no implicit `Public` and no configuration that supplies one. A route that is public because
its author decided so and one that is public because its author forgot are otherwise indistinguishable
at every later reading, and a forgotten permission check is the most exploited class of web
vulnerability. The qualifiers and the sinks delete injection; without this rule, nothing would catch
a handler that forgets its check, which compiles, serves and produces no diagnostic.

The cost is one line on every route, public ones included, so the reader of a route never has to ask
whether an absence meant anything. Nothing here runs: the check is compile-time, and the check a
handler performs at run time is one it would have performed anyway.

`#[Route]` gains no field. Two attributes rather than one is what keeps the route table free of
attached behaviour: this attaches a *declaration* to the method, not a filter, a group or a dispatch
opinion. The guarantee is that the decision was **written**, not that it was **honoured** — the
compiler never asks what the name means.

**Declined:**
- **An `access:` field on `#[Route]`.** One attribute reads better and cannot be half-applied, but it
  puts behaviour on the attribute that carries none, and the line between metadata the table carries
  and a dispatch opinion gets harder to hold for whatever asks next.
- **A completeness check against an access map the framework declares.** The map is a second place to
  keep in sync with the routes, and the diagnostic points at the map rather than at the route that is
  missing.
- **Leaving the decision entirely to a framework layer.** That is the arrangement every framework
  already has, and the most exploited class of vulnerability lives underneath it.
- **`#[Access(allow: Audience::Public)]` when the attribute is omitted.** It is the implicit default
  this rule exists to make visible.
