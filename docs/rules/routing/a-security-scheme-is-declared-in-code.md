A security scheme is declared in the program, by a compiler-recognized `#[Api\Scheme]` on a class,
and every name an `#[Api(security: [...])]` writes must be one the program declares.

```nvs
use Core\Api\KeyIn;
use Core\Api\Scheme;
use Core\Api\SchemeType;

#[Scheme(name: "bearer", type: SchemeType::Http, scheme: "bearer", format: "JWT")]
#[Scheme(name: "key", type: SchemeType::ApiKey, in: KeyIn::Header, parameter: "X-Api-Key")]
class Auth { … }

#[Route(path: "/orders/{id}", method: Method::Get)]
#[Access(allow: Role::User)]
#[Api(security: ["bearer"])]
public static function show(uint $id): Order { … }
```

The attribute is repeatable and may sit on any class. A qualified name is absolute
(`rule:statements/a-qualified-name-is-absolute`), so a file writes it `#[Core\Api\Scheme]` or imports
it with `use Core\Api\Scheme;`, and `use Core\Api;` alone does not place it. Its fields are OpenAPI 3.1's Security Scheme
Object, spelled the way `#[Api]` spells its own fields. The scheme names form one program-wide set,
because `components.securitySchemes` is one map per document.

| `type` | Required | Optional | Written to the document as |
|---|---|---|---|
| every type | `name: string`, `type: Core\Api\SchemeType` | `description: string` | the key, `type`, `description` |
| `ApiKey` | `in: Core\Api\KeyIn`, `parameter: string` | | `in`, `name` |
| `Http` | `scheme: string` | `format: string` | `scheme`, `bearerFormat` |
| `MutualTls` | | | |
| `OAuth2` | `flows` | | `flows` |
| `OpenIdConnect` | `url: string` | | `openIdConnectUrl` |

`type` is an enum, never a string (`rule:core-api/no-mode-strings`): `Core\Api\SchemeType` has the five
cases the specification lists. `in` is the enum `Core\Api\KeyIn` for the same reason, with `Query`,
`Header` and `Cookie`. `parameter` is the header, query or cookie name, because `name` is the scheme's
own key. `scheme` stays a string: it is a name from IANA's open HTTP authentication registry, such as
`bearer` or `basic`, and an enum would close a list the specification leaves open.

`flows` is an anonymous object with at least one of `implicit`, `password`, `clientCredentials` and
`authorizationCode`. Each flow is an anonymous object with `scopes`, an array from scope name to its
description (`[]` for none), and the URLs its kind needs: `authorizationUrl` for `implicit`, `tokenUrl`
for `password` and `clientCredentials`, both for `authorizationCode`. `refreshUrl` is optional on each.

**The compiler checks both ends.** A `security` entry naming no declared scheme does not compile, and
the message lists the names the program declares, so a typo is a compile error rather than a document
that references nothing (`E0886`). Two declarations of one name do not compile, and the error names
both sites (`E0887`). A field the scheme's `type` does not take, a required field left out, and a flow
that is not one of the four or lacks its URL are errors where they are written (`E0888`). A field no
type takes is the ordinary unknown option. A declared scheme that no route names is not an error: a
document may describe a scheme before a route uses it.

**The route table carries the declared schemes**, and `nvs build --openapi` writes them to
`components.securitySchemes`, sorted by name, so the document stays deterministic
(`rule:routing/api-document-is-a-deterministic-build-artifact`). The emitter still reads every fact
off the table (`rule:routing/api-document-is-generated-from-the-route-table`). A program that declares
no scheme writes no `components.securitySchemes`.

A scheme describes; it enforces nothing. Who may call a route is `#[Access]`'s decision
(`rule:attributes/access-is-a-required-sibling`), and a request runs the same with or without an
`#[Api\Scheme]` in the program.

- **Memory:** nothing per request. While compiling, one row per declared scheme and one entry per
  written `security` name, held in the route table.

**Declined:**
- **Schemes in `nvs.toml`.** The compiler never reads a deployment's configuration, so it could not
  check a `security` name against it, and the document would depend on which file a build saw.
- **A guessed definition for an undeclared name.** The emitter would be stating a fact about
  deployment that nobody wrote (`rule:routing/api-document-is-generated-from-the-route-table`).
- **Checking that `#[Access]` and the scheme agree.** An access decision is a name the compiler does
  not interpret (`rule:security/access-is-checked-for-presence-not-meaning`), so there is nothing to
  compare it with.
- **`type` and `in` as strings, as the specification spells them.** A mode string is a typo the
  compiler cannot see (`rule:core-api/no-mode-strings`), and the document still writes the
  specification's spelling.
- **Scopes as an anonymous object.** A scope name such as `orders:read` is not a field name a
  program can write, so the scopes are an array keyed by string.
