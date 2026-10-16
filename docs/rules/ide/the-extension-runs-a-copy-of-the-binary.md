The VS Code extension never runs the binary `nvs.path` or `PATH` names: every process it starts — the
server, the Tasks, the formatter, the AST panel and the test controller — runs from a verified copy of
that binary in the extension's own storage, and the copy is never an older build than the one on disk.
The server lives as long as the window and a program in a Task's terminal as long as the user leaves it,
and every platform guards the file of a running program — Windows refuses to overwrite or delete it,
Linux fails a write into it with *Text file busy*, macOS may kill a signed process whose file is
rewritten — so an editor that ran the named file would turn being open into a build or an upgrade that
cannot write its output.

**The copy and the binary are the same program, or the copy is not used.** A copy is handed out only
after both files have been hashed and found equal and the binary's size and modification time have been
read again and found unchanged, whether the copy was just made or was already there. A copy taken while
a linker is still writing is discarded and made again. The copy is named for the binary's path, size and
modification time, so every window on one build shares one file and no window replaces a file another is
running.

**Every spawn asks which file to run at the moment it spawns**, in one place, `src/binary.ts`. The answer
is the remembered copy only while the binary and the copy both still read the size and modification time
they had when they were hashed — two `stat`s — and anything else is a new verified copy before the
process starts. A Task is the one thing that carries a path for longer than a spawn, because the editor
keeps Task objects of its own; one started over the copy of an earlier build is stopped and run again
over the current one.

**A changed binary takes over the server within seconds, and the user sees nothing.** The extension reads
the binary's size and modification time once a second, and acts when a reading differs from the running
build and equals the reading before it; the second equal reading keeps it off a half-written file. It
polls rather than subscribing because the file is replaced, not edited, and one `stat` a second behaves
the same on every platform, network mount and WSL path. The new copy is made and verified and the new
server started and its version compared (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`)
while the old server is still answering, and only then is the old one stopped, so there is no gap in
diagnostics or completion and the status item never leaves its resting state. **A server that is
answering is the build on disk, or there is no server**: when the new build will not start or is refused,
the old one is stopped all the same and the status item says why. The watch is set whatever the start
came to, so the rebuild of a binary that would not start is picked up without being asked for.

Where no copy can be made, or the copy will not start — storage mounted `noexec` — the named binary is
run and the status item says that file is being held, and why. There is no setting for any of this.
Copies live under `server/` in the extension's global storage, which is on the machine the code is on
(`rule:ide/the-extension-is-a-workspace-extension`); a copy of an earlier build of the same binary is
deleted when a new one is made, a copy of some other binary once it is a week old, and a file another
window is running is left for the next sweep, and so is any file this module did not write.

A copy is the same program in the two places `nvs` reads its own file. A bundle's payload is read from
the end of the running executable, which a byte copy keeps. The compile cache's build stamp hashes the
executable's path, length and modification time, so a copy keys apart from its binary: one cold compile
for each new build, and the same key on every later run of that copy. A new build costs one copy and two
hashes of `nvs`, once per window; every other spawn costs two `stat`s, and nothing is on a request path.
For the moment two servers answer, a request may be answered by both.

**Memory:** nothing beyond a second server process for the moment of a switch. It spends disk instead:
one file the size of `nvs` for each binary in use, under the extension's storage.

**Declined:**
- **Copying on Windows only.** The lock is not only Windows', the stale server is every platform's, and
  a branch taken on one platform is one the other platforms' test runs never execute.
- **Copying only for the server.** A program in an `nvs run` Task runs as long as the server does, and
  "the editor never holds the binary" is a rule a user can rely on where "only sometimes" is not.
- **Renaming the old binary aside when updating.** Windows allows renaming a running executable, but it
  does nothing for a `nvs.path` pointed into `target/`, where the linker writes the file.
- **`nvs lsp` copying and re-executing itself.** The editor tracks the process it spawned, so the parent
  stays alive, and the parent is the file that is held.
- **A command that stops the server.** It needs the user to remember the cause, which is what keeps not
  happening.
- **Stopping the old server before starting the new one.** It costs a gap in diagnostics and completion
  on every rebuild.
- **Keeping the old server when the new build fails.** Its answers would come from a program that is no
  longer on disk.
- **Naming the copy for a hash of its content.** That needs the binary hashed before there is a name to
  look for; the stamp gives the name for one `stat`, and the hash still decides whether the file is used.
- **A setting to turn it off.** The fallback is automatic, and a setting is kept forever.
