The opinionated half is `nvs/web`, a first-party package fetched, locked, logged and granted exactly
like any other, and bound by every package rule — including the one that makes a breaking release a new
package name. Its cadence is deliberately slow. The `Core` half versions with the binary and carries
every compatibility rule `Core` already carries.

It provides the `Web` namespace, and holds what an application needs and a language should not fix:
`Web\Middleware`, a pipeline a `Core\Router\Gate` runs, since dispatch itself is `Core`'s
`Core\Router::dispatch` (`rule:routing/a-gate-answers-around-every-handler`); `Web\Response`, with
`view`, `json`, `redirect`, `file` and `stream` constructors over the safe HTTP defaults; `Web\Auth`,
a gate whose `allows` interprets `#[Access]`, holding login, logout, remember-me, password reset,
policies and role checks; `Web\Validation`, named rule sets
and form binding over `Core\Validate` — the ergonomics, never the laundering; and `Web\Mail`,
`Web\I18n`, `Web\Storage`, `Web\Pagination`, `Web\Filter`, `Web\Migration`
(`rule:programs/no-migration-runner`), `Web\Job`, `Web\Api` and the scaffolding templates.

**There is no view layer, because the language is one.** Inline HTML with `<?nvs` and `<?=` is already
the template engine and the HTML sink already auto-escapes by default, so a second templating language
would be a second spelling of one job. `Web\Response::view` renders a `.nvs` file and nothing more.

**Not on disk.** There is no `nvs/web` package and no `Web` namespace.
