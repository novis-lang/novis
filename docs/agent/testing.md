<!-- looper:begin testing v1 -->
# Testing

A test proves a behaviour, and a run proves only what a change can affect.

- **Test only what a change can affect, through `looper verify`.** Never run a wider command. Verify
  runs each step and each test only when a change reaches what it reads.
- **`looper affected` says what a change reaches.** A session's handoff names what it found.
- **A new kind of test ships with what records its reads, or declares its `inputs`** in `looper.toml`.
  A test that rests on the whole tree runs after every change, and that is a defect to fix.
- **A test asserts a property or an exact value,** never "it did not crash".
- **A test builds its own scratch repository** under `.agent-tmp/`, deletes it on success, and never
  touches a real project or a real service.
- **A test's name states the behaviour it checks** ([conventions.md](conventions.md) § *Names*).
- **A goal's check names its tests** in `tests: [...]`, which proves they ran as well as passed.
