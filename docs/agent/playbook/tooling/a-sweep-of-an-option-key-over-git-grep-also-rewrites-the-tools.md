- **A rename sweep of an option key over `git grep -l` also rewrites the tools' TypeScript.** A
  stage 7 script that turned `{ cwd:` into `{ workingDirectory:` across `.nvs`, `.md`, `.rs` and
  `.ts` rewrote about 120 `spawn(..., { cwd: ROOT })` calls under `tools/`, which are Bun options
  and not Novis code. Keep a key sweep to the Novis files (`.nvs`, `.nvst`, `.lspt`) and the Novis
  strings in Rust tests, and check the result with `nvs check --json`, which names an old key as
  "is not an option of this member". See `tools/nv/cmd/api-rename.ts`.
