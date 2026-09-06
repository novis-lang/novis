```toml
[mode]
default = "production"      # Runtime — the mode an application starts in
ceiling = "development"     # System  — the most permissive mode any code may select
```

`nvs.toml` is one source: `[mode] default` is `Runtime`-class, which is what makes a runtime flip
possible at all. **`nvs serve --mode=development` is the other, and it overrides the file** —
ordinary CLI precedence, the flag being the last word about the mode the server starts in. The flag
replaces the **global** value; a matching `[[app]]` block still layers over it, so a flag never drags
an application that pins its own mode along with it. The flag set is closed and there is no `--set`.

This is a deliberate choice of ergonomics over one safety catch. Refusing a flag that contradicts the
file would have caught a deploy script carrying a stale `--mode`; what covers that case instead is the
banner and `Warn` record a development-mode server emits when it binds a public interface, so the
payment for flag-over-file is visible rather than silent.

The ceiling bounds a runtime flip, not the startup value: `nvs serve --mode=development` in a
directory with no `nvs.toml` simply works, and the ceiling follows it.
