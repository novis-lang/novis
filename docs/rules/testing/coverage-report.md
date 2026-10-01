`nvs test --coverage-lcov <FILE>` and `--coverage-clover <FILE>` write which lines a program's
`#[Test]` run reached, and a run may write both. Every line a statement starts on is listed with the
largest count of the statements starting on it; every function is compiled, so a line no test reached
reads `0`. A file not on disk is left out, and a file is named relative to the directory the run
started in, with `/` between parts. The flags are refused beside a `.nvst` tree and beside `--list`,
and under a machine `--format` stdout is still that format's document alone.

The counts come from `rule:testing/debug-probes`'s statement probe. Codegen adds a per-function base to
each `StmtId`, so the number a probe passes names one statement in the whole program, and
`nvs_ir::Program::stmt_spans` is the table of where each is written. The runner turns
`DebugFlags::COVERAGE` on and gives the suite's context one `StmtHits` table; every context the run
makes from it — each test's isolate, a fixture, an in-process request, a request a `server: true` test
sends — counts into that one table. Novis code cannot read it. A run that does not ask makes no table.
