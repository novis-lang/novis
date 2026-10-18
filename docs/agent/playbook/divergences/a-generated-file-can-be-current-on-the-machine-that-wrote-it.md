- **A generated file can be current on the machine that wrote it and stale everywhere else.**
  `website/public/llms-full.txt` embeds `nvs meta --json`, which answers for the platform the binary
  was built on, so a platform-varying constant such as `Core\Env::OS` makes a Windows-generated file
  stale on CI while `bun nv render --website --check` passes locally and says only that it is not
  what its sources render. Pin the constant to one spelling in `PLATFORM_VALUES` in
  `tools/nv/cmd/reference.ts`, or regenerate under WSL with a Linux `nvs` and read the `git diff`.
  `until: gone tools/nv/cmd/reference.ts:PLATFORM_VALUES`
