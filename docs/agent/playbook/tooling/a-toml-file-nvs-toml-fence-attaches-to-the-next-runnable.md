- **A `toml file=nvs.toml` fence attaches to the next *runnable* example, not to the next fence, so an
  `nvs skip` between them hands the config to a later program.** `tools/nv/cmd/reference.ts`'s
  `examplesIn` skips a `skip` fence without clearing `pending`, so a configuration block written to
  illustrate an unrunnable example lands in the working directory of whatever runnable example comes
  next — on a reference page that ends with the house `LogicError` refusal, that is the one it reaches.
  Put the configuration in a comment inside the `skip` fence, or place the `file=` block directly before
  the runnable program it belongs to. [until: gone tools/nv/cmd/reference.ts:pending]
