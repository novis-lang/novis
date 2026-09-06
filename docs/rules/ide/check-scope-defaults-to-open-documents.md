`nvs.check.scope` is `"open"` or `"workspace"`, default `"open"`: diagnostics are published for open
documents and their `require`/`autoload` graph, or for every file the index holds. The default does not
change what the editor did before the setting existed. `nvs.checkWorkspace` runs one workspace pass on
demand without changing the setting, which is the cheap version of the same thing.

Workspace scope is the expensive setting, and the one whose cost is measured least, which is why it is
off by default. The one feature that is only correct at workspace scope —
`rule:ide/five-features-are-one-reference-index`'s unused-member dimming — is silent under the default
rather than wrong, and lands together with this setting.

Both identifiers, with `nvs.codeLens.enable`, `nvs.template.services` and the `nvs/regions` request, are
added to the extension's frozen roster under that roster's own rule: a name is added and never renamed.
