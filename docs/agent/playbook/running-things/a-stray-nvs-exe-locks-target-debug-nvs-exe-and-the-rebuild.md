- **A stray `nvs.exe` locks `target/debug/nvs.exe`, and the rebuild that cannot replace it still
  looks like it worked.** A language server left by a test, a probe or a killed editor holds the
  binary open, so the linker fails with `os error 5` in the middle of ordinary output and the *old*
  binary stays on disk. `looper exec list` names the job running an `nvs.exe`: end your own with
  `looper exec kill <id>`, ask the user about any other, and check the mtime, not the exit status.
  `until: gone tools/nv/cmd/verify.ts:const NVS`
