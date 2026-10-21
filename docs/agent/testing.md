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
