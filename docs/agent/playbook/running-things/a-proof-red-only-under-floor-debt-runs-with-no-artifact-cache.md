- **A proof program red under `bun nv debt` and green under `bun nv proofs --verify` ran with no
  artifact cache: the debt records it, and a recorded run sets `NOVIS_NO_FILE_CACHE`.** So it
  compiles every built-in component cold, inside the request, and a fault that only a cold compile
  has shows only there. Five PDF attacks ran past their memory limit that way, because the compile
  widened the request's ceiling. Reproduce with `NV_PROOF_RECORD=.agent-tmp/<dir>` before
  `bun nv proofs --verify`, and compare a fresh `--data` folder with a warm one. See
  `tools/nv/proofs/run.ts`. `until: gone tools/nv/proofs/run.ts:NOVIS_NO_FILE_CACHE`
