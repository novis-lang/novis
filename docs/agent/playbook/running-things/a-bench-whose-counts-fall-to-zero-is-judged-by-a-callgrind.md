- **A bench whose counts fall to 0 per op is judged by callgrind, on a WSL binary nothing rebuilds.**
  With every count flat at 0 the batch ramp cannot settle, so `bun nv proofs --record-perf` runs
  `/var/tmp/nvs-target-wsl/debug/nvs` under callgrind. That binary is only as new as its last WSL
  build, and its jitter of tens of thousands of instructions per run reads as growth at batch 32 to
  128. `core/Metrics/observe.nvs` failed with "instructions grows" twice while a hand ramp to 512 was
  flat at about 9,000 per op. Rebuild the binary with `syncMirror` and `wslBuildLine`
  (`tools/nv/accept/`), then raise the bench's `iterations` until the ramp is flat.
  `until: gone tools/nv/cmd/scaling.ts:DEFAULT_WSL_NVS`
