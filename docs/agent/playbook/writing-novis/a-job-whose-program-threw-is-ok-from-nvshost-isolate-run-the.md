- **A job whose program threw is `Ok` from `nvs_host::Isolate::run`; the judgement is
  `Completion::ok`.** `run(ctx)` answers `Result<Completion, GraphError>` and the `Err` half is only
  the argument graph refusing to cross, so a fixture that throws on purpose reads as success to an
  `if let Err(…)` and lands in the wrong bucket with no error anywhere. Read `Completion::ok`, which
  is already false for a throw and for a budget teardown alike. [until: reviewed 2026-09-06]
