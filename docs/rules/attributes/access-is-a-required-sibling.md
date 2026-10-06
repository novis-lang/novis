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
at every later reading.

`#[Route]` gains no field. Two attributes rather than one is what keeps the route table free of
attached behaviour: this attaches a *declaration* to the method, not a filter, a group or a dispatch
opinion. The guarantee is that the decision was **written**, not that it was **honoured** — the
compiler never asks what the name means.
