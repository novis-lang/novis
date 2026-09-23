- **A generated file can be current on the machine that wrote it and stale everywhere else.**
  `docs/novis.md` embeds `nvs meta --json`, which answers for the platform the binary was built on,
  so a platform-varying constant such as `Core\Env::OS` makes a Windows-generated file stale on CI
  while `reference.py --check` passes locally and says only *stale*. Pin the constant to one
  spelling in `PLATFORM_VALUES` in `tools/reference.py`, or regenerate under WSL with a Linux `nvs`
  in `target/debug/` and read the `git diff`. [until: gone tools/reference.py:PLATFORM_VALUES]
