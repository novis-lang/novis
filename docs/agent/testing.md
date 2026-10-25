# Testing

A test proves a behaviour, and a run proves only what a change can affect.

- **A goal is verified once, in its last stage, through `bun nv affected --run`.** Never run a wider
  command. Verify runs each step and each test only when a change reaches what it reads. A loop session
  runs `looper verify`, which runs `bun nv verify`. Every other stage checks with a build, and runs only
  a test it wrote, once and by name, or a run one of its items names (AGENTS.md § *The rules only this
  project has*).
- **In the loop, run `looper verify` even while another session's verify runs in the tree.** Yours
  joins that run, and its result covers both sessions' code. Never wait for another verify, and never
  kill one.
- **`bun nv affected` says what a change reaches.** A session's handoff names what it found.
- **A new kind of test ships with what records its reads,** as `tools/nv/select/checks.ts`'s module doc
  says. A test that rests on the whole tree runs after every change, and that is a defect to fix.
- **A test asserts a property or an exact value,** never "it did not crash".
- **A test builds its own scratch repository** under `.agent-tmp/`, deletes it on success, and never
  touches a real project or a real service.
- **A test's name states the behaviour it checks** ([conventions.md](conventions.md) § *Names*).
- **A goal's check names its tests** in `tests: [...]`, which proves they ran as well as passed.

## A test holds on every CI runner

CI runs each test on Linux, macOS, aarch64 and a fresh Windows runner, under load, beside the rest of
its binary's tests. A test that passes here and fails there costs a CI round of an hour or two. Each
rule below is a failure CI found that the dev machine could not.

- **A test waits for an event, never for a span of time.** Count the frames, calls or lines it expects,
  or wait on the signal that says the other side is ready. A bound it still needs gets ten times the
  time it takes here: a 20 ms window failed on aarch64, and again on Windows.
- **A test sets the process-wide state it reads.** A drain bit, an environment variable, a current
  directory, a global counter: the neighbouring test in the same binary changes it between two reads.
  Begin the state inside the test, or hold the lock that test also takes.
- **A test or an attack declares its whole environment.** A grant it relies on is written, as
  `exec = []` is when no program may start. A loopback address is `127.0.0.1` or `::1`: macOS has no
  `127.0.0.2`. Its files go under its own scratch directory, whose ownership and permissions the test
  sets: a runner's drive grants `BUILTIN\Users` what this machine does not.
- **A recorded output never contains a version, a host, a time or an absolute path.** The `.out` elides
  those lines: CI builds `0.0.1+dev`, and its host and paths are not these.
- **The code under test follows [coding.md](coding.md)**, whose last rule runs the tests of a change
  to paths, permissions, sockets, processes or signals on Linux: `bun nv linux-test`.
- **A test Miri runs is sized for Miri.** Miri runs a test thousands of times slower and models no
  `getrusage`, `mmap(PROT_NONE)` or real process: shrink the loop under `cfg!(miri)`, or mark the test
  `#[cfg_attr(miri, ignore = "<why>")]` on one line, as rustfmt writes it.
- **Before a goal is claimed done, `bun nv ci --check` is green.** It is the checks the goal end's push
  runs: rustfmt, the lint tables, the rules, the rendered files and the website, the license file,
  the advisories, and the feature proofs on a binary built from the tree. They take about a minute,
  and a red one refuses the push (`tools/nv/cmd/ci.ts`).
