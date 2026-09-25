- **A new `examples/*.nvs` fixture is a capability denial until the repository's own `nvs.toml`
  grants it, and the driver reports that as wrong output.** A member
  `rule:security/capability-question-is-grant-and-scope` denies by default prints nothing on stdout
  and one `RuntimeError` on stderr, so an `exact` check reads as "stdout was [], wanted [...]" with
  nothing pointing at the config. `grep -n 'entry = "examples' nvs.toml` lists the existing
  `[[app]]` blocks; copy the neighbouring one with the narrowest grant, in the same slice as the
  fixture. [until: gone nvs.toml:entry]
