- **A `cargo-named` check matches its name as a *substring* of the run's output, so a longer test
  name satisfies a shorter one and a differently-worded one satisfies nothing.**
  `two_body_spellings_in_one_spec_are_refused` is green under `..._refused_naming_both`, so
  `did not run` can mean *named differently* rather than *not written*. `judgeTests` in
  `tools/nv/driver/accept.ts` is the matcher: grep the crate for what the name *describes*, then rename that test rather than landing a
  twin beside it. [until: gone tools/nv/driver/accept.ts:judgeTests]
