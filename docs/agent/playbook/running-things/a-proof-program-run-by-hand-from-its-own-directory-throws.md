- **A proof program run by hand from its own directory throws for a capability the tree grants.**
  The grant for `docs/examples/`, `tests/hostile/` and `benches/members/` is the root `nvs.toml`,
  and `nvs run` finds it from the working directory, so `cd docs/examples/core/IO/write && nvs run
  01-….nvs` throws `Core\IO::temporaryDir needs the capability fs.write` and reads as a broken
  member. Run every proof from the repository root as `target/debug/nvs.exe run <repo-relative
  path>`; `bun nv proofs` runs them from there too. [until: gone tools/nv/proofs/run.ts:runPrograms]
