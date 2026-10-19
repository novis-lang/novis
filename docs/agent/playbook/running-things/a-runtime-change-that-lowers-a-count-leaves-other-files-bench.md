- **A change to a shared runtime path leaves every bench recorded against another file looking
  current, while its declared count is already wrong.** A figure is keyed on its implementing file's
  hash, so after `call_callable` stopped allocating its slots, `Core\Cli::live` and
  `lang:types/callable-classes-object-shapes` still showed a current record and only failed once
  re-measured. Before the wrap, grep `benches/members` for the shape the change reaches (here
  `fn (`) and run `bun nv proofs --record-perf --force --only <id> ...` over those benches.
  `until: gone tools/nv/proofs/collect.ts:implHash`
