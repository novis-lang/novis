Every user-written identifier is checked against exactly one pattern for its category, and a mismatch is a
hard compile error — not a lint, not a warning.

| Category | Convention | Example |
|---|---|---|
| Class, interface, enum | `PascalCase` | `HTTPClient`, `Comparable` |
| `type` alias, at file scope or in a body | `PascalCase` | `UserId` |
| Enum case | `PascalCase` | `Active` |
| Namespace segment | `PascalCase` | `Core\Html\Markup` |
| Method, instance or `static` | `camelCase` | `parseXMLPayload` |
| Property, any visibility | `camelCase` | `userName` |
| Parameter, local variable | `camelCase` | `$rowCount` |
| Class constant | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES` |

Built-in type keywords and language keywords are reserved words the grammar already lowercases and are not
checked. No identifier may begin with `_`, in any category, which is what leaves the rule exception-free:
with the constructor spelled as a word rather than with underscores, there is no reserved name a casing
rule has to be told to skip.

The check needs no name resolution — every category is already its own syntax node — so it runs before
anything is resolved and its diagnostic carries a mechanically derived rename. It is decided while the
standard library is unwritten because renaming a member later is a breaking change with no deprecation path
(`rule:statements/nothing-gets-a-second-name`). The cost is a structural break from PHP: `snake_case`
source does not compile, and a converted name that collides with another after rewriting is the one case a
converter cannot settle alone.
