- **`bun nv bench-compare` behind any prefix is killed as a hang while Docker builds its image.**
  `looper.toml`'s `[jobs] hang_exempt` matches the start of the Bash call, so `mkdir -p x && bun nv
  bench-compare --smoke > log` is watched. The build's CPU is in Docker's VM, and the call is killed
  after 90 s with exit 124. Run `bun nv bench-compare ...` as the whole call, with
  `run_in_background`, and read its output file. See `tools/looper/src/jobs/hang.ts`.
  `until: gone looper.toml:hang_exempt = ["^bun nv bench-`
