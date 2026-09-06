M4B ships two code actions and only two: the casing fix (`rule:core-api/identifier-casing`) and the
legacy-cast fix `(int)$x` → `$x as int` (`rule:types/no-legacy-cast`). They are admitted for one reason,
and it is not that they are useful: their replacement text is already computed, sitting in the
`Diagnostic::suggestions` field `nvs-diagnostics` has carried since M0. The provider is a translation from
`Suggestion` to `CodeAction` — a dozen lines and no new analysis.

The boundary is exactly that. A quick fix whose replacement a diagnostic already knows may ship; one that
would need the checker to compute something new is M10's. Both are registered under `source.fixAll.nvs`
so `editor.codeActionsOnSave` composes them with format-on-save when that arrives.
