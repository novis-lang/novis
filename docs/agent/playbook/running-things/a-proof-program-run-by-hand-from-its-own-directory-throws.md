- **A proof program run by hand from its own directory throws for a capability the tree grants.**
  The grant for `docs/examples/`, `tests/hostile/` and `benches/members/` is the root `nvs.toml`,
  and `nvs run` finds it from the working directory, so `cd docs/examples/core/IO/write && nvs run
  01-….nvs` throws `Core\IO::temporaryDir needs the capability fs.write` and reads as a broken
  member. Run every proof as `target/debug/nvs.exe run <repo-relative path>` from the repository
  root, which is what `tools/dossier.py` does. [until: reviewed 2026-09-23]
