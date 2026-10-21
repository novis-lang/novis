<!-- looper:begin bun v3 -->
# Bun and TypeScript

The rules of this project's TypeScript code, on top of `AGENTS.md`.

- **Bun is pinned in `package.json`'s `engines.bun`.** Bump it in a commit of its own.
- **By default, dependencies are installed at exact versions,** and each bump is a commit of its own. A
  project may change this in its own part of `AGENTS.md`.
- **`looper verify` runs `bun install`, then `tsc`, then the tests a change reaches.** `tsconfig.json`
  is strict, and a type error fails the gate.
- **A stage other than the gate stage checks with `tsc`.** It type-checks the tests too, and runs none.
- **Never run a bare `bun test`.** It runs every test file. Run `looper verify`, or name one test file.
- **A TypeScript file's name is kebab-case.**
- **Look up a Bun API in the installed version's reference,** never from memory, because it moves fast.
