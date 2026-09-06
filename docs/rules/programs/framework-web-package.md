The opinionated half is `nvs/web`, a first-party package fetched, locked, logged and granted exactly
like any other, and bound by every package rule — including the one that makes a breaking release a new
package name. Its cadence is deliberately slow. The `Core` half versions with the binary and carries
every compatibility rule `Core` already carries.

It provides the `Web` namespace, and holds what an application needs and a language should not fix:
`Web\Controller` and `Web\Middleware`, dispatch and a middleware pipeline over the compile-time route
table, whose `switch` is generated while compiling so an application never writes one and receives
typed, laundered parameters directly; `Web\Response`, with `view`, `json`, `redirect`, `file` and
`stream` constructors over the safe HTTP defaults; `Web\Auth`, **the** enforcer of `#[Access]`, holding
login, logout, remember-me, password reset, policies and role checks; `Web\Validation`, named rule sets
and form binding over `Core\Validate` — the ergonomics, never the laundering; and `Web\Mail`,
`Web\I18n`, `Web\Storage`, `Web\Pagination`, `Web\Filter`, `Web\Migration`
(`rule:programs/no-migration-runner`), `Web\Job`, `Web\Api` and the scaffolding templates.

**There is no view layer, because the language is one.** Inline HTML with `<?nvs` and `<?=` is already
the template engine and the HTML sink already auto-escapes by default, so a second templating language
would be a second spelling of one job. `Web\Response::view` renders a `.nvs` file and nothing more.
