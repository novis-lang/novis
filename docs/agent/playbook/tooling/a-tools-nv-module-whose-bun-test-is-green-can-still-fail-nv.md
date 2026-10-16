- **A `tools/nv` module whose `bun test` is green can still fail `nv verify` at `nv selftest`'s `tsc`.**
  The tools' tsconfig sets `exactOptionalPropertyTypes`, which Bun's runner does not check, so an
  object literal that passes `field: maybeUndefined` to an optional `field?: T` is a type error there
  only. Declare such a field `field?: T | undefined` when you write it, as `tools/nv/cmd/deps.ts`'s
  `Pin` does.
