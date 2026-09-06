`Env\Mode` is a closed enum with exactly two cases, `Env\Mode::Production` and
`Env\Mode::Development`. **With nothing configured, the mode is `Production`.** That direction
matches every other default in the language — secure headers, closed CORS, the refusing sink,
deny-by-default capabilities — and it is the direction Laravel and Django did not take: a deployment
that forgets the line is safe, and a developer who wants the other one asks for it, once.

**The set is closed, and staging is not a third value.** A staging host runs `production` mode with
different configuration — different credentials, a different log target, wider limits — which is what
staging has always actually been. An open set of named environments buys per-name config files at the
cost of the compiler no longer being able to enumerate what a name does, which is the property
`rule:config/a-mode-is-five-defaults` exists to preserve; and since an unknown *key* in `nvs.toml` is
already an error, an unknown *value* silently meaning "some third thing" would be the odd one out.

A `test` run selects development-mode defaults inside its own isolate; it is not a third value
either.
