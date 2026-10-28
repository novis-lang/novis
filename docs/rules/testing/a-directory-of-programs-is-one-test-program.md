A directory handed to `nvs test` that holds `.nvs` files is one program: the runner writes an entry
that requires every `.nvs` file under the directory that no other file under it requires,
subdirectories walked, in name order, and runs that program's test table. The entry is never put on
disk, and its one job is to bring every file into the compile.

The directory carries its own `autoload` map. Declarations reach the program the way they reach any
program, through a file the entry requires. Run over a project's root — `nvs test .`, the command
the website teaches — that file is the project's own entry file, so tests written under the autoload
root, beside the classes they test or in a class of their own, need nothing else, and the editor
lends them the same map because the root owns them. A directory of tests outside every autoload root
— a project's `tests/` — holds one file that requires the application's bootstrap, and every other
file in it requires nothing; the editor needs that file to lend the map, and the project's root still
runs as one program with both. A directory holding no such file compiles as it is, which is right
for tests that need no class outside themselves and reports `E0306` for the rest, exactly as running
the same files by hand would.

- **A file another file requires is reached through it.** `require` has no once-only guard
  (`rule:statements/require-is-the-only-inclusion-construct`) and `Core\Test::request` runs the
  program, requires included, so an entry that required the project's entry beside a bootstrap that
  requires it would run it twice per request. The edges are the ones the name-resolution walk
  resolves, so the runner agrees with the build on what a path names, and a directory whose files
  hold no `require` walks nothing first.
- **A test request runs `public/index.nvs` alone.** The entry requires every file, so a script
  under `jobs/` is checked with the rest. When the directory has a `public/index.nvs`,
  `Core\Test::request` runs that file's frame in place of the entry, as a served request does, and
  never a scheduled or operator script (`rule:testing/in-process-request`). A directory without one
  answers with the entry.
- **One program per call.** A second path beside a program is refused with a sentence about the
  count: `nvs test .` over the root is how a project with tests in two places runs them together.
- **Name order, not test order.** The order the files are required in decides nothing a reader can
  see: classes are reported in name order and methods in declaration order, whatever brought them in.
- **A file is a program, a directory of files is a program, and a directory holding both `.nvs` and
  `.nvst` files is refused**, since the two suites share no report
  (`rule:testing/nvst-is-separate`). A directory holding only `.nvst` files is the case tree it always
  was.
- **The configuration is folded for the directory.** An `[[app]]` block whose `root` contains it
  applies; an `entry` glob names a file and matches no directory.
- **The editor reads the directory the same way.** A plain file under the directory of a file that
  requires the bootstrap is analysed through that bootstrap's map
  (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).

A new test file is in the suite the moment it exists, and nothing is added to the language, the
configuration or the command line for it. The cost is one directory walk per invocation before the
compile, and a compile that requires as many files as a hand-written entry naming each one would.
Nothing is on a request path of the runtime.

**Memory:** the written entry is one string, held for the run.

**Declined:**
- **A glob in a literal `require`**, `require 'tests/*Test.nvs';`. A `require` site is one call to
  one file's script frame, and a site that expands to many files reaches into the type checker and
  the compiler for a feature the tests never execute.
- **A `[test]` block naming a bootstrap and directories.** The configuration is per server and an
  application's tests are per application, the editor reads no `--config`, and a suite would need a
  setting before its first run.
- **A `--bootstrap` flag beside the directory.** Every run would spell it, and the editor cannot
  see it.
- **Running each `.nvs` file under the directory as its own program.** A test file that requires
  nothing resolves nothing, and a report per file has no summary.
- **Requiring the bootstrap from every test file.** It is one line per file across the suite, the
  same list an entry naming every file is.
- **Reading the directory as a program only when it holds no `index.nvs`.** It is a convention on a
  name the runner would have to explain, for a case the mixed-directory refusal already covers.
