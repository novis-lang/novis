The set of top-level blocks is closed, and the typed tree refuses any other with the unknown-key
diagnostic (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`). The format rule owns
only the *syntax*; every block's directives, defaults and changeability class are argued by the rule
that adds the block, so a reader of `nvs.toml` has one place to start:

| Block | Argued by |
|---|---|
| `[[include]]` | `rule:config/include-takes-a-path-or-a-dir` |
| `[[app]]` | `rule:config/an-application-is-its-entry-file-path` |
| `[limits]`, `[limits.hard]`, `[capabilities]` | the changeability model and the capability rules |
| `[mode]` | the run-mode rules |
| `[[extension]]` | the extension system |
| `[debug]` | `rule:testing/debug-mode-directive` |
| `[log]`, `[http.errors]` | `rule:errors/log-level`, `rule:errors/engine-floor` |
| `[errors]` | `rule:errors/a-use-of-deprecated-code-may-log-or-throw` |
| `[db.<name>]` | `rule:core-classes/db-connection-is-named` |
| `[deferred]` | `rule:concurrency/deferred-is-bounded-by-two-directives` |
| `[[schedule]]`, `[queue]` | the scheduled-work rules, `rule:core-classes/queue-storage-is-a-table` |
| `[http.headers]`, `[http.cors]`, `[http.cookies]`, `[http.client]` | the safe-defaults rules |
| `[metrics]`, `[trace]` | the observability rules |
| `[server]`, `[[server.mount]]` | `rule:routing/a-request-reads-its-mount` |
| `[cache]`, `[opcache]` | the artifact cache and hot reload rules |
| `[control]` | the control socket rules |
| `[image]` | `rule:core-classes/image-pipeline` |
| `[io]` | `rule:core-classes/temporary-dir-sweep` |
| `[mail.<name>]`, `[storage.<name>]` | `rule:programs/framework-core-half` |
| `[session]` | `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot` |
| `[app.log]` | `rule:errors/handler-script` |

`[[schedule]]`, like `[[extension]]`, is an array of tables because it is a repeated record with
several fields.
