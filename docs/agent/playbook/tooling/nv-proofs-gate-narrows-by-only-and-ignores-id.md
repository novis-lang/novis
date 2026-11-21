- **`bun nv proofs --gate` narrows by `--only`, and ignores `--id`.** A goal's check written as
  `proofs --id 'Core\Test::request' --gate` gated the whole roster and failed on 144 features the
  goal never touched. Write a feature's proofs check as `bun nv proofs --only '<feature>' --gate`.
  See `tools/nv/cmd/proofs.ts`, the `scoped` line.
