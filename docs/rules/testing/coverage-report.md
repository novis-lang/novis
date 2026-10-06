`nvs test --coverage-lcov <FILE>`, `--coverage-clover <FILE>` and `--coverage-cobertura <FILE>` write
which lines and branches a program's `#[Test]` run reached, and a run may write any of them together.
Every line a statement starts on is listed with the largest count of the statements starting on it;
every function is compiled, so a line no test reached reads `0`. lcov and Clover also list every
`nvs_ir::Function` that has a statement, under the IR's name, at the line of its first statement and
with that statement's count. Every format lists every `Terminator::Branch` at the line its condition
starts on, with how often its true and its false side ran: lcov as `BRDA` lines with `BRF`/`BRH`, and
`-` for both sides of a branch that never ran; Clover as a `<line type="cond">` with `truecount` and
`falsecount` and two `conditionals` per branch; Cobertura as `branch="true"` and a
`condition-coverage` on the line and a `branch-rate`. A `Terminator::Switch` has no probe and is in no
report. Cobertura groups the files into one package per directory and gives a `line-rate` and a
`branch-rate` at every level. A file not on disk is left out, and a file is named relative to the
directory the run started in, with `/` between parts. The flags are refused beside a `.nvst` tree and
beside `--list`, and under a machine `--format` stdout is still that format's document alone.

The counts come from `rule:testing/debug-probes`'s statement and edge probes. Codegen adds a
per-function base to each `StmtId` and each `EdgeId`, so the number a probe passes names one statement
or one edge in the whole program, and `nvs_ir::Program::stmt_spans` and `edge_spans` are the tables of
where each is written. Every coverage flag turns both `DebugFlags::COVERAGE` and `DebugFlags::BRANCH`
on and gives the suite's context one `StmtHits` table with a counter per statement and per edge; every
context the run makes from it — each test's isolate, a fixture, an in-process request, a request a
`server: true` test sends — counts into that one table. Novis code cannot read it. A run that does not
ask makes no table.
