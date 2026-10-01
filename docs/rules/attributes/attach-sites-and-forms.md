An attribute attaches to four things: a class or interface declaration, a method declaration, a
property declaration, and a parameter. It is written directly above the declaration, or directly
before the parameter.

```php
#[Route(path: "/users/:id", method: "GET")]
class UserController {
    #[Column(type: "varchar", length: 255)]
    public string $name;

    #[Cache(ttlSeconds: 60)]
    public function show(#[Autowire] UserRepository $repo): Response { … }
}
```

Two forms carry one payload between them, and both take named fields — there is no positional form.
The **bare** `#[{...}]` names no shape and is checked only as a well-formed object literal. The
**named** `#[Name(...)]` requires `Name` to resolve, through the ordinary namespace and `use` scope, to
a `type` alias whose right-hand side is a shape type; the literal is then checked against it by the
width subtyping every shape-typed position already uses.

The name may be written `Owner::Name`, an alias declared as a member of an interface, class or enum
body (`rule:types/class-scoped-alias`), and it is then resolved exactly as the type `Owner::Name` is,
the owner's file being autoloaded like any other name the attribute writes. A name is therefore
written the same way in an attribute and in a type, and an alias that belongs to an interface needs
no second, file-scope declaration to be usable as an attribute.

Three answers, deliberately three. A name nothing declares is the ordinary undefined-name `E0303` —
an attribute name is not a second namespace, so it gets no refusal of its own; for `Owner::Name` it is
the answer the same type would get. A name resolving to something that is not a shape-typed alias, a
class, a `type Id = int;` or an enum case, is `E0726`. A payload that fails the shape is the ordinary
mismatch `E0401`.

A malformed attribute is one error. The parser skips the rest of its `#[...]` group up to the
group's own `]`, so the declaration after it still parses and nothing else is reported for it.

The one exemption is the closed, `Core`-owned roster of compiler-recognized attributes, matched by name
and naming no shape at all. Each is a plain name, so `Owner::Name` never matches one. Every userland
name is an alias or a mistake.
