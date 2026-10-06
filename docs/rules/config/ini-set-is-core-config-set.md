`Core\Config` is the run-time API over the configuration, four static methods and no free function
(`rule:classes/no-free-functions-or-constants`):

| Method | What it does |
|---|---|
| `Core\Config::set(string $name, string $value): bool` | changes a directive for this request |
| `Core\Config::get(string $name): ?string` | reads the value in force |
| `Core\Config::restore(string $name): void` | drops this request's change and reads the configured value again |
| `Core\Config::all(): array<string, string>` | reads every directive |

The semantics are the changeability model's, unchanged: a set the class or a ceiling refuses returns
`false` and leaves the value in force untouched, and every accepted change is request-local on the
copy-on-write overlay, invisible to the next request on the same core. A bare limit name and its
`[limits]` spelling are one key to every member.

Values cross this API as `string` in both directions even where the file is typed, because the
directive *name* is dynamic here and one return type is what makes that possible. The registry parses
the string with the parser the boot path uses
(`rule:config/one-parser-for-the-boot-path-and-config-set`). On a host with no configuration at all
`get` is `null`, `all` is empty, `set` is `false` and nothing throws
(`rule:config/no-configuration-file-is-a-complete-configuration`).
