A security scheme is declared in the program, by a compiler-recognized `#[Api\Scheme]` on a class,
and every name an `#[Api(security: [...])]` writes must be one the program declares.

```nvs
use Core\Api;

#[Api\Scheme(name: "bearer", type: "http", scheme: "bearer", format: "JWT")]
#[Api\Scheme(name: "key", type: "apiKey", in: "header", parameter: "X-Api-Key")]
class Auth { … }

#[Route(path: "/orders/{id}", method: Http\Method::Get, name: "orders.show")]
#[Access(allow: Role::User)]
#[Api(security: ["bearer"])]
public static function show(uint $id): Order { … }
```

The attribute is repeatable and may sit on any class. Its fields are OpenAPI 3.1's Security Scheme
Object, spelled the way `#[Api]` spells its own fields: `name` is the key the document files the scheme
under, `type` is one of the five types the specification lists, and each type's required fields are
required. The scheme names form one program-wide set, because `components.securitySchemes` is one map
per document.

**The compiler checks both ends.** A `security` entry naming no declared scheme does not compile, and
the message lists the names the program declares, so a typo is a compile error rather than a document
that references nothing. Two declarations of one name do not compile, and the error names both sites. A
field the scheme's `type` does not take, and a required field left out, are errors where they are
written. A declared scheme that no route names is not an error: a document may describe a scheme
before a route uses it.

**The route table carries the declared schemes**, and `nvs build --openapi` writes them to
`components.securitySchemes`, sorted by name, so the document stays deterministic
(`rule:routing/api-document-is-a-deterministic-build-artifact`). The emitter still reads every fact
off the table (`rule:routing/api-document-is-generated-from-the-route-table`).

A scheme describes; it enforces nothing. Who may call a route is `#[Access]`'s decision
(`rule:attributes/access-is-a-required-sibling`), and a request runs the same with or without an
`#[Api\Scheme]` in the program.

- **Memory:** nothing per request. While compiling, one row per declared scheme, held in the route table.

**Not on disk.** `Core\Api` is the only `Api` attribute (`crates/nvs-types/src/derive.rs:594`), and
`security` names are collected without a comparison (`crates/nvs-types/src/routes.rs:1097`).

**Declined:**
- **Schemes in `nvs.toml`.** The compiler never reads a deployment's configuration, so it could not
  check a `security` name against it, and the document would depend on which file a build saw.
- **A guessed definition for an undeclared name.** The emitter would be stating a fact about
  deployment that nobody wrote (`rule:routing/api-document-is-generated-from-the-route-table`).
- **Checking that `#[Access]` and the scheme agree.** An access decision is a name the compiler does
  not interpret (`rule:security/access-is-checked-for-presence-not-meaning`), so there is nothing to
  compare it with.
