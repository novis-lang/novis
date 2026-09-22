A directory handed to `nvs test` that holds `.nvs` files is one program: the runner writes an entry
that requires every `.nvs` file under the directory, subdirectories walked, in name order, and runs
that program's test table. The entry is never put on disk, its statements never run — no entry's do
under `nvs test` — and its one job is to bring every file into the compile.

The directory carries its own bootstrap. `autoload` declarations reach the program the way they
reach any program, through a file the entry requires, so a directory whose tests use the
application's classes holds one file that requires the application's bootstrap file, and every
other file in it requires nothing. A directory holding no such file compiles as it is, which is
right for tests that need no class outside themselves and reports `E0306` for the rest, exactly as
running the same files by hand would.

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
