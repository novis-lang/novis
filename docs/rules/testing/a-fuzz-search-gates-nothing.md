A fuzz search finds new failing inputs. It never decides whether a run, a push or a release is green,
and nothing runs it on a schedule. What gates is the replay of every input already found.

**The check is a function, and `cargo test` replays it.** Each target's properties are one function in
`crates/nvs-fuzz-checks`, and the file under `fuzz/fuzz_targets/` only calls it. That crate's tests run
every input under `fuzz/seeds/<target>/` through the same function, in the ordinary `cargo test` of
every CI platform. So the inputs a search found stay checked forever, and the replay gives the same
verdict on every run. The `lint` job compiles the fuzz workspace, so a wrapper never drifts from its
check.

**A search runs after a change to the code a target reaches.** `bun nv fuzz <target>` runs it, and
`bun nv fuzz --all` runs every target and goes on after a crash. A goal that changes a parser of
untrusted input runs it until a run finds nothing, copies each input found into
`fuzz/seeds/<target>/`, and fixes each cause. A search of unchanged code mostly repeats work done
before.

**A target checks safety, not fidelity.** A target asserts what must hold for every input and what an
attacker could use: no panic, a bound on output and work, sanitized output that sanitizing again does
not change, a streamed result equal to the whole one. A property with no written end, such as two
converters agreeing on what a document means, belongs in a fixed table of tests. As a fuzz property
it fails on a new input every few minutes, and each failure is fidelity, not safety.
`markdown_from_html` therefore checks no panic and that Markdown written from its own HTML comes back
the same, and `markdown_to_html_to_markdown_keeps_the_meaning` holds the round trip.

What it spends: a parser bug that no search has found yet waits for the next change to that parser
or the next search a person runs, where a search in every run might have found it sooner. It saves
the 300 s per target that each CI run spent, the nightly toolchain and `cargo-fuzz` install in CI, the
corpus cache, and every red run that was luck.

**Declined:**
- **A fuzz search in every CI run, or in the nightly, as a job that can go red.** A green search says
  only that nothing was found in its time, so the gate passed by chance. A young target found the next
  input in nearly every run, and a loop that waited for a green search never ended.
- **A search that runs but cannot fail the run.** Nobody reads a report that is never red, and it
  spends runner time on every run.
- **Replaying a corpus cached between CI runs.** The cache is not in the repository, so a local run and
  CI would replay different inputs.
