The kind of security scheme that an `#[Core\Api\Scheme]` declares. There are five kinds. They are
the five that OpenAPI 3.1 lists.

You write a case as the `type` of a scheme. The type decides which other fields the scheme needs:

| Case | Needs | Optional |
|---|---|---|
| `ApiKey` | `in`, `parameter` | |
| `Http` | `scheme`, such as `bearer` or `basic` | `format` |
| `MutualTls` | nothing | |
| `OAuth2` | `flows` | |
| `OpenIdConnect` | `url` | |

A missing field does not compile. A field that the type does not take does not compile either.

`nvs build --openapi` writes each scheme to the API document. The case becomes the scheme's `type`
in the way OpenAPI writes it, for example `apiKey` or `mutualTLS`.

**Good to know:** a scheme only describes your API to its clients. It does not check a request.
`#[Core\Access]` decides who may call a route.
